use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

use crate::audio::decoder::{decode_file, DecodedAudio};
use crate::audio::player::{Player, PlayerCommand};
use crate::models::playlist::RepeatMode;
use crate::models::track::Track;

// ── Decode Job ─────────────────────────────────────────────────────

/// Returned by controller methods when decode is needed outside the lock.
#[derive(Debug)]
pub enum DecodeJob {
    None,
    Play { path: PathBuf, id: u64 },
}

// ── Controller State ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CtrlState {
    Stopped,
    Playing,
    Paused,
}

// ── Playback Controller ──────────────────────────────────────────

/// Audio playback controller — the "brain" owned by Tauri managed State.
pub struct PlaybackController {
    pub library: Vec<Arc<Track>>,
    player: Option<Player>,
    /// Ordered list of track IDs (the playback queue).
    queue: Vec<u64>,
    /// Index into `queue` of the currently playing (or to-play) track.
    queue_index: Option<usize>,
    pub current_track_id: Option<u64>,
    pub state: CtrlState,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub volume: f32,
    last_volume: f32,
    pub scanning: bool,
    /// Guards against double-processing a natural track end.
    ended_track_handled: bool,
}

impl PlaybackController {
    pub fn new() -> Self {
        let mut ctrl = Self {
            library: Vec::new(),
            player: Some(Player::new()),
            queue: Vec::new(),
            queue_index: None,
            current_track_id: None,
            state: CtrlState::Stopped,
            shuffle: false,
            repeat: RepeatMode::None,
            volume: 0.8,
            last_volume: 0.8,
            scanning: false,
            ended_track_handled: false,
        };
        ctrl.sync_player_volume();
        ctrl
    }

    fn player(&self) -> &Player {
        self.player.as_ref().expect("Player not initialized")
    }

    // ── Library ──

    pub fn set_library(&mut self, tracks: Vec<Arc<Track>>) {
        self.library = tracks;
        // Reset queue to full library in natural album/track-number order.
        self.queue = self.library.iter().map(|t| t.id).collect();
        self.queue_index = None;
        self.current_track_id = None;
        self.state = CtrlState::Stopped;
        self.ended_track_handled = false;
    }

    pub fn find_track(&self, id: u64) -> Option<&Arc<Track>> {
        self.library.iter().find(|t| t.id == id)
    }

    fn find_track_path(&self, id: u64) -> Option<PathBuf> {
        self.find_track(id).map(|t| t.path.clone())
    }

    pub fn current_track_path(&self) -> Option<PathBuf> {
        self.current_track_id.and_then(|id| self.find_track_path(id))
    }

    pub fn current_track_duration(&self) -> f64 {
        self.current_track_id
            .and_then(|id| self.find_track(id))
            .map(|t| t.duration.as_secs_f64())
            .unwrap_or(0.0)
    }

    // ── Queue & Transport ──

    /// Set the playback queue and start playing at `start_id`.
    pub fn prepare_play_tracks(&mut self, ids: Vec<u64>, start_id: u64) -> DecodeJob {
        self.queue = ids;
        self.shuffle = false; // Explicit queue overrides shuffle
        let start_idx = self.queue.iter().position(|&id| id == start_id);
        self.queue_index = start_idx;
        self.current_track_id = Some(start_id);
        self.state = CtrlState::Playing;
        self.ended_track_handled = false;
        match self.find_track_path(start_id) {
            Some(path) => DecodeJob::Play { path, id: start_id },
            None => DecodeJob::None,
        }
    }

    pub fn prepare_toggle_play(&mut self) -> DecodeJob {
        match self.state {
            CtrlState::Playing => {
                self.state = CtrlState::Paused;
                let _ = self.player().send(PlayerCommand::Pause);
                DecodeJob::None
            }
            CtrlState::Paused => {
                self.state = CtrlState::Playing;
                let _ = self.player().send(PlayerCommand::Resume);
                DecodeJob::None
            }
            CtrlState::Stopped => {
                if self.current_track_id.is_none() && !self.queue.is_empty() {
                    self.queue_index = Some(0);
                    self.current_track_id = self.queue.first().copied();
                }
                match self.current_track_id.and_then(|id| self.find_track_path(id)) {
                    Some(path) => {
                        let id = self.current_track_id.unwrap();
                        self.state = CtrlState::Playing;
                        self.ended_track_handled = false;
                        DecodeJob::Play { path, id }
                    }
                    None => DecodeJob::None,
                }
            }
        }
    }

