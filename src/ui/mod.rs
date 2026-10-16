//! Interface : tout est dessiné à la main (egui) pour un rendu sur mesure.
use crate::config::{self, HkMode, Mode, PatternFile, Settings, SpamLimit, Step, Unit};
use crate::control::{Recording, Shared, Target};
use crate::i18n::{fmt_duration, fmt_num, tr, trf, Lang};
use crate::keys::combo_label;
use crate::platform;
use egui::{
    self, epaint, pos2, vec2, Align, Align2, Color32, CursorIcon, FontFamily, FontId, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, TextEdit, Ui,
    Vec2, ViewportCommand,
};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod chrome;
mod pattern;
mod settings;
mod spam;
mod theme;
mod widgets;
use theme::*;
pub use theme::{height_for, setup_fonts, width_for, BG};
use widgets::*;

// ------------------------------------------------------------------ application
/// Touche Windows (gauche / droite)
fn is_win(k: &crate::keys::KeySpec) -> bool {
    matches!(k.vk, 0x5B | 0x5C)
}

/// Étape qui utilise une touche Windows
fn sensitive_steps(steps: &[Step]) -> bool {
    steps.iter().any(|st| matches!(st, Step::Hold { key, .. } | Step::Press { key, .. } if is_win(key)))
}

/// Raccourci contenant Win, ou combinaison Ctrl + Alt
fn sensitive_combo(keys: &[crate::keys::KeySpec]) -> bool {
    let has = |vks: &[u16]| keys.iter().any(|k| vks.contains(&k.vk));
    keys.iter().any(is_win) || (has(&[0x11, 0xA2, 0xA3]) && has(&[0x12, 0xA4, 0xA5]))
}

/// Réglages importés à confirmer : touche Win ou Ctrl+Alt dans un raccourci, Win dans le spam ou le pattern
fn sensitive_settings(s: &Settings) -> bool {
    sensitive_combo(&s.spam.hotkey)
        || sensitive_combo(&s.pattern.hotkey)
        || sensitive_combo(&s.stop_hotkey)
        || is_win(&s.spam.key)
        || sensitive_steps(&s.pattern.steps)
        || s.profiles.iter().any(|p| is_win(&p.spam.key) || sensitive_steps(&p.pattern.steps))
}

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
    OpenConfigDir,
    ExportConfig,
    ImportConfig,
    /// Réinitialise les réglages (profils conservés), après confirmation
    ResetSettings,
    Autostart(bool),
    CheckUpdates,
    /// Import de configuration confirmé
    ApplyImport(Box<Settings>),
    /// Pattern importé confirmé
    ApplyPreset(config::PatternCfg),
}

/// Action destructive ou sensible en attente d'un « Oui »
enum Confirm {
    Clear,
    Reset,
    /// Remplacer le pattern par l'exemple
    Example,
    Profile(usize),
    /// Macro capturée à la place du pattern actuel
    Macro(Vec<Step>),
    /// Configuration importée contenant des touches sensibles
    ImportConfig(Box<Settings>),
    /// Pattern importé contenant des touches sensibles
    ImportPreset(config::PatternCfg),
}

