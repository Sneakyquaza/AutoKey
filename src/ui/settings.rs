//! Panneau Paramètres : modal plein-fenêtre, sections, défilement vertical, info-bulles « i ».
use super::*;
use crate::platform::{self, AutoStart};
use crate::update::{Check, UpState};

/// Hauteur d'une ligne d'option
const ROW_H: f32 = 46.0;

/// Libellé tronqué à `maxw` + « i » d'aide juste après
fn label(ui: &mut Ui, x0: f32, ym: f32, maxw: f32, lbl: &str, col: Color32, key: &str, tip: &str) {
    let p = ui.painter().clone();
    let l = fit_text(&p, lbl, 13.0, W::S, maxw);
    let r = text(&p, pos2(x0, ym), Align2::LEFT_CENTER, &l, 13.0, W::S, col);
    info_tip(ui, pos2(r.max.x + 14.0, ym), Id::new("tip").with(key), tip);
}

/// Titre de section + filet ; avance `y`
fn section(p: &Painter, x0: f32, x1: f32, y: &mut f32, title: &str) {
    spaced_text(p, pos2(x0, *y + 8.0), title, 11.0, MUTED);
    p.line_segment([pos2(x0, *y + 30.0), pos2(x1, *y + 30.0)], sk(1.0, LINE));
    *y += 38.0;
}

/// Interrupteur Non / Oui à droite de la ligne ; grisé si `enabled` est faux
fn toggle(ui: &mut Ui, x1: f32, ym: f32, id: &str, v: &mut bool, enabled: bool, lang: Lang) -> bool {
    let r = Rect::from_center_size(pos2(x1 - 45.0, ym), vec2(90.0, 32.0));
    if !enabled {
        toggle_off(ui.painter(), r, *v, [tr(lang, "off"), tr(lang, "on")], 13.0);
        return false;
    }
    segmented(ui, r, Id::new(id), &[(false, tr(lang, "off").into()), (true, tr(lang, "on").into())], v, 13.0, false)
}

/// Bouton d'action à droite de la ligne
fn action(ui: &mut Ui, x1: f32, ym: f32, id: &str, lbl: &str, w: f32, danger: bool, enabled: bool) -> bool {
    pill_btn(ui, Rect::from_center_size(pos2(x1 - w / 2.0, ym), vec2(w, 32.0)), Id::new(id), lbl, danger, enabled)
}

