//! Binary writer for m2emulator's per-game `CFG/<rom>.input` control files.
//!
//! Format: one 4-byte entry per input slot in the game's display order,
//! followed by an 8-byte footer of analog-enable flags (also display order).
//! Layout reverse-engineered from files produced by the emulator's own
//! control-config dialog on real hardware (Logitech G923), cross-checked
//! against RetroBat's generator and
//! <https://gist.github.com/andersstorhaug/38ceadbae32790c08c8b130cb4a8486b>.
//!
//! Entry encodings:
//! - unbound:  `[0, 0, 0, 0]`
//! - keyboard: `[scan_code, 0, 0, 0]` (DirectInput scan code)
//! - button:   `[button << 4, pad, 0, 0]` (1-based button, 1-based pad)
//! - d-pad:    `[0x0C + dir, pad, 0, 0]` with dir 0=left 1=right 2=up 3=down.
//!   Codes 0x00-0x0B are the six axes as digital directions; the POV hat
//!   starts at 0x0C (the gist documents the axis-direction block as "hats").
//! - axis:     `[axis, invert << 4 | pad, min, max]` with axis
//!   0=X 1=Y 2=RZ 3=Z 4=RX 5=RY 6=S1 7=S2 and min/max the mapped range
//!   (0x00-0xFF for a whole axis; split ranges support combined pedals).

use crate::domain::game::GameDef;
use crate::domain::wheel::{AxisBinding, DiAxis, GearControl, HatDir, WheelProfile};

#[derive(Clone, Copy)]
#[allow(dead_code)] // Key is part of the file format, unused by current layouts
pub enum Binding {
    None,
    /// DirectInput keyboard scan code.
    Key(u8),
    /// 1-based pad and button numbers (this encoding fits buttons 1-15).
    Button { pad: u8, number: u8 },
    /// Direction on the pad's POV hat (the d-pad).
    Dpad { pad: u8, dir: HatDir },
    Axis { pad: u8, binding: AxisBinding },
}