pub struct App {
    sh: Arc<Shared>,
    up: Arc<crate::update::Updater>,
    lang_open: bool,
    settings_open: bool,
    /// Confirmation en attente (texte et action du « Oui » viennent de la même valeur)
    confirm: Option<Confirm>,
    /// Décalage de défilement imposé au panneau Paramètres (captures)
    settings_scroll: Option<f32>,
    profiles_open: bool,
    /// Fenêtre de mise à jour (versions + nouveautés) ouverte
    update_dialog: bool,
    /// Profil en cours de renommage (index, saisie)
    prof_rename: Option<(usize, String)>,
    /// Dernier nombre de boucles fini (restauré en quittant le mode infini)
    loops_prev: u32,
    /// État du démarrage automatique : lu à l'ouverture des Réglages et après un changement (None = à relire)
    autostart: Option<platform::AutoStart>,
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
            confirm: None,
            settings_scroll: None,
            profiles_open: false,
            update_dialog: false,
            prof_rename: None,
            loops_prev: 1,
            autostart: None,
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
        self.confirm = on.then_some(Confirm::Clear);
    }

    #[cfg(feature = "shots")]
    pub fn open_update_for_shots(&mut self, on: bool) {
        self.update_dialog = on;
    }

    #[cfg(feature = "shots")]
    pub fn open_settings_for_shots(&mut self, on: bool) {
        self.settings_open = on;
    }

    #[cfg(feature = "shots")]
    pub fn scroll_settings_for_shots(&mut self, offset: f32) {
        self.settings_scroll = Some(offset);
    }

    /// Force l'info-bulle `tip.<clé>` ouverte (captures)
    #[cfg(feature = "shots")]
    pub fn force_tip_for_shots(&self, ctx: &egui::Context, key: Option<&str>) {
        ctx.data_mut(|d| match key {
            Some(k) => d.insert_temp(Id::new("force_tip"), Id::new("tip").with(k)),
            None => d.remove::<Id>(Id::new("force_tip")),
        });
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
            Act::OpenConfigDir => platform::open_folder(&config::config_dir()),
            Act::ExportConfig => {
                let picked = platform::pick_save_as("autokey-config.json");
                platform::trim_memory();
                if let Some(path) = picked {
                    let s = self.sh.settings.lock().unwrap().clone();
                    let ok = serde_json::to_string_pretty(&s).is_ok_and(|txt| std::fs::write(path, txt).is_ok());
                    self.flash(if ok { "cfg_exported" } else { "io_err" });
                }
            }
            Act::ImportConfig => {
                let picked = platform::pick_open();
                platform::trim_memory();
                if let Some(path) = picked {
                    // lecture bornée (≤ 1 Mo), analyse + bornes + conflits de raccourcis avant de remplacer quoi que ce soit
                    match config::read_small(&path).ok_or("bad_file").and_then(|t| Shared::parse_import(&t)) {
                        Ok(new) if sensitive_settings(&new) => self.confirm = Some(Confirm::ImportConfig(Box::new(new))),
                        Ok(new) => self.apply(ctx, Act::ApplyImport(Box::new(new))),
                        Err(k) => self.flash(k),
                    }
                }
            }
            Act::ApplyImport(new) => {
                self.apply_new_settings(ctx, *new);
                self.flash("cfg_imported");
            }
            Act::ApplyPreset(pat) => self.set_pattern(pat),
            Act::ResetSettings => {
                let new = config::reset_keep_profiles(&self.sh.settings.lock().unwrap());
                self.apply_new_settings(ctx, new);
                self.flash("cfg_reset");
            }
            Act::Autostart(on) => {
                if !platform::autostart_set(on) {
                    self.flash("autostart_err");
                }
                self.autostart = None;
            }
            Act::CheckUpdates => self.up.check_now(),
            Act::OpenPreset => {
                let picked = crate::platform::pick_open();
                crate::platform::trim_memory();
                if let Some(path) = picked {
                    let parsed = config::read_pattern_file(&path);
                    match parsed {
                        // une étape ne peut pas utiliser une touche de raccourci (cf. `Shared::conflicts`)
                        Some(f)
                            if {
                                let s = self.sh.settings.lock().unwrap();
                                f.pattern.steps.iter().any(|st| match st {
                                    Step::Hold { key, .. } | Step::Press { key, .. } => {
                                        crate::control::Shared::conflicts(&s, crate::control::Target::SpamKey, std::slice::from_ref(key))
                                    }
                                    Step::Wait { .. } => false,
                                })
                            } =>
                        {
                            *self.sh.flash.lock().unwrap() = Some(("hk_conflict", Instant::now()))
                        }
                        Some(f) if sensitive_steps(&f.pattern.steps) => self.confirm = Some(Confirm::ImportPreset(f.pattern)),
                        Some(f) => self.set_pattern(f.pattern),
                        None => *self.sh.flash.lock().unwrap() = Some(("bad_file", Instant::now())),
                    }
                }
            }
        }
    }
}

