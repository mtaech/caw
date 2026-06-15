mod app;
mod audio;
mod models;
mod ui;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use egui::{Color32, ViewportBuilder};

use crate::app::PlayerApp;
use crate::audio::player::PlayerCommand;

/// The eframe application struct.
pub struct CawApp {
    /// Shared state: the app data model and audio player.
    app: Arc<Mutex<PlayerApp>>,
    /// egui context clone for background threads to call `request_repaint`.
    ctx: egui::Context,
    /// Search text (cached copy, synced with PlayerApp).
    search_text: String,
    /// Volume slider value (cached between frames).
    volume_slider: f32,
    /// Progress slider value (cached between frames).
    progress_slider: f64,
}

impl CawApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        let t0 = std::time::Instant::now();
        let ctx = cc.egui_ctx.clone();

        // ── Install bundled CJK font ──────────────────────────
        eprintln!("caw: new() start, loading font ({} KB)", include_bytes!("../assets/fonts/MapleMono-NF-CN-Regular.ttf").len() / 1024);
        let font_bytes: &[u8] = include_bytes!("../assets/fonts/MapleMono-NF-CN-Regular.ttf");
        let mut fonts = egui::FontDefinitions::empty();

        fonts
            .font_data
            .insert("MapleMono-NF-CN".into(), egui::FontData::from_owned(font_bytes.to_vec()).into());

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "MapleMono-NF-CN".into());

        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "MapleMono-NF-CN".into());

        ctx.set_fonts(fonts);

        // Register Google Material Symbols font (font-independent vector glyphs).
        egui_material_icons::initialize(&ctx);

        // ── Apply dark theme ───────────────────────────────────
        let mut style = (*ctx.global_style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.window_fill = Color32::from_rgb(0x12, 0x12, 0x12);
        style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(0x12, 0x12, 0x12);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(0x1a, 0x1a, 0x1a);
        style.visuals.selection.bg_fill = Color32::from_rgb(0x1d, 0xb9, 0x54);
        style.visuals.hyperlink_color = Color32::from_rgb(0x1d, 0xb9, 0x54);
        style.spacing.item_spacing = egui::vec2(8.0, 4.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        ctx.set_global_style(style);

        // ── Create the audio player ────────────────────────────
        eprintln!("caw: new() fonts+style done at +{:?}, creating Player...", t0.elapsed());
        let player = crate::audio::player::Player::new();
        eprintln!("caw: new() Player created at +{:?}", t0.elapsed());

        // ── Create shared app state ────────────────────────────
        let mut app_state = PlayerApp::new();
        app_state.scanning = true;
        app_state.player = Some(player);
        let app = Arc::new(Mutex::new(app_state));

        // ── Spawn background library scan ──────────────────────
        let app_for_scan = app.clone();
        let ctx_for_scan = ctx.clone();
        std::thread::spawn(move || {
            let dirs = crate::audio::library::find_music_dirs();
            eprintln!("caw: async scan starting, dirs={:?}", dirs);

            let mut all_tracks = Vec::new();
            for dir in &dirs {
                eprintln!("caw: scanning {:?}...", dir);
                match crate::audio::library::scan_directory(dir) {
                    Ok(t) => {
                        eprintln!("caw: found {} tracks in {:?}", t.len(), dir);
                        all_tracks.extend(t);
                    }
                    Err(e) => eprintln!("caw: scan error for {:?}: {}", dir, e),
                }
            }

            eprintln!("caw: total {} tracks, applying to app", all_tracks.len());

            if let Ok(mut guard) = app_for_scan.lock() {
                guard.set_library(all_tracks);
            }
            ctx_for_scan.request_repaint();
            eprintln!("caw: scan complete, UI updated");
        });

        // ── Spawn playback engine thread ───────────────────────
        let app_for_engine = app.clone();
        let ctx_for_engine = ctx.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(250));

                // Phase 1: check needs_decode under lock
                let needs_decode;
                let current_path;
                let volume;
                {
                    let Ok(mut guard) = app_for_engine.lock() else {
                        continue;
                    };
                    let playing = guard.playback_state
                        == crate::models::playlist::PlaybackState::Playing;
                    needs_decode = guard.needs_decode && playing;
                    current_path = if needs_decode {
                        guard.current_track().map(|t| t.path.clone())
                    } else {
                        None
                    };
                    if needs_decode {
                        guard.needs_decode = false;
                    }
                    volume = guard.volume;
                }

                // Phase 2: decode if needed (OFF the lock)
                if let Some(ref path) = current_path {
                    match crate::audio::decoder::decode_file(path) {
                        Ok(audio) => {
                            if let Ok(mut guard) = app_for_engine.lock() {
                                if guard.playback_state == crate::models::playlist::PlaybackState::Playing {
                                    if let Some(ref player) = guard.player {
                                        let _ = player.send(PlayerCommand::SetVolume(volume));
                                        let _ = player.send(PlayerCommand::Play(audio, 0.0));
                                    }
                                }
                            }
                        }
                        Err(e) => eprintln!("caw: decode error: {e}"),
                    }
                }

                // Phase 3: auto-advance check
                if let Ok(mut guard) = app_for_engine.lock() {
                    let playing = guard.playback_state == crate::models::playlist::PlaybackState::Playing;
                    let position = guard.audio_position();
                    let duration = guard.duration_secs();
                    if playing && duration > 0.0 && position >= duration - 0.3 {
                        guard.next_track();
                    }
                }

                // Always repaint
                ctx_for_engine.request_repaint();
            }
        });

        Self {
            app,
            ctx,
            search_text: String::new(),
            volume_slider: 0.8,
            progress_slider: 0.0,
        }
    }
}

