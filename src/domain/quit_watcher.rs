//! Quit-on-Escape for emulators that don't do it themselves (m2emulator's
//! ESC only toggles fullscreen). While the launched emulator owns the
//! foreground window, pressing Escape sends it WM_CLOSE — a graceful close,
//! so the emulator still flushes NVRAM and config on the way out — with a
//! hard kill only if it ignores the request.

use std::process::Child;

#[cfg(windows)]
pub fn watch_escape(child: Child) {
    std::thread::spawn(move || windows_impl::run(child));
}

#[cfg(not(windows))]
pub fn watch_escape(mut child: Child) {
    // No watcher off-Windows; reap the child in the background so it never
    // zombies.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
}

#[cfg(windows)]
mod windows_impl {
    use std::process::Child;
    use std::time::{Duration, Instant};

    #[link(name = "user32")]
    extern "system" {
        fn GetAsyncKeyState(v_key: i32) -> i16;
        fn GetForegroundWindow() -> isize;
        fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
        fn PostMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> i32;
    }

    const VK_ESCAPE: i32 = 0x1B;
    const WM_CLOSE: u32 = 0x0010;
    const CLOSE_GRACE: Duration = Duration::from_secs(5);

    pub fn run(mut child: Child) {
        loop {
            match child.try_wait() {
                Ok(Some(_)) | Err(_) => return, // emulator exited on its own
                Ok(None) => {}
            }

            if escape_pressed() {
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

            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn escape_pressed() -> bool {
        (unsafe { GetAsyncKeyState(VK_ESCAPE) } as u16) & 0x8000 != 0
    }

    /// The foreground window, but only when it belongs to the given process —
    /// Escape pressed in any other app must not close the emulator.
    fn foreground_window_of(pid: u32) -> Option<isize> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd == 0 {
            return None;
        }
        let mut owner: u32 = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
        (owner == pid).then_some(hwnd)
    }
}
