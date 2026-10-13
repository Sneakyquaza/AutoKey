//! Interface : tout est dessiné à la main (egui) pour un rendu sur mesure.
use crate::config::{self, HkMode, Mode, PatternFile, Settings, Step, Unit};
use crate::control::{Recording, Shared, Target};
use crate::i18n::{fmt_duration, fmt_num, tr, trf, Lang};
use crate::keys::combo_label;
use egui::{
    self, epaint, pos2, vec2, Align, Align2, Color32, CursorIcon, FontFamily, FontId, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, TextEdit, Ui,
    Vec2, ViewportCommand,
};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ------------------------------------------------------------------ palette
const fn rgb(h: u32) -> Color32 {
    Color32::from_rgb((h >> 16) as u8, (h >> 8) as u8, h as u8)
}
pub const BG: Color32 = rgb(0x0B0D12);
const SURFACE: Color32 = rgb(0x12151C);
const SURFACE2: Color32 = rgb(0x181C25);
const CARD_LINE: Color32 = rgb(0x1F2431);
const LINE: Color32 = rgb(0x222838);
const LINE2: Color32 = rgb(0x2A3042);
const TEXT: Color32 = rgb(0xE8EAF2);
const MUTED: Color32 = rgb(0x8A90A6);
const DIM: Color32 = rgb(0x4A5066);
const VIOLET: Color32 = rgb(0x8B5CF6);
const BLUE: Color32 = rgb(0x6D8BF5);
const CYAN: Color32 = rgb(0x22D3EE);
const AMBER: Color32 = rgb(0xF59E0B);
const ROSE: Color32 = rgb(0xF43F5E);
const ORANGE: Color32 = rgb(0xFB923C);
const GREEN: Color32 = rgb(0x34D399);

pub const WIDTH: f32 = 1120.0;
pub fn height_for(m: Mode) -> f32 {
    match m {
        Mode::Spam => 420.0,
        Mode::Pattern => 530.0,
    }
}

fn kind_color(kind: &str) -> Color32 {
    match kind {
        "hold" => VIOLET,
        "press" => CYAN,
        _ => AMBER,
    }
}

