use std::sync::Arc;

use crate::audio::player::{Player, PlayerCommand};
use crate::models::playlist::{PlaybackState, RepeatMode};
use crate::models::track::Track;

/// Sidebar navigation items.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum NavItem {
    AllMusic,
    Artists,
    Albums,
    Playlists,
    Folders,
}

/// Sort order for track display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    #[default]
    Album,
    Artist,
    Title,
    Duration,
}

/// Detail browsing target — opens a narrowed view for a specific album or artist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailTarget {
    Album(String),
    Artist(String),
}

/// Index-based reference to a filtered track view.
/// Rather than cloning `Vec<Track>` on every render, we maintain a
/// `filtered_indices: Vec<usize>` that points into the library.
#[derive(Debug, Clone, Default)]
pub struct FilteredView {
    pub indices: Vec<usize>,
    pub sort_key: SortKey,
    pub search_query: String,
}

/// Top-level application state shared across all views.
/// Wrapped in `Arc<Mutex<PlayerApp>>` by the eframe `CawApp`.
pub struct PlayerApp {
    /// All tracks in the library, wrapped in Arc to avoid copies.
    pub library: Vec<Arc<Track>>,
    /// Index-based filtered view of the library.
    pub filtered: FilteredView,
    /// Index into `filtered.indices` of the currently playing track.
    pub current_index: Option<usize>,
    /// Index into `filtered.indices` of the UI-selected track (for single-click).
    pub selected_index: Option<usize>,
    /// Playback state.
    pub playback_state: PlaybackState,
    /// Current playback position in seconds (updated by audio thread).
    pub position: f64,
    /// Volume (0.0 – 1.0).
    pub volume: f32,
    /// Repeat mode.
    pub repeat_mode: RepeatMode,
    /// Shuffle enabled.
    pub shuffle: bool,
    /// Selected nav item.
    pub selected_nav: NavItem,
    /// Detail browsing target (album or artist); overrides nav & search when set.
    pub detail: Option<DetailTarget>,
    /// Whether a library scan is in progress.
    pub scanning: bool,
    /// Audio player engine (CPAL output).
    pub player: Option<Player>,
    /// Flag: true when we need to decode current track and send PCM to player.
    pub needs_decode: bool,
    /// Last non-zero volume saved for mute toggle restore.
    pub last_volume: f32,
}

impl PlayerApp {
    pub fn new() -> Self {
        Self {
            library: Vec::new(),
            filtered: FilteredView::default(),
            current_index: None,
            selected_index: None,
            playback_state: PlaybackState::Stopped,
            position: 0.0,
            volume: 0.8,
            repeat_mode: RepeatMode::None,
            shuffle: false,
            selected_nav: NavItem::AllMusic,
            detail: None,
            scanning: false,
            player: None,
            needs_decode: false,
            last_volume: 0.8,
        }
    }

    /// Get the currently playing track.
    pub fn current_track(&self) -> Option<&Arc<Track>> {
        self.current_index
            .and_then(|fi| self.filtered.indices.get(fi))
            .and_then(|&li| self.library.get(li))
    }

    /// Duration of the currently playing track in seconds.
    pub fn duration_secs(&self) -> f64 {
        self.current_track()
            .map(|t| t.duration.as_secs_f64())
            .unwrap_or(0.0)
    }

    /// Return filtered track count.
    #[inline]
    pub fn filtered_count(&self) -> usize {
        self.filtered.indices.len()
    }

    /// Sum of durations of all filtered tracks, in seconds.
    pub fn total_duration_secs(&self) -> f64 {
        self.filtered
            .indices
            .iter()
            .filter_map(|&li| self.library.get(li))
            .fold(0.0, |acc, t| acc + t.duration.as_secs_f64())
    }

    /// Set the UI selection (single-click) without starting playback.
    pub fn select_at(&mut self, idx: usize) {
        if idx < self.filtered.indices.len() {
            self.selected_index = Some(idx);
        }
    }

    /// Open an album detail view, filtering to that album.
    pub fn open_album(&mut self, name: String) {
        self.detail = Some(DetailTarget::Album(name));
        self.selected_index = None;
        self.update_filtered_view();
    }

