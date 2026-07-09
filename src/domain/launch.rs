//! Launching games — shared by the desktop (egui) and cabinet (SDL) UIs:
//! resolve the emulator and wheel, regenerate configuration, spawn, and
//! attach the quit watcher.

use super::game::GameDef;
use super::paths::AppPaths;
use super::settings::Settings;
use super::{emulator, quit_watcher, wheel};
use anyhow::{Context, Result};
use std::sync::mpsc::{Receiver, TryRecvError};

/// A launched game. Dropping it detaches; the quit watcher keeps running.
pub struct RunningGame {
    pub game: &'static GameDef,
    watcher: Receiver<()>,
}

impl RunningGame {
    /// False once the emulator has exited (the watcher thread hangs up its
    /// end of the channel; nothing is ever sent on it).
    pub fn is_running(&self) -> bool {
        !matches!(self.watcher.try_recv(), Err(TryRecvError::Disconnected))
    }
}

pub fn launch(
    game: &'static GameDef,
    settings: &Settings,
    paths: &AppPaths,
) -> Result<RunningGame> {
    let emu = emulator::find(game.emulator_id).context("unknown emulator for game")?;
    let wheel = wheel::find(&settings.wheel_id).context("no wheel profile selected")?;
    emu.configure(game, settings, wheel, paths)?;
    let child = emu.launch(game, settings, paths)?;
    let watcher = quit_watcher::watch(child, wheel, emu.needs_escape_quit());
    Ok(RunningGame { game, watcher })
}
