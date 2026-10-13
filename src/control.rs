//! État partagé + contrôleur : reçoit les touches physiques (hook global), gère
//! l'enregistrement des raccourcis et le démarrage/arrêt, même fenêtre en arrière-plan.
use crate::config::{self, HkMode, Mode, Settings, Step};
use crate::engine::{CStep, Job, Keyboard, Runner};
use crate::keys::{combo_sort, KeySpec, RawKey};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering::*};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    SpamKey,
    SpamHotkey,
    PatHotkey,
    #[allow(dead_code)] // enregistrable depuis l'UI des réglages
    StopHotkey,
    StepKey(usize),
}

impl Target {
    pub fn is_combo(self) -> bool {
        matches!(self, Target::SpamHotkey | Target::PatHotkey | Target::StopHotkey)
    }
}

#[derive(Clone, Debug)]
pub struct Recording {
    pub target: Target,
    pub keys: Vec<KeySpec>,
    down: HashSet<u16>,
}

pub struct Shared {
    pub settings: Mutex<Settings>,
    pub rec: Mutex<Option<Recording>>,
    pub dirty: AtomicBool,
    pub runner: Runner,
    pub flash: Mutex<Option<(&'static str, std::time::Instant)>>,
    ctx: OnceLock<egui::Context>,
    pressed: Mutex<HashSet<u16>>,
    active: Mutex<[bool; 3]>,
}

impl Shared {
    pub fn new(settings: Settings, kb: Arc<dyn Keyboard>) -> Arc<Self> {
        Arc::new(Shared {
            settings: Mutex::new(settings),
            rec: Mutex::new(None),
            dirty: AtomicBool::new(false),
            runner: Runner::new(kb),
            flash: Mutex::new(None),
            ctx: OnceLock::new(),
            pressed: Mutex::new(HashSet::new()),
            active: Mutex::new([false; 3]),
        })
    }

    pub fn set_ctx(&self, ctx: egui::Context) {
        let _ = self.ctx.set(ctx);
    }

    pub fn repaint(&self) {
        if let Some(c) = self.ctx.get() {
            c.request_repaint();
        }
    }

    pub fn mark_dirty(&self) {
        self.dirty.store(true, SeqCst);
    }

    pub fn save_now(&self) {
        let s = self.settings.lock().unwrap().clone();
        config::save(&s);
        self.dirty.store(false, SeqCst);
    }

    // ---------------------------------------------------------------- run
    fn idx(m: Mode) -> u8 {
        (m == Mode::Pattern) as u8
    }

    pub fn is_running(&self, m: Mode) -> bool {
        self.runner.running() && self.runner.mode() == Self::idx(m)
    }

    pub fn toggle(&self, m: Mode) {
        if self.is_running(m) {
            self.runner.stop();
            self.repaint();
        } else {
            self.start(m);
        }
    }

    pub fn stop_if(&self, m: Mode) {
        if self.is_running(m) {
            self.runner.stop();
            self.repaint();
        }
    }

    pub fn start(&self, m: Mode) {
        if self.is_running(m) {
            return;
        }
        let job = {
            let s = self.settings.lock().unwrap();
            match m {
                Mode::Spam => Job::Spam { key: s.spam.key.clone(), interval: config::secs(s.spam.value, s.spam.unit), press: s.press_ms / 1000.0 },
                Mode::Pattern => {
                    if s.pattern.steps.is_empty() {
                        drop(s);
                        *self.flash.lock().unwrap() = Some(("no_steps", std::time::Instant::now()));
                        self.repaint();
                        return;
                    }
                    let steps = s
                        .pattern
                        .steps
                        .iter()
                        .map(|st| match st {
                            Step::Hold { key, value, unit } => CStep::Hold(key.clone(), config::secs(*value, *unit)),
                            Step::Press { key, count, gap } => CStep::Press(key.clone(), (*count).max(1), gap / 1000.0, s.press_ms / 1000.0),
                            Step::Wait { value, unit } => CStep::Wait(config::secs(*value, *unit)),
                        })
                        .collect();
                    Job::Pattern { steps, loops: s.pattern.loops }
                }
            }
        };
        self.runner.start(job);
        self.repaint();
    }

    // ---------------------------------------------------------------- enregistrement
    pub fn begin_record(&self, t: Target) {
        let mut r = self.rec.lock().unwrap();
        if r.as_ref().map(|x| x.target) == Some(t) {
            *r = None; // re-clic = annuler
        } else {
            *r = Some(Recording { target: t, keys: vec![], down: HashSet::new() });
        }
    }

    pub fn cancel_record(&self) {
        *self.rec.lock().unwrap() = None;
    }

    pub fn recording(&self) -> Option<Recording> {
        self.rec.lock().unwrap().clone()
    }

