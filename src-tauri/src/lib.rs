// Allow dead code for items not yet wired (e.g. models without playlist crate).
#![allow(dead_code)]

mod audio;
mod controller;
mod db;
mod models;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::audio::library;
use crate::controller::{
    CtrlState, DecodeJob, PlaybackController, PlayerStateDto, PositionPayload, TrackDto,
};

// ── Shared State (managed by Tauri) ────────────────────────────────

pub struct CawState {
    pub ctrl: Mutex<PlaybackController>,
    pub music_dir: Mutex<Option<PathBuf>>,
    pub db: db::Database,
}

// ── Commands ───────────────────────────────────────────────────────

/// Health-check.
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! Caw Tauri backend is alive.")
}

/// Open a native folder picker, persist the choice, and start scanning.
#[tauri::command]
fn pick_music_folder(app: AppHandle, state: tauri::State<CawState>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let picked = app
        .dialog()
        .file()
        .blocking_pick_folder();

    let Some(fp) = picked else {
        return Ok(None);
    };
    let path = fp.into_path().unwrap();
    let path_str = path.to_string_lossy().to_string();

    // Persist to store.
    {
        use tauri_plugin_store::StoreExt;
        if let Ok(store) = app.store("config.json") {
            store.set(
                "music_dir",
                serde_json::Value::String(path_str.clone()),
            );
            let _ = store.save();
        }
    }

    // Mark scanning and spawn background scan.
    {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.scanning = true;
    }

    let h = app.clone();
    std::thread::spawn(move || {
        scan_library(h, path);
    });

    Ok(Some(path_str))
}

/// Run a blocking library scan on a background thread.
fn scan_library(app: AppHandle, path: PathBuf) {
    let _ = app.emit("scan_progress", serde_json::json!({ "scanned": 0 }));

    let tracks = match library::scan_directory(&path) {
        Ok(t) => {
            eprintln!("caw: scanned {} tracks from {:?}", t.len(), path);
            t
        }
        Err(e) => {
            eprintln!("caw: library scan error: {}", e);
            let state = app.state::<CawState>();
            let mut ctrl = state.ctrl.lock().unwrap();
            ctrl.scanning = false;
            return;
        }
    };

    {
        let state = app.state::<CawState>();
        let mut ctrl = state.ctrl.lock().unwrap();
        ctrl.set_library(tracks);
        ctrl.scanning = false;
    }

    let _ = app.emit("library_updated", serde_json::json!({}));
}

/// Return the full library as DTOs.
#[tauri::command]
fn get_library(state: tauri::State<CawState>) -> Vec<TrackDto> {
    let ctrl = state.ctrl.lock().unwrap();
    ctrl.library.iter().map(|t| TrackDto::from_track(t)).collect()
}

/// Return a single track's cover art as raw bytes.
#[tauri::command]
fn get_cover(state: tauri::State<CawState>, id: u64) -> Option<Vec<u8>> {
    let ctrl = state.ctrl.lock().unwrap();
    ctrl
        .find_track(id)
        .and_then(|t| t.cover_data.clone())
        .map(|arc| (*arc).clone())
}

/// Return a full snapshot of the current playback state (for initial bootstrap).
#[tauri::command]
fn get_state(state: tauri::State<CawState>) -> PlayerStateDto {
    let ctrl = state.ctrl.lock().unwrap();
    ctrl.state_dto()
}

/// Set the playback queue and start playing at `start_id`.
#[tauri::command]
fn play_tracks(
    app: AppHandle,
    state: tauri::State<CawState>,
    ids: Vec<u64>,
    start_id: u64,
) -> Result<(), String> {
    let job = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.prepare_play_tracks(ids, start_id)
    };
    controller::execute_decode_job(job, &state.ctrl, &app);
    Ok(())
}

/// Toggle between Playing / Paused / Stopped->Playing.
#[tauri::command]
fn toggle_play(app: AppHandle, state: tauri::State<CawState>) -> Result<(), String> {
    let job = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        let job = ctrl.prepare_toggle_play();
        // For state toggle without decode (play<->pause), emit immediately.
        if matches!(job, DecodeJob::None) {
            let dto = ctrl.state_dto();
            drop(ctrl);
            let _ = app.emit("playback_state", &dto);
        }
        job
    };
    controller::execute_decode_job(job, &state.ctrl, &app);
    Ok(())
}

