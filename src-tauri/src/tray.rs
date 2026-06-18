//! System tray for Caw.
//!
//! Tauri v2 tray with icon, menu (Show/Hide, Play/Pause, Next, Previous, Quit),
//! and click-to-toggle visibility.

use tauri::{
    AppHandle, Emitter, Manager,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::controller::DecodeJob;
use crate::CawState;

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let play_pause =
        MenuItemBuilder::with_id("play_pause", "播放 / 暂停").build(app)?;
    let next = MenuItemBuilder::with_id("next", "下一首").build(app)?;
    let previous =
        MenuItemBuilder::with_id("previous", "上一首").build(app)?;
    let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
    let show_hide =
        MenuItemBuilder::with_id("show_hide", "显示 / 隐藏").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show_hide)
        .item(&separator)
        .item(&play_pause)
        .item(&next)
        .item(&previous)
        .item(&separator)
        .item(&quit)
        .build()?;

    // Load and decode the tray icon from the embedded 32x32 PNG.
    let icon_bytes = include_bytes!("../icons/32x32.png");
    let img = image::load_from_memory(icon_bytes)
        .map_err(|e| format!("failed to decode tray icon: {e}"))?
        .into_rgba8();
    let (w, h) = img.dimensions();
    let icon = tauri::image::Image::new_owned(img.into_raw(), w, h);
    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .tooltip("Caw")
        .menu(&menu)
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "play_pause" => {
                    let state = app.state::<CawState>();
                    let mut ctrl = state.ctrl.lock().unwrap();
                    let job = ctrl.prepare_toggle_play();
                    if let DecodeJob::Play { .. } = &job {
                        drop(ctrl);
                        crate::controller::execute_decode_job(job, &state.ctrl, app);
                    } else {
                        let dto = ctrl.state_dto();
                        drop(ctrl);
                        let _ = app.emit("playback_state", &dto);
                    }
                }
                "next" => {
                    let state = app.state::<CawState>();
                    let mut ctrl = state.ctrl.lock().unwrap();
                    let job = ctrl.prepare_next();
                    drop(ctrl);
                    crate::controller::execute_decode_job(job, &state.ctrl, app);
                }
                "previous" => {
                    let state = app.state::<CawState>();
                    let mut ctrl = state.ctrl.lock().unwrap();
                    let job = ctrl.prepare_prev();
                    drop(ctrl);
                    crate::controller::execute_decode_job(job, &state.ctrl, app);
                }
                "show_hide" => {
                    if let Some(window) = app.get_webview_window("main") {
                        if window.is_visible().unwrap_or(true) {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(true) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
