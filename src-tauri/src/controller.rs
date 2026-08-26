use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

use crate::audio::decoder::{DecodedAudio, decode_file};
use crate::audio::player::{Player, PlayerCommand};
use crate::models::playlist::RepeatMode;
use crate::models::track::Track;

// ── Decode Job ─────────────────────────────────────────────────────

/// Returned by controller methods when decode is needed outside the lock.
#[derive(Debug)]
pub enum DecodeJob {
    None,
    Play { path: PathBuf, id: u64, seek_pos: f64 },
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
    /// History of recently played track IDs (most recent first).
    pub history: Vec<u64>,
    /// Saved position in seconds (used before decode or after restoration)
    pub saved_position: f64,
    /// Persisted last track path for recovery across library scans/restarts
    pub saved_track_path: Option<String>,
    /// Persisted queue paths for recovery
    pub saved_queue_paths: Vec<String>,
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
            history: Vec::new(),
            saved_position: 0.0,
            saved_track_path: None,
            saved_queue_paths: Vec::new(),
        };
        ctrl.sync_player_volume();
        ctrl
    }

    fn player(&self) -> &Player {
        self.player.as_ref().expect("Player not initialized")
    }

    // ── Library ──

    pub fn set_library(&mut self, tracks: Vec<Arc<Track>>) {
        let target_path = self
            .current_track_path()
            .map(|p| p.to_string_lossy().to_string())
            .or_else(|| self.saved_track_path.clone());

        self.library = tracks;

        // Build queue from saved queue paths if available, otherwise default to full library
        if !self.saved_queue_paths.is_empty() {
            let mut restored_queue = Vec::new();
            for qp in &self.saved_queue_paths {
                if let Some(t) = self.library.iter().find(|t| t.path.to_string_lossy() == *qp) {
                    restored_queue.push(t.id);
                }
            }
            if !restored_queue.is_empty() {
                self.queue = restored_queue;
            } else {
                self.queue = self.library.iter().map(|t| t.id).collect();
            }
        } else {
            self.queue = self.library.iter().map(|t| t.id).collect();
        }

        // Try to match target track
        if let Some(ref path_str) = target_path {
            if let Some(matching) = self
                .library
                .iter()
                .find(|t| t.path.to_string_lossy() == *path_str)
            {
                let matched_id = matching.id;
                self.current_track_id = Some(matched_id);
                self.queue_index = self.queue.iter().position(|&id| id == matched_id);
                if self.state == CtrlState::Stopped {
                    self.state = CtrlState::Paused;
                }
                return;
            }
        }

        if self.current_track_id.is_none() {
            self.queue_index = None;
            self.state = CtrlState::Stopped;
        }
    }

    pub fn find_track(&self, id: u64) -> Option<&Arc<Track>> {
        self.library.iter().find(|t| t.id == id)
    }

    pub fn find_track_path(&self, id: u64) -> Option<PathBuf> {
        self.find_track(id).map(|t| t.path.clone())
    }

    pub fn current_track_path(&self) -> Option<PathBuf> {
        self.current_track_id
            .and_then(|id| self.find_track_path(id))
    }

    pub fn current_queue_paths(&self) -> Vec<String> {
        self.queue
            .iter()
            .filter_map(|&id| self.find_track_path(id))
            .map(|p| p.to_string_lossy().to_string())
            .collect()
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
        self.saved_position = 0.0;
        self.state = CtrlState::Playing;
        self.ended_track_handled = false;
        match self.find_track_path(start_id) {
            Some(path) => DecodeJob::Play {
                path,
                id: start_id,
                seek_pos: 0.0,
            },
            None => DecodeJob::None,
        }
    }

    /// Insert a track into the queue without switching playback.
    /// If nothing is playing, insert at end; otherwise insert right after current.
    pub fn prepare_play_next(&mut self, id: u64) -> DecodeJob {
        let insert_pos = match self.queue_index {
            Some(i) => i + 1,
            None => self.queue.len(),
        };
        self.queue.insert(insert_pos, id);
        DecodeJob::None
    }

    /// Insert a batch of tracks right after the current queue position and play them.
    /// Original queue after the batch is preserved.
    pub fn prepare_play_tracks_insert(&mut self, ids: Vec<u64>, start_id: u64) -> DecodeJob {
        if ids.is_empty() {
            return DecodeJob::None;
        }
        let insert_pos = match self.queue_index {
            Some(i) => i + 1,
            None => self.queue.len(),
        };
        // Push old track to history before switching
        if let Some(old_id) = self.current_track_id {
            let first_new = ids.first().copied();
            if first_new != Some(old_id) {
                self.push_history(old_id);
            }
        }
        // Insert all new IDs at insert_pos
        for (offset, id) in ids.iter().enumerate() {
            self.queue.insert(insert_pos + offset, *id);
        }
        let start_offset = ids.iter().position(|&id| id == start_id).unwrap_or(0);
        let start_idx = insert_pos + start_offset;
        self.queue_index = Some(start_idx);
        self.current_track_id = Some(start_id);
        self.saved_position = 0.0;
        self.state = CtrlState::Playing;
        self.ended_track_handled = false;
        self.shuffle = false; // Explicit queue overrides shuffle
        match self.find_track_path(start_id) {
            Some(path) => DecodeJob::Play {
                path,
                id: start_id,
                seek_pos: 0.0,
            },
            None => DecodeJob::None,
        }
    }

    pub fn prepare_toggle_play(&mut self) -> DecodeJob {
        match self.state {
            CtrlState::Playing => {
                self.saved_position = self.position_secs();
                self.state = CtrlState::Paused;
                let _ = self.player().send(PlayerCommand::Pause);
                DecodeJob::None
            }
            CtrlState::Paused => {
                let has_stream = if let Some(ref p) = self.player {
                    p.state.total_frames.load(Ordering::Relaxed) > 0
                } else {
                    false
                };

                if has_stream {
                    self.state = CtrlState::Playing;
                    let _ = self.player().send(PlayerCommand::Resume);
                    DecodeJob::None
                } else {
                    // Stream not yet loaded (e.g. restored from previous session)
                    match self
                        .current_track_id
                        .and_then(|id| self.find_track_path(id))
                    {
                        Some(path) => {
                            let id = self.current_track_id.unwrap();
                            let seek_pos = self.saved_position;
                            self.state = CtrlState::Playing;
                            self.ended_track_handled = false;
                            DecodeJob::Play { path, id, seek_pos }
                        }
                        None => DecodeJob::None,
                    }
                }
            }
            CtrlState::Stopped => {
                if self.current_track_id.is_none() && !self.queue.is_empty() {
                    self.queue_index = Some(0);
                    self.current_track_id = self.queue.first().copied();
                }
                match self
                    .current_track_id
                    .and_then(|id| self.find_track_path(id))
                {
                    Some(path) => {
                        let id = self.current_track_id.unwrap();
                        let seek_pos = self.saved_position;
                        self.state = CtrlState::Playing;
                        self.ended_track_handled = false;
                        DecodeJob::Play { path, id, seek_pos }
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
        self.saved_position = 0.0;
        self.state = CtrlState::Playing;
        self.ended_track_handled = false;
        match self.find_track_path(id) {
            Some(path) => DecodeJob::Play {
                path,
                id,
                seek_pos: 0.0,
            },
            None => DecodeJob::None,
        }
    }

    pub fn prepare_prev(&mut self) -> DecodeJob {
        // If more than 3 seconds in, restart current track.
        if self.position_secs() > 3.0 {
            match self
                .current_track_id
                .and_then(|id| self.find_track_path(id))
            {
                Some(path) => {
                    let id = self.current_track_id.unwrap();
                    self.saved_position = 0.0;
                    self.state = CtrlState::Playing;
                    self.ended_track_handled = false;
                    return DecodeJob::Play {
                        path,
                        id,
                        seek_pos: 0.0,
                    };
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
        self.saved_position = 0.0;
        self.state = CtrlState::Playing;
        self.ended_track_handled = false;
        match self.find_track_path(id) {
            Some(path) => DecodeJob::Play {
                path,
                id,
                seek_pos: 0.0,
            },
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
        if self.state == CtrlState::Playing
            && total_frames > 0
            && pos_frames >= total_frames
            && !self.ended_track_handled
        {
            self.ended_track_handled = true;
            self.state = CtrlState::Stopped;

            // Repeat One → restart same track.
            if self.repeat == RepeatMode::One
                && let Some(id) = self.current_track_id
                && let Some(path) = self.find_track_path(id)
            {
                self.saved_position = 0.0;
                self.state = CtrlState::Playing;
                self.ended_track_handled = false;
                return DecodeJob::Play {
                    path,
                    id,
                    seek_pos: 0.0,
                };
            }

            // Otherwise advance to next track (honouring repeat/shuffle).
            return self.prepare_next();
        }

        DecodeJob::None
    }

    // ── Simple state changes (no decode needed) ──

    pub fn pause(&mut self) {
        self.saved_position = self.position_secs();
        self.state = CtrlState::Paused;
        let _ = self.player().send(PlayerCommand::Pause);
    }

    pub fn resume(&mut self) {
        self.state = CtrlState::Playing;
        let _ = self.player().send(PlayerCommand::Resume);
    }

    pub fn seek(&mut self, pos_secs: f64) {
        self.saved_position = pos_secs;
        let _ = self.player().send(PlayerCommand::Seek(pos_secs));
    }

    /// Stop playback: send Stop command, clear track, set state to Stopped.
    pub fn stop_playback(&mut self) {
        if let Some(ref player) = self.player {
            let _ = player.send(PlayerCommand::Stop);
        }
        self.current_track_id = None;
        self.saved_position = 0.0;
        self.state = CtrlState::Stopped;
    }

    fn sync_player_volume(&mut self) {
        let _ = self.player().send(PlayerCommand::SetVolume(self.volume));
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
                .filter_map(|id| {
                    self.find_track(*id)
                        .map(|t| (*id, t.album.clone(), t.track_number))
                })
                .collect();
            entries.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));
            self.queue = entries.into_iter().map(|(id, _, _)| id).collect();
        }
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }

    // ── Apply decoded audio ──

    pub fn apply_decoded(&mut self, decoded: DecodedAudio, id: u64, seek_pos: f64) {
        if let Some(old_id) = self.current_track_id
            && old_id != id
        {
            self.push_history(old_id);
        }
        self.current_track_id = Some(id);
        self.saved_position = seek_pos;
        self.state = CtrlState::Playing;
        self.ended_track_handled = false;
        let _ = self.player().send(PlayerCommand::Play(decoded, seek_pos));
    }

    // ── History ──

    pub fn push_history(&mut self, id: u64) {
        if self.history.first() != Some(&id) {
            self.history.insert(0, id);
            self.history.truncate(200);
        }
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
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
        self.saved_position
    }

    // ── Queue helpers ──

    pub fn add_to_queue(&mut self, id: u64) {
        self.queue.push(id);
    }

    pub fn remove_from_queue(&mut self, index: usize) {
        if index >= self.queue.len() {
            return;
        }
        // Don't allow removing currently playing track.
        if let Some(qi) = self.queue_index
            && index == qi
        {
            return;
        }
        let removed_id = self.queue.remove(index);
        if let Some(ref mut qi) = self.queue_index {
            if index < *qi {
                *qi -= 1;
            }
        }
        // If queue became empty, clear current track
        if self.queue.is_empty() {
            self.queue_index = None;
            self.current_track_id = None;
            self.state = CtrlState::Stopped;
        } else if self.current_track_id == Some(removed_id) {
            self.queue_index = None;
            self.current_track_id = None;
            self.state = CtrlState::Stopped;
        }
    }

    pub fn reorder_queue(&mut self, from: usize, to: usize) {
        if from >= self.queue.len() || to >= self.queue.len() {
            return;
        }
        if from == to {
            return;
        }
        let id = self.queue.remove(from);
        self.queue.insert(to, id);
        if let Some(ref mut qi) = self.queue_index {
            if from < *qi && to >= *qi {
                *qi -= 1;
            } else if from > *qi && to <= *qi {
                *qi += 1;
            } else if from == *qi {
                *qi = to;
            }
        }
    }

    pub fn clear_queue(&mut self) {
        if let Some(qi) = self.queue_index {
            let id = self.queue[qi];
            self.queue = vec![id];
            self.queue_index = Some(0);
        } else {
            self.queue.clear();
            self.queue_index = None;
        }
    }

    pub fn save_queue_as_playlist(
        &self,
        db: &crate::db::Database,
        name: &str,
    ) -> anyhow::Result<i64> {
        let id = db.create_playlist(name)?;
        if !self.queue.is_empty() {
            let track_ids: Vec<i64> = self.queue.iter().map(|&id| id as i64).collect();
            db.add_tracks(id, &track_ids)?;
        }
        Ok(id)
    }

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
        let (path, id, seek_pos) = match job {
            DecodeJob::None => return,
            DecodeJob::Play { path, id, seek_pos } => (path, id, seek_pos),
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
            c.apply_decoded(decoded, id, seek_pos);
            crate::save_playback_config(handle, &c);
            c.state_dto()
        };

        let _ = handle.emit("playback_state", &state_dto);
        let _ = handle.emit(
            "track_changed",
            serde_json::json!({ "track_id": id }),
        );
        return;
    }
}