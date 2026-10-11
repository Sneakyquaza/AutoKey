//! Captures automatiques de l'interface (dev) : AUTOKEY_SHOTS=dossier
use crate::config::{self, Mode};
use crate::control::Target;
use crate::i18n::Lang;
use crate::ui::{height_for, App, WIDTH};
use eframe::egui::{self, vec2, ViewportCommand};

pub struct Shots {
    dir: String,
    scene: usize,
    frame: u32,
    waiting: bool,
}

const SCENES: [&str; 7] = ["spam_fr", "spam_rec_en", "pattern_empty", "pattern_ex", "pattern_run", "pattern_pt", "modal"];

pub fn init(app: &mut App) {
    if let Ok(dir) = std::env::var("AUTOKEY_SHOTS") {
        app.shots = Some(Shots { dir, scene: 0, frame: 0, waiting: false });
    }
}

fn setup(app: &App, ctx: &egui::Context, name: &str) {
    let sh = app.shared();
    sh.runner.stop();
    sh.cancel_record();
    let mut s = sh.settings.lock().unwrap();
    s.lang = Lang::Fr;
    match name {
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
            setup(app, ctx, SCENES[st.scene]);
        }
        if SCENES[st.scene] == "modal" && st.frame == 1 {
            app.open_confirm_for_shots();
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
