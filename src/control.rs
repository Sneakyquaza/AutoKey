//! État partagé + contrôleur : reçoit les touches physiques (hook global), gère
//! l'enregistrement des raccourcis et le démarrage/arrêt, même fenêtre en arrière-plan.
use crate::config::{self, HkMode, Mode, Profile, Settings, Step};
use crate::engine::{CStep, Job, Keyboard, Runner};
use crate::keys::{combo_sort, KeySpec, RawKey};
use crate::macro_rec::{self, MacroRec};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering::*};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    SpamKey,
    SpamHotkey,
    PatHotkey,
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
    /// Capture de macro en cours (exclusive avec `rec`)
    pub macro_rec: Mutex<Option<MacroRec>>,
    /// Résultat d'une capture terminée, en attente de reprise par l'UI
    macro_out: Mutex<Option<Vec<Step>>>,
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
            macro_rec: Mutex::new(None),
            macro_out: Mutex::new(None),
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
        // en cas d'échec, `dirty` reste levé : nouvelle tentative au prochain cycle
        if config::save(&s).is_ok() {
            self.dirty.store(false, SeqCst);
        }
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
        if self.is_running(m) || self.macro_active() {
            return;
        }
        let bind;
        let job = {
            let s = self.settings.lock().unwrap();
            self.runner.beep.store(s.beep, SeqCst);
            bind = s.bind_window;
            match m {
                Mode::Spam => Job::Spam {
                    key: s.spam.key.clone(),
                    interval: config::secs(s.spam.value, s.spam.unit),
                    press: s.press_ms / 1000.0,
                    jitter: s.jitter_pct / 100.0,
                    limit: s.spam.limit,
                },
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
                    Job::Pattern { steps, loops: s.pattern.loops, jitter: s.jitter_pct / 100.0 }
                }
            }
        };
        // Liaison : fenêtre au premier plan à cet instant ; si c'est AutoKey (démarrage par le bouton
        // de l'UI), on attend la prochaine fenêtre étrangère au premier plan et on la verrouille.
        let focus = if bind { crate::platform::bind_foreground() } else { None };
        self.runner.start_bound(job, focus);
        self.repaint();
    }

    // ---------------------------------------------------------------- enregistrement
    pub fn begin_record(&self, t: Target) {
        let mut r = self.rec.lock().unwrap();
        if r.as_ref().map(|x| x.target) == Some(t) {
            *r = None; // re-clic = annuler
        } else {
            *self.macro_rec.lock().unwrap() = None;
            *r = Some(Recording { target: t, keys: vec![], down: HashSet::new() });
        }
        drop(r);
        self.reset_keys();
    }

    // ---------------------------------------------------------------- capture de macro
    pub fn macro_active(&self) -> bool {
        self.macro_rec.lock().unwrap().is_some()
    }

    /// Instantané de la capture (nombre d'évènements, durée) pour l'UI
    pub fn macro_progress(&self) -> Option<(usize, std::time::Duration)> {
        self.macro_rec.lock().unwrap().as_ref().map(|m| (m.events.len(), m.started.elapsed()))
    }

    /// Démarre (ou arrête, si déjà en cours) la capture d'une macro. Refusé si une tâche tourne.
    pub fn toggle_macro(&self) {
        if self.macro_active() {
            self.stop_macro(Instant::now());
            return;
        }
        if self.runner.running() {
            self.flash_msg("macro_busy");
            return;
        }
        self.cancel_record();
        self.reset_keys();
        *self.macro_rec.lock().unwrap() = Some(MacroRec { events: vec![], started: Instant::now() });
        self.repaint();
    }

    pub fn cancel_macro(&self) {
        *self.macro_rec.lock().unwrap() = None;
        self.reset_keys();
    }

    /// Reprend le résultat d'une capture terminée (non vide)
    pub fn take_macro_out(&self) -> Option<Vec<Step>> {
        self.macro_out.lock().unwrap().take()
    }

    /// Les raccourcis ne doivent pas se déclencher sur l'état « touches enfoncées » d'avant/après la capture
    fn reset_keys(&self) {
        self.pressed.lock().unwrap().clear();
        *self.active.lock().unwrap() = [false; 3];
    }

    fn stop_macro(&self, now: Instant) {
        let Some(m) = self.macro_rec.lock().unwrap().take() else { return };
        self.reset_keys();
        let (steps, truncated) = macro_rec::to_steps(&m.events, now);
        if steps.is_empty() {
            self.flash_msg("macro_empty");
            return;
        }
        if truncated {
            self.flash_msg("macro_trunc");
        }
        *self.macro_out.lock().unwrap() = Some(steps);
        self.repaint();
    }

    /// Évènement clavier pendant la capture. Vrai si la capture est active (évènement consommé).
    /// Ni raccourci spam/pattern ne se déclenche ; ces touches ne sont pas enregistrées.
    /// Le raccourci d'arrêt termine la capture ; les touches de tous les raccourcis (y compris
    /// celles d'un combo, ex. Ctrl) ne sont jamais enregistrées.
    fn handle_macro(&self, ev: &RawKey, now: Instant) -> bool {
        if !self.macro_active() {
            return false;
        }
        let (stop_keys, hk_keys): (Vec<u16>, Vec<u16>) = {
            let s = self.settings.lock().unwrap();
            (s.stop_hotkey.iter().map(|k| k.vk).collect(), s.spam.hotkey.iter().chain(s.pattern.hotkey.iter()).map(|k| k.vk).collect())
        };
        let id = ev.key.vk;
        let full = {
            let mut p = self.pressed.lock().unwrap();
            if ev.down {
                p.insert(id);
            } else {
                p.remove(&id);
            }
            !stop_keys.is_empty() && stop_keys.iter().all(|v| p.contains(v))
        };
        let mut g = self.macro_rec.lock().unwrap();
        let Some(m) = g.as_mut() else { return true };
        if ev.down && full && stop_keys.contains(&id) {
            drop(g);
            self.stop_macro(now);
        } else if m.events.len() < macro_rec::MAX_EVENTS && !hk_keys.contains(&id) && !stop_keys.contains(&id) {
            // une touche de raccourci n'est jamais enregistrée : une étape la contenant violerait `conflicts`
            m.events.push((now, ev.key.clone(), ev.down));
            drop(g);
            self.repaint();
        }
        true
    }

    pub fn cancel_record(&self) {
        *self.rec.lock().unwrap() = None;
    }

    pub fn recording(&self) -> Option<Recording> {
        self.rec.lock().unwrap().clone()
    }

    /// Vrai si l'enregistrement de `keys` pour `t` créerait un conflit : deux raccourcis identiques,
    /// un raccourci contenant une touche de spam/d'étape (nos propres frappes sont ignorées), ou
    /// une touche de spam/d'étape appartenant à un raccourci.
    pub fn conflicts(s: &Settings, t: Target, keys: &[KeySpec]) -> bool {
        let hotkeys: [(Target, &Vec<KeySpec>); 3] =
            [(Target::SpamHotkey, &s.spam.hotkey), (Target::PatHotkey, &s.pattern.hotkey), (Target::StopHotkey, &s.stop_hotkey)];
        let has = |v: &[KeySpec], vk: u16| v.iter().any(|k| k.vk == vk);
        if !t.is_combo() {
            return keys.first().is_some_and(|k| hotkeys.iter().any(|(_, h)| has(h, k.vk)));
        }
        if keys.is_empty() {
            return false;
        }
        let same = |a: &[KeySpec], b: &[KeySpec]| a.len() == b.len() && a.iter().all(|k| has(b, k.vk));
        let action_keys = std::iter::once(&s.spam.key).chain(s.pattern.steps.iter().filter_map(|st| match st {
            Step::Hold { key, .. } | Step::Press { key, .. } => Some(key),
            Step::Wait { .. } => None,
        }));
        hotkeys.iter().any(|(o, h)| *o != t && same(h, keys)) || action_keys.into_iter().any(|k| has(keys, k.vk))
    }

    // ---------------------------------------------------------------- import / réinitialisation
    /// Configuration complète importée : ≤ 1 Mo, JSON valide, bornée par `sanitize`, sans conflit de raccourcis
    pub fn parse_import(txt: &str) -> Result<Settings, &'static str> {
        if txt.len() as u64 > config::MAX_FILE {
            return Err("bad_file");
        }
        let mut s: Settings = serde_json::from_str(config::strip_bom(txt)).map_err(|_| "bad_file")?;
        config::sanitize(&mut s);
        let hk = [(Target::SpamHotkey, &s.spam.hotkey), (Target::PatHotkey, &s.pattern.hotkey), (Target::StopHotkey, &s.stop_hotkey)];
        let hk_bad = hk.iter().any(|(t, k)| Self::conflicts(&s, *t, k));
        let keys = std::iter::once(&s.spam.key).chain(s.pattern.steps.iter().filter_map(|st| match st {
            Step::Hold { key, .. } | Step::Press { key, .. } => Some(key),
            Step::Wait { .. } => None,
        }));
        let key_bad = keys.into_iter().any(|k| Self::conflicts(&s, Target::SpamKey, std::slice::from_ref(k)));
        if hk_bad || key_bad {
            return Err("hk_conflict");
        }
        Ok(s)
    }

    /// Remplace tous les réglages (import, réinitialisation) : tâche et écoute en cours arrêtées
    pub fn replace_settings(&self, new: Settings) {
        self.runner.stop();
        self.cancel_record();
        self.cancel_macro();
        self.reset_keys();
        *self.settings.lock().unwrap() = new;
        self.mark_dirty();
        self.repaint();
    }

    // ---------------------------------------------------------------- profils
    fn flash_msg(&self, k: &'static str) {
        *self.flash.lock().unwrap() = Some((k, std::time::Instant::now()));
        self.repaint();
    }

    /// Vrai si une touche d'action du profil (spam ou étape) est un raccourci en vigueur
    fn profile_conflicts(s: &Settings, p: &Profile) -> bool {
        let keys = std::iter::once(&p.spam.key).chain(p.pattern.steps.iter().filter_map(|st| match st {
            Step::Hold { key, .. } | Step::Press { key, .. } => Some(key),
            Step::Wait { .. } => None,
        }));
        keys.into_iter().any(|k| Self::conflicts(s, Target::SpamKey, std::slice::from_ref(k)))
    }

    /// Instantané du réglage courant (spam + pattern) sous un nom
    fn snapshot(s: &Settings, name: String) -> Profile {
        Profile { name, spam: s.spam.clone(), pattern: s.pattern.clone() }
    }

    /// Enregistre l'état courant : nouveau profil (`None`) ou écrasement de `idx`
    pub fn save_profile(&self, idx: Option<usize>, default_name: String) {
        let err = {
            let mut s = self.settings.lock().unwrap();
            if s.pattern.steps.len() > config::MAX_PROFILE_STEPS {
                Some("prof_too_big")
            } else {
                match idx {
                    Some(i) if i < s.profiles.len() => {
                        let name = s.profiles[i].name.clone();
                        s.profiles[i] = Self::snapshot(&s, name);
                        None
                    }
                    Some(_) => None,
                    None if s.profiles.len() >= config::MAX_PROFILES => Some("prof_full"),
                    None => {
                        let p = Self::snapshot(&s, config::clean_profile_name(&default_name, "Profil"));
                        s.profiles.push(p);
                        s.active_profile = Some(s.profiles.len() - 1);
                        None
                    }
                }
            }
        };
        match err {
            Some(k) => self.flash_msg(k),
            None => self.mark_dirty(),
        }
    }

    /// Charge le profil `i` (tout sauf les raccourcis). Refusé en cas de conflit de touches ;
    /// la tâche en cours est arrêtée d'abord (aucune touche laissée enfoncée).
    pub fn load_profile(&self, i: usize) {
        let conflict = {
            let s = self.settings.lock().unwrap();
            match s.profiles.get(i) {
                None => return,
                Some(p) => Self::profile_conflicts(&s, p),
            }
        };
        if conflict {
            self.flash_msg("hk_conflict");
            return;
        }
        self.runner.stop();
        self.cancel_record();
        self.cancel_macro();
        {
            let mut s = self.settings.lock().unwrap();
            let Some(p) = s.profiles.get(i).cloned() else { return };
            let (hk, mode) = (s.spam.hotkey.clone(), s.spam.hk_mode);
            s.spam = p.spam;
            s.spam.hotkey = hk;
            s.spam.hk_mode = mode;
            let (hk, mode) = (s.pattern.hotkey.clone(), s.pattern.hk_mode);
            s.pattern = p.pattern;
            s.pattern.hotkey = hk;
            s.pattern.hk_mode = mode;
            s.active_profile = Some(i);
        }
        self.mark_dirty();
    }

    pub fn rename_profile(&self, i: usize, name: &str) {
        {
            let mut s = self.settings.lock().unwrap();
            let Some(p) = s.profiles.get_mut(i) else { return };
            p.name = config::clean_profile_name(name, &p.name.clone());
        }
        self.mark_dirty();
    }

    pub fn delete_profile(&self, i: usize) {
        {
            let mut s = self.settings.lock().unwrap();
            if i >= s.profiles.len() {
                return;
            }
            s.profiles.remove(i);
            s.active_profile = match s.active_profile {
                Some(a) if a == i => None,
                Some(a) if a > i => Some(a - 1),
                a => a,
            };
        }
        self.mark_dirty();
    }

    fn commit(&self, t: Target, mut keys: Vec<KeySpec>) {
        keys.truncate(config::MAX_COMBO);
        {
            let mut s = self.settings.lock().unwrap();
            if Self::conflicts(&s, t, &keys) {
                drop(s);
                *self.flash.lock().unwrap() = Some(("hk_conflict", std::time::Instant::now()));
                self.repaint();
                return;
            }
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
        self.reset_keys();
        self.mark_dirty();
    }

    // ---------------------------------------------------------------- évènements clavier
    pub fn handle(&self, ev: RawKey) {
        self.handle_at(ev, Instant::now());
    }

    fn handle_at(&self, ev: RawKey, now: Instant) {
        if self.handle_macro(&ev, now) {
            return;
        }
        let id = ev.key.vk;
        // 1) enregistrement en cours ?
        {
            let mut guard = self.rec.lock().unwrap();
            if let Some(rec) = guard.as_mut() {
                if ev.down {
                    if rec.keys.len() < config::MAX_COMBO && !rec.keys.iter().any(|k| k.vk == id) {
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
        // [spam, pattern, arrêt] : un raccourci n'est « on » que si aucun raccourci plus spécifique
        // (sur-ensemble strict de touches) n'est lui aussi entièrement pressé.
        let sets: [&[KeySpec]; 3] = [&binds[0].1, &binds[1].1, &stop_keys];
        let full = sets.map(|k| !k.is_empty() && k.iter().all(|x| pressed.contains(&x.vk)));
        let ons: [bool; 3] = std::array::from_fn(|i| {
            full[i] && !(0..3).any(|j| j != i && full[j] && sets[j].len() > sets[i].len() && sets[i].iter().all(|a| sets[j].iter().any(|b| b.vk == a.vk)))
        });
        // arrêt d'urgence : indépendant du mode
        let stop_changed = {
            let mut a = self.active.lock().unwrap();
            let c = a[2] != ons[2];
            a[2] = ons[2];
            c
        };
        if stop_changed && ons[2] {
            self.runner.stop();
            self.repaint();
        }
        for (m, _, hk) in binds {
            let i = Self::idx(m) as usize;
            let on = ons[i];
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
            let recording = shared.rec.lock().unwrap().is_some() || shared.macro_active();
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
    use crate::config::SpamLimit;
    use crate::engine::tests::Log;

    fn ev(vk: u16, down: bool) -> RawKey {
        RawKey { key: KeySpec::vk(vk), down }
    }

    fn setup() -> (Arc<Shared>, Arc<Log>) {
        let log = Arc::new(Log::default());
        (Shared::new(Settings::default(), log.clone()), log)
    }

    #[test]
    fn import_valid_and_defaults_roundtrip() {
        let txt = serde_json::to_string_pretty(&Settings::default()).unwrap();
        assert_eq!(Shared::parse_import(&txt).unwrap(), Settings::default());
        // config 3.3.1 partielle : champs manquants = défauts
        let s = Shared::parse_import(r#"{"pin":true,"press_ms":40.0}"#).unwrap();
        assert!(s.pin && s.auto_check_updates && !s.start_minimized);
    }

    #[test]
    fn import_hostile_is_bounded_or_rejected() {
        assert_eq!(Shared::parse_import("pas du json"), Err("bad_file"));
        assert_eq!(Shared::parse_import(r#"{"press_ms":"x"}"#), Err("bad_file"));
        assert_eq!(Shared::parse_import(""), Err("bad_file"));
        assert!(Shared::parse_import("\u{feff}{\"pin\":true}").unwrap().pin);
        // valeurs absurdes : ramenées dans les bornes
        let s = Shared::parse_import(r#"{"press_ms":1e12,"jitter_pct":-5,"stop_hotkey":[]}"#).unwrap();
        assert!(s.press_ms <= config::MAX_PRESS_MS && s.jitter_pct >= 0.0);
    }

    #[test]
    fn import_too_big_rejected() {
        let big = format!(r#"{{"pad":"{}"}}"#, "a".repeat(config::MAX_FILE as usize));
        assert_eq!(Shared::parse_import(&big), Err("bad_file"));
    }

    #[test]
    fn import_conflict_rejected() {
        let mut s = Settings::default();
        s.spam.hotkey = vec![KeySpec::vk(0x75)];
        s.pattern.hotkey = vec![KeySpec::vk(0x75)];
        let txt = serde_json::to_string(&s).unwrap();
        assert_eq!(Shared::parse_import(&txt), Err("hk_conflict"));
        // touche du spam = raccourci du pattern
        let mut s = Settings::default();
        s.spam.key = s.pattern.hotkey[0].clone();
        let txt = serde_json::to_string(&s).unwrap();
        assert_eq!(Shared::parse_import(&txt), Err("hk_conflict"));
    }

    #[test]
    fn replace_settings_stops_and_marks_dirty() {
        let (sh, _) = setup();
        let s = Settings { pin: true, ..Settings::default() };
        sh.replace_settings(s);
        assert!(sh.settings.lock().unwrap().pin);
        assert!(sh.dirty.load(SeqCst));
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
        std::thread::sleep(std::time::Duration::from_millis(50)); // laisse partir une frappe avant l'arrêt
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
    fn record_combo_capped_at_six() {
        let (sh, _) = setup();
        sh.begin_record(Target::SpamHotkey);
        for vk in 0x31..0x3C {
            sh.handle(ev(vk, true));
        }
        for vk in 0x31..0x3C {
            sh.handle(ev(vk, false));
        }
        assert_eq!(sh.settings.lock().unwrap().spam.hotkey.len(), config::MAX_COMBO);
    }

    #[test]
    fn begin_record_clears_stale_pressed() {
        let (sh, _) = setup();
        sh.settings.lock().unwrap().spam.hotkey = vec![KeySpec::vk(0xA2), KeySpec::vk(0x78)];
        sh.handle(ev(0xA2, true)); // Ctrl tenu
        sh.begin_record(Target::PatHotkey);
        sh.cancel_record();
        // relâchement de Ctrl avalé pendant l'enregistrement
        sh.handle(ev(0x78, true));
        assert!(!sh.is_running(Mode::Spam), "F9 seule ne doit pas déclencher");
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

    #[test]
    fn conflicting_hotkeys_rejected() {
        let (sh, _) = setup();
        let rec = |t: Target, keys: &[u16]| {
            sh.begin_record(t);
            for &vk in keys {
                sh.handle(ev(vk, true));
            }
            for &vk in keys {
                sh.handle(ev(vk, false));
            }
        };
        let hk = |t: Target| -> Vec<u16> {
            let s = sh.settings.lock().unwrap();
            let v = match t {
                Target::SpamHotkey => &s.spam.hotkey,
                Target::PatHotkey => &s.pattern.hotkey,
                _ => &s.stop_hotkey,
            };
            v.iter().map(|k| k.vk).collect()
        };
        // même ensemble qu'un autre raccourci (F7 = motif)
        rec(Target::SpamHotkey, &[0x76]);
        assert_eq!(hk(Target::SpamHotkey), vec![0x75]);
        assert_eq!(sh.flash.lock().unwrap().map(|f| f.0), Some("hk_conflict"));
        *sh.flash.lock().unwrap() = None;
        // idem pour l'arrêt (F6 = spam)
        rec(Target::StopHotkey, &[0x75]);
        assert_eq!(hk(Target::StopHotkey), vec![0x77]);
        // raccourci contenant la touche de spam ('E' = 0x45)
        rec(Target::PatHotkey, &[0xA2, 0x45]);
        assert_eq!(hk(Target::PatHotkey), vec![0x76]);
        // raccourci contenant une touche d'étape
        sh.settings.lock().unwrap().pattern.steps = config::example_steps(); // z, x, z
        rec(Target::PatHotkey, &[0xA2, 0x58]);
        assert_eq!(hk(Target::PatHotkey), vec![0x76]);
        // touche de spam / d'étape appartenant à un raccourci
        sh.flash.lock().unwrap().take();
        rec(Target::SpamKey, &[0x77]);
        assert_eq!(sh.settings.lock().unwrap().spam.key.vk, 0x45);
        rec(Target::StepKey(1), &[0x76]);
        assert!(matches!(&sh.settings.lock().unwrap().pattern.steps[1], Step::Hold { key, .. } if key.vk == 0x58));
        assert_eq!(sh.flash.lock().unwrap().map(|f| f.0), Some("hk_conflict"));
        // cas valides inchangés
        rec(Target::StopHotkey, &[0xA2, 0x75]);
        assert_eq!(hk(Target::StopHotkey), vec![0xA2, 0x75]);
        rec(Target::SpamKey, &[0x51]);
        assert_eq!(sh.settings.lock().unwrap().spam.key.vk, 0x51);
    }

    #[test]
    fn most_specific_combo_wins() {
        let (sh, _) = setup();
        // Ctrl+F6 pour le spam, F6 pour le motif (steps non vides)
        {
            let mut s = sh.settings.lock().unwrap();
            s.spam.hotkey = vec![KeySpec::vk(0xA2), KeySpec::vk(0x75)];
            s.pattern.hotkey = vec![KeySpec::vk(0x75)];
            s.pattern.steps = config::example_steps();
        }
        sh.handle(ev(0xA2, true));
        sh.handle(ev(0x75, true));
        assert!(sh.is_running(Mode::Spam));
        assert!(!sh.is_running(Mode::Pattern), "F6 seul ne doit pas se déclencher");
        sh.handle(ev(0x75, false));
        sh.handle(ev(0xA2, false));
        sh.runner.stop();
        // F6 seul + touche non liée tenue (Shift) : le raccourci fonctionne
        sh.handle(ev(0xA0, true));
        sh.handle(ev(0x75, true));
        assert!(sh.is_running(Mode::Pattern));
        sh.handle(ev(0x75, false));
        sh.handle(ev(0xA0, false));
    }

    fn profile(name: &str, key: u16) -> Profile {
        let mut p = Profile { name: name.into(), ..Default::default() };
        p.spam.key = KeySpec::vk(key);
        p.spam.value = 250.0;
        p.pattern.steps = vec![Step::Hold { key: KeySpec::vk(0x5A), value: 1.0, unit: config::Unit::S }];
        p.pattern.hotkey = vec![KeySpec::vk(0x70)];
        p
    }

    #[test]
    fn load_profile_keeps_hotkeys() {
        let (sh, _) = setup();
        {
            let mut s = sh.settings.lock().unwrap();
            s.profiles = vec![profile("A", 0x51)];
            s.spam.hk_mode = HkMode::Hold;
        }
        sh.load_profile(0);
        let s = sh.settings.lock().unwrap();
        assert_eq!((s.spam.key.vk, s.spam.value), (0x51, 250.0));
        assert_eq!(s.pattern.steps.len(), 1);
        assert_eq!(s.spam.hotkey, vec![KeySpec::vk(0x75)], "raccourci spam conservé");
        assert_eq!(s.pattern.hotkey, vec![KeySpec::vk(0x76)], "raccourci pattern conservé");
        assert_eq!(s.spam.hk_mode, HkMode::Hold);
        assert_eq!(s.active_profile, Some(0));
    }

    #[test]
    fn load_profile_refused_on_conflict() {
        let (sh, _) = setup();
        sh.settings.lock().unwrap().profiles = vec![profile("A", 0x75)]; // F6 = raccourci spam
        let before = sh.settings.lock().unwrap().spam.key.clone();
        sh.load_profile(0);
        let s = sh.settings.lock().unwrap();
        assert_eq!(s.spam.key, before);
        assert_eq!(s.active_profile, None);
        assert_eq!(sh.flash.lock().unwrap().as_ref().map(|f| f.0), Some("hk_conflict"));
        drop(s);
        // touche d'étape = raccourci d'arrêt (F8)
        let mut p = profile("B", 0x51);
        p.pattern.steps = vec![Step::Press { key: KeySpec::vk(0x77), count: 1, gap: 0.0 }];
        sh.settings.lock().unwrap().profiles = vec![p];
        sh.load_profile(0);
        assert_eq!(sh.settings.lock().unwrap().active_profile, None);
    }

    #[test]
    fn load_profile_stops_running_task() {
        let (sh, _) = setup();
        sh.settings.lock().unwrap().profiles = vec![profile("A", 0x51)];
        sh.start(Mode::Spam);
        assert!(sh.is_running(Mode::Spam));
        sh.load_profile(0);
        assert!(!sh.is_running(Mode::Spam));
    }

    #[test]
    fn save_rename_delete_profiles() {
        let (sh, _) = setup();
        sh.save_profile(None, "Un".into());
        sh.save_profile(None, "Deux".into());
        sh.settings.lock().unwrap().spam.value = 777.0;
        sh.save_profile(Some(0), String::new());
        sh.rename_profile(0, "  Renommé  ");
        {
            let s = sh.settings.lock().unwrap();
            assert_eq!((s.profiles[0].name.as_str(), s.profiles[0].spam.value), ("Renommé", 777.0));
            assert_eq!(s.active_profile, Some(1));
        }
        sh.delete_profile(0);
        {
            let s = sh.settings.lock().unwrap();
            assert_eq!((s.profiles.len(), s.active_profile), (1, Some(0)));
        }
        sh.delete_profile(0);
        assert_eq!(sh.settings.lock().unwrap().active_profile, None);
        // trop d'étapes pour un profil : refusé
        sh.settings.lock().unwrap().pattern.steps = vec![Step::new_wait(); config::MAX_PROFILE_STEPS + 1];
        sh.save_profile(None, "X".into());
        assert!(sh.settings.lock().unwrap().profiles.is_empty());
        assert_eq!(sh.flash.lock().unwrap().as_ref().map(|f| f.0), Some("prof_too_big"));
    }

    #[test]
    fn macro_capture_ignores_hotkeys_and_stop_key() {
        let (sh, _) = setup();
        sh.settings.lock().unwrap().pattern.steps = config::example_steps();
        sh.toggle_macro();
        assert!(sh.macro_active());
        let t0 = Instant::now();
        let at = |ms: u64| t0 + std::time::Duration::from_millis(ms);
        // F6 (raccourci spam) et F7 (pattern) : ni déclenchés ni enregistrés
        sh.handle_at(ev(0x75, true), at(0));
        sh.handle_at(ev(0x75, false), at(30));
        sh.handle_at(ev(0x76, true), at(40));
        sh.handle_at(ev(0x76, false), at(60));
        assert!(!sh.runner.running());
        // Échap est enregistrée normalement, puis une touche quelconque
        sh.handle_at(ev(0x1B, true), at(100));
        sh.handle_at(ev(0x1B, false), at(150));
        sh.handle_at(ev(0x41, true), at(600));
        sh.handle_at(ev(0x41, false), at(1100));
        assert_eq!(sh.macro_progress().unwrap().0, 4);
        // F8 (arrêt) : termine la capture, absente de la macro
        sh.handle_at(ev(0x77, true), at(1500));
        assert!(!sh.macro_active());
        assert!(!sh.runner.running());
        let steps = sh.take_macro_out().unwrap();
        assert_eq!(steps.len(), 3, "{steps:?}");
        assert!(matches!(&steps[0], Step::Press { key, count: 1, .. } if key.vk == 0x1B));
        assert!(matches!(&steps[1], Step::Wait { .. }));
        assert!(matches!(&steps[2], Step::Hold { key, .. } if key.vk == 0x41));
        sh.handle_at(ev(0x77, false), at(1520));
        // après la capture, les raccourcis refonctionnent
        sh.handle_at(ev(0x76, true), at(2000));
        assert!(sh.is_running(Mode::Pattern));
    }

    #[test]
    fn macro_capture_never_records_hotkey_keys() {
        let (sh, _) = setup();
        sh.settings.lock().unwrap().stop_hotkey = vec![KeySpec::vk(0xA2), KeySpec::vk(0x77)]; // Ctrl+F8
        sh.toggle_macro();
        let t0 = Instant::now();
        let at = |ms: u64| t0 + std::time::Duration::from_millis(ms);
        for (vk, d, ms) in [(0xA2, true, 0), (0x43, true, 10), (0x43, false, 60), (0xA2, false, 100), (0xA2, true, 400), (0x77, true, 450)] {
            sh.handle_at(ev(vk, d), at(ms));
        }
        assert!(!sh.macro_active());
        let steps = sh.take_macro_out().unwrap();
        assert!(steps.iter().all(|s| !matches!(s, Step::Hold { key, .. } | Step::Press { key, .. } if key.vk == 0xA2 || key.vk == 0x77)), "{steps:?}");
    }

    #[test]
    fn macro_capture_empty_and_busy() {
        let (sh, _) = setup();
        sh.toggle_macro();
        sh.handle(ev(0x77, true)); // arrêt immédiat : rien enregistré
        assert!(!sh.macro_active());
        assert!(sh.take_macro_out().is_none());
        assert_eq!(sh.flash.lock().unwrap().as_ref().map(|f| f.0), Some("macro_empty"));
        sh.handle(ev(0x77, false));
        // tâche en cours : démarrage refusé
        sh.start(Mode::Spam);
        sh.toggle_macro();
        assert!(!sh.macro_active());
        assert_eq!(sh.flash.lock().unwrap().as_ref().map(|f| f.0), Some("macro_busy"));
        sh.runner.stop();
        // re-clic = arrêt par bouton
        sh.toggle_macro();
        sh.handle_at(ev(0x41, true), Instant::now());
        sh.toggle_macro();
        assert!(!sh.macro_active());
        assert_eq!(sh.take_macro_out().unwrap().len(), 1);
    }

    #[test]
    fn start_refused_during_macro_capture() {
        let (sh, log) = setup();
        sh.toggle_macro();
        assert!(sh.macro_active());
        sh.start(Mode::Spam);
        sh.start(Mode::Pattern);
        sh.toggle(Mode::Spam);
        std::thread::sleep(std::time::Duration::from_millis(60));
        assert!(!sh.runner.running());
        assert!(log.0.lock().unwrap().is_empty());
        assert!(sh.macro_active(), "la capture continue");
    }

    #[test]
    fn load_profile_cancels_macro_capture() {
        let (sh, _) = setup();
        sh.settings.lock().unwrap().profiles = vec![profile("A", 0x51)];
        sh.toggle_macro();
        sh.handle_at(ev(0x41, true), Instant::now());
        assert!(sh.macro_active());
        sh.load_profile(0);
        assert!(!sh.macro_active());
        assert!(sh.take_macro_out().is_none(), "rien d'enregistré");
        assert_eq!(sh.settings.lock().unwrap().active_profile, Some(0));
    }

    #[test]
    fn import_replaces_settings_and_stops_running_task() {
        let (sh, _) = setup();
        sh.start(Mode::Spam);
        assert!(sh.is_running(Mode::Spam));
        let mut src = Settings::default();
        src.spam.key = KeySpec::vk(0x51);
        src.pin = true;
        src.jitter_pct = 12.0;
        src.spam.limit = SpamLimit::Count(42);
        let txt = serde_json::to_string(&src).unwrap();
        let new = Shared::parse_import(&txt).unwrap();
        sh.begin_record(Target::SpamKey);
        sh.replace_settings(new);
        assert!(!sh.is_running(Mode::Spam));
        assert!(sh.recording().is_none());
        let s = sh.settings.lock().unwrap();
        assert_eq!(s.spam.key.vk, 0x51);
        assert!(s.pin);
        assert_eq!(s.jitter_pct, 12.0);
        assert_eq!(s.spam.limit, SpamLimit::Count(42));
        assert!(sh.dirty.load(SeqCst));
    }

    #[test]
    fn hotkey_relaunches_after_limit_reached() {
        for mode in [HkMode::Toggle, HkMode::Hold] {
            let (sh, log) = setup();
            {
                let mut s = sh.settings.lock().unwrap();
                s.spam.hk_mode = mode;
                s.spam.value = 20.0;
                s.spam.unit = config::Unit::Ms;
                s.spam.limit = SpamLimit::Count(2);
            }
            let wait_end = || {
                for _ in 0..100 {
                    if !sh.runner.running() {
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                panic!("la limite n'arrête pas la tâche");
            };
            sh.handle(ev(0x75, true));
            assert!(sh.is_running(Mode::Spam), "{mode:?}");
            wait_end();
            let downs = |l: &Log| l.0.lock().unwrap().iter().filter(|e| e.0).count();
            assert_eq!(downs(&log), 2);
            sh.handle(ev(0x75, false)); // relâche : en maintien, stop_if sans effet car déjà finie
            assert!(!sh.is_running(Mode::Spam));
            sh.handle(ev(0x75, true)); // nouvel appui : relance (pas un « arrêt » de bascule)
            assert!(sh.is_running(Mode::Spam) || downs(&log) > 2, "{mode:?} : pas relancé");
            wait_end();
            assert_eq!(downs(&log), 4, "{mode:?}");
        }
    }
}
