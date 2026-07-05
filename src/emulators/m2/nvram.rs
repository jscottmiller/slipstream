//! Pre-seeded backup RAM (NVDATA) for games whose factory defaults block
//! solo play. Daytona's game assignments live in battery-backed SRAM; when
//! Link ID is MASTER/SLAVE the game demands the cabinet-link network board
//! ("NETWORK BOARD NOT PRESENT") and boot-loops. m2emulator stores this SRAM
//! as a zip archive (one deflated "SRAM" entry) at NVDATA/<rom>.DAT.
//!
//! The embedded image is a known-good single-cabinet configuration
//! (community preset by SkylarZYX, daytona-usa-script-utils). Byte 0x0B of
//! the SRAM — mirrored at 0x8B — encodes link mode: 00=SINGLE, 01=MASTER,
//! 02=SLAVE; 0x0C is the car number. Bytes 0x08/0x09 checksum the block, so
//! we ship the whole image instead of patching individual bytes.

use anyhow::{Context, Result};
use std::io::Read;
use std::path::Path;

static DAYTONA_SINGLE: &[u8] = include_bytes!("../../../assets/m2/daytona.nvram.dat");

const LINK_MODE_OFFSETS: [usize; 2] = [0x0B, 0x8B];
const LINK_SINGLE: u8 = 0x00;

pub fn for_game(game_id: &str) -> Option<&'static [u8]> {
    match game_id {
        "daytona" => Some(DAYTONA_SINGLE),
        _ => None,
    }
}

/// Ensure the game's NVRAM is configured for single-cabinet play. Writes the
/// embedded image when the file is missing, unreadable, or configured for
/// linked play; leaves it alone otherwise so calibration, settings tweaks,
/// and high scores survive.
pub fn ensure_single_link(nvdata_dir: &Path, rom_name: &str, image: &[u8]) -> Result<()> {
    let path = nvdata_dir.join(format!("{rom_name}.DAT"));
    if is_single_link(&path) {
        return Ok(());
    }
    std::fs::create_dir_all(nvdata_dir)?;
    std::fs::write(&path, image).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn is_single_link(path: &Path) -> bool {
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return false;
    };
    let Ok(mut entry) = archive.by_name("SRAM") else {
        return false;
    };
    let mut sram = Vec::new();
    if entry.read_to_end(&mut sram).is_err() {
        return false;
    }
    LINK_MODE_OFFSETS
        .iter()
        .all(|&off| sram.get(off) == Some(&LINK_SINGLE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_nvram(link_mode: u8) -> Vec<u8> {
        use std::io::Write as _;
        let mut sram = vec![0u8; 65536];
        for off in LINK_MODE_OFFSETS {
            sram[off] = link_mode;
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(&mut buf);
        writer
            .start_file("SRAM", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&sram).unwrap();
        writer.finish().unwrap();
        buf.into_inner()
    }

    #[test]
    fn embedded_daytona_image_is_single_link() {
        // Guards the asset itself: valid zip, SRAM entry, SINGLE in both
        // mirrored settings blocks.
        let dir = std::env::temp_dir().join(format!("slipstream-nvram-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("daytona.DAT");
        std::fs::write(&path, DAYTONA_SINGLE).unwrap();
        assert!(is_single_link(&path));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ensure_replaces_linked_config_and_keeps_single() {
        let dir = std::env::temp_dir().join(format!("slipstream-nvram2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("daytona.DAT");

        // Missing file → seeded.
        ensure_single_link(&dir, "daytona", DAYTONA_SINGLE).unwrap();
        assert!(is_single_link(&path));

        // MASTER config → replaced.
        std::fs::write(&path, make_nvram(0x01)).unwrap();
        ensure_single_link(&dir, "daytona", DAYTONA_SINGLE).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), DAYTONA_SINGLE);

        // Already-single (user-tweaked) config → untouched.
        let custom = make_nvram(0x00);
        std::fs::write(&path, &custom).unwrap();
        ensure_single_link(&dir, "daytona", DAYTONA_SINGLE).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), custom);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