impl App {
    pub(super) fn settings_panel(&mut self, ui: &mut Ui, full: Rect, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>) {
        let lang = s.lang;
        let p = ui.painter().clone();
        p.rect_filled(full, 0.0, Color32::from_black_alpha(150));
        let back = ui.interact(full, Id::new("set_block"), Sense::click());
        let r = if s.compact { full.shrink(6.0) } else { Rect::from_center_size(full.center(), vec2(720.0f32.min(full.width() - 40.0), full.height() - 24.0)) };
        p.rect_filled(r.translate(vec2(0.0, 6.0)).expand(2.0), 18.0, Color32::from_black_alpha(90));
        p.rect(r, 16.0, SURFACE, sk(1.0, LINE2));
        // en-tête : titre + fermer
        let head = Rect::from_min_size(r.min, vec2(r.width(), 48.0));
        icon(&p, Ic::Gear, pos2(r.min.x + 28.0, head.center().y), 18.0, VIOLET);
        text(&p, pos2(r.min.x + 46.0, head.center().y), Align2::LEFT_CENTER, tr(lang, "settings"), 15.0, W::B, TEXT);
        let cb = Rect::from_min_size(pos2(r.max.x - 10.0 - 34.0, head.center().y - 17.0), vec2(34.0, 34.0));
        if icon_btn(ui, cb, "set_close", Ic::Close, tr(lang, "close"), false, false).clicked() {
            self.settings_open = false;
        }
        p.line_segment([pos2(r.min.x + 1.0, head.max.y), pos2(r.max.x - 1.0, head.max.y)], sk(1.0, LINE));
        // corps défilant
        let view = Rect::from_min_max(pos2(r.min.x + 4.0, head.max.y + 2.0), pos2(r.max.x - 4.0, r.max.y - 10.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(view));
        child.set_clip_rect(view);
        child.style_mut().spacing.scroll = egui::style::ScrollStyle::floating();
        let mut sa = egui::ScrollArea::vertical().id_salt("settings_scroll").auto_shrink([false, false]);
        if let Some(o) = self.settings_scroll.take() {
            sa = sa.vertical_scroll_offset(o);
        }
        sa.show(&mut child, |ui| {
            // le contenu est dessiné dans un enfant (ses widgets n'agrandissent pas la zone) ; la hauteur totale est imposée ensuite
            let (o, w) = (ui.max_rect().min, ui.max_rect().width());
            let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(o, vec2(w, 10_000.0))));
            let h = self.settings_body(&mut inner, s, rec, acts);
            ui.set_min_size(vec2(w, h));
        });
        // clic en dehors du panneau : fermeture
        if back.clicked() && back.interact_pointer_pos().is_some_and(|pt| !r.contains(pt)) {
            self.settings_open = false;
        }
    }

    fn settings_body(&mut self, ui: &mut Ui, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>) -> f32 {
        let lang = s.lang;
        let o = ui.max_rect().min;
        let w = ui.max_rect().width();
        let (x0, x1) = (o.x + 16.0, o.x + w - 22.0);
        let p = ui.painter().clone();
        let mut y = o.y + 2.0;
        let lw = |ctl: f32| x1 - x0 - ctl - 36.0; // largeur max du libellé selon le contrôle à droite
        let sys_ok = platform::autostart_supported();

        // ------------------------------------------------ exécution
        section(&p, x0, x1, &mut y, tr(lang, "sec_exec"));
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(130.0), tr(lang, "press_dur"), TEXT, "press", tr(lang, "tip_press"));
        let nr = Rect::from_center_size(pos2(x1 - 30.0 - 40.0, ym), vec2(80.0, 32.0));
        num_field(ui, nr, Id::new("pressms"), &mut s.press_ms, 0.0, config::MAX_PRESS_MS, true, 16.0, true, lang);
        text(&p, pos2(x1, ym), Align2::RIGHT_CENTER, "ms", 12.0, W::S, MUTED);
        y += ROW_H;
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(130.0), tr(lang, "jitter"), TEXT, "jitter", tr(lang, "tip_jitter"));
        let jr = Rect::from_center_size(pos2(x1 - 30.0 - 40.0, ym), vec2(80.0, 32.0));
        num_field(ui, jr, Id::new("jitterpct"), &mut s.jitter_pct, 0.0, config::MAX_JITTER_PCT, true, 16.0, true, lang);
        text(&p, pos2(x1, ym), Align2::RIGHT_CENTER, "%", 12.0, W::S, MUTED);
        y += ROW_H;
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(90.0), tr(lang, "bind_win"), TEXT, "bind", tr(lang, "tip_bind"));
        toggle(ui, x1, ym, "bindwin", &mut s.bind_window, true, lang);
        y += ROW_H;
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(120.0), tr(lang, "stop_hk"), TEXT, "stop", tr(lang, "tip_stop"));
        let (lbl, recording) = Self::rec_label(s, rec, Target::StopHotkey, combo_label(&s.stop_hotkey, lang));
        let cr = Rect::from_center_size(pos2(x1 - 60.0, ym), vec2(120.0, 32.0));
        if keycap(ui, cr, Id::new("stophk"), &lbl, recording, CapSize::Mid, lang, true).on_hover_text(tr(lang, "click_change")).clicked() {
            acts.push(Act::Record(Target::StopHotkey));
        }
        y += ROW_H;

        // ------------------------------------------------ interface
        y += 6.0;
        section(&p, x0, x1, &mut y, tr(lang, "sec_ui"));
        let ym = y + ROW_H / 2.0;
        let (lgw, short) = if s.compact { (150.0, true) } else { (270.0, false) };
        label(ui, x0, ym, lw(lgw), tr(lang, "language"), TEXT, "lang", tr(lang, "tip_lang"));
        let items: Vec<(Lang, String)> = Lang::ALL.iter().map(|l| (*l, if short { l.code() } else { l.name() }.to_string())).collect();
        let mut cur = s.lang;
        if segmented(ui, Rect::from_center_size(pos2(x1 - lgw / 2.0, ym), vec2(lgw, 32.0)), Id::new("setlang"), &items, &mut cur, 12.0, false) {
            s.lang = cur;
            acts.push(Act::SetLang);
        }
        y += ROW_H;
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(90.0), tr(lang, "pin"), TEXT, "pin", tr(lang, "tip_pin"));
        if toggle(ui, x1, ym, "setpin", &mut s.pin, true, lang) {
            acts.push(Act::Pin(s.pin));
        }
        y += ROW_H;
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(90.0), tr(lang, "compact"), TEXT, "compact", tr(lang, "tip_compact"));
        if toggle(ui, x1, ym, "setcompact", &mut s.compact, true, lang) {
            acts.push(Act::Resize(s.mode, s.compact));
        }
        y += ROW_H;
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(90.0), tr(lang, "beep"), TEXT, "beep", tr(lang, "tip_beep"));
        toggle(ui, x1, ym, "beep", &mut s.beep, true, lang);
        y += ROW_H;

        // ------------------------------------------------ système
        y += 6.0;
        section(&p, x0, x1, &mut y, tr(lang, "sec_sys"));
        let col = if sys_ok { TEXT } else { DIM };
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(90.0), tr(lang, "tray"), col, "tray", tr(lang, "tip_tray"));
        toggle(ui, x1, ym, "tray", &mut s.tray, sys_ok, lang);
        y += ROW_H;
        let ym = y + ROW_H / 2.0;
        let sm_ok = sys_ok && s.tray;
        label(ui, x0, ym, lw(90.0), tr(lang, "start_min"), if sm_ok { TEXT } else { DIM }, "startmin", tr(lang, "tip_startmin"));
        toggle(ui, x1, ym, "startmin", &mut s.start_minimized, sm_ok, lang);
        y += ROW_H;
        let st = if sys_ok { *self.autostart.get_or_insert_with(platform::autostart_state) } else { AutoStart::Off };
        let rh = if st == AutoStart::OtherPath { ROW_H + 30.0 } else { ROW_H };
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(90.0), tr(lang, "autostart"), col, "autostart", tr(lang, "tip_autostart"));
        let mut on = st != AutoStart::Off;
        if toggle(ui, x1, ym, "autostart", &mut on, sys_ok, lang) {
            acts.push(Act::Autostart(on));
        }
        if st == AutoStart::OtherPath {
            let fr = Rect::from_min_size(pos2(x0, y + ROW_H - 6.0), vec2(200.0f32.min(x1 - x0), 26.0));
            if pill_btn(ui, fr, Id::new("autostart_fix"), tr(lang, "autostart_fix"), false, true) {
                acts.push(Act::Autostart(true));
            }
        }
        y += rh;

        // ------------------------------------------------ mises à jour
        y += 6.0;
        section(&p, x0, x1, &mut y, tr(lang, "sec_upd"));
        let ym = y + ROW_H / 2.0;
        let build = crate::update::current_build();
        let ver = if build > 0 { format!("v{} · build {build}", env!("CARGO_PKG_VERSION")) } else { format!("v{}", env!("CARGO_PKG_VERSION")) };
        let vw = text_width(&p, &ver, 13.0, W::S);
        label(ui, x0, ym, lw(vw), tr(lang, "version"), TEXT, "version", tr(lang, "tip_version"));
        text(&p, pos2(x1, ym), Align2::RIGHT_CENTER, &ver, 13.0, W::S, MUTED);
        y += ROW_H;
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(90.0), tr(lang, "auto_upd"), TEXT, "autoupd", tr(lang, "tip_autoupd"));
        toggle(ui, x1, ym, "autoupd", &mut s.auto_check_updates, true, lang);
        y += ROW_H;
        let ym = y + 23.0;
        label(ui, x0, ym, lw(170.0), tr(lang, "check_now"), TEXT, "check", tr(lang, "tip_check"));
        let busy = *self.up.check.lock().unwrap() == Check::Busy;
        if action(ui, x1, ym, "checknow", tr(lang, "check_now"), 170.0, false, !busy) {
            acts.push(Act::CheckUpdates);
        }
        let status = match (*self.up.check.lock().unwrap(), self.up.state()) {
            (_, UpState::Available { build, .. }) => Some((trf(lang, "chk_avail", &[("b", build.to_string())]), AMBER)),
            (Check::Busy, _) => Some((tr(lang, "chk_busy").to_string(), CYAN)),
            (Check::UpToDate, _) => Some((tr(lang, "chk_ok").to_string(), GREEN)),
            (Check::Failed("up_na"), _) => Some((tr(lang, "chk_na").to_string(), MUTED)),
            (Check::Failed(_), _) => Some((tr(lang, "chk_err").to_string(), AMBER)),
            (Check::Idle, _) => None,
        };
        if let Some((msg, c)) = status {
            wrapped(&p, Rect::from_min_size(pos2(x0, y + 42.0), vec2(x1 - x0, 30.0)), &msg, 12.0, W::R, c, false);
        }
        y += 76.0;

        // ------------------------------------------------ données
        y += 6.0;
        section(&p, x0, x1, &mut y, tr(lang, "sec_data"));
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(130.0), tr(lang, "d_folder"), TEXT, "folder", tr(lang, "tip_folder"));
        if action(ui, x1, ym, "d_open", tr(lang, "d_open"), 130.0, false, true) {
            acts.push(Act::OpenConfigDir);
        }
        y += ROW_H;
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(130.0), tr(lang, "d_export"), TEXT, "export", tr(lang, "tip_export"));
        if action(ui, x1, ym, "d_export", tr(lang, "d_exp_btn"), 130.0, false, true) {
            acts.push(Act::ExportConfig);
        }
        y += ROW_H;
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(130.0), tr(lang, "d_import"), TEXT, "import", tr(lang, "tip_import"));
        if action(ui, x1, ym, "d_import", tr(lang, "d_imp_btn"), 130.0, false, true) {
            acts.push(Act::ImportConfig);
        }
        y += ROW_H;
        let ym = y + ROW_H / 2.0;
        label(ui, x0, ym, lw(130.0), tr(lang, "d_reset"), TEXT, "reset", tr(lang, "tip_reset"));
        if action(ui, x1, ym, "d_reset", tr(lang, "d_reset_btn"), 130.0, true, true) {
            self.confirm = Some(Confirm::Reset);
        }
        y += ROW_H + 8.0;

        y - o.y
    }
}
