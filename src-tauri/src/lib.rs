// Allow dead code for items not yet wired (e.g. models without playlist crate).
#![allow(dead_code)]

mod audio;
mod controller;
mod db;
mod models;
mod mpris;
mod platform;
mod tray;
pub mod paths;
pub mod logger;

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
    pub music_dirs: Mutex<Vec<PathBuf>>,
    pub db: db::Database,
    pub mpris_tx: Mutex<Option<std::sync::mpsc::SyncSender<u32>>>,
    pub minimize_to_tray: Mutex<bool>,
    pub queue_replace_mode: Mutex<bool>,
}

// ── Commands ───────────────────────────────────────────────────────

/// Health-check.
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! Caw Tauri backend is alive.")
}

/// Open a native folder picker, add the chosen dir to the list, and start scanning.
///
/// MUST be `async` + `blocking_pick_folder()`: a sync command runs on the main
/// thread, and `blocking_*` does `rx.recv()` on a rendezvous channel whose
/// callback must run on the main thread's event loop (GTK). Calling it from the
/// main thread deadlocks — the window freezes and the dialog never appears.
/// An `async` command runs on a worker thread, leaving the main thread free to
/// pump the dialog loop. (See tauri-plugin-dialog `blocking_*` docs.)
#[tauri::command]
async fn pick_music_folder(
    app: AppHandle,
    state: tauri::State<'_, CawState>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let picked = app.dialog().file().blocking_pick_folder();

    let Some(fp) = picked else {
        return Ok(None);
    };
    let path = fp.into_path().unwrap();
    let path_str = path.to_string_lossy().to_string();
    let path_buf = PathBuf::from(&path_str);

    // Append to list (skip if already present).
    {
        let mut dirs = state.music_dirs.lock().map_err(|e| e.to_string())?;
        if !dirs.contains(&path_buf) {
            dirs.push(path_buf);
        }
    }

    // Persist full list to store.
    {
        use tauri_plugin_store::StoreExt;
        if let Ok(store) = app.store(crate::paths::config_path()) {
            let dirs = state.music_dirs.lock().map_err(|e| e.to_string())?;
            let arr: serde_json::Value = dirs
                .iter()
                .map(|d| serde_json::Value::String(d.to_string_lossy().into_owned()))
                .collect();
            store.set("music_dirs", arr);
            let _ = store.save();
        }
    }

    // Mark scanning and spawn background scan over ALL dirs.
    {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.scanning = true;
    }

    let h = app.clone();
    let dirs = state.music_dirs.lock().map_err(|e| e.to_string())?.clone();
    std::thread::spawn(move || {
        scan_all_libraries(h, dirs);
    });

    Ok(Some(path_str))
}

/// Return whether "minimize to tray on close" is enabled.
#[tauri::command]
fn get_minimize_to_tray(state: tauri::State<CawState>) -> bool {
    *state.minimize_to_tray.lock().unwrap()
}

/// Enable or disable "minimize to tray on close".
///
/// Persists the choice to the store so it survives restarts; also updates
/// the in-memory flag on CawState so the window event handler can read it
/// without hitting the store on every close attempt.
#[tauri::command]
async fn set_minimize_to_tray(
    app: AppHandle,
    state: tauri::State<'_, CawState>,
    enable: bool,
) -> Result<(), String> {
    {
        let mut v = state.minimize_to_tray.lock().map_err(|e| e.to_string())?;
        *v = enable;
    }
    // Persist
    use tauri_plugin_store::StoreExt;
    if let Ok(store) = app.store(crate::paths::config_path()) {
        store.set("minimize_to_tray", serde_json::Value::Bool(enable));
        let _ = store.save();
    }
    Ok(())
}

/// Return whether queue replace mode is enabled.
#[tauri::command]
fn get_queue_replace_mode(state: tauri::State<CawState>) -> bool {
    *state.queue_replace_mode.lock().unwrap()
}

/// Enable or disable queue replace mode.
#[tauri::command]
async fn set_queue_replace_mode(
    app: AppHandle,
    state: tauri::State<'_, CawState>,
    enabled: bool,
) -> Result<(), String> {
    {
        let mut v = state.queue_replace_mode.lock().map_err(|e| e.to_string())?;
        *v = enabled;
    }
    // Persist
    use tauri_plugin_store::StoreExt;
    if let Ok(store) = app.store(crate::paths::config_path()) {
        store.set("queue_replace_mode", serde_json::Value::Bool(enabled));
        let _ = store.save();
    }
    Ok(())
}