// ------------------------------------------------------------------ polices
#[derive(Clone, Copy)]
enum W {
    R,
    S,
    B,
}
fn font(size: f32, w: W) -> FontId {
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
fn lerp_c(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

fn lighten(c: Color32, t: f32) -> Color32 {
    lerp_c(c, Color32::WHITE, t)
}

/// Rectangle arrondi rempli d'un dégradé (diagonal ou vertical)
fn grad_rect(p: &Painter, r: Rect, rounding: f32, c0: Color32, c1: Color32, diagonal: bool) {
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

fn sk(w: f32, c: Color32) -> Stroke {
    Stroke::new(w, c)
}

fn card(p: &Painter, r: Rect) {
    p.rect(r, 16.0, SURFACE, sk(1.0, CARD_LINE));
}

fn spaced_text(p: &Painter, pos: Pos2, text: &str, size: f32, color: Color32) -> Rect {
    let mut job = egui::text::LayoutJob::default();
    job.append(text, 0.0, egui::TextFormat { font_id: font(size, W::B), color, extra_letter_spacing: 1.2, ..Default::default() });
    let g = p.layout_job(job);
    let r = Rect::from_min_size(pos, g.size());
    p.galley(pos, g, color);
    r
}

fn card_title(p: &Painter, r: Rect, text: &str) {
    spaced_text(p, r.min + vec2(18.0, 16.0), text, 11.0, MUTED);
}

fn text(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, w: W, color: Color32) -> Rect {
    p.text(pos, anchor, s, font(size, w), color)
}

fn text_width(p: &Painter, s: &str, size: f32, w: W) -> f32 {
    p.layout_no_wrap(s.to_string(), font(size, w), TEXT).size().x
}

fn wrapped(p: &Painter, r: Rect, s: &str, size: f32, w: W, color: Color32, center: bool) {
    let mut job = egui::text::LayoutJob::simple(s.to_string(), font(size, w), color, r.width());
    if center {
        job.halign = Align::Center;
    }
    let g = p.layout_job(job);
    let pos = if center { pos2(r.center().x, r.center().y - g.size().y / 2.0) } else { r.min };
    p.galley(pos, g, color);
}

fn dashed_rrect(p: &Painter, r: Rect, rad: f32, stroke: Stroke) {
    let mut pts = vec![];
    let corners = [
        (pos2(r.max.x - rad, r.min.y + rad), -90.0f32),
        (pos2(r.max.x - rad, r.max.y - rad), 0.0),
        (pos2(r.min.x + rad, r.max.y - rad), 90.0),
        (pos2(r.min.x + rad, r.min.y + rad), 180.0),
    ];
    for (c, a0) in corners {
        for i in 0..=8 {
            let a = (a0 + i as f32 * 90.0 / 8.0).to_radians();
            pts.push(c + vec2(a.cos(), a.sin()) * rad);
        }
    }
    pts.push(pts[0]);
    p.extend(Shape::dashed_line(&pts, stroke, 7.0, 5.0));
}

// ------------------------------------------------------------------ icônes vectorielles
#[derive(Clone, Copy, PartialEq)]
enum Ic {
    Close,
    Min,
    Pin,
    Left,
    Right,
    Copy,
    Trash,
    Plus,
    Globe,
    Gear,
    Play,
    Stop,
    Hold,
    Press,
    Wait,
    Open,
    Save,
    Spark,
    Loop,
    Keyboard,
}

fn icon(p: &Painter, ic: Ic, c: Pos2, size: f32, color: Color32) {
    let s = size;
    let o = c - vec2(s / 2.0, s / 2.0);
    let pt = |x: f32, y: f32| o + vec2(x * s, y * s);
    let st = sk((1.7 * s / 18.0).max(1.2), color);
    let line = |a: Pos2, b: Pos2| {
        p.line_segment([a, b], st);
    };
    let poly = |pts: Vec<Pos2>| {
        p.add(Shape::line(pts, st));
    };
    match ic {
        Ic::Close => {
            line(pt(0.3, 0.3), pt(0.7, 0.7));
            line(pt(0.7, 0.3), pt(0.3, 0.7));
        }
        Ic::Min => line(pt(0.28, 0.5), pt(0.72, 0.5)),
        Ic::Plus => {
            line(pt(0.5, 0.26), pt(0.5, 0.74));
            line(pt(0.26, 0.5), pt(0.74, 0.5));
        }
        Ic::Pin => {
            p.circle_stroke(pt(0.5, 0.36), s * 0.15, st);
            line(pt(0.5, 0.52), pt(0.5, 0.84));
            line(pt(0.3, 0.52), pt(0.7, 0.52));
        }
        Ic::Left => poly(vec![pt(0.6, 0.28), pt(0.38, 0.5), pt(0.6, 0.72)]),
        Ic::Right => poly(vec![pt(0.4, 0.28), pt(0.62, 0.5), pt(0.4, 0.72)]),
        Ic::Copy => {
            p.rect_stroke(Rect::from_min_max(pt(0.36, 0.36), pt(0.74, 0.74)), 2.0, st);
            poly(vec![pt(0.26, 0.62), pt(0.26, 0.26), pt(0.62, 0.26)]);
        }
        Ic::Trash => {
            line(pt(0.24, 0.3), pt(0.76, 0.3));
            poly(vec![pt(0.41, 0.3), pt(0.43, 0.2), pt(0.57, 0.2), pt(0.59, 0.3)]);
            poly(vec![pt(0.32, 0.3), pt(0.36, 0.8), pt(0.64, 0.8), pt(0.68, 0.3)]);
        }
        Ic::Globe => {
            p.circle_stroke(c, s * 0.3, st);
            let pts: Vec<Pos2> = (0..=24).map(|i| i as f32 / 24.0 * std::f32::consts::TAU).map(|a| c + vec2(a.cos() * s * 0.13, a.sin() * s * 0.3)).collect();
            poly(pts);
            line(pt(0.2, 0.5), pt(0.8, 0.5));
        }
        Ic::Gear => {
            p.circle_stroke(c, s * 0.17, st);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let d = vec2(a.cos(), a.sin());
                line(c + d * s * 0.27, c + d * s * 0.37);
            }
            p.circle_stroke(c, s * 0.27, st);
        }
        Ic::Play => {
            p.add(Shape::convex_polygon(vec![pt(0.32, 0.22), pt(0.8, 0.5), pt(0.32, 0.78)], color, Stroke::NONE));
        }
        Ic::Stop => {
            p.rect_filled(Rect::from_min_max(pt(0.27, 0.27), pt(0.73, 0.73)), s * 0.08, color);
        }
        Ic::Hold => {
            p.rect_filled(Rect::from_min_max(pt(0.18, 0.38), pt(0.82, 0.62)), s * 0.12, color);
        }
        Ic::Press => {
            for x in [0.26, 0.5, 0.74] {
                p.circle_filled(pt(x, 0.5), s * 0.085, color);
            }
        }
        Ic::Wait => {
            p.circle_stroke(pt(0.5, 0.54), s * 0.28, st);
            line(pt(0.5, 0.54), pt(0.5, 0.38));
            line(pt(0.5, 0.54), pt(0.62, 0.6));
            line(pt(0.42, 0.16), pt(0.58, 0.16));
        }
        Ic::Open => poly(vec![pt(0.2, 0.74), pt(0.2, 0.28), pt(0.42, 0.28), pt(0.48, 0.36), pt(0.8, 0.36), pt(0.8, 0.74), pt(0.2, 0.74)]),
        Ic::Save => {
            line(pt(0.5, 0.2), pt(0.5, 0.6));
            poly(vec![pt(0.34, 0.45), pt(0.5, 0.61), pt(0.66, 0.45)]);
            poly(vec![pt(0.22, 0.62), pt(0.22, 0.8), pt(0.78, 0.8), pt(0.78, 0.62)]);
        }
        Ic::Spark => {
            p.add(Shape::convex_polygon(vec![pt(0.5, 0.12), pt(0.6, 0.5), pt(0.5, 0.88), pt(0.4, 0.5)], color, Stroke::NONE));
            p.add(Shape::convex_polygon(vec![pt(0.12, 0.5), pt(0.5, 0.4), pt(0.88, 0.5), pt(0.5, 0.6)], color, Stroke::NONE));
        }
        Ic::Loop => {
            let pts: Vec<Pos2> = (0..=20).map(|i| (-60.0 + i as f32 * 300.0 / 20.0).to_radians()).map(|a| c + vec2(a.cos(), a.sin()) * s * 0.28).collect();
            let end = *pts.last().unwrap();
            poly(pts);
            poly(vec![end + vec2(-s * 0.16, -s * 0.02), end, end + vec2(-s * 0.02, -s * 0.16)]);
        }
        Ic::Keyboard => {
            p.rect_stroke(Rect::from_min_max(pt(0.12, 0.3), pt(0.88, 0.72)), s * 0.07, st);
            line(pt(0.32, 0.6), pt(0.68, 0.6));
            for x in [0.28, 0.42, 0.58, 0.72] {
                p.circle_filled(pt(x, 0.44), s * 0.025, color);
            }
        }
    }
}

fn logo(p: &Painter, r: Rect) {
    // icône « Rafale » simplifiée : fond sombre, touche dégradée, ▶▶
    let n = r.width();
    let pt = |x: f32, y: f32| r.min + vec2(x * n, y * n);
    grad_rect(p, r, n * 0.22, rgb(0x1A1D29), rgb(0x0B0D12), true);
    p.rect_filled(Rect::from_min_max(pt(0.15, 0.24), pt(0.85, 0.86)), n * 0.13, rgb(0x1B6E8C));
    grad_rect(p, Rect::from_min_max(pt(0.15, 0.17), pt(0.85, 0.78)), n * 0.13, rgb(0x9D74FF), CYAN, true);
    for off in [-0.1f32, 0.06] {
        let (cx, cy, s) = (0.5 + off, 0.475, 0.19);
        p.add(Shape::convex_polygon(
            vec![pt(cx - s * 0.45, cy - s * 0.72), pt(cx + s * 0.55, cy), pt(cx - s * 0.45, cy + s * 0.72)],
            Color32::WHITE,
            Stroke::NONE,
        ));
    }
}

// ------------------------------------------------------------------ widgets
fn hand(r: Response) -> Response {
    r.on_hover_cursor(CursorIcon::PointingHand)
}

fn icon_btn(ui: &mut Ui, r: Rect, id: &str, ic: Ic, tip: &str, checked: bool, danger: bool) -> Response {
    let resp = hand(ui.interact(r, Id::new(id), Sense::click()));
    let p = ui.painter();
    let h = ui.ctx().animate_bool_with_time(Id::new(id).with("h"), resp.hovered(), 0.12);
    if checked {
        p.rect_filled(r, 8.0, rgb(0x2A2345));
    }
    if h > 0.0 {
        let c = if danger { ROSE } else { SURFACE2 };
        p.rect_filled(r, 8.0, c.gamma_multiply(h));
    }
    let col = if danger && h > 0.5 {
        Color32::WHITE
    } else if checked {
        lighten(VIOLET, 0.3)
    } else {
        TEXT
    };
    icon(p, ic, r.center(), 18.0, col);
    if tip.is_empty() {
        resp
    } else {
        resp.on_hover_text(tip)
    }
}

fn mini_btn(ui: &mut Ui, r: Rect, id: Id, ic: Ic, tip: &str, enabled: bool, danger: bool) -> bool {
    let resp = ui.interact(r, id, if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    if enabled && resp.hovered() {
        p.rect_filled(r, 6.0, if danger { rgb(0x3A1620) } else { LINE });
    }
    let col = if !enabled {
        rgb(0x343A4C)
    } else if danger {
        rgb(0xF87185)
    } else if resp.hovered() {
        TEXT
    } else {
        MUTED
    };
    icon(p, ic, r.center(), 16.0, col);
    let clicked = enabled && resp.clicked();
    if enabled {
        hand(resp).on_hover_text(tip);
    }
    clicked
}

/// Bouton secondaire avec icône + texte
fn tool_btn(ui: &mut Ui, pos: Pos2, h: f32, id: Id, ic: Ic, ic_col: Color32, label: &str, checked: bool) -> (Response, f32) {
    let w = text_width(ui.painter(), label, 13.0, W::S) + 16.0 + 8.0 + 24.0;
    let r = Rect::from_min_size(pos, vec2(w, h));
    let resp = hand(ui.interact(r, id, Sense::click()));
    let p = ui.painter();
    let hov = resp.hovered();
    p.rect(r, 10.0, if checked { rgb(0x2A2345) } else { SURFACE2 }, sk(1.0, if hov || checked { VIOLET } else { LINE2 }));
    icon(p, ic, pos2(r.min.x + 12.0 + 8.0, r.center().y), 16.0, ic_col);
    text(p, pos2(r.min.x + 12.0 + 16.0 + 8.0, r.center().y), Align2::LEFT_CENTER, label, 13.0, W::S, TEXT);
    (resp, w)
}

/// Contrôle segmenté avec pastille animée
fn segmented<T: PartialEq + Copy>(ui: &mut Ui, r: Rect, id: Id, items: &[(T, String)], cur: &mut T, size: f32, grad: bool) -> bool {
    let p = ui.painter().clone();
    p.rect(r, 10.0, SURFACE, sk(1.0, LINE));
    let inner = r.shrink(3.0);
    let n = items.len() as f32;
    let bw = inner.width() / n;
    let idx = items.iter().position(|(v, _)| v == cur).unwrap_or(0) as f32;
    let x = ui.ctx().animate_value_with_time(id.with("pill"), idx, 0.14);
    let pill = Rect::from_min_size(pos2(inner.min.x + x * bw, inner.min.y), vec2(bw, inner.height()));
    if grad {
        grad_rect(&p, pill, 8.0, VIOLET, BLUE, true);
    } else {
        p.rect_filled(pill, 7.0, VIOLET);
    }
    let mut changed = false;
    for (i, (v, label)) in items.iter().enumerate() {
        let br = Rect::from_min_size(pos2(inner.min.x + i as f32 * bw, inner.min.y), vec2(bw, inner.height()));
        let resp = hand(ui.interact(br, id.with(i), Sense::click()));
        if resp.clicked() && v != cur {
            *cur = *v;
            changed = true;
        }
        let sel = v == cur;
        let col = if sel {
            Color32::WHITE
        } else if resp.hovered() {
            TEXT
        } else {
            MUTED
        };
        text(&p, br.center(), Align2::CENTER_CENTER, label, size, W::S, col);
    }
    changed
}

#[derive(Clone, Copy)]
enum CapSize {
    Big,
    Mid,
    Step,
}

/// Touche « capturable » : clic → écoute du clavier
fn keycap(ui: &mut Ui, r: Rect, id: Id, label: &str, recording: bool, size: CapSize, lang: Lang, combo: bool) -> Response {
    let resp = hand(ui.interact(r, id, Sense::click()));
    let p = ui.painter();
    let hov = ui.ctx().animate_bool_with_time(id.with("h"), resp.hovered(), 0.12);
    let (rad, depth, fs) = match size {
        CapSize::Big => (16.0, 6.0, 46.0),
        CapSize::Mid => (12.0, 0.0, 16.0),
        CapSize::Step => (11.0, 4.0, 22.0),
    };
    if depth > 0.0 {
        p.rect_filled(r, rad, rgb(0x0E1118));
    }
    let face = Rect::from_min_max(r.min, pos2(r.max.x, r.max.y - depth));
    grad_rect(p, face, rad, rgb(0x1F2433), rgb(0x161A23), false);
    if recording {
        p.rect_stroke(face.expand(3.0), rad + 3.0, sk(3.0, CYAN.gamma_multiply(0.3)));
        p.rect_stroke(face, rad, sk(2.0, CYAN));
        let msg = if combo { tr(lang, "press_combo") } else { tr(lang, "press_key") };
        let shown = if label.is_empty() { msg.to_string() } else { label.to_string() };
        let fsz = if label.is_empty() {
            match size {
                CapSize::Big => 13.0,
                CapSize::Mid => 12.0,
                CapSize::Step => 11.0,
            }
        } else {
            (fs * 0.6f32).max(13.0)
        };
        wrapped(p, face.shrink2(vec2(10.0, 4.0)), &shown, fsz, W::S, CYAN, true);
    } else {
        let border = lerp_c(LINE2, VIOLET, hov);
        p.rect_stroke(face, rad, sk(1.0, border));
        // une seule mesure, puis taille choisie parmi quelques paliers (atlas de police plus petit)
        let maxw = face.width() - 20.0;
        let w0 = text_width(p, label, fs, W::B).max(1.0);
        let ideal = fs * (maxw / w0).min(1.0);
        let fsz = [46.0, 34.0, 26.0, 22.0, 17.0, 15.0, 13.0, 11.0].into_iter().find(|s| *s <= ideal).unwrap_or(11.0);
        text(p, face.center(), Align2::CENTER_CENTER, label, fsz, W::B, TEXT);
    }
    resp
}

fn parse_num(s: &str, lo: f64, hi: f64, int: bool) -> Option<f64> {
    let v: f64 = s.trim().replace(',', ".").parse().ok()?;
    if !v.is_finite() {
        return None;
    }
    let v = v.clamp(lo, hi);
    Some(if int { v.round() } else { v })
}

/// Champ numérique : saisie libre (virgule ou point) + molette
fn num_field(ui: &mut Ui, r: Rect, id: Id, v: &mut f64, lo: f64, hi: f64, int: bool, size: f32, enabled: bool, lang: Lang) -> bool {
    let fmt = |x: f64| if int { format!("{}", x as i64) } else { fmt_num(lang, x) };
    let p = ui.painter().clone();
    let focused = ui.memory(|m| m.has_focus(id));
    let bg = ui.interact(r, id.with("bg"), if enabled { Sense::click() } else { Sense::hover() });
    let hov = enabled && bg.hovered();
    let border = if focused {
        VIOLET
    } else if hov {
        rgb(0x3A4160)
    } else {
        LINE2
    };
    p.rect(r, 10.0, SURFACE2, sk(1.0, border));
    if !enabled {
        text(&p, r.center(), Align2::CENTER_CENTER, &fmt(*v), size, W::B, DIM);
        return false;
    }
    if bg.clicked() {
        ui.memory_mut(|m| m.request_focus(id));
    }
    let mut buf = if focused { ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_else(|| fmt(*v)) } else { fmt(*v) };
    let h = size * 1.3;
    let tr_ = Rect::from_center_size(r.center(), vec2(r.width() - 12.0, h));
    let te = TextEdit::singleline(&mut buf)
        .id(id)
        .frame(false)
        .font(font(size, W::B))
        .horizontal_align(Align::Center)
        .margin(egui::Margin::ZERO)
        .desired_width(tr_.width());
    let resp = ui.put(tr_, te);
    let mut changed = false;
    if resp.changed() {
        if let Some(nv) = parse_num(&buf, lo, hi, int) {
            if nv != *v {
                *v = nv;
                changed = true;
            }
        }
    }
    if resp.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, buf));
    } else {
        ui.data_mut(|d| d.remove::<String>(id));
    }
    if hov || resp.hovered() {
        let dy = ui.input_mut(|i| {
            let d = i.raw_scroll_delta.y;
            if d != 0.0 {
                i.raw_scroll_delta = Vec2::ZERO;
                i.smooth_scroll_delta = Vec2::ZERO;
            }
            d
        });
        if dy != 0.0 {
            let nv = (*v + dy.signum() as f64).clamp(lo, hi);
            if nv != *v {
                *v = if int { nv.round() } else { nv };
                changed = true;
                ui.memory_mut(|m| m.surrender_focus(id));
            }
        }
    }
    changed
}