    /// Open an artist detail view, filtering to that artist.
    pub fn open_artist(&mut self, name: String) {
        self.detail = Some(DetailTarget::Artist(name));
        self.selected_index = None;
        self.update_filtered_view();
    }

    /// Close detail view and return to the main filtered list.
    pub fn close_detail(&mut self) {
        self.detail = None;
        self.selected_index = None;
        self.update_filtered_view();
    }

    /// Set the sort key and re-sort.
    pub fn set_sort(&mut self, key: SortKey) {
        self.filtered.sort_key = key;
        self.update_filtered_view();
    }

    /// Get a track by filtered index.
    #[inline]
    pub fn get_filtered_track(&self, filtered_idx: usize) -> Option<&Arc<Track>> {
        self.filtered
            .indices
            .get(filtered_idx)
            .and_then(|&li| self.library.get(li))
    }

    /// Get current playback position from the audio thread (atomic read).
    pub fn audio_position(&self) -> f64 {
        if let Some(ref player) = self.player {
            let pos = player
                .state
                .position_frames
                .load(std::sync::atomic::Ordering::Relaxed);
            let total = player
                .state
                .total_frames
                .load(std::sync::atomic::Ordering::Relaxed);
            let dur = self.duration_secs();
            if total > 0 && dur > 0.0 {
                return (pos as f64 / total as f64) * dur;
            }
        }
        self.position
    }

    /// Update the filtered view based on current nav, detail target, search, and sort key.
    pub fn update_filtered_view(&mut self) {
        let q = &self.filtered.search_query;
        let q_lower = q.to_lowercase();
        let search_active = !q.is_empty();
        let nav = self.selected_nav;
        let sort_key = self.filtered.sort_key;

        let mut indices: Vec<usize> = (0..self.library.len()).collect();

        // If inside a detail view, filter by album/artist — ignores nav & search.
        if let Some(ref target) = self.detail {
            match target {
                DetailTarget::Album(name) => {
                    indices.retain(|&i| self.library[i].album == *name);
                    indices.sort_by(|&a, &b| {
                        self.library[a]
                            .track_number
                            .cmp(&self.library[b].track_number)
                    });
                }
                DetailTarget::Artist(name) => {
                    indices.retain(|&i| self.library[i].display_artist() == name);
                    indices.sort_by(|&a, &b| {
                        self.library[a]
                            .album
                            .cmp(&self.library[b].album)
                            .then(
                                self.library[a]
                                    .track_number
                                    .cmp(&self.library[b].track_number),
                            )
                    });
                }
            }
        } else {
            // Main list: apply nav-derived ordering, then sort_key, then search filter.
            match nav {
                NavItem::Artists => {
                    indices.sort_by(|&a, &b| {
                        self.library[a]
                            .artist
                            .cmp(&self.library[b].artist)
                            .then_with(|| self.library[a].album.cmp(&self.library[b].album))
                    });
                }
                _ => {
                    indices.sort_by(|&a, &b| {
                        self.library[a]
                            .album
                            .cmp(&self.library[b].album)
                            .then(self.library[a].track_number.cmp(&self.library[b].track_number))
                    });
                }
            }

            // Apply sort_key (may re-order after nav sorting).
            match sort_key {
                SortKey::Album => { /* already sorted by album */ }
                SortKey::Artist => {
                    indices.sort_by(|&a, &b| {
                        self.library[a]
                            .display_artist()
                            .cmp(self.library[b].display_artist())
                    });
                }
                SortKey::Title => {
                    indices.sort_by(|&a, &b| {
                        self.library[a]
                            .display_title()
                            .cmp(self.library[b].display_title())
                    });
                }
                SortKey::Duration => {
                    indices.sort_by(|&a, &b| {
                        self.library[b]
                            .duration
                            .cmp(&self.library[a].duration)
                    });
                }
            }

            // Filter by search query.
            if search_active {
                indices.retain(|&i| {
                    let t = &self.library[i];
                    t.display_title().to_lowercase().contains(&q_lower)
                        || t.display_artist().to_lowercase().contains(&q_lower)
                        || t.album.to_lowercase().contains(&q_lower)
                });
            }
        }

        // Preserve current_index if possible.
        if let Some(idx) = self.current_index {
            if idx >= indices.len() {
                self.current_index = if indices.is_empty() {
                    None
                } else {
                    Some(0)
                };
            }
        }

        self.filtered.indices = indices;
    }

