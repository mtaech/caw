//! Desktop platform integration.
//!
//! Two independent features live here:
//!
//! * mpris (Linux only) -- an MPRIS2 D-Bus service so desktop media widgets and
//!   media keys can control playback and see now-playing metadata.
//! * window_state -- persisted window geometry (~/.config/caw/window.json) so the
//!   window reopens at the same size/position.
//!
//! The tray is a hand-written StatusNotifierItem + DBusMenu implementation (ksni 0.3.7
//! is incompatible with zbus 5.19); see PLAN.md.
//!
//! # Threading contract
//!
//! Platform threads NEVER touch gpui entities. Data flows in two directions:
//!
//! * platform -> app: PlatformCommand values over a crossbeam channel, drained by
//!   the app's 250 ms timer via Platform::poll.
//! * app -> platform: a NowPlaying snapshot published by that same timer via
//!   Platform::publish; MPRIS reads it to answer D-Bus property queries.
//!
//! Every feature degrades gracefully: if the session bus is missing, the MPRIS thread
//! logs a warning and exits; the player keeps running.

// Off Linux there is no MPRIS producer, so state readers/commands are legitimately unused.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, unbounded};

use crate::app::PlayerApp;
use crate::models::playlist::{PlaybackMode, PlaybackState};

#[cfg(target_os = "linux")]
pub mod kwin;
#[cfg(target_os = "linux")]
pub mod mpris;
#[cfg(target_os = "linux")]
pub mod tray;
pub mod window_state;

/// Commands produced by platform threads and applied by the app timer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlatformCommand {
    /// Toggle between play and pause.
    TogglePlayPause,
    /// Resume playback (no-op while already playing).
    Play,
    /// Pause playback (no-op while not playing).
    Pause,
    Next,
    Previous,
    /// Seek to an absolute position in seconds.
    SeekTo(f64),
    /// Seek by a relative offset in seconds (positive = forward).
    SeekBy(f64),
    /// Set volume in 0.0 - 1.0.
    SetVolume(f32),
    /// Toggle mute, restoring the previous volume.
    ToggleMute,
    /// Switch playback mode (sequence / shuffle / repeat list / repeat one).
    SetPlaybackMode(PlaybackMode),
    /// Bring the main window to the foreground.
    ShowWindow,
    /// Quit the application.
    Quit,
}

/// Snapshot of playback state published by the app timer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NowPlaying {
    pub playback_state: PlaybackState,
    pub track_id: Option<u64>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track_number: u32,
    pub duration_secs: f64,
    pub position_secs: f64,
    pub volume: f32,
    pub playback_mode: PlaybackMode,
    /// file:// URL of the track itself (xesam:url).
    pub file_url: Option<String>,
    /// file:// URL of the cover image (mpris:artUrl).
    pub art_url: Option<String>,
    pub can_go_next: bool,
    pub can_go_previous: bool,
}

impl NowPlaying {
    /// MPRIS PlaybackStatus value.
    pub(crate) fn mpris_playback_status(&self) -> &'static str {
        match self.playback_state {
            PlaybackState::Playing => "Playing",
            PlaybackState::Paused => "Paused",
            PlaybackState::Stopped => "Stopped",
        }
    }

    /// MPRIS LoopStatus value derived from the merged playback mode.
    pub(crate) fn mpris_loop_status(&self) -> &'static str {
        match self.playback_mode {
            PlaybackMode::RepeatOne => "Track",
            PlaybackMode::RepeatList => "Playlist",
            PlaybackMode::Sequence | PlaybackMode::Shuffle => "None",
        }
    }

    /// MPRIS Shuffle value derived from the merged playback mode.
    pub(crate) fn mpris_shuffle(&self) -> bool {
        self.playback_mode == PlaybackMode::Shuffle
    }

    pub(crate) fn has_track(&self) -> bool {
        self.track_id.is_some()
    }
}

/// State shared between the app timer and the platform threads.
pub(crate) struct Shared {
    state: Mutex<NowPlaying>,
    changed: Condvar,
    commands: Sender<PlatformCommand>,
}