impl App {
    /// Échap ferme, du plus haut au plus bas : confirmation (= Annuler), fenêtre de MAJ (= Plus tard), popups, Réglages.
    /// Ignoré pendant une capture de touche (les touches y sont des données) ; un champ de saisie actif garde son Échap.
    fn escape_close(&mut self, ctx: &egui::Context, capturing: bool) {
        if capturing {
            return;
        }
        let modal = self.confirm.is_some() || self.update_dialog;
        if !modal && ctx.memory(|m| m.focused().is_some()) {
            return;
        }
        if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            return;
        }
        if self.confirm.is_some() {
            self.confirm = None;
        } else if self.update_dialog {
            self.update_dialog = false;
        } else if self.profiles_open {
            self.profiles_open = false;
            self.prof_rename = None;
        } else if self.lang_open {
            self.lang_open = false;
        } else if self.settings_open {
            self.settings_open = false;
        }
    }

    fn flash(&self, k: &'static str) {
        *self.sh.flash.lock().unwrap() = Some((k, Instant::now()));
    }

    /// Remplace le pattern (preset importé) : tâche et capture en cours arrêtées
    fn set_pattern(&mut self, pattern: config::PatternCfg) {
        self.sh.runner.stop();
        self.sh.cancel_record();
        self.sh.settings.lock().unwrap().pattern = pattern;
        self.sh.mark_dirty();
    }

    /// Remplace tous les réglages (import, réinitialisation) et répercute pin / taille sur la fenêtre
    fn apply_new_settings(&mut self, ctx: &egui::Context, new: Settings) {
        let (pin, mode, compact) = (new.pin, new.mode, new.compact);
        self.sh.replace_settings(new);
        self.apply(ctx, Act::Pin(pin));
        self.apply(ctx, Act::Resize(mode, compact));
    }

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
        // macro terminée (bouton ou raccourci d'arrêt) : remplacement direct ou après confirmation.
        // Une confirmation déjà ouverte n'est jamais écrasée : la macro reste dans `Shared` jusqu'à sa résolution.
        if self.confirm.is_none() {
            if let Some(steps) = self.sh.take_macro_out() {
                if self.sh.settings.lock().unwrap().pattern.steps.is_empty() {
                    self.sh.settings.lock().unwrap().pattern.steps = steps;
                    self.sh.mark_dirty();
                } else {
                    self.confirm = Some(Confirm::Macro(steps));
                }
            }
        }
        if !self.settings_open {
            self.autostart = None;
        }
        self.escape_close(ctx, rec.is_some() || self.sh.macro_active());
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

    /// Mémorise la position de la fenêtre (sans écriture disque : sauvée à la sortie ou au masquage)
    pub fn set_window_pos(&self, pos: (i32, i32)) {
        let mut s = self.sh.settings.lock().unwrap();
        if s.window_pos != Some(pos) {
            s.window_pos = Some(pos);
            drop(s);
            self.sh.mark_dirty(); // sauvegarde débouncée : survit à un arrêt brutal
        }
    }

    pub fn window_pos(&self) -> Option<(i32, i32)> {
        self.sh.settings.lock().unwrap().window_pos
    }

    /// Fenêtre cachée dans la zone de notification : sauvegarde (débouncée) de la position
    pub fn mark_dirty(&self) {
        self.sh.mark_dirty();
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

        if self.lang_open || self.settings_open || self.profiles_open || self.confirm.is_some() || self.update_dialog {
            let ctx = ui.ctx().clone();
            egui::Area::new(Id::new("overlay")).order(egui::Order::Foreground).fixed_pos(full.min).show(&ctx, |ui| {
                ui.set_min_size(full.size());
                if self.lang_open {
                    self.lang_popup(ui, full, s, acts);
                }
                if self.settings_open {
                    self.settings_panel(ui, full, s, rec, acts);
                }
                if self.profiles_open {
                    self.profiles_popup(ui, full, s, acts);
                }
                if self.confirm.is_some() {
                    self.confirm_modal(ui, full, s, acts);
                } else if self.update_dialog {
                    self.update_modal(ui, full, s);
                }
            });
        }
        let _ = lang;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tests::Log;

    /// Dessine `frames` images de l'application sur un contexte egui sans fenêtre
    fn draw_frames(app: &mut App, ctx: &egui::Context, size: Vec2, frames: usize) {
        for _ in 0..frames {
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)), ..Default::default() };
            let _ = ctx.run(input, |ctx| app.update(ctx));
        }
    }

    #[test]
    fn sensitive_keys_detected() {
        use crate::keys::KeySpec;
        let k = KeySpec::vk;
        assert!(!sensitive_settings(&Settings::default()));
        assert!(sensitive_combo(&[k(0x5B), k(0x41)]));
        assert!(sensitive_combo(&[k(0x11), k(0x12), k(0x41)]));
        assert!(!sensitive_combo(&[k(0x11), k(0x41)]));
        let mut s = Settings::default();
        s.pattern.steps = vec![Step::Press { key: k(0x5C), count: 1, gap: 0.0 }];
        assert!(sensitive_settings(&s));
    }

    /// Fumée : chaque vue (spam, pattern, vue réduite, panneau Réglages, profils, confirmation)
    /// dans chaque langue se dessine sans panique. Ignoré si la police de l'interface est absente.
    #[test]
    fn every_view_draws_without_panic() {
        #[cfg(not(windows))]
        if !std::path::Path::new("/usr/share/fonts/opentype/inter/Inter-Regular.otf").exists() {
            eprintln!("police Inter absente : test de dessin ignoré");
            return;
        }
        let ctx = egui::Context::default();
        setup_fonts(&ctx);
        let sh = Shared::new(Settings::default(), Arc::new(Log::default()));
        let up = crate::update::Updater::new(sh.clone());
        let mut app = App::new(sh.clone(), up);
        for lang in [Lang::Fr, Lang::En, Lang::Pt] {
            for compact in [false, true] {
                for mode in [Mode::Spam, Mode::Pattern] {
                    for view in 0..5 {
                        {
                            let mut s = sh.settings.lock().unwrap();
                            s.lang = lang;
                            s.compact = compact;
                            s.mode = mode;
                            s.pattern.steps = if view == 4 { vec![] } else { config::example_steps() };
                            s.profiles = vec![config::Profile { name: "Profil".into(), ..Default::default() }];
                            s.active_profile = Some(0);
                            s.spam.limit = if view == 1 { SpamLimit::Duration(90.0) } else { SpamLimit::Count(10) };
                        }
                        app.settings_open = view == 2;
                        app.profiles_open = view == 3;
                        app.confirm = None;
                        app.update_dialog = false;
                        draw_frames(&mut app, &ctx, vec2(width_for(compact), height_for(mode, compact)), 3);
                    }
                }
            }
        }
        // fenêtre de mise à jour (avec et sans notes)
        for notes in [vec![], vec![("fr".to_string(), "Un".to_string()), ("en".to_string(), "One".to_string())]] {
            *app.up.state.lock().unwrap() = crate::update::UpState::Available {
                build: 58,
                version: Some("3.4.1".into()),
                notes,
                exe_url: String::new(),
                sha_url: String::new(),
                sig_url: String::new(),
            };
            app.update_dialog = true;
            draw_frames(&mut app, &ctx, vec2(width_for(false), height_for(Mode::Spam, false)), 3);
        }
        app.update_dialog = false;
        *app.up.state.lock().unwrap() = crate::update::UpState::Idle;
        // capture de macro en cours et tâche en cours
        app.settings_open = false;
        app.profiles_open = false;
        sh.settings.lock().unwrap().compact = false;
        sh.settings.lock().unwrap().mode = Mode::Pattern;
        sh.toggle_macro();
        draw_frames(&mut app, &ctx, vec2(width_for(false), height_for(Mode::Pattern, false)), 3);
        sh.cancel_macro();
        sh.start(Mode::Pattern);
        draw_frames(&mut app, &ctx, vec2(width_for(false), height_for(Mode::Pattern, false)), 3);
        sh.runner.stop();
    }
}
