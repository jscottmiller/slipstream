use super::paths::AppPaths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Directory containing the user's own ROM sets (e.g. daytona.zip).
    pub rom_dir: Option<PathBuf>,
    pub wheel_id: String,
    /// The wheel's 1-based DirectInput device number. Stays 1 unless other
    /// game controllers (including phantom ones — some Razer keyboards
    /// register as gamepads) enumerate ahead of the wheel.
    pub wheel_pad: u8,
    pub fullscreen: bool,
    pub screen_width: u32,
    pub screen_height: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rom_dir: None,
            wheel_id: "logitech-g923".to_string(),
            wheel_pad: 1,
            fullscreen: true,
            screen_width: 1920,
            screen_height: 1080,
        }
    }
}

impl Settings {
    pub fn load(paths: &AppPaths) -> Self {
        let mut settings: Settings = match std::fs::read_to_string(&paths.config_file) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                log::warn!("config.toml is invalid ({e}); using defaults");
                Self::default()
            }),
            Err(_) => Self::default(),
        };
        // A relative rom_dir (e.g. 'roms' in a portable folder) is anchored
        // at the config file's directory, keeping the folder relocatable.
        if let (Some(dir), Some(base)) = (&mut settings.rom_dir, paths.config_file.parent()) {
            if dir.is_relative() {
                *dir = base.join(&*dir);
            }
        }
        settings
    }

    pub fn save(&self, paths: &AppPaths) -> Result<()> {
        if let Some(dir) = paths.config_file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        // Store the rom_dir relative when it lives inside the config's
        // folder, so portable installs survive being moved.
        let mut on_disk = self.clone();
        if let (Some(dir), Some(base)) = (&mut on_disk.rom_dir, paths.config_file.parent()) {
            if let Ok(rel) = dir.strip_prefix(base) {
                *dir = rel.to_path_buf();
            }
        }
        let text = toml::to_string_pretty(&on_disk).context("serializing settings")?;
        std::fs::write(&paths.config_file, text).context("writing config.toml")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_windows_style_config() {
        let text = r#"
rom_dir = 'D:\Arcade ROMs'
wheel_id = "logitech-g923"
fullscreen = true
screen_width = 1920
screen_height = 1080
"#;
        let settings: Settings = toml::from_str(text).unwrap();
        assert_eq!(
            settings.rom_dir.as_deref(),
            Some(std::path::Path::new(r"D:\Arcade ROMs"))
        );
        assert_eq!(settings.wheel_id, "logitech-g923");
        assert!(settings.fullscreen);
        // Absent in older configs; must default rather than fail.
        assert_eq!(settings.wheel_pad, 1);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let settings: Settings = toml::from_str("").unwrap();
        assert!(settings.rom_dir.is_none());
        assert_eq!(settings.wheel_id, "logitech-g923");
        assert_eq!((settings.screen_width, settings.screen_height), (1920, 1080));
    }

    #[test]
    fn portable_roundtrip_keeps_rom_dir_relative() {
        let root = std::env::temp_dir().join(format!("slipstream-portable-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let paths = AppPaths::portable(&root);

        std::fs::write(&paths.config_file, "rom_dir = 'roms'\n").unwrap();
        let settings = Settings::load(&paths);
        // Relative rom_dir resolves against the portable root...
        assert_eq!(settings.rom_dir.as_deref(), Some(root.join("roms").as_path()));

        // ...and saving writes it back relative, so the folder can move.
        settings.save(&paths).unwrap();
        let text = std::fs::read_to_string(&paths.config_file).unwrap();
        assert!(text.contains("rom_dir = \"roms\""), "got: {text}");

        // A rom_dir outside the root stays absolute.
        let outside = Settings {
            rom_dir: Some(std::env::temp_dir()),
            ..Default::default()
        };
        outside.save(&paths).unwrap();
        let reloaded = Settings::load(&paths);
        assert_eq!(reloaded.rom_dir.as_deref(), Some(std::env::temp_dir().as_path()));

        std::fs::remove_dir_all(&root).unwrap();
    }
}
