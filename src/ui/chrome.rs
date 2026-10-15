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
        let vr = text(&p, pos2(r.max.x + 8.0, bar.center().y + 1.0), Align2::LEFT_CENTER, concat!("v", env!("CARGO_PKG_VERSION")), 11.0, W::R, MUTED);
        // vue réduite (à gauche : la zone de droite est déjà pleine quand le bouton de mise à jour est visible)
        let cr = Rect::from_min_size(pos2(vr.max.x + 14.0, bar.center().y - 17.0), vec2(34.0, 34.0));
        if icon_btn(ui, cr, "compactbtn", Ic::Shrink, tr(lang, "compact"), false, false).clicked() {
            s.compact = true;
            self.close_popups();
            acts.push(Act::Resize(s.mode, true));
        }

        // onglets
        let tabs = Rect::from_center_size(bar.center(), vec2(236.0, 40.0));
        let mut mode = s.mode;
        if segmented(ui, tabs, Id::new("tabs"), &[(Mode::Spam, tr(lang, "spam").into()), (Mode::Pattern, tr(lang, "pattern").into())], &mut mode, 13.0, true) {
            s.mode = mode;
            acts.push(Act::Resize(mode, false));
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
            self.profiles_open = false;
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
            self.profiles_open = false;
        }

        // profils (nom du profil actif, tronqué)
        // (réduit à l'icône quand le bouton de mise à jour doit tenir entre les onglets et la langue)
        let compact = matches!(self.up.state(), crate::update::UpState::Available { .. });
        let pw = if compact { 34.0 } else { 150.0 };
        let pr = Rect::from_min_size(pos2(lr.min.x - 10.0 - pw, bar.center().y - 16.0), vec2(pw, 32.0));
        let resp = hand(ui.interact(pr, Id::new("profbtn"), Sense::click())).on_hover_text(tr(lang, "profiles"));
        let active = s.active_profile.and_then(|i| s.profiles.get(i));
        let p = ui.painter();
        p.rect(pr, 9.0, SURFACE, sk(1.0, if resp.hovered() || self.profiles_open { VIOLET } else { LINE }));
        let ic_x = if compact { pr.center().x } else { pr.min.x + 20.0 };
        icon(p, Ic::Open, pos2(ic_x, pr.center().y), 16.0, if active.is_some() { lighten(VIOLET, 0.3) } else { TEXT });
        let label = active.map(|a| a.name.as_str()).unwrap_or_else(|| tr(lang, "profiles"));
        let label = fit_text(p, label, 13.0, W::S, pw - 34.0 - 12.0);
        if !compact {
            text(p, pos2(pr.min.x + 34.0, pr.center().y), Align2::LEFT_CENTER, &label, 13.0, W::S, TEXT);
        }
        if resp.clicked() {
            self.profiles_open = !self.profiles_open;
            self.prof_rename = None;
            self.lang_open = false;
            self.settings_open = false;
        }

        // bouton de mise à jour (visible seulement si une nouvelle version existe)
        if let crate::update::UpState::Available { build, .. } = self.up.state() {
            let label = tr(lang, "update");
            let uw = text_width(ui.painter(), label, 13.0, W::S) + 44.0;
            let ur = Rect::from_min_size(pos2(pr.min.x - 10.0 - uw, bar.center().y - 16.0), vec2(uw, 32.0));
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

    fn close_popups(&mut self) {
        self.lang_open = false;
        self.settings_open = false;
        self.profiles_open = false;
        self.prof_rename = None;
    }

    /// Barre de titre de la vue réduite : logo, retour à la vue complète, épingle, réduire, fermer
    pub(super) fn titlebar_compact(&mut self, ui: &mut Ui, bar: Rect, s: &mut Settings, acts: &mut Vec<Act>) {
        let lang = s.lang;
        let drag = ui.interact(bar, Id::new("drag"), Sense::click_and_drag());
        if drag.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        let p = ui.painter().clone();
        logo(&p, Rect::from_min_size(bar.min + vec2(16.0, 16.0), vec2(28.0, 28.0)));
        text(&p, pos2(bar.min.x + 54.0, bar.center().y), Align2::LEFT_CENTER, "AutoKey", 15.0, W::B, TEXT);
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
        x -= 38.0;
        if icon_btn(ui, br(x), "compactbtn", Ic::Expand, tr(lang, "compact_back"), false, false).clicked() {
            s.compact = false;
            acts.push(Act::Resize(s.mode, false));
        }
        x -= 38.0;
        if icon_btn(ui, br(x), "gearbtn", Ic::Gear, tr(lang, "settings"), self.settings_open, false).clicked() {
            self.settings_open = !self.settings_open;
        }
    }

    /// Vue réduite : mode, raccourci, démarrer / arrêter (l'état est dans le pied de page)
    pub(super) fn compact_page(&mut self, ui: &mut Ui, c: Rect, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>) {
        let lang = s.lang;
        let inner = Rect::from_min_max(c.min + vec2(20.0, 10.0), c.max - vec2(20.0, 14.0));
        let tabs = Rect::from_min_size(inner.min, vec2(inner.width(), 36.0));
        let mut mode = s.mode;
        if segmented(ui, tabs, Id::new("ctabs"), &[(Mode::Spam, tr(lang, "spam").into()), (Mode::Pattern, tr(lang, "pattern").into())], &mut mode, 13.0, true) {
            s.mode = mode;
        }
        let top = tabs.max.y + 12.0;
        let h = inner.max.y - top;
        let kw = 170.0;
        let key_r = Rect::from_min_size(pos2(inner.min.x, top), vec2(kw, h));
        let st_r = Rect::from_min_max(pos2(key_r.max.x + 14.0, top), inner.max);
        let p = ui.painter().clone();
        card(&p, key_r);
        card_title(&p, key_r, tr(lang, "hotkey"));
        let t = if s.mode == Mode::Spam { Target::SpamHotkey } else { Target::PatHotkey };
        let cur = if s.mode == Mode::Spam { combo_label(&s.spam.hotkey, lang) } else { combo_label(&s.pattern.hotkey, lang) };
        let (lbl, recording) = Self::rec_label(s, rec, t, cur);
        let cap = Rect::from_min_max(key_r.min + vec2(14.0, 44.0), key_r.max - vec2(14.0, 16.0));
        if keycap(ui, cap, Id::new("chk").with(s.mode as u8), &lbl, recording, CapSize::Mid, lang, true).on_hover_text(tr(lang, "click_change")).clicked() {
            acts.push(Act::Record(t));
        }
        let hint = Self::hotkey_hint(s, s.mode, rec);
        if start_button(ui, st_r, Id::new("start_compact"), self.sh.is_running(s.mode), &hint, lang, false) {
            acts.push(Act::Toggle(s.mode));
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

    pub(super) fn profiles_popup(&mut self, ui: &mut Ui, full: Rect, s: &mut Settings, acts: &mut Vec<Act>) {
        const ROWS: usize = 6;
        const RH: f32 = 34.0;
        let lang = s.lang;
        let right = full.max.x - 12.0 - 34.0 - 38.0 - 38.0 - 38.0 - 10.0 - 74.0 - 10.0;
        let n = s.profiles.len();
        let shown = n.clamp(1, ROWS);
        let r = Rect::from_min_size(pos2(right - 360.0, full.min.y + 52.0), vec2(360.0, 6.0 + 36.0 + 6.0 + shown as f32 * RH + 6.0));
        let p = ui.painter().clone();
        p.rect_filled(r.translate(vec2(0.0, 6.0)).expand(2.0), 14.0, Color32::from_black_alpha(90));
        p.rect(r, 12.0, SURFACE, sk(1.0, LINE2));
        let blocker = ui.interact(r, Id::new("profpop"), Sense::click());
        let mut inside = blocker.hovered();

        // nouveau profil
        let nb = Rect::from_min_size(r.min + vec2(6.0, 6.0), vec2(r.width() - 12.0, 36.0));
        let enabled = n < config::MAX_PROFILES;
        let nbi = Rect::from_min_max(nb.min, pos2(nb.max.x - 70.0, nb.max.y)); // laisse la place au « i »
        let resp = ui.interact(nbi, Id::new("prof_new"), if enabled { Sense::click() } else { Sense::hover() });
        inside |= resp.hovered();
        let hov = enabled && resp.hovered();
        p.rect(nb, 10.0, SURFACE2, sk(1.0, if hov { VIOLET } else { LINE2 }));
        let col = if enabled { TEXT } else { DIM };
        icon(&p, Ic::Plus, pos2(nb.min.x + 22.0, nb.center().y), 16.0, if enabled { CYAN } else { DIM });
        text(&p, pos2(nb.min.x + 38.0, nb.center().y), Align2::LEFT_CENTER, tr(lang, "prof_new"), 13.0, W::S, col);
        let cnt = text(&p, pos2(nb.max.x - 12.0, nb.center().y), Align2::RIGHT_CENTER, &format!("{n}/{}", config::MAX_PROFILES), 11.0, W::S, MUTED);
        info_tip(ui, pos2(cnt.min.x - 16.0, nb.center().y), Id::new("tip").with("profiles"), tr(lang, "tip_profiles"));
        if enabled && hand(resp).clicked() {
            acts.push(Act::ProfileNew);
        }

        // liste (défilement à la molette au-delà de 6 profils)
        let top = nb.max.y + 6.0;
        if n == 0 {
            text(&p, pos2(r.center().x, top + RH / 2.0), Align2::CENTER_CENTER, tr(lang, "prof_none"), 12.0, W::R, MUTED);
        }
        if n > ROWS && ui.rect_contains_pointer(r) {
            let dy = ui.input(|i| i.raw_scroll_delta.y);
            if dy != 0.0 {
                self.prof_scroll = if dy < 0.0 { self.prof_scroll + 1 } else { self.prof_scroll.saturating_sub(1) };
            }
        }
        self.prof_scroll = self.prof_scroll.min(n.saturating_sub(ROWS));
        let first = self.prof_scroll;
        if n > ROWS {
            let track = Rect::from_min_size(pos2(r.max.x - 5.0, top), vec2(2.0, ROWS as f32 * RH));
            let th = track.height() * ROWS as f32 / n as f32;
            let ty = track.min.y + (track.height() - th) * first as f32 / (n - ROWS) as f32;
            p.rect_filled(Rect::from_min_size(pos2(track.min.x, ty), vec2(2.0, th)), 1.0, LINE2);
        }
        for i in first..n.min(first + ROWS) {
            let ir = Rect::from_min_size(pos2(r.min.x + 6.0, top + (i - first) as f32 * RH), vec2(r.width() - 12.0 - if n > ROWS { 4.0 } else { 0.0 }, RH));
            let active = s.active_profile == Some(i);
            if active {
                p.rect_filled(ir, 8.0, rgb(0x2A2345));
                p.rect_filled(Rect::from_min_size(ir.min + vec2(0.0, 8.0), vec2(3.0, RH - 16.0)), 1.5, VIOLET);
            }
            // renommage en ligne
            if let Some((ri, buf)) = self.prof_rename.as_mut().filter(|(ri, _)| *ri == i) {
                let id = Id::new("prof_edit");
                let fr = Rect::from_min_max(ir.min + vec2(4.0, 3.0), ir.max - vec2(4.0, 3.0));
                p.rect(fr, 8.0, SURFACE2, sk(1.0, VIOLET));
                if buf.chars().count() > config::MAX_PROFILE_NAME {
                    *buf = buf.chars().take(config::MAX_PROFILE_NAME).collect();
                }
                let te = TextEdit::singleline(buf).id(id).frame(false).font(font(13.0, W::S)).margin(egui::Margin::ZERO).desired_width(fr.width() - 20.0);
                let resp = ui.put(Rect::from_center_size(fr.center(), vec2(fr.width() - 20.0, 20.0)), te);
                inside |= ui.rect_contains_pointer(fr);
                if !resp.has_focus() && !resp.lost_focus() {
                    resp.request_focus();
                }
                if resp.lost_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        acts.push(Act::ProfileRename(*ri, buf.clone()));
                    }
                    self.prof_rename = None;
                }
                continue;
            }
            let icons = if active { 3.0 } else { 2.0 };
            let nr = Rect::from_min_max(ir.min, pos2(ir.max.x - icons * 28.0 - 4.0, ir.max.y));
            let resp = hand(ui.interact(nr, Id::new("prof_row").with(i), Sense::click())).on_hover_text(tr(lang, "prof_load"));
            inside |= resp.hovered();
            if resp.hovered() && !active {
                p.rect_filled(ir, 8.0, SURFACE2);
            }
            let name = fit_text(&p, &s.profiles[i].name, 13.0, W::S, nr.width() - 24.0);
            text(&p, pos2(nr.min.x + 14.0, nr.center().y), Align2::LEFT_CENTER, &name, 13.0, W::S, if active { lighten(VIOLET, 0.45) } else { TEXT });
            if resp.clicked() {
                acts.push(Act::ProfileLoad(i));
            }
            let mut x = ir.max.x - 28.0;
            let mut btn = |ic: Ic, tag: &str, tip: &str, danger: bool| {
                let br = Rect::from_min_size(pos2(x, ir.min.y + 3.0), vec2(28.0, RH - 6.0));
                x -= 28.0;
                inside |= ui.rect_contains_pointer(br);
                mini_btn(ui, br, Id::new(tag).with(i), ic, tip, true, danger)
            };
            if btn(Ic::Trash, "prof_del", tr(lang, "prof_delete"), true) {
                self.confirm_profile = Some(i);
            }
            if btn(Ic::Edit, "prof_ren", tr(lang, "prof_rename"), false) {
                self.prof_rename = Some((i, s.profiles[i].name.clone()));
            }
            if active && btn(Ic::Save, "prof_ovw", tr(lang, "prof_overwrite"), false) {
                acts.push(Act::ProfileOverwrite(i));
            }
        }
        let btn_hover = ui.ctx().read_response(Id::new("profbtn")).map(|r| r.hovered()).unwrap_or(false);
        if ui.input(|i| i.pointer.any_pressed()) && !inside && !btn_hover && self.confirm_profile.is_none() {
            self.profiles_open = false;
            self.prof_rename = None;
        }
    }

    pub(super) fn confirm_modal(&mut self, ui: &mut Ui, full: Rect, s: &mut Settings, acts: &mut Vec<Act>) {
        let lang = s.lang;
        let p = ui.painter().clone();
        p.rect_filled(full, 0.0, Color32::from_black_alpha(150));
        ui.interact(full, Id::new("modal_block"), Sense::click());
        let r = Rect::from_center_size(full.center(), vec2(380.0, 160.0));
        p.rect(r, 18.0, SURFACE, sk(1.0, LINE2));
        icon(&p, if self.confirm_reset { Ic::Gear } else { Ic::Trash }, pos2(r.center().x, r.min.y + 36.0), 28.0, ROSE);
        let msg = match self.confirm_profile.and_then(|i| s.profiles.get(i)) {
            _ if self.confirm_reset => tr(lang, "reset_q").to_string(),
            _ if self.confirm_macro.is_some() => tr(lang, "macro_replace_q").to_string(),
            Some(pf) => fit_text(&p, &trf(lang, "prof_del_q", &[("name", pf.name.clone())]), 15.0, W::S, r.width() - 32.0),
            None => tr(lang, "clear_q").to_string(),
        };
        if self.confirm_reset {
            text(&p, pos2(r.center().x, r.min.y + 70.0), Align2::CENTER_CENTER, &msg, 15.0, W::S, TEXT);
            text(&p, pos2(r.center().x, r.min.y + 92.0), Align2::CENTER_CENTER, tr(lang, "reset_note"), 12.0, W::R, MUTED);
        } else {
            text(&p, pos2(r.center().x, r.min.y + 78.0), Align2::CENTER_CENTER, &msg, 15.0, W::S, TEXT);
        }
        let bw = 150.0;
        let y = r.max.y - 26.0 - 36.0 + 18.0;
        let cancel = Rect::from_min_size(pos2(r.center().x - bw - 6.0, y - 18.0), vec2(bw, 36.0));
        let del = Rect::from_min_size(pos2(r.center().x + 6.0, y - 18.0), vec2(bw, 36.0));
        let rc = hand(ui.interact(cancel, Id::new("m_cancel"), Sense::click()));
        let rd = hand(ui.interact(del, Id::new("m_del"), Sense::click()));
        p.rect(cancel, 10.0, SURFACE2, sk(1.0, if rc.hovered() { VIOLET } else { LINE2 }));
        text(&p, cancel.center(), Align2::CENTER_CENTER, tr(lang, "cancel"), 13.0, W::S, TEXT);
        p.rect_filled(del, 10.0, if rd.hovered() { lighten(ROSE, 0.12) } else { ROSE });
        text(&p, del.center(), Align2::CENTER_CENTER, tr(lang, if self.confirm_reset { "reset_yes" } else { "yes" }), 13.0, W::S, Color32::WHITE);
        if rc.clicked() {
            self.confirm_clear = false;
            self.confirm_reset = false;
            self.confirm_profile = None;
            self.confirm_macro = None;
        }
        if rd.clicked() {
            match self.confirm_profile.take() {
                _ if self.confirm_reset => acts.push(Act::ResetSettings),
                Some(i) => acts.push(Act::ProfileDelete(i)),
                None if self.confirm_macro.is_some() => {
                    s.pattern.steps = self.confirm_macro.take().unwrap_or_default();
                    acts.push(Act::StepsChanged);
                }
                None => {
                    s.pattern.steps.clear();
                    acts.push(Act::StepsChanged);
                }
            }
            self.confirm_clear = false;
            self.confirm_reset = false;
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
        if running && st.waiting.load(SeqCst) {
            msg = tr(lang, "arming").to_string();
            col = AMBER;
        } else if running && st.paused.load(SeqCst) {
            msg = tr(lang, "paused").to_string();
            col = AMBER;
        }
        if let Some((n, _)) = self.sh.macro_progress() {
            msg = trf(lang, "macro_msg", &[("n", n.to_string())]);
            col = ROSE;
        }
        let mut flash = self.sh.flash.lock().unwrap();
        if let Some((k, t)) = *flash {
            if t.elapsed() < Duration::from_secs(3) {
                msg = tr(lang, k).to_string();
                col = if matches!(k, "saved" | "cfg_exported" | "cfg_imported" | "cfg_reset") { GREEN } else { AMBER };
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
        if s.compact {
            // vue réduite : pas d'indication « arrière-plan », le message prend toute la largeur
            let msg = fit_text(p, &msg, 12.0, W::S, r.width() - 40.0 - 12.0);
            text(p, pos2(r.min.x + 40.0, r.center().y), Align2::LEFT_CENTER, &msg, 12.0, W::S, col);
            return;
        }
        text(p, pos2(r.min.x + 40.0, r.center().y), Align2::LEFT_CENTER, &msg, 12.0, W::S, col);
        text(p, pos2(r.max.x - 20.0, r.center().y), Align2::RIGHT_CENTER, tr(lang, "bg_hint"), 12.0, W::R, MUTED);
    }
}