fn chip(ui: &mut Ui, pos: Pos2, id: Id, label: &str) -> (bool, f32) {
    let w = text_width(ui.painter(), label, 12.0, W::S) + 22.0;
    let r = Rect::from_min_size(pos, vec2(w, 26.0));
    let resp = hand(ui.interact(r, id, Sense::click()));
    let p = ui.painter();
    let hov = resp.hovered();
    p.rect(r, 13.0, if hov { rgb(0x1C2030) } else { Color32::TRANSPARENT }, sk(1.0, if hov { VIOLET } else { LINE2 }));
    text(p, r.center(), Align2::CENTER_CENTER, label, 12.0, W::S, if hov { TEXT } else { rgb(0xA9AEC2) });
    (resp.clicked(), w)
}

fn start_button(ui: &mut Ui, r: Rect, id: Id, running: bool, hint: &str, lang: Lang, with_icon: bool) -> bool {
    let resp = hand(ui.interact(r, id, Sense::click()));
    let p = ui.painter();
    let h = ui.ctx().animate_bool_with_time(id.with("h"), resp.hovered(), 0.15) * 0.12;
    let (c0, c1) = if running { (ROSE, ORANGE) } else { (VIOLET, CYAN) };
    // halo
    p.rect_filled(r.expand(4.0), 22.0, c0.gamma_multiply(0.06 + h * 0.6));
    grad_rect(p, r, 18.0, lighten(c0, h), lighten(c1, h), true);
    let label = if running { tr(lang, "stop") } else { tr(lang, "start") };
    let c = r.center();
    if with_icon {
        icon(p, if running { Ic::Stop } else { Ic::Play }, c - vec2(0.0, 34.0), 30.0, Color32::WHITE);
        text(p, c + vec2(0.0, 4.0), Align2::CENTER_CENTER, label, 21.0, W::B, Color32::WHITE);
        text(p, c + vec2(0.0, 30.0), Align2::CENTER_CENTER, hint, 12.0, W::S, Color32::from_white_alpha(210));
    } else {
        text(p, c - vec2(0.0, 9.0), Align2::CENTER_CENTER, label, 20.0, W::B, Color32::WHITE);
        text(p, c + vec2(0.0, 16.0), Align2::CENTER_CENTER, hint, 12.0, W::S, Color32::from_white_alpha(210));
    }
    resp.clicked()
}

fn rhythm(p: &Painter, r: Rect, iv: f64) {
    let win = if iv <= 1.0 {
        1.0
    } else if iv <= 60.0 {
        60.0
    } else {
        3600.0
    };
    let unit = if win == 1.0 {
        "1 s"
    } else if win == 60.0 {
        "1 min"
    } else {
        "1 h"
    };
    let track = Rect::from_min_size(r.min + vec2(0.0, 4.0), vec2(r.width(), 20.0));
    p.rect_filled(track, 8.0, SURFACE2);
    let n = (win / iv.max(0.001)) as usize;
    if n > 80 {
        let inner = track.shrink(3.0);
        grad_rect(p, inner, 6.0, VIOLET, CYAN, false);
        // dégradé horizontal : on recolore à la main
        let mut mesh = epaint::Mesh::default();
        let mut tess = epaint::Tessellator::new(p.ctx().pixels_per_point(), Default::default(), [1, 1], vec![]);
        tess.tessellate_rect(&epaint::RectShape::filled(inner, 6.0, Color32::WHITE), &mut mesh);
        for v in &mut mesh.vertices {
            let c = lerp_c(VIOLET, CYAN, (v.pos.x - inner.min.x) / inner.width());
            v.color = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), v.color.a());
        }
        p.add(Shape::mesh(mesh));
    } else {
        let span = track.width() - 16.0;
        for i in 0..=n {
            let x = track.min.x + 8.0 + span * (i as f64 * iv / win) as f32;
            if x > track.max.x - 6.0 {
                break;
            }
            let c = lerp_c(VIOLET, CYAN, (x - track.min.x) / track.width());
            p.rect_filled(Rect::from_center_size(pos2(x, track.center().y), vec2(3.0, 12.0)), 1.5, c);
        }
    }
    text(p, pos2(r.min.x + 2.0, track.max.y + 10.0), Align2::LEFT_CENTER, "0", 11.0, W::R, MUTED);
    text(p, pos2(r.max.x - 2.0, track.max.y + 10.0), Align2::RIGHT_CENTER, unit, 11.0, W::R, MUTED);
}

// ------------------------------------------------------------------ application
enum Act {
    Record(Target),
    Toggle(Mode),
    SetMode(Mode),
    SetLang,
    Pin(bool),
    Minimize,
    Close,
    StepsChanged,
    SavePreset,
    OpenPreset,
}

