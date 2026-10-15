//! Barre de titre, popups, modal de confirmation et pied de page.
use super::*;

impl App {
    // ---------------------------------------------------------------- barre de titre
    pub(super) fn titlebar(&mut self, ui: &mut Ui, bar: Rect, s: &mut Settings, acts: &mut Vec<Act>) {
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

    pub(super) fn lang_popup(&mut self, ui: &mut Ui, full: Rect, s: &mut Settings, acts: &mut Vec<Act>) {
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

    pub(super) fn settings_popup(&mut self, ui: &mut Ui, full: Rect, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>) {
        let lang = s.lang;
        let gear_right = full.max.x - 12.0 - 34.0 - 38.0 - 38.0;
        let r = Rect::from_min_size(pos2(gear_right - 300.0, full.min.y + 52.0), vec2(300.0, 4.0 * 50.0 + 12.0));
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
        // aléa sur les délais
        text(&p, pos2(r.min.x + 16.0, row(1.0)), Align2::LEFT_CENTER, tr(lang, "jitter"), 13.0, W::S, TEXT);
        let jr = Rect::from_center_size(pos2(r.max.x - 16.0 - 40.0 - 22.0, row(1.0)), vec2(80.0, 36.0));
        num_field(ui, jr, Id::new("jitterpct"), &mut s.jitter_pct, 0.0, config::MAX_JITTER_PCT, true, 16.0, true, lang);
        inside |= ui.rect_contains_pointer(jr);
        text(&p, pos2(jr.max.x + 8.0, row(1.0)), Align2::LEFT_CENTER, "%", 12.0, W::S, MUTED);
        // bip démarrage / arrêt
        text(&p, pos2(r.min.x + 16.0, row(2.0)), Align2::LEFT_CENTER, tr(lang, "beep"), 13.0, W::S, TEXT);
        let br = Rect::from_center_size(pos2(r.max.x - 16.0 - 45.0, row(2.0)), vec2(90.0, 36.0));
        segmented(ui, br, Id::new("beep"), &[(false, tr(lang, "off").into()), (true, tr(lang, "on").into())], &mut s.beep, 13.0, false);
        inside |= ui.rect_contains_pointer(br);
        // arrêt d'urgence
        text(&p, pos2(r.min.x + 16.0, row(3.0)), Align2::LEFT_CENTER, tr(lang, "stop_hk"), 13.0, W::S, TEXT);
        let (lbl, recording) = Self::rec_label(s, rec, Target::StopHotkey, combo_label(&s.stop_hotkey, lang));
        let cr = Rect::from_center_size(pos2(r.max.x - 16.0 - 60.0, row(3.0)), vec2(120.0, 36.0));
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

    pub(super) fn confirm_modal(&mut self, ui: &mut Ui, full: Rect, s: &mut Settings, acts: &mut Vec<Act>) {
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
    pub(super) fn footer(&mut self, ui: &mut Ui, r: Rect, s: &Settings) {
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
        // temps écoulé (secondes entières)
        let elapsed = match *st.started.lock().unwrap() {
            Some(t) if running => {
                let e = t.elapsed().as_secs();
                format!(" · {}", if e == 0 { "0 s".to_string() } else { fmt_duration(lang, e as f64) })
            }
            _ => String::new(),
        };
        if running && self.sh.runner.mode() == 0 {
            msg = trf(lang, "running_spam", &[("n", st.count.load(SeqCst).to_string())]);
            msg += &elapsed;
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
                msg += &elapsed;
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
}