/// Pause playback.
#[tauri::command]
fn pause(app: AppHandle, state: tauri::State<CawState>) -> Result<(), String> {
    let dto = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.pause();
        ctrl.state_dto()
    };
    let _ = app.emit("playback_state", &dto);
    Ok(())
}

/// Resume from pause.
#[tauri::command]
fn resume(app: AppHandle, state: tauri::State<CawState>) -> Result<(), String> {
    let dto = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.resume();
        ctrl.state_dto()
    };
    let _ = app.emit("playback_state", &dto);
    Ok(())
}

/// Next track.
#[tauri::command]
fn next_track(app: AppHandle, state: tauri::State<CawState>) -> Result<(), String> {
    let job = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.prepare_next()
    };
    controller::execute_decode_job(job, &state.ctrl, &app);
    Ok(())
}

/// Previous track.
#[tauri::command]
fn prev_track(app: AppHandle, state: tauri::State<CawState>) -> Result<(), String> {
    let job = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.prepare_prev()
    };
    controller::execute_decode_job(job, &state.ctrl, &app);
    Ok(())
}

/// Seek to a position in seconds.
#[tauri::command]
fn seek(app: AppHandle, state: tauri::State<CawState>, sec: f64) -> Result<(), String> {
    let dto = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.seek(sec);
        ctrl.state_dto()
    };
    let _ = app.emit("playback_state", &dto);
    Ok(())
}

/// Set volume (0.0 – 1.0).
#[tauri::command]
fn set_volume(state: tauri::State<CawState>, vol: f32) -> Result<(), String> {
    let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
    ctrl.set_volume(vol);
    Ok(())
}

/// Toggle between mute and last non-zero volume.
#[tauri::command]
fn toggle_mute(app: AppHandle, state: tauri::State<CawState>) -> Result<(), String> {
    let dto = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.toggle_mute();
        ctrl.state_dto()
    };
    let _ = app.emit("playback_state", &dto);
    Ok(())
}

/// Enable / disable shuffle.
#[tauri::command]
fn set_shuffle(app: AppHandle, state: tauri::State<CawState>, on: bool) -> Result<(), String> {
    let dto = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.set_shuffle(on);
        ctrl.state_dto()
    };
    let _ = app.emit("playback_state", &dto);
    Ok(())
}

/// Set repeat mode: "none", "one", "all".
#[tauri::command]
fn set_repeat(
    app: AppHandle,
    state: tauri::State<CawState>,
    mode: String,
) -> Result<(), String> {
    use crate::models::playlist::RepeatMode;
    let rpt = match mode.as_str() {
        "one" => RepeatMode::One,
        "all" => RepeatMode::All,
        _ => RepeatMode::None,
    };
    let dto = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.set_repeat(rpt);
        ctrl.state_dto()
    };
    let _ = app.emit("playback_state", &dto);
    Ok(())
}

// ── Playlist Commands ────────────────────────────────────────────────

/// List all playlists.
#[tauri::command]
fn list_playlists(state: tauri::State<CawState>) -> Vec<db::PlaylistRow> {
    state.db.list_playlists()
}

/// Get a single playlist with its track IDs.
#[tauri::command]
fn get_playlist(state: tauri::State<CawState>, id: i64) -> Option<db::PlaylistWithTracks> {
    state.db.get_playlist(id)
}

/// Create a new playlist. Returns the new playlist ID.
#[tauri::command]
fn create_playlist(app: AppHandle, state: tauri::State<CawState>, name: String) -> Result<i64, String> {
    let id = state.db.create_playlist(&name).map_err(|e| e.to_string())?;
    let _ = app.emit("playlist_changed", serde_json::json!({}));
    Ok(id)
}

/// Rename a playlist.
#[tauri::command]
fn rename_playlist(app: AppHandle, state: tauri::State<CawState>, id: i64, name: String) -> Result<(), String> {
    state.db.rename_playlist(id, &name).map_err(|e| e.to_string())?;
    let _ = app.emit("playlist_changed", serde_json::json!({}));
    Ok(())
}

