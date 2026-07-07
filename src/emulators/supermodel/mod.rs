//! Supermodel — the Sega Model 3 emulator. Open source, actively developed,
//! with official Windows/Linux/macOS builds on GitHub. Configuration is a
//! plain-text `Config/Supermodel.ini` (not shipped in the archive, so
//! Slipstream owns it outright) and games launch by passing the ROM zip path
//! on the command line. Force feedback is built in (`ForceFeedback = 1`),
//! driven by the drive-board ROM inside each game's romset.

pub mod ini;

use crate::domain::emulator::{ArchiveKind, DownloadSpec, Emulator, ExtractRule};
use crate::domain::game::GameDef;
use crate::domain::paths::AppPaths;
use crate::domain::settings::Settings;
use crate::domain::wheel::WheelProfile;
use anyhow::{bail, Context, Result};
use std::process::{Child, Command};

pub const EXE_NAME: &str = "supermodel.exe";

pub struct SupermodelEmulator;

static DOWNLOADS: &[DownloadSpec] = &[DownloadSpec {
    label: "Supermodel 0.3a (2026-05-28)",
    url: "https://github.com/trzy/Supermodel/releases/download/v0.3a-20260528-git-77d28ee/supermodel-0.3a-20260528-git-77d28ee-windows.zip",
    sha256: "91e70f1e743b333db28b467c30a3d2f5c92948063a91d55207e578ee8543a87b",
    kind: ArchiveKind::Zip,
    // The archive wraps everything in a versioned folder.
    extract: ExtractRule::Subdir("supermodel-0.3a-20260528-git-77d28ee"),
}];

impl Emulator for SupermodelEmulator {
    fn id(&self) -> &'static str {
        "supermodel"
    }

    fn name(&self) -> &'static str {
        "Supermodel"
    }

    fn downloads(&self) -> &'static [DownloadSpec] {
        DOWNLOADS
    }

    fn is_installed(&self, paths: &AppPaths) -> bool {
        self.install_dir(paths).join(EXE_NAME).exists()
    }

    fn configure(
        &self,
        _game: &GameDef,
        settings: &Settings,
        wheel: &WheelProfile,
        paths: &AppPaths,
    ) -> Result<()> {
        let dir = self.install_dir(paths);
        let config_dir = dir.join("Config");
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(
            config_dir.join("Supermodel.ini"),
            ini::supermodel_ini(wheel, settings.wheel_pad),
        )
        .context("writing Supermodel.ini")?;
        Ok(())
    }

    fn launch(&self, game: &GameDef, settings: &Settings, paths: &AppPaths) -> Result<Child> {
        let dir = self.install_dir(paths);
        let exe = dir.join(EXE_NAME);
        if !exe.exists() {
            bail!("{} is not installed", self.name());
        }
        let rom_dir = settings
            .rom_dir
            .as_ref()
            .context("ROM directory is not set (Settings → ROM directory)")?;
        let rom = rom_dir.join(format!("{}.zip", game.rom_name));

        let mut command = Command::new(&exe);
        command
            .arg(&rom)
            .arg(format!(
                "-res={},{}",
                settings.screen_width, settings.screen_height
            ))
            // Working dir must be the install dir so Config/, NVRAM/ and
            // Saves/ resolve.
            .current_dir(&dir);
        if settings.fullscreen {
            command.arg("-fullscreen");
        }
        command.spawn().context("failed to start Supermodel")
    }
}