impl eframe::App for CawApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx();

        // ── Keyboard shortcuts ─────────────────────────────────
        if !ctx.egui_wants_keyboard_input() {
            let mut actions: Vec<Box<dyn FnOnce(&mut PlayerApp)>> = Vec::new();

            ctx.input(|i| {
                if i.key_pressed(egui::Key::Space) {
                    actions.push(Box::new(|a| a.toggle_play()));
                }
                if i.key_pressed(egui::Key::ArrowLeft) {
                    actions.push(Box::new(|a| a.prev_track()));
                }
                if i.key_pressed(egui::Key::ArrowRight) {
                    actions.push(Box::new(|a| a.next_track()));
                }
                if i.key_pressed(egui::Key::ArrowUp) {
                    actions.push(Box::new(|a| a.set_volume((a.volume + 0.05).min(1.0))));
                }
                if i.key_pressed(egui::Key::ArrowDown) {
                    actions.push(Box::new(|a| a.set_volume((a.volume - 0.05).max(0.0))));
                }
                if i.key_pressed(egui::Key::M) {
                    actions.push(Box::new(|a| a.toggle_mute()));
                }
                if i.key_pressed(egui::Key::Escape) {
                    actions.push(Box::new(|a| {
                        if a.detail.is_some() {
                            a.close_detail();
                        }
                        if !a.filtered.search_query.is_empty() {
                            a.set_search(String::new());
                        }
                    }));
                }
                if i.key_pressed(egui::Key::F) && i.modifiers.ctrl {
                    // Handled below via memory_mut
                }
            });

            for action in actions {
                if let Ok(mut guard) = self.app.lock() {
                    action(&mut *guard);
                }
            }

            // Ctrl+F: focus search input
            if ctx.input(|i| i.key_pressed(egui::Key::F) && i.modifiers.ctrl) {
                ctx.memory_mut(|m| m.request_focus(egui::Id::new("search-input")));
            }
        }

        // ── Sync search text ───────────────────────────────────
        if let Ok(guard) = self.app.lock() {
            let sq = &guard.filtered.search_query;
            if !sq.is_empty() && self.search_text.is_empty() {
                self.search_text = sq.clone();
            }
        }

        // ── Three-panel layout ─────────────────────────────────

        // Bottom player bar (must be first to reserve space)
        egui::TopBottomPanel::bottom("player_bar")
            .min_height(88.0)
            .resizable(false)
            .show_inside(ui, |ui| {
                if let Ok(mut guard) = self.app.lock() {
                    crate::ui::player_bar::render_player_bar(
                        ui,
                        &mut *guard,
                        &mut self.volume_slider,
                        &mut self.progress_slider,
                    );
                }
            });

        // Left sidebar
        egui::SidePanel::left("sidebar")
            .exact_width(240.0)
            .resizable(false)
            .show_inside(ui, |ui| {
                if let Ok(mut guard) = self.app.lock() {
                    crate::ui::sidebar::render_sidebar(ui, &mut *guard);
                }
            });

        // Central content area
        if let Ok(mut guard) = self.app.lock() {
            crate::ui::content::render_content(ui, &mut *guard, &mut self.search_text);
        }
    }
}