pub struct App {
    sh: Arc<Shared>,
    up: Arc<crate::update::Updater>,
    lang_open: bool,
    settings_open: bool,
    confirm_clear: bool,
    last_active: i32,
    frames: u64,
    dirty_since: Option<Instant>,
    #[cfg(feature = "shots")]
    pub shots: Option<crate::shots::Shots>,
}

impl App {
    pub fn new(sh: Arc<Shared>, up: Arc<crate::update::Updater>) -> Self {
        App {
            sh,
            up,
            lang_open: false,
            settings_open: false,
            confirm_clear: false,
            last_active: -1,
            frames: 0,
            dirty_since: None,
            #[cfg(feature = "shots")]
            shots: None,
        }
    }

    #[cfg(feature = "shots")]
    pub fn shared(&self) -> &Arc<Shared> {
        &self.sh
    }

    #[cfg(feature = "shots")]
    pub fn updater(&self) -> &Arc<crate::update::Updater> {
        &self.up
    }

    #[cfg(feature = "shots")]
    pub fn open_confirm_for_shots(&mut self, on: bool) {
        self.confirm_clear = on;
    }

    #[cfg(feature = "shots")]
    pub fn open_settings_for_shots(&mut self, on: bool) {
        self.settings_open = on;
    }

    fn apply(&mut self, ctx: &egui::Context, a: Act) {
        match a {
            Act::Record(t) => {
                ctx.memory_mut(|m| {
                    if let Some(id) = m.focused() {
                        m.surrender_focus(id)
                    }
                });
                self.sh.begin_record(t);
            }
            Act::Toggle(m) => self.sh.toggle(m),
            Act::SetMode(m) => {
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(WIDTH, height_for(m))));
            }
            Act::SetLang => {}
            Act::Pin(on) => ctx.send_viewport_cmd(ViewportCommand::WindowLevel(if on { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal })),
            Act::Minimize => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
            Act::Close => ctx.send_viewport_cmd(ViewportCommand::Close),
            Act::StepsChanged => {
                if matches!(self.sh.recording().map(|r| r.target), Some(Target::StepKey(_))) {
                    self.sh.cancel_record();
                }
                if self.sh.is_running(Mode::Pattern) {
                    self.sh.runner.stop();
                }
            }
            Act::SavePreset => {
                let pat = self.sh.settings.lock().unwrap().pattern.clone();
                let picked = crate::platform::pick_save();
                crate::platform::trim_memory(); // la boîte de dialogue Windows charge beaucoup de DLL
                if let Some(path) = picked {
                    let file = PatternFile { autokey: 3, pattern: pat };
                    if let Ok(txt) = serde_json::to_string_pretty(&file) {
                        if std::fs::write(path, txt).is_ok() {
                            *self.sh.flash.lock().unwrap() = Some(("saved", Instant::now()));
                        }
                    }
                }
            }
            Act::OpenPreset => {
                let picked = crate::platform::pick_open();
                crate::platform::trim_memory();
                if let Some(path) = picked {
                    let parsed = config::read_pattern_file(&path);
                    match parsed {
                        Some(f) => {
                            self.sh.runner.stop();
                            self.sh.cancel_record();
                            self.sh.settings.lock().unwrap().pattern = f.pattern;
                            self.sh.mark_dirty();
                        }
                        None => *self.sh.flash.lock().unwrap() = Some(("bad_file", Instant::now())),
                    }
                }
            }
        }
    }
}

impl App {
    pub fn wants_quit(&self) -> bool {
        self.up.quit.load(SeqCst)
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        self.frames += 1;
        let rec = self.sh.recording();
        if rec.is_some() {
            // pendant l'écoute, le clavier ne doit rien activer dans la fenêtre
            ctx.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Key { .. } | egui::Event::Text(_))));
        }
        let orig = self.sh.settings.lock().unwrap().clone();
        let mut s = orig.clone();
        let mut acts = vec![];
        egui::CentralPanel::default().frame(egui::Frame::none().fill(BG)).show(ctx, |ui| {
            self.draw(ui, &mut s, &rec, &mut acts);
        });
        if s != orig {
            *self.sh.settings.lock().unwrap() = s;
            self.sh.mark_dirty();
        }
        for a in acts {
            self.apply(ctx, a);
        }

        // sauvegarde différée
        if self.sh.dirty.load(SeqCst) {
            let since = *self.dirty_since.get_or_insert_with(Instant::now);
            if since.elapsed() > Duration::from_millis(400) {
                self.sh.save_now();
                self.dirty_since = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(450));
            }
        }
        // rafraîchissement minimal : 4 i/s pendant l'exécution (compteur, étape en cours), rien au repos
        if self.sh.runner.running() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        if self.sh.flash.lock().unwrap().is_some() {
            ctx.request_repaint_after(Duration::from_millis(3050));
        }
        #[cfg(feature = "shots")]
        crate::shots::tick(self, ctx);
    }

    pub fn on_exit(&mut self) {
        self.sh.runner.stop();
        crate::platform::release_held();
        self.sh.save_now();
    }
}

impl App {
    fn draw(&mut self, ui: &mut Ui, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>) {
        let full = ui.max_rect();
        let lang = s.lang;
        let bar = Rect::from_min_size(full.min, vec2(full.width(), 60.0));
        let foot = Rect::from_min_max(pos2(full.min.x, full.max.y - 40.0), full.max);
        let content = Rect::from_min_max(pos2(full.min.x, bar.max.y), pos2(full.max.x, foot.min.y));

        self.titlebar(ui, bar, s, acts);
        match s.mode {
            Mode::Spam => self.spam_page(ui, content, s, rec, acts),
            Mode::Pattern => self.pattern_page(ui, content, s, rec, acts),
        }
        self.footer(ui, foot, s);
        ui.painter().rect_stroke(full.shrink(0.5), 0.0, sk(1.0, LINE));

        if self.lang_open || self.settings_open || self.confirm_clear {
            let ctx = ui.ctx().clone();
            egui::Area::new(Id::new("overlay")).order(egui::Order::Foreground).fixed_pos(full.min).show(&ctx, |ui| {
                ui.set_min_size(full.size());
                if self.lang_open {
                    self.lang_popup(ui, full, s, acts);
                }
                if self.settings_open {
                    self.settings_popup(ui, full, s, rec, acts);
                }
                if self.confirm_clear {
                    self.confirm_modal(ui, full, s, acts);
                }
            });
        }
        let _ = lang;
    }

    // ---------------------------------------------------------------- barre de titre
    fn titlebar(&mut self, ui: &mut Ui, bar: Rect, s: &mut Settings, acts: &mut Vec<Act>) {
        let lang = s.lang;
        let drag = ui.interact(bar, Id::new("drag"), Sense::click_and_drag());
        if drag.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        let p = ui.painter().clone();
        logo(&p, Rect::from_min_size(bar.min + vec2(20.0, 16.0), vec2(28.0, 28.0)));
        let r = text(&p, pos2(bar.min.x + 60.0, bar.center().y), Align2::LEFT_CENTER, "AutoKey", 15.0, W::B, TEXT);
        text(&p, pos2(r.max.x + 8.0, bar.center().y + 1.0), Align2::LEFT_CENTER, concat!("v", env!("CARGO_PKG_VERSION")), 11.0, W::R, MUTED);

        // onglets
        let tabs = Rect::from_center_size(bar.center(), vec2(236.0, 40.0));
        let mut mode = s.mode;
        if segmented(ui, tabs, Id::new("tabs"), &[(Mode::Spam, tr(lang, "spam").into()), (Mode::Pattern, tr(lang, "pattern").into())], &mut mode, 13.0, true) {
            s.mode = mode;
            acts.push(Act::SetMode(mode));
        }

        // boutons droite
        let mut x = bar.max.x - 12.0 - 34.0;
        let y = bar.center().y - 17.0;
        let br = |x: f32| Rect::from_min_size(pos2(x, y), vec2(34.0, 34.0));
        if icon_btn(ui, br(x), "close", Ic::Close, tr(lang, "close"), false, true).clicked() {
            acts.push(Act::Close);
        }
        x -= 38.0;
        if icon_btn(ui, br(x), "min", Ic::Min, tr(lang, "minimize"), false, false).clicked() {
            acts.push(Act::Minimize);
        }
        x -= 38.0;
        if icon_btn(ui, br(x), "pin", Ic::Pin, tr(lang, "pin"), s.pin, false).clicked() {
            s.pin = !s.pin;
            acts.push(Act::Pin(s.pin));
        }
        // réglages
        x -= 38.0;
        if icon_btn(ui, br(x), "gearbtn", Ic::Gear, tr(lang, "settings"), self.settings_open, false).clicked() {
            self.settings_open = !self.settings_open;
            self.lang_open = false;
        }
        // langue
        let lw = 74.0;
        let lr = Rect::from_min_size(pos2(x - 10.0 - lw, bar.center().y - 16.0), vec2(lw, 32.0));
        let resp = hand(ui.interact(lr, Id::new("langbtn"), Sense::click())).on_hover_text(tr(lang, "language"));
        let p = ui.painter();
        p.rect(lr, 9.0, SURFACE, sk(1.0, if resp.hovered() || self.lang_open { VIOLET } else { LINE }));
        icon(p, Ic::Globe, pos2(lr.min.x + 20.0, lr.center().y), 16.0, TEXT);
        text(p, pos2(lr.min.x + 34.0, lr.center().y), Align2::LEFT_CENTER, lang.code(), 13.0, W::S, TEXT);
        if resp.clicked() {
            self.lang_open = !self.lang_open;
            self.settings_open = false;
        }

        // bouton de mise à jour (visible seulement si une nouvelle version existe)
        if let crate::update::UpState::Available { build, .. } = self.up.state() {
            let label = tr(lang, "update");
            let uw = text_width(ui.painter(), label, 13.0, W::S) + 44.0;
            let ur = Rect::from_min_size(pos2(lr.min.x - 10.0 - uw, bar.center().y - 16.0), vec2(uw, 32.0));
            let resp = hand(ui.interact(ur, Id::new("updbtn"), Sense::click())).on_hover_text(trf(lang, "update_tip", &[("b", build.to_string())]));
            let p = ui.painter();
            let h = if resp.hovered() { 0.12 } else { 0.0 };
            grad_rect(p, ur, 9.0, lighten(VIOLET, h), lighten(CYAN, h), true);
            icon(p, Ic::Save, pos2(ur.min.x + 18.0, ur.center().y), 16.0, Color32::WHITE);
            text(p, pos2(ur.min.x + 32.0, ur.center().y), Align2::LEFT_CENTER, label, 13.0, W::S, Color32::WHITE);
            if resp.clicked() {
                self.up.install();
            }
        }
    }

