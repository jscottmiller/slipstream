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

/// A gear-lever position mapped to a physical control on the wheel.
#[derive(Clone, Copy)]
#[allow(dead_code)] // Dpad is available for profiles that shift on the hat
pub enum GearControl {
    /// 1-based DirectInput button number.
    Button(u8),
    /// D-pad (POV hat) direction.
    Dpad(HatDir),
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
    pub gears: [GearControl; 4],
    /// 1-based DirectInput button numbers.
    pub btn_start: u8,
    pub btn_coin: u8,
    /// View/VR buttons in game display order.
    pub vr_buttons: [u8; 4],
    pub ffb: FfbTuning,
}

/// Logitech G923 for Xbox/PC (PID 0xC26E). Every code below was captured
/// from m2emulator's own control-config dialog on real hardware: steering X,
/// throttle Y (inverted), brake RZ (inverted); buttons 1=A 2=B 3=X 4=Y
/// 5=right paddle 6=left paddle 7=Menu 8=View 9/10=rear wheel buttons.
pub static LOGITECH_G923_XBOX: WheelProfile = WheelProfile {
    id: "logitech-g923",
    name: "Logitech G923 (Xbox/PC)",
    vendor_id: 0x046D,
    product_ids: &[0xC26E],
    steering: AxisBinding { axis: DiAxis::X, inverted: false },
    accelerator: AxisBinding { axis: DiAxis::Y, inverted: true },
    brake: AxisBinding { axis: DiAxis::RZ, inverted: true },
    // Face buttons as the H pattern: X=1st, A=2nd, Y=3rd, B=4th.
    gears: [
        GearControl::Button(3),
        GearControl::Button(1),
        GearControl::Button(4),
        GearControl::Button(2),
    ],
    btn_start: 7, // Menu
    btn_coin: 8,  // View
    vr_buttons: [10, 6, 9, 5],
    ffb: FfbTuning { min_force: 15, max_force: 100 },
};

/// Logitech G923 for PlayStation (PIDs 0xC266/0xC267). Same axes as the
/// Xbox variant; button numbers follow the PS layout (1=Cross, 2=Square,
/// 3=Circle, 4=Triangle, 9=Share, 10=Options). Not yet hardware-verified.
pub static LOGITECH_G923_PS: WheelProfile = WheelProfile {
    id: "logitech-g923-ps",
    name: "Logitech G923 (PlayStation)",
    vendor_id: 0x046D,
    product_ids: &[0xC266, 0xC267],
    steering: AxisBinding { axis: DiAxis::X, inverted: false },
    accelerator: AxisBinding { axis: DiAxis::Y, inverted: true },
    brake: AxisBinding { axis: DiAxis::RZ, inverted: true },
    gears: [
        GearControl::Button(2),
        GearControl::Button(1),
        GearControl::Button(4),
        GearControl::Button(3),
    ],
    btn_start: 10, // Options
    btn_coin: 9,   // Share
    vr_buttons: [1, 2, 3, 4],
    ffb: FfbTuning { min_force: 15, max_force: 100 },
};

pub static WHEELS: &[&WheelProfile] = &[&LOGITECH_G923_XBOX, &LOGITECH_G923_PS];

pub fn find(id: &str) -> Option<&'static WheelProfile> {
    WHEELS.iter().copied().find(|w| w.id == id)
}

/// Best-effort HID scan for a connected, known wheel.
#[cfg(windows)]
pub fn detect() -> Option<&'static WheelProfile> {
    let api = hidapi::HidApi::new().ok()?;
    let found = api.device_list().find_map(|dev| {
        WHEELS.iter().copied().find(|w| {
            w.vendor_id == dev.vendor_id() && w.product_ids.contains(&dev.product_id())
        })
    });
    found
}

#[cfg(not(windows))]
pub fn detect() -> Option<&'static WheelProfile> {
    None
}
