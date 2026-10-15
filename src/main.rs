#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod config;
mod control;
mod engine;
mod i18n;
mod keys;
mod macro_rec;
mod platform;
mod render;
#[cfg(feature = "shots")]
mod shots;
mod ui;
mod update;

use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Icon, Window, WindowId, WindowLevel};

/// Cadence max : 30 images/s (suffisant pour les survols/animations courtes, deux fois moins de CPU)
const MIN_FRAME: Duration = Duration::from_millis(33);

enum UserEvent {
    Repaint(Duration),
    /// Commande de l'icône de la zone de notification
    Tray(platform::TrayCmd),
}

fn sched(next: &mut Option<Instant>, delay: Duration) {
    let at = Instant::now() + delay;
    *next = Some(next.map_or(at, |t| t.min(at)));
}

struct Gfx {
    window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    state: egui_winit::State,
    size: (u32, u32),
}

struct Runner {
    ctx: egui::Context,
    app: ui::App,
    gfx: Option<Gfx>,
    renderer: render::Renderer,
    vinfo: egui::ViewportInfo,
    next_frame: Option<Instant>,
    last_frame: Instant,
    frames: u64,
    shot_pending: bool,
    full_present: bool,
    proxy: EventLoopProxy<UserEvent>,
    pin: bool,
    width: f32,
    height: f32,
}

impl Runner {
    fn schedule(&mut self, delay: Duration) {
        sched(&mut self.next_frame, delay);
    }

    fn paint(&mut self, el: &ActiveEventLoop) {
        let Some(g) = self.gfx.as_mut() else { return };
        self.last_frame = Instant::now();
        let first = self.frames == 0;
        self.frames += 1;

        let mut raw = g.state.take_egui_input(&g.window);
        egui_winit::update_viewport_info(&mut self.vinfo, &self.ctx, &g.window, first);
        raw.viewports.insert(egui::ViewportId::ROOT, self.vinfo.clone());

        let app = &mut self.app;
        let out = self.ctx.run(raw, |ctx| app.update(ctx));
        g.state.handle_platform_output(&g.window, out.platform_output);

        // rendu logiciel
        let size = g.window.inner_size();
        let (w, h) = (size.width.max(1), size.height.max(1));
        self.renderer.set_textures(&out.textures_delta);
        let damage = self.renderer.render(&self.ctx, out.shapes, out.pixels_per_point, w as usize, h as usize);
        self.renderer.free_textures(&out.textures_delta);
        let full = std::mem::take(&mut self.full_present) || first;
        if !damage.is_empty() || full {
            let resized = g.size != (w, h);
            if !resized || g.surface.resize(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap()).is_ok() {
                g.size = (w, h);
                if let Ok(mut buf) = g.surface.buffer_mut() {
                    if buf.len() == self.renderer.fb.len() {
                        if buf.age() == 1 && !resized && !full {
                            // le tampon contient déjà l'image précédente : on ne copie que les zones modifiées
                            let fb = &self.renderer.fb;
                            for d in &damage {
                                for y in d.y0.max(0)..d.y1.min(h as i32) {
                                    let r = y as usize * w as usize;
                                    let (a, b) = (r + d.x0.max(0) as usize, r + d.x1.min(w as i32) as usize);
                                    buf[a..b].copy_from_slice(&fb[a..b]);
                                }
                            }
                        } else {
                            buf.copy_from_slice(&self.renderer.fb);
                        }
                        let rects: Vec<softbuffer::Rect> = damage
                            .iter()
                            .filter_map(|d| {
                                Some(softbuffer::Rect {
                                    x: d.x0.max(0) as u32,
                                    y: d.y0.max(0) as u32,
                                    width: NonZeroU32::new((d.x1 - d.x0).max(0) as u32)?,
                                    height: NonZeroU32::new((d.y1 - d.y0).max(0) as u32)?,
                                })
                            })
                            .collect();
                        let _ = if full || rects.is_empty() { buf.present() } else { buf.present_with_damage(&rects) };
                    }
                }
            }
        }
        if first {
            g.window.set_visible(true);
            platform::after_window_shown(&g.window);
            // une fois visible, on ré-affiche tout au prochain passage
            self.full_present = true;
            sched(&mut self.next_frame, Duration::from_millis(50));
        }

        // commandes de fenêtre (déplacement, taille, réduire, fermer…)
        if let Some(vo) = out.viewport_output.get(&egui::ViewportId::ROOT) {
            let mut actions = egui::ahash::HashSet::default();
            egui_winit::process_viewport_commands(&self.ctx, &mut self.vinfo, vo.commands.iter().cloned(), &g.window, &mut actions);
            if actions.contains(&egui_winit::ActionRequested::Screenshot) {
                self.shot_pending = true;
            }
            if vo.repaint_delay < Duration::from_secs(3600) {
                sched(&mut self.next_frame, vo.repaint_delay);
            }
        }
        if self.shot_pending {
            self.shot_pending = false;
            let img = egui::ColorImage {
                size: [self.renderer.w, self.renderer.h],
                pixels: self.renderer.fb.iter().map(|p| egui::Color32::from_rgb((p >> 16) as u8, (p >> 8) as u8, *p as u8)).collect(),
            };
            g.state.egui_input_mut().events.push(egui::Event::Screenshot { viewport_id: egui::ViewportId::ROOT, image: Arc::new(img) });
            sched(&mut self.next_frame, Duration::ZERO);
        }
        if self.vinfo.close_requested() || self.app.wants_quit() {
            self.app.on_exit();
            el.exit();
        }
    }
}