    pub fn prepare_next(&mut self) -> DecodeJob {
        let len = self.queue.len();
        if len == 0 {
            return DecodeJob::None;
        }
        let next = match self.queue_index {
            Some(i) if i + 1 < len => i + 1,
            _ if self.repeat == RepeatMode::All => 0,
            _ => {
                self.state = CtrlState::Stopped;
                return DecodeJob::None;
            }
        };
        self.queue_index = Some(next);
        let id = self.queue[next];
        self.current_track_id = Some(id);
        self.state = CtrlState::Playing;
        self.ended_track_handled = false;
        match self.find_track_path(id) {
            Some(path) => DecodeJob::Play { path, id },
            None => DecodeJob::None,
        }
    }

    pub fn prepare_prev(&mut self) -> DecodeJob {
        // If more than 3 seconds in, restart current track.
        if self.position_secs() > 3.0 {
            match self.current_track_id.and_then(|id| self.find_track_path(id)) {
                Some(path) => {
                    let id = self.current_track_id.unwrap();
                    self.state = CtrlState::Playing;
                    self.ended_track_handled = false;
                    return DecodeJob::Play { path, id };
                }
                None => return DecodeJob::None,
            }
        }
        let len = self.queue.len();
        if len == 0 {
            return DecodeJob::None;
        }
        let prev = match self.queue_index {
            Some(i) if i > 0 => i - 1,
            _ if self.repeat == RepeatMode::All => len - 1,
            _ => {
                self.state = CtrlState::Stopped;
                return DecodeJob::None;
            }
        };
        self.queue_index = Some(prev);
        let id = self.queue[prev];
        self.current_track_id = Some(id);
        self.state = CtrlState::Playing;
        self.ended_track_handled = false;
        match self.find_track_path(id) {
            Some(path) => DecodeJob::Play { path, id },
            None => DecodeJob::None,
        }
    }

    /// Auto-advance check: return a DecodeJob if the current track naturally ended.
    /// Call this once per position tick.
    pub fn check_auto_advance(&mut self) -> DecodeJob {
        let Some(ref player) = self.player else {
            return DecodeJob::None;
        };
        let is_playing = player.state.is_playing.load(Ordering::Relaxed);
        let pos_frames = player.state.position_frames.load(Ordering::Relaxed);
        let total_frames = player.state.total_frames.load(Ordering::Relaxed);

        // Reset guard if playing is true (new track started externally).
        if is_playing {
            self.ended_track_handled = false;
            return DecodeJob::None;
        }

        // Natural end: was playing, is_playing flipped to false, frames at end.
        if self.state == CtrlState::Playing && total_frames > 0 && pos_frames >= total_frames && !self.ended_track_handled
        {
            self.ended_track_handled = true;
            self.state = CtrlState::Stopped;

            // Repeat One → restart same track.
            if self.repeat == RepeatMode::One {
                if let Some(id) = self.current_track_id {
                    if let Some(path) = self.find_track_path(id) {
                        self.state = CtrlState::Playing;
                        self.ended_track_handled = false;
                        return DecodeJob::Play { path, id };
                    }
                }
            }

            // Otherwise advance to next track (honouring repeat/shuffle).
            return self.prepare_next();
        }

        DecodeJob::None
    }

    // ── Simple state changes (no decode needed) ──

    pub fn pause(&mut self) {
        self.state = CtrlState::Paused;
        let _ = self.player().send(PlayerCommand::Pause);
    }

    pub fn resume(&mut self) {
        self.state = CtrlState::Playing;
        let _ = self.player().send(PlayerCommand::Resume);
    }

    pub fn seek(&mut self, pos_secs: f64) {
        let _ = self.player().send(PlayerCommand::Seek(pos_secs));
    }

    /// Stop playback: send Stop command, clear track, set state to Stopped.
    pub fn stop_playback(&mut self) {
        if let Some(ref player) = self.player {
            let _ = player.send(PlayerCommand::Stop);
        }
        self.current_track_id = None;
        self.state = CtrlState::Stopped;
    }

