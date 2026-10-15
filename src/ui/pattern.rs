//! Page pattern, timeline et cartes d'étapes.
use super::*;

impl App {
    // ---------------------------------------------------------------- page pattern
    pub(super) fn pattern_page(&mut self, ui: &mut Ui, c: Rect, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>) {
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
        if let Some((n, el)) = self.sh.macro_progress() {
            // capture de macro en cours : seul le bouton d'arrêt (rouge, pulsé) reste
            let label = trf(lang, "macro_stop", &[("n", n.to_string()), ("t", fmt_duration(lang, el.as_secs() as f64))]);
            let w = text_width(&p, &label, 13.0, W::S) + 16.0 + 8.0 + 24.0;
            let r = Rect::from_min_size(pos2(inner.max.x - w, hy - 16.0), vec2(w, 32.0));
            let resp = hand(ui.interact(r, Id::new("tool_macro"), Sense::click()));
            #[allow(unused_mut)]
            let mut pulse = (ui.input(|i| i.time) * std::f64::consts::TAU / 1.2).sin() as f32 * 0.5 + 0.5;
            #[cfg(feature = "shots")]
            if self.shots.is_some() {
                pulse = 1.0; // captures déterministes
            }
            p.rect(r, 10.0, ROSE.gamma_multiply(0.18 + 0.2 * pulse), sk(1.0, if resp.hovered() { lighten(ROSE, 0.2) } else { ROSE }));
            icon(&p, Ic::Rec, pos2(r.min.x + 20.0, r.center().y), 16.0, lerp_c(ROSE, Color32::WHITE, pulse * 0.5));
            text(&p, pos2(r.min.x + 36.0, r.center().y), Align2::LEFT_CENTER, &label, 13.0, W::S, TEXT);
            if resp.clicked() {
                acts.push(Act::MacroToggle);
            }
        } else {
            let tools: [(Ic, Color32, &str); 5] =
                [(Ic::Rec, ROSE, "macro_rec"), (Ic::Spark, AMBER, "example"), (Ic::Open, TEXT, "open"), (Ic::Save, TEXT, "save"), (Ic::Trash, TEXT, "clear")];
            let widths: Vec<f32> = tools.iter().map(|t| text_width(&p, tr(lang, t.2), 13.0, W::S) + 48.0).collect();
            let mut x = inner.max.x - widths.iter().sum::<f32>() - 8.0 * 4.0;
            for (i, (ic, col, key)) in tools.iter().enumerate() {
                let (resp, w) = tool_btn(ui, pos2(x, hy - 16.0), 32.0, Id::new("tool").with(i), *ic, *col, tr(lang, key), false);
                let resp = if *key == "macro_rec" {
                    let k = combo_label(&s.stop_hotkey, lang);
                    resp.on_hover_text(trf(lang, "macro_tip", &[("k", k)]))
                } else {
                    resp
                };
                if resp.clicked() {
                    match *key {
                        "macro_rec" => acts.push(Act::MacroToggle),
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

    pub(super) fn timeline(&mut self, ui: &mut Ui, tl: Rect, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>) {
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
    pub(super) fn step_card(
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
