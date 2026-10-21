use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::component::combobox::{ComboboxEvent, ComboboxState};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::slider::{SliderEvent, SliderState};
use gpui_kit::component::{IndexPath, Theme, ThemeMode};
use gpui_kit::*;
use rand::Rng;

use crate::audio::decoder::decode_file;
use crate::audio::library::{find_music_dirs, scan_directories};
use crate::audio::player::{Player, PlayerCommand};
use crate::config::AppConfig;
use crate::db::{Database, PlaylistRow};
use crate::models::playlist::{PlaybackMode, PlaybackState};
use crate::models::track::Track;
use crate::ui::fonts::{FontItem, system_font_items};

/// Minimum interval between config.json writes (see `PlayerApp::flush_config`).
const CONFIG_SAVE_THROTTLE: Duration = Duration::from_millis(750);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavItem {
    AllMusic,
    Artists,
    Albums,
    Playlists,
    Folders,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailTarget {
    Artist(String),
    Album(String),
    Playlist(i64),
    Folder(PathBuf),
}

pub struct PlayerApp {
    /// Full scanned music library.
    pub library: Vec<Arc<Track>>,
    /// Filtered indices matching search query.
    pub filtered_indices: Vec<usize>,
    /// Search query string.
    pub search_query: String,
    /// Search input entity.
    pub search_input: Entity<InputState>,

    /// Currently active sidebar nav item.
    pub selected_nav: NavItem,
    /// Detailed drilled-down view target.
    pub detail_target: Option<DetailTarget>,
    /// Selected track in table (for highlight).
    pub selected_track_id: Option<u64>,

    /// Audio player thread handle.
    pub player: Player,
    /// Playback transport state.
    pub playback_state: PlaybackState,
    /// Playback mode (sequence, shuffle, repeat list, repeat one).
    pub playback_mode: PlaybackMode,
    /// Currently playing or paused track.
    pub current_track: Option<Arc<Track>>,
    /// Playback queue.
    pub queue: Vec<Arc<Track>>,
    /// Current queue index.
    pub queue_index: Option<usize>,

    /// Playback position in seconds.
    pub position: f64,
    /// Current track duration in seconds.
    pub duration: f64,
    /// Volume 0.0 - 1.0.
    pub volume: f32,
    /// Last volume before mute.
    pub last_volume: f32,
    /// Mute state.
    pub is_muted: bool,
    /// Whether user is dragging the seek slider.
    pub seeking: bool,

    /// Progress slider entity (0.0 - 100.0%).
    pub progress_slider: Entity<SliderState>,
    /// Volume slider entity (0.0 - 100.0%).
    pub volume_slider: Entity<SliderState>,

    /// SQLite database for playlists and settings.
    pub db: Arc<Database>,
    /// Cached playlist rows.
    pub playlists: Vec<PlaylistRow>,
    /// Configured music directories.
    pub music_dirs: Vec<PathBuf>,
    /// Whether library scan is in progress.
    pub scanning: bool,
    /// Whether queue drawer is open.
    pub show_queue: bool,

    /// Input state for creating playlist dialog.
    pub create_playlist_input: Option<Entity<InputState>>,
    /// Track being added to a playlist (modal open).
    pub add_to_playlist_track: Option<Arc<Track>>,

    /// Searchable picker listing every installed system font family.
    pub font_picker: Entity<ComboboxState<SearchableVec<FontItem>>>,

    /// Persisted user preferences (`~/.config/caw/config.json`).
    pub config: AppConfig,
    /// Focus handle for the root view (global keyboard shortcuts).
    pub focus_handle: FocusHandle,
    /// Set when `config` changed but the debounced write has not run yet.
    config_dirty: bool,
    /// Last successful config write, used to throttle disk writes.
    last_config_save: Instant,
    /// Bumped on every play request; background decodes from older requests
    /// are discarded so a slow decode cannot override a newer selection.
    play_generation: u64,
    /// Whether the previous session (track/queue/position) has been restored.
    session_restored: bool,
}

impl PlayerApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = AppConfig::load();

        // Enumerate installed fonts once; the settings picker searches this list.
        let font_items = system_font_items(cx, &config.font_family);
        crate::log_debug!(
            "loaded {} system font families (current: {})",
            font_items.len(),
            config.font_family
        );
        let selected_font_row = font_items
            .iter()
            .position(|item| item.family().as_ref() == config.font_family)
            .unwrap_or(0);
        let font_picker = cx.new(|cx| {
            ComboboxState::new(
                SearchableVec::new(font_items),
                vec![IndexPath::new(selected_font_row)],
                window,
                cx,
            )
            .searchable(true)
        });
        cx.subscribe(
            &font_picker,
            |this, _state, event: &ComboboxEvent<SearchableVec<FontItem>>, cx| {
                let values = match event {
                    ComboboxEvent::Change(values) | ComboboxEvent::Confirm(values) => values,
                };
                if let Some(family) = values.first().cloned()
                    && this.config.font_family != family.as_ref()
                {
                    this.set_font_family(family, cx);
                }
            },
        )
        .detach();

        let db = Arc::new(Database::open());
        let playlists = db.list_playlists();
        let mut music_dirs = db.get_music_dirs();
        if music_dirs.is_empty() {
            music_dirs = find_music_dirs();
            for d in &music_dirs {
                let _ = db.add_music_dir(d);
            }
        }

        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("搜索歌曲、歌手、专辑...")
                .clean_on_escape()
        });

        let progress_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .step(0.1)
                .default_value(0.0)
        });

        let initial_volume = config.volume;
        let volume_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(initial_volume * 100.0)
        });

        // Subscribe to search input changes
        cx.subscribe(&search_input, |this, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                // `InputState::value()` returns the current text as a SharedString.
                let text = this.search_input.read(cx).value();
                this.search_query = text.to_string();
                this.apply_filter();
                cx.notify();
            }
        })
        .detach();

        // Subscribe to progress slider drag/change
        cx.subscribe(
            &progress_slider,
            |this, _, event: &SliderEvent, cx| match event {
                SliderEvent::Change(val) => {
                    this.seeking = true;
                    let pct = (val.start() / 100.0).clamp(0.0, 1.0);
                    this.position = (pct as f64) * this.duration;
                    cx.notify();
                }
                SliderEvent::Release(val) => {
                    this.seeking = false;
                    let pct = (val.start() / 100.0).clamp(0.0, 1.0);
                    let pos = (pct as f64) * this.duration;
                    this.seek_to(pos, cx);
                }
            },
        )
        .detach();

        // Subscribe to volume slider
        cx.subscribe(&volume_slider, |this, _, event: &SliderEvent, cx| {
            let val = match event {
                SliderEvent::Change(v) | SliderEvent::Release(v) => v,
            };
            let vol = (val.start() / 100.0).clamp(0.0, 1.0);
            this.set_volume(vol, cx);
        })
        .detach();

        let mut app = Self {
            library: Vec::new(),
            filtered_indices: Vec::new(),
            search_query: String::new(),
            search_input,
            selected_nav: NavItem::AllMusic,
            detail_target: None,
            selected_track_id: None,
            player: Player::new(),
            playback_state: PlaybackState::Stopped,
            playback_mode: config.playback_mode,
            current_track: None,
            queue: Vec::new(),
            queue_index: None,
            position: 0.0,
            duration: 0.0,
            volume: config.volume,
            last_volume: if config.volume > 0.0 {
                config.volume
            } else {
                crate::config::DEFAULT_VOLUME
            },
            is_muted: config.volume == 0.0,
            seeking: false,
            progress_slider,
            volume_slider,
            db,
            playlists,
            music_dirs,
            scanning: false,
            show_queue: false,
            create_playlist_input: None,
            add_to_playlist_track: None,
            font_picker,
            config,
            focus_handle: cx.focus_handle(),
            config_dirty: false,
            last_config_save: Instant::now() - Duration::from_secs(1),
            play_generation: 0,
            session_restored: false,
        };

        // Push the persisted volume into the audio engine before the first play.
        let volume = app.config.volume;
        let _ = app.player.send(PlayerCommand::SetVolume(volume));

        // Trigger initial library scan
        app.rescan(cx);

        app
    }

    /// Refresh library by scanning configured directories in background.
    pub fn rescan(&mut self, cx: &mut Context<Self>) {
        if self.scanning {
            return;
        }
        self.scanning = true;
        cx.notify();

        let dirs = self.music_dirs.clone();
        cx.spawn(async move |entity, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { scan_directories(&dirs) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                this.scanning = false;
                if let Ok(tracks) = result {
                    crate::log_info!("scan complete, found {} tracks", tracks.len());
                    this.library = tracks;
                    this.apply_filter();
                    this.config.last_scan_time = Some(crate::config::now_unix());
                    this.config_dirty = true;
                    // First scan of this launch: restore the previous session, paused.
                    if !this.session_restored && this.current_track.is_none() {
                        this.restore_session(cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Re-calculate filtered indices based on search query.
    pub fn apply_filter(&mut self) {
        if self.search_query.trim().is_empty() {
            self.filtered_indices = (0..self.library.len()).collect();
        } else {
            let q = self.search_query.trim().to_lowercase();
            self.filtered_indices = self
                .library
                .iter()
                .enumerate()
                .filter(|(_, t)| {
                    t.title.to_lowercase().contains(&q)
                        || t.artist.to_lowercase().contains(&q)
                        || t.album.to_lowercase().contains(&q)
                })
                .map(|(i, _)| i)
                .collect();
        }
    }

    /// Switch active nav section.
    pub fn select_nav(&mut self, nav: NavItem, cx: &mut Context<Self>) {
        self.selected_nav = nav;
        self.detail_target = None;
        cx.notify();
    }

    /// Open detail view (e.g. artist, album, playlist, folder).
    pub fn open_detail(&mut self, target: DetailTarget, cx: &mut Context<Self>) {
        self.detail_target = Some(target);
        cx.notify();
    }

    /// Back to list view.
    pub fn back_to_list(&mut self, cx: &mut Context<Self>) {
        self.detail_target = None;
        cx.notify();
    }

    /// Play a track directly, creating a queue from surrounding tracks.
    pub fn play_track(&mut self, track: Arc<Track>, cx: &mut Context<Self>) {
        let queue = vec![track.clone()];
        self.play_all(queue, 0, cx);
    }

    /// Play all tracks in a list starting from a given index.
    pub fn play_all(
        &mut self,
        tracks: Vec<Arc<Track>>,
        start_index: usize,
        cx: &mut Context<Self>,
    ) {
        if tracks.is_empty() || start_index >= tracks.len() {
            return;
        }

        self.queue = tracks;
        self.queue_index = Some(start_index);
        self.play_current_queue(cx);
    }

    /// Decode and play the current queue index from the start.
    fn play_current_queue(&mut self, cx: &mut Context<Self>) {
        self.play_current_queue_from(0.0, cx);
    }

    /// Decode and play the current queue index starting at `start_pos` seconds.
    ///
    /// Decoding happens on the background executor; the UI thread only swaps
    /// state. A generation counter drops decodes that a newer play request has
    /// already superseded.
    fn play_current_queue_from(&mut self, start_pos: f64, cx: &mut Context<Self>) {
        let Some(idx) = self.queue_index else { return };
        let Some(track) = self.queue.get(idx).cloned() else {
            return;
        };

        let previous = self.current_track.clone();
        self.current_track = Some(track.clone());
        self.duration = track.duration.as_secs_f64();
        self.position = start_pos.clamp(0.0, self.duration);
        self.playback_state = PlaybackState::Playing;
        self.seeking = false;
        self.play_generation = self.play_generation.wrapping_add(1);
        let generation = self.play_generation;
        self.persist_session();

        // Silence the previous track and clear the frame counters right away.
        // While the new track decodes, the position poller must not mistake the
        // stale counters of the finished track for another natural end.
        let _ = self.player.send(PlayerCommand::Stop);

        cx.notify();

        let path = track.path.clone();
        cx.spawn(async move |entity, cx| {
            let decode_path = path.clone();
            let decoded_result = cx
                .background_executor()
                .spawn(async move { decode_file(&decode_path) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                if this.play_generation != generation {
                    // A newer play request won; discard this decode.
                    return;
                }
                match decoded_result {
                    Ok(decoded) => {
                        let _ = this.player.send(PlayerCommand::Play(decoded, start_pos));
                        // Only tracks that actually started playing enter the history.
                        if let Some(previous) = previous
                            && previous.path != path
                        {
                            this.push_history(&previous);
                        }
                        // The user may have hit pause while the decode ran.
                        if this.playback_state == PlaybackState::Paused {
                            let _ = this.player.send(PlayerCommand::Pause);
                        }
                    }
                    Err(e) => {
                        crate::log_warn!("failed to decode {:?}: {}", path, e);
                        this.next_track(cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Toggle Play / Pause transport.
    pub fn toggle_play_pause(&mut self, cx: &mut Context<Self>) {
        match self.playback_state {
            PlaybackState::Playing => {
                let _ = self.player.send(PlayerCommand::Pause);
                self.playback_state = PlaybackState::Paused;
                self.persist_session();
                cx.notify();
            }
            PlaybackState::Paused => {
                let has_stream = self
                    .player
                    .state
                    .total_frames
                    .load(std::sync::atomic::Ordering::Relaxed)
                    > 0;
                if has_stream {
                    let _ = self.player.send(PlayerCommand::Resume);
                    self.playback_state = PlaybackState::Playing;
                } else {
                    // Restored session (or an earlier failed decode): the engine
                    // holds no audio, so decode at the saved position instead of
                    // resuming silence.
                    self.play_current_queue_from(self.position, cx);
                }
                cx.notify();
            }
            PlaybackState::Stopped => {
                if self.current_track.is_some() {
                    self.play_current_queue(cx);
                } else if !self.library.is_empty() {
                    let tracks = self.library.clone();
                    self.play_all(tracks, 0, cx);
                }
            }
        }
    }

    /// Advance to next track.
    pub fn next_track(&mut self, cx: &mut Context<Self>) {
        if self.queue.is_empty() {
            return;
        }

        match self.playback_mode {
            PlaybackMode::RepeatOne => {
                self.play_current_queue(cx);
            }
            PlaybackMode::Shuffle => {
                let len = self.queue.len();
                if len == 1 {
                    self.play_current_queue(cx);
                } else {
                    let current = self.queue_index.unwrap_or(0);
                    let mut rng = rand::rng();
                    let mut next = rng.random_range(0..len);
                    if next == current {
                        next = (next + 1) % len;
                    }
                    self.queue_index = Some(next);
                    self.play_current_queue(cx);
                }
            }
            PlaybackMode::RepeatList => {
                let next = (self.queue_index.unwrap_or(0) + 1) % self.queue.len();
                self.queue_index = Some(next);
                self.play_current_queue(cx);
            }
            PlaybackMode::Sequence => {
                let current = self.queue_index.unwrap_or(0);
                if current + 1 < self.queue.len() {
                    self.queue_index = Some(current + 1);
                    self.play_current_queue(cx);
                } else {
                    let _ = self.player.send(PlayerCommand::Stop);
                    self.playback_state = PlaybackState::Stopped;
                    self.position = 0.0;
                    self.persist_session();
                    cx.notify();
                }
            }
        }
    }

    /// Return to previous track or beginning of song.
    pub fn prev_track(&mut self, cx: &mut Context<Self>) {
        if self.queue.is_empty() {
            return;
        }

        if self.position > 3.0 {
            self.seek_to(0.0, cx);
            return;
        }

        let current = self.queue_index.unwrap_or(0);
        if current > 0 {
            self.queue_index = Some(current - 1);
            self.play_current_queue(cx);
        } else if self.playback_mode == PlaybackMode::RepeatList {
            self.queue_index = Some(self.queue.len() - 1);
            self.play_current_queue(cx);
        } else {
            self.seek_to(0.0, cx);
        }
    }

    /// Seek to position in seconds.
    pub fn seek_to(&mut self, pos: f64, cx: &mut Context<Self>) {
        let pos = pos.clamp(0.0, self.duration);
        let _ = self.player.send(PlayerCommand::Seek(pos));
        self.position = pos;
        self.persist_session();
        cx.notify();
    }

    /// Set volume 0.0 - 1.0.
    pub fn set_volume(&mut self, vol: f32, cx: &mut Context<Self>) {
        let vol = vol.clamp(0.0, 1.0);
        self.volume = vol;
        self.is_muted = vol == 0.0;
        let _ = self.player.send(PlayerCommand::SetVolume(vol));
        self.update_config(|config| config.volume = vol);
        cx.notify();
    }

    /// Toggle mute/unmute.
    pub fn toggle_mute(&mut self, cx: &mut Context<Self>) {
        if self.is_muted {
            let restored = if self.last_volume > 0.0 {
                self.last_volume
            } else {
                crate::config::DEFAULT_VOLUME
            };
            self.set_volume(restored, cx);
        } else {
            self.last_volume = self.volume;
            self.set_volume(0.0, cx);
        }
    }

    /// Cycle playback mode.
    pub fn cycle_playback_mode(&mut self, cx: &mut Context<Self>) {
        let mode = self.playback_mode.next();
        self.playback_mode = mode;
        self.update_config(|config| config.playback_mode = mode);
        cx.notify();
    }

    /// Periodic position update called by the UI timer loop (~250ms).
    ///
    /// Also flushes dirty preferences, so config writes ride on the same tick.
    pub fn update_position(&mut self, cx: &mut Context<Self>) {
        self.flush_config(cx);

        if self.playback_state != PlaybackState::Playing {
            return;
        }

        use std::sync::atomic::Ordering::Relaxed;

        let is_playing = self.player.state.is_playing.load(Relaxed);
        let pos_frames = self.player.state.position_frames.load(Relaxed);
        let total_frames = self.player.state.total_frames.load(Relaxed);

        if total_frames > 0 && self.duration > 0.0 {
            let pos = (pos_frames as f64 / total_frames as f64) * self.duration;
            if !self.seeking {
                self.position = pos;
            }

            // Natural end: the audio thread consumed every frame and flipped
            // `is_playing` off. Frame counters are cleared when a new play
            // starts, so an in-flight decode cannot trigger this twice.
            if !is_playing && pos_frames >= total_frames {
                self.next_track(cx);
                return;
            }
        }

        cx.notify();
    }

    /// Toggle queue drawer.
    pub fn toggle_queue(&mut self, cx: &mut Context<Self>) {
        self.show_queue = !self.show_queue;
        cx.notify();
    }

    /// Play track at specific index in queue.
    pub fn play_queue_index(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.queue.len() {
            self.queue_index = Some(idx);
            self.play_current_queue(cx);
        }
    }

    /// Clear current queue.
    pub fn clear_queue(&mut self, cx: &mut Context<Self>) {
        let _ = self.player.send(PlayerCommand::Stop);
        self.playback_state = PlaybackState::Stopped;
        self.queue.clear();
        self.queue_index = None;
        self.current_track = None;
        self.position = 0.0;
        self.duration = 0.0;
        self.persist_session();
        cx.notify();
    }

    // ── Preferences / Config ──

    /// Apply a theme mode and persist it.
    ///
    /// gpui-kit has no "follow system" mode: callers resolve the OS appearance
    /// to `ThemeMode::Light` or `ThemeMode::Dark` first.
    #[allow(dead_code)] // wired by the settings view (P2)
    pub fn set_theme_mode(&mut self, mode: ThemeMode, cx: &mut Context<Self>) {
        self.update_config(|config| config.theme_mode = mode);
        Theme::change(mode, None, cx);
        self.flush_config(cx);
    }

    /// Apply a UI font family and persist it.
    #[allow(dead_code)] // wired by the settings view (P2)
    pub fn set_font_family(&mut self, family: SharedString, cx: &mut Context<Self>) {
        self.update_config(|config| config.font_family = family.to_string());
        let family = family.clone();
        Theme::update(cx, |theme| {
            theme.font_family = family;
        });
        self.flush_config(cx);
    }

    /// Hide to the tray instead of quitting when the window closes.
    #[allow(dead_code)] // wired by the settings view (P2)
    pub fn set_close_to_tray(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.update_config(|config| config.close_to_tray = enabled);
        cx.notify();
    }

    /// Mutate persisted preferences.
    ///
    /// Only marks the config dirty; `flush_config` (driven by the 250ms tick)
    /// performs the actual write.
    pub fn update_config(&mut self, f: impl FnOnce(&mut AppConfig)) {
        f(&mut self.config);
        self.config_dirty = true;
    }

    /// Persist dirty preferences, throttled to one write per
    /// `CONFIG_SAVE_THROTTLE`. The write itself runs on the background executor.
    ///
    /// Call this once more on shutdown for a final flush.
    pub fn flush_config(&mut self, cx: &mut Context<Self>) {
        if !self.config_dirty {
            return;
        }
        let now = Instant::now();
        if now.duration_since(self.last_config_save) < CONFIG_SAVE_THROTTLE {
            return;
        }
        self.config_dirty = false;
        self.last_config_save = now;

        let config = self.config.clone();
        cx.background_executor()
            .spawn(async move {
                if let Err(err) = config.save() {
                    crate::log_error!("failed to save config: {}", err);
                }
            })
            .detach();
    }

    /// Resolve a playlist's track ids against the current library.
    #[allow(dead_code)] // wired by the playlist detail view (P2)
    pub fn playlist_tracks(&self, id: i64) -> Vec<Arc<Track>> {
        let Some(playlist) = self.db.get_playlist(id) else {
            return Vec::new();
        };
        let by_id: std::collections::HashMap<u64, Arc<Track>> = self
            .library
            .iter()
            .map(|track| (track.id, track.clone()))
            .collect();
        playlist
            .track_ids
            .iter()
            .filter_map(|track_id| by_id.get(&(*track_id as u64)).cloned())
            .collect()
    }

    // ── Playlist Management ──

    pub fn load_playlists(&mut self) {
        self.playlists = self.db.list_playlists();
    }

    pub fn create_playlist(&mut self, name: &str, cx: &mut Context<Self>) {
        let name = name.trim();
        if !name.is_empty() {
            let _ = self.db.create_playlist(name);
            self.load_playlists();
            self.create_playlist_input = None;
            cx.notify();
        }
    }

    pub fn delete_playlist(&mut self, id: i64, cx: &mut Context<Self>) {
        let _ = self.db.delete_playlist(id);
        self.load_playlists();
        if let Some(DetailTarget::Playlist(cur_id)) = self.detail_target
            && cur_id == id
        {
            self.detail_target = None;
        }
        cx.notify();
    }

    pub fn add_track_to_playlist(
        &mut self,
        playlist_id: i64,
        track_id: u64,
        cx: &mut Context<Self>,
    ) {
        let _ = self.db.add_tracks(playlist_id, &[track_id as i64]);
        self.load_playlists();
        self.add_to_playlist_track = None;
        cx.notify();
    }

    #[allow(dead_code)] // wired by the playlist detail view (P2)
    pub fn remove_track_from_playlist(
        &mut self,
        playlist_id: i64,
        track_id: u64,
        cx: &mut Context<Self>,
    ) {
        let _ = self.db.remove_tracks(playlist_id, &[track_id as i64]);
        self.load_playlists();
        cx.notify();
    }

    // ── Session restore / play history (P3) ──

    /// Snapshot the transport and queue state into `config` (dirty flag set; the
    /// actual write is debounced by `flush_config`).
    fn persist_session(&mut self) {
        self.config.last_track_path = self
            .current_track
            .as_ref()
            .map(|track| track.path.to_string_lossy().into_owned());
        self.config.last_queue_paths = self
            .queue
            .iter()
            .map(|track| track.path.to_string_lossy().into_owned())
            .collect();
        self.config.last_queue_index = self.queue_index;
        self.config.last_position = self.position;
        self.config_dirty = true;
    }

    /// Push a track onto the bounded play history (most recent first).
    fn push_history(&mut self, track: &Track) {
        let path = track.path.to_string_lossy().into_owned();
        if self.config.history_paths.first() != Some(&path) {
            self.config.history_paths.insert(0, path);
            self.config
                .history_paths
                .truncate(crate::config::MAX_HISTORY);
            self.config_dirty = true;
        }
    }

    /// Restore the previous session as paused: current track, queue, queue index
    /// and position. Never auto-plays; pressing play decodes at the saved position.
    fn restore_session(&mut self, cx: &mut Context<Self>) {
        self.session_restored = true;

        let Some(saved_path) = self.config.last_track_path.clone() else {
            return;
        };

        fn find_track(library: &[Arc<Track>], path: &str) -> Option<Arc<Track>> {
            library
                .iter()
                .find(|track| track.path.to_string_lossy() == path)
                .cloned()
        }

        let Some(current) = find_track(&self.library, &saved_path) else {
            crate::log_warn!(
                "cannot restore session, last track is not in the library: {}",
                saved_path
            );
            return;
        };

        let mut queue: Vec<Arc<Track>> = self
            .config
            .last_queue_paths
            .iter()
            .filter_map(|path| find_track(&self.library, path))
            .collect();
        if queue.is_empty() {
            queue = self.library.clone();
        }

        let index = match self.config.last_queue_index {
            Some(index)
                if queue
                    .get(index)
                    .map(|track| track.path == current.path)
                    .unwrap_or(false) =>
            {
                index
            }
            _ => queue
                .iter()
                .position(|track| track.path == current.path)
                .unwrap_or_else(|| {
                    queue.insert(0, current.clone());
                    0
                }),
        };

        self.queue = queue;
        self.queue_index = Some(index);
        self.duration = current.duration.as_secs_f64();
        self.position = self.config.last_position.clamp(0.0, self.duration);
        self.current_track = Some(current);
        self.playback_state = PlaybackState::Paused;
        crate::log_info!(
            "restored session at {:.1}s ({} queue items)",
            self.position,
            self.queue.len()
        );
        cx.notify();
    }

    /// Recently played tracks, most recent first.
    #[allow(dead_code)] // wired by the queue drawer history tab (P3)
    pub fn history_tracks(&self) -> Vec<Arc<Track>> {
        let by_path: std::collections::HashMap<&Path, Arc<Track>> = self
            .library
            .iter()
            .map(|track| (track.path.as_path(), track.clone()))
            .collect();
        self.config
            .history_paths
            .iter()
            .filter_map(|path| by_path.get(Path::new(path)).cloned())
            .collect()
    }

    /// Play the history entry at `idx` (0 = most recent).
    ///
    /// Matches the old UI: a history click starts a single-track queue.
    #[allow(dead_code)] // wired by the queue drawer history tab (P3)
    pub fn play_history_index(&mut self, idx: usize, cx: &mut Context<Self>) {
        let Some(path) = self.config.history_paths.get(idx).cloned() else {
            return;
        };
        let Some(track) = self
            .library
            .iter()
            .find(|track| track.path.as_path() == Path::new(&path))
            .cloned()
        else {
            return;
        };
        self.play_track(track, cx);
    }

    /// Clear the play history.
    #[allow(dead_code)] // wired by the queue drawer history tab (P3)
    pub fn clear_history(&mut self, cx: &mut Context<Self>) {
        self.config.history_paths.clear();
        self.config_dirty = true;
        self.flush_config(cx);
        cx.notify();
    }

    /// Create a playlist containing the current queue, in order.
    #[allow(dead_code)] // wired by the queue drawer (P3)
    pub fn save_queue_as_playlist(&mut self, name: &str, cx: &mut Context<Self>) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let playlist_id = match self.db.create_playlist(name) {
            Ok(id) => id,
            Err(err) => {
                crate::log_error!("failed to create playlist '{}': {}", name, err);
                return;
            }
        };
        let track_ids: Vec<i64> = self.queue.iter().map(|track| track.id as i64).collect();
        if !track_ids.is_empty()
            && let Err(err) = self.db.add_tracks(playlist_id, &track_ids)
        {
            crate::log_error!("failed to fill playlist '{}': {}", name, err);
        }
        self.load_playlists();
        cx.notify();
    }

    // ── Music Folders Management ──

    /// Open the native folder picker (xdg-desktop-portal on Linux) and add the result.
    pub fn pick_music_folder(&mut self, cx: &mut Context<Self>) {
        crate::log_debug!("folder picker: opening native dialog");
        cx.spawn(async move |entity, cx| {
            let folder = rfd::AsyncFileDialog::new()
                .set_title("选择音乐文件夹")
                .pick_folder()
                .await;
            let Some(folder) = folder else {
                crate::log_debug!("folder picker: cancelled or unavailable");
                return;
            };
            let path = folder.path().to_path_buf();
            crate::log_debug!("folder picker: chosen {:?}", path);
            let _ = entity.update(cx, |app, cx| app.add_music_dir(path, cx));
        })
        .detach();
    }

    pub fn add_music_dir(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !self.music_dirs.contains(&path) {
            let _ = self.db.add_music_dir(&path);
            self.music_dirs.push(path);
            self.rescan(cx);
        }
    }

    pub fn remove_music_dir(&mut self, path: &Path, cx: &mut Context<Self>) {
        let _ = self.db.remove_music_dir(path);
        self.music_dirs.retain(|p| p != path);
        self.rescan(cx);
    }
}

impl Focusable for PlayerApp {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Drop for PlayerApp {
    fn drop(&mut self) {
        // Final flush on shutdown: capture the exact transport position and any
        // change made inside the write-throttle window. The file is tiny (a few
        // hundred bytes) and this runs during teardown, not on a hot path.
        self.persist_session();
        if let Err(err) = self.config.save() {
            crate::log_error!("failed to save config on exit: {}", err);
        }
    }
}
