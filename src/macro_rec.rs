//! Enregistrement de macro : événements clavier horodatés -> `Vec<Step>`.
//! Conversion pure (testable) ; la capture elle-même est dans `control::Shared::handle`.
use crate::config::{Step, Unit, MAX_STEPS};
use crate::keys::KeySpec;
use std::time::Instant;

/// Appui maintenu au moins aussi longtemps = `Step::Hold` (ms)
const HOLD_MS: f64 = 300.0;
/// Silence minimal entre deux actions pour produire un `Step::Wait` (ms)
const WAIT_MIN_MS: f64 = 30.0;
/// Tolérance absolue sur l'écart d'une rafale (ms) : la lecture se fait à 10 ms près
const GAP_SLACK_MS: f64 = 15.0;

/// Évènement capturé : (instant, touche, enfoncée ?)
pub type MacroEv = (Instant, KeySpec, bool);

/// Capture en cours (distincte de `control::Recording`, qui sert aux raccourcis)
#[derive(Clone, Debug)]
pub struct MacroRec {
    pub events: Vec<MacroEv>,
    pub started: Instant,
}

/// Une touche enfoncée de `start` à `end` (ms depuis le premier évènement)
struct Act {
    key: KeySpec,
    start: f64,
    end: f64,
}

fn round10(ms: f64) -> f64 {
    (ms / 10.0).round() * 10.0
}

fn timed(ms: f64) -> (f64, Unit) {
    if ms >= 1000.0 {
        (ms / 1000.0, Unit::S)
    } else {
        (ms, Unit::Ms)
    }
}

/// Appariement appui/relâchement. Une touche sans relâchement est relâchée à `end`.
fn pair(events: &[MacroEv], end: Instant) -> Vec<Act> {
    let Some(base) = events.first().map(|e| e.0) else { return vec![] };
    let ms = |t: Instant| t.saturating_duration_since(base).as_secs_f64() * 1000.0;
    let mut open: Vec<(KeySpec, f64)> = vec![];
    let mut acts = vec![];
    for (t, key, down) in events {
        let pos = open.iter().position(|(k, _)| k.vk == key.vk);
        match (down, pos) {
            (true, None) => open.push((key.clone(), ms(*t))),
            (false, Some(i)) => {
                let (k, s) = open.remove(i);
                acts.push(Act { key: k, start: s, end: ms(*t).max(s) });
            }
            _ => {} // répétition d'appui ou relâchement orphelin
        }
    }
    let e = ms(end);
    acts.extend(open.into_iter().map(|(k, s)| Act { key: k, start: s, end: e.max(s) }));
    acts.sort_by(|a, b| a.start.total_cmp(&b.start));
    acts
}

