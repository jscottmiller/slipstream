//! Binary writer for m2emulator's per-game `CFG/<rom>.input` control files.
//!
//! Format (reverse-engineered; see
//! <https://gist.github.com/andersstorhaug/38ceadbae32790c08c8b130cb4a8486b>
//! and RetroBat's Model2 generator, which produces these files the same way):
//! one 4-byte entry per input slot in the game's display order, followed by
//! an 8-byte footer of analog-enable flags (also display order).
//!
//! Entry encodings:
//! - keyboard: `[scan_code, 0, 0, 0]` (DirectInput scan code)
//! - button:   `[button << 4, pad, 0, 0]` (1-based button, 1-based pad)
//! - hat:      `[hat * 4 + dir, pad, 0, 0]` with dir 0=left 1=right 2=up 3=down
//! - axis:     `[axis, invert << 4 | pad, 0x00, 0xFF]` with axis
//!   0=X 1=Y 2=RZ 3=Z 4=RX 5=RY 6=S1 7=S2

use crate::domain::game::GameDef;
use crate::domain::wheel::{AxisBinding, DiAxis, HatDir, WheelProfile};

#[derive(Clone, Copy)]
#[allow(dead_code)] // Key is part of the file format, unused by current layouts
pub enum Binding {
    None,
    /// DirectInput keyboard scan code.
    Key(u8),
    /// 1-based pad and button numbers (this encoding fits buttons 1-15).
    Button { pad: u8, number: u8 },
    /// Direction on the pad's first hat (the d-pad).
    Hat { pad: u8, dir: HatDir },
    Axis { pad: u8, binding: AxisBinding },
}

impl Binding {
    fn encode(self) -> [u8; 4] {
        match self {
            Binding::None => [0, 0, 0, 0],
            Binding::Key(scan) => [scan, 0, 0, 0],
            Binding::Button { pad, number } => [(number & 0x0F) << 4, pad & 0x0F, 0, 0],
            Binding::Hat { pad, dir } => {
                let code = match dir {
                    HatDir::Left => 0,
                    HatDir::Right => 1,
                    HatDir::Up => 2,
                    HatDir::Down => 3,
                };
                [code, pad & 0x0F, 0, 0]
            }
            Binding::Axis { pad, binding } => {
                let code = match binding.axis {
                    DiAxis::X => 0x00,
                    DiAxis::Y => 0x01,
                    DiAxis::RZ => 0x02,
                    DiAxis::Z => 0x03,
                    DiAxis::RX => 0x04,
                    DiAxis::RY => 0x05,
                    DiAxis::Slider1 => 0x06,
                    DiAxis::Slider2 => 0x07,
                };
                [code, ((binding.inverted as u8) << 4) | (pad & 0x0F), 0x00, 0xFF]
            }
        }
    }
}

