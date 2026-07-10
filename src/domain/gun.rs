//! Lightgun profiles. Guns run in **mouse mode** — both m2emulator and
//! Supermodel consume a gun as the system mouse natively, so no per-device
//! input config is generated. The profile exists for HID detection: the
//! cabinet UI sorts and dims its rail groups by which controllers are
//! actually connected.

pub struct GunProfile {
    /// Stable identifier, for a settings-side gun selection when profiles
    /// multiply (mirrors `WheelProfile::id`).
    #[allow(dead_code)]
    pub id: &'static str,
    pub name: &'static str,
    // USB ids feed HID detection, which only runs on Windows builds.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub vendor_id: u16,
    #[cfg_attr(not(windows), allow(dead_code))]
    pub product_ids: &'static [u16],
}

/// Gun4IR (ATmega32u4, Arduino vendor id). In mouse mode it enumerates as
/// a composite device: mouse + keyboard + game controller + a serial port
/// for the config app. PID 0x8042 captured from real hardware (player 1);
/// 0x8043-0x8045 are Gun4IR's player 2-4 convention, unverified.
#[cfg_attr(not(windows), allow(dead_code))]
pub static GUN4IR: GunProfile = GunProfile {
    id: "gun4ir",
    name: "Gun4IR",
    vendor_id: 0x2341,
    product_ids: &[0x8042, 0x8043, 0x8044, 0x8045],
};

#[cfg_attr(not(windows), allow(dead_code))]
pub static GUNS: &[&GunProfile] = &[&GUN4IR];

/// Best-effort HID scan for a connected, known lightgun.
#[cfg(windows)]
pub fn detect() -> Option<&'static GunProfile> {
    let api = hidapi::HidApi::new().ok()?;
    let found = api.device_list().find_map(|dev| {
        GUNS.iter()
            .copied()
            .find(|g| g.vendor_id == dev.vendor_id() && g.product_ids.contains(&dev.product_id()))
    });
    found
}

#[cfg(not(windows))]
pub fn detect() -> Option<&'static GunProfile> {
    None
}