impl Shared {
    fn new(commands: Sender<PlatformCommand>) -> Self {
        Self {
            state: Mutex::new(NowPlaying::default()),
            changed: Condvar::new(),
            commands,
        }
    }

    /// Replace the published snapshot and wake every waiting platform thread.
    pub(crate) fn publish(&self, snapshot: NowPlaying) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *state = snapshot;
        drop(state);
        self.changed.notify_all();
    }

    /// Current snapshot (cheap clone; called by D-Bus property getters).
    pub(crate) fn read(&self) -> NowPlaying {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Block until a new snapshot is published or the timeout expires.
    pub(crate) fn wait_for_change(&self, timeout: Duration) -> NowPlaying {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let (state, _) = self
            .changed
            .wait_timeout(state, timeout)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.clone()
    }

    /// Send a command to the app timer. Never blocks (unbounded channel).
    pub(crate) fn send(&self, command: PlatformCommand) {
        let _ = self.commands.send(command);
    }
}

/// Handle owned by the app: publish state, drain commands, keep threads alive.
#[derive(Clone)]
pub struct Platform {
    shared: Arc<Shared>,
    commands: Arc<Receiver<PlatformCommand>>,
}

impl Platform {
    /// Start MPRIS (on Linux) and return the app-side handle.
    ///
    /// Never fails: missing D-Bus / unsupported platforms only log a warning.
    pub fn start() -> Self {
        let (commands_tx, commands_rx) = unbounded();
        let shared = Arc::new(Shared::new(commands_tx));
        crate::log_info!("platform: starting desktop integration");

        #[cfg(target_os = "linux")]
        {
            mpris::spawn(Arc::clone(&shared));
            tray::spawn(Arc::clone(&shared));
        }

        #[cfg(not(target_os = "linux"))]
        crate::log_info!(
            "platform: MPRIS is only available on Linux; desktop integration disabled"
        );

        Self {
            shared,
            commands: Arc::new(commands_rx),
        }
    }

    /// Publish current playback state for platform threads (call every tick).
    pub fn publish(&self, snapshot: NowPlaying) {
        self.shared.publish(snapshot);
    }

    /// Drain pending commands from platform threads (call every tick, apply immediately).
    pub fn poll(&self) -> Vec<PlatformCommand> {
        self.commands.try_iter().collect()
    }
}

/// Build a NowPlaying snapshot from the app state. Read-only; safe to call per tick.
pub fn snapshot_from_app(app: &PlayerApp) -> NowPlaying {
    let mut snapshot = NowPlaying {
        playback_state: app.playback_state,
        position_secs: app.position.max(0.0),
        duration_secs: app.duration.max(0.0),
        volume: app.volume.clamp(0.0, 1.0),
        playback_mode: app.playback_mode,
        can_go_next: !app.queue.is_empty(),
        can_go_previous: !app.queue.is_empty(),
        ..NowPlaying::default()
    };

    if let Some(track) = app.current_track.as_ref() {
        snapshot.track_id = Some(track.id);
        snapshot.title = track.display_title().to_owned();
        snapshot.artist = track.display_artist().to_owned();
        snapshot.album = track.display_album().to_owned();
        snapshot.track_number = track.track_number;
        snapshot.duration_secs = track.duration.as_secs_f64();
        snapshot.file_url = Some(file_url(&track.path));
        // Writes the cover to ~/.config/caw/covers/ on first use and returns its path.
        snapshot.art_url = crate::paths::get_or_save_cover(track).map(|path| file_url(&path));
    }

    snapshot
}

/// Format a local path as a file:// URL (space-escaped, as MPRIS clients expect).
fn file_url(path: &std::path::Path) -> String {
    let mut url = String::from("file://");
    for ch in path.to_string_lossy().chars() {
        match ch {
            ' ' => url.push_str("%20"),
            '#' => url.push_str("%23"),
            '%' => url.push_str("%25"),
            _ => url.push(ch),
        }
    }
    url
}