    fn sync_player_volume(&mut self) {
        let _ = self
            .player()
            .send(PlayerCommand::SetVolume(self.volume));
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 1.0);
        if self.volume > 0.0 {
            self.last_volume = self.volume;
        }
        self.sync_player_volume();
    }

    pub fn toggle_mute(&mut self) {
        if self.volume > 0.0 {
            let prev = self.last_volume;
            self.volume = 0.0;
            self.last_volume = prev;
        } else {
            self.volume = self.last_volume.max(0.01);
        }
        self.sync_player_volume();
    }

    pub fn set_shuffle(&mut self, on: bool) {
        self.shuffle = on;
        if on {
            self.build_shuffle_queue();
        } else {
            // Restore to sorted-by-album-then-track-number order.
            // Pre-collect sort data to avoid borrowing self twice.
            let mut entries: Vec<(u64, String, u32)> = self
                .queue
                .iter()
                .filter_map(|id| self.find_track(*id).map(|t| (*id, t.album.clone(), t.track_number)))
                .collect();
            entries.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));
            self.queue = entries.into_iter().map(|(id, _, _)| id).collect();
        }
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }

    // ── Apply decoded audio ──

    pub fn apply_decoded(&mut self, decoded: DecodedAudio, id: u64) {
        self.current_track_id = Some(id);
        self.state = CtrlState::Playing;
        self.ended_track_handled = false;
        let _ = self.player().send(PlayerCommand::Play(decoded, 0.0));
    }

    // ── Position helper ──

    pub fn position_secs(&self) -> f64 {
        if let Some(ref player) = self.player {
            let pos = player.state.position_frames.load(Ordering::Relaxed);
            let total = player.state.total_frames.load(Ordering::Relaxed);
            let dur = self.current_track_duration();
            if total > 0 && dur > 0.0 {
                return (pos as f64 / total as f64) * dur;
            }
        }
        0.0
    }

    // ── Queue helpers ──

    fn build_shuffle_queue(&mut self) {
        if self.queue.len() <= 1 {
            return;
        }
        // Simple LCG-based Fisher-Yates shuffle (no external rand crate).
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_nanos();
        let mut rng = Lcg::new(seed as u64);
        let len = self.queue.len();
        for i in (1..len).rev() {
            let j = (rng.next() as usize) % (i + 1);
            self.queue.swap(i, j);
        }
    }

    // ── Snapshot for frontend ──

    pub fn state_dto(&self) -> PlayerStateDto {
        PlayerStateDto {
            is_playing: self.state == CtrlState::Playing,
            current_track_id: self.current_track_id,
            position: self.position_secs(),
            duration: self.current_track_duration(),
            volume: self.volume,
            shuffle: self.shuffle,
            repeat: match self.repeat {
                RepeatMode::None => "none",
                RepeatMode::One => "one",
                RepeatMode::All => "all",
            }
            .to_string(),
            queue: self.queue.clone(),
        }
    }
}

// ── Simple LCG PRNG (deterministic, no heap) ──────────────────────

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state >> 33
    }
}

// ── DTOs ───────────────────────────────────────────────────────────

#[derive(serde::Serialize, Clone)]
pub struct PlayerStateDto {
    pub is_playing: bool,
    pub current_track_id: Option<u64>,
    pub position: f64,
    pub duration: f64,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: String,
    pub queue: Vec<u64>,
}

#[derive(serde::Serialize, Clone)]
pub struct PositionPayload {
    pub current: f64,
    pub total: f64,
    pub is_playing: bool,
    pub track_id: Option<u64>,
}

#[derive(serde::Serialize, Clone)]
pub struct TrackDto {
    pub id: u64,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: f64,
    pub track_number: u32,
    pub has_cover: bool,
}

impl TrackDto {
    pub fn from_track(t: &Arc<Track>) -> Self {
        TrackDto {
            id: t.id,
            path: t.path.to_string_lossy().to_string(),
            title: if t.title.is_empty() {
                t.path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default()
            } else {
                t.title.clone()
            },
            artist: if t.artist.is_empty() {
                "Unknown Artist".to_string()
            } else {
                t.artist.clone()
            },
            album: if t.album.is_empty() {
                "Unknown Album".to_string()
            } else {
                t.album.clone()
            },
            duration_secs: t.duration.as_secs_f64(),
            track_number: t.track_number,
            has_cover: t.cover_data.is_some(),
        }
    }
}

// ── Execute Job (decode outside the controller lock) ─────────────

/// Decode the required track outside the controller lock, re-acquire and apply.
/// Uses a loop to handle decode failures by skipping to the next track.
pub fn execute_decode_job(
    mut job: DecodeJob,
    ctrl: &Mutex<PlaybackController>,
    handle: &AppHandle,
) {
    loop {
        let (path, id) = match job {
            DecodeJob::None => return,
            DecodeJob::Play { path, id } => (path, id),
        };

        let decoded = match decode_file(&path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("caw: decode error for {:?}: {}", path, e);
                let next_job = {
                    let mut c = ctrl.lock().unwrap();
                    c.state = CtrlState::Stopped;
                    c.prepare_next()
                };
                job = next_job;
                continue;
            }
        };

        let state_dto = {
            let mut c = ctrl.lock().unwrap();
            c.apply_decoded(decoded, id);
            c.state_dto()
        };

        let _ = handle.emit("playback_state", &state_dto);
        let _ = handle.emit("track_changed", serde_json::json!({ "track_id": id }));
        return;
    }
}
