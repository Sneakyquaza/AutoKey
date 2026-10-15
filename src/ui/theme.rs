//! Palette, polices et helpers de couleur.
use super::*;

// ------------------------------------------------------------------ palette
pub(super) const fn rgb(h: u32) -> Color32 {
    Color32::from_rgb((h >> 16) as u8, (h >> 8) as u8, h as u8)
}
pub const BG: Color32 = rgb(0x0B0D12);
pub(super) const SURFACE: Color32 = rgb(0x12151C);
pub(super) const SURFACE2: Color32 = rgb(0x181C25);
pub(super) const CARD_LINE: Color32 = rgb(0x1F2431);
pub(super) const LINE: Color32 = rgb(0x222838);
pub(super) const LINE2: Color32 = rgb(0x2A3042);
pub(super) const TEXT: Color32 = rgb(0xE8EAF2);
pub(super) const MUTED: Color32 = rgb(0x8A90A6);
pub(super) const DIM: Color32 = rgb(0x4A5066);
pub(super) const VIOLET: Color32 = rgb(0x8B5CF6);
pub(super) const BLUE: Color32 = rgb(0x6D8BF5);
pub(super) const CYAN: Color32 = rgb(0x22D3EE);
pub(super) const AMBER: Color32 = rgb(0xF59E0B);
pub(super) const ROSE: Color32 = rgb(0xF43F5E);
pub(super) const ORANGE: Color32 = rgb(0xFB923C);
pub(super) const GREEN: Color32 = rgb(0x34D399);

pub const WIDTH: f32 = 1120.0;
/// Largeur de la vue réduite
pub const COMPACT_WIDTH: f32 = 420.0;
/// Hauteur de la vue réduite : barre 60 + carte démarrer + pied 40
pub const COMPACT_HEIGHT: f32 = 300.0;

pub fn width_for(compact: bool) -> f32 {
    if compact {
        COMPACT_WIDTH
    } else {
        WIDTH
    }
}

pub fn height_for(m: Mode, compact: bool) -> f32 {
    if compact {
        return COMPACT_HEIGHT;
    }
    match m {
        Mode::Spam => 470.0,
        Mode::Pattern => 530.0,
    }
}

pub(super) fn kind_color(kind: &str) -> Color32 {
    match kind {
        "hold" => VIOLET,
        "press" => CYAN,
        _ => AMBER,
    }
}

// ------------------------------------------------------------------ polices
#[derive(Clone, Copy)]
pub(super) enum W {
    R,
    S,
    B,
}
pub(super) fn font(size: f32, w: W) -> FontId {
    FontId::new(
        size,
        match w {
            W::R => FontFamily::Proportional,
            W::S => FontFamily::Name("semi".into()),
            W::B => FontFamily::Name("bold".into()),
        },
    )
}

pub fn setup_fonts(ctx: &egui::Context) {
    // Polices système (rien d'embarqué → exe plus léger). Chaque graisse a des solutions de repli.
    let mut defs = egui::FontDefinitions::empty();
    #[cfg(windows)]
    let dir = std::env::var("WINDIR").map(|w| format!("{w}\\Fonts\\")).unwrap_or_else(|_| "C:\\Windows\\Fonts\\".into());
    #[cfg(windows)]
    let files: [&[&str]; 3] =
        [&["segoeui.ttf", "arial.ttf", "tahoma.ttf"], &["seguisb.ttf", "segoeuib.ttf", "arialbd.ttf"], &["segoeuib.ttf", "arialbd.ttf", "tahomabd.ttf"]];
    #[cfg(not(windows))]
    let dir = "/usr/share/fonts/opentype/inter/".to_string();
    #[cfg(not(windows))]
    let files: [&[&str]; 3] = [&["Inter-Regular.otf"], &["Inter-SemiBold.otf"], &["Inter-Bold.otf"]];

    let fams = [FontFamily::Proportional, FontFamily::Name("semi".into()), FontFamily::Name("bold".into())];
    let mut loaded: Vec<String> = vec![];
    for (cands, fam) in files.iter().zip(fams.iter()) {
        let mut list: Vec<String> = vec![];
        for file in cands.iter() {
            if !loaded.iter().any(|l| l == file) {
                if let Ok(bytes) = std::fs::read(format!("{dir}{file}")) {
                    defs.font_data.insert(file.to_string(), egui::FontData::from_owned(bytes));
                    loaded.push(file.to_string());
                }
            }
            if loaded.iter().any(|l| l == file) {
                list.push(file.to_string());
                break;
            }
        }
        defs.families.insert(fam.clone(), list);
    }
    // repli croisé : si une graisse manque, on prend n'importe quelle police chargée
    for fam in &fams {
        let l = defs.families.entry(fam.clone()).or_default();
        for f in &loaded {
            if !l.contains(f) {
                l.push(f.clone());
            }
        }
    }
    defs.families.insert(FontFamily::Monospace, loaded.clone());
    ctx.set_fonts(defs);

    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = SURFACE;
    v.extreme_bg_color = SURFACE2;
    v.selection.bg_fill = VIOLET.gamma_multiply(0.6);
    v.selection.stroke = sk(1.0, TEXT);
    v.text_cursor.stroke = sk(2.0, CYAN);
    v.override_text_color = Some(TEXT);
    v.window_stroke = sk(1.0, LINE2);
    v.popup_shadow = epaint::Shadow::NONE;
    ctx.set_visuals(v);
    ctx.options_mut(|o| o.zoom_with_keyboard = false);
    ctx.style_mut(|s| {
        s.interaction.tooltip_delay = 0.35;
        s.spacing.tooltip_width = 320.0;
    });
}

// ------------------------------------------------------------------ primitives de dessin
pub(super) fn lerp_c(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

pub(super) fn lighten(c: Color32, t: f32) -> Color32 {
    lerp_c(c, Color32::WHITE, t)
}

/// Rectangle arrondi rempli d'un dégradé (diagonal ou vertical)
pub(super) fn grad_rect(p: &Painter, r: Rect, rounding: f32, c0: Color32, c1: Color32, diagonal: bool) {
    let mut mesh = epaint::Mesh::default();
    let mut tess = epaint::Tessellator::new(p.ctx().pixels_per_point(), epaint::TessellationOptions::default(), [1, 1], vec![]);
    tess.tessellate_rect(&epaint::RectShape::filled(r, rounding, Color32::WHITE), &mut mesh);
    for v in &mut mesh.vertices {
        let tx = (v.pos.x - r.min.x) / r.width().max(1.0);
        let ty = (v.pos.y - r.min.y) / r.height().max(1.0);
        let t = if diagonal { (tx + ty) * 0.5 } else { ty };
        let c = lerp_c(c0, c1, t);
        let a = v.color.a();
        v.color = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a);
    }
    p.add(Shape::mesh(mesh));
}

pub(super) fn sk(w: f32, c: Color32) -> Stroke {
    Stroke::new(w, c)
}