    /// Set search query and re-filter. Clears any active detail view.
    pub fn set_search(&mut self, query: String) {
        self.filtered.search_query = query;
        self.detail = None;
        self.update_filtered_view();
    }

    /// Set nav item and re-filter. Clears any active detail view and search.
    pub fn set_nav(&mut self, nav: NavItem) {
        self.selected_nav = nav;
        self.detail = None;
        self.filtered.search_query.clear();
        self.update_filtered_view();
    }

    pub fn toggle_play(&mut self) {
        match self.playback_state {
            PlaybackState::Playing => {
                self.playback_state = PlaybackState::Paused;
                if let Some(ref player) = self.player {
                    let _ = player.send(PlayerCommand::Pause);
                }
            }
            PlaybackState::Paused => {
                self.playback_state = PlaybackState::Playing;
                if let Some(ref player) = self.player {
                    let _ = player.send(PlayerCommand::Resume);
                }
            }
            PlaybackState::Stopped => {
                if self.current_track().is_none() && !self.filtered.indices.is_empty() {
                    self.current_index = Some(0);
                }
                self.playback_state = PlaybackState::Playing;
                self.needs_decode = true;
            }
        }
    }

    pub fn next_track(&mut self) {
        let len = self.filtered.indices.len();
        if len == 0 {
            return;
        }
        let next = match self.current_index {
            Some(i) if i + 1 < len => i + 1,
            _ => {
                if self.repeat_mode == RepeatMode::All {
                    0
                } else {
                    return;
                }
            }
        };
        self.current_index = Some(next);
        self.position = 0.0;
        self.playback_state = PlaybackState::Playing;
        self.needs_decode = true;
    }

    pub fn prev_track(&mut self) {
        let len = self.filtered.indices.len();
        if len == 0 {
            return;
        }
        if self.position > 3.0 {
            self.position = 0.0;
            if let Some(ref player) = self.player {
                let _ = player.send(PlayerCommand::Seek(0.0));
            }
            return;
        }
        let prev = match self.current_index {
            Some(i) if i > 0 => i - 1,
            _ => {
                if self.repeat_mode == RepeatMode::All {
                    len - 1
                } else {
                    return;
                }
            }
        };
        self.current_index = Some(prev);
        self.position = 0.0;
        self.playback_state = PlaybackState::Playing;
        self.needs_decode = true;
    }

    /// Play a track at the given filtered index.
    pub fn play_at(&mut self, idx: usize) {
        if idx < self.filtered.indices.len() {
            self.current_index = Some(idx);
            self.position = 0.0;
            self.playback_state = PlaybackState::Playing;
            self.needs_decode = true;
        }
    }

    /// Set the library (from scan results) and update filtered view.
    pub fn set_library(&mut self, tracks: Vec<Arc<Track>>) {
        self.library = tracks;
        self.scanning = false;
        self.update_filtered_view();
    }

    /// Seek to a position in seconds.
    pub fn seek_to(&mut self, pos: f64) {
        self.position = pos.clamp(0.0, self.duration_secs());
        if let Some(ref player) = self.player {
            let _ = player.send(PlayerCommand::Seek(pos));
        }
    }

    /// Set volume (0.0 – 1.0) on both the app state and the audio player.
    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol;
        if vol > 0.0 {
            self.last_volume = vol;
        }
        if let Some(ref player) = self.player {
            let _ = player.send(PlayerCommand::SetVolume(vol));
        }
    }

    /// Toggle between current volume and 0 (mute), restoring last non-zero volume.
    pub fn toggle_mute(&mut self) {
        if self.volume > 0.0 {
            self.set_volume(0.0);
        } else {
            self.set_volume(self.last_volume.max(0.01));
        }
    }
}
