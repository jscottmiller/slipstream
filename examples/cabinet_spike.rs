//! Cross-build spike for the cabinet UI stack: proves SDL3 (statically
//! linked, built from source) can create an exclusive-fullscreen window at
//! an explicit display mode with a GL context, and that joysticks are
//! visible. Run on the Windows box; on success it flashes a clear color for
//! two seconds and prints what it found.

use sdl3::event::Event;
use std::time::{Duration, Instant};

const TARGET_W: i32 = 1920;
const TARGET_H: i32 = 1080;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdl = sdl3::init()?;
    let video = sdl.video()?;
    let joystick = sdl.joystick()?;

    let display = video.get_primary_display()?;
    let modes = display.get_fullscreen_modes()?;
    for mode in &modes {
        println!("mode {}x{} @{}Hz", mode.w, mode.h, mode.refresh_rate);
    }
    // Exclusive fullscreen at the game target resolution: the highest
    // refresh among hardware modes matching it exactly.
    let target = modes
        .into_iter()
        .filter(|m| m.w == TARGET_W && m.h == TARGET_H)
        .max_by(|a, b| a.refresh_rate.total_cmp(&b.refresh_rate))
        .ok_or("no display mode matches the game target resolution")?;
    println!(
        "using {}x{} @{}Hz",
        target.w, target.h, target.refresh_rate
    );

    let mut window = video
        .window("slipstream spike", TARGET_W as u32, TARGET_H as u32)
        .fullscreen() // exclusive once the explicit mode is applied below
        .opengl()
        .build()?;
    window.set_display_mode(target)?;

    let _gl = window.gl_create_context()?;

    for id in joystick.joysticks()? {
        let name = joystick.open(id).map(|j| j.name());
        println!("joystick: {name:?}");
    }

    let mut events = sdl.event_pump()?;
    let deadline = Instant::now() + Duration::from_secs(2);
    'run: while Instant::now() < deadline {
        for event in events.poll_iter() {
            if let Event::Quit { .. } = event {
                break 'run;
            }
        }
        window.gl_swap_window();
        std::thread::sleep(Duration::from_millis(16));
    }

    println!("spike ok");
    Ok(())
}
