#[macro_use]
mod logging;

mod app;
mod assets;
mod audio;
mod config;
mod db;
mod models;
mod paths;
mod platform;
mod root_view;
mod ui;

use std::time::Duration;

use gpui_kit::component::scroll::ScrollbarMode;
use gpui_kit::component::{ActiveTheme, Root, Theme, TitleBar};
use gpui_kit::*;

use crate::app::PlayerApp;
use crate::assets::CawAssets;
use crate::models::playlist::PlaybackState;
use crate::platform::PlatformCommand;

fn main() {
    logging::init_logging();

    gpui_kit::application()
        .with_assets(CawAssets)
        .run(|cx: &mut App| {
            gpui_kit::init(cx);

            // Apply persisted theme mode and fonts before the first frame.
            config::AppConfig::load().apply_theme(cx);
            // Scrollbars stay visible instead of fading out after scrolling, so every
            // scrollable region reads as scrollable at a glance.
            Theme::set_scrollbar_mode(ScrollbarMode::Always, cx);

            let window_bounds = platform::window_state::load()
                .map(|state| state.to_window_bounds())
                .unwrap_or_else(|| WindowBounds::centered(size(px(1080.), px(720.)), cx));
            let mut window_options = WindowOptions {
                window_bounds: Some(window_bounds),
                window_min_size: Some(size(px(860.), px(540.))),
                // Wayland app_id / X11 WM_CLASS: compositors use it to match the
                // .desktop entry, which is where the taskbar/dock icon comes from.
                app_id: Some("caw".to_owned()),
                ..TitleBar::window_options()
            };
            window_options.titlebar = Some(TitlebarOptions {
                title: Some("Caw".into()),
                ..TitleBar::title_bar_options()
            });
            let root_background = cx.theme().background;

            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| PlayerApp::new(window, cx));

                // Start keyboard focus on the app view so transport shortcuts work
                // before the user clicks anything.
                let focus_handle = view.read(cx).focus_handle.clone();
                window.focus(&focus_handle, cx);

                // Platform close requests (window-manager X, Alt+F4, session close)
                // must follow the same preference as the in-app title bar: either
                // minimize to tray or ask before quitting. Returning false keeps the
                // window alive; only the confirm dialog's OK actually quits.
                {
                    let close_entity = view.clone();
                    window.on_window_should_close(cx, move |window, cx| {
                        let close_to_tray = close_entity.read(cx).config.close_to_tray;
                        crate::log_info!("window close requested (close_to_tray={close_to_tray})");
                        crate::platform::window_state::save_now(window);
                        close_entity.update(cx, |app, _| app.save_state_now());
                        if close_to_tray {
                            crate::platform::set_taskbar_visible(false);
                            window.minimize_window();
                        } else {
                            crate::ui::dialogs::open_close_confirm(
                                window,
                                cx,
                                close_entity.clone(),
                            );
                        }
                        false
                    });
                }

                // MPRIS / desktop media integration (Linux). Never fails: without a
                // session bus it logs a warning and stays inert.
                let platform = platform::Platform::start();
                let window_handle = window.window_handle();

                // Background tick: poll the audio thread, platform commands and UI.
                let polled = view.downgrade();
                cx.spawn(async move |cx: &mut AsyncApp| {
                    loop {
                        cx.background_executor()
                            .timer(Duration::from_millis(250))
                            .await;

                        let pending = match polled.update(cx, |app, cx| {
                            app.update_position(cx);
                            app.flush_config(cx);
                            platform.publish(platform::snapshot_from_app(app));
                            platform.poll()
                        }) {
                            Ok(commands) => commands,
                            Err(_) => break,
                        };

                        for command in pending {
                            match command {
                                PlatformCommand::ShowWindow => {
                                    // Restoring from the tray must bring the taskbar entry back.
                                    crate::platform::set_taskbar_visible(true);
                                    let _ = window_handle.update(cx, |_, window, _| {
                                        window.activate_window();
                                    });
                                }
                                PlatformCommand::Quit => {
                                    crate::log_info!("quit requested; saving session");
                                    let _ = polled.update(cx, |app, _| app.save_state_now());
                                    let _ = window_handle.update(cx, |_, window, _| {
                                        platform::window_state::save_now(window);
                                    });
                                    cx.update(|cx| cx.quit());
                                    return;
                                }
                                command => {
                                    let _ = polled.update(cx, |app, cx| {
                                        apply_platform_command(app, command, cx);
                                    });
                                }
                            }
                        }
                    }
                })
                .detach();

                cx.new(|cx| Root::new(view, window, cx).bg(root_background))
            })
            .expect("caw: failed to open main window");
        });
}

/// Apply a transport command coming from MPRIS / the tray on the UI thread.
fn apply_platform_command(
    app: &mut PlayerApp,
    command: PlatformCommand,
    cx: &mut Context<PlayerApp>,
) {
    match command {
        PlatformCommand::TogglePlayPause => app.toggle_play_pause(cx),
        PlatformCommand::Play => {
            if app.playback_state != PlaybackState::Playing {
                app.toggle_play_pause(cx);
            }
        }
        PlatformCommand::Pause => {
            if app.playback_state == PlaybackState::Playing {
                app.toggle_play_pause(cx);
            }
        }
        PlatformCommand::Next => app.next_track(cx),
        PlatformCommand::Previous => app.prev_track(cx),
        PlatformCommand::SeekTo(seconds) => app.seek_to(seconds, cx),
        PlatformCommand::SeekBy(delta) => {
            let target = (app.position + delta).clamp(0.0, app.duration);
            app.seek_to(target, cx);
        }
        PlatformCommand::SetVolume(volume) => app.set_volume(volume, cx),
        PlatformCommand::ToggleMute => app.toggle_mute(cx),
        PlatformCommand::SetPlaybackMode(mode) => app.set_playback_mode(mode, cx),
        PlatformCommand::ShowWindow | PlatformCommand::Quit => {}
    }
}
