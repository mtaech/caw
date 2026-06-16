//! MPRIS2 media player integration (Linux).
//!
//! Exposes two D-Bus interfaces at `/org/mpris/MediaPlayer2`:
//! - `org.mpris.MediaPlayer2` (root / lifecycle)
//! - `org.mpris.MediaPlayer2.Player` (playback control + now-playing)
//!
//! Runs on a dedicated thread with its own `zbus::blocking::Connection`.

use std::time::Duration;

use anyhow::Result;
use zbus::zvariant::{OwnedValue, Str, Value};

use tauri::{AppHandle, Emitter, Manager};

use crate::controller::CtrlState;

// ── MPRIS thread shared state ──────────────────────────────────────

/// Bridges the audio-thread tick task and the MPRIS thread.
/// The tick task pushes an encoded bitmap of changed properties.
/// The MPRIS thread reads it and emits `PropertiesChanged`.
#[derive(Clone)]
pub struct MprisNotifier {
    pub sender: std::sync::mpsc::SyncSender<u32>,
}

// Bit flags for property changes.
pub const FLAG_PLAYBACK_STATUS: u32 = 1;
pub const FLAG_METADATA: u32 = 2;
pub const FLAG_SHUFFLE: u32 = 4;
pub const FLAG_LOOP: u32 = 8;
pub const FLAG_VOLUME: u32 = 16;
pub const FLAG_POSITION: u32 = 32;
pub const FLAG_CAN_GO: u32 = 64;
pub const FLAG_ALL: u32 = !0;

// ── D-Bus connection builder ───────────────────────────────────────

/// Spawn the MPRIS thread and return a channel for property-change notifications.
pub fn spawn_mpris(app: AppHandle) -> MprisNotifier {
    let (tx, rx) = std::sync::mpsc::sync_channel::<u32>(64);
    std::thread::spawn(move || {
        if let Err(e) = run_mpris(app, rx) {
            eprintln!("caw: MPRIS thread exited: {e}");
        }
    });
    MprisNotifier { sender: tx }
}

fn run_mpris(app: AppHandle, rx: std::sync::mpsc::Receiver<u32>) -> Result<()> {
    let conn = zbus::blocking::Connection::session()?;
    let root_path = "/org/mpris/MediaPlayer2";

    // Register both interfaces on the same object path.
    conn.object_server()
        .at(root_path, MprisRoot { handle: app.clone() })?;
    conn.object_server()
        .at(root_path, MprisPlayer { handle: app.clone() })?;

    conn.request_name("org.mpris.MediaPlayer2.caw")?;
    eprintln!("caw: MPRIS service registered as org.mpris.MediaPlayer2.caw");

    // Periodic loop: handle property-changed notifications from the tick task.
    loop {
        // Block up to 1 s waiting for a notification; if none, loop anyway (keep connection alive).
        let _ = rx.recv_timeout(Duration::from_secs(10));

        // Build changed-properties dict. Scope the lock tightly so the guard is dropped
        // before building MPRIS values (which borrow non-temporary data).
        let (dto, track_info) = {
            let state = app.state::<crate::CawState>();
            let ctrl = state.ctrl.lock().unwrap();
            let dto = ctrl.state_dto();
            let track_info = dto.current_track_id.and_then(|id| {
                ctrl.find_track(id).map(|t| {
                    (
                        t.id,
                        t.duration,
                        t.display_title().to_owned(),
                        t.album.clone(),
                        t.track_number,
                        t.display_artist().to_owned(),
                        t.cover_data.clone(),
                    )
                })
            });
            (dto, track_info)
        };

        let mut changed: std::collections::HashMap<&str, Value<'_>> =
            std::collections::HashMap::new();
        let status = match &dto.is_playing {
            true => {
                if dto.position > 0.0 || dto.duration > 0.0 {
                    "Playing"
                } else {
                    "Stopped"
                }
            }
            false if dto.current_track_id.is_some() => "Paused",
            false => "Stopped",
        };
        changed.insert("PlaybackStatus", Value::from(Str::from(status)));

        let loop_status = match dto.repeat.as_str() {
            "one" => "Track",
            "all" => "Playlist",
            _ => "None",
        };
        changed.insert("LoopStatus", Value::from(Str::from(loop_status)));
        changed.insert("Shuffle", Value::from(dto.shuffle));

        // Metadata
        let mut meta: std::collections::HashMap<&str, Value<'_>> =
            std::collections::HashMap::new();
        if let Some((tid, dur, title, album, track_num, artist_str, cover_data)) = track_info {
            let track_id_val = Value::ObjectPath(
                zbus::zvariant::ObjectPath::try_from(format!(
                "/org/mpris/MediaPlayer2/Track/{tid}"
                )).unwrap()
            );
            meta.insert("mpris:trackid", track_id_val);
            meta.insert(
                "mpris:length",
                Value::from(dur.as_micros() as i64),
            );
            meta.insert("xesam:title", Value::from(Str::from(title)));
            meta.insert("xesam:album", Value::from(Str::from(album)));
            meta.insert("xesam:trackNumber", Value::from(track_num as i32));

            // Artist array
            let art_arr = zbus::zvariant::Array::from(vec![Value::Str(Str::from(artist_str))]);
            meta.insert("xesam:artist", Value::Array(art_arr));

            // Cover art
            if cover_data.is_some() {
                let cache_dir = std::env::temp_dir().join("caw-covers");
                let _ = std::fs::create_dir_all(&cache_dir);
                let cover_path = cache_dir.join(format!("cover-{tid}.jpg"));
                if !cover_path.exists() {
                    if let Some(data) = &cover_data {
                        let _ = std::fs::write(&cover_path, &**data);
                    }
                }
                meta.insert(
                    "mpris:artUrl",
                    Value::from(Str::from(format!("file://{}", cover_path.display()))),
                );
            }
        }
        changed.insert("Metadata", Value::from(meta));

        // Emit PropertiesChanged signal
        let body = (
            "org.mpris.MediaPlayer2.Player",
            changed,
            Vec::<String>::new(),
        );
        if let Err(e) = conn.emit_signal(
            None::<&str>,
            root_path,
            "org.freedesktop.DBus.Properties",
            "PropertiesChanged",
            &body,
        ) {
            eprintln!("caw: MPRIS PropertiesChanged emit error: {e}");
        }
    }
}