    fn commit(&self, t: Target, keys: Vec<KeySpec>) {
        {
            let mut s = self.settings.lock().unwrap();
            match t {
                Target::SpamKey => s.spam.key = keys[0].clone(),
                Target::SpamHotkey => s.spam.hotkey = keys,
                Target::PatHotkey => s.pattern.hotkey = keys,
                Target::StopHotkey => s.stop_hotkey = keys,
                Target::StepKey(i) => {
                    if let Some(k) = s.pattern.steps.get_mut(i).and_then(|st| st.key_mut()) {
                        *k = keys[0].clone();
                    }
                }
            }
        }
        *self.active.lock().unwrap() = [false; 3];
        self.mark_dirty();
    }

    // ---------------------------------------------------------------- évènements clavier
    pub fn handle(&self, ev: RawKey) {
        let id = ev.key.vk;
        // 1) enregistrement en cours ?
        {
            let mut guard = self.rec.lock().unwrap();
            if let Some(rec) = guard.as_mut() {
                if ev.down {
                    if !rec.keys.iter().any(|k| k.vk == id) {
                        rec.keys.push(ev.key.clone());
                    }
                    rec.down.insert(id);
                    if !rec.target.is_combo() {
                        let (t, k) = (rec.target, vec![ev.key]);
                        *guard = None;
                        drop(guard);
                        self.commit(t, k);
                    }
                } else if rec.target.is_combo() {
                    rec.down.remove(&id);
                    if !rec.keys.is_empty() && rec.down.is_empty() {
                        let (t, k) = (rec.target, combo_sort(rec.keys.clone()));
                        *guard = None;
                        drop(guard);
                        self.commit(t, k);
                    }
                }
                self.repaint();
                return;
            }
        }
        // 2) raccourcis
        {
            let mut p = self.pressed.lock().unwrap();
            if ev.down {
                p.insert(id);
            } else {
                p.remove(&id);
            }
        }
        self.check_bindings();
    }

