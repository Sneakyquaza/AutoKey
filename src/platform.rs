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

#[cfg(windows)]
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

    /// Bip système (bloquant : appelé depuis un thread dédié)
    pub fn beep(freq: u32, ms: u32) {
        unsafe {
            windows_sys::Win32::System::Diagnostics::Debug::Beep(freq, ms);
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
                let ok = r == WAIT_OBJECT_0 || r == WAIT_ABANDONED;
                if !ok {
                    // la première instance tourne vraiment (peut-être cachée dans la zone de notification) : on la réaffiche
                    tray::wake_existing();
                }
                return ok;
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

    /// Icône de la zone de notification : fenêtre cachée (de premier niveau, pas « message-only » :
    /// ces dernières ne reçoivent pas le broadcast `TaskbarCreated`) dans un thread dédié.
    /// Lancement au démarrage de Windows : valeur `AutoKey` de HKCU\...\Run (jamais HKLM, jamais d'admin)
    pub mod autorun {
        use std::ptr::null_mut;
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_SZ,
        };

        const SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
        const NAME: &str = "AutoKey";

        fn w(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(Some(0)).collect()
        }

        fn open(access: u32) -> Option<HKEY> {
            let mut key: HKEY = null_mut();
            (unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, w(SUBKEY).as_ptr(), 0, access, &mut key) } == 0).then_some(key)
        }

        pub fn read() -> Option<String> {
            let key = open(KEY_READ)?;
            let mut buf = vec![0u16; 2048];
            let (mut ty, mut len) = (0u32, (buf.len() * 2) as u32);
            let r = unsafe { RegQueryValueExW(key, w(NAME).as_ptr(), null_mut(), &mut ty, buf.as_mut_ptr() as *mut u8, &mut len) };
            unsafe { RegCloseKey(key) };
            if r != 0 || ty != REG_SZ {
                return None;
            }
            let n = (len as usize / 2).min(buf.len());
            Some(String::from_utf16_lossy(&buf[..n]).trim_end_matches('\0').to_string())
        }

        /// `Some(commande)` : écrit la valeur ; `None` : la supprime
        pub fn write(cmd: Option<&str>) -> bool {
            let Some(key) = open(KEY_SET_VALUE) else { return false };
            let name = w(NAME);
            let ok = match cmd {
                Some(c) => {
                    let v = w(c);
                    unsafe { RegSetValueExW(key, name.as_ptr(), 0, REG_SZ, v.as_ptr() as *const u8, (v.len() * 2) as u32) == 0 }
                }
                // supprimer une valeur absente n'est pas une erreur
                None => unsafe { RegDeleteValueW(key, name.as_ptr()) == 0 || read().is_none() },
            };
            unsafe { RegCloseKey(key) };
            ok
        }
    }

    pub mod tray {
        use super::super::TrayCmd;
        use crate::i18n::{tr, Lang};
        use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU8, Ordering::SeqCst};
        use std::sync::OnceLock;
        use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows_sys::Win32::UI::Shell::{Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW,
            GetSystemMetrics, LoadIconW, LoadImageW, PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow,
            TrackPopupMenu, TranslateMessage, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTCOLOR, MF_SEPARATOR, MF_STRING, MSG, SM_CXSMICON, SM_CYSMICON,
            TPM_BOTTOMALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_DESTROY, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WNDCLASSW,
        };

        const WM_TRAY: u32 = WM_APP + 1;
        const WM_SYNC: u32 = WM_APP + 2;
        const WM_SHOW: u32 = WM_APP + 3;
        const CLASS: &str = "AutoKeyTrayWnd";
        /// Ressource d'icône embarquée par build.rs (`set_icon` → ID 1)
        const ICON_ID: usize = 1;

        static HWND_TRAY: AtomicIsize = AtomicIsize::new(0);
        static STARTED: AtomicBool = AtomicBool::new(false);
        static ADDED: AtomicBool = AtomicBool::new(false);
        static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
        static ON: AtomicBool = AtomicBool::new(false);
        static LANG: AtomicU8 = AtomicU8::new(0);
        static SPAM: AtomicBool = AtomicBool::new(false);
        static PAT: AtomicBool = AtomicBool::new(false);
        pub static SENDER: OnceLock<Box<dyn Fn(TrayCmd) + Send + Sync>> = OnceLock::new();

        fn wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }

        fn lang() -> Lang {
            Lang::ALL[(LANG.load(SeqCst) as usize).min(Lang::ALL.len() - 1)]
        }

        fn send(c: TrayCmd) {
            if let Some(f) = SENDER.get() {
                f(c);
            }
        }

        /// État voulu (appelé à chaque image de l'UI) : ne réveille le thread que si quelque chose a changé
        pub fn sync(on: bool, l: Lang, spam: bool, pat: bool) {
            let c = [ON.swap(on, SeqCst) != on, LANG.swap(l as u8, SeqCst) != l as u8, SPAM.swap(spam, SeqCst) != spam, PAT.swap(pat, SeqCst) != pat];
            let changed = c.contains(&true);
            if !changed {
                return;
            }
            if on && !STARTED.swap(true, SeqCst) {
                std::thread::spawn(run);
                return; // le thread lit l'état voulu à sa création
            }
            let h = HWND_TRAY.load(SeqCst);
            if h != 0 {
                unsafe { PostMessageW(h as HWND, WM_SYNC, 0, 0) };
            }
        }

        fn nid(hwnd: HWND) -> NOTIFYICONDATAW {
            let mut d: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
            d.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            d.hWnd = hwnd;
            d.uID = 1;
            d
        }

        /// Icône chargée une seule fois (le Shell en garde une copie ; pas de fuite à chaque mise à jour)
        fn load_icon() -> windows_sys::Win32::UI::WindowsAndMessaging::HICON {
            static ICON: AtomicIsize = AtomicIsize::new(0);
            let c = ICON.load(SeqCst);
            if c != 0 {
                return c as _;
            }
            let ic = load_icon_raw();
            ICON.store(ic as isize, SeqCst);
            ic
        }

        fn load_icon_raw() -> windows_sys::Win32::UI::WindowsAndMessaging::HICON {
            unsafe {
                let hinst = GetModuleHandleW(std::ptr::null());
                let ic = LoadImageW(hinst, ICON_ID as *const u16, IMAGE_ICON, GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON), LR_DEFAULTCOLOR);
                if ic.is_null() {
                    LoadIconW(std::ptr::null_mut(), IDI_APPLICATION)
                } else {
                    ic as _
                }
            }
        }

        /// Ajoute / met à jour / retire l'icône selon l'état voulu
        unsafe fn apply(hwnd: HWND) {
            let added = ADDED.load(SeqCst);
            if !ON.load(SeqCst) {
                if added {
                    Shell_NotifyIconW(NIM_DELETE, &nid(hwnd));
                    ADDED.store(false, SeqCst);
                }
                return;
            }
            let mut d = nid(hwnd);
            d.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            d.uCallbackMessage = WM_TRAY;
            d.hIcon = load_icon();
            let state = if SPAM.load(SeqCst) || PAT.load(SeqCst) { tr(lang(), "tray_running") } else { tr(lang(), "idle") };
            for (i, c) in wide(&format!("AutoKey — {state}")).into_iter().take(d.szTip.len() - 1).enumerate() {
                d.szTip[i] = c;
            }
            if Shell_NotifyIconW(if added { NIM_MODIFY } else { NIM_ADD }, &d) != 0 {
                ADDED.store(true, SeqCst);
            }
        }

        /// Retrait immédiat (sortie, panique) : n'utilise que des atomiques et une API Shell
        pub fn active() -> bool {
            ADDED.load(SeqCst)
        }

        pub fn remove() {
            let h = HWND_TRAY.load(SeqCst);
            if h != 0 && ADDED.swap(false, SeqCst) {
                unsafe { Shell_NotifyIconW(NIM_DELETE, &nid(h as HWND)) };
            }
        }

        const ID_SHOW: usize = 1;
        const ID_SPAM: usize = 2;
        const ID_PAT: usize = 3;
        const ID_QUIT: usize = 4;

        unsafe fn show_menu(hwnd: HWND) {
            let l = lang();
            let m = CreatePopupMenu();
            if m.is_null() {
                return;
            }
            let spam = if SPAM.load(SeqCst) { "tray_stop_spam" } else { "tray_start_spam" };
            let pat = if PAT.load(SeqCst) { "tray_stop_pat" } else { "tray_start_pat" };
            AppendMenuW(m, MF_STRING, ID_SHOW, wide(tr(l, "tray_show")).as_ptr());
            AppendMenuW(m, MF_SEPARATOR, 0, std::ptr::null());
            AppendMenuW(m, MF_STRING, ID_SPAM, wide(tr(l, spam)).as_ptr());
            AppendMenuW(m, MF_STRING, ID_PAT, wide(tr(l, pat)).as_ptr());
            AppendMenuW(m, MF_SEPARATOR, 0, std::ptr::null());
            AppendMenuW(m, MF_STRING, ID_QUIT, wide(tr(l, "tray_quit")).as_ptr());
            let mut pt = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pt);
            // requis par Windows pour que le menu se ferme quand on clique ailleurs
            SetForegroundWindow(hwnd);
            let cmd = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, pt.x, pt.y, 0, hwnd, std::ptr::null());
            PostMessageW(hwnd, WM_NULL, 0, 0);
            DestroyMenu(m);
            match cmd as usize {
                ID_SHOW => send(TrayCmd::Show),
                ID_SPAM => send(TrayCmd::ToggleSpam),
                ID_PAT => send(TrayCmd::TogglePattern),
                ID_QUIT => send(TrayCmd::Quit),
                _ => {}
            }
        }

        unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
            match msg {
                WM_TRAY => match (lp & 0xFFFF) as u32 {
                    WM_LBUTTONUP => send(TrayCmd::Show),
                    WM_RBUTTONUP => show_menu(hwnd),
                    _ => {}
                },
                WM_SYNC => apply(hwnd),
                WM_SHOW => send(TrayCmd::Show),
                WM_DESTROY => {
                    remove();
                    PostQuitMessage(0);
                }
                m if m != 0 && m == TASKBAR_CREATED.load(SeqCst) => {
                    // l'explorateur a redémarré : l'icône a disparu
                    ADDED.store(false, SeqCst);
                    apply(hwnd);
                }
                _ => return DefWindowProcW(hwnd, msg, wp, lp),
            }
            0
        }

        fn run() {
            unsafe {
                let hinst = GetModuleHandleW(std::ptr::null());
                let class = wide(CLASS);
                TASKBAR_CREATED.store(RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()), SeqCst);
                let wc = WNDCLASSW {
                    style: 0,
                    lpfnWndProc: Some(wndproc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: hinst,
                    hIcon: std::ptr::null_mut(),
                    hCursor: std::ptr::null_mut(),
                    hbrBackground: std::ptr::null_mut(),
                    lpszMenuName: std::ptr::null(),
                    lpszClassName: class.as_ptr(),
                };
                RegisterClassW(&wc);
                let hwnd =
                    CreateWindowExW(0, class.as_ptr(), class.as_ptr(), 0, 0, 0, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), hinst, std::ptr::null());
                if hwnd.is_null() {
                    STARTED.store(false, SeqCst);
                    return;
                }
                HWND_TRAY.store(hwnd as isize, SeqCst);
                apply(hwnd);
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }

        /// Demande à une autre instance (fenêtre cachée dans la zone de notification) de se réafficher
        pub fn wake_existing() {
            unsafe {
                let h = FindWindowW(wide(CLASS).as_ptr(), std::ptr::null());
                if !h.is_null() {
                    PostMessageW(h, WM_SHOW, 0, 0);
                }
            }
        }
    }

    /// Liaison à une fenêtre cible (racine du premier plan). Tant que `target` vaut 0, la tâche est
    /// « armée » : la première fenêtre au premier plan qui n'appartient pas à AutoKey est verrouillée.
    pub struct Bound {
        target: std::sync::atomic::AtomicIsize,
        /// candidat en armement + instant d'apparition : verrouillé seulement s'il reste
        /// 400 ms au premier plan (évite le sélecteur Alt+Tab / la barre des tâches)
        cand: std::sync::Mutex<(isize, std::time::Instant)>,
    }

    /// Racine de la fenêtre au premier plan et PID propriétaire (0 si aucune)
    fn foreground() -> (isize, u32) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetAncestor, GetForegroundWindow, GetWindowThreadProcessId, GA_ROOT};
        unsafe {
            let fg = GetForegroundWindow();
            if fg.is_null() {
                return (0, 0);
            }
            let root = GetAncestor(fg, GA_ROOT);
            let w = if root.is_null() { fg } else { root };
            let mut pid = 0u32;
            GetWindowThreadProcessId(w, &mut pid);
            (w as isize, pid)
        }
    }

    impl Bound {
        /// Capture la fenêtre au premier plan ; si c'est AutoKey (démarrage par le bouton de l'UI),
        /// reste armée et lie la prochaine fenêtre étrangère qui passera au premier plan.
        pub fn new() -> Self {
            let b = Bound { target: std::sync::atomic::AtomicIsize::new(0), cand: std::sync::Mutex::new((0, std::time::Instant::now())) };
            // démarrage par raccourci : la fenêtre au premier plan est la cible, sans délai
            let (w, pid) = foreground();
            if w != 0 && pid != unsafe { windows_sys::Win32::System::Threading::GetCurrentProcessId() } {
                b.target.store(w, std::sync::atomic::Ordering::SeqCst);
            }
            b
        }
        fn arm(&self) -> bool {
            let (w, pid) = foreground();
            let mut c = self.cand.lock().unwrap_or_else(|e| e.into_inner());
            if w == 0 || pid == unsafe { windows_sys::Win32::System::Threading::GetCurrentProcessId() } {
                c.0 = 0;
                return false;
            }
            if c.0 != w {
                *c = (w, std::time::Instant::now());
                return false;
            }
            if c.1.elapsed() < std::time::Duration::from_millis(400) {
                return false;
            }
            self.target.store(w, std::sync::atomic::Ordering::SeqCst);
            true
        }
    }

    impl crate::engine::Focus for Bound {
        fn ok(&self) -> bool {
            let t = self.target.load(std::sync::atomic::Ordering::SeqCst);
            if t == 0 {
                return self.arm();
            }
            foreground().0 == t
        }
        fn gone(&self) -> bool {
            use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;
            let t = self.target.load(std::sync::atomic::Ordering::SeqCst);
            t != 0 && unsafe { IsWindow(t as _) } == 0
        }
        fn armed(&self) -> bool {
            self.target.load(std::sync::atomic::Ordering::SeqCst) != 0
        }
    }
}

