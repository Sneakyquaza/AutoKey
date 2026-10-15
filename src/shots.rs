//! Captures automatiques de l'interface (dev) : AUTOKEY_SHOTS=dossier
use crate::config::{self, Mode};
use crate::control::Target;
use crate::i18n::Lang;
use crate::ui::{height_for, App, WIDTH};
use egui::{self, vec2, ViewportCommand};

pub struct Shots {
    dir: String,
    scene: usize,
    frame: u32,
    waiting: bool,
}

const SCENES: [&str; 14] = [
    "spam_fr",
    "spam_rec_en",
    "pattern_empty",
    "pattern_ex",
    "pattern_run",
    "pattern_pt",
    "modal",
    "update",
    "update_dl",
    "settings",
    "spam_limit_pt",
    "settings_pt",
    "profiles_pt",
    "profiles_en",
];

pub fn init(app: &mut App) {
    if let Ok(dir) = std::env::var("AUTOKEY_SHOTS") {
        app.shots = Some(Shots { dir, scene: 0, frame: 0, waiting: false });
    }
}

fn setup(app: &App, ctx: &egui::Context, name: &str) {
    use crate::update::UpState;
    *app.updater().state.lock().unwrap() = match name {
        "update" => UpState::Available { build: 58, exe_url: String::new(), sha_url: String::new(), sig_url: String::new() },
        "update_dl" => UpState::Downloading(42),
        _ => UpState::Idle,
    };
    let sh = app.shared();
    sh.runner.stop();
    sh.cancel_record();
    let mut s = sh.settings.lock().unwrap();
    s.lang = Lang::Fr;
    s.profiles.clear();
    s.active_profile = None;
    match name {
        "profiles_pt" => {
            s.mode = Mode::Spam;
            s.lang = Lang::Pt;
            for n in ["Farm de recursos", "Perfil de combate com nome muito longo para caber", "Pesca"] {
                s.profiles.push(config::Profile { name: n.into(), ..Default::default() });
            }
            s.active_profile = Some(1);
        }
        "profiles_en" => {
            s.mode = Mode::Pattern;
            s.lang = Lang::En;
            for i in 1..=9 {
                s.profiles.push(config::Profile { name: format!("Profile {i}"), ..Default::default() });
            }
            s.active_profile = Some(2);
        }
        "spam_fr" => s.mode = Mode::Spam,
        "spam_rec_en" => {
            s.mode = Mode::Spam;
            s.lang = Lang::En;
            drop(s);
            sh.begin_record(Target::SpamHotkey);
            s = sh.settings.lock().unwrap();
        }
        "pattern_empty" => {
            s.mode = Mode::Pattern;
            s.pattern.steps.clear();
        }
        "pattern_ex" => {
            s.mode = Mode::Pattern;
            s.pattern.steps = config::example_steps();
            s.pattern.steps.push(config::Step::new_wait());
        }
        "pattern_run" => {
            s.mode = Mode::Pattern;
            s.pattern.loops = 3;
            drop(s);
            sh.start(Mode::Pattern);
            std::thread::sleep(std::time::Duration::from_millis(3200));
            s = sh.settings.lock().unwrap();
        }
        "update" | "update_dl" => s.mode = Mode::Spam,
        "settings" => s.mode = Mode::Spam,
        "spam_limit_pt" => {
            s.mode = Mode::Spam;
            s.lang = Lang::Pt;
            s.spam.limit = config::SpamLimit::Duration(90.0);
        }
        "settings_pt" => {
            s.mode = Mode::Spam;
            s.lang = Lang::Pt;
            s.jitter_pct = 15.0;
            s.beep = true;
        }
        "pattern_pt" => {
            s.mode = Mode::Pattern;
            s.lang = Lang::Pt;
        }
        _ => {
            s.mode = Mode::Pattern;
        }
    }
    let m = s.mode;
    drop(s);
    ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(WIDTH, height_for(m))));
}

pub fn tick(app: &mut App, ctx: &egui::Context) {
    let Some(mut st) = app.shots.take() else { return };
    // réception de la capture
    let img = ctx.input(|i| {
        i.raw.events.iter().find_map(|e| match e {
            egui::Event::Screenshot { image, .. } => Some(image.clone()),
            _ => None,
        })
    });
    if let Some(img) = img {
        let name = SCENES[st.scene];
        let [w, h] = img.size;
        let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
        for px in &img.pixels {
            out.extend_from_slice(&[px.r(), px.g(), px.b()]);
        }
        let _ = std::fs::write(format!("{}/{}.ppm", st.dir, name), out);
        st.scene += 1;
        st.frame = 0;
        st.waiting = false;
        if st.scene >= SCENES.len() {
            app.shared().runner.stop();
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }
    }
    if !st.waiting {
        if st.frame == 0 {
            app.open_confirm_for_shots(false);
            app.open_settings_for_shots(false);
            app.open_profiles_for_shots(false);
            setup(app, ctx, SCENES[st.scene]);
        }
        if SCENES[st.scene].starts_with("settings") && st.frame == 1 {
            app.open_settings_for_shots(true);
        }
        if SCENES[st.scene].starts_with("profiles") && st.frame == 1 {
            app.open_profiles_for_shots(true);
        }
        if SCENES[st.scene] == "modal" && st.frame == 1 {
            app.open_confirm_for_shots(true);
        }
        st.frame += 1;
        if st.frame > 12 {
            ctx.send_viewport_cmd(ViewportCommand::Screenshot);
            st.waiting = true;
        }
    }
    ctx.request_repaint();
    app.shots = Some(st);
}
