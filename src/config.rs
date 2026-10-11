use crate::i18n::Lang;
use crate::keys::KeySpec;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

pub fn load() -> Settings {
    std::fs::read_to_string(config_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save(s: &Settings) {
    let p = config_path();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(txt) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(p, txt);
    }
}

#[derive(Serialize, Deserialize)]
pub struct PatternFile {
    pub autokey: u32,
    pub pattern: PatternCfg,
}
