//! Pre-seeded NVRAM for Model 3 games whose factory defaults block solo
//! play. Daytona USA 2's game assignments default to a linked-cabinet Link
//! ID, making the game demand the network board ("NETWORK BOARD NOT
//! PRESENT") . The embedded image was captured from a real machine after
//! setting Game Assignments → Link ID → SINGLE via the test menu.
//!
//! Supermodel's .nv container embeds the emulator version and game name, so
//! images are per-game and should be revalidated whenever the pinned
//! Supermodel release is bumped. Unlike the m2 seeder we don't decode the
//! link byte out of the container, so existing files are left untouched —
//! seed only when missing.

use anyhow::{Context, Result};
use std::path::Path;

static DAYTONA2_SINGLE: &[u8] =
    include_bytes!("../../../assets/supermodel/daytona2.nvram.nv");

pub fn for_game(game_id: &str) -> Option<&'static [u8]> {
    match game_id {
        "daytona2" => Some(DAYTONA2_SINGLE),
        _ => None,
    }
}

pub fn seed_if_missing(nvram_dir: &Path, rom_name: &str, image: &[u8]) -> Result<()> {
    let path = nvram_dir.join(format!("{rom_name}.nv"));
    if path.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(nvram_dir)?;
    std::fs::write(&path, image).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_daytona2_image_is_wellformed() {
        assert!(DAYTONA2_SINGLE.len() > 100_000);
        let has = |needle: &[u8]| DAYTONA2_SINGLE.windows(needle.len()).any(|w| w == needle);
        assert!(has(b"Supermodel NVRAM State"));
        assert!(has(b"daytona2"));
    }

    #[test]
    fn seed_only_when_missing() {
        let dir = std::env::temp_dir().join(format!("slipstream-smnv-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        seed_if_missing(&dir, "daytona2", DAYTONA2_SINGLE).unwrap();
        assert_eq!(std::fs::read(dir.join("daytona2.nv")).unwrap(), DAYTONA2_SINGLE);

        // An existing file (user settings, high scores) is preserved.
        std::fs::write(dir.join("daytona2.nv"), b"user data").unwrap();
        seed_if_missing(&dir, "daytona2", DAYTONA2_SINGLE).unwrap();
        assert_eq!(std::fs::read(dir.join("daytona2.nv")).unwrap(), b"user data");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