// ── Headless UI smoke test ───────────────────────────────────────────
// Drives the real `render_*` functions with an egui `Context` off-screen,
// catching any panic so we can read the EXACT message + line (egui normally
// swallows UI panics into its red error overlay, hiding them from stderr).
#[cfg(test)]
mod ui_smoke {
    use super::*;
    use crate::app::{DetailTarget, NavItem, PlayerApp};
    use crate::models::track::{AudioFormat, Track};
    use std::panic::AssertUnwindSafe;
    use std::sync::Arc;

    fn make_app() -> PlayerApp {
        let tracks: Vec<Arc<Track>> = (0..3)
            .map(|i| {
                Arc::new(Track {
                    id: i as u64,
                    path: format!("/tmp/t{}.flac", i).into(),
                    title: format!("Track {}", i),
                    artist: if i == 0 { String::new() } else { format!("Artist {}", i) },
                    album: format!("Album {}", i % 2),
                    duration: std::time::Duration::from_secs(180 + i * 37),
                    track_number: i as u32 + 1,
                    format: AudioFormat::Flac,
                    cover_data: None,
                })
            })
            .collect();
        let mut app = PlayerApp::new();
        // player = None to isolate UI from cpal in the test.
        app.set_library(tracks);
        app.current_index = Some(0);
        app.playback_state = crate::models::playlist::PlaybackState::Playing;
        app
    }