/// Convertit une capture en étapes. Retourne `(étapes, tronqué ?)` (≤ `MAX_STEPS`).
///
/// Limite : le modèle `Step` est séquentiel. Les chevauchements (Maj+A…) sont traités dans
/// l'ordre des appuis : chaque touche garde sa durée réelle mais ne démarre pas avant la fin
/// de la précédente (jamais de durée négative). Le délai avant le premier appui et le silence
/// final sont ignorés.
pub fn to_steps(events: &[MacroEv], end: Instant) -> (Vec<Step>, bool) {
    // 1) mise en séquence : (touche, début, fin)
    let mut cursor = 0.0f64;
    let mut seq: Vec<Act> = vec![];
    for a in pair(events, end) {
        let dur = a.end - a.start;
        let start = a.start.max(cursor);
        cursor = start + dur;
        seq.push(Act { key: a.key, start, end: cursor });
    }

    // 2) étapes
    let mut out: Vec<Step> = vec![];
    let mut prev_end: Option<f64> = None;
    let mut i = 0;
    while i < seq.len() {
        let a = &seq[i];
        if let Some(pe) = prev_end {
            let gap = a.start - pe;
            if gap >= WAIT_MIN_MS {
                let (value, unit) = timed(round10(gap));
                out.push(Step::Wait { value, unit });
            }
        }
        if a.end - a.start >= HOLD_MS {
            let (value, unit) = timed(round10(a.end - a.start));
            out.push(Step::Hold { key: a.key.clone(), value, unit });
            prev_end = Some(a.end);
            i += 1;
            continue;
        }
        // rafale : appuis courts consécutifs de la même touche, écarts réguliers
        let mut j = i + 1;
        let mut gaps: Vec<f64> = vec![];
        while j < seq.len() {
            let b = &seq[j];
            if b.key.vk != a.key.vk || b.end - b.start >= HOLD_MS {
                break;
            }
            let mut g = gaps.clone();
            g.push(b.start - seq[j - 1].end);
            let mean = g.iter().sum::<f64>() / g.len() as f64;
            if g.iter().any(|x| (x - mean).abs() > (mean * 0.5).max(GAP_SLACK_MS)) {
                break;
            }
            gaps = g;
            j += 1;
        }
        let mean = if gaps.is_empty() { 100.0 } else { (gaps.iter().sum::<f64>() / gaps.len() as f64).round() };
        out.push(Step::Press { key: a.key.clone(), count: (j - i) as u32, gap: mean });
        prev_end = Some(seq[j - 1].end);
        i = j;
    }
    let truncated = out.len() > MAX_STEPS;
    out.truncate(MAX_STEPS);
    (out, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Fabrique des évènements `(vk, appui ms, relâchement ms)` ; retourne aussi l'instant de fin
    fn evs(list: &[(u16, u64, u64)], end_ms: u64) -> (Vec<MacroEv>, Instant) {
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        let mut v: Vec<(u64, u16, bool)> = vec![];
        for &(vk, d, u) in list {
            v.push((d, vk, true));
            v.push((u, vk, false));
        }
        v.sort_by_key(|e| e.0);
        (v.into_iter().map(|(t, vk, d)| (at(t), KeySpec::vk(vk), d)).collect(), at(end_ms))
    }

    #[test]
    fn empty_gives_nothing() {
        let (s, tr) = to_steps(&[], Instant::now());
        assert!(s.is_empty() && !tr);
    }

    #[test]
    fn initial_delay_ignored_and_long_hold() {
        let (e, end) = evs(&[(0x5A, 2000, 3500)], 5000);
        let (s, _) = to_steps(&e, end);
        assert_eq!(s, vec![Step::Hold { key: KeySpec::vk(0x5A), value: 1.5, unit: Unit::S }]);
    }

    #[test]
    fn hold_in_ms_below_one_second() {
        let (e, end) = evs(&[(0x5A, 0, 450)], 500);
        let (s, _) = to_steps(&e, end);
        assert_eq!(s, vec![Step::Hold { key: KeySpec::vk(0x5A), value: 450.0, unit: Unit::Ms }]);
    }

    #[test]
    fn burst_becomes_press_with_mean_gap() {
        // 4 appuis de 50 ms, 100 ms entre relâchement et appui suivant
        let (e, end) = evs(&[(0x45, 0, 50), (0x45, 150, 200), (0x45, 300, 350), (0x45, 450, 500)], 600);
        let (s, _) = to_steps(&e, end);
        assert_eq!(s, vec![Step::Press { key: KeySpec::vk(0x45), count: 4, gap: 100.0 }]);
    }

    #[test]
    fn irregular_burst_is_split_and_waits_inserted() {
        // écarts 100, 100 puis 1000 : la rafale est coupée, un Wait de 1 s s'intercale
        let (e, end) = evs(&[(0x45, 0, 50), (0x45, 150, 200), (0x45, 300, 350), (0x45, 1350, 1400)], 1500);
        let (s, _) = to_steps(&e, end);
        assert_eq!(
            s,
            vec![
                Step::Press { key: KeySpec::vk(0x45), count: 3, gap: 100.0 },
                Step::Wait { value: 1.0, unit: Unit::S },
                Step::Press { key: KeySpec::vk(0x45), count: 1, gap: 100.0 },
            ]
        );
    }

    #[test]
    fn short_silence_ignored_and_wait_rounded() {
        // 20 ms de silence : ignoré ; 234 ms : Wait 230
        let (e, end) = evs(&[(0x41, 0, 50), (0x42, 70, 120), (0x43, 354, 400)], 500);
        let (s, _) = to_steps(&e, end);
        assert_eq!(
            s,
            vec![
                Step::Press { key: KeySpec::vk(0x41), count: 1, gap: 100.0 },
                Step::Press { key: KeySpec::vk(0x42), count: 1, gap: 100.0 },
                Step::Wait { value: 230.0, unit: Unit::Ms },
                Step::Press { key: KeySpec::vk(0x43), count: 1, gap: 100.0 },
            ]
        );
    }

    #[test]
    fn overlap_is_sequential_without_negative_durations() {
        // Maj (0xA0) maintenue 500 ms, A tapée pendant ce temps
        let (e, end) = evs(&[(0xA0, 0, 500), (0x41, 100, 150)], 600);
        let (s, _) = to_steps(&e, end);
        assert_eq!(s, vec![Step::Hold { key: KeySpec::vk(0xA0), value: 500.0, unit: Unit::Ms }, Step::Press { key: KeySpec::vk(0x41), count: 1, gap: 100.0 }]);
        assert!(s.iter().all(|st| st.duration() >= 0.0));
    }

    #[test]
    fn unreleased_key_closed_at_end() {
        let t0 = Instant::now();
        let e = vec![(t0, KeySpec::vk(0x41), true)];
        let (s, _) = to_steps(&e, t0 + Duration::from_millis(800));
        assert_eq!(s, vec![Step::Hold { key: KeySpec::vk(0x41), value: 800.0, unit: Unit::Ms }]);
    }

    #[test]
    fn truncated_to_max_steps() {
        // touches alternées : une étape chacune
        let list: Vec<(u16, u64, u64)> = (0..(MAX_STEPS as u64 + 20)).map(|i| (0x41 + (i % 2) as u16, i * 100, i * 100 + 50)).collect();
        let (e, end) = evs(&list, 100_000);
        let (s, tr) = to_steps(&e, end);
        assert_eq!(s.len(), MAX_STEPS);
        assert!(tr);
    }
}
