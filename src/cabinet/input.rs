//! Navigation events for the cabinet, unified across keyboard and wheel.
//! The wheel side leans on the profile: d-pad (POV hat) browses, and the
//! A/Cross face button or Start launches.

use crate::domain::wheel::WheelProfile;
use sdl3::event::Event;
use sdl3::joystick::HatState;
use sdl3::keyboard::Keycode;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nav {
    Prev,
    Next,
    /// Jump to the previous/next control-kind group in the rail.
    PrevGroup,
    NextGroup,
    Select,
    Back,
}

pub fn map(event: &Event, wheel: Option<&WheelProfile>) -> Option<Nav> {
    match event {
        Event::KeyDown {
            keycode: Some(key),
            repeat,
            ..
        } => match key {
            // Held arrows repeat (fast browsing); Enter must not.
            Keycode::Left => Some(Nav::Prev),
            Keycode::Right => Some(Nav::Next),
            Keycode::Up => Some(Nav::PrevGroup),
            Keycode::Down => Some(Nav::NextGroup),
            Keycode::Return if !repeat => Some(Nav::Select),
            Keycode::Escape if !repeat => Some(Nav::Back),
            _ => None,
        },
        Event::JoyHatMotion { state, .. } => match state {
            HatState::Left => Some(Nav::Prev),
            HatState::Right => Some(Nav::Next),
            HatState::Up => Some(Nav::PrevGroup),
            HatState::Down => Some(Nav::NextGroup),
            _ => None,
        },
        Event::JoyButtonDown { button_idx, .. } => {
            // SDL buttons are 0-based; profiles store 1-based DirectInput
            // numbers. Button 1 is A/Cross on both G923 variants.
            let start = wheel.map(|w| w.btn_start - 1);
            (*button_idx == 0 || Some(*button_idx) == start).then_some(Nav::Select)
        }
        _ => None,
    }
}
