//! Primitives de dessin et widgets.
use super::*;

pub(super) fn card(p: &Painter, r: Rect) {
    p.rect(r, 16.0, SURFACE, sk(1.0, CARD_LINE));
}

pub(super) fn spaced_text(p: &Painter, pos: Pos2, text: &str, size: f32, color: Color32) -> Rect {
    let mut job = egui::text::LayoutJob::default();
    job.append(text, 0.0, egui::TextFormat { font_id: font(size, W::B), color, extra_letter_spacing: 1.2, ..Default::default() });
    let g = p.layout_job(job);
    let r = Rect::from_min_size(pos, g.size());
    p.galley(pos, g, color);
    r
}

pub(super) fn card_title(p: &Painter, r: Rect, text: &str) -> Rect {
    spaced_text(p, r.min + vec2(18.0, 16.0), text, 11.0, MUTED)
}

pub(super) fn text(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, w: W, color: Color32) -> Rect {
    p.text(pos, anchor, s, font(size, w), color)
}

pub(super) fn text_width(p: &Painter, s: &str, size: f32, w: W) -> f32 {
    p.layout_no_wrap(s.to_string(), font(size, w), TEXT).size().x
}

/// Texte tronqué avec « … » pour tenir dans `maxw`
pub(super) fn fit_text(p: &Painter, s: &str, size: f32, w: W, maxw: f32) -> String {
    if text_width(p, s, size, w) <= maxw {
        return s.to_string();
    }
    let mut t: String = s.to_string();
    while !t.is_empty() && text_width(p, &format!("{t}…"), size, w) > maxw {
        t.pop();
    }
    format!("{}…", t.trim_end())
}

pub(super) fn wrapped(p: &Painter, r: Rect, s: &str, size: f32, w: W, color: Color32, center: bool) {
    let mut job = egui::text::LayoutJob::simple(s.to_string(), font(size, w), color, r.width());
    if center {
        job.halign = Align::Center;
    }
    let g = p.layout_job(job);
    let pos = if center { pos2(r.center().x, r.center().y - g.size().y / 2.0) } else { r.min };
    p.galley(pos, g, color);
}

