//! Moteur d'exécution : un thread par tâche, arrêt interruptible, aucune touche laissée enfoncée.
use crate::config::SpamLimit;
use crate::keys::KeySpec;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicU8, Ordering::*};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub trait Keyboard: Send + Sync {
    fn down(&self, k: &KeySpec);
    fn up(&self, k: &KeySpec);
}

/// Fenêtre cible : la tâche n'envoie rien tant que `ok()` est faux (pause).
pub trait Focus: Send + Sync {
    /// La cible est au premier plan (et verrouillée)
    fn ok(&self) -> bool;
    /// La cible n'existe plus : la tâche doit s'arrêter
    fn gone(&self) -> bool {
        false
    }
    /// Faux tant que la cible n'est pas encore choisie (armement)
    fn armed(&self) -> bool {
        true
    }
}

/// Période de vérification du focus pendant les attentes et les maintiens
const FOCUS_POLL: Duration = Duration::from_millis(20);

#[derive(Clone, Debug)]
pub enum Job {
    /// `jitter` : fraction 0..0.5 (0 = aucun aléa) ; `limit` : arrêt automatique
    Spam {
        key: KeySpec,
        interval: f64,
        press: f64,
        jitter: f64,
        limit: SpamLimit,
    },
    Pattern {
        steps: Vec<CStep>,
        loops: u32,
        jitter: f64,
    },
}

#[derive(Clone, Debug)]
pub enum CStep {
    Hold(KeySpec, f64),
    Press(KeySpec, u32, f64, f64),
    Wait(f64),
}

#[derive(Default)]
pub struct Status {
    pub running: AtomicBool,
    pub mode: AtomicU8, // 0 spam, 1 pattern
    pub count: AtomicU64,
    pub loop_i: AtomicU32,
    pub loops: AtomicU32,
    /// En pause : fenêtre cible inactive
    pub paused: AtomicBool,
    /// Armement : cible pas encore choisie
    pub waiting: AtomicBool,
    pub step: AtomicI32,
    pub steps_total: AtomicU32,
    /// Instant de démarrage de la tâche en cours
    pub started: Mutex<Option<Instant>>,
}

/// PRNG xorshift64 (aléa non cryptographique, suffisant pour le jitter)
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
        let addr = &t as *const u64 as u64;
        // mélange (splitmix64) ; jamais 0 pour xorshift
        let mut z = t ^ addr.rotate_left(32) ^ 0x9E37_79B9_7F4A_7C15;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Rng((z ^ (z >> 31)) | 1)
    }
    /// Uniforme dans [0, 1)
    fn next_f64(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }
    /// `v` multiplié par un facteur uniforme dans [1-p, 1+p] (p = 0 : inchangé)
    fn jitter(&mut self, v: f64, p: f64) -> f64 {
        if p > 0.0 {
            v * (1.0 + p * (2.0 * self.next_f64() - 1.0))
        } else {
            v
        }
    }
}

struct Stop {
    flag: Mutex<bool>,
    cv: Condvar,
}

impl Stop {
    fn new() -> Self {
        Stop { flag: Mutex::new(false), cv: Condvar::new() }
    }
    fn set(&self) {
        *self.flag.lock().unwrap() = true;
        self.cv.notify_all();
    }
    fn is_set(&self) -> bool {
        *self.flag.lock().unwrap()
    }
    /// Attend jusqu'à `deadline`. Renvoie false si un arrêt a été demandé.
    fn wait_until(&self, deadline: Instant) -> bool {
        let mut g = self.flag.lock().unwrap();
        loop {
            if *g {
                return false;
            }
            let now = Instant::now();
            if now >= deadline {
                return true;
            }
            g = self.cv.wait_timeout(g, deadline - now).unwrap().0;
        }
    }
    fn wait(&self, secs: f64) -> bool {
        self.wait_until(Instant::now() + Duration::from_secs_f64(secs.max(0.0)))
    }
}