impl ApplicationHandler<UserEvent> for Runner {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let icon = Icon::from_rgba(include_bytes!("../assets/icon64.rgba").to_vec(), 64, 64).ok();
        let mut attrs = Window::default_attributes()
            .with_title("AutoKey")
            .with_inner_size(LogicalSize::new(self.width as f64, self.height as f64))
            .with_decorations(false)
            .with_resizable(false)
            .with_visible(false)
            .with_window_icon(icon)
            .with_window_level(if self.pin { WindowLevel::AlwaysOnTop } else { WindowLevel::Normal });
        if let Some(m) = el.primary_monitor() {
            let s = m.scale_factor();
            let ms = m.size();
            let mp = m.position();
            let (ww, wh) = (self.width as f64 * s, self.height as f64 * s);
            attrs = attrs.with_position(PhysicalPosition::new(
                mp.x + ((ms.width as f64 - ww) / 2.0).max(0.0) as i32,
                mp.y + ((ms.height as f64 - wh) / 2.0).max(0.0) as i32,
            ));
        }
        let window = Rc::new(el.create_window(attrs).expect("fenêtre"));
        let context = softbuffer::Context::new(window.clone()).expect("softbuffer");
        let surface = softbuffer::Surface::new(&context, window.clone()).expect("surface");
        let state = egui_winit::State::new(
            self.ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(1024), // atlas de police compact
        );
        self.gfx = Some(Gfx { window, surface, state, size: (0, 0) });
        self.schedule(Duration::ZERO);
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(g) = self.gfx.as_mut() else { return };
        match &event {
            WindowEvent::CloseRequested if self.app.tray_enabled() => {
                // icône de notification active : on cache la fenêtre, la tâche continue (Quitter : menu de l'icône)
                g.window.set_visible(false);
                return;
            }
            WindowEvent::CloseRequested => {
                self.app.on_exit();
                el.exit();
                return;
            }
            WindowEvent::RedrawRequested => {
                // demandé par le système (fenêtre affichée, découverte, restaurée…) : tout ré-afficher
                self.full_present = true;
                self.paint(el);
                return;
            }
            _ => {}
        }
        let resized =
            matches!(event, WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } | WindowEvent::Occluded(false) | WindowEvent::Focused(_));
        if resized {
            self.full_present = true;
        }
        let resp = g.state.on_window_event(&g.window, &event);
        if resp.repaint || resized {
            self.schedule(Duration::ZERO);
        }
    }

    fn user_event(&mut self, el: &ActiveEventLoop, ev: UserEvent) {
        match ev {
            UserEvent::Repaint(d) => self.schedule(d),
            UserEvent::Tray(cmd) => {
                match cmd {
                    platform::TrayCmd::Show => {
                        if let Some(g) = self.gfx.as_ref() {
                            g.window.set_visible(true);
                            g.window.set_minimized(false);
                            g.window.focus_window();
                        }
                        self.full_present = true;
                    }
                    platform::TrayCmd::ToggleSpam => self.app.tray_toggle(config::Mode::Spam),
                    platform::TrayCmd::TogglePattern => self.app.tray_toggle(config::Mode::Pattern),
                    // même chemin de sortie que la fermeture : touches relâchées, config sauvegardée, icône retirée
                    platform::TrayCmd::Quit => {
                        self.app.on_exit();
                        el.exit();
                        return;
                    }
                }
                self.schedule(Duration::ZERO);
            }
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        match self.next_frame {
            Some(t) => {
                // limite à 60 i/s : on regroupe les demandes trop rapprochées
                let t = t.max(self.last_frame + MIN_FRAME);
                if Instant::now() >= t {
                    self.next_frame = None;
                    // on dessine directement : sous Windows, une fenêtre encore invisible
                    // ne reçoit jamais RedrawRequested (elle ne s'afficherait jamais)
                    self.paint(el);
                    match self.next_frame {
                        Some(n) => el.set_control_flow(ControlFlow::WaitUntil(n.max(self.last_frame + MIN_FRAME))),
                        None => el.set_control_flow(ControlFlow::Wait),
                    }
                } else {
                    el.set_control_flow(ControlFlow::WaitUntil(t));
                }
            }
            None => el.set_control_flow(ControlFlow::Wait),
        }
    }
}

