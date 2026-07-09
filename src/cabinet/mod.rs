//! The cabinet UI: a fullscreen, wheel-navigable game carousel rendered
//! with SDL3 + OpenGL. It runs at the configured game resolution in
//! exclusive fullscreen (a real hardware display mode, so launcher and
//! game share one mode), falling back to a plain window where no mode
//! matches — which is exactly what developing under WSLg looks like.

mod gfx;
mod input;
mod media;
mod scene;
mod text;

use crate::domain::game::GAMES;
use crate::domain::launch::{self, RunningGame};
use crate::domain::paths::AppPaths;
use crate::domain::settings::Settings;
use crate::domain::wheel;
use anyhow::{anyhow, bail, Context, Result};
use sdl3::event::Event;
use sdl3::video::GLProfile;
use std::time::{Duration, Instant};

pub fn run() -> Result<()> {
    let paths = AppPaths::resolve()?;
    let settings = Settings::load(&paths);
    let wheel = wheel::find(&settings.wheel_id);
    if GAMES.is_empty() {
        bail!("no games registered");
    }

    let sdl = sdl3::init().map_err(|e| anyhow!("SDL init: {e}"))?;
    let video = sdl.video().map_err(|e| anyhow!("SDL video: {e}"))?;
    let joystick = sdl.joystick().map_err(|e| anyhow!("SDL joystick: {e}"))?;

    // Windows denies foreground to background processes, so when an
    // emulator exits, raise() alone leaves focus on the desktop. This hint
    // makes SDL_RaiseWindow perform the documented AttachThreadInput
    // workaround and actually take the display back.
    sdl3::hint::set("SDL_FORCE_RAISEWINDOW", "1");

    let gl_attr = video.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_version(3, 3);

    // Exclusive fullscreen wants a hardware mode matching the game target;
    // without one (WSLg, odd monitors) run windowed at the same size.
    let (w, h) = (settings.screen_width, settings.screen_height);
    let display = video
        .get_primary_display()
        .map_err(|e| anyhow!("primary display: {e}"))?;
    let mode = display
        .get_fullscreen_modes()
        .map_err(|e| anyhow!("display modes: {e}"))?
        .into_iter()
        .filter(|m| m.w == w as i32 && m.h == h as i32)
        .max_by(|a, b| a.refresh_rate.total_cmp(&b.refresh_rate));
    if mode.is_none() {
        log::warn!("no {w}x{h} display mode; running windowed");
    }

    let mut builder = video.window("Slipstream", w, h);
    builder.opengl();
    if mode.is_some() {
        builder.fullscreen();
    }
    let mut window = builder.build().context("creating window")?;
    if let Some(mode) = mode {
        window
            .set_display_mode(mode)
            .map_err(|e| anyhow!("setting display mode: {e}"))?;
    }

    let _gl_ctx = window
        .gl_create_context()
        .map_err(|e| anyhow!("GL context: {e}"))?;
    if let Err(e) = video.gl_set_swap_interval(sdl3::video::SwapInterval::VSync) {
        log::warn!("vsync unavailable: {e}");
    }
    let gl = unsafe {
        glow::Context::from_loader_function(|s| match video.gl_get_proc_address(s) {
            Some(f) => f as *const std::ffi::c_void,
            None => std::ptr::null(),
        })
    };

    let mut renderer = gfx::Renderer::new(gl)?;
    let mut fonts = text::TextRenderer::new(&renderer)?;
    let mut art = media::MediaCache::new(paths.media_dir.clone());

    // Joysticks only deliver events while held open; keep handles alive and
    // pick up hot-plugged devices (wheel switched on after the launcher).
    let mut sticks = Vec::new();
    for id in joystick.joysticks().unwrap_or_default() {
        if let Ok(stick) = joystick.open(id) {
            sticks.push(stick);
        }
    }

    let mut selected = 0usize;
    let mut scroll = 0.0f32;
    let mut status: Option<String> = None;
    let mut running: Option<RunningGame> = None;
    let mut events = sdl
        .event_pump()
        .map_err(|e| anyhow!("SDL event pump: {e}"))?;
    let mut last_frame = Instant::now();

    // Dev hook: SLIPSTREAM_SHOT=<path.png> renders a few frames, saves the
    // framebuffer, and exits — visual iteration without a display to watch.
    let shot: Option<std::path::PathBuf> = std::env::var_os("SLIPSTREAM_SHOT").map(Into::into);
    if shot.is_some() {
        // SLIPSTREAM_SHOT_INDEX picks which game the shot has selected.
        if let Some(i) = std::env::var("SLIPSTREAM_SHOT_INDEX")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
        {
            selected = i.min(GAMES.len() - 1);
            scroll = selected as f32;
        }
    }
    let mut frame: u32 = 0;

    'main: loop {
        for event in events.poll_iter() {
            match &event {
                Event::Quit { .. } => break 'main,
                Event::JoyDeviceAdded { which, .. } => {
                    if let Ok(stick) = joystick.open(sdl3::sys::joystick::SDL_JoystickID(*which)) {
                        sticks.push(stick);
                    }
                }
                _ => {}
            }
            // While a game runs it owns all input; the quit watcher handles
            // getting back here.
            if running.is_some() {
                continue;
            }
            match input::map(&event, wheel) {
                Some(input::Nav::Prev) => selected = selected.saturating_sub(1),
                Some(input::Nav::Next) => selected = (selected + 1).min(GAMES.len() - 1),
                Some(input::Nav::Select) => {
                    match launch::launch(&GAMES[selected], &settings, &paths) {
                        Ok(game) => {
                            status = None;
                            running = Some(game);
                        }
                        Err(e) => status = Some(format!("Launch failed: {e:#}")),
                    }
                }
                Some(input::Nav::Back) => break 'main,
                None => {}
            }
        }

        if let Some(game) = &running {
            if !game.is_running() {
                running = None;
                // Exclusive fullscreen minimizes on focus loss; take the
                // display back now that the emulator is gone.
                window.restore();
                window.raise();
            }
        }

        let dt = last_frame.elapsed().as_secs_f32().min(0.1);
        last_frame = Instant::now();
        scroll += (selected as f32 - scroll) * (dt * 10.0).min(1.0);

        let (dw, dh) = window.size();
        renderer.begin(dw, dh, gfx::Color::rgb(0.01, 0.01, 0.03));
        scene::draw(
            &mut renderer,
            &mut fonts,
            &mut art,
            &scene::Scene {
                games: GAMES,
                selected,
                scroll,
                status: status.as_deref(),
                running: running.as_ref().map(|g| g.game.title),
            },
            dw as f32,
            dh as f32,
        );
        renderer.end();

        // Dev screenshot: read the back buffer before the swap makes its
        // contents undefined.
        frame += 1;
        if let Some(path) = &shot {
            if frame >= 5 {
                save_framebuffer(&renderer, dw, dh, path)?;
                break 'main;
            }
        }
        window.gl_swap_window();
        // Vsync paces us when it works; this caps the WSLg/windowed case,
        // and there's no reason to spin while an emulator owns the screen.
        let idle = if running.is_some() { 100 } else { 8 };
        std::thread::sleep(Duration::from_millis(idle));
    }
    Ok(())
}

fn save_framebuffer(r: &gfx::Renderer, w: u32, h: u32, path: &std::path::Path) -> Result<()> {
    let pixels = r.read_pixels(w, h);
    // GL reads bottom-up; flip rows for the image file.
    let stride = (w * 4) as usize;
    let mut flipped = Vec::with_capacity(pixels.len());
    for row in pixels.chunks_exact(stride).rev() {
        flipped.extend_from_slice(row);
    }
    image::save_buffer(path, &flipped, w, h, image::ColorType::Rgba8)?;
    log::info!("saved frame to {}", path.display());
    Ok(())
}
