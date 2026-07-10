//! Quit-the-game watcher. Two signals end a session: the Escape key and the
//! wheel's console button (Xbox/PS logo), so quitting never means reaching
//! for the keyboard. Either signal sends the emulator WM_CLOSE — a graceful
//! close, so NVRAM and config still flush on the way out — with a hard kill
//! only if it ignores the request. (WM_CLOSE reaches SDL emulators like
//! Supermodel as SDL_QUIT: the same clean shutdown as their own quit key.
//! Synthesizing an Escape keypress instead was tried and never reached
//! Supermodel's DirectInput keyboard path.) The Escape key is only watched
//! for emulators with no quit key of their own — Supermodel handles Escape
//! itself; m2emulator's ESC just toggles fullscreen. The watcher only acts
//! while the launched emulator owns the foreground window.

use crate::domain::wheel::WheelProfile;
use std::process::Child;
use std::sync::mpsc::{self, Receiver};

/// Watch the launched emulator until it exits. `watch_escape_key` is the
/// emulator's `needs_escape_quit()`: whether the Escape key should also
/// trigger the close (the console button always does).
///
/// The returned channel never carries a message; it disconnects when the
/// emulator has exited, so callers can poll `try_recv()` for
/// `Err(Disconnected)` (or simply drop the receiver to ignore it).
#[cfg(windows)]
pub fn watch(
    child: Child,
    companions: Vec<Child>,
    wheel: &'static WheelProfile,
    watch_escape_key: bool,
) -> Receiver<()> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _hangup = tx;
        windows_impl::run(child, wheel, watch_escape_key);
        reap_companions(companions);
    });
    rx
}

#[cfg(not(windows))]
pub fn watch(
    mut child: Child,
    companions: Vec<Child>,
    _wheel: &'static WheelProfile,
    _watch_escape_key: bool,
) -> Receiver<()> {
    // No quit signals off-Windows; reap the child in the background so it
    // never zombies, and hang up on exit like the Windows watcher.
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _hangup = tx;
        let _ = child.wait();
        reap_companions(companions);
    });
    rx
}

/// Companion helpers (DemulShooter) have no quit signal of their own; once
/// the emulator is gone they are killed outright.
fn reap_companions(companions: Vec<Child>) {
    for mut companion in companions {
        let _ = companion.kill();
        let _ = companion.wait();
    }
}

#[cfg(windows)]
mod windows_impl {
    use crate::domain::wheel::WheelProfile;
    use std::process::Child;
    use std::time::{Duration, Instant};

    #[link(name = "user32")]
    extern "system" {
        fn GetAsyncKeyState(v_key: i32) -> i16;
        fn GetForegroundWindow() -> isize;
        fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
        fn PostMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> i32;
    }

    #[link(name = "winmm")]
    extern "system" {
        fn joyGetNumDevs() -> u32;
        fn joyGetDevCapsW(joy_id: usize, caps: *mut JoyCapsW, cb: u32) -> u32;
        fn joyGetPosEx(joy_id: u32, info: *mut JoyInfoEx) -> u32;
    }

    const VK_ESCAPE: i32 = 0x1B;
    const WM_CLOSE: u32 = 0x0010;
    const JOY_RETURNBUTTONS: u32 = 0x0080;
    const JOYERR_NOERROR: u32 = 0;
    const CLOSE_GRACE: Duration = Duration::from_secs(5);
    const POLL: Duration = Duration::from_millis(50);
    /// Polls to wait between device scans while the wheel isn't found (~2s).
    const RESCAN_POLLS: u32 = 40;

    /// Only the leading vendor/product ids matter; the rest of JOYCAPSW
    /// (name, ranges, driver strings) is opaque padding. The u32 padding
    /// keeps alignment at 4 and the total at 728 bytes, matching the SDK
    /// definition winmm validates against.
    #[repr(C)]
    struct JoyCapsW {
        mid: u16,
        pid: u16,
        rest: [u32; 181],
    }

