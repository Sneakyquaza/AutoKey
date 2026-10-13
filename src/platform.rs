//! Accès clavier bas niveau.
//! Windows : envoi par SendInput (scancodes, compatible jeux) et lecture de l'état
//! des touches par GetAsyncKeyState (global, fonctionne fenêtre en arrière-plan).
use crate::engine::Keyboard;
use crate::keys::KeySpec;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

// ------------------------------------------------------------------ marqueurs d'injection
// Les touches que NOUS envoyons sont ignorées à la lecture (pas d'auto-déclenchement).
const HELD: u64 = u64::MAX;
static INJ: [AtomicU64; 256] = [const { AtomicU64::new(0) }; 256];
static T0: OnceLock<Instant> = OnceLock::new();

fn now_ms() -> u64 {
    T0.get_or_init(Instant::now).elapsed().as_millis() as u64 + 1
}

fn mark_injected(vk: u16, down: bool) {
    let v = if down { HELD } else { now_ms() };
    INJ[(vk & 0xFF) as usize].store(v, Relaxed);
}

/// Vrai si la touche vient d'être envoyée par AutoKey (maintenue ou relâchée il y a < 60 ms)
fn recently_injected(vk: u16) -> bool {
    let v = INJ[(vk & 0xFF) as usize].load(Relaxed);
    v == HELD || (v != 0 && now_ms().saturating_sub(v) < 60)
}

#[cfg(any(windows, feature = "wincheck"))]
mod win {
    use super::{mark_injected, recently_injected};
    use crate::engine::Keyboard;
    use crate::keys::{is_extended_vk, KeySpec};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetKeyNameTextW, MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        KEYEVENTF_SCANCODE, MAPVK_VK_TO_VSC, MAPVK_VK_TO_VSC_EX,
    };

    pub struct WinKeyboard;

    fn send(k: &KeySpec, up: bool) {
        mark_injected(k.vk, !up);
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
            let input =
                INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: wvk, wScan: wscan, dwFlags: flags, time: 0, dwExtraInfo: 0 } } };
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

    /// (enfoncée maintenant, appuyée-relâchée depuis la dernière lecture)
    pub fn key_poll(vk: u16) -> (bool, bool) {
        let s = unsafe { GetAsyncKeyState(vk as i32) } as u16;
        if recently_injected(vk) {
            return (false, false);
        }
        ((s & 0x8000) != 0, (s & 1) != 0)
    }

    pub fn key_spec(vk: u16) -> KeySpec {
        unsafe {
            let sc = MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC_EX);
            let ext = (sc & 0xFF00) == 0xE000 || (sc & 0xFF00) == 0xE100 || is_extended_vk(vk);
            let scan = (sc & 0xFF) as u16;
            let mut buf = [0u16; 64];
            let lp = ((scan as u32) << 16) | if ext { 1 << 24 } else { 0 };
            let n = GetKeyNameTextW(lp as i32, buf.as_mut_ptr(), buf.len() as i32);
            let name = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
            KeySpec { vk, scan, ext, name }
        }
    }

    /// Timer système à 1 ms, uniquement pendant qu'une tâche tourne (économie d'énergie sinon)
    pub fn hires(on: bool) {
        unsafe {
            if on {
                windows_sys::Win32::Media::timeBeginPeriod(1);
            } else {
                windows_sys::Win32::Media::timeEndPeriod(1);
            }
        }
    }

    /// Mutex nommé de session : une seule instance. Si une autre tourne encore (ex. ancien exe
    /// en train de quitter après une mise à jour), on attend jusqu'à 8 s qu'elle libère le mutex.
    pub fn single_instance() -> bool {
        use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, WAIT_ABANDONED, WAIT_OBJECT_0};
        use windows_sys::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};
        let name: Vec<u16> = "Local\\AutoKey.SingleInstance\0".encode_utf16().collect();
        unsafe {
            // handle volontairement gardé ouvert pendant toute la vie du processus
            let h = CreateMutexW(std::ptr::null(), 1, name.as_ptr());
            if h.is_null() {
                return true; // échec de création : on ne bloque pas le démarrage
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let r = WaitForSingleObject(h, 8000);
                return r == WAIT_OBJECT_0 || r == WAIT_ABANDONED;
            }
        }
        true
    }

    /// Rend au système la mémoire qui n'a servi qu'à l'initialisation
    pub fn trim_memory() {
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
        unsafe {
            SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
        }
    }

    /// Coins arrondis + ombre natifs (Windows 11) pour la fenêtre sans bordure
    pub fn round_corners(hwnd: isize) {
        use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
        let hwnd = hwnd as windows_sys::Win32::Foundation::HWND;
        unsafe {
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

// ------------------------------------------------------------------ simulation hors Windows
// Table d'état des touches qui imite GetAsyncKeyState (touches physiques simulées + nos frappes).
#[cfg(not(windows))]
static FAKE: [std::sync::atomic::AtomicBool; 256] = [const { std::sync::atomic::AtomicBool::new(false) }; 256];

#[cfg(not(windows))]
struct NullKeyboard;
#[cfg(not(windows))]
impl Keyboard for NullKeyboard {
    fn down(&self, k: &KeySpec) {
        mark_injected(k.vk, true);
        FAKE[(k.vk & 0xFF) as usize].store(true, Relaxed);
    }
    fn up(&self, k: &KeySpec) {
        FAKE[(k.vk & 0xFF) as usize].store(false, Relaxed);
        mark_injected(k.vk, false);
    }
}

/// Relâche toute touche que nous avons envoyée enfoncée (panique, sortie) : n'utilise que des atomiques.
pub fn release_held() {
    for vk in 0..256u16 {
        if INJ[vk as usize].load(Relaxed) == HELD {
            #[cfg(windows)]
            win::WinKeyboard.up(&KeySpec::vk(vk));
            #[cfg(not(windows))]
            NullKeyboard.up(&KeySpec::vk(vk));
        }
    }
}

pub fn keyboard() -> Arc<dyn Keyboard> {
    #[cfg(windows)]
    return Arc::new(win::WinKeyboard);
    #[cfg(not(windows))]
    return Arc::new(NullKeyboard);
}

/// État physique d'une touche (hors frappes envoyées par AutoKey) :
/// (enfoncée maintenant, appui très bref survenu entre deux lectures)
pub fn key_poll(_vk: u16) -> (bool, bool) {
    #[cfg(windows)]
    return win::key_poll(_vk);
    #[cfg(not(windows))]
    return (FAKE[(_vk & 0xFF) as usize].load(Relaxed) && !recently_injected(_vk), false);
}

pub fn key_spec(vk: u16) -> KeySpec {
    #[cfg(windows)]
    return win::key_spec(vk);
    #[cfg(not(windows))]
    return KeySpec::vk(vk);
}

/// Tests hors Windows : touches simulées lues sur stdin (« d 51 » / « u 51 », vk en hexa)
pub fn start_test_input() {
    #[cfg(not(windows))]
    if std::env::var_os("AUTOKEY_STDIN_KEYS").is_some() {
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in std::io::stdin().lock().lines().map_while(Result::ok) {
                let mut it = line.split_whitespace();
                let (Some(d), Some(v)) = (it.next(), it.next()) else { continue };
                let Ok(vk) = u16::from_str_radix(v.trim_start_matches("0x"), 16) else { continue };
                FAKE[(vk & 0xFF) as usize].store(d == "d", Relaxed); // lu par le poller, comme sous Windows
            }
        });
    }
}