    fn lang_popup(&mut self, ui: &mut Ui, full: Rect, s: &mut Settings, acts: &mut Vec<Act>) {
        let btn_right = full.max.x - 12.0 - 34.0 - 38.0 - 38.0 - 38.0 - 10.0;
        let r = Rect::from_min_size(pos2(btn_right - 160.0, full.min.y + 52.0), vec2(160.0, 3.0 * 34.0 + 12.0));
        let p = ui.painter().clone();
        p.rect_filled(r.translate(vec2(0.0, 6.0)).expand(2.0), 14.0, Color32::from_black_alpha(90));
        p.rect(r, 12.0, SURFACE, sk(1.0, LINE2));
        let blocker = ui.interact(r, Id::new("langpop"), Sense::click());
        let mut inside = blocker.hovered();
        for (i, l) in Lang::ALL.iter().enumerate() {
            let ir = Rect::from_min_size(pos2(r.min.x + 6.0, r.min.y + 6.0 + i as f32 * 34.0), vec2(r.width() - 12.0, 34.0));
            let resp = hand(ui.interact(ir, Id::new("lang").with(i), Sense::click()));
            inside |= resp.hovered();
            let sel = *l == s.lang;
            if resp.hovered() {
                p.rect_filled(ir, 8.0, VIOLET);
            } else if sel {
                p.rect_filled(ir, 8.0, rgb(0x2A2345));
            }
            text(&p, pos2(ir.min.x + 12.0, ir.center().y), Align2::LEFT_CENTER, l.name(), 13.0, W::S, TEXT);
            text(&p, pos2(ir.max.x - 12.0, ir.center().y), Align2::RIGHT_CENTER, l.code(), 11.0, W::S, if resp.hovered() { TEXT } else { MUTED });
            if resp.clicked() {
                s.lang = *l;
                acts.push(Act::SetLang);
                self.lang_open = false;
            }
        }
        let lr_hover = ui.ctx().read_response(Id::new("langbtn")).map(|r| r.hovered()).unwrap_or(false);
        let clicked_out = ui.input(|i| i.pointer.any_pressed()) && !inside && !lr_hover;
        if clicked_out {
            self.lang_open = false;
        }
    }

    fn settings_popup(&mut self, ui: &mut Ui, full: Rect, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>) {
        let lang = s.lang;
        let gear_right = full.max.x - 12.0 - 34.0 - 38.0 - 38.0;
        let r = Rect::from_min_size(pos2(gear_right - 300.0, full.min.y + 52.0), vec2(300.0, 2.0 * 50.0 + 12.0));
        let p = ui.painter().clone();
        p.rect_filled(r.translate(vec2(0.0, 6.0)).expand(2.0), 14.0, Color32::from_black_alpha(90));
        p.rect(r, 12.0, SURFACE, sk(1.0, LINE2));
        let blocker = ui.interact(r, Id::new("setpop"), Sense::click());
        let mut inside = blocker.hovered();
        let row = |i: f32| r.min.y + 6.0 + i * 50.0 + 25.0;
        // durée d'appui
        text(&p, pos2(r.min.x + 16.0, row(0.0)), Align2::LEFT_CENTER, tr(lang, "press_dur"), 13.0, W::S, TEXT);
        let nr = Rect::from_center_size(pos2(r.max.x - 16.0 - 40.0 - 22.0, row(0.0)), vec2(80.0, 36.0));
        num_field(ui, nr, Id::new("pressms"), &mut s.press_ms, 0.0, config::MAX_PRESS_MS, true, 16.0, true, lang);
        inside |= ui.rect_contains_pointer(nr);
        text(&p, pos2(nr.max.x + 8.0, row(0.0)), Align2::LEFT_CENTER, "ms", 12.0, W::S, MUTED);
        // arrêt d'urgence
        text(&p, pos2(r.min.x + 16.0, row(1.0)), Align2::LEFT_CENTER, tr(lang, "stop_hk"), 13.0, W::S, TEXT);
        let (lbl, recording) = Self::rec_label(s, rec, Target::StopHotkey, combo_label(&s.stop_hotkey, lang));
        let cr = Rect::from_center_size(pos2(r.max.x - 16.0 - 60.0, row(1.0)), vec2(120.0, 36.0));
        let resp = keycap(ui, cr, Id::new("stophk"), &lbl, recording, CapSize::Mid, lang, true).on_hover_text(tr(lang, "click_change"));
        inside |= resp.hovered();
        if resp.clicked() {
            acts.push(Act::Record(Target::StopHotkey));
        }
        let btn_hover = ui.ctx().read_response(Id::new("gearbtn")).map(|r| r.hovered()).unwrap_or(false);
        if ui.input(|i| i.pointer.any_pressed()) && !inside && !btn_hover {
            self.settings_open = false;
        }
    }

    fn confirm_modal(&mut self, ui: &mut Ui, full: Rect, s: &mut Settings, acts: &mut Vec<Act>) {
        let lang = s.lang;
        let p = ui.painter().clone();
        p.rect_filled(full, 0.0, Color32::from_black_alpha(150));
        ui.interact(full, Id::new("modal_block"), Sense::click());
        let r = Rect::from_center_size(full.center(), vec2(380.0, 160.0));
        p.rect(r, 18.0, SURFACE, sk(1.0, LINE2));
        icon(&p, Ic::Trash, pos2(r.center().x, r.min.y + 36.0), 28.0, ROSE);
        text(&p, pos2(r.center().x, r.min.y + 78.0), Align2::CENTER_CENTER, tr(lang, "clear_q"), 15.0, W::S, TEXT);
        let bw = 150.0;
        let y = r.max.y - 26.0 - 36.0 + 18.0;
        let cancel = Rect::from_min_size(pos2(r.center().x - bw - 6.0, y - 18.0), vec2(bw, 36.0));
        let del = Rect::from_min_size(pos2(r.center().x + 6.0, y - 18.0), vec2(bw, 36.0));
        let rc = hand(ui.interact(cancel, Id::new("m_cancel"), Sense::click()));
        let rd = hand(ui.interact(del, Id::new("m_del"), Sense::click()));
        p.rect(cancel, 10.0, SURFACE2, sk(1.0, if rc.hovered() { VIOLET } else { LINE2 }));
        text(&p, cancel.center(), Align2::CENTER_CENTER, tr(lang, "cancel"), 13.0, W::S, TEXT);
        p.rect_filled(del, 10.0, if rd.hovered() { lighten(ROSE, 0.12) } else { ROSE });
        text(&p, del.center(), Align2::CENTER_CENTER, tr(lang, "yes"), 13.0, W::S, Color32::WHITE);
        if rc.clicked() {
            self.confirm_clear = false;
        }
        if rd.clicked() {
            s.pattern.steps.clear();
            acts.push(Act::StepsChanged);
            self.confirm_clear = false;
        }
    }

