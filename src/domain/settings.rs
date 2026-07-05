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
    pub fullscreen: bool,
    pub screen_width: u32,
    pub screen_height: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rom_dir: None,
            wheel_id: "logitech-g923".to_string(),
            fullscreen: true,
            screen_width: 1920,
            screen_height: 1080,
        }
    }
}

impl Settings {
    pub fn load(paths: &AppPaths) -> Self {
        match std::fs::read_to_string(&paths.config_file) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                log::warn!("config.toml is invalid ({e}); using defaults");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, paths: &AppPaths) -> Result<()> {
        if let Some(dir) = paths.config_file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).context("serializing settings")?;
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
rom_dir = 'F:\Sega Model 2'
wheel_id = "logitech-g923"
fullscreen = true
screen_width = 1920
screen_height = 1080
"#;
        let settings: Settings = toml::from_str(text).unwrap();
        assert_eq!(
            settings.rom_dir.as_deref(),
            Some(std::path::Path::new(r"F:\Sega Model 2"))
        );
        assert_eq!(settings.wheel_id, "logitech-g923");
        assert!(settings.fullscreen);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let settings: Settings = toml::from_str("").unwrap();
        assert!(settings.rom_dir.is_none());
        assert_eq!(settings.wheel_id, "logitech-g923");
        assert_eq!((settings.screen_width, settings.screen_height), (1920, 1080));
    }
}
