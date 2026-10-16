//! Page spam et carte raccourci.
use super::*;

impl App {
    // ---------------------------------------------------------------- carte raccourci
    pub(super) fn hotkey_hint(s: &Settings, m: Mode, rec: &Option<Recording>) -> String {
        let t = if m == Mode::Spam { Target::SpamHotkey } else { Target::PatHotkey };
        let keys = if m == Mode::Spam { &s.spam.hotkey } else { &s.pattern.hotkey };
        match rec {
            Some(r) if r.target == t => "…".into(),
            _ => combo_label(keys, s.lang),
        }
    }

    pub(super) fn rec_label(s: &Settings, rec: &Option<Recording>, t: Target, current: String) -> (String, bool) {
        match rec {
            Some(r) if r.target == t => (combo_label(&crate::keys::combo_sort(r.keys.clone()), s.lang).replace(tr(s.lang, "none"), ""), true),
            _ => (current, false),
        }
    }

    // ---------------------------------------------------------------- page spam
    pub(super) fn spam_page(&mut self, ui: &mut Ui, c: Rect, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>) {
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
        let t = card_title(&p, key_r, tr(lang, "key"));
        tip_after(ui, t, "spam_key", tr(lang, "tip_spam_key"));
        let cap = Rect::from_min_max(key_r.min + vec2(18.0, 44.0), key_r.max - vec2(18.0, 44.0));
        let (lbl, recording) = Self::rec_label(s, rec, Target::SpamKey, s.spam.key.label(lang));
        if keycap(ui, cap, Id::new("spamkey"), &lbl, recording, CapSize::Big, lang, false).on_hover_text(tr(lang, "click_change")).clicked() {
            acts.push(Act::Record(Target::SpamKey));
        }
        text(&p, pos2(key_r.center().x, key_r.max.y - 22.0), Align2::CENTER_CENTER, tr(lang, "click_change"), 12.0, W::R, MUTED);

        // intervalle
        card(&p, int_r);
        let t = card_title(&p, int_r, tr(lang, "interval"));
        tip_after(ui, t, "interval", tr(lang, "tip_interval"));
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
        // arrêt automatique
        let ly = int_r.min.y + 192.0;
        let t = text(&p, pos2(int_r.min.x + 18.0, ly), Align2::LEFT_CENTER, tr(lang, "limit"), 12.0, W::R, MUTED);
        tip_after(ui, t, "spam_limit", tr(lang, "tip_limit"));
        let seg_r = Rect::from_min_size(pos2(int_r.min.x + 18.0, ly + 12.0), vec2(250.0, 36.0));
        let mut kind = match s.spam.limit {
            SpamLimit::None => 0u8,
            SpamLimit::Count(_) => 1,
            SpamLimit::Duration(_) => 2,
        };
        let items = [(0u8, tr(lang, "limit_none").to_string()), (1, tr(lang, "limit_count").to_string()), (2, tr(lang, "limit_dur").to_string())];
        if segmented(ui, seg_r, Id::new("spamlimit"), &items, &mut kind, 13.0, false) {
            s.spam.limit = match kind {
                1 => SpamLimit::Count(100),
                2 => SpamLimit::Duration(60.0),
                _ => SpamLimit::None,
            };
        }
        let nr = Rect::from_min_size(pos2(seg_r.max.x + 10.0, seg_r.min.y), vec2(110.0, 36.0));
        match &mut s.spam.limit {
            SpamLimit::None => {}
            SpamLimit::Count(n) => {
                let mut v = *n as f64;
                num_field(ui, nr, Id::new("limitcount"), &mut v, 1.0, config::MAX_LIMIT_COUNT as f64, true, 16.0, true, lang);
                *n = v as u64;
            }
            SpamLimit::Duration(d) => {
                num_field(ui, nr, Id::new("limitdur"), d, config::MIN_LIMIT_S, config::MAX_LIMIT_S, false, 16.0, true, lang);
                text(&p, pos2(nr.max.x + 8.0, nr.center().y), Align2::LEFT_CENTER, "s", 12.0, W::S, MUTED);
            }
        }
        let iv = config::secs(s.spam.value, s.spam.unit).max(config::MIN_INTERVAL_S);
        rhythm(&p, Rect::from_min_size(pos2(int_r.min.x + 18.0, int_r.max.y - 84.0), vec2(iw - 36.0, 46.0)), iv, lang);
        let rate = if iv <= 1.0 {
            trf(lang, "rate_s", &[("n", fmt_num(lang, (10.0 / iv).round() / 10.0))])
        } else if iv <= 60.0 {
            trf(lang, "rate_m", &[("n", fmt_num(lang, (600.0 / iv).round() / 10.0))])
        } else if iv <= 36000.0 {
            trf(lang, "rate_h", &[("n", fmt_num(lang, (36000.0 / iv).round() / 10.0))])
        } else if iv <= 864_000.0 {
            trf(lang, "rate_d", &[("n", fmt_num(lang, (864_000.0 / iv).round() / 10.0))])
        } else {
            tr(lang, "rate_lt").to_string()
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

    pub(super) fn hotkey_card(&mut self, ui: &mut Ui, r: Rect, s: &mut Settings, rec: &Option<Recording>, acts: &mut Vec<Act>, m: Mode) {
        let lang = s.lang;
        let p = ui.painter().clone();
        card(&p, r);
        let t = card_title(&p, r, tr(lang, "hotkey"));
        tip_after(ui, t, "hotkey", tr(lang, "tip_hotkey"));
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
}
