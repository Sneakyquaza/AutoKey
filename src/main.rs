#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod config;
mod control;
mod engine;
mod i18n;
mod keys;
mod platform;
#[cfg(feature = "shots")]
mod shots;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    platform::init();
    let settings = config::load();
    let mode = settings.mode;
    let pin = settings.pin;

    let shared = control::Shared::new(settings, platform::keyboard());
    platform::start_test_input();
    control::spawn_poller(shared.clone());

    let icon = egui::IconData { rgba: include_bytes!("../assets/icon64.rgba").to_vec(), width: 64, height: 64 };
    let mut vp = egui::ViewportBuilder::default()
        .with_title("AutoKey")
        .with_inner_size([ui::WIDTH, ui::height_for(mode)])
        .with_decorations(false)
        .with_resizable(false)
        .with_icon(icon);
    if pin {
        vp = vp.with_window_level(egui::WindowLevel::AlwaysOnTop);
    }
    let options = eframe::NativeOptions { viewport: vp, centered: true, ..Default::default() };

    eframe::run_native(
        "AutoKey",
        options,
        Box::new(move |cc| {
            ui::setup_fonts(&cc.egui_ctx);
            shared.set_ctx(cc.egui_ctx.clone());
            #[allow(unused_mut)]
            let mut app = ui::App::new(shared);
            #[cfg(feature = "shots")]
            shots::init(&mut app);
            Ok(Box::new(app))
        }),
    )
}