pub fn init() {
    T0.get_or_init(Instant::now);
}

static HIRES: std::sync::Mutex<u32> = std::sync::Mutex::new(0);

/// Active/désactive le timer 1 ms (compteur de références)
pub fn hires(on: bool) {
    let mut n = HIRES.lock().unwrap();
    if on {
        *n += 1;
        #[cfg(windows)]
        if *n == 1 {
            win::hires(true);
        }
    } else if *n > 0 {
        *n -= 1;
        #[cfg(windows)]
        if *n == 0 {
            win::hires(false);
        }
    }
}

/// Une seule instance d'AutoKey par session. Faux : une autre instance tourne déjà.
pub fn single_instance() -> bool {
    #[cfg(windows)]
    return win::single_instance();
    #[cfg(not(windows))]
    return true;
}

pub fn trim_memory() {
    #[cfg(windows)]
    win::trim_memory();
}

/// Appelé une fois la fenêtre affichée : coins arrondis (Win 11) puis libération de la mémoire d'init
pub fn after_window_shown(_w: &winit::window::Window) {
    #[cfg(any(windows, feature = "wincheck"))]
    {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(h) = _w.window_handle() {
            if let RawWindowHandle::Win32(h) = h.as_raw() {
                win::round_corners(h.hwnd.get());
            }
        }
    }
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(3));
        trim_memory();
    });
}

// garde le code Windows « utilisé » quand on le vérifie depuis Linux
#[cfg(all(not(windows), feature = "wincheck"))]
pub fn _wincheck() {
    let kb: Arc<dyn Keyboard> = Arc::new(win::WinKeyboard);
    kb.down(&KeySpec::vk(0x41));
    let _ = win::key_poll(0x41);
    let _ = win::key_spec(0x41);
    win::hires(true);
    let _ = win::single_instance();
    win::trim_memory();
    win::round_corners(0);
}

/// Boîtes de dialogue natives (Windows). Ailleurs : aucune.
pub fn pick_save() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    return rfd::FileDialog::new().add_filter("AutoKey", &["json"]).set_file_name("pattern.json").save_file();
    #[cfg(not(windows))]
    return None;
}

pub fn pick_open() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    return rfd::FileDialog::new().add_filter("AutoKey", &["json"]).pick_file();
    #[cfg(not(windows))]
    return None;
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;

    #[test]
    fn release_held_releases_only_held_keys() {
        let (a, b) = (KeySpec::vk(0xE8), KeySpec::vk(0xE9));
        NullKeyboard.down(&a);
        NullKeyboard.down(&b);
        NullKeyboard.up(&b);
        release_held();
        assert_ne!(INJ[0xE8].load(Relaxed), HELD);
        assert!(!FAKE[0xE8].load(Relaxed));
        assert_ne!(INJ[0xE9].load(Relaxed), HELD);
    }
}