/// Re-scan all configured music directories (handles additions/deletions/dedup).
#[tauri::command]
fn rescan_all(app: AppHandle, state: tauri::State<CawState>) -> Result<(), String> {
    let dirs = state.music_dirs.lock().map_err(|e| e.to_string())?.clone();
    if dirs.is_empty() {
        return Err("没有配置的音乐目录".into());
    }
    {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.scanning = true;
    }
    let h = app.clone();
    std::thread::spawn(move || {
        scan_all_libraries(h, dirs);
    });
    Ok(())
}

/// Return the list of configured music directories.
#[tauri::command]
fn get_music_dirs(state: tauri::State<CawState>) -> Vec<String> {
    state
        .music_dirs
        .lock()
        .unwrap()
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

/// Remove a music directory from the list and re-scan.
#[tauri::command]
async fn remove_music_dir(
    app: AppHandle,
    state: tauri::State<'_, CawState>,
    path: String,
) -> Result<(), String> {
    {
        let mut dirs = state.music_dirs.lock().map_err(|e| e.to_string())?;
        dirs.retain(|d| d.to_string_lossy().as_ref() != path.as_str());
    }

    // Persist updated list.
    {
        use tauri_plugin_store::StoreExt;
        if let Ok(store) = app.store(crate::paths::config_path()) {
            let dirs = state.music_dirs.lock().map_err(|e| e.to_string())?;
            let arr: serde_json::Value = dirs
                .iter()
                .map(|d| serde_json::Value::String(d.to_string_lossy().into_owned()))
                .collect();
            store.set("music_dirs", arr);
            let _ = store.save();
        }
    }

    // Re-scan remaining dirs.
    let h = app.clone();
    let dirs = state.music_dirs.lock().map_err(|e| e.to_string())?.clone();
    std::thread::spawn(move || {
        if dirs.is_empty() {
            let state = h.state::<CawState>();
            let mut ctrl = state.ctrl.lock().unwrap();
            ctrl.set_library(Vec::new());
            ctrl.scanning = false;
            let _ = h.emit("library_updated", serde_json::json!({}));
        } else {
            scan_all_libraries(h, dirs);
        }
    });

    Ok(())
}

/// Persist current playback state (last track path, progress, volume, shuffle, repeat, queue) to config.json
pub(crate) fn save_playback_config(app: &AppHandle, ctrl: &PlaybackController) {
    use tauri_plugin_store::StoreExt;
    if let Ok(store) = app.store(crate::paths::config_path()) {
        if let Some(path) = ctrl.current_track_path() {
            store.set(
                "last_track_path",
                serde_json::Value::String(path.to_string_lossy().into_owned()),
            );
        }
        store.set("last_position", serde_json::json!(ctrl.position_secs()));
        store.set("volume", serde_json::json!(ctrl.volume));
        store.set("shuffle", serde_json::json!(ctrl.shuffle));
        store.set(
            "repeat",
            serde_json::Value::String(
                match ctrl.repeat {
                    crate::models::playlist::RepeatMode::None => "none",
                    crate::models::playlist::RepeatMode::One => "one",
                    crate::models::playlist::RepeatMode::All => "all",
                }
                .to_string(),
            ),
        );
        let q_paths = ctrl.current_queue_paths();
        if !q_paths.is_empty() {
            let q_val: serde_json::Value = q_paths
                .into_iter()
                .map(serde_json::Value::String)
                .collect();
            store.set("last_queue_paths", q_val);
        }
        let _ = store.save();
    }
}

/// Run a blocking library scan over all configured directories on a background thread.
/// Merges results deduped by path, then calls set_library once at the end.
fn scan_all_libraries(app: AppHandle, dirs: Vec<PathBuf>) {
    let _ = app.emit("scan_progress", serde_json::json!({ "scanned": 0 }));

    let mut seen: std::collections::HashMap<PathBuf, std::sync::Arc<crate::models::track::Track>> =
        std::collections::HashMap::new();

    for dir in &dirs {
        match library::scan_directory(dir) {
            Ok(tracks) => {
                eprintln!("caw: scanned {} tracks from {:?}", tracks.len(), dir);
                for t in tracks {
                    seen.entry(t.path.clone()).or_insert(t);
                }
            }
            Err(e) => {
                eprintln!("caw: library scan error for {:?}: {}", dir, e);
            }
        }
    }

    let merged: Vec<std::sync::Arc<crate::models::track::Track>> = seen.into_values().collect();
    eprintln!(
        "caw: merged library has {} tracks (from {} dirs)",
        merged.len(),
        dirs.len()
    );

    let (dto, pos_payload) = {
        let state = app.state::<CawState>();
        let mut ctrl = state.ctrl.lock().unwrap();
        ctrl.set_library(merged);
        ctrl.scanning = false;
        let dto = ctrl.state_dto();
        let pos = PositionPayload {
            current: ctrl.position_secs(),
            total: ctrl.current_track_duration(),
            is_playing: ctrl.state == CtrlState::Playing,
            track_id: ctrl.current_track_id,
        };
        (dto, pos)
    };

    let _ = app.emit("library_updated", serde_json::json!({}));
    let _ = app.emit("playback_state", &dto);
    let _ = app.emit("position", &pos_payload);
}

/// Return the full library as DTOs.
#[tauri::command]
fn get_library(state: tauri::State<CawState>) -> Vec<TrackDto> {
    let ctrl = state.ctrl.lock().unwrap();
    ctrl.library.iter().map(TrackDto::from_track).collect()
}

/// Return a single track's cover art as raw bytes.
#[tauri::command]
fn get_cover(state: tauri::State<CawState>, id: u64) -> Option<Vec<u8>> {
    let ctrl = state.ctrl.lock().unwrap();
    ctrl.find_track(id)
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

/// Insert tracks into the queue after the current position and start playing.
#[tauri::command]
fn play_tracks_insert(
    app: AppHandle,
    state: tauri::State<CawState>,
    ids: Vec<u64>,
    start_id: u64,
) -> Result<(), String> {
    let job = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.prepare_play_tracks_insert(ids, start_id)
    };
    controller::execute_decode_job(job, &state.ctrl, &app);
    Ok(())
}

/// Insert a track into the queue without switching playback.
#[tauri::command]
fn play_next(app: AppHandle, state: tauri::State<CawState>, id: u64) -> Result<(), String> {
    let dto = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.prepare_play_next(id);
        ctrl.state_dto()
    };
    let _ = app.emit("queue_changed", &dto);
    Ok(())
}

