use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

/// Filesystem locations for everything Slipstream owns. On Windows these
/// land under %APPDATA% / %LOCALAPPDATA%, on Linux under XDG dirs.
#[derive(Clone)]
pub struct AppPaths {
    pub config_file: PathBuf,
    pub emulators_dir: PathBuf,
    pub downloads_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self> {
        let dirs = ProjectDirs::from("dev", "cowboyscott", "slipstream")
            .context("could not determine platform data directories")?;
        Ok(Self {
            config_file: dirs.config_dir().join("config.toml"),
            emulators_dir: dirs.data_local_dir().join("emulators"),
            downloads_dir: dirs.data_local_dir().join("downloads"),
        })
    }

    pub fn emulator_dir(&self, emulator_id: &str) -> PathBuf {
        self.emulators_dir.join(emulator_id)
    }
}