impl Binding {
    fn encode(self) -> [u8; 4] {
        match self {
            Binding::None => [0, 0, 0, 0],
            Binding::Key(scan) => [scan, 0, 0, 0],
            Binding::Button { pad, number } => [(number & 0x0F) << 4, pad & 0x0F, 0, 0],
            Binding::Dpad { pad, dir } => {
                let offset = match dir {
                    HatDir::Left => 0,
                    HatDir::Right => 1,
                    HatDir::Up => 2,
                    HatDir::Down => 3,
                };
                [0x0C + offset, pad & 0x0F, 0, 0]
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

fn gear_binding(pad: u8, control: GearControl) -> Binding {
    match control {
        GearControl::Button(number) => Binding::Button { pad, number },
        GearControl::Dpad(dir) => Binding::Dpad { pad, dir },
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
/// layout defined yet. `pad` is the wheel's 1-based DirectInput device
/// number (1 unless other game controllers enumerate ahead of the wheel).
pub fn for_game(game: &GameDef, wheel: &WheelProfile, pad: u8) -> Option<Vec<u8>> {
    match game.id {
        "daytona" => Some(daytona(wheel, pad)),
        "srallyc" => Some(srallyc(wheel, pad)),
        _ => None,
    }
}

/// Shared prefix of the Model 2 driving games' input display order:
///   0-3   menu navigation up/down/left/right
///   4-6   steering, accelerate, brake (analog)
///   7-10  gears 1-4
///   11    gear neutral (left unbound so gears latch)
fn driving_common(slots: &mut [Binding], wheel: &WheelProfile, pad: u8) {
    slots[0] = Binding::Dpad { pad, dir: HatDir::Up };
    slots[1] = Binding::Dpad { pad, dir: HatDir::Down };
    slots[2] = Binding::Dpad { pad, dir: HatDir::Left };
    slots[3] = Binding::Dpad { pad, dir: HatDir::Right };

    slots[4] = Binding::Axis { pad, binding: wheel.steering };
    slots[5] = Binding::Axis { pad, binding: wheel.accelerator };
    slots[6] = Binding::Axis { pad, binding: wheel.brake };

    for (i, control) in wheel.gears.iter().enumerate() {
        slots[7 + i] = gear_binding(pad, *control);
    }
}

/// Daytona USA: 24 input slots in the emulator's display order (captured
/// from the emulator's config dialog; matches RetroBat's generator):
///   0-3   menu navigation up/down/left/right
///   4-6   steering, accelerate, brake (analog)
///   7-10  gears 1-4
///   11    gear neutral (left unbound so gears latch)
///   12-15 VR view buttons 1-4
///   16    start
///   17    coin
///   18-23 service/test block, left unbound
/// Footer: analog flags for steering/accelerate/brake.
fn daytona(wheel: &WheelProfile, pad: u8) -> Vec<u8> {
    let mut slots = [Binding::None; 24];
    driving_common(&mut slots, wheel, pad);

    for (i, button) in wheel.vr_buttons.iter().enumerate() {
        slots[12 + i] = Binding::Button { pad, number: *button };
    }

    slots[16] = Binding::Button { pad, number: wheel.btn_start };
    slots[17] = Binding::Button { pad, number: wheel.btn_coin };

    encode_file(&slots, &[1, 1, 1, 0, 0, 0, 0, 0])
}

/// Sega Rally Championship: 21 input slots; same driving prefix as Daytona
/// (cross-checked with RetroBat's generator), then:
///   12    view-change button
///   13    start
///   14    coin
///   15-20 service/test block, left unbound
fn srallyc(wheel: &WheelProfile, pad: u8) -> Vec<u8> {
    let mut slots = [Binding::None; 21];
    driving_common(&mut slots, wheel, pad);

    slots[12] = Binding::Button { pad, number: wheel.vr_buttons[0] };
    slots[13] = Binding::Button { pad, number: wheel.btn_start };
    slots[14] = Binding::Button { pad, number: wheel.btn_coin };

    encode_file(&slots, &[1, 1, 1, 0, 0, 0, 0, 0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::game::GAMES;
    use crate::domain::wheel::LOGITECH_G923_XBOX;

    /// Byte-for-byte contents of CFG/daytona.input as written by
    /// m2emulator's own control-config dialog with a G923 (Xbox/PC) as the
    /// first DirectInput device — captured 2026-07-05.
    #[rustfmt::skip]
    const CAPTURED_G923_XBOX: [u8; 104] = [
        0x0E, 0x01, 0x00, 0x00, // nav up (d-pad)
        0x0F, 0x01, 0x00, 0x00, // nav down
        0x0C, 0x01, 0x00, 0x00, // nav left
        0x0D, 0x01, 0x00, 0x00, // nav right
        0x00, 0x01, 0x00, 0xFF, // steering: X
        0x01, 0x11, 0x00, 0xFF, // accelerate: Y inverted
        0x02, 0x11, 0x00, 0xFF, // brake: RZ inverted
        0x30, 0x01, 0x00, 0x00, // gear 1: X
        0x10, 0x01, 0x00, 0x00, // gear 2: A
        0x40, 0x01, 0x00, 0x00, // gear 3: Y
        0x20, 0x01, 0x00, 0x00, // gear 4: B
        0x00, 0x00, 0x00, 0x00, // gear N: unbound
        0xA0, 0x01, 0x00, 0x00, // VR 1: button 10
        0x60, 0x01, 0x00, 0x00, // VR 2: left paddle
        0x90, 0x01, 0x00, 0x00, // VR 3: button 9
        0x50, 0x01, 0x00, 0x00, // VR 4: right paddle
        0x70, 0x01, 0x00, 0x00, // start: Menu
        0x80, 0x01, 0x00, 0x00, // coin: View
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // analog flags
    ];

    #[test]
    fn daytona_g923_xbox_matches_hardware_capture() {
        assert_eq!(daytona(&LOGITECH_G923_XBOX, 1), CAPTURED_G923_XBOX);
    }

    #[test]
    fn pad_number_lands_in_every_bound_entry() {
        let bytes = daytona(&LOGITECH_G923_XBOX, 2);
        for slot in 0..24 {
            let entry = &bytes[slot * 4..slot * 4 + 4];
            if entry.iter().any(|&b| b != 0) {
                assert_eq!(
                    entry[1] & 0x0F,
                    2,
                    "slot {slot} should target pad 2: {entry:02X?}"
                );
            }
        }
    }

    #[test]
    fn srallyc_g923_layout() {
        let bytes = srallyc(&LOGITECH_G923_XBOX, 1);
        assert_eq!(bytes.len(), 92, "21 slots * 4 bytes + 8-byte footer");

        // Driving prefix identical to Daytona's.
        assert_eq!(&bytes[..48], &daytona(&LOGITECH_G923_XBOX, 1)[..48]);
        // View = button 10, start = Menu (7), coin = View (8).
        assert_eq!(&bytes[48..52], &[0xA0, 0x01, 0x00, 0x00]);
        assert_eq!(&bytes[52..56], &[0x70, 0x01, 0x00, 0x00]);
        assert_eq!(&bytes[56..60], &[0x80, 0x01, 0x00, 0x00]);
        // Service block unbound; analog footer at 84.
        assert!(bytes[60..84].iter().all(|&b| b == 0));
        assert_eq!(&bytes[84..], &[1, 1, 1, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn every_m2_game_has_a_layout() {
        for game in GAMES.iter().filter(|g| g.emulator_id == "m2") {
            assert!(
                for_game(game, &LOGITECH_G923_XBOX, 1).is_some(),
                "game {} targets m2 but has no control layout",
                game.id
            );
        }
    }
}