pub(super) fn dashed_rrect(p: &Painter, r: Rect, rad: f32, stroke: Stroke) {
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
pub(super) enum Ic {
    Close,
    Min,
    Pin,
    Shrink,
    Expand,
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
    Edit,
    Rec,
}

pub(super) fn icon(p: &Painter, ic: Ic, c: Pos2, size: f32, color: Color32) {
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
        Ic::Shrink => {
            poly(vec![pt(0.58, 0.26), pt(0.58, 0.42), pt(0.74, 0.42)]);
            line(pt(0.58, 0.42), pt(0.8, 0.2));
            poly(vec![pt(0.42, 0.74), pt(0.42, 0.58), pt(0.26, 0.58)]);
            line(pt(0.42, 0.58), pt(0.2, 0.8));
        }
        Ic::Expand => {
            poly(vec![pt(0.6, 0.22), pt(0.78, 0.22), pt(0.78, 0.4)]);
            line(pt(0.78, 0.22), pt(0.56, 0.44));
            poly(vec![pt(0.4, 0.78), pt(0.22, 0.78), pt(0.22, 0.6)]);
            line(pt(0.22, 0.78), pt(0.44, 0.56));
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
        Ic::Rec => {
            p.circle_stroke(c, s * 0.34, st);
            p.circle_filled(c, s * 0.17, color);
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
        Ic::Edit => poly(vec![pt(0.24, 0.76), pt(0.28, 0.6), pt(0.62, 0.26), pt(0.74, 0.38), pt(0.4, 0.72), pt(0.24, 0.76)]),
        Ic::Keyboard => {
            p.rect_stroke(Rect::from_min_max(pt(0.12, 0.3), pt(0.88, 0.72)), s * 0.07, st);
            line(pt(0.32, 0.6), pt(0.68, 0.6));
            for x in [0.28, 0.42, 0.58, 0.72] {
                p.circle_filled(pt(x, 0.44), s * 0.025, color);
            }
        }
    }
}

pub(super) fn logo(p: &Painter, r: Rect) {
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
pub(super) fn hand(r: Response) -> Response {
    r.on_hover_cursor(CursorIcon::PointingHand)
}

pub(super) fn icon_btn(ui: &mut Ui, r: Rect, id: &str, ic: Ic, tip: &str, checked: bool, danger: bool) -> Response {
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

pub(super) fn mini_btn(ui: &mut Ui, r: Rect, id: Id, ic: Ic, tip: &str, enabled: bool, danger: bool) -> bool {
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
pub(super) fn tool_btn(ui: &mut Ui, pos: Pos2, h: f32, id: Id, ic: Ic, ic_col: Color32, label: &str, checked: bool) -> (Response, f32) {
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
pub(super) fn segmented<T: PartialEq + Copy>(ui: &mut Ui, r: Rect, id: Id, items: &[(T, String)], cur: &mut T, size: f32, grad: bool) -> bool {
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

/// Petit « i » d'aide centré en `center` : info-bulle (fond SURFACE, bord LINE2, ≤ 260 px, retour à la ligne) au survol.
/// Peinte sur la couche `Tooltip`, bornée à la fenêtre (sous le « i », sinon au-dessus ; décalée à gauche près du bord).
pub(super) fn info_tip(ui: &mut Ui, center: Pos2, id: Id, tip: &str) {
    const R: f32 = 7.0;
    let resp = ui.interact(Rect::from_center_size(center, Vec2::splat(R * 2.0 + 4.0)), id, Sense::hover());
    let hov = resp.hovered();
    #[cfg(feature = "shots")]
    let hov = hov || ui.ctx().data(|d| d.get_temp::<Id>(Id::new("force_tip"))) == Some(id);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Help);
    }
    let p = ui.painter();
    p.circle(center, R, if hov { SURFACE2 } else { Color32::TRANSPARENT }, sk(1.0, if hov { MUTED } else { LINE2 }));
    text(p, pos2(center.x, center.y + 0.5), Align2::CENTER_CENTER, "i", 10.0, W::B, if hov { TEXT } else { MUTED });
    if !hov || tip.is_empty() {
        return;
    }
    let tp = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, id.with("tip")));
    let job = egui::text::LayoutJob::simple(tip.to_string(), font(12.0, W::R), TEXT, 260.0 - 24.0);
    let g = tp.layout_job(job);
    let size = g.size() + vec2(24.0, 20.0);
    let screen = ui.ctx().screen_rect();
    let mut x = center.x - 14.0;
    x = x.min(screen.max.x - 6.0 - size.x).max(screen.min.x + 6.0);
    let mut y = center.y + R + 6.0;
    if y + size.y > screen.max.y - 6.0 {
        y = (center.y - R - 6.0 - size.y).max(screen.min.y + 6.0);
    }
    let r = Rect::from_min_size(pos2(x, y), size);
    tp.rect_filled(r.translate(vec2(0.0, 4.0)).expand(1.0), 10.0, Color32::from_black_alpha(90));
    tp.rect(r, 10.0, SURFACE, sk(1.0, LINE2));
    tp.galley(r.min + vec2(12.0, 10.0), g, TEXT);
}

/// « i » d'aide juste à droite du texte `after` (libellé de section) ; `key` identifie l'info-bulle (`tip.<clé>`)
pub(super) fn tip_after(ui: &mut Ui, after: Rect, key: &str, tip: &str) {
    info_tip(ui, pos2(after.max.x + 14.0, after.center().y), Id::new("tip").with(key), tip);
}

/// Bouton texte rectangulaire (actions des paramètres) ; `danger` = liseré rose
pub(super) fn pill_btn(ui: &mut Ui, r: Rect, id: Id, label: &str, danger: bool, enabled: bool) -> bool {
    let resp = ui.interact(r, id, if enabled { Sense::click() } else { Sense::hover() });
    let hov = enabled && resp.hovered();
    if hov {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let accent = if danger { ROSE } else { VIOLET };
    let p = ui.painter();
    p.rect(r, 10.0, SURFACE2, sk(1.0, if hov { accent } else { LINE2 }));
    let label = fit_text(p, label, 13.0, W::S, r.width() - 16.0);
    let col = if !enabled {
        DIM
    } else if danger {
        lighten(ROSE, 0.15)
    } else {
        TEXT
    };
    text(p, r.center(), Align2::CENTER_CENTER, &label, 13.0, W::S, col);
    enabled && resp.clicked()
}

/// Version grisée (non interactive) d'un interrupteur Non / Oui
pub(super) fn toggle_off(p: &Painter, r: Rect, on: bool, labels: [&str; 2], size: f32) {
    p.rect(r, 10.0, SURFACE, sk(1.0, LINE));
    let inner = r.shrink(3.0);
    let bw = inner.width() / 2.0;
    for (i, l) in labels.iter().enumerate() {
        let br = Rect::from_min_size(pos2(inner.min.x + i as f32 * bw, inner.min.y), vec2(bw, inner.height()));
        if (i == 1) == on {
            p.rect_filled(br, 7.0, SURFACE2);
        }
        text(p, br.center(), Align2::CENTER_CENTER, l, size, W::S, DIM);
    }
}

#[derive(Clone, Copy)]
pub(super) enum CapSize {
    Big,
    Mid,
    Step,
}

/// Touche « capturable » : clic → écoute du clavier
pub(super) fn keycap(ui: &mut Ui, r: Rect, id: Id, label: &str, recording: bool, size: CapSize, lang: Lang, combo: bool) -> Response {
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

pub(super) fn parse_num(s: &str, lo: f64, hi: f64, int: bool) -> Option<f64> {
    let v: f64 = s.trim().replace(',', ".").parse().ok()?;
    if !v.is_finite() {
        return None;
    }
    let v = v.clamp(lo, hi);
    Some(if int { v.round() } else { v })
}

/// Champ numérique : saisie libre (virgule ou point) + molette
pub(super) fn num_field(ui: &mut Ui, r: Rect, id: Id, v: &mut f64, lo: f64, hi: f64, int: bool, size: f32, enabled: bool, lang: Lang) -> bool {
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
    // la molette ne modifie que le champ ayant le focus : sinon le défilement du parent reste intact
    if resp.has_focus() {
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
                ui.data_mut(|d| d.insert_temp(id, fmt(*v)));
            }
        }
    }
    changed
}

pub(super) fn chip(ui: &mut Ui, pos: Pos2, id: Id, label: &str) -> (bool, f32) {
    let w = text_width(ui.painter(), label, 12.0, W::S) + 22.0;
    let r = Rect::from_min_size(pos, vec2(w, 26.0));
    let resp = hand(ui.interact(r, id, Sense::click()));
    let p = ui.painter();
    let hov = resp.hovered();
    p.rect(r, 13.0, if hov { rgb(0x1C2030) } else { Color32::TRANSPARENT }, sk(1.0, if hov { VIOLET } else { LINE2 }));
    text(p, r.center(), Align2::CENTER_CENTER, label, 12.0, W::S, if hov { TEXT } else { rgb(0xA9AEC2) });
    (resp.clicked(), w)
}

pub(super) fn start_button(ui: &mut Ui, r: Rect, id: Id, running: bool, hint: &str, lang: Lang, with_icon: bool) -> bool {
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

pub(super) fn rhythm(p: &Painter, r: Rect, iv: f64, lang: Lang) {
    let win = if iv <= 1.0 {
        1.0
    } else if iv <= 60.0 {
        60.0
    } else {
        3600.0
    };
    let unit = fmt_duration(lang, win);
    let track = Rect::from_min_size(r.min + vec2(0.0, 4.0), vec2(r.width(), 20.0));
    p.rect_filled(track, 8.0, SURFACE2);
    let n = (win / iv.max(0.001)) as usize;
    if n > 80 {
        let inner = track.shrink(3.0);
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
    text(p, pos2(r.min.x + 2.0, track.max.y + 10.0), Align2::LEFT_CENTER, &fmt_num(lang, 0.0), 11.0, W::R, MUTED);
    text(p, pos2(r.max.x - 2.0, track.max.y + 10.0), Align2::RIGHT_CENTER, &unit, 11.0, W::R, MUTED);
}
