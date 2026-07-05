//! Background emulator installs: streaming download with SHA-256
//! verification, then archive extraction into the emulator's directory.
//! Progress flows to the UI over an mpsc channel drained each frame.

use super::emulator::{ArchiveKind, DownloadSpec, Emulator, ExtractRule};
use super::paths::AppPaths;
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum InstallEvent {
    Progress {
        label: &'static str,
        received: u64,
        total: Option<u64>,
    },
    Extracting {
        label: &'static str,
    },
    Finished,
    Failed(String),
}

pub struct InstallHandle {
    pub events: Receiver<InstallEvent>,
}

pub fn start_install(emu: &'static dyn Emulator, paths: &AppPaths) -> InstallHandle {
    let (tx, rx) = channel();
    let paths = paths.clone();
    std::thread::spawn(move || {
        let outcome = run_install(emu, &paths, &tx);
        let _ = match outcome {
            Ok(()) => tx.send(InstallEvent::Finished),
            Err(e) => tx.send(InstallEvent::Failed(format!("{e:#}"))),
        };
    });
    InstallHandle { events: rx }
}

fn run_install(
    emu: &'static dyn Emulator,
    paths: &AppPaths,
    tx: &Sender<InstallEvent>,
) -> Result<()> {
    let install_dir = emu.install_dir(paths);
    std::fs::create_dir_all(&install_dir)?;
    std::fs::create_dir_all(&paths.downloads_dir)?;

    for spec in emu.downloads() {
        let archive = download_verified(spec, &paths.downloads_dir, tx)
            .with_context(|| format!("downloading {}", spec.label))?;
        let _ = tx.send(InstallEvent::Extracting { label: spec.label });
        extract(&archive, spec, &install_dir)
            .with_context(|| format!("extracting {}", spec.label))?;
    }
    Ok(())
}

fn download_verified(
    spec: &DownloadSpec,
    downloads_dir: &Path,
    tx: &Sender<InstallEvent>,
) -> Result<PathBuf> {
    let file_name = spec.url.rsplit('/').next().unwrap_or("download.bin");
    let dest = downloads_dir.join(file_name);

    // Reuse a previously downloaded archive if its checksum still matches.
    if dest.exists() && file_sha256(&dest)? == spec.sha256 {
        return Ok(dest);
    }

    let mut response = ureq::get(spec.url).call()?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());

    let mut reader = response.body_mut().as_reader();
    let mut file = std::fs::File::create(&dest)?;
    let mut hasher = Sha256::new();
    let mut received: u64 = 0;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n])?;
        received += n as u64;
        let _ = tx.send(InstallEvent::Progress {
            label: spec.label,
            received,
            total,
        });
    }
    file.flush()?;
    drop(file);

    let digest = hex(&hasher.finalize());
    if digest != spec.sha256 {
        let _ = std::fs::remove_file(&dest);
        bail!(
            "checksum mismatch for {file_name}: expected {}, got {digest}",
            spec.sha256
        );
    }
    Ok(dest)
}

fn extract(archive: &Path, spec: &DownloadSpec, install_dir: &Path) -> Result<()> {
    match spec.kind {
        ArchiveKind::Zip => extract_zip(archive, spec.extract, install_dir),
        ArchiveKind::SevenZ => extract_7z(archive, spec.extract, install_dir),
    }
}

/// Map an archive entry path to its destination, honoring the extract rule
/// and rejecting absolute paths and traversal.
fn dest_for(entry_path: &str, rule: ExtractRule, install_dir: &Path) -> Option<PathBuf> {
    let entry = entry_path.replace('\\', "/");
    let rel = match rule {
        ExtractRule::All => entry.as_str(),
        ExtractRule::Subdir(prefix) => entry.strip_prefix(prefix)?.trim_start_matches('/'),
    };
    if rel.is_empty() {
        return None;
    }
    let path = Path::new(rel);
    let unsafe_component = path.components().any(|c| {
        matches!(
            c,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    });
    if unsafe_component {
        return None;
    }
    Some(install_dir.join(path))
}

fn extract_zip(archive: &Path, rule: ExtractRule, install_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let Some(dest) = dest_for(&name, rule, install_dir) else {
            continue;
        };
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&dest)?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

fn extract_7z(archive: &Path, rule: ExtractRule, install_dir: &Path) -> Result<()> {
    // Decompress fully into a staging dir next to the archive, then move the
    // wanted entries into place. Simple and independent of the 7z crate's
    // selective-extraction API.
    let staging = archive.with_extension("staging");
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    sevenz_rust2::decompress_file(archive, &staging).context("decompressing 7z archive")?;

    let source = match rule {
        ExtractRule::All => staging.clone(),
        ExtractRule::Subdir(prefix) => staging.join(prefix),
    };
    if !source.is_dir() {
        bail!("archive does not contain expected folder {source:?}");
    }
    copy_tree(&source, install_dir)?;
    std::fs::remove_dir_all(&staging)?;
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

fn file_sha256(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dest_for_honors_subdir_rule() {
        let base = Path::new("/tmp/emu");
        assert_eq!(
            dest_for("M2Emulator/dinput8.dll", ExtractRule::Subdir("M2Emulator"), base),
            Some(base.join("dinput8.dll"))
        );
        assert_eq!(
            dest_for("OtherGame/file.dll", ExtractRule::Subdir("M2Emulator"), base),
            None
        );
        assert_eq!(
            dest_for("EMULATOR.INI", ExtractRule::All, base),
            Some(base.join("EMULATOR.INI"))
        );
    }

    #[test]
    fn dest_for_rejects_traversal() {
        let base = Path::new("/tmp/emu");
        assert_eq!(dest_for("../evil.dll", ExtractRule::All, base), None);
        assert_eq!(dest_for("a/../../evil.dll", ExtractRule::All, base), None);
        assert_eq!(dest_for("/abs/path.dll", ExtractRule::All, base), None);
    }

    #[test]
    fn zip_roundtrip_extracts_files() -> Result<()> {
        use std::io::Write as _;
        use zip::write::SimpleFileOptions;

        let dir = std::env::temp_dir().join(format!("slipstream-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;

        let archive_path = dir.join("test.zip");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive_path)?);
        writer.start_file("EMULATOR.INI", SimpleFileOptions::default())?;
        writer.write_all(b"[Renderer]")?;
        writer.start_file("scripts/daytona.lua", SimpleFileOptions::default())?;
        writer.write_all(b"-- lua")?;
        writer.finish()?;

        let out = dir.join("out");
        extract_zip(&archive_path, ExtractRule::All, &out)?;
        assert_eq!(std::fs::read_to_string(out.join("EMULATOR.INI"))?, "[Renderer]");
        assert_eq!(
            std::fs::read_to_string(out.join("scripts/daytona.lua"))?,
            "-- lua"
        );

        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