/// Liaison à la fenêtre au premier plan (voir `win::Bound`). `None` hors Windows : toujours actif.
pub fn bind_foreground() -> Option<Arc<dyn crate::engine::Focus>> {
    #[cfg(windows)]
    return Some(Arc::new(win::Bound::new()));
    #[cfg(not(windows))]
    return None;
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
    #[cfg(windows)]
    win::tray::remove();
    for vk in 0..256u16 {
        if INJ[vk as usize].load(Relaxed) == HELD {
            #[cfg(windows)]
            win::WinKeyboard.up(&KeySpec::vk(vk));
            #[cfg(not(windows))]
            NullKeyboard.up(&KeySpec::vk(vk));
        }
    }
}

/// Commande émise par l'icône de la zone de notification
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayCmd {
    Show,
    ToggleSpam,
    TogglePattern,
    Quit,
}

/// Enregistre le récepteur des commandes de l'icône (appelé une fois au démarrage). No-op hors Windows.
pub fn tray_init(_f: Box<dyn Fn(TrayCmd) + Send + Sync>) {
    #[cfg(windows)]
    let _ = win::tray::SENDER.set(_f);
}

/// État voulu de l'icône : présence, langue du menu, tâches en cours (info-bulle + libellés). No-op hors Windows.
pub fn tray_sync(_on: bool, _lang: crate::i18n::Lang, _spam: bool, _pat: bool) {
    #[cfg(windows)]
    win::tray::sync(_on, _lang, _spam, _pat);
}

