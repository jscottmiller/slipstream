//! Wheel profiles: how a physical wheel's DirectInput axes and buttons map
//! onto the control roles arcade racers need. Emulator-specific writers
//! (e.g. the m2emulator `.input` generator) consume these.

/// The full DirectInput axis set; profiles only use a few, but the m2
/// `.input` encoding covers all of them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum DiAxis {
    X,
    Y,
    Z,
    RX,
    RY,
    RZ,
    Slider1,
    Slider2,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HatDir {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy)]
pub struct AxisBinding {
    pub axis: DiAxis,
    /// True when the resting position reads as maximum (typical for pedals).
    pub inverted: bool,
}

pub struct FfbTuning {
    /// Percent of force below which the wheel motor doesn't overcome its own
    /// friction; the FFB plugin scales forces up from here.
    pub min_force: u8,
    pub max_force: u8,
}

pub struct WheelProfile {
    pub id: &'static str,
    pub name: &'static str,
    // USB ids feed HID detection, which only runs on Windows builds.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub vendor_id: u16,
    #[cfg_attr(not(windows), allow(dead_code))]
    pub product_ids: &'static [u16],
    pub steering: AxisBinding,
    pub accelerator: AxisBinding,
    pub brake: AxisBinding,
    /// Gears 1-4 on the d-pad hat.
    pub gears: [HatDir; 4],
    /// 1-based DirectInput button numbers.
    pub btn_start: u8,
    pub btn_coin: u8,
    /// View/VR buttons in game display order.
    pub vr_buttons: [u8; 4],
    pub ffb: FfbTuning,
}

/// Logitech G923. Both the PlayStation and Xbox variants present the same
/// DirectInput layout: steering on X, throttle on Y, brake on RZ, with both
/// pedals resting at maximum (inverted). Buttons (1-based): 1=Cross/A,
/// 2=Square/X, 3=Circle/B, 4=Triangle/Y, 9=Share/Back, 10=Options/Menu.
pub static LOGITECH_G923: WheelProfile = WheelProfile {
    id: "logitech-g923",
    name: "Logitech G923",
    vendor_id: 0x046D,
    product_ids: &[0xC266, 0xC267, 0xC26E], // PS, PS alt mode, Xbox/PC
    steering: AxisBinding { axis: DiAxis::X, inverted: false },
    accelerator: AxisBinding { axis: DiAxis::Y, inverted: true },
    brake: AxisBinding { axis: DiAxis::RZ, inverted: true },
    // Clockwise from Up: d-pad Up=1st, Right=2nd, Down=3rd, Left=4th.
    gears: [HatDir::Up, HatDir::Right, HatDir::Down, HatDir::Left],
    btn_start: 10, // Options
    btn_coin: 9,   // Share
    vr_buttons: [1, 2, 3, 4], // Cross, Square, Circle, Triangle
    ffb: FfbTuning { min_force: 15, max_force: 100 },
};

pub static WHEELS: &[&WheelProfile] = &[&LOGITECH_G923];

pub fn find(id: &str) -> Option<&'static WheelProfile> {
    WHEELS.iter().copied().find(|w| w.id == id)
}

/// Best-effort HID scan for a connected, known wheel.
#[cfg(windows)]
pub fn detect() -> Option<&'static WheelProfile> {
    let api = hidapi::HidApi::new().ok()?;
    api.device_list().find_map(|dev| {
        WHEELS.iter().copied().find(|w| {
            w.vendor_id == dev.vendor_id() && w.product_ids.contains(&dev.product_id())
        })
    })
}

#[cfg(not(windows))]
pub fn detect() -> Option<&'static WheelProfile> {
    None
}