    #[derive(Default)]
    #[repr(C)]
    struct JoyInfoEx {
        size: u32,
        flags: u32,
        xpos: u32,
        ypos: u32,
        zpos: u32,
        rpos: u32,
        upos: u32,
        vpos: u32,
        buttons: u32,
        button_number: u32,
        pov: u32,
        reserved1: u32,
        reserved2: u32,
    }

    pub fn run(mut child: Child, wheel: &'static WheelProfile, watch_escape_key: bool) {
        let mut quit_button = QuitButton::new(wheel);
        loop {
            match child.try_wait() {
                Ok(Some(_)) | Err(_) => return, // emulator exited on its own
                Ok(None) => {}
            }

            // Poll the button every tick so its edge state never goes stale,
            // but act only while the emulator owns the foreground.
            let quit = quit_button.as_mut().is_some_and(QuitButton::just_pressed)
                || (watch_escape_key && escape_pressed());

            if quit {
                if let Some(hwnd) = foreground_window_of(child.id()) {
                    unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
                    let deadline = Instant::now() + CLOSE_GRACE;
                    while Instant::now() < deadline {
                        if let Ok(Some(_)) = child.try_wait() {
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    let _ = child.kill();
                    let _ = child.wait();
                    return;
                }
            }

            std::thread::sleep(POLL);
        }
    }

    fn escape_pressed() -> bool {
        (unsafe { GetAsyncKeyState(VK_ESCAPE) } as u16) & 0x8000 != 0
    }

    /// The foreground window, but only when it belongs to the given process —
    /// quit signals from any other app must not close the emulator.
    fn foreground_window_of(pid: u32) -> Option<isize> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd == 0 {
            return None;
        }
        let mut owner: u32 = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
        (owner == pid).then_some(hwnd)
    }

    /// Rising-edge detector for the wheel's console button, polled through
    /// winmm (whose joystick ids cover the same devices DirectInput sees).
    /// Survives the wheel being absent or unplugged mid-game: the device is
    /// re-scanned every couple of seconds until it answers again.
    struct QuitButton {
        wheel: &'static WheelProfile,
        mask: u32,
        joy_id: Option<u32>,
        polls_until_rescan: u32,
        was_down: bool,
    }

    impl QuitButton {
        fn new(wheel: &'static WheelProfile) -> Option<Self> {
            wheel.btn_quit.map(|button| Self {
                wheel,
                mask: 1 << (button - 1),
                joy_id: None,
                polls_until_rescan: 0,
                was_down: false,
            })
        }

        fn just_pressed(&mut self) -> bool {
            let down = self.is_down();
            let edge = down && !self.was_down;
            self.was_down = down;
            edge
        }

        fn is_down(&mut self) -> bool {
            let Some(joy_id) = self.device() else {
                return false;
            };
            let mut info = JoyInfoEx {
                size: std::mem::size_of::<JoyInfoEx>() as u32,
                flags: JOY_RETURNBUTTONS,
                ..Default::default()
            };
            if unsafe { joyGetPosEx(joy_id, &mut info) } != JOYERR_NOERROR {
                self.joy_id = None; // unplugged; rescan picks it back up
                return false;
            }
            info.buttons & self.mask != 0
        }

        fn device(&mut self) -> Option<u32> {
            if self.joy_id.is_none() {
                if self.polls_until_rescan > 0 {
                    self.polls_until_rescan -= 1;
                    return None;
                }
                self.polls_until_rescan = RESCAN_POLLS;
                self.joy_id = find_wheel(self.wheel);
            }
            self.joy_id
        }
    }

    /// The winmm joystick id whose USB ids match the wheel profile.
    fn find_wheel(wheel: &WheelProfile) -> Option<u32> {
        (0..unsafe { joyGetNumDevs() }).find(|&joy_id| {
            let mut caps: JoyCapsW = unsafe { std::mem::zeroed() };
            let cb = std::mem::size_of::<JoyCapsW>() as u32;
            (unsafe { joyGetDevCapsW(joy_id as usize, &mut caps, cb) } == JOYERR_NOERROR)
                && caps.mid == wheel.vendor_id
                && wheel.product_ids.contains(&caps.pid)
        })
    }
}
