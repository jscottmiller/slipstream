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
    /// Non-fatal problem worth showing (e.g. gun reload needs admin).
    pub warning: Option<String>,
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

    // Pre-flight the ROM sets: a missing zip becomes a one-line status
    // instead of the emulator's own error wall.
    if let Some(rom_dir) = &settings.rom_dir {
        let missing: Vec<String> = emu
            .required_rom_sets(game)
            .iter()
            .map(|stem| format!("{stem}.zip"))
            .filter(|zip| !rom_dir.join(zip).exists())
            .collect();
        if !missing.is_empty() {
            anyhow::bail!(
                "missing ROM set{}: {} (in {})",
                if missing.len() > 1 { "s" } else { "" },
                missing.join(", "),
                rom_dir.display()
            );
        }
    }

    emu.configure(game, settings, wheel, paths)?;
    // Companions first, so process hooks are already waiting when the
    // emulator starts; the quit watcher reaps them after it exits.
    let warning = emu.launch_warning(game, paths);
    let companions = emu.launch_companions(game, paths);
    let child = emu.launch(game, settings, paths)?;
    let watcher = quit_watcher::watch(child, companions, wheel, emu.needs_escape_quit());
    Ok(RunningGame {
        game,
        warning,
        watcher,
    })
}