#[tauri::command]
fn add_to_queue(app: AppHandle, state: tauri::State<CawState>, id: u64) -> Result<(), String> {
    let dto = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.add_to_queue(id);
        ctrl.state_dto()
    };
    let _ = app.emit("queue_changed", &dto);
    Ok(())
}

#[tauri::command]
fn remove_from_queue(
    app: AppHandle,
    state: tauri::State<CawState>,
    index: usize,
) -> Result<(), String> {
    let dto = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.remove_from_queue(index);
        ctrl.state_dto()
    };
    let _ = app.emit("queue_changed", &dto);
    Ok(())
}

#[tauri::command]
fn reorder_queue(
    app: AppHandle,
    state: tauri::State<CawState>,
    from: usize,
    to: usize,
) -> Result<(), String> {
    let dto = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.reorder_queue(from, to);
        ctrl.state_dto()
    };
    let _ = app.emit("queue_changed", &dto);
    Ok(())
}

#[tauri::command]
fn clear_queue(app: AppHandle, state: tauri::State<CawState>) -> Result<(), String> {
    let dto = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.clear_queue();
        ctrl.state_dto()
    };
    let _ = app.emit("queue_changed", &dto);
    Ok(())
}

#[tauri::command]
fn get_queue_state(state: tauri::State<CawState>) -> serde_json::Value {
    let ctrl = state.ctrl.lock().unwrap();
    serde_json::json!({
        "queue": ctrl.state_dto().queue,
        "current_track_id": ctrl.current_track_id,
        "history": ctrl.history.clone(),
    })
}

#[tauri::command]
fn save_queue_as_playlist(
    app: AppHandle,
    state: tauri::State<CawState>,
    name: String,
) -> Result<i64, String> {
    let id = {
        let ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.save_queue_as_playlist(&state.db, &name)
            .map_err(|e| e.to_string())?
    };
    let _ = app.emit("playlist_changed", serde_json::json!({}));
    Ok(id)
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
        save_playback_config(&app, &ctrl);
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
        save_playback_config(&app, &ctrl);
        ctrl.state_dto()
    };
    let _ = app.emit("playback_state", &dto);
    Ok(())
}

/// Set volume (0.0 – 1.0).
#[tauri::command]
fn set_volume(app: AppHandle, state: tauri::State<CawState>, vol: f32) -> Result<(), String> {
    let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
    ctrl.set_volume(vol);
    save_playback_config(&app, &ctrl);
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
        save_playback_config(&app, &ctrl);
        ctrl.state_dto()
    };
    let _ = app.emit("playback_state", &dto);
    Ok(())
}