    // ---------------------------------------------------------------- pied de page
    fn footer(&mut self, ui: &mut Ui, r: Rect, s: &Settings) {
        let lang = s.lang;
        let p = ui.painter();
        p.line_segment([r.left_top(), r.right_top()], sk(1.0, rgb(0x1A1F2A)));
        let st = &self.sh.runner.status;
        let running = self.sh.runner.running();
        let c = pos2(r.min.x + 26.0, r.center().y);
        if running {
            p.circle_filled(c, 7.0, GREEN.gamma_multiply(0.3));
            p.circle_filled(c, 3.5, GREEN);
        } else {
            p.circle_filled(c, 3.5, MUTED);
        }
        let mut msg = tr(lang, "idle").to_string();
        let mut col = TEXT;
        if running && self.sh.runner.mode() == 0 {
            msg = trf(lang, "running_spam", &[("n", st.count.load(SeqCst).to_string())]);
        } else if running {
            let step = st.step.load(SeqCst);
            if step >= 0 {
                let loops = st.loops.load(SeqCst);
                msg = trf(
                    lang,
                    "running_pat",
                    &[
                        ("i", st.loop_i.load(SeqCst).to_string()),
                        ("n", if loops == 0 { "∞".into() } else { loops.to_string() }),
                        ("s", (step + 1).to_string()),
                        ("t", st.steps_total.load(SeqCst).to_string()),
                    ],
                );
            }
        }
        let mut flash = self.sh.flash.lock().unwrap();
        if let Some((k, t)) = *flash {
            if t.elapsed() < Duration::from_secs(3) {
                msg = tr(lang, k).to_string();
                col = if k == "saved" { GREEN } else { AMBER };
            } else {
                *flash = None;
            }
        }
        drop(flash);
        match self.up.state() {
            crate::update::UpState::Downloading(pc) => {
                msg = trf(lang, "up_dl", &[("p", pc.to_string())]);
                col = CYAN;
            }
            crate::update::UpState::Restarting => {
                msg = tr(lang, "up_restart").to_string();
                col = GREEN;
            }
            crate::update::UpState::Failed(k) => {
                msg = tr(lang, k).to_string();
                col = AMBER;
            }
            _ => {}
        }
        text(p, pos2(r.min.x + 40.0, r.center().y), Align2::LEFT_CENTER, &msg, 12.0, W::S, col);
        text(p, pos2(r.max.x - 20.0, r.center().y), Align2::RIGHT_CENTER, tr(lang, "bg_hint"), 12.0, W::R, MUTED);
    }

    // ---------------------------------------------------------------- carte raccourci
    fn hotkey_hint(s: &Settings, m: Mode, rec: &Option<Recording>) -> String {
        let t = if m == Mode::Spam { Target::SpamHotkey } else { Target::PatHotkey };
        let keys = if m == Mode::Spam { &s.spam.hotkey } else { &s.pattern.hotkey };
        match rec {
            Some(r) if r.target == t => "…".into(),
            _ => combo_label(keys, s.lang),
        }
    }

    fn rec_label(s: &Settings, rec: &Option<Recording>, t: Target, current: String) -> (String, bool) {
        match rec {
            Some(r) if r.target == t => (combo_label(&crate::keys::combo_sort(r.keys.clone()), s.lang).replace(tr(s.lang, "none"), ""), true),
            _ => (current, false),
        }
    }

    // ---------------------------------------------------------------- page spam
    fn spam_page(&mut self, ui: &mut Ui, c: Rect, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>) {
        let lang = s.lang;
        let inner = Rect::from_min_max(c.min + vec2(22.0, 10.0), c.max - vec2(22.0, 20.0));
        let gap = 14.0;
        let (kw, hw, sw) = (180.0, 250.0, 160.0);
        let iw = inner.width() - kw - hw - sw - 3.0 * gap;
        let h = inner.height();
        let mut x = inner.min.x;
        let key_r = Rect::from_min_size(pos2(x, inner.min.y), vec2(kw, h));
        x += kw + gap;
        let int_r = Rect::from_min_size(pos2(x, inner.min.y), vec2(iw, h));
        x += iw + gap;
        let hk_r = Rect::from_min_size(pos2(x, inner.min.y), vec2(hw, h));
        x += hw + gap;
        let st_r = Rect::from_min_size(pos2(x, inner.min.y), vec2(sw, h));
        let p = ui.painter().clone();

        // touche
        card(&p, key_r);
        card_title(&p, key_r, tr(lang, "key"));
        let cap = Rect::from_min_max(key_r.min + vec2(18.0, 44.0), key_r.max - vec2(18.0, 44.0));
        let (lbl, recording) = Self::rec_label(s, rec, Target::SpamKey, s.spam.key.label(lang));
        if keycap(ui, cap, Id::new("spamkey"), &lbl, recording, CapSize::Big, lang, false).on_hover_text(tr(lang, "click_change")).clicked() {
            acts.push(Act::Record(Target::SpamKey));
        }
        text(&p, pos2(key_r.center().x, key_r.max.y - 22.0), Align2::CENTER_CENTER, tr(lang, "click_change"), 12.0, W::R, MUTED);

        // intervalle
        card(&p, int_r);
        card_title(&p, int_r, tr(lang, "interval"));
        text(&p, int_r.min + vec2(18.0, 52.0), Align2::LEFT_CENTER, tr(lang, "every"), 12.0, W::R, MUTED);
        let row_y = int_r.min.y + 70.0;
        let seg_w = 150.0;
        let num_r = Rect::from_min_size(pos2(int_r.min.x + 18.0, row_y), vec2(iw - 36.0 - seg_w - 10.0, 64.0));
        let min_v = config::MIN_INTERVAL_S / s.spam.unit.factor();
        num_field(ui, num_r, Id::new("spamval"), &mut s.spam.value, min_v, 999_999.0, false, 32.0, true, lang);
        let seg_r = Rect::from_min_size(pos2(num_r.max.x + 10.0, row_y), vec2(seg_w, 64.0));
        let items: Vec<(Unit, String)> = Unit::ALL.iter().map(|u| (*u, u.label().to_string())).collect();
        segmented(ui, seg_r, Id::new("spamunit"), &items, &mut s.spam.unit, 13.0, false);
        // changement d'unité : on garde l'intervalle minimum
        s.spam.value = s.spam.value.max(config::MIN_INTERVAL_S / s.spam.unit.factor());
        let mut cx = int_r.min.x + 18.0;
        for (i, (v, u)) in [(50.0, Unit::Ms), (100.0, Unit::Ms), (500.0, Unit::Ms), (1.0, Unit::S), (1.0, Unit::Min)].iter().enumerate() {
            let (clicked, w) = chip(ui, pos2(cx, row_y + 76.0), Id::new("chip").with(i), &format!("{} {}", fmt_num(lang, *v), u.label()));
            if clicked {
                s.spam.value = *v;
                s.spam.unit = *u;
            }
            cx += w + 6.0;
        }
        let iv = config::secs(s.spam.value, s.spam.unit).max(config::MIN_INTERVAL_S);
        rhythm(&p, Rect::from_min_size(pos2(int_r.min.x + 18.0, int_r.max.y - 84.0), vec2(iw - 36.0, 46.0)), iv);
        let rate = if iv <= 1.0 {
            trf(lang, "rate_s", &[("n", fmt_num(lang, (10.0 / iv).round() / 10.0))])
        } else if iv <= 60.0 {
            trf(lang, "rate_m", &[("n", fmt_num(lang, (600.0 / iv).round() / 10.0))])
        } else {
            trf(lang, "rate_h", &[("n", fmt_num(lang, (36000.0 / iv).round() / 10.0))])
        };
        text(&p, pos2(int_r.min.x + 18.0, int_r.max.y - 24.0), Align2::LEFT_CENTER, &rate, 12.0, W::S, CYAN);

        // raccourci
        self.hotkey_card(ui, hk_r, s, rec, acts, Mode::Spam);

        // démarrer
        let hint = Self::hotkey_hint(s, Mode::Spam, rec);
        if start_button(ui, st_r, Id::new("start_spam"), self.sh.is_running(Mode::Spam), &hint, lang, true) {
            acts.push(Act::Toggle(Mode::Spam));
        }
    }