/// Vrai si l'icône est effectivement présente dans la zone de notification
pub fn tray_active() -> bool {
    #[cfg(windows)]
    return win::tray::active();
    #[cfg(not(windows))]
    return false;
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

/// État du lancement au démarrage de Windows (registre = source de vérité)
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AutoStart {
    Off,
    On,
    /// Activé, mais la valeur pointe vers un autre exe (déplacé / ancienne copie)
    OtherPath,
}

#[cfg_attr(not(windows), allow(dead_code))]
/// Commande écrite dans le registre : chemin de l'exe entre guillemets
pub fn autorun_command(exe: &std::path::Path) -> String {
    format!("\"{}\"", exe.display())
}

#[cfg_attr(not(windows), allow(dead_code))]
/// Compare la valeur du registre à la commande voulue (insensible à la casse, comme les chemins Windows)
pub fn autorun_classify(cur: Option<&str>, want: &str) -> AutoStart {
    match cur.map(str::trim) {
        None | Some("") => AutoStart::Off,
        Some(c) if c.eq_ignore_ascii_case(want) => AutoStart::On,
        Some(_) => AutoStart::OtherPath,
    }
}

/// Options système (zone de notification, lancement au démarrage) : Windows seulement
/// (affichées aussi pour les captures `shots` sous Linux)
pub fn autostart_supported() -> bool {
    cfg!(windows) || cfg!(feature = "shots")
}

pub fn autostart_state() -> AutoStart {
    #[cfg(windows)]
    if let Ok(exe) = std::env::current_exe() {
        return autorun_classify(win::autorun::read().as_deref(), &autorun_command(&exe));
    }
    AutoStart::Off
}

/// Active (chemin de l'exe courant) ou désactive le lancement au démarrage. Faux en cas d'échec / hors Windows.
pub fn autostart_set(_on: bool) -> bool {
    #[cfg(windows)]
    match (_on, std::env::current_exe()) {
        (false, _) => win::autorun::write(None),
        (true, Ok(exe)) => win::autorun::write(Some(&autorun_command(&exe))),
        (true, Err(_)) => false,
    }
    #[cfg(not(windows))]
    false
}

/// Ouvre un dossier dans l'explorateur. No-op hors Windows.
pub fn open_folder(_dir: &std::path::Path) {
    #[cfg(windows)]
    {
        let _ = std::fs::create_dir_all(_dir);
        let sys = std::env::var_os("SystemRoot").map(std::path::PathBuf::from).unwrap_or_default();
        let explorer = sys.join("explorer.exe");
        let _ = std::process::Command::new(if explorer.is_file() { explorer } else { "explorer".into() }).arg(_dir).spawn();
    }
}

/// Bip court non bloquant (thread dédié) : aigu au démarrage, grave à l'arrêt. No-op hors Windows.
pub fn beep(start: bool) {
    #[cfg(windows)]
    {
        // un seul bip à la fois : pas de cascade de threads si on démarre / arrête en rafale
        use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
        static BUSY: AtomicBool = AtomicBool::new(false);
        if !BUSY.swap(true, SeqCst) {
            let (f, ms) = if start { (1000, 60) } else { (600, 80) };
            std::thread::spawn(move || {
                win::beep(f, ms);
                BUSY.store(false, SeqCst);
            });
        }
    }
    #[cfg(not(windows))]
    let _ = start;
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
    #[cfg(windows)]
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

/// Boîtes de dialogue natives (Windows). Ailleurs : aucune.
pub fn pick_save() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    return rfd::FileDialog::new().add_filter("AutoKey", &["json"]).set_file_name("pattern.json").save_file();
    #[cfg(not(windows))]
    return None;
}

/// Enregistrer sous (JSON) avec un nom proposé
pub fn pick_save_as(_name: &str) -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    return rfd::FileDialog::new().add_filter("AutoKey", &["json"]).set_file_name(_name).save_file();
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
    fn autorun_command_and_classify() {
        let want = autorun_command(std::path::Path::new(r"C:\Apps\AutoKey.exe"));
        assert_eq!(want, r#""C:\Apps\AutoKey.exe""#);
        assert_eq!(autorun_classify(None, &want), AutoStart::Off);
        assert_eq!(autorun_classify(Some(""), &want), AutoStart::Off);
        assert_eq!(autorun_classify(Some(&want), &want), AutoStart::On);
        assert_eq!(autorun_classify(Some(r#""c:\apps\autokey.exe""#), &want), AutoStart::On);
        assert_eq!(autorun_classify(Some(r#""D:\Old\AutoKey.exe""#), &want), AutoStart::OtherPath);
    }

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
