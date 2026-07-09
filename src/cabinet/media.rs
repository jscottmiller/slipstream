//! Game artwork for the cabinet UI, loaded lazily from the user's media
//! directory: `media/<game-id>/logo.png` and `screenshot.png` (jpg accepted
//! too). Missing files are normal — the scene draws typographic fallbacks —
//! and Slipstream never ships game artwork of its own.

use super::gfx::{Renderer, Texture};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Default)]
pub struct GameArt {
    pub logo: Option<Texture>,
    pub screenshot: Option<Texture>,
}

pub struct MediaCache {
    root: PathBuf,
    cache: HashMap<&'static str, GameArt>,
}

impl MediaCache {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            cache: HashMap::new(),
        }
    }

    pub fn art(&mut self, r: &Renderer, game_id: &'static str) -> &GameArt {
        self.cache.entry(game_id).or_insert_with(|| {
            let dir = self.root.join(game_id);
            GameArt {
                logo: load(r, &dir, "logo"),
                screenshot: load(r, &dir, "screenshot"),
            }
        })
    }
}

fn load(r: &Renderer, dir: &std::path::Path, stem: &str) -> Option<Texture> {
    for ext in ["png", "jpg", "jpeg"] {
        let path = dir.join(format!("{stem}.{ext}"));
        if !path.exists() {
            continue;
        }
        match image::open(&path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                match r.create_texture(w, h, &rgba) {
                    Ok(tex) => return Some(tex),
                    Err(e) => log::warn!("uploading {}: {e:#}", path.display()),
                }
            }
            Err(e) => log::warn!("decoding {}: {e}", path.display()),
        }
    }
    None
}