fn main() {
    if !platform::single_instance() {
        return;
    }
    platform::init();
    // panique : ne jamais laisser de touche enfoncée (le hook tourne aussi avec panic = "abort")
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        platform::release_held();
        prev_hook(info);
    }));
    let settings = config::load();
    let (mode, pin, compact) = (settings.mode, settings.pin, settings.compact);

    let shared = control::Shared::new(settings, platform::keyboard());
    platform::start_test_input();
    control::spawn_poller(shared.clone());
    let updater = update::Updater::new(shared.clone());
    updater.spawn_checker();

    let event_loop = EventLoop::<UserEvent>::with_user_event().build().expect("event loop");
    let proxy = event_loop.create_proxy();
    let ctx = egui::Context::default();
    ui::setup_fonts(&ctx);
    {
        let p = proxy.clone();
        ctx.set_request_repaint_callback(move |info| {
            let _ = p.send_event(UserEvent::Repaint(info.delay));
        });
    }
    {
        let p = proxy.clone();
        platform::tray_init(Box::new(move |c| {
            let _ = p.send_event(UserEvent::Tray(c));
        }));
    }
    shared.set_ctx(ctx.clone());
    #[allow(unused_mut)]
    let mut app = ui::App::new(shared, updater);
    #[cfg(feature = "shots")]
    shots::init(&mut app);

    let mut runner = Runner {
        ctx,
        app,
        gfx: None,
        renderer: render::Renderer::new(ui::BG),
        vinfo: Default::default(),
        next_frame: None,
        last_frame: Instant::now() - MIN_FRAME,
        frames: 0,
        shot_pending: false,
        full_present: true,
        proxy,
        pin,
        width: ui::width_for(compact),
        height: ui::height_for(mode, compact),
    };
    let _ = &runner.proxy;
    let _ = event_loop.run_app(&mut runner);
}