/// Set repeat mode: "none", "one", "all".
#[tauri::command]
fn set_repeat(app: AppHandle, state: tauri::State<CawState>, mode: String) -> Result<(), String> {
    use crate::models::playlist::RepeatMode;
    let rpt = match mode.as_str() {
        "one" => RepeatMode::One,
        "all" => RepeatMode::All,
        _ => RepeatMode::None,
    };
    let dto = {
        let mut ctrl = state.ctrl.lock().map_err(|e| e.to_string())?;
        ctrl.set_repeat(rpt);
        save_playback_config(&app, &ctrl);
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
fn create_playlist(
    app: AppHandle,
    state: tauri::State<CawState>,
    name: String,
) -> Result<i64, String> {
    let id = state.db.create_playlist(&name).map_err(|e| e.to_string())?;
    let _ = app.emit("playlist_changed", serde_json::json!({}));
    Ok(id)
}

/// Rename a playlist.
#[tauri::command]
fn rename_playlist(
    app: AppHandle,
    state: tauri::State<CawState>,
    id: i64,
    name: String,
) -> Result<(), String> {
    state
        .db
        .rename_playlist(id, &name)
        .map_err(|e| e.to_string())?;
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
fn add_to_playlist(
    app: AppHandle,
    state: tauri::State<CawState>,
    playlist_id: i64,
    track_ids: Vec<i64>,
) -> Result<(), String> {
    state
        .db
        .add_tracks(playlist_id, &track_ids)
        .map_err(|e| e.to_string())?;
    let _ = app.emit("playlist_changed", serde_json::json!({ "id": playlist_id }));
    Ok(())
}

/// Remove tracks from a playlist.
#[tauri::command]
fn remove_from_playlist(
    app: AppHandle,
    state: tauri::State<CawState>,
    playlist_id: i64,
    track_ids: Vec<i64>,
) -> Result<(), String> {
    state
        .db
        .remove_tracks(playlist_id, &track_ids)
        .map_err(|e| e.to_string())?;
    let _ = app.emit("playlist_changed", serde_json::json!({ "id": playlist_id }));
    Ok(())
}

/// Reorder a track within a playlist.
#[tauri::command]
fn reorder_playlist(
    app: AppHandle,
    state: tauri::State<CawState>,
    playlist_id: i64,
    track_id: i64,
    new_position: i64,
) -> Result<(), String> {
    state
        .db
        .reorder(playlist_id, track_id, new_position)
        .map_err(|e| e.to_string())?;
    let _ = app.emit("playlist_changed", serde_json::json!({ "id": playlist_id }));
    Ok(())
}

/// Cached result of the `fc-list` enumeration.
///
/// The font set does not change while the app runs, and the settings dialog
/// re-queries on every mount, so the subprocess + parse is done once.
static FONT_CACHE: Mutex<Option<Vec<String>>> = Mutex::new(None);

/// Enumerate system-installed fonts via platform-specific APIs.
/// Linux uses fontconfig (fc-list); other platforms return empty for now.
#[tauri::command]
fn get_system_fonts() -> Result<Vec<String>, String> {
    if let Ok(guard) = FONT_CACHE.lock()
        && let Some(ref cached) = *guard
    {
        return Ok(cached.clone());
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("fc-list")
            .args(["--format=%{family}\n"])
            .output()
            .map_err(|e| format!("caw: failed to run fc-list: {}", e))?;

        if !output.status.success() {
            return Err(format!("caw: fc-list exited with {}", output.status));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut families: Vec<String> = stdout
            .lines()
            .flat_map(|line| line.split(','))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        families.sort();
        families.dedup();

        // Exclude icon/symbol/emoji fonts that are unsuitable for UI.
        families.retain(|f| {
            let l = f.to_lowercase();
            !l.contains("emoji")
                && !l.contains("symbol")
                && !l.contains("dingbat")
                && !l.contains("wingding")
                && !l.contains("webdings")
        });

        if let Ok(mut guard) = FONT_CACHE.lock() {
            *guard = Some(families.clone());
        }

        Ok(families)
    }

    #[cfg(not(target_os = "linux"))]
    {
        // TODO: implement for Windows (DirectWrite) and macOS (CoreText)
        Ok(vec![])
    }
}

// ── App Entry ──────────────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(CawState {
            ctrl: Mutex::new(PlaybackController::new()),
            music_dirs: Mutex::new(Vec::new()),
            db: db::Database::open(),
            mpris_tx: Mutex::new(None),
            minimize_to_tray: Mutex::new(false),
            queue_replace_mode: Mutex::new(true), // default: replace mode on
        })
        .setup(|app| {
            crate::log_info!("caw: starting up — directory unified at {:?}", crate::paths::caw_dir());

            // ── Resolution-aware window sizing & precise centering ──
            if let Some(window) = app.get_webview_window("main") {
                let monitor_opt = window
                    .current_monitor()
                    .ok()
                    .flatten()
                    .or_else(|| window.primary_monitor().ok().flatten());

                if let Some(monitor) = monitor_opt {
                    let mon_pos = monitor.position();
                    let mon_size = monitor.size();
                    let scale_factor = monitor.scale_factor();

                    let screen_w = mon_size.width as f64 / scale_factor;
                    let screen_h = mon_size.height as f64 / scale_factor;

                    // Scale relative to 2K (2560x1440 baseline: 1600x1200, aspect 4:3)
                    // On 1080p (1920x1080), scale is 0.75 -> 1200x900
                    let scale = (screen_w / 2560.0).min(screen_h / 1440.0).clamp(0.5, 1.5);
                    let target_w = (1600.0 * scale).min(screen_w * 0.92).round();
                    let target_h = (1200.0 * scale).min(screen_h * 0.90).round();

                    // Convert to physical coordinates to avoid GTK/X11 rounding offsets
                    let phys_w = (target_w * scale_factor).round() as u32;
                    let phys_h = (target_h * scale_factor).round() as u32;

                    let target_x = mon_pos.x + ((mon_size.width as i64 - phys_w as i64) / 2) as i32;
                    let target_y = mon_pos.y + ((mon_size.height as i64 - phys_h as i64) / 2) as i32;

                    crate::log_info!(
                        "caw: window sizing - monitor: {}x{}, scale: {}, target: {}x{} at ({}, {})",
                        mon_size.width, mon_size.height, scale_factor, phys_w, phys_h, target_x, target_y
                    );

                    let _ = window.set_size(tauri::PhysicalSize::new(phys_w, phys_h));
                    let _ = window.set_position(tauri::PhysicalPosition::new(target_x, target_y));
                }
            }

            // ── Restore persisted music dirs and start background scan ──
            {
                use tauri_plugin_store::StoreExt;
                if let Ok(store) = app.store(crate::paths::config_path()) {
                    // Try new format: "music_dirs" as JSON array.
                    let mut dirs: Vec<PathBuf> = Vec::new();
                    if let Some(val) = store.get("music_dirs")
                        && let Some(arr) = val.as_array()
                    {
                        for v in arr {
                            if let Some(s) = v.as_str() {
                                let p = PathBuf::from(s);
                                if p.exists() {
                                    dirs.push(p);
                                }
                            }
                        }
                    }

                    // Migration from old single-string "music_dir" format.
                    if dirs.is_empty()
                        && let Some(val) = store.get("music_dir")
                        && let Some(path_str) = val.as_str()
                        && !path_str.is_empty()
                    {
                        let p = PathBuf::from(path_str);
                        if p.exists() {
                            dirs.push(p);
                        }
                        // Migrate: write new key, delete old key, save.
                        let arr: serde_json::Value = dirs
                            .iter()
                            .map(|d| serde_json::Value::String(d.to_string_lossy().into_owned()))
                            .collect();
                        store.set("music_dirs", arr);
                        store.delete("music_dir");
                        let _ = store.save();
                    }

                    if !dirs.is_empty() {
                        let state = app.state::<CawState>();
                        *state.music_dirs.lock().unwrap() = dirs.clone();

                        let h = app.handle().clone();
                        std::thread::spawn(move || {
                            scan_all_libraries(h, dirs);
                        });
                    }
                }
            }

            // ── System tray (attempt even on Wayland — GTK may use
            //    StatusNotifierItem if available via appindicator/SNI bridge) ──
            if let Err(e) = tray::setup_tray(app.handle()) {
                eprintln!("caw: tray setup error: {e}");
                if platform::is_wayland() {
                    eprintln!("caw: on Wayland, install GNOME AppIndicator extension or");
                    eprintln!("caw: use KDE for tray icon support (StatusNotifierItem)");
                }
            }

            // ── MPRIS (media keys + DE integration) ──
            let mpris_tx = mpris::spawn_mpris(app.handle().clone());
            *app.state::<CawState>().mpris_tx.lock().unwrap() = Some(mpris_tx.sender.clone());

            // ── Read persistable minimze_to_tray setting ──
            {
                use tauri_plugin_store::StoreExt;
                if let Ok(store) = app.store(crate::paths::config_path())
                    && let Some(val) = store.get("minimize_to_tray")
                    && let Some(b) = val.as_bool()
                {
                    *app.state::<CawState>().minimize_to_tray.lock().unwrap() = b;
                }
            }

            // ── Read persisted queue_replace_mode setting ──
            {
                use tauri_plugin_store::StoreExt;
                if let Ok(store) = app.store(crate::paths::config_path())
                    && let Some(val) = store.get("queue_replace_mode")
                    && let Some(b) = val.as_bool()
                {
                    *app.state::<CawState>().queue_replace_mode.lock().unwrap() = b;
                }
            }

            // ── Read persisted playback settings (last track, position, queue, volume, shuffle, repeat) ──
            {
                use tauri_plugin_store::StoreExt;
                if let Ok(store) = app.store(crate::paths::config_path()) {
                    let state = app.state::<CawState>();
                    let mut ctrl = state.ctrl.lock().unwrap();

                    if let Some(val) = store.get("volume")
                        && let Some(v) = val.as_f64()
                    {
                        ctrl.set_volume(v as f32);
                    }
                    if let Some(val) = store.get("shuffle")
                        && let Some(b) = val.as_bool()
                    {
                        ctrl.shuffle = b;
                    }
                    if let Some(val) = store.get("repeat")
                        && let Some(s) = val.as_str()
                    {
                        ctrl.repeat = match s {
                            "one" => crate::models::playlist::RepeatMode::One,
                            "all" => crate::models::playlist::RepeatMode::All,
                            _ => crate::models::playlist::RepeatMode::None,
                        };
                    }
                    if let Some(val) = store.get("last_track_path")
                        && let Some(s) = val.as_str()
                    {
                        ctrl.saved_track_path = Some(s.to_string());
                    }
                    if let Some(val) = store.get("last_position")
                        && let Some(pos) = val.as_f64()
                    {
                        ctrl.saved_position = pos;
                    }
                    if let Some(val) = store.get("last_queue_paths")
                        && let Some(arr) = val.as_array()
                    {
                        ctrl.saved_queue_paths = arr
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                    }
                }
            }

            // ── Position tick + auto-advance task ──
            let tick_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut tick_count: u64 = 0;
                loop {
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

                    // 2. Emit position event & periodic persistence save
                    if ctrl.current_track_id.is_some() {
                        let payload = PositionPayload {
                            current: ctrl.position_secs(),
                            total: ctrl.current_track_duration(),
                            is_playing: ctrl.state == CtrlState::Playing,
                            track_id: ctrl.current_track_id,
                        };
                        tick_count = tick_count.wrapping_add(1);
                        if tick_count % 8 == 0 {
                            save_playback_config(&tick_handle, &ctrl);
                        }
                        drop(ctrl);
                        let _ = tick_handle.emit("position", &payload);
                    } else {
                        drop(ctrl);
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.app_handle().state::<CawState>();
                {
                    if let Ok(ctrl) = state.ctrl.lock() {
                        save_playback_config(window.app_handle(), &ctrl);
                    }
                }
                if *state.minimize_to_tray.lock().unwrap() {
                    api.prevent_close();
                    let _ = window.hide();
                }
            } else if let tauri::WindowEvent::Destroyed = event {
                let state = window.app_handle().state::<CawState>();
                if let Ok(ctrl) = state.ctrl.lock() {
                    save_playback_config(window.app_handle(), &ctrl);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_library,
            get_cover,
            get_state,
            play_tracks,
            play_tracks_insert,
            play_next,
            add_to_queue,
            remove_from_queue,
            reorder_queue,
            clear_queue,
            get_queue_state,
            save_queue_as_playlist,
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
            get_music_dirs,
            remove_music_dir,
            get_minimize_to_tray,
            set_minimize_to_tray,
            get_queue_replace_mode,
            set_queue_replace_mode,
            rescan_all,
            list_playlists,
            get_playlist,
            create_playlist,
            rename_playlist,
            delete_playlist,
            add_to_playlist,
            remove_from_playlist,
            reorder_playlist,
            get_system_fonts,
        ])
        .run(tauri::generate_context!())
        .expect("error while running caw application");
}