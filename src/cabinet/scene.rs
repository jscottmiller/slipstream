//! The carousel scene: full-bleed screenshot backdrop, metadata block, and
//! a logo rail along the bottom. Everything scales from a 1080p reference
//! layout so any target resolution keeps the same proportions.

use super::gfx::{Color, Rect, Renderer};
use super::media::MediaCache;
use super::text::{self, TextRenderer};
use crate::domain::game::GameDef;

/// One rail slot: a game plus whether its control-kind group is dimmed
/// (controller not detected while another kind is present).
pub struct Entry {
    pub game: &'static GameDef,
    pub dim: bool,
}

const ACCENT: Color = Color::rgb(0.05, 0.48, 0.95); // Sega blue, near enough
const AMBER: Color = Color::rgb(0.98, 0.75, 0.18);

/// Rail geometry, shared by drawing and mouse hit-testing.
pub struct RailLayout {
    pub margin: f32,
    pub slot_w: f32,
    pub slot_h: f32,
    pub gap: f32,
    pub rail_y: f32,
}

pub fn rail_layout(h: f32) -> RailLayout {
    let s = h / 1080.0;
    RailLayout {
        margin: 96.0 * s,
        slot_w: 300.0 * s,
        slot_h: 160.0 * s,
        gap: 28.0 * s,
        rail_y: h - 270.0 * s,
    }
}

impl RailLayout {
    pub fn slot_x(&self, i: usize, scroll: f32) -> f32 {
        self.margin + (i as f32 - scroll) * (self.slot_w + self.gap)
    }

    /// The rail slot under a point, if any.
    pub fn hit(&self, count: usize, scroll: f32, x: f32, y: f32) -> Option<usize> {
        if y < self.rail_y || y > self.rail_y + self.slot_h {
            return None;
        }
        (0..count).find(|&i| {
            let sx = self.slot_x(i, scroll);
            x >= sx && x <= sx + self.slot_w
        })
    }
}

pub struct Scene<'a> {
    pub entries: &'a [Entry],
    pub selected: usize,
    /// Animated carousel position, in units of selection index.
    pub scroll: f32,
    pub status: Option<&'a str>,
    /// Title of the running game, when one owns the screen.
    pub running: Option<&'a str>,
}

