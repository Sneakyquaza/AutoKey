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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpamCfg {
    pub key: KeySpec,
    pub value: f64,
    pub unit: Unit,
    pub hotkey: Vec<KeySpec>,
    pub hk_mode: HkMode,
}

impl Default for SpamCfg {
    fn default() -> Self {
        SpamCfg { key: KeySpec::letter('e'), value: 100.0, unit: Unit::Ms, hotkey: vec![KeySpec::vk(0x75)], hk_mode: HkMode::Toggle }
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    pub lang: Lang,
    pub pin: bool,
    pub mode: Mode,
    pub spam: SpamCfg,
    pub pattern: PatternCfg,
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
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        home.join(".config")
    });
    base.join("AutoKey").join("config3.json")
}

const MAX_FILE: u64 = 1024 * 1024;
const MAX_STEPS: usize = 500;

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

pub fn sanitize(s: &mut Settings) {
    s.spam.value = clamp_f(s.spam.value, 0.0, 999_999.0, 100.0);
    s.spam.hotkey.truncate(6);
    sanitize_pattern(&mut s.pattern);
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