    fn hotkey_card(&mut self, ui: &mut Ui, r: Rect, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>, m: Mode) {
        let lang = s.lang;
        let p = ui.painter().clone();
        card(&p, r);
        card_title(&p, r, tr(lang, "hotkey"));
        let t = if m == Mode::Spam { Target::SpamHotkey } else { Target::PatHotkey };
        let cur = if m == Mode::Spam { combo_label(&s.spam.hotkey, lang) } else { combo_label(&s.pattern.hotkey, lang) };
        let (lbl, recording) = Self::rec_label(s, rec, t, cur);
        let cap = Rect::from_min_size(r.min + vec2(18.0, 44.0), vec2(r.width() - 36.0, 64.0));
        if keycap(ui, cap, Id::new("hk").with(m as u8), &lbl, recording, CapSize::Mid, lang, true).on_hover_text(tr(lang, "click_change")).clicked() {
            acts.push(Act::Record(t));
        }
        let hm = if m == Mode::Spam { &mut s.spam.hk_mode } else { &mut s.pattern.hk_mode };
        let seg = Rect::from_min_size(pos2(cap.min.x, cap.max.y + 12.0), vec2(cap.width(), 38.0));
        segmented(
            ui,
            seg,
            Id::new("hkmode").with(m as u8),
            &[(HkMode::Toggle, tr(lang, "toggle").into()), (HkMode::Hold, tr(lang, "hold").into())],
            hm,
            13.0,
            false,
        );
        let hint = if *hm == HkMode::Toggle { tr(lang, "hk_toggle_hint") } else { tr(lang, "hk_hold_hint") };
        wrapped(&p, Rect::from_min_size(pos2(seg.min.x + 2.0, seg.max.y + 12.0), vec2(seg.width() - 4.0, 40.0)), hint, 12.0, W::R, MUTED, false);
    }

    // ---------------------------------------------------------------- page pattern
    fn pattern_page(&mut self, ui: &mut Ui, c: Rect, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>) {
        let lang = s.lang;
        let inner = Rect::from_min_max(c.min + vec2(22.0, 4.0), c.max - vec2(22.0, 16.0));
        let p = ui.painter().clone();

        // en-tête
        let hy = inner.min.y + 17.0;
        let tr_ = spaced_text(&p, pos2(inner.min.x, hy - 7.0), tr(lang, "steps"), 11.0, MUTED);
        if !s.pattern.steps.is_empty() {
            let d = fmt_duration(lang, config::cycle_seconds(&s.pattern.steps));
            text(&p, pos2(tr_.max.x + 16.0, hy), Align2::LEFT_CENTER, &trf(lang, "cycle", &[("d", d)]), 12.0, W::S, CYAN);
        }
        let tools: [(Ic, Color32, &str); 4] = [(Ic::Spark, AMBER, "example"), (Ic::Open, TEXT, "open"), (Ic::Save, TEXT, "save"), (Ic::Trash, TEXT, "clear")];
        let widths: Vec<f32> = tools.iter().map(|t| text_width(&p, tr(lang, t.2), 13.0, W::S) + 48.0).collect();
        let mut x = inner.max.x - widths.iter().sum::<f32>() - 8.0 * 3.0;
        for (i, (ic, col, key)) in tools.iter().enumerate() {
            let (resp, w) = tool_btn(ui, pos2(x, hy - 16.0), 32.0, Id::new("tool").with(i), *ic, *col, tr(lang, key), false);
            if resp.clicked() {
                match *key {
                    "example" => {
                        s.pattern.steps = config::example_steps();
                        acts.push(Act::StepsChanged);
                    }
                    "open" => acts.push(Act::OpenPreset),
                    "save" => acts.push(Act::SavePreset),
                    _ => {
                        if !s.pattern.steps.is_empty() {
                            self.confirm_clear = true;
                        }
                    }
                }
            }
            x += w + 8.0;
        }

        // frise des étapes
        let tl = Rect::from_min_size(pos2(inner.min.x, inner.min.y + 44.0), vec2(inner.width(), 258.0));
        self.timeline(ui, tl, s, rec, acts);

        // bas : boucles / raccourci / démarrer
        let by = inner.max.y - 92.0;
        let loops_r = Rect::from_min_size(pos2(inner.min.x, by), vec2(300.0, 92.0));
        let st_r = Rect::from_min_size(pos2(inner.max.x - 190.0, by), vec2(190.0, 92.0));
        let hk_r = Rect::from_min_max(pos2(loops_r.max.x + 12.0, by), pos2(st_r.min.x - 12.0, inner.max.y));

        card(&p, loops_r);
        spaced_text(&p, loops_r.min + vec2(16.0, 14.0), tr(lang, "loops"), 11.0, MUTED);
        let ry = loops_r.min.y + 40.0;
        let inf = s.pattern.loops == 0;
        let mut lv = if inf { 1.0 } else { s.pattern.loops as f64 };
        let minus = Rect::from_min_size(pos2(loops_r.min.x + 16.0, ry), vec2(36.0, 36.0));
        let numr = Rect::from_min_size(pos2(minus.max.x + 6.0, ry), vec2(76.0, 36.0));
        let plus = Rect::from_min_size(pos2(numr.max.x + 6.0, ry), vec2(36.0, 36.0));
        for (rr, ic, d, idn) in [(minus, Ic::Min, -1.0, "lminus"), (plus, Ic::Plus, 1.0, "lplus")] {
            let resp = ui.interact(rr, Id::new(idn), if inf { Sense::hover() } else { Sense::click() });
            let hov = !inf && resp.hovered();
            p.rect(rr, 10.0, SURFACE2, sk(1.0, if hov { VIOLET } else { LINE2 }));
            icon(&p, ic, rr.center(), 16.0, if inf { DIM } else { TEXT });
            if !inf && hand(resp).clicked() {
                lv = (lv + d).clamp(1.0, 999_999.0);
            }
        }
        num_field(ui, numr, Id::new("loops"), &mut lv, 1.0, 999_999.0, true, 17.0, !inf, lang);
        let (resp, _) = tool_btn(ui, pos2(plus.max.x + 8.0, ry), 36.0, Id::new("inf"), Ic::Loop, TEXT, tr(lang, "infinite"), inf);
        let mut new_inf = inf;
        if resp.clicked() {
            new_inf = !inf;
        }
        s.pattern.loops = if new_inf { 0 } else { lv as u32 };

        // raccourci (compact)
        card(&p, hk_r);
        spaced_text(&p, hk_r.min + vec2(16.0, 14.0), tr(lang, "hotkey"), 11.0, MUTED);
        let (lbl, recording) = Self::rec_label(s, rec, Target::PatHotkey, combo_label(&s.pattern.hotkey, lang));
        let seg_w = 190.0;
        let cap = Rect::from_min_max(pos2(hk_r.min.x + 16.0, ry), pos2(hk_r.max.x - 16.0 - seg_w - 10.0, ry + 36.0));
        if keycap(ui, cap, Id::new("hk").with(1u8), &lbl, recording, CapSize::Mid, lang, true).on_hover_text(tr(lang, "click_change")).clicked() {
            acts.push(Act::Record(Target::PatHotkey));
        }
        let seg = Rect::from_min_size(pos2(cap.max.x + 10.0, ry), vec2(seg_w, 36.0));
        segmented(
            ui,
            seg,
            Id::new("hkmode").with(1u8),
            &[(HkMode::Toggle, tr(lang, "toggle").into()), (HkMode::Hold, tr(lang, "hold").into())],
            &mut s.pattern.hk_mode,
            13.0,
            false,
        );
        let hint_txt = if s.pattern.hk_mode == HkMode::Toggle { tr(lang, "hk_toggle_hint") } else { tr(lang, "hk_hold_hint") };
        ui.interact(seg, Id::new("hkmode_tip"), Sense::hover()).on_hover_text(hint_txt);

        let hint = Self::hotkey_hint(s, Mode::Pattern, rec);
        if start_button(ui, st_r, Id::new("start_pat"), self.sh.is_running(Mode::Pattern), &hint, lang, false) {
            acts.push(Act::Toggle(Mode::Pattern));
        }
    }