// ── org.mpris.MediaPlayer2 (root interface) ────────────────────────

struct MprisRoot {
    handle: AppHandle,
}

#[zbus::interface(name = "org.mpris.MediaPlayer2")]
impl MprisRoot {
    fn can_raise(&self) -> bool {
        true
    }
    fn can_quit(&self) -> bool {
        true
    }
    fn has_track_list(&self) -> bool {
        false
    }
    fn identity(&self) -> &str {
        "Caw"
    }
    fn supported_uri_schemes(&self) -> Vec<&str> {
        vec![]
    }
    fn supported_mime_types(&self) -> Vec<&str> {
        vec![
            "audio/flac",
            "audio/mpeg",
            "audio/wav",
            "audio/ogg",
            "audio/aac",
            "audio/mp4",
            "audio/x-flac",
            "audio/x-mpeg",
        ]
    }
    fn desktop_entry(&self) -> &str {
        "caw"
    }

    fn raise(&self) {
        if let Some(window) = self.handle.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }

    fn quit(&self) {
        self.handle.exit(0);
    }
}

// ── org.mpris.MediaPlayer2.Player ──────────────────────────────────

struct MprisPlayer {
    handle: AppHandle,
}

impl MprisPlayer {
    fn play_err(e: impl std::fmt::Display) -> zbus::fdo::Error {
        zbus::fdo::Error::Failed(format!("caw: {e}"))
    }
}