pub struct Runner {
    kb: Arc<dyn Keyboard>,
    cur: Mutex<Option<(Arc<Stop>, JoinHandle<()>)>>,
    /// Sérialise start/stop (UI et raccourcis concurrents : sinon un thread orphelin resterait sans arrêt possible)
    ctl: Mutex<()>,
    pub status: Arc<Status>,
    /// Bip au démarrage / à l'arrêt effectifs d'une tâche
    pub beep: AtomicBool,
}

impl Runner {
    pub fn new(kb: Arc<dyn Keyboard>) -> Self {
        Runner { kb, cur: Mutex::new(None), ctl: Mutex::new(()), status: Arc::new(Status::default()), beep: AtomicBool::new(false) }
    }

    pub fn running(&self) -> bool {
        self.status.running.load(SeqCst)
    }

    /// 0 = spam, 1 = pattern
    pub fn mode(&self) -> u8 {
        self.status.mode.load(SeqCst)
    }

    pub fn stop(&self) {
        let _c = self.ctl.lock().unwrap_or_else(|e| e.into_inner());
        self.stop_locked();
    }

    fn stop_locked(&self) {
        let cur = self.cur.lock().unwrap().take();
        if let Some((stop, h)) = cur {
            stop.set();
            let _ = h.join();
        }
    }

    #[cfg(test)]
    pub fn start(&self, job: Job) {
        self.start_bound(job, None);
    }

    /// Comme `start`, avec une fenêtre cible optionnelle (pause hors focus, arrêt si fermée)
    pub fn start_bound(&self, job: Job, focus: Option<Arc<dyn Focus>>) {
        let _c = self.ctl.lock().unwrap_or_else(|e| e.into_inner());
        self.stop_locked();
        let st = &self.status;
        st.count.store(0, SeqCst);
        st.loop_i.store(0, SeqCst);
        st.step.store(-1, SeqCst);
        st.mode.store(matches!(job, Job::Pattern { .. }) as u8, SeqCst);
        *st.started.lock().unwrap() = Some(Instant::now());
        st.paused.store(false, SeqCst);
        st.waiting.store(false, SeqCst);
        st.running.store(true, SeqCst);
        let beep = self.beep.load(SeqCst);
        let stop = Arc::new(Stop::new());
        let (kb, status, stop2) = (self.kb.clone(), self.status.clone(), stop.clone());
        let h = std::thread::spawn(move || {
            crate::platform::hires(true);
            if beep {
                crate::platform::beep(true);
            }
            run_job(&*kb, &status, &stop2, focus, job);
            status.paused.store(false, SeqCst);
            status.waiting.store(false, SeqCst);
            crate::platform::hires(false);
            status.step.store(-1, SeqCst);
            status.running.store(false, SeqCst);
            if beep {
                crate::platform::beep(false);
            }
        });
        *self.cur.lock().unwrap() = Some((stop, h));
    }
}

/// Contexte d'exécution : clavier, état, arrêt et (option) fenêtre cible.
struct Gate<'a> {
    kb: &'a dyn Keyboard,
    st: &'a Status,
    stop: &'a Stop,
    focus: Option<Arc<dyn Focus>>,
    /// Fin réelle de la limite de durée du spam (compte aussi le temps en pause)
    limit: Option<Instant>,
}

