use crate::i18n::{tr, Lang};
use serde::{Deserialize, Serialize};

/// Une touche physique : code virtuel Windows + scancode (0 = calculé à l'envoi).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KeySpec {
    pub vk: u16,
    #[serde(default)]
    pub scan: u16,
    #[serde(default)]
    pub ext: bool,
    /// Nom système (fallback d'affichage pour les touches exotiques)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

impl KeySpec {
    pub fn vk(vk: u16) -> Self {
        KeySpec { vk, ..Default::default() }
    }

    pub fn letter(c: char) -> Self {
        Self::vk(c.to_ascii_uppercase() as u16)
    }

    pub fn label(&self, lang: Lang) -> String {
        let vk = self.vk;
        let named = |k: &str| tr(lang, k).to_string();
        match vk {
            0x41..=0x5A | 0x30..=0x39 => (vk as u8 as char).to_string(),
            0x60..=0x69 => format!("Num {}", vk - 0x60),
            0x70..=0x87 => format!("F{}", vk - 0x6F),
            0x20 => named("Space"),
            0x0D => named("Enter"),
            0x1B => named("Esc"),
            0x09 => "Tab".into(),
            0x08 => named("Back"),
            0x2E => named("Del"),
            0x2D => named("Ins"),
            0x24 => named("Home"),
            0x23 => named("End"),
            0x21 => named("PgUp"),
            0x22 => named("PgDn"),
            0x25 => "←".into(),
            0x26 => "↑".into(),
            0x27 => "→".into(),
            0x28 => "↓".into(),
            0x14 => named("Caps"),
            0x10 => named("Shift"),
            0xA0 => named("LShift"),
            0xA1 => named("RShift"),
            0x11 => named("Ctrl"),
            0xA2 => named("LCtrl"),
            0xA3 => named("RCtrl"),
            0x12 | 0xA4 => "Alt".into(),
            0xA5 => "AltGr".into(),
            0x5B | 0x5C => "Win".into(),
            0x6A => "Num *".into(),
            0x6B => "Num +".into(),
            0x6D => "Num -".into(),
            0x6E => "Num .".into(),
            0x6F => "Num /".into(),
            _ if !self.name.is_empty() => self.name.clone(),
            _ => format!("VK {vk:02X}"),
        }
    }

    /// Rang de tri : modificateurs d'abord (Ctrl, Maj, Alt, Win)
    fn rank(&self) -> u8 {
        match self.vk {
            0x11 | 0xA2 | 0xA3 => 0,
            0x10 | 0xA0 | 0xA1 => 1,
            0x12 | 0xA4 | 0xA5 => 2,
            0x5B | 0x5C => 3,
            _ => 4,
        }
    }
}

pub fn combo_sort(mut v: Vec<KeySpec>) -> Vec<KeySpec> {
    v.sort_by_key(|k| k.rank());
    v
}

pub fn combo_label(keys: &[KeySpec], lang: Lang) -> String {
    if keys.is_empty() {
        return tr(lang, "none").to_string();
    }
    keys.iter().map(|k| k.label(lang)).collect::<Vec<_>>().join(" + ")
}

/// Touche « étendue » (flag requis à l'envoi)
#[cfg_attr(not(windows), allow(dead_code))]
pub fn is_extended_vk(vk: u16) -> bool {
    matches!(vk, 0x21..=0x28 | 0x2D | 0x2E | 0xA3 | 0xA5 | 0x5B | 0x5C | 0x6F | 0x90)
}

/// Évènement clavier physique reçu du hook global
#[derive(Clone, Debug)]
pub struct RawKey {
    pub key: KeySpec,
    pub down: bool,
}
