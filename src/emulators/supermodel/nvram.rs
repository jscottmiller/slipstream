//! Pre-seeded NVRAM for Model 3 games whose factory defaults block solo
//! play: their game assignments default to a linked-cabinet Link ID, making
//! the game demand the network board ("NETWORK BOARD NOT PRESENT"). Each
//! embedded image was captured from a real machine after setting Game
//! Assignments → Link ID → SINGLE via the test menu.
//!
//! NVRAM files are named after the game Supermodel identifies *inside* the
//! ROM zip, which can differ from our rom_name — scud.zip holds the
//! Australian set, so Supermodel reads and writes `scudau.nv`. Hence each
//! seed carries its own filename.
//!
//! Supermodel's .nv container embeds the emulator version and game name, so
//! images are per-game and should be revalidated whenever the pinned
//! Supermodel release is bumped. Unlike the m2 seeder we don't decode the
//! link byte out of the container, so existing files are left untouched —
//! seed only when missing.

use anyhow::{Context, Result};
use std::path::Path;

pub struct Seed {
    /// Filename stem Supermodel uses for this game's NVRAM.
    pub nv_name: &'static str,
    pub image: &'static [u8],
}

pub fn for_game(game_id: &str) -> Option<Seed> {
    let (nv_name, image): (_, &'static [u8]) = match game_id {
        "daytona2" => (
            "daytona2",
            include_bytes!("../../../assets/supermodel/daytona2.nvram.nv"),
        ),
        "dayto2pe" => (
            "dayto2pe",
            include_bytes!("../../../assets/supermodel/dayto2pe.nvram.nv"),
        ),
        "scud" => (
            "scudau",
            include_bytes!("../../../assets/supermodel/scudau.nvram.nv"),
        ),
        _ => return None,
    };
    Some(Seed { nv_name, image })
}

pub fn seed_if_missing(nvram_dir: &Path, seed: &Seed) -> Result<()> {
    let path = nvram_dir.join(format!("{}.nv", seed.nv_name));
    if path.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(nvram_dir)?;
    std::fs::write(&path, seed.image).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_images_are_wellformed() {
        for (game_id, nv_name) in [
            ("daytona2", "daytona2"),
            ("dayto2pe", "dayto2pe"),
            ("scud", "scudau"),
        ] {
            let seed = for_game(game_id).unwrap();
            assert_eq!(seed.nv_name, nv_name);
            assert!(seed.image.len() > 100_000, "{game_id} image too small");
            let has = |needle: &[u8]| seed.image.windows(needle.len()).any(|w| w == needle);
            assert!(has(b"Supermodel NVRAM State"), "{game_id} missing header");
            assert!(has(nv_name.as_bytes()), "{game_id} missing game name");
        }
    }

    #[test]
    fn seed_only_when_missing() {
        let dir = std::env::temp_dir().join(format!("slipstream-smnv-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let seed = for_game("scud").unwrap();

        seed_if_missing(&dir, &seed).unwrap();
        // Written under Supermodel's identified-game name, not rom_name.
        assert_eq!(std::fs::read(dir.join("scudau.nv")).unwrap(), seed.image);

        // An existing file (user settings, high scores) is preserved.
        std::fs::write(dir.join("scudau.nv"), b"user data").unwrap();
        seed_if_missing(&dir, &seed).unwrap();
        assert_eq!(std::fs::read(dir.join("scudau.nv")).unwrap(), b"user data");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