    fn check_bindings(&self) {
        let (binds, stop_keys, pressed) = {
            let s = self.settings.lock().unwrap();
            (
                [(Mode::Spam, s.spam.hotkey.clone(), s.spam.hk_mode), (Mode::Pattern, s.pattern.hotkey.clone(), s.pattern.hk_mode)],
                s.stop_hotkey.clone(),
                self.pressed.lock().unwrap().clone(),
            )
        };
        // arrêt d'urgence : indépendant du mode
        let stop_on = !stop_keys.is_empty() && stop_keys.iter().all(|k| pressed.contains(&k.vk));
        let stop_changed = {
            let mut a = self.active.lock().unwrap();
            let c = a[2] != stop_on;
            a[2] = stop_on;
            c
        };
        if stop_changed && stop_on {
            self.runner.stop();
            self.repaint();
        }
        for (m, keys, hk) in binds {
            let i = Self::idx(m) as usize;
            let on = !keys.is_empty() && keys.iter().all(|k| pressed.contains(&k.vk));
            let changed = {
                let mut a = self.active.lock().unwrap();
                let c = a[i] != on;
                a[i] = on;
                c
            };
            if !changed {
                continue;
            }
            match (hk, on) {
                (HkMode::Toggle, true) => self.toggle(m),
                (HkMode::Toggle, false) => {}
                (HkMode::Hold, true) => self.start(m),
                (HkMode::Hold, false) => self.stop_if(m),
            }
        }
    }
}

/// Lecture globale du clavier : on interroge l'état des touches toutes les ~8 ms.
/// Hors enregistrement, seules les touches des raccourcis sont surveillées (coût CPU quasi nul).
pub fn spawn_poller(shared: Arc<Shared>) {
    std::thread::spawn(move || {
        let mut prev = [false; 256];
        let mut was_watched = [false; 256];
        loop {
            let recording = shared.rec.lock().unwrap().is_some();
            let mut watch = [false; 256];
            if recording {
                for (vk, w) in watch.iter_mut().enumerate().take(0xFF).skip(0x08) {
                    // ni souris (0x01-0x07), ni modificateurs génériques (on garde gauche/droite)
                    if !matches!(vk, 0x10..=0x12) {
                        *w = true;
                    }
                }
            } else {
                let s = shared.settings.lock().unwrap();
                for k in s.spam.hotkey.iter().chain(s.pattern.hotkey.iter()).chain(s.stop_hotkey.iter()) {
                    watch[(k.vk & 0xFF) as usize] = true;
                }
            }
            for vk in 0..256usize {
                if !watch[vk] {
                    prev[vk] = false;
                    was_watched[vk] = false;
                    continue;
                }
                let (d, tapped) = crate::platform::key_poll(vk as u16);
                if !was_watched[vk] {
                    // première lecture : on prend l'état actuel sans rien déclencher (ignore l'historique)
                    was_watched[vk] = true;
                    prev[vk] = d;
                    continue;
                }
                if d != prev[vk] {
                    prev[vk] = d;
                    let key = if d { crate::platform::key_spec(vk as u16) } else { KeySpec::vk(vk as u16) };
                    shared.handle(RawKey { key, down: d });
                } else if !d && tapped {
                    // appui plus court que l'intervalle de lecture : on le rejoue
                    shared.handle(RawKey { key: crate::platform::key_spec(vk as u16), down: true });
                    shared.handle(RawKey { key: KeySpec::vk(vk as u16), down: false });
                }
            }
            // 10 ms pendant la capture, 15 ms sinon (un appui humain dure > 30 ms)
            std::thread::sleep(std::time::Duration::from_millis(if recording { 10 } else { 15 }));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tests::Log;

    fn ev(vk: u16, down: bool) -> RawKey {
        RawKey { key: KeySpec::vk(vk), down }
    }

    fn setup() -> (Arc<Shared>, Arc<Log>) {
        let log = Arc::new(Log::default());
        (Shared::new(Settings::default(), log.clone()), log)
    }

    #[test]
    fn record_combo_then_toggle_and_hold() {
        let (sh, log) = setup();
        sh.begin_record(Target::SpamHotkey);
        for (vk, d) in [(0x59, true), (0xA2, true), (0x59, false), (0xA2, false)] {
            sh.handle(ev(vk, d));
        }
        let hk: Vec<u16> = sh.settings.lock().unwrap().spam.hotkey.iter().map(|k| k.vk).collect();
        assert_eq!(hk, vec![0xA2, 0x59], "ctrl trié en premier");
        assert!(sh.recording().is_none());

        // bascule
        sh.handle(ev(0xA2, true));
        sh.handle(ev(0x59, true));
        assert!(sh.is_running(Mode::Spam));
        sh.handle(ev(0x59, false));
        sh.handle(ev(0xA2, false));
        assert!(sh.is_running(Mode::Spam), "reste actif en bascule");
        sh.handle(ev(0xA2, true));
        sh.handle(ev(0x59, true));
        assert!(!sh.is_running(Mode::Spam));
        sh.handle(ev(0x59, false));
        sh.handle(ev(0xA2, false));
        assert!(!log.0.lock().unwrap().is_empty());

        // maintien
        sh.settings.lock().unwrap().spam.hk_mode = HkMode::Hold;
        sh.handle(ev(0xA2, true));
        sh.handle(ev(0x59, true));
        assert!(sh.is_running(Mode::Spam));
        sh.handle(ev(0x59, false));
        assert!(!sh.is_running(Mode::Spam));
    }

    #[test]
    fn record_single_key_and_step_key() {
        let (sh, _) = setup();
        sh.begin_record(Target::SpamKey);
        sh.handle(ev(0x51, true));
        assert_eq!(sh.settings.lock().unwrap().spam.key.vk, 0x51);
        sh.handle(ev(0x51, false));

        sh.settings.lock().unwrap().pattern.steps = config::example_steps();
        sh.begin_record(Target::StepKey(1));
        sh.handle(ev(0x47, true));
        let s = sh.settings.lock().unwrap();
        assert!(matches!(&s.pattern.steps[1], Step::Hold { key, .. } if key.vk == 0x47));
    }

    #[test]
    fn reclick_cancels_and_f7_runs_pattern() {
        let (sh, _) = setup();
        sh.begin_record(Target::PatHotkey);
        sh.begin_record(Target::PatHotkey);
        assert!(sh.recording().is_none());
        sh.handle(ev(0x76, true)); // F7 sans étapes -> message
        assert!(!sh.is_running(Mode::Pattern));
        assert!(sh.flash.lock().unwrap().is_some());
        sh.handle(ev(0x76, false));
        sh.settings.lock().unwrap().pattern.steps = config::example_steps();
        sh.handle(ev(0x76, true));
        assert!(sh.is_running(Mode::Pattern));
        sh.handle(ev(0x76, false));
        sh.handle(ev(0x76, true));
        assert!(!sh.is_running(Mode::Pattern));
    }

    #[test]
    fn stop_hotkey_stops_any_mode() {
        let (sh, _) = setup();
        assert_eq!(sh.settings.lock().unwrap().stop_hotkey[0].vk, 0x77);
        sh.settings.lock().unwrap().spam.hk_mode = HkMode::Hold;
        sh.start(Mode::Spam);
        assert!(sh.is_running(Mode::Spam));
        sh.handle(ev(0x77, true));
        assert!(!sh.runner.running());
        sh.handle(ev(0x77, false));

        sh.settings.lock().unwrap().pattern.steps = config::example_steps();
        sh.start(Mode::Pattern);
        assert!(sh.is_running(Mode::Pattern));
        sh.handle(ev(0x77, true));
        assert!(!sh.runner.running());
        sh.handle(ev(0x77, false));

        // enregistrement d'un combo d'arrêt
        sh.begin_record(Target::StopHotkey);
        for (vk, d) in [(0xA2, true), (0x78, true), (0x78, false), (0xA2, false)] {
            sh.handle(ev(vk, d));
        }
        let hk: Vec<u16> = sh.settings.lock().unwrap().stop_hotkey.iter().map(|k| k.vk).collect();
        assert_eq!(hk, vec![0xA2, 0x78]);
    }
}