/// Delete a playlist.
#[tauri::command]
fn delete_playlist(app: AppHandle, state: tauri::State<CawState>, id: i64) -> Result<(), String> {
    state.db.delete_playlist(id).map_err(|e| e.to_string())?;
    let _ = app.emit("playlist_changed", serde_json::json!({}));
    Ok(())
}

/// Add tracks to a playlist.
#[tauri::command]
fn add_to_playlist(app: AppHandle, state: tauri::State<CawState>, playlist_id: i64, track_ids: Vec<i64>) -> Result<(), String> {
    state.db.add_tracks(playlist_id, &track_ids).map_err(|e| e.to_string())?;
    let _ = app.emit("playlist_changed", serde_json::json!({ "id": playlist_id }));
    Ok(())
}

/// Remove tracks from a playlist.
#[tauri::command]
fn remove_from_playlist(app: AppHandle, state: tauri::State<CawState>, playlist_id: i64, track_ids: Vec<i64>) -> Result<(), String> {
    state.db.remove_tracks(playlist_id, &track_ids).map_err(|e| e.to_string())?;
    let _ = app.emit("playlist_changed", serde_json::json!({ "id": playlist_id }));
    Ok(())
}

/// Reorder a track within a playlist.
#[tauri::command]
fn reorder_playlist(app: AppHandle, state: tauri::State<CawState>, playlist_id: i64, track_id: i64, new_position: i64) -> Result<(), String> {
    state.db.reorder(playlist_id, track_id, new_position).map_err(|e| e.to_string())?;
    let _ = app.emit("playlist_changed", serde_json::json!({ "id": playlist_id }));
    Ok(())
}

// ── App Entry ──────────────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(CawState {
            ctrl: Mutex::new(PlaybackController::new()),
            music_dir: Mutex::new(None),
            db: db::Database::open(),
        })
        .setup(|app| {
            // ── Restore persisted music_dir and start background scan ──
            {
                use tauri_plugin_store::StoreExt;
                if let Ok(store) = app.store("config.json") {
                    if let Some(val) = store.get("music_dir") {
                        if let Some(path_str) = val.as_str() {
                            if !path_str.is_empty() {
                                let path = PathBuf::from(path_str);
                                if path.exists() {
                                    let state = app.state::<CawState>();
                                    *state.music_dir.lock().unwrap() = Some(path.clone());

                                    let h = app.handle().clone();
                                    std::thread::spawn(move || {
                                        scan_library(h, path);
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // ── Position tick + auto-advance task ──
            let tick_handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_millis(250));

                let state = tick_handle.state::<CawState>();
                let mut ctrl = match state.ctrl.lock() {
                    Ok(g) => g,
                    Err(_) => continue,
                };

                // 1. Auto-advance check
                let job = ctrl.check_auto_advance();
                if let DecodeJob::Play { .. } = &job {
                    // Decode outside the lock.
                    drop(ctrl);
                    controller::execute_decode_job(job, &state.ctrl, &tick_handle);
                    // Re-acquire for the position emit below.
                    ctrl = match state.ctrl.lock() {
                        Ok(g) => g,
                        Err(_) => continue,
                    };
                }

                // 2. Emit position event (only when a track is loaded).
                if ctrl.current_track_id.is_some() {
                    let payload = PositionPayload {
                        current: ctrl.position_secs(),
                        total: ctrl.current_track_duration(),
                        is_playing: ctrl.state == CtrlState::Playing,
                        track_id: ctrl.current_track_id,
                    };
                    drop(ctrl);
                    let _ = tick_handle.emit("position", &payload);
                } else {
                    drop(ctrl);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_library,
            get_cover,
            get_state,
            play_tracks,
            toggle_play,
            pause,
            resume,
            next_track,
            prev_track,
            seek,
            set_volume,
            toggle_mute,
            set_shuffle,
            set_repeat,
            pick_music_folder,
            list_playlists,
            get_playlist,
            create_playlist,
            rename_playlist,
            delete_playlist,
            add_to_playlist,
            remove_from_playlist,
            reorder_playlist,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Caw Tauri application");
}