pub fn draw(
    r: &mut Renderer,
    t: &mut TextRenderer,
    m: &mut MediaCache,
    scene: &Scene,
    w: f32,
    h: f32,
) {
    let s = h / 1080.0; // reference-layout scale
    let game = scene.entries[scene.selected].game;

    // Backdrop: the selected game's screenshot, dimmed to keep text legible;
    // a quiet gradient when there is none.
    if let Some(shot) = &m.art(r, game.id).screenshot {
        let dst = cover(shot.w as f32 / shot.h as f32, w, h);
        r.image(shot, dst, Color::WHITE.dimmed(0.55));
    } else {
        r.rect_vgradient(
            Rect::new(0.0, 0.0, w, h),
            Color::rgb(0.07, 0.09, 0.14),
            Color::rgb(0.01, 0.01, 0.03),
        );
    }
    // Legibility vignette over the lower half.
    r.rect_vgradient(
        Rect::new(0.0, h * 0.45, w, h * 0.55),
        Color::rgba(0.0, 0.0, 0.0, 0.0),
        Color::rgba(0.0, 0.0, 0.0, 0.85),
    );

    // Metadata block.
    let margin = 96.0 * s;
    t.draw(
        r,
        text::BLACK,
        84.0 * s,
        (margin, h - 470.0 * s),
        Color::WHITE,
        game.title,
    );
    let meta = format!(
        "{} · {} · {} · {}",
        game.manufacturer,
        game.year,
        game.system.label(),
        game.controls.label()
    );
    t.draw(
        r,
        text::REGULAR,
        34.0 * s,
        (margin, h - 370.0 * s),
        Color::gray(0.78),
        &meta,
    );

    // Logo rail.
    let rail = rail_layout(h);
    let (slot_w, slot_h, rail_y) = (rail.slot_w, rail.slot_h, rail.rail_y);
    let slot_x = |i: usize| rail.slot_x(i, scene.scroll);

    // Group headers: one per control-kind run, pinned to the left margin
    // while its group occupies it, pushed out by the next group's arrival —
    // so the current group is always labeled even mid-scroll.
    let starts: Vec<usize> = scene
        .entries
        .iter()
        .enumerate()
        .filter(|(i, e)| *i == 0 || scene.entries[i - 1].game.controls != e.game.controls)
        .map(|(i, _)| i)
        .collect();
    for (run, &start) in starts.iter().enumerate() {
        let entry = &scene.entries[start];
        let label = if entry.dim {
            format!("{} — NOT DETECTED", entry.game.controls.label().to_uppercase())
        } else {
            entry.game.controls.label().to_uppercase()
        };
        let label_w = t.measure(text::BOLD, 22.0 * s, &label);
        let x_end = starts
            .get(run + 1)
            .map(|&n| slot_x(n) - label_w - 40.0 * s)
            .unwrap_or(f32::INFINITY);
        let x = slot_x(start).max(margin).min(x_end);
        if x < w && x_end > 0.0 {
            t.draw(
                r,
                text::BOLD,
                22.0 * s,
                (x, rail_y - 46.0 * s),
                Color::gray(if entry.dim { 0.35 } else { 0.55 }),
                &label,
            );
        }
    }

    for (i, entry) in scene.entries.iter().enumerate() {
        let g = entry.game;
        let x = slot_x(i);
        if x + slot_w < -slot_w || x > w + slot_w {
            continue;
        }

        let is_selected = i == scene.selected;
        let dim_factor = if entry.dim { 0.45 } else { 1.0 };
        let (slot, bright) = if is_selected {
            let grow = 0.12;
            (
                Rect::new(
                    x - slot_w * grow / 2.0,
                    rail_y - slot_h * grow / 2.0,
                    slot_w * (1.0 + grow),
                    slot_h * (1.0 + grow),
                ),
                dim_factor,
            )
        } else {
            (Rect::new(x, rail_y, slot_w, slot_h), 0.55 * dim_factor)
        };

        if let Some(logo) = &m.art(r, g.id).logo {
            let dst = fit(logo.w as f32 / logo.h as f32, slot);
            r.image(logo, dst, Color::WHITE.dimmed(bright));
        } else {
            // Typographic fallback: a dark panel with the title. The panel
            // stays dark regardless of selection; brightness lives in the
            // text and the accent bar so the rail reads consistently.
            r.rect(slot, Color::rgb(0.06, 0.07, 0.09).with_alpha(0.92));
            let text_bright = if is_selected { 1.0 } else { 0.62 };
            let px = 30.0 * s;
            let title_w = t.measure(text::BOLD, px, g.title);
            let scale = (1.0f32).min((slot.w - 24.0 * s) / title_w);
            t.draw(
                r,
                text::BOLD,
                px * scale,
                (
                    slot.x + (slot.w - title_w * scale) / 2.0,
                    slot.y + slot.h / 2.0 - px * scale * 0.6,
                ),
                Color::gray(0.88).dimmed(text_bright),
                g.title,
            );
        }

        if is_selected {
            r.rect(
                Rect::new(slot.x, slot.y + slot.h + 10.0 * s, slot.w, 6.0 * s),
                ACCENT,
            );
        }
    }

    // Hints and status.
    let hint = "Steer or arrows: browse    A or Enter: launch    Esc: exit";
    let hint_w = t.measure(text::REGULAR, 24.0 * s, hint);
    t.draw(
        r,
        text::REGULAR,
        24.0 * s,
        (w - margin - hint_w, h - 60.0 * s),
        Color::gray(0.45),
        hint,
    );
    if let Some(status) = scene.status {
        t.draw(
            r,
            text::BOLD,
            26.0 * s,
            (margin, h - 62.0 * s),
            AMBER,
            status,
        );
    }

    // Running overlay: the emulator owns the screen; this is what you see
    // for a moment on the way in and out.
    if let Some(title) = scene.running {
        r.rect(Rect::new(0.0, 0.0, w, h), Color::rgba(0.0, 0.0, 0.0, 0.78));
        let px = 64.0 * s;
        let tw = t.measure(text::BLACK, px, title);
        t.draw(
            r,
            text::BLACK,
            px,
            ((w - tw) / 2.0, h / 2.0 - 80.0 * s),
            Color::WHITE,
            title,
        );
        let sub = "Running — Xbox button or Escape quits";
        let sw = t.measure(text::REGULAR, 30.0 * s, sub);
        t.draw(
            r,
            text::REGULAR,
            30.0 * s,
            ((w - sw) / 2.0, h / 2.0 + 20.0 * s),
            Color::gray(0.7),
            sub,
        );
    }
}

/// Scale to fill `w`×`h` entirely (cropping overflow), centered.
fn cover(aspect: f32, w: f32, h: f32) -> Rect {
    let scale = (w / aspect).max(h);
    let (dw, dh) = (scale * aspect, scale);
    Rect::new((w - dw) / 2.0, (h - dh) / 2.0, dw, dh)
}

/// Scale to fit inside `slot` (letterboxing), centered.
fn fit(aspect: f32, slot: Rect) -> Rect {
    let scale = (slot.w / aspect).min(slot.h);
    let (dw, dh) = (scale * aspect, scale);
    Rect::new(
        slot.x + (slot.w - dw) / 2.0,
        slot.y + (slot.h - dh) / 2.0,
        dw,
        dh,
    )
}