#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    // ── Properties ──

    fn playback_status(&self) -> String {
        let state = self.handle.state::<crate::CawState>();
        let ctrl = state.ctrl.lock().unwrap();
        let state = &ctrl.state;
        match state {
            CtrlState::Playing => {
                if ctrl.current_track_id.is_some() {
                    "Playing".into()
                } else {
                    "Stopped".into()
                }
            }
            CtrlState::Paused => "Paused".into(),
            CtrlState::Stopped => "Stopped".into(),
        }
    }

    fn loop_status(&self) -> String {
        let state = self.handle.state::<crate::CawState>();
        let ctrl = state.ctrl.lock().unwrap();
        match ctrl.repeat {
            crate::models::playlist::RepeatMode::One => "Track".into(),
            crate::models::playlist::RepeatMode::All => "Playlist".into(),
            crate::models::playlist::RepeatMode::None => "None".into(),
        }
    }

    fn rate(&self) -> f64 {
        1.0
    }

    fn shuffle(&self) -> bool {
        let state = self.handle.state::<crate::CawState>();
        let ctrl = state.ctrl.lock().unwrap();
        ctrl.shuffle
    }

    fn metadata(&self) -> std::collections::HashMap<String, zbus::zvariant::OwnedValue> {
        let state = self.handle.state::<crate::CawState>();
        let ctrl = state.ctrl.lock().unwrap();
        let mut meta: std::collections::HashMap<String, zbus::zvariant::OwnedValue> =
            std::collections::HashMap::new();
        let Some(track) = ctrl.current_track_id.and_then(|id| ctrl.find_track(id)) else {
            return meta;
        };

        meta.insert(
            "mpris:trackid".into(),
            OwnedValue::try_from(Value::ObjectPath(
                zbus::zvariant::ObjectPath::try_from(format!(
                    "/org/mpris/MediaPlayer2/Track/{}",
                    track.id
                )).unwrap()
            )).unwrap(),
        );
        meta.insert(
            "mpris:length".into(),
            OwnedValue::from(track.duration.as_micros() as i64),
        );
        meta.insert(
            "xesam:title".into(),
            OwnedValue::try_from(Value::Str(Str::from(
                track.display_title().to_string(),
            )))
            .unwrap(),
        );
        meta.insert(
            "xesam:album".into(),
            OwnedValue::try_from(Value::Str(Str::from(track.album.clone()))).unwrap(),
        );
        meta.insert(
            "xesam:trackNumber".into(),
            OwnedValue::from(track.track_number as i32),
        );

        // Artist array
        let art_arr = zbus::zvariant::Array::from(vec![Value::Str(Str::from(
            track.display_artist().to_string(),
        ))]);
        meta.insert(
            "xesam:artist".into(),
            OwnedValue::try_from(Value::Array(art_arr)).unwrap(),
        );

        // Cover artUrl
        if let Some(data) = &track.cover_data {
            let cache_dir = std::env::temp_dir().join("caw-covers");
            let _ = std::fs::create_dir_all(&cache_dir);
            let cover_path = cache_dir.join(format!("cover-{}.jpg", track.id));
            if !cover_path.exists() {
                let _ = std::fs::write(&cover_path, &**data);
            }
            meta.insert(
                "mpris:artUrl".into(),
                OwnedValue::try_from(Value::Str(Str::from(format!(
                    "file://{}",
                    cover_path.display()
                ))))
                .unwrap(),
            );
        }

        meta
    }

    fn volume(&self) -> f64 {
        let state = self.handle.state::<crate::CawState>();
        let ctrl = state.ctrl.lock().unwrap();
        ctrl.volume as f64
    }

    fn position(&self) -> i64 {
        let state = self.handle.state::<crate::CawState>();
        let ctrl = state.ctrl.lock().unwrap();
        (ctrl.position_secs() * 1_000_000.0) as i64
    }

    fn minimum_rate(&self) -> f64 {
        0.0
    }
    fn maximum_rate(&self) -> f64 {
        1.0
    }

    fn can_go_next(&self) -> bool {
        let state = self.handle.state::<crate::CawState>();
        let ctrl = state.ctrl.lock().unwrap();
        ctrl.current_track_id.is_some() && !ctrl.library.is_empty()
    }
    fn can_go_previous(&self) -> bool {
        let state = self.handle.state::<crate::CawState>();
        let ctrl = state.ctrl.lock().unwrap();
        ctrl.current_track_id.is_some() && !ctrl.library.is_empty()
    }
    fn can_play(&self) -> bool {
        let state = self.handle.state::<crate::CawState>();
        let ctrl = state.ctrl.lock().unwrap();
        !ctrl.library.is_empty()
    }
    fn can_pause(&self) -> bool {
        let state = self.handle.state::<crate::CawState>();
        let ctrl = state.ctrl.lock().unwrap();
        ctrl.current_track_id.is_some()
    }
    fn can_seek(&self) -> bool {
        let state = self.handle.state::<crate::CawState>();
        let ctrl = state.ctrl.lock().unwrap();
        ctrl.current_track_id.is_some()
    }
    fn can_control(&self) -> bool {
        true
    }

    // ── Methods ──

    fn next(&mut self) -> zbus::fdo::Result<()> {
        let state = self.handle.state::<crate::CawState>();
        let mut ctrl = state.ctrl.lock().map_err(Self::play_err)?;
        let job = ctrl.prepare_next();
        drop(ctrl);
        crate::controller::execute_decode_job(job, &state.ctrl, &self.handle);
        Ok(())
    }

    fn previous(&mut self) -> zbus::fdo::Result<()> {
        let state = self.handle.state::<crate::CawState>();
        let mut ctrl = state.ctrl.lock().map_err(Self::play_err)?;
        let job = ctrl.prepare_prev();
        drop(ctrl);
        crate::controller::execute_decode_job(job, &state.ctrl, &self.handle);
        Ok(())
    }

    fn pause(&mut self) -> zbus::fdo::Result<()> {
        let state = self.handle.state::<crate::CawState>();
        let mut ctrl = state.ctrl.lock().map_err(Self::play_err)?;
        ctrl.pause();
        let dto = ctrl.state_dto();
        drop(ctrl);
        let _ = self.handle.emit("playback_state", &dto);
        Ok(())
    }

    fn play_pause(&mut self) -> zbus::fdo::Result<()> {
        let state = self.handle.state::<crate::CawState>();
        let mut ctrl = state.ctrl.lock().map_err(Self::play_err)?;
        match ctrl.state {
            CtrlState::Playing => {
                ctrl.pause();
                let dto = ctrl.state_dto();
                drop(ctrl);
                let _ = self.handle.emit("playback_state", &dto);
            }
            CtrlState::Paused => {
                ctrl.resume();
                let dto = ctrl.state_dto();
                drop(ctrl);
                let _ = self.handle.emit("playback_state", &dto);
            }
            CtrlState::Stopped => {
                let job = ctrl.prepare_toggle_play();
                drop(ctrl);
                crate::controller::execute_decode_job(job, &state.ctrl, &self.handle);
            }
        }
        Ok(())
    }

    fn stop(&mut self) -> zbus::fdo::Result<()> {
        let state = self.handle.state::<crate::CawState>();
        let mut ctrl = state.ctrl.lock().map_err(Self::play_err)?;
        ctrl.stop_playback();
        let dto = ctrl.state_dto();
        drop(ctrl);
        let _ = self.handle.emit("playback_state", &dto);
        let _ = self
            .handle
            .emit("track_changed", serde_json::json!({ "track_id": null }));
        Ok(())
    }

    fn play(&mut self) -> zbus::fdo::Result<()> {
        let state = self.handle.state::<crate::CawState>();
        let mut ctrl = state.ctrl.lock().map_err(Self::play_err)?;
        match ctrl.state {
            CtrlState::Paused => {
                ctrl.resume();
                let dto = ctrl.state_dto();
                drop(ctrl);
                let _ = self.handle.emit("playback_state", &dto);
            }
            CtrlState::Stopped if ctrl.current_track_id.is_some() => {
                let job = ctrl.prepare_toggle_play();
                drop(ctrl);
                crate::controller::execute_decode_job(job, &state.ctrl, &self.handle);
            }
            _ => {
                drop(ctrl);
            }
        }
        Ok(())
    }

    fn seek(&mut self, offset: i64) -> zbus::fdo::Result<()> {
        // offset is in microseconds, positive = forward
        let state = self.handle.state::<crate::CawState>();
        let ctrl = state.ctrl.lock().map_err(Self::play_err)?;
        let current_pos = ctrl.position_secs();
        let dur = ctrl.current_track_duration();
        let new_pos = ((current_pos * 1_000_000.0 + offset as f64) / 1_000_000.0)
            .clamp(0.0, dur);
        drop(ctrl);
        let mut ctrl = state.ctrl.lock().map_err(Self::play_err)?;
        ctrl.seek(new_pos);
        Ok(())
    }

    fn set_position(&mut self, _track_id: zbus::zvariant::ObjectPath<'_>, position: i64) -> zbus::fdo::Result<()> {
        // position is in microseconds
        let pos_sec = position as f64 / 1_000_000.0;
        let state = self.handle.state::<crate::CawState>();
        let mut ctrl = state.ctrl.lock().map_err(Self::play_err)?;
        ctrl.seek(pos_sec);
        Ok(())
    }

    fn open_uri(&mut self, _uri: &str) -> zbus::fdo::Result<()> {
        Err(zbus::fdo::Error::NotSupported(
            "OpenUri is not supported".into(),
        ))
    }
}
