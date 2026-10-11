//! Accès clavier bas niveau. Windows : SendInput (scancodes, compatible jeux) +
//! hook clavier global WH_KEYBOARD_LL (ignore nos propres frappes injectées).
use crate::engine::Keyboard;
use crate::keys::RawKey;
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[cfg(any(windows, feature = "wincheck"))]
mod win {
    use crate::engine::Keyboard;
    use crate::keys::{is_extended_vk, KeySpec, RawKey};
    use std::ptr::null_mut;
    use std::sync::mpsc::Sender;
    use std::sync::OnceLock;
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyNameTextW, MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
        KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MAPVK_VK_TO_VSC,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, SetWindowsHookExW, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, MSG,
        WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    pub struct WinKeyboard;

    fn send(k: &KeySpec, up: bool) {
        unsafe {
            let mut scan = k.scan;
            if scan == 0 {
                scan = MapVirtualKeyW(k.vk as u32, MAPVK_VK_TO_VSC) as u16;
            }
            let mut flags = if up { KEYEVENTF_KEYUP } else { 0 };
            if k.ext || is_extended_vk(k.vk) {
                flags |= KEYEVENTF_EXTENDEDKEY;
            }
            let (wvk, wscan) = if scan != 0 {
                flags |= KEYEVENTF_SCANCODE; // reconnu par les jeux (DirectInput / RawInput)
                (0u16, scan)
            } else {
                (k.vk, 0u16)
            };
            let input = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: wvk, wScan: wscan, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
            };
            SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
        }
    }

    impl Keyboard for WinKeyboard {
        fn down(&self, k: &KeySpec) {
            send(k, false)
        }
        fn up(&self, k: &KeySpec) {
            send(k, true)
        }
    }

    static TX: OnceLock<Sender<RawKey>> = OnceLock::new();

    fn key_name(scan: u32, ext: bool) -> String {
        let mut buf = [0u16; 64];
        let lp = ((scan & 0xFF) << 16) | if ext { 1 << 24 } else { 0 };
        let n = unsafe { GetKeyNameTextW(lp as i32, buf.as_mut_ptr(), buf.len() as i32) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 && lparam != 0 {
            let k = &*(lparam as *const KBDLLHOOKSTRUCT);
            if k.flags & LLKHF_INJECTED == 0 {
                let msg = wparam as u32;
                let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
                let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
                if down || up {
                    if let Some(tx) = TX.get() {
                        let ext = k.flags & LLKHF_EXTENDED != 0;
                        let name = if down { key_name(k.scanCode, ext) } else { String::new() };
                        let key = KeySpec { vk: k.vkCode as u16, scan: k.scanCode as u16, ext, name };
                        let _ = tx.send(RawKey { key, down });
                    }
                }
            }
        }
        CallNextHookEx(null_mut(), code, wparam, lparam)
    }

    pub fn start_hook(tx: Sender<RawKey>) {
        let _ = TX.set(tx);
        std::thread::spawn(|| unsafe {
            use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
            let hmod = GetModuleHandleW(std::ptr::null());
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), hmod, 0);
            if hook.is_null() {
                return;
            }
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {}
        });
    }

    pub fn init() {
        unsafe {
            // timer système à 1 ms : intervalles courts précis
            windows_sys::Win32::Media::timeBeginPeriod(1);
        }
    }

    /// Coins arrondis + ombre natifs (Windows 11) pour la fenêtre sans bordure
    pub fn round_corners(title: &str) {
        use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
        use windows_sys::Win32::System::Threading::GetCurrentProcessId;
        use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowThreadProcessId};
        let wide: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), wide.as_ptr());
            if hwnd.is_null() {
                return;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid != GetCurrentProcessId() {
                return;
            }
            let pref: i32 = DWMWCP_ROUND;
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                &pref as *const i32 as *const core::ffi::c_void,
                std::mem::size_of::<i32>() as u32,
            );
        }
    }
}

/// Clavier factice hors Windows (aperçus / tests)
#[cfg(not(windows))]
struct NullKeyboard;
#[cfg(not(windows))]
impl Keyboard for NullKeyboard {
    fn down(&self, _: &crate::keys::KeySpec) {}
    fn up(&self, _: &crate::keys::KeySpec) {}
}

pub fn keyboard() -> Arc<dyn Keyboard> {
    #[cfg(windows)]
    return Arc::new(win::WinKeyboard);
    #[cfg(not(windows))]
    return Arc::new(NullKeyboard);
}

pub fn start_hook(tx: Sender<RawKey>) {
    #[cfg(windows)]
    win::start_hook(tx);
    #[cfg(not(windows))]
    drop(tx);
}

pub fn init() {
    #[cfg(windows)]
    win::init();
}

pub fn round_corners(_title: &str) {
    #[cfg(windows)]
    win::round_corners(_title);
}

// garde le code Windows « utilisé » quand on le vérifie depuis Linux
#[cfg(all(not(windows), feature = "wincheck"))]
pub fn _wincheck(tx: Sender<RawKey>) {
    let kb: Arc<dyn Keyboard> = Arc::new(win::WinKeyboard);
    kb.down(&crate::keys::KeySpec::vk(0x41));
    win::start_hook(tx);
    win::init();
    win::round_corners("AutoKey");
}
