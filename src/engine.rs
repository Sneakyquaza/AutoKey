//! Moteur d'exécution : un thread par tâche, arrêt interruptible, aucune touche laissée enfoncée.
use crate::keys::KeySpec;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicU8, Ordering::*};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub trait Keyboard: Send + Sync {
    fn down(&self, k: &KeySpec);
    fn up(&self, k: &KeySpec);
}

#[derive(Clone, Debug)]
pub enum Job {
    Spam { key: KeySpec, interval: f64 },
    Pattern { steps: Vec<CStep>, loops: u32 },
}

#[derive(Clone, Debug)]
pub enum CStep {
    Hold(KeySpec, f64),
    Press(KeySpec, u32, f64),
    Wait(f64),
}

#[derive(Default)]
pub struct Status {
    pub running: AtomicBool,
    pub mode: AtomicU8, // 0 spam, 1 pattern
    pub count: AtomicU64,
    pub loop_i: AtomicU32,
    pub loops: AtomicU32,
    pub step: AtomicI32,
    pub steps_total: AtomicU32,
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
    pub status: Arc<Status>,
}

impl Runner {
    pub fn new(kb: Arc<dyn Keyboard>) -> Self {
        Runner { kb, cur: Mutex::new(None), status: Arc::new(Status::default()) }
    }

    pub fn running(&self) -> bool {
        self.status.running.load(SeqCst)
    }

    /// 0 = spam, 1 = pattern
    pub fn mode(&self) -> u8 {
        self.status.mode.load(SeqCst)
    }

    pub fn stop(&self) {
        let cur = self.cur.lock().unwrap().take();
        if let Some((stop, h)) = cur {
            stop.set();
            let _ = h.join();
        }
    }

    pub fn start(&self, job: Job) {
        self.stop();
        let st = &self.status;
        st.count.store(0, SeqCst);
        st.loop_i.store(0, SeqCst);
        st.step.store(-1, SeqCst);
        st.mode.store(matches!(job, Job::Pattern { .. }) as u8, SeqCst);
        st.running.store(true, SeqCst);
        let stop = Arc::new(Stop::new());
        let (kb, status, stop2) = (self.kb.clone(), self.status.clone(), stop.clone());
        let h = std::thread::spawn(move || {
            run_job(&*kb, &status, &stop2, job);
            status.step.store(-1, SeqCst);
            status.running.store(false, SeqCst);
        });
        *self.cur.lock().unwrap() = Some((stop, h));
    }
}

fn tap(kb: &dyn Keyboard, k: &KeySpec, st: &Status) {
    kb.down(k);
    kb.up(k);
    st.count.fetch_add(1, SeqCst);
}

fn run_job(kb: &dyn Keyboard, st: &Status, stop: &Stop, job: Job) {
    match job {
        Job::Spam { key, interval } => {
            let iv = Duration::from_secs_f64(interval.max(0.001));
            let mut next = Instant::now();
            loop {
                tap(kb, &key, st);
                next += iv;
                let now = Instant::now();
                if next < now {
                    next = now; // pas de rattrapage en rafale
                }
                if !stop.wait_until(next) {
                    break;
                }
            }
        }
        Job::Pattern { steps, loops } => {
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
                            kb.down(k);
                            let ok = stop.wait(*sec);
                            kb.up(k); // toujours relâchée
                            ok
                        }
                        CStep::Press(k, count, gap) => {
                            let mut ok = true;
                            for c in 0..*count {
                                tap(kb, k, st);
                                if c + 1 < *count && !stop.wait(*gap) {
                                    ok = false;
                                    break;
                                }
                            }
                            ok
                        }
                        CStep::Wait(sec) => stop.wait(*sec),
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
        r.start(Job::Spam { key: KeySpec::letter('e'), interval: 0.05 });
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
        let steps = vec![
            CStep::Hold(KeySpec::letter('z'), 0.02),
            CStep::Wait(0.01),
            CStep::Press(KeySpec::letter('e'), 3, 0.005),
        ];
        r.start(Job::Pattern { steps, loops: 2 });
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
        r.start(Job::Pattern { steps: vec![CStep::Hold(KeySpec::letter('z'), 5.0)], loops: 0 });
        sleep(50);
        let t = Instant::now();
        r.stop();
        assert!(t.elapsed() < Duration::from_millis(150));
        assert_eq!(*log.0.lock().unwrap(), vec![(true, 0x5A), (false, 0x5A)]);
    }
}