    fn timeline(&mut self, ui: &mut Ui, tl: Rect, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>) {
        let lang = s.lang;
        let (cw, ch, conn, addw) = (214.0, 238.0, 28.0, 190.0);
        let n = s.pattern.steps.len();
        let active = if self.sh.is_running(Mode::Pattern) { self.sh.runner.status.step.load(SeqCst) } else { -1 };
        let mut action: Option<(&'static str, usize)> = None;
        let mut add: Option<&'static str> = None;

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(tl), |ui| {
            egui::ScrollArea::horizontal().id_salt("timeline").auto_shrink([false, false]).show(ui, |ui| {
                let empty_w = (tl.width() - addw - 12.0).max(300.0);
                let total = if n == 0 { empty_w + 12.0 + addw } else { n as f32 * cw + (n as f32 - 1.0) * conn + 12.0 + addw };
                let (area, _) = ui.allocate_exact_size(vec2(total, ch + 2.0), Sense::hover());
                let o = area.min;
                let p = ui.painter().clone();
                let mut x = o.x;
                if n == 0 {
                    let r = Rect::from_min_size(pos2(x, o.y), vec2(empty_w, ch));
                    card(&p, r);
                    icon(&p, Ic::Keyboard, pos2(r.center().x, r.center().y - 44.0), 46.0, MUTED);
                    text(&p, pos2(r.center().x, r.center().y + 2.0), Align2::CENTER_CENTER, tr(lang, "empty"), 14.0, W::R, MUTED);
                    let bw = text_width(&p, tr(lang, "example"), 13.0, W::S) + 48.0;
                    let (resp, _) = tool_btn(
                        ui,
                        pos2(r.center().x - bw / 2.0, r.center().y + 26.0),
                        34.0,
                        Id::new("ex_empty"),
                        Ic::Spark,
                        AMBER,
                        tr(lang, "example"),
                        false,
                    );
                    if resp.clicked() {
                        add = Some("example");
                    }
                    x += empty_w + 12.0;
                }
                for i in 0..n {
                    let r = Rect::from_min_size(pos2(x, o.y), vec2(cw, ch));
                    if active == i as i32 && self.last_active != active {
                        ui.scroll_to_rect(r, Some(Align::Center));
                    }
                    if let Some(a) = self.step_card(ui, r, i, n, &mut s.pattern.steps[i], rec, active == i as i32, lang, acts) {
                        action = Some((a, i));
                    }
                    x += cw;
                    if i + 1 < n {
                        icon(&p, Ic::Right, pos2(x + conn / 2.0, o.y + ch / 2.0), 16.0, rgb(0x3A4054));
                        x += conn;
                    }
                }
                if n > 0 {
                    x += 12.0;
                }
                // carte d'ajout
                let r = Rect::from_min_size(pos2(x, o.y), vec2(addw, ch));
                dashed_rrect(&p, r.shrink(1.0), 16.0, sk(2.0, rgb(0x2A3042)));
                text(&p, pos2(r.center().x, r.min.y + 28.0), Align2::CENTER_CENTER, tr(lang, "add"), 13.0, W::S, TEXT);
                let by = r.min.y + 62.0;
                for (j, (kind, ic)) in [("hold", Ic::Hold), ("press", Ic::Press), ("wait", Ic::Wait)].iter().enumerate() {
                    let br = Rect::from_min_size(pos2(r.min.x + 14.0, by + j as f32 * 50.0), vec2(addw - 28.0, 42.0));
                    let resp = hand(ui.interact(br, Id::new("add").with(j), Sense::click()));
                    let hov = resp.hovered();
                    p.rect(br, 11.0, if hov { rgb(0x1C2030) } else { SURFACE2 }, sk(1.0, if hov { VIOLET } else { LINE2 }));
                    icon(&p, *ic, pos2(br.min.x + 22.0, br.center().y), 18.0, kind_color(kind));
                    text(&p, pos2(br.min.x + 42.0, br.center().y), Align2::LEFT_CENTER, tr(lang, &format!("k_{kind}")), 13.0, W::S, TEXT);
                    if resp.clicked() {
                        add = Some(kind);
                    }
                }
                if add.is_some() && add != Some("example") {
                    ui.scroll_to_rect(r, Some(Align::Max));
                }
            });
        });
        self.last_active = active;

        let steps = &mut s.pattern.steps;
        if let Some(kind) = add {
            match kind {
                "hold" => steps.push(Step::new_hold()),
                "press" => steps.push(Step::new_press()),
                "wait" => steps.push(Step::new_wait()),
                _ => *steps = config::example_steps(),
            }
            acts.push(Act::StepsChanged);
        }
        if let Some((a, i)) = action {
            match a {
                "left" if i > 0 => steps.swap(i, i - 1),
                "right" if i + 1 < steps.len() => steps.swap(i, i + 1),
                "copy" => {
                    let c = steps[i].clone();
                    steps.insert(i + 1, c);
                }
                "trash" => {
                    steps.remove(i);
                }
                _ => {}
            }
            acts.push(Act::StepsChanged);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn step_card(
        &mut self,
        ui: &mut Ui,
        r: Rect,
        i: usize,
        n: usize,
        step: &mut Step,
        rec: &Option<Recording>,
        active: bool,
        lang: Lang,
        acts: &mut Vec<Act>,
    ) -> Option<&'static str> {
        let p = ui.painter().clone();
        let kind = step.kind();
        let col = kind_color(kind);
        let id = Id::new("step").with(i);
        let glow = ui.ctx().animate_bool_with_time(id.with("act"), active, 0.2);
        if glow > 0.0 {
            p.rect_stroke(r.expand(3.0), 19.0, sk(4.0, CYAN.gamma_multiply(0.18 * glow)));
        }
        p.rect(r, 16.0, lerp_c(SURFACE, rgb(0x101B22), glow), sk(1.0 + glow, lerp_c(CARD_LINE, CYAN, glow)));
        let mut out = None;

        // en-tête
        let hy = r.min.y + 25.0;
        let badge = Rect::from_center_size(pos2(r.min.x + 27.0, hy), vec2(26.0, 26.0));
        p.rect_filled(badge, 8.0, col.gamma_multiply(0.16));
        let bic = match kind {
            "hold" => Ic::Hold,
            "press" => Ic::Press,
            _ => Ic::Wait,
        };
        icon(&p, bic, badge.center(), 17.0, col);
        let tr_ = text(&p, pos2(badge.max.x + 9.0, hy), Align2::LEFT_CENTER, tr(lang, &format!("k_{kind}")), 13.0, W::B, col);
        text(&p, pos2(tr_.max.x + 7.0, hy), Align2::LEFT_CENTER, &(i + 1).to_string(), 12.0, W::B, MUTED);
        let del = Rect::from_center_size(pos2(r.max.x - 25.0, hy), vec2(26.0, 26.0));
        if mini_btn(ui, del, id.with("del"), Ic::Trash, tr(lang, "del"), true, true) {
            out = Some("trash");
        }

        // touche
        let cap = Rect::from_min_size(pos2(r.min.x + 14.0, r.min.y + 48.0), vec2(r.width() - 28.0, 56.0));
        match step {
            Step::Hold { key, .. } | Step::Press { key, .. } => {
                let t = Target::StepKey(i);
                let (lbl, recording) = match rec {
                    Some(rr) if rr.target == t => (String::new(), true),
                    _ => (key.label(lang), false),
                };
                if keycap(ui, cap, id.with("cap"), &lbl, recording, CapSize::Step, lang, false).on_hover_text(tr(lang, "click_change")).clicked() {
                    acts.push(Act::Record(t));
                }
            }
            Step::Wait { .. } => icon(&p, Ic::Wait, cap.center(), 42.0, col),
        }

        // paramètres
        let row1 = Rect::from_min_size(pos2(r.min.x + 14.0, r.min.y + 116.0), vec2(r.width() - 28.0, 34.0));
        let row2 = row1.translate(vec2(0.0, 42.0));
        let field = |row: Rect| Rect::from_min_size(pos2(row.max.x - 96.0, row.min.y), vec2(96.0, 34.0));
        match step {
            Step::Hold { value, unit, .. } | Step::Wait { value, unit } => {
                text(&p, pos2(row1.min.x + 2.0, row1.center().y), Align2::LEFT_CENTER, tr(lang, "for"), 12.0, W::R, MUTED);
                num_field(ui, field(row1), id.with("val"), value, 0.0, 999_999.0, false, 17.0, true, lang);
                let items: Vec<(Unit, String)> = Unit::ALL.iter().map(|u| (*u, u.label().to_string())).collect();
                segmented(ui, row2, id.with("unit"), &items, unit, 12.0, false);
            }
            Step::Press { count, gap, .. } => {
                text(&p, pos2(row1.min.x + 2.0, row1.center().y), Align2::LEFT_CENTER, tr(lang, "times"), 12.0, W::R, MUTED);
                let mut cv = *count as f64;
                if num_field(ui, field(row1), id.with("cnt"), &mut cv, 1.0, 100_000.0, true, 17.0, true, lang) {
                    *count = cv as u32;
                }
                text(&p, pos2(row2.min.x + 2.0, row2.center().y), Align2::LEFT_CENTER, &format!("{} (ms)", tr(lang, "gap")), 12.0, W::R, MUTED);
                num_field(ui, field(row2), id.with("gap"), gap, 0.0, 600_000.0, false, 17.0, true, lang);
            }
        }

        // pied : déplacer / dupliquer
        let fy = r.max.y - 26.0;
        if mini_btn(ui, Rect::from_center_size(pos2(r.min.x + 24.0, fy), vec2(26.0, 24.0)), id.with("l"), Ic::Left, tr(lang, "left"), i > 0, false) {
            out = Some("left");
        }
        if mini_btn(ui, Rect::from_center_size(pos2(r.min.x + 52.0, fy), vec2(26.0, 24.0)), id.with("r"), Ic::Right, tr(lang, "right"), i + 1 < n, false) {
            out = Some("right");
        }
        if mini_btn(ui, Rect::from_center_size(pos2(r.max.x - 25.0, fy), vec2(26.0, 24.0)), id.with("c"), Ic::Copy, tr(lang, "dup"), true, false) {
            out = Some("copy");
        }
        out
    }
}