    fn fresh_ctx() -> egui::Context {
        let ctx = egui::Context::default();
        let font_bytes: &[u8] = include_bytes!("../assets/fonts/MapleMono-NF-CN-Regular.ttf");
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "MapleMono-NF-CN".into(),
            egui::FontData::from_owned(font_bytes.to_vec()).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "MapleMono-NF-CN".into());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "MapleMono-NF-CN".into());
        ctx.set_fonts(fonts);
        ctx
    }

    /// Run `f` inside a CentralPanel for one frame; return Err(msg) if it panics.
    fn run_one<F>(label: &str, mut app: PlayerApp, f: F) -> Result<(), String>
    where
        F: Fn(&mut egui::Ui, &mut PlayerApp) + std::panic::RefUnwindSafe,
    {
        let ctx = fresh_ctx();
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let mut raw = egui::RawInput::default();
            raw.screen_rect =
                Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1200.0, 800.0)));
            let _ = ctx.run(raw, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_max_size(egui::vec2(1200.0, 800.0));
                    f(ui, &mut app);
                });
            });
        }));
        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<&'static str>() {
                    (*s).to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "(non-string panic)".to_string()
                };
                Err(format!("[{}] PANIC: {}", label, msg))
            }
        }
    }

    /// Faithfully reproduce the REAL `CawApp::ui` panel nesting with a real
    /// 1200×800 screen_rect, over several frames (catches cross-frame +
    /// size-dependent panics that the single-frame `CentralPanel` test misses).
    fn run_real_layout(label: &str, mut app: PlayerApp) -> Result<(), String> {
        let ctx = fresh_ctx();
        for frame in 0..4u32 {
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                let mut raw = egui::RawInput::default();
                raw.screen_rect = Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(1200.0, 800.0),
                ));
                let _ = ctx.run(raw, |ctx| {
                    egui::TopBottomPanel::bottom("player_bar")
                        .min_height(88.0)
                        .resizable(false)
                        .show(ctx, |ui| {
                            let mut v = app.volume;
                            let mut p = 0.0f64;
                            crate::ui::player_bar::render_player_bar(ui, &mut app, &mut v, &mut p);
                        });
                    egui::SidePanel::left("sidebar")
                        .exact_width(240.0)
                        .resizable(false)
                        .show(ctx, |ui| {
                            crate::ui::sidebar::render_sidebar(ui, &mut app);
                        });
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let mut s = String::new();
                        crate::ui::content::render_content(ui, &mut app, &mut s);
                    });
                });
            }));
            if let Err(e) = result {
                let msg = if let Some(s) = e.downcast_ref::<&'static str>() {
                    (*s).to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "(non-string panic)".to_string()
                };
                return Err(format!("[{}] frame {} PANIC: {}", label, frame, msg));
            }
        }
        Ok(())
    }

    #[test]
    fn render_all_views_dont_panic() {
        let cases: Vec<(&str, PlayerApp, Box<dyn Fn(&mut egui::Ui, &mut PlayerApp) + std::panic::RefUnwindSafe>)> = vec![
            ("sidebar", make_app(), Box::new(|ui, app| crate::ui::sidebar::render_sidebar(ui, app))),
            ("player_bar", make_app(), Box::new(|ui, app| {
                let mut v = 0.8f32;
                let mut p = 0.0f64;
                crate::ui::player_bar::render_player_bar(ui, app, &mut v, &mut p);
            })),
            ("content_all_music", make_app(), Box::new(|ui, app| {
                let mut s = String::new();
                crate::ui::content::render_content(ui, app, &mut s);
            })),
        ];

        let mut errs = Vec::new();
        for (label, app, f) in cases {
            if let Err(e) = run_one(label, app, &*f) {
                errs.push(e);
            }
        }

        // nav variants + detail view (REAL layout, multi-frame)
        let mut a_albums = make_app();
        a_albums.set_nav(NavItem::Albums);
        if let Err(e) = run_real_layout("real_albums", a_albums) {
            errs.push(e);
        }

        let mut a_artists = make_app();
        a_artists.set_nav(NavItem::Artists);
        if let Err(e) = run_real_layout("real_artists", a_artists) {
            errs.push(e);
        }

        let mut a_detail = make_app();
        a_detail.open_album("Album 0".to_string());
        if let Err(e) = run_real_layout("real_detail", a_detail) {
            errs.push(e);
        }

        // the default view (AllMusic) with the real 3-panel layout
        if let Err(e) = run_real_layout("real_all_music", make_app()) {
            errs.push(e);
        }

        // scanning + empty states
        let mut a_scan = PlayerApp::new();
        a_scan.scanning = true;
        if let Err(e) = run_one("content_scanning", a_scan, |ui, app| {
            let mut s = String::new();
            crate::ui::content::render_content(ui, app, &mut s);
        }) {
            errs.push(e);
        }

        if !errs.is_empty() {
            panic!("UI panics found:\n{}", errs.join("\n"));
        }
    }
}

fn main() -> eframe::Result {
    let t0 = std::time::Instant::now();
    eprintln!("caw: startup t=0 (process main entered)");

    // Surface panics to stderr even when eframe/egui swallows them into the
    // red error overlay (the panic hook runs before catch_unwind unwinds).
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        eprintln!("caw: ===== PANIC =====");
        eprintln!("caw: {}", info);
        if let Some(loc) = info.location() {
            eprintln!("caw: at {}:{}:{}", loc.file(), loc.line(), loc.column());
        }
        default_hook(info);
    }));

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(egui::vec2(1200.0, 800.0))
            .with_min_inner_size(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };
    eprintln!("caw: startup calling run_native at +{:?}", t0.elapsed());

    let result = eframe::run_native(
        "Caw Music Player",
        options,
        Box::new(move |cc| {
            eprintln!("caw: startup AppCreator invoked at +{:?}", t0.elapsed());
            Ok(Box::new(CawApp::new(cc)))
        }),
    );
    eprintln!("caw: startup run_native returned at +{:?}", t0.elapsed());
    result
}
