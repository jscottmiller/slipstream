use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::path::{Path, PathBuf};

/// Filesystem locations for everything Slipstream owns.
///
/// Two modes:
/// - **Portable**: a `config.toml` sitting next to the executable makes the
///   exe's folder self-contained — config, emulators, and downloads all live
///   beside it, and the whole folder can be moved or copied.
/// - **Platform**: otherwise Windows %APPDATA% / %LOCALAPPDATA% (XDG dirs on
///   Linux).
#[derive(Clone)]
pub struct AppPaths {
    pub config_file: PathBuf,
    pub emulators_dir: PathBuf,
    pub downloads_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self> {
        if let Some(root) = portable_root() {
            return Ok(Self::portable(&root));
        }
        let dirs = ProjectDirs::from("dev", "cowboyscott", "slipstream")
            .context("could not determine platform data directories")?;
        Ok(Self {
            config_file: dirs.config_dir().join("config.toml"),
            emulators_dir: dirs.data_local_dir().join("emulators"),
            downloads_dir: dirs.data_local_dir().join("downloads"),
        })
    }

    pub fn portable(root: &Path) -> Self {
        Self {
            config_file: root.join("config.toml"),
            emulators_dir: root.join("emulators"),
            downloads_dir: root.join("downloads"),
        }
    }

    pub fn emulator_dir(&self, emulator_id: &str) -> PathBuf {
        self.emulators_dir.join(emulator_id)
    }
}

fn portable_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    dir.join("config.toml").exists().then(|| dir.to_path_buf())
}
