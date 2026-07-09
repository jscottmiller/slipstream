//! Glyph-atlas text rendering on top of the gfx renderer. Fonts are Lato
//! (OFL — see assets/fonts/LICENSE-Lato.txt), embedded so the exe stays
//! self-contained. Glyphs rasterize lazily into one shared RGBA atlas
//! (white, alpha = coverage) and are tinted by vertex color at draw time.

use super::gfx::{Color, Rect, Renderer, Texture};
use ab_glyph::{point, Font, FontArc, ScaleFont};
use anyhow::Result;
use std::collections::HashMap;

pub const REGULAR: usize = 0;
pub const BOLD: usize = 1;
pub const BLACK: usize = 2;

const FONT_BYTES: [&[u8]; 3] = [
    include_bytes!("../../assets/fonts/Lato-Regular.ttf"),
    include_bytes!("../../assets/fonts/Lato-Bold.ttf"),
    include_bytes!("../../assets/fonts/Lato-Black.ttf"),
];

const ATLAS_SIZE: u32 = 1024;
const PADDING: u32 = 1;

#[derive(Clone, Copy)]
struct Entry {
    uv: [f32; 4],
    w: f32,
    h: f32,
    // Offset from the pen position (at the baseline) to the bitmap's
    // top-left, in pixels.
    off_x: f32,
    off_y: f32,
}

pub struct TextRenderer {
    fonts: Vec<FontArc>,
    atlas: Texture,
    cache: HashMap<(usize, u16, u32), Option<Entry>>,
    cursor: (u32, u32),
    row_height: u32,
}

impl TextRenderer {
    pub fn new(r: &Renderer) -> Result<Self> {
        let fonts = FONT_BYTES
            .iter()
            .map(|bytes| FontArc::try_from_slice(bytes))
            .collect::<Result<Vec<_>, _>>()?;
        let atlas = r.create_texture(
            ATLAS_SIZE,
            ATLAS_SIZE,
            &vec![0u8; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize],
        )?;
        Ok(Self {
            fonts,
            atlas,
            cache: HashMap::new(),
            cursor: (PADDING, PADDING),
            row_height: 0,
        })
    }

    /// Draw a line of text with its top-left at the given position;
    /// returns the width.
    pub fn draw(
        &mut self,
        r: &mut Renderer,
        font: usize,
        px: f32,
        (x, y): (f32, f32),
        color: Color,
        text: &str,
    ) -> f32 {
        // FontArc clones are Arc-cheap; a clone frees `self` for the
        // cache-mutating entry() calls below.
        let font_arc = self.fonts[font].clone();
        let scaled = font_arc.as_scaled(px);
        let ascent = scaled.ascent();
        let mut pen = x;
        let mut prev = None;
        for ch in text.chars() {
            let id = scaled.font.glyph_id(ch);
            if let Some(p) = prev {
                pen += scaled.kern(p, id);
            }
            if let Some(entry) = self.entry(r, font, id, px) {
                r.quad_uv(
                    self.atlas.raw,
                    Rect::new(
                        pen + entry.off_x,
                        y + ascent + entry.off_y,
                        entry.w,
                        entry.h,
                    ),
                    entry.uv,
                    color,
                );
            }
            pen += scaled.h_advance(id);
            prev = Some(id);
        }
        pen - x
    }

    pub fn measure(&self, font: usize, px: f32, text: &str) -> f32 {
        let scaled = self.fonts[font].as_scaled(px);
        let mut width = 0.0;
        let mut prev = None;
        for ch in text.chars() {
            let id = scaled.font.glyph_id(ch);
            if let Some(p) = prev {
                width += scaled.kern(p, id);
            }
            width += scaled.h_advance(id);
            prev = Some(id);
        }
        width
    }

    /// Rasterize (or fetch) one glyph. None for whitespace and glyphs the
    /// font can't outline.
    fn entry(
        &mut self,
        r: &Renderer,
        font: usize,
        id: ab_glyph::GlyphId,
        px: f32,
    ) -> Option<Entry> {
        let key = (font, id.0, px.round() as u32);
        if let Some(cached) = self.cache.get(&key) {
            return *cached;
        }

        let glyph = id.with_scale_and_position(px, point(0.0, 0.0));
        let entry = self.fonts[font].outline_glyph(glyph).and_then(|outline| {
            let bounds = outline.px_bounds();
            let (w, h) = (bounds.width().ceil() as u32, bounds.height().ceil() as u32);
            if w == 0 || h == 0 || w + 2 * PADDING > ATLAS_SIZE {
                return None;
            }
            // Shelf packing: left to right, new row when full.
            if self.cursor.0 + w + PADDING > ATLAS_SIZE {
                self.cursor = (PADDING, self.cursor.1 + self.row_height + PADDING);
                self.row_height = 0;
            }
            if self.cursor.1 + h + PADDING > ATLAS_SIZE {
                log::error!("glyph atlas full; some text will not render");
                return None;
            }
            let (ax, ay) = self.cursor;
            self.cursor.0 += w + PADDING;
            self.row_height = self.row_height.max(h);

            let mut rgba = vec![0u8; (w * h * 4) as usize];
            outline.draw(|gx, gy, coverage| {
                if gx < w && gy < h {
                    let i = ((gy * w + gx) * 4) as usize;
                    rgba[i..i + 3].copy_from_slice(&[255, 255, 255]);
                    rgba[i + 3] = (coverage * 255.0) as u8;
                }
            });
            r.update_texture(&self.atlas, ax, ay, w, h, &rgba);

            let size = ATLAS_SIZE as f32;
            Some(Entry {
                uv: [
                    ax as f32 / size,
                    ay as f32 / size,
                    (ax + w) as f32 / size,
                    (ay + h) as f32 / size,
                ],
                w: w as f32,
                h: h as f32,
                off_x: bounds.min.x,
                off_y: bounds.min.y,
            })
        });
        self.cache.insert(key, entry);
        entry
    }
}