fn encode_file(slots: &[Binding], analog_flags: &[u8; 8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(slots.len() * 4 + 8);
    for slot in slots {
        bytes.extend_from_slice(&slot.encode());
    }
    bytes.extend_from_slice(analog_flags);
    bytes
}

/// Returns the control file contents for a game, or None if the game has no
/// layout defined yet.
pub fn for_game(game: &GameDef, wheel: &WheelProfile) -> Option<Vec<u8>> {
    match game.id {
        "daytona" => Some(daytona(wheel)),
        _ => None,
    }
}

/// Daytona USA: 24 input slots in the emulator's display order (layout
/// cross-checked against RetroBat's generator):
///   0-3   menu navigation up/down/left/right
///   4-6   steering, accelerate, brake (analog)
///   7-10  gears 1-4
///   11    gear neutral (left unbound so the d-pad latches the chosen gear)
///   12-15 VR view buttons 1-4
///   16    start
///   17    coin
///   18-23 service/test block, left unbound
/// Footer: analog flags for steering/accelerate/brake.
fn daytona(wheel: &WheelProfile) -> Vec<u8> {
    const PAD: u8 = 1;
    let mut slots = [Binding::None; 24];

    slots[0] = Binding::Hat { pad: PAD, dir: HatDir::Up };
    slots[1] = Binding::Hat { pad: PAD, dir: HatDir::Down };
    slots[2] = Binding::Hat { pad: PAD, dir: HatDir::Left };
    slots[3] = Binding::Hat { pad: PAD, dir: HatDir::Right };

    slots[4] = Binding::Axis { pad: PAD, binding: wheel.steering };
    slots[5] = Binding::Axis { pad: PAD, binding: wheel.accelerator };
    slots[6] = Binding::Axis { pad: PAD, binding: wheel.brake };

    for (i, dir) in wheel.gears.iter().enumerate() {
        slots[7 + i] = Binding::Hat { pad: PAD, dir: *dir };
    }

    for (i, button) in wheel.vr_buttons.iter().enumerate() {
        slots[12 + i] = Binding::Button { pad: PAD, number: *button };
    }

    slots[16] = Binding::Button { pad: PAD, number: wheel.btn_start };
    slots[17] = Binding::Button { pad: PAD, number: wheel.btn_coin };

    encode_file(&slots, &[1, 1, 1, 0, 0, 0, 0, 0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::game::GAMES;
    use crate::domain::wheel::LOGITECH_G923;

    #[test]
    fn daytona_g923_golden_bytes() {
        let bytes = daytona(&LOGITECH_G923);
        assert_eq!(bytes.len(), 104, "24 slots * 4 bytes + 8-byte footer");

        // Menu navigation on the d-pad hat: up, down, left, right.
        assert_eq!(&bytes[0..4], &[0x02, 0x01, 0x00, 0x00]);
        assert_eq!(&bytes[4..8], &[0x03, 0x01, 0x00, 0x00]);
        assert_eq!(&bytes[8..12], &[0x00, 0x01, 0x00, 0x00]);
        assert_eq!(&bytes[12..16], &[0x01, 0x01, 0x00, 0x00]);

        // Steering: X axis, pad 1, not inverted.
        assert_eq!(&bytes[16..20], &[0x00, 0x01, 0x00, 0xFF]);
        // Accelerator: Y axis, pad 1, inverted.
        assert_eq!(&bytes[20..24], &[0x01, 0x11, 0x00, 0xFF]);
        // Brake: RZ axis, pad 1, inverted.
        assert_eq!(&bytes[24..28], &[0x02, 0x11, 0x00, 0xFF]);

        // Gears 1-4 clockwise on the hat: up, right, down, left.
        assert_eq!(&bytes[28..32], &[0x02, 0x01, 0x00, 0x00]);
        assert_eq!(&bytes[32..36], &[0x01, 0x01, 0x00, 0x00]);
        assert_eq!(&bytes[36..40], &[0x03, 0x01, 0x00, 0x00]);
        assert_eq!(&bytes[40..44], &[0x00, 0x01, 0x00, 0x00]);
        // Gear neutral unbound.
        assert_eq!(&bytes[44..48], &[0x00, 0x00, 0x00, 0x00]);

        // VR buttons 1-4 = Cross, Square, Circle, Triangle.
        assert_eq!(&bytes[48..52], &[0x10, 0x01, 0x00, 0x00]);
        assert_eq!(&bytes[52..56], &[0x20, 0x01, 0x00, 0x00]);
        assert_eq!(&bytes[56..60], &[0x30, 0x01, 0x00, 0x00]);
        assert_eq!(&bytes[60..64], &[0x40, 0x01, 0x00, 0x00]);

        // Start = Options (10), Coin = Share (9).
        assert_eq!(&bytes[64..68], &[0xA0, 0x01, 0x00, 0x00]);
        assert_eq!(&bytes[68..72], &[0x90, 0x01, 0x00, 0x00]);

        // Service/test block unbound.
        assert!(bytes[72..96].iter().all(|&b| b == 0));

        // Analog footer: steering, accelerate, brake enabled.
        assert_eq!(&bytes[96..], &[1, 1, 1, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn every_registered_game_has_a_layout() {
        for game in GAMES {
            assert!(
                for_game(game, &LOGITECH_G923).is_some(),
                "game {} is registered but has no m2 control layout",
                game.id
            );
        }
    }
}
