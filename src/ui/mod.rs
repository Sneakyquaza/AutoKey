//! Interface : tout est dessiné à la main (egui) pour un rendu sur mesure.
use crate::config::{self, HkMode, Mode, PatternFile, Settings, SpamLimit, Step, Unit};
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

mod chrome;
mod pattern;
mod spam;
mod theme;
mod widgets;
use theme::*;
pub use theme::{height_for, setup_fonts, width_for, BG};
use widgets::*;

// ------------------------------------------------------------------ application
enum Act {
    Record(Target),
    MacroToggle,
    Toggle(Mode),
    /// Nouvelle taille de fenêtre (mode ou vue réduite changés)
    Resize(Mode, bool),
    SetLang,
    Pin(bool),
    Minimize,
    Close,
    StepsChanged,
    SavePreset,
    OpenPreset,
    ProfileNew,
    ProfileOverwrite(usize),
    ProfileLoad(usize),
    ProfileRename(usize, String),
    ProfileDelete(usize),
}

pub struct App {
    sh: Arc<Shared>,
    up: Arc<crate::update::Updater>,
    lang_open: bool,
    settings_open: bool,
    confirm_clear: bool,
    /// Macro capturée en attente de confirmation de remplacement
    confirm_macro: Option<Vec<Step>>,
    profiles_open: bool,
    /// Profil en cours de renommage (index, saisie)
    prof_rename: Option<(usize, String)>,
    /// Profil dont la suppression attend confirmation
    confirm_profile: Option<usize>,
    /// Premier profil visible dans la liste (défilement)
    prof_scroll: usize,
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
            confirm_macro: None,
            profiles_open: false,
            prof_rename: None,
            confirm_profile: None,
            prof_scroll: 0,
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

    #[cfg(feature = "shots")]
    pub fn open_profiles_for_shots(&mut self, on: bool) {
        self.profiles_open = on;
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
            Act::MacroToggle => {
                ctx.memory_mut(|m| {
                    if let Some(id) = m.focused() {
                        m.surrender_focus(id)
                    }
                });
                self.sh.toggle_macro();
            }
            Act::Toggle(m) => self.sh.toggle(m),
            Act::Resize(m, compact) => {
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(width_for(compact), height_for(m, compact))));
            }
            Act::SetLang => {}
            Act::Pin(on) => ctx.send_viewport_cmd(ViewportCommand::WindowLevel(if on { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal })),
            // avec l'icône de notification, réduire / fermer cache la fenêtre (la tâche continue)
            Act::Minimize if self.tray_enabled() => ctx.send_viewport_cmd(ViewportCommand::Visible(false)),
            Act::Close if self.tray_enabled() => ctx.send_viewport_cmd(ViewportCommand::Visible(false)),
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
            Act::ProfileNew => {
                let n = self.sh.settings.lock().unwrap().profiles.len() + 1;
                let lang = self.sh.settings.lock().unwrap().lang;
                self.sh.save_profile(None, trf(lang, "prof_default", &[("n", n.to_string())]));
            }
            Act::ProfileOverwrite(i) => self.sh.save_profile(Some(i), String::new()),
            Act::ProfileLoad(i) => self.sh.load_profile(i),
            Act::ProfileRename(i, name) => self.sh.rename_profile(i, &name),
            Act::ProfileDelete(i) => self.sh.delete_profile(i),
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

    /// Icône de notification active : fermer cache la fenêtre au lieu de quitter
    pub fn tray_enabled(&self) -> bool {
        // seulement si l'icône est réellement affichée, sinon la fenêtre deviendrait inaccessible
        self.sh.settings.lock().unwrap().tray && crate::platform::tray_active()
    }

    /// Commande du menu de l'icône de notification
    pub fn tray_toggle(&self, m: Mode) {
        self.sh.toggle(m);
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        self.frames += 1;
        let rec = self.sh.recording();
        // macro terminée (bouton ou raccourci d'arrêt) : remplacement direct ou après confirmation
        if let Some(steps) = self.sh.take_macro_out() {
            if self.sh.settings.lock().unwrap().pattern.steps.is_empty() {
                self.sh.settings.lock().unwrap().pattern.steps = steps;
                self.sh.mark_dirty();
            } else {
                self.confirm_macro = Some(steps);
            }
        }
        let macro_on = self.sh.macro_active();
        if macro_on {
            // compteur et pulsation du bouton
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if rec.is_some() || macro_on {
            // pendant l'écoute, le clavier ne doit rien activer dans la fenêtre
            ctx.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Key { .. } | egui::Event::Text(_))));
        }
        let orig = self.sh.settings.lock().unwrap().clone();
        crate::platform::tray_sync(orig.tray, orig.lang, self.sh.is_running(Mode::Spam), self.sh.is_running(Mode::Pattern));
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

        if s.compact {
            self.titlebar_compact(ui, bar, s, acts);
            self.compact_page(ui, content, s, rec, acts);
        } else {
            self.titlebar(ui, bar, s, acts);
            match s.mode {
                Mode::Spam => self.spam_page(ui, content, s, rec, acts),
                Mode::Pattern => self.pattern_page(ui, content, s, rec, acts),
            }
        }
        self.footer(ui, foot, s);
        ui.painter().rect_stroke(full.shrink(0.5), 0.0, sk(1.0, LINE));

        if self.lang_open || self.settings_open || self.profiles_open || self.confirm_clear || self.confirm_profile.is_some() || self.confirm_macro.is_some() {
            let ctx = ui.ctx().clone();
            egui::Area::new(Id::new("overlay")).order(egui::Order::Foreground).fixed_pos(full.min).show(&ctx, |ui| {
                ui.set_min_size(full.size());
                if self.lang_open {
                    self.lang_popup(ui, full, s, acts);
                }
                if self.settings_open {
                    self.settings_popup(ui, full, s, rec, acts);
                }
                if self.profiles_open {
                    self.profiles_popup(ui, full, s, acts);
                }
                if self.confirm_clear || self.confirm_profile.is_some() || self.confirm_macro.is_some() {
                    self.confirm_modal(ui, full, s, acts);
                }
            });
        }
        let _ = lang;
    }
}