impl Gate<'_> {
    fn blocked(&self) -> bool {
        self.focus.as_ref().is_some_and(|f| !f.ok())
    }

    /// Pause tant que la cible n'est pas au premier plan. Renvoie true à la reprise du focus,
    /// false si arrêt demandé, cible fermée (la tâche est alors arrêtée) ou limite de durée atteinte.
    fn pause(&self) -> bool {
        let Some(f) = &self.focus else { return true };
        self.st.paused.store(true, SeqCst);
        let ok = loop {
            if f.ok() {
                break true;
            }
            if f.gone() {
                self.stop.set();
                break false;
            }
            self.st.waiting.store(!f.armed(), SeqCst);
            if self.limit.is_some_and(|d| Instant::now() >= d) || !self.stop.wait(FOCUS_POLL.as_secs_f64()) {
                break false;
            }
        };
        self.st.paused.store(false, SeqCst);
        self.st.waiting.store(false, SeqCst);
        ok
    }

    /// Attend `secs` (interruptible), `held` éventuellement maintenue. Hors focus : la touche est
    /// relâchée et le temps ne s'écoule pas ; à la reprise elle est re-pressée pour la durée restante.
    /// Renvoie false si arrêt demandé. Aucune touche n'est laissée enfoncée.
    fn run(&self, held: Option<&KeySpec>, secs: f64) -> bool {
        let mut rem = Duration::from_secs_f64(secs.max(0.0));
        if let Some(k) = held {
            if self.blocked() && !self.pause() {
                return false;
            }
            self.kb.down(k);
        }
        let mut down = held.is_some();
        let ok = loop {
            if rem.is_zero() {
                break true;
            }
            if self.focus.is_none() {
                break self.stop.wait(rem.as_secs_f64());
            }
            if self.blocked() {
                if let (true, Some(k)) = (down, held) {
                    self.kb.up(k);
                    down = false;
                }
                if !self.pause() {
                    break false;
                }
                if let Some(k) = held {
                    self.kb.down(k);
                    down = true;
                }
            }
            let t = Instant::now();
            if !self.stop.wait(rem.min(FOCUS_POLL).as_secs_f64()) {
                break false;
            }
            rem = rem.saturating_sub(t.elapsed());
        };
        if let (true, Some(k)) = (down, held) {
            self.kb.up(k);
        }
        ok
    }

    /// Frappe : appui maintenu `press` s, toujours relâché. Renvoie false si arrêt demandé.
    fn tap(&self, k: &KeySpec, press: f64) -> bool {
        let ok = self.run(Some(k), press);
        if ok {
            self.st.count.fetch_add(1, SeqCst);
        }
        ok
    }
}

