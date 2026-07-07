use super::{game::GameDef, paths::AppPaths, settings::Settings, wheel::WheelProfile};
use anyhow::Result;
use std::path::PathBuf;
use std::process::Child;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    Zip,
    SevenZ,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExtractRule {
    /// Extract every entry into the emulator directory.
    All,
    /// Extract only entries under this top-level archive folder, with the
    /// prefix stripped.
    Subdir(&'static str),
}

pub struct DownloadSpec {
    pub label: &'static str,
    pub url: &'static str,
    /// Hex-encoded SHA-256 of the archive.
    pub sha256: &'static str,
    pub kind: ArchiveKind,
    pub extract: ExtractRule,
}

pub trait Emulator: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    /// Components fetched when the user explicitly installs this emulator.
    fn downloads(&self) -> &'static [DownloadSpec];
    fn install_dir(&self, paths: &AppPaths) -> PathBuf {
        paths.emulator_dir(self.id())
    }
    fn is_installed(&self, paths: &AppPaths) -> bool;
    /// Write all configuration (video, ROM paths, controls, force feedback)
    /// so the given game is ready to play on the given wheel.
    fn configure(
        &self,
        game: &GameDef,
        settings: &Settings,
        wheel: &WheelProfile,
        paths: &AppPaths,
    ) -> Result<()>;
    fn launch(&self, game: &GameDef, settings: &Settings, paths: &AppPaths) -> Result<Child>;
    /// True when the emulator has no quit key of its own and the launcher
    /// should close it on Escape (graceful window close while it holds the
    /// foreground).
    fn needs_escape_quit(&self) -> bool {
        false
    }
}

pub static EMULATORS: &[&dyn Emulator] = &[
    &crate::emulators::m2::M2Emulator,
    &crate::emulators::supermodel::SupermodelEmulator,
];

pub fn find(id: &str) -> Option<&'static dyn Emulator> {
    EMULATORS.iter().copied().find(|e| e.id() == id)
}
