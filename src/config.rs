use crate::i18n::Lang;
use crate::keys::KeySpec;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Unit {
    #[serde(rename = "ms")]
    #[default]
    Ms,
    #[serde(rename = "s")]
    S,
    #[serde(rename = "min")]
    Min,
}

impl Unit {
    pub const ALL: [Unit; 3] = [Unit::Ms, Unit::S, Unit::Min];
    pub fn factor(self) -> f64 {
        match self {
            Unit::Ms => 0.001,
            Unit::S => 1.0,
            Unit::Min => 60.0,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Unit::Ms => "ms",
            Unit::S => "s",
            Unit::Min => "min",
        }
    }
}

pub fn secs(value: f64, unit: Unit) -> f64 {
    value * unit.factor()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HkMode {
    #[default]
    Toggle,
    Hold,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Spam,
    Pattern,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Step {
    Hold { key: KeySpec, value: f64, unit: Unit },
    Press { key: KeySpec, count: u32, gap: f64 },
    Wait { value: f64, unit: Unit },
}

impl Step {
    pub fn new_hold() -> Step {
        Step::Hold { key: KeySpec::letter('z'), value: 1.0, unit: Unit::S }
    }
    pub fn new_press() -> Step {
        Step::Press { key: KeySpec::letter('e'), count: 5, gap: 100.0 }
    }
    pub fn new_wait() -> Step {
        Step::Wait { value: 1.0, unit: Unit::S }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Step::Hold { .. } => "hold",
            Step::Press { .. } => "press",
            Step::Wait { .. } => "wait",
        }
    }
    pub fn key_mut(&mut self) -> Option<&mut KeySpec> {
        match self {
            Step::Hold { key, .. } | Step::Press { key, .. } => Some(key),
            Step::Wait { .. } => None,
        }
    }
    pub fn duration(&self) -> f64 {
        match self {
            Step::Hold { value, unit, .. } | Step::Wait { value, unit } => secs(*value, *unit),
            Step::Press { count, gap, .. } => (count.saturating_sub(1)) as f64 * gap / 1000.0,
        }
    }
}

/// Limite d'exécution du spam (arrêt automatique)
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum SpamLimit {
    #[default]
    None,
    /// Nombre de frappes
    Count(u64),
    /// Durée en secondes
    Duration(f64),
}

pub const MAX_LIMIT_COUNT: u64 = 10_000_000;
pub const MIN_LIMIT_S: f64 = 0.1;
pub const MAX_LIMIT_S: f64 = 86_400.0;
pub const MAX_JITTER_PCT: f64 = 50.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpamCfg {
    pub key: KeySpec,
    pub value: f64,
    pub unit: Unit,
    pub hotkey: Vec<KeySpec>,
    pub hk_mode: HkMode,
    pub limit: SpamLimit,
}

impl Default for SpamCfg {
    fn default() -> Self {
        SpamCfg { key: KeySpec::letter('e'), value: 100.0, unit: Unit::Ms, hotkey: vec![KeySpec::vk(0x75)], hk_mode: HkMode::Toggle, limit: SpamLimit::None }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PatternCfg {
    pub steps: Vec<Step>,
    /// 0 = infini
    pub loops: u32,
    pub hotkey: Vec<KeySpec>,
    pub hk_mode: HkMode,
}

impl Default for PatternCfg {
    fn default() -> Self {
        PatternCfg { steps: vec![], loops: 0, hotkey: vec![KeySpec::vk(0x76)], hk_mode: HkMode::Toggle }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub lang: Lang,
    pub pin: bool,
    pub mode: Mode,
    pub spam: SpamCfg,
    pub pattern: PatternCfg,
    /// Durée de maintien de chaque frappe (ms)
    pub press_ms: f64,
    /// Raccourci d'arrêt d'urgence (arrête tout, quel que soit le mode)
    pub stop_hotkey: Vec<KeySpec>,
    /// Aléa sur les intervalles (± %, 0–50)
    pub jitter_pct: f64,
    /// Bip au démarrage / à l'arrêt
    pub beep: bool,
    /// N'envoyer les touches que dans la fenêtre où la tâche a été lancée
    pub bind_window: bool,
    /// Configurations nommées (spam + pattern ; les raccourcis restent globaux)
    pub profiles: Vec<Profile>,
    /// Index du profil actif dans `profiles`
    pub active_profile: Option<usize>,
}

/// Configuration nommée : seuls la touche/cadence du spam et le pattern sont rechargés,
/// les raccourcis (`hotkey`, `hk_mode`) restent ceux de `Settings`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub spam: SpamCfg,
    pub pattern: PatternCfg,
}

pub const MAX_PROFILES: usize = 20;
pub const MAX_PROFILE_NAME: usize = 32;
/// Étapes max par profil : 20 profils × 100 + le pattern courant (500) tiennent dans les 1 Mo du fichier
pub const MAX_PROFILE_STEPS: usize = 100;

/// Durée d'appui par défaut (ms)
pub const DEFAULT_PRESS_MS: f64 = 20.0;
/// Durée d'appui maximale (ms)
pub const MAX_PRESS_MS: f64 = 500.0;

impl Default for Settings {
    fn default() -> Self {
        Settings {
            lang: Lang::default(),
            pin: false,
            mode: Mode::default(),
            spam: SpamCfg::default(),
            pattern: PatternCfg::default(),
            press_ms: DEFAULT_PRESS_MS,
            stop_hotkey: vec![KeySpec::vk(0x77)],
            jitter_pct: 0.0,
            beep: false,
            bind_window: false,
            profiles: vec![],
            active_profile: None,
        }
    }
}

pub fn example_steps() -> Vec<Step> {
    vec![
        Step::Hold { key: KeySpec::letter('z'), value: 3.0, unit: Unit::S },
        Step::Hold { key: KeySpec::letter('x'), value: 2.0, unit: Unit::S },
        Step::Hold { key: KeySpec::letter('z'), value: 10.0, unit: Unit::S },
        Step::Press { key: KeySpec::letter('e'), count: 10, gap: 100.0 },
    ]
}

pub fn cycle_seconds(steps: &[Step]) -> f64 {
    steps.iter().map(|s| s.duration()).sum()
}

fn config_path() -> PathBuf {
    let base = std::env::var_os("APPDATA").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        home.join(".config")
    });
    base.join("AutoKey").join("config3.json")
}

const MAX_FILE: u64 = 1024 * 1024;
pub const MAX_STEPS: usize = 500;
/// Intervalle minimum du spam (10 ms)
pub const MIN_INTERVAL_S: f64 = 0.010;

/// Lecture bornée : fichier ≤ 1 Mo, sinon ignoré
fn read_small(p: &std::path::Path) -> Option<String> {
    let meta = std::fs::metadata(p).ok()?;
    if !meta.is_file() || meta.len() > MAX_FILE {
        return None;
    }
    std::fs::read_to_string(p).ok()
}

fn clamp_f(v: f64, lo: f64, hi: f64, def: f64) -> f64 {
    if v.is_finite() {
        v.clamp(lo, hi)
    } else {
        def
    }
}

/// Ramène toutes les valeurs dans des bornes sûres (fichier modifié à la main, corrompu…)
pub fn sanitize_pattern(p: &mut PatternCfg) {
    p.steps.truncate(MAX_STEPS);
    p.hotkey.truncate(6);
    p.loops = p.loops.min(999_999);
    for st in &mut p.steps {
        match st {
            Step::Hold { value, .. } | Step::Wait { value, .. } => *value = clamp_f(*value, 0.0, 999_999.0, 1.0),
            Step::Press { count, gap, .. } => {
                *count = (*count).clamp(1, 100_000);
                *gap = clamp_f(*gap, 0.0, 600_000.0, 100.0);
            }
        }
    }
}

pub fn sanitize_spam(sp: &mut SpamCfg) {
    sp.value = clamp_f(sp.value, 0.0, 999_999.0, 100.0);
    if secs(sp.value, sp.unit) < MIN_INTERVAL_S {
        sp.value = MIN_INTERVAL_S / sp.unit.factor();
    }
    sp.limit = match sp.limit {
        SpamLimit::None => SpamLimit::None,
        SpamLimit::Count(n) => SpamLimit::Count(n.clamp(1, MAX_LIMIT_COUNT)),
        SpamLimit::Duration(d) => SpamLimit::Duration(clamp_f(d, MIN_LIMIT_S, MAX_LIMIT_S, 60.0)),
    };
    sp.hotkey.truncate(6);
}

/// Nom de profil : espaces rognés, ≤ 32 caractères (pas octets) ; vide → `fallback`
pub fn clean_profile_name(name: &str, fallback: &str) -> String {
    let n: String = name.trim().chars().take(MAX_PROFILE_NAME).collect();
    let n = n.trim_end().to_string();
    if n.is_empty() {
        fallback.to_string()
    } else {
        n
    }
}

fn sanitize_profiles(s: &mut Settings) {
    s.profiles.truncate(MAX_PROFILES);
    for (i, p) in s.profiles.iter_mut().enumerate() {
        p.name = clean_profile_name(&p.name, &format!("Profil {}", i + 1));
        sanitize_spam(&mut p.spam);
        sanitize_pattern(&mut p.pattern);
        p.pattern.steps.truncate(MAX_PROFILE_STEPS);
    }
    s.active_profile = s.active_profile.filter(|&i| i < s.profiles.len());
}

pub fn sanitize(s: &mut Settings) {
    sanitize_spam(&mut s.spam);
    s.jitter_pct = clamp_f(s.jitter_pct, 0.0, MAX_JITTER_PCT, 0.0);
    s.stop_hotkey.truncate(6);
    s.press_ms = clamp_f(s.press_ms, 0.0, MAX_PRESS_MS, DEFAULT_PRESS_MS);
    sanitize_pattern(&mut s.pattern);
    sanitize_profiles(s);
    // arrêt d'urgence (F8 par défaut) en conflit avec un réglage existant : désactivé plutôt que d'interférer
    let vks = |v: &[KeySpec]| v.iter().map(|k| k.vk).collect::<std::collections::BTreeSet<u16>>();
    let stop = vks(&s.stop_hotkey);
    let action_key = std::iter::once(&s.spam.key)
        .chain(s.pattern.steps.iter().filter_map(|st| match st {
            Step::Hold { key, .. } | Step::Press { key, .. } => Some(key),
            Step::Wait { .. } => None,
        }))
        .any(|k| stop.contains(&k.vk));
    if action_key || stop == vks(&s.spam.hotkey) || stop == vks(&s.pattern.hotkey) {
        s.stop_hotkey.clear();
    }
}

pub fn load() -> Settings {
    load_from(&config_path())
}

/// Fichier présent mais illisible : copié en `config3.bad.json` pour ne pas être écrasé sans trace
fn load_from(p: &Path) -> Settings {
    let parsed = read_small(p).and_then(|t| serde_json::from_str::<Settings>(&t).ok());
    let mut s = match parsed {
        Some(s) => s,
        None => {
            if std::fs::metadata(p).map(|m| m.is_file() && m.len() > 0).unwrap_or(false) {
                let _ = std::fs::copy(p, p.with_file_name("config3.bad.json"));
            }
            Settings::default()
        }
    };
    sanitize(&mut s);
    s
}

pub fn read_pattern_file(p: &std::path::Path) -> Option<PatternFile> {
    let mut f: PatternFile = serde_json::from_str(&read_small(p)?).ok()?;
    sanitize_pattern(&mut f.pattern);
    Some(f)
}

pub fn save(s: &Settings) {
    let _ = save_to(&config_path(), s);
}

/// Écriture atomique : fichier temporaire voisin puis renommage
fn save_to(p: &Path, s: &Settings) -> std::io::Result<()> {
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let txt = serde_json::to_string_pretty(s).map_err(std::io::Error::other)?;
    let tmp = p.with_file_name("config3.json.tmp");
    std::fs::write(&tmp, txt)?;
    std::fs::rename(&tmp, p)
}

#[derive(Serialize, Deserialize)]
pub struct PatternFile {
    pub autokey: u32,
    pub pattern: PatternCfg,
}

#[cfg(test)]
mod tests {
    #[test]
    fn stop_hotkey_disabled_when_conflicting() {
        let mut s = Settings::default();
        s.spam.hotkey = vec![KeySpec::vk(0x77)]; // F8 = arrêt par défaut
        sanitize(&mut s);
        assert!(s.stop_hotkey.is_empty());
        let mut s = Settings::default();
        s.spam.key = KeySpec::vk(0x77);
        sanitize(&mut s);
        assert!(s.stop_hotkey.is_empty());
        let mut s = Settings::default();
        sanitize(&mut s);
        assert_eq!(s.stop_hotkey.iter().map(|k| k.vk).collect::<Vec<_>>(), vec![0x77]);
    }

    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static N: AtomicU32 = AtomicU32::new(0);

    /// Dossier temporaire unique, supprimé à la destruction
    struct Tmp(PathBuf);
    impl Tmp {
        fn new() -> Tmp {
            let d = std::env::temp_dir().join(format!("autokey_test_{}_{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            Tmp(d)
        }
        fn file(&self) -> PathBuf {
            self.0.join("config3.json")
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn press(count: u32, gap: f64) -> Step {
        Step::Press { key: KeySpec::letter('a'), count, gap }
    }

    #[test]
    fn sanitize_spam_value_bounds() {
        for (v, want) in [(f64::NAN, 100.0), (f64::INFINITY, 100.0), (-5.0, 10.0), (1e12, 999_999.0)] {
            let mut s = Settings::default();
            s.spam.value = v;
            sanitize(&mut s);
            assert_eq!(s.spam.value, want, "entrée {v}");
        }
    }

    #[test]
    fn sanitize_min_interval() {
        let mut s = Settings::default();
        s.spam.value = 0.0;
        s.spam.unit = Unit::Ms;
        sanitize(&mut s);
        assert!((secs(s.spam.value, s.spam.unit) - MIN_INTERVAL_S).abs() < 1e-12);

        s.spam.value = 0.001;
        s.spam.unit = Unit::S;
        sanitize(&mut s);
        assert!((secs(s.spam.value, s.spam.unit) - MIN_INTERVAL_S).abs() < 1e-12);
        assert_eq!(s.spam.unit, Unit::S);

        s.spam.value = 20.0;
        s.spam.unit = Unit::Ms;
        sanitize(&mut s);
        assert_eq!(s.spam.value, 20.0);
    }

    #[test]
    fn sanitize_press_ms() {
        let mut s = Settings::default();
        assert_eq!(s.press_ms, 20.0);
        for (v, want) in [(-5.0, 0.0), (9999.0, 500.0), (f64::NAN, 20.0), (35.0, 35.0)] {
            s.press_ms = v;
            sanitize(&mut s);
            assert_eq!(s.press_ms, want);
        }
    }

    #[test]
    fn sanitize_truncates() {
        let mut s = Settings::default();
        s.pattern.steps = vec![Step::new_wait(); 600];
        s.pattern.hotkey = vec![KeySpec::vk(0x70); 9];
        s.spam.hotkey = vec![KeySpec::vk(0x70); 9];
        s.stop_hotkey = vec![KeySpec::vk(0x71); 9];
        s.pattern.loops = u32::MAX;
        sanitize(&mut s);
        assert_eq!(s.pattern.steps.len(), MAX_STEPS);
        assert_eq!(s.pattern.hotkey.len(), 6);
        assert_eq!(s.spam.hotkey.len(), 6);
        assert_eq!(s.stop_hotkey.len(), 6);
        assert_eq!(s.pattern.loops, 999_999);
    }

    #[test]
    fn sanitize_step_values() {
        let mut s = Settings::default();
        s.pattern.steps = vec![
            Step::Hold { key: KeySpec::letter('a'), value: f64::NAN, unit: Unit::S },
            Step::Wait { value: -1.0, unit: Unit::Ms },
            Step::Wait { value: 1e12, unit: Unit::Ms },
            press(0, f64::INFINITY),
            press(u32::MAX, -3.0),
        ];
        sanitize(&mut s);
        assert_eq!(s.pattern.steps[0], Step::Hold { key: KeySpec::letter('a'), value: 1.0, unit: Unit::S });
        assert_eq!(s.pattern.steps[1], Step::Wait { value: 0.0, unit: Unit::Ms });
        assert_eq!(s.pattern.steps[2], Step::Wait { value: 999_999.0, unit: Unit::Ms });
        assert_eq!(s.pattern.steps[3], press(1, 100.0));
        assert_eq!(s.pattern.steps[4], press(100_000, 0.0));
    }

    #[test]
    fn partial_and_legacy_json() {
        let t = Tmp::new();
        std::fs::write(t.file(), r#"{"lang":"en"}"#).unwrap();
        let s = load_from(&t.file());
        let d = Settings { lang: Lang::En, ..Default::default() };
        assert_eq!(s, d);

        std::fs::write(t.file(), r#"{"pin":true,"champ_inconnu":42,"spam":{"value":250.0}}"#).unwrap();
        let s = load_from(&t.file());
        assert!(s.pin);
        assert_eq!(s.spam.value, 250.0);
        assert_eq!(s.spam.key, SpamCfg::default().key);
        assert_eq!(s.pattern, PatternCfg::default());
        assert!(!t.0.join("config3.bad.json").exists());
    }

    #[test]
    fn new_fields_defaults_and_compat() {
        let t = Tmp::new();
        std::fs::write(t.file(), r#"{"spam":{"value":250.0}}"#).unwrap();
        let s = load_from(&t.file());
        assert_eq!(s.spam.limit, SpamLimit::None);
        assert_eq!(s.jitter_pct, 0.0);
        assert!(!s.beep);
        assert!(!s.bind_window);
        std::fs::write(t.file(), r#"{"jitter_pct":12.5,"beep":true,"spam":{"limit":{"Count":42}}}"#).unwrap();
        let s = load_from(&t.file());
        assert_eq!((s.jitter_pct, s.beep, s.spam.limit), (12.5, true, SpamLimit::Count(42)));
        std::fs::write(t.file(), r#"{"bind_window":true}"#).unwrap();
        assert!(load_from(&t.file()).bind_window);
    }

    #[test]
    fn sanitize_limit_and_jitter() {
        let mut s = Settings::default();
        for (v, want) in [(-5.0, 0.0), (20.0, 20.0), (99.0, 50.0), (f64::NAN, 0.0)] {
            s.jitter_pct = v;
            sanitize(&mut s);
            assert_eq!(s.jitter_pct, want);
        }
        for (v, want) in [(0, 1), (5, 5), (u64::MAX, MAX_LIMIT_COUNT)] {
            s.spam.limit = SpamLimit::Count(v);
            sanitize(&mut s);
            assert_eq!(s.spam.limit, SpamLimit::Count(want));
        }
        for (v, want) in [(0.0, 0.1), (30.0, 30.0), (1e9, 86_400.0), (f64::NAN, 60.0)] {
            s.spam.limit = SpamLimit::Duration(v);
            sanitize(&mut s);
            assert_eq!(s.spam.limit, SpamLimit::Duration(want));
        }
    }

    #[test]
    fn missing_file_gives_defaults() {
        let t = Tmp::new();
        assert_eq!(load_from(&t.file()), Settings::default());
        assert!(!t.0.join("config3.bad.json").exists());
    }

    #[test]
    fn save_load_roundtrip() {
        let t = Tmp::new();
        let p = t.0.join("sous").join("config3.json");
        let mut s = Settings { lang: Lang::Pt, pin: true, mode: Mode::Pattern, ..Default::default() };
        s.spam.value = 2.5;
        s.spam.unit = Unit::S;
        s.pattern.steps = crate::config::example_steps();
        s.pattern.loops = 3;
        save_to(&p, &s).unwrap();
        assert!(!p.with_file_name("config3.json.tmp").exists());
        assert_eq!(load_from(&p), s);
        // écrasement d'une sauvegarde existante
        s.pin = false;
        save_to(&p, &s).unwrap();
        assert_eq!(load_from(&p), s);
    }

    #[test]
    fn corrupt_file_is_kept() {
        let t = Tmp::new();
        std::fs::write(t.file(), "{ pas du json").unwrap();
        assert_eq!(load_from(&t.file()), Settings::default());
        let bad = t.0.join("config3.bad.json");
        assert_eq!(std::fs::read_to_string(&bad).unwrap(), "{ pas du json");

        // un nouveau fichier corrompu écrase l'ancien .bad
        std::fs::write(t.file(), "[1,2").unwrap();
        load_from(&t.file());
        assert_eq!(std::fs::read_to_string(&bad).unwrap(), "[1,2");
    }

    #[test]
    fn sanitize_profiles_bounds() {
        let mut s = Settings { profiles: (0..25).map(|i| Profile { name: format!("  p{i}  "), ..Default::default() }).collect(), ..Default::default() };
        s.profiles[1].name = "   ".into();
        s.profiles[2].name = "é".repeat(50);
        s.profiles[3].pattern.steps = vec![Step::new_wait(); 300];
        s.profiles[3].spam.value = f64::NAN;
        s.active_profile = Some(30);
        sanitize(&mut s);
        assert_eq!(s.profiles.len(), MAX_PROFILES);
        assert_eq!(s.profiles[0].name, "p0");
        assert_eq!(s.profiles[1].name, "Profil 2");
        assert_eq!(s.profiles[2].name.chars().count(), MAX_PROFILE_NAME);
        assert_eq!(s.profiles[3].pattern.steps.len(), MAX_PROFILE_STEPS);
        assert_eq!(s.profiles[3].spam.value, 100.0);
        assert_eq!(s.active_profile, None);
        s.active_profile = Some(19);
        sanitize(&mut s);
        assert_eq!(s.active_profile, Some(19));
    }

    #[test]
    fn old_config_without_profiles() {
        let mut s: Settings = serde_json::from_str(r#"{"lang":"en","press_ms":30}"#).unwrap();
        sanitize(&mut s);
        assert!(s.profiles.is_empty());
        assert_eq!(s.active_profile, None);
    }

    #[test]
    fn full_profiles_fit_in_file_limit() {
        let mut s = Settings::default();
        let step = Step::Hold { key: KeySpec::vk(0x5A), value: 999_999.0, unit: Unit::S };
        s.pattern.steps = vec![step.clone(); 500];
        s.profiles = (0..MAX_PROFILES)
            .map(|i| Profile {
                name: format!("{i:>32}"),
                pattern: PatternCfg { steps: vec![step.clone(); MAX_PROFILE_STEPS], ..Default::default() },
                ..Default::default()
            })
            .collect();
        sanitize(&mut s);
        let len = serde_json::to_string_pretty(&s).unwrap().len() as u64;
        assert!(len < MAX_FILE * 3 / 4, "{len} octets");
    }
}
