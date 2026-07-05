use super::{emulator, game::GameDef, paths::AppPaths, settings::Settings};
use std::path::PathBuf;

pub struct GameStatus {
    pub emulator_installed: bool,
    pub rom_found: bool,
    pub rom_path: Option<PathBuf>,
}

impl GameStatus {
    pub fn ready(&self) -> bool {
        self.emulator_installed && self.rom_found
    }
}

pub fn game_status(game: &GameDef, settings: &Settings, paths: &AppPaths) -> GameStatus {
    let emulator_installed = emulator::find(game.emulator_id)
        .map(|e| e.is_installed(paths))
        .unwrap_or(false);
    let rom_path = settings
        .rom_dir
        .as_ref()
        .map(|dir| dir.join(format!("{}.zip", game.rom_name)));
    let rom_found = rom_path.as_ref().is_some_and(|p| p.exists());
    GameStatus {
        emulator_installed,
        rom_found,
        rom_path,
    }
}