fn run_job(kb: &dyn Keyboard, st: &Status, stop: &Stop, focus: Option<Arc<dyn Focus>>, job: Job) {
    let mut rng = Rng::new();
    let mut g = Gate { kb, st, stop, focus, limit: None };
    match job {
        Job::Spam { key, interval, press, jitter, limit } => {
            let base = interval.max(crate::config::MIN_INTERVAL_S);
            let mut next = Instant::now();
            let deadline = match limit {
                SpamLimit::Duration(d) => Some(next + Duration::from_secs_f64(d.max(0.0))),
                _ => None,
            };
            g.limit = deadline;
            let mut taps = 0u64;
            loop {
                if deadline.is_some_and(|d| Instant::now() >= d) {
                    break;
                }
                let iv = rng.jitter(base, jitter).max(crate::config::MIN_INTERVAL_S);
                let hold = press.max(0.0).min(iv / 2.0);
                if !g.tap(&key, hold) {
                    break;
                }
                taps += 1;
                if matches!(limit, SpamLimit::Count(n) if taps >= n) {
                    break;
                }
                next += Duration::from_secs_f64(iv);
                let now = Instant::now();
                if next < now {
                    next = now; // pas de rattrapage en rafale (ni après une pause)
                }
                // attente plafonnée à la fin de la durée limite
                let until = deadline.map_or(next, |d| next.min(d));
                if !g.run(None, until.saturating_duration_since(Instant::now()).as_secs_f64()) {
                    break;
                }
            }
        }
        Job::Pattern { steps, loops, jitter } => {
            st.loops.store(loops, SeqCst);
            st.steps_total.store(steps.len() as u32, SeqCst);
            let mut i = 0u32;
            'outer: while loops == 0 || i < loops {
                i += 1;
                st.loop_i.store(i, SeqCst);
                for (n, s) in steps.iter().enumerate() {
                    st.step.store(n as i32, SeqCst);
                    let ok = match s {
                        CStep::Hold(k, sec) => {
                            g.run(Some(k), *sec) // toujours relâchée ; pause = durée restante conservée
                        }
                        CStep::Press(k, count, gap, press) => {
                            let mut ok = true;
                            for c in 0..*count {
                                if !g.tap(k, *press) || (c + 1 < *count && !g.run(None, rng.jitter(*gap, jitter))) {
                                    ok = false;
                                    break;
                                }
                            }
                            ok
                        }
                        CStep::Wait(sec) => g.run(None, rng.jitter(*sec, jitter)), // le temps en pause ne compte pas
                    };
                    if !ok || stop.is_set() {
                        break 'outer;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[derive(Default)]
    pub struct Log(pub Mutex<Vec<(bool, u16)>>);
    impl Keyboard for Log {
        fn down(&self, k: &KeySpec) {
            self.0.lock().unwrap().push((true, k.vk));
        }
        fn up(&self, k: &KeySpec) {
            self.0.lock().unwrap().push((false, k.vk));
        }
    }

    fn sleep(ms: u64) {
        std::thread::sleep(Duration::from_millis(ms))
    }

    #[test]
    fn spam_rate_and_stop() {
        let log = Arc::new(Log::default());
        let r = Runner::new(log.clone());
        r.start(Job::Spam { key: KeySpec::letter('e'), interval: 0.05, press: 0.0, jitter: 0.0, limit: SpamLimit::None });
        sleep(525);
        r.stop();
        let n = log.0.lock().unwrap().iter().filter(|e| e.0).count();
        assert!((9..=12).contains(&n), "presses = {n}");
        assert!(!r.running());
    }

    #[test]
    fn pattern_counts() {
        let log = Arc::new(Log::default());
        let r = Runner::new(log.clone());
        let steps = vec![CStep::Hold(KeySpec::letter('z'), 0.02), CStep::Wait(0.01), CStep::Press(KeySpec::letter('e'), 3, 0.005, 0.0)];
        r.start(Job::Pattern { steps, loops: 2, jitter: 0.0 });
        sleep(800);
        assert!(!r.running());
        let l = log.0.lock().unwrap();
        assert_eq!(l.iter().filter(|e| e.0).count(), 8);
        assert_eq!(l.iter().filter(|e| !e.0).count(), 8);
    }

    #[test]
    fn stop_during_hold_releases() {
        let log = Arc::new(Log::default());
        let r = Runner::new(log.clone());
        r.start(Job::Pattern { steps: vec![CStep::Hold(KeySpec::letter('z'), 5.0)], loops: 0, jitter: 0.0 });
        sleep(50);
        let t = Instant::now();
        r.stop();
        assert!(t.elapsed() < Duration::from_millis(150));
        assert_eq!(*log.0.lock().unwrap(), vec![(true, 0x5A), (false, 0x5A)]);
    }

    #[test]
    fn stop_during_long_press_releases() {
        let log = Arc::new(Log::default());
        let r = Runner::new(log.clone());
        r.start(Job::Spam { key: KeySpec::letter('e'), interval: 20.0, press: 5.0, jitter: 0.0, limit: SpamLimit::None });
        sleep(50);
        let t = Instant::now();
        r.stop();
        assert!(t.elapsed() < Duration::from_millis(150));
        assert_eq!(*log.0.lock().unwrap(), vec![(true, 0x45), (false, 0x45)]);

        let r = Runner::new(log.clone());
        log.0.lock().unwrap().clear();
        r.start(Job::Pattern { steps: vec![CStep::Press(KeySpec::letter('e'), 3, 0.01, 5.0)], loops: 0, jitter: 0.0 });
        sleep(50);
        r.stop();
        assert_eq!(*log.0.lock().unwrap(), vec![(true, 0x45), (false, 0x45)]);
    }

    #[test]
    fn press_duration_spacing() {
        /// Journal horodaté
        #[derive(Default)]
        struct Ts(Mutex<Vec<(bool, Instant)>>);
        impl Keyboard for Ts {
            fn down(&self, _: &KeySpec) {
                self.0.lock().unwrap().push((true, Instant::now()));
            }
            fn up(&self, _: &KeySpec) {
                self.0.lock().unwrap().push((false, Instant::now()));
            }
        }
        let ts = Arc::new(Ts::default());
        let r = Runner::new(ts.clone());
        r.start(Job::Pattern { steps: vec![CStep::Press(KeySpec::letter('e'), 3, 0.03, 0.04)], loops: 1, jitter: 0.0 });
        sleep(400);
        assert!(!r.running());
        let l = ts.0.lock().unwrap();
        assert_eq!(l.len(), 6);
        let ms = |a: Instant, b: Instant| b.duration_since(a).as_millis() as i64;
        for t in 0..3 {
            let (d, u) = (l[t * 2], l[t * 2 + 1]);
            assert!(d.0 && !u.0);
            assert!(ms(d.1, u.1) >= 38, "maintien = {}", ms(d.1, u.1));
        }
        for t in 0..2 {
            let gap = ms(l[t * 2 + 1].1, l[t * 2 + 2].1);
            assert!(gap >= 28, "écart = {gap}");
        }
    }

    #[test]
    fn spam_hold_capped_to_half_interval() {
        let log = Arc::new(Log::default());
        let r = Runner::new(log.clone());
        // press 1 s demandé, plafonné à 25 ms : l'arrêt reste rapide et la cadence tient
        r.start(Job::Spam { key: KeySpec::letter('e'), interval: 0.05, press: 1.0, jitter: 0.0, limit: SpamLimit::None });
        sleep(525);
        r.stop();
        let n = log.0.lock().unwrap().iter().filter(|e| e.0).count();
        assert!((8..=12).contains(&n), "presses = {n}");
    }

    #[test]
    fn jitter_bounded() {
        let mut r = Rng::new();
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for _ in 0..5000 {
            let v = r.jitter(1.0, 0.3);
            assert!((0.7..=1.3).contains(&v), "v = {v}");
            lo = lo.min(v);
            hi = hi.max(v);
        }
        assert!(lo < 0.8 && hi > 1.2, "pas d'aléa : {lo}..{hi}");
        assert_eq!(r.jitter(2.0, 0.0), 2.0);
    }

    #[test]
    fn spam_limit_count() {
        let log = Arc::new(Log::default());
        let r = Runner::new(log.clone());
        r.start(Job::Spam { key: KeySpec::letter('e'), interval: 0.02, press: 0.0, jitter: 0.5, limit: SpamLimit::Count(7) });
        sleep(600);
        assert!(!r.running());
        let l = log.0.lock().unwrap();
        assert_eq!(l.iter().filter(|e| e.0).count(), 7);
        assert_eq!(l.iter().filter(|e| !e.0).count(), 7);
    }

    #[test]
    fn spam_limit_duration() {
        let log = Arc::new(Log::default());
        let r = Runner::new(log.clone());
        r.start(Job::Spam { key: KeySpec::letter('e'), interval: 0.05, press: 0.0, jitter: 0.0, limit: SpamLimit::Duration(0.3) });
        sleep(250);
        assert!(r.running());
        sleep(450);
        assert!(!r.running());
        let l = log.0.lock().unwrap();
        let (d, u) = (l.iter().filter(|e| e.0).count(), l.iter().filter(|e| !e.0).count());
        assert!((5..=7).contains(&d), "presses = {d}");
        assert_eq!(d, u);
    }

    /// Focus simulé
    struct Mock(AtomicBool);
    impl Focus for Mock {
        fn ok(&self) -> bool {
            self.0.load(SeqCst)
        }
    }

    fn mock(on: bool) -> Arc<Mock> {
        Arc::new(Mock(AtomicBool::new(on)))
    }

    #[test]
    fn no_keys_while_paused() {
        let log = Arc::new(Log::default());
        let r = Runner::new(log.clone());
        let f = mock(false);
        r.start_bound(Job::Spam { key: KeySpec::letter('e'), interval: 0.02, press: 0.0, jitter: 0.0, limit: SpamLimit::None }, Some(f.clone()));
        sleep(200);
        assert!(log.0.lock().unwrap().is_empty());
        assert!(r.status.paused.load(SeqCst));
        f.0.store(true, SeqCst);
        sleep(150);
        assert!(!r.status.paused.load(SeqCst));
        assert!(!log.0.lock().unwrap().is_empty());
        r.stop();
        let l = log.0.lock().unwrap();
        assert_eq!(l.iter().filter(|e| e.0).count(), l.iter().filter(|e| !e.0).count());
    }

    #[test]
    fn hold_released_on_focus_loss_and_repressed() {
        let log = Arc::new(Log::default());
        let r = Runner::new(log.clone());
        let f = mock(true);
        r.start_bound(Job::Pattern { steps: vec![CStep::Hold(KeySpec::letter('z'), 0.4)], loops: 1, jitter: 0.0 }, Some(f.clone()));
        sleep(100);
        f.0.store(false, SeqCst);
        sleep(150);
        assert_eq!(*log.0.lock().unwrap(), vec![(true, 0x5A), (false, 0x5A)]);
        let t = Instant::now();
        f.0.store(true, SeqCst);
        sleep(60);
        assert_eq!(log.0.lock().unwrap().len(), 3, "re-pressée à la reprise");
        while r.running() {
            sleep(10);
        }
        // durée restante ≈ 0,3 s (le temps en pause ne compte pas)
        assert!(t.elapsed() >= Duration::from_millis(240), "restant = {:?}", t.elapsed());
        assert_eq!(*log.0.lock().unwrap(), vec![(true, 0x5A), (false, 0x5A), (true, 0x5A), (false, 0x5A)]);
    }

    #[test]
    fn stop_while_paused_releases_all() {
        let log = Arc::new(Log::default());
        let r = Runner::new(log.clone());
        let f = mock(true);
        r.start_bound(Job::Pattern { steps: vec![CStep::Hold(KeySpec::letter('z'), 5.0)], loops: 0, jitter: 0.0 }, Some(f.clone()));
        sleep(60);
        f.0.store(false, SeqCst);
        sleep(80);
        let t = Instant::now();
        r.stop();
        assert!(t.elapsed() < Duration::from_millis(150));
        assert_eq!(*log.0.lock().unwrap(), vec![(true, 0x5A), (false, 0x5A)]);
        assert!(!r.status.paused.load(SeqCst));
    }

    #[test]
    fn spam_no_burst_after_pause() {
        let log = Arc::new(Log::default());
        let r = Runner::new(log.clone());
        let f = mock(false);
        r.start_bound(Job::Spam { key: KeySpec::letter('e'), interval: 0.05, press: 0.0, jitter: 0.0, limit: SpamLimit::None }, Some(f.clone()));
        sleep(500);
        f.0.store(true, SeqCst);
        sleep(120);
        r.stop();
        let n = log.0.lock().unwrap().iter().filter(|e| e.0).count();
        assert!((1..=4).contains(&n), "presses = {n}");
    }

    #[test]
    fn spam_duration_counts_real_time_in_pause() {
        let log = Arc::new(Log::default());
        let r = Runner::new(log.clone());
        let f = mock(false);
        r.start_bound(Job::Spam { key: KeySpec::letter('e'), interval: 0.05, press: 0.0, jitter: 0.0, limit: SpamLimit::Duration(0.2) }, Some(f));
        sleep(500);
        assert!(!r.running());
        assert!(log.0.lock().unwrap().is_empty());
    }

    #[test]
    fn gone_window_stops_task() {
        struct Gone;
        impl Focus for Gone {
            fn ok(&self) -> bool {
                false
            }
            fn gone(&self) -> bool {
                true
            }
        }
        let log = Arc::new(Log::default());
        let r = Runner::new(log.clone());
        r.start_bound(Job::Pattern { steps: vec![CStep::Press(KeySpec::letter('e'), 1, 0.0, 0.0)], loops: 0, jitter: 0.0 }, Some(Arc::new(Gone)));
        sleep(150);
        assert!(!r.running());
        assert!(log.0.lock().unwrap().is_empty());
    }
}
