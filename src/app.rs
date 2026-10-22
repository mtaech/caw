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
use crate::lyrics::Lyrics;
use crate::models::playlist::{PlaybackMode, PlaybackState};
use crate::models::track::Track;
use crate::tags::EditableTags;
use crate::ui::fonts::{FontItem, system_font_items};

/// Minimum interval between config.json writes (see `PlayerApp::flush_config`).
const CONFIG_SAVE_THROTTLE: Duration = Duration::from_millis(750);

/// Height of one row in the shared track table, in logical pixels.
///
/// The virtualized list needs a uniform row height, and the "locate now
/// playing" visibility check converts pixels back into row indexes.
pub const LIST_ROW_HEIGHT: f32 = 38.0;

/// Distance from the top of the folder scroll area to the first row.
const FIRST_ROW_TOP_FOLDER: f32 = 104.0;
/// Folder rows are natural-height rows with an 8px gap between them.
const FOLDER_ROW_PITCH: f32 = 44.0;

/// Where the now-playing row sits inside the content scroll area.
struct RowMetrics {
    index: usize,
    top: f32,
    pitch: f32,
}

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

/// Inputs and status of the metadata editor modal.
///
/// Every editable field is its own `InputState`; `track_id`/`path` freeze the
/// target so a list re-sort or filter change cannot retarget the save.
pub struct TagEditState {
    pub track_id: u64,
    pub path: PathBuf,
    pub title: Entity<InputState>,
    pub artist: Entity<InputState>,
    pub album: Entity<InputState>,
    pub album_artist: Entity<InputState>,
    pub genre: Entity<InputState>,
    pub year: Entity<InputState>,
    pub track_number: Entity<InputState>,
    pub disc_number: Entity<InputState>,
    /// Shown inside the modal; set on read/save failures.
    pub error: Option<String>,
    /// A write is in flight; the modal disables its buttons.
    pub saving: bool,
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
    /// Scroll handle of the content area, used by grid/settings/folder views.
    pub content_scroll_handle: ScrollHandle,
    /// Scroll position of the virtualized track list (All Music and details).
    pub track_list_scroll_handle: UniformListScrollHandle,
    /// Scroll position of the virtualized queue drawer list.
    pub queue_scroll_handle: UniformListScrollHandle,
    /// Scroll position of the recent-history section in the queue drawer.
    pub queue_history_scroll_handle: ScrollHandle,

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

    /// Whether the content area shows the now-playing view (large cover + lyrics).
    pub show_now_playing: bool,
    /// Parsed lyrics for `current_track`, loaded off the UI thread.
    pub lyrics: Option<Arc<Lyrics>>,
    /// Track id the loaded lyrics belong to; guards against a late load winning.
    lyrics_track_id: Option<u64>,
    /// Whether a lyrics load is currently in flight.
    pub lyrics_loading: bool,
    /// Highlighted line index in `lyrics` (synced lyrics only).
    pub lyrics_active_line: Option<usize>,
    /// Scroll offset owner of the lyrics column.
    pub lyrics_scroll_handle: ScrollHandle,
    /// Last active line the lyrics column auto-scrolled to.
    lyrics_scrolled_line: Option<usize>,
    /// Tracks already looked up for online cover art this session.
    cover_attempts: std::collections::HashSet<u64>,

    /// Input state for creating playlist dialog.
    pub create_playlist_input: Option<Entity<InputState>>,
    /// Track being added to a playlist (modal open).
    pub add_to_playlist_track: Option<Arc<Track>>,
    /// Open tag/metadata editor (modal), if any.
    pub tag_edit: Option<TagEditState>,

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
    /// Last periodic session save, used to keep `config.last_position` fresh.
    last_session_save: Instant,
}

/// Fresh single-line input for the metadata editor.
///
/// Deliberately no `clean_on_escape`: Escape must bubble to the root view and
/// close the modal instead of silently clearing a field.
fn tag_input(window: &mut Window, cx: &mut Context<PlayerApp>) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx))
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
            content_scroll_handle: ScrollHandle::new(),
            track_list_scroll_handle: UniformListScrollHandle::new(),
            queue_scroll_handle: UniformListScrollHandle::new(),
            queue_history_scroll_handle: ScrollHandle::new(),
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
            show_now_playing: false,
            lyrics: None,
            lyrics_track_id: None,
            lyrics_loading: false,
            lyrics_active_line: None,
            lyrics_scroll_handle: ScrollHandle::new(),
            lyrics_scrolled_line: None,
            cover_attempts: std::collections::HashSet::new(),
            create_playlist_input: None,
            add_to_playlist_track: None,
            tag_edit: None,
            font_picker,
            config,
            focus_handle: cx.focus_handle(),
            config_dirty: false,
            last_config_save: Instant::now() - Duration::from_secs(1),
            play_generation: 0,
            session_restored: false,
            last_session_save: Instant::now(),
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
        self.filtered_indices = self.compute_filter();

        // A shorter result set must not leave the virtual list scrolled past
        // its end.
        self.track_list_scroll_handle
            .scroll_to_item(0, ScrollStrategy::Top);
    }

    /// Indices into `library` matching the current search query.
    fn compute_filter(&self) -> Vec<usize> {
        if self.search_query.trim().is_empty() {
            (0..self.library.len()).collect()
        } else {
            let query = self.search_query.trim().to_lowercase();
            self.library
                .iter()
                .enumerate()
                .filter(|(_, track)| {
                    track.title.to_lowercase().contains(&query)
                        || track.artist.to_lowercase().contains(&query)
                        || track.album.to_lowercase().contains(&query)
                })
                .map(|(index, _)| index)
                .collect()
        }
    }

    /// Switch active nav section.
    pub fn select_nav(&mut self, nav: NavItem, cx: &mut Context<Self>) {
        self.selected_nav = nav;
        self.detail_target = None;
        self.show_now_playing = false;
        self.track_list_scroll_handle
            .scroll_to_item(0, ScrollStrategy::Top);
        cx.notify();
    }

    /// Open detail view (e.g. artist, album, playlist, folder).
    pub fn open_detail(&mut self, target: DetailTarget, cx: &mut Context<Self>) {
        self.detail_target = Some(target);
        self.show_now_playing = false;
        self.track_list_scroll_handle
            .scroll_to_item(0, ScrollStrategy::Top);
        cx.notify();
    }

    /// Back to list view.
    pub fn back_to_list(&mut self, cx: &mut Context<Self>) {
        self.detail_target = None;
        self.track_list_scroll_handle
            .scroll_to_item(0, ScrollStrategy::Top);
        cx.notify();
    }

    /// Scroll the current view so the now-playing track is visible.
    ///
    /// If the active view cannot show the track (artist/album/folder grid,
    /// playlists list, settings), navigate to a view that can first. The search
    /// filter is cleared when it hides the current track.
    pub fn locate_now_playing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(current) = self.current_track.clone() else {
            return;
        };

        // Locating means showing the list row, so leave the now-playing view.
        self.close_now_playing(cx);

        if self.detail_target.is_none() {
            match self.selected_nav {
                NavItem::AllMusic => {}
                NavItem::Artists => {
                    self.detail_target =
                        Some(DetailTarget::Artist(current.display_artist().to_string()));
                }
                NavItem::Albums => {
                    self.detail_target =
                        Some(DetailTarget::Album(current.display_album().to_string()));
                }
                NavItem::Folders => {
                    if let Some(parent) = current.path.parent() {
                        self.detail_target = Some(DetailTarget::Folder(parent.to_path_buf()));
                    }
                }
                NavItem::Playlists | NavItem::Settings => {
                    self.selected_nav = NavItem::AllMusic;
                }
            }
        }

        // A search filter may hide the track; clear it so the row exists.
        if matches!(self.selected_nav, NavItem::AllMusic)
            && self.detail_target.is_none()
            && !self
                .filtered_indices
                .iter()
                .any(|&i| self.library.get(i).map(|track| track.id) == Some(current.id))
        {
            self.search_query.clear();
            self.search_input.update(cx, |input, cx| {
                input.set_value("", window, cx);
            });
            self.apply_filter();
        }

        if self.is_track_list_view() {
            let Some(row) = self.visible_row_of(current.id) else {
                crate::log_debug!("locate: now-playing track is not in the active list");
                return;
            };
            crate::log_debug!(
                "locate: track '{}' at row {}, scrolling virtual list",
                current.display_title(),
                row
            );
            self.track_list_scroll_handle
                .scroll_to_item_with_offset(row, ScrollStrategy::Top, 1);
            cx.notify();
            return;
        }

        let Some(metrics) = self.current_row_metrics() else {
            crate::log_debug!("locate: now-playing track is not in the active view");
            return;
        };

        // Folder rows are natural-height; leave a row and a half above the
        // target so it does not sit flush against the top edge.
        let offset = (metrics.top - metrics.pitch * 1.5).max(0.0);
        crate::log_debug!(
            "locate: track '{}' at row {}, scroll offset {:.0}px",
            current.display_title(),
            metrics.index,
            offset
        );
        self.content_scroll_handle
            .set_offset(point(px(0.), px(-offset)));
        cx.notify();
    }

    /// Whether the active content view renders the virtualized track table.
    pub fn is_track_list_view(&self) -> bool {
        match self.selected_nav {
            NavItem::AllMusic => true,
            NavItem::Artists | NavItem::Albums | NavItem::Playlists => self.detail_target.is_some(),
            NavItem::Folders | NavItem::Settings => false,
        }
    }

    /// Whether the now-playing row is currently inside the content viewport.
    ///
    /// Returns `true` when the active view has no row for the track (so callers
    /// do not offer a locate affordance that cannot do anything) or before the
    /// scroll area has been laid out.
    pub fn now_playing_row_visible(&self) -> bool {
        if self.is_track_list_view() {
            let Some(current) = self.current_track.as_ref() else {
                return true;
            };
            let Some(row) = self.visible_row_of(current.id) else {
                return true;
            };

            let handle = self.track_list_scroll_handle.0.borrow().base_handle.clone();
            let bounds = handle.bounds();
            if bounds.size.height <= px(1.0) || bounds.size.width <= px(1.0) {
                return true;
            }
            let first = ((-handle.offset().y).as_f32() / LIST_ROW_HEIGHT)
                .floor()
                .max(0.0) as usize;
            let visible = (bounds.size.height.as_f32() / LIST_ROW_HEIGHT).ceil() as usize + 1;
            return row >= first && row < first + visible;
        }

        let Some(metrics) = self.current_row_metrics() else {
            return true;
        };
        let bounds = self.content_scroll_handle.bounds();
        if bounds.size.height <= px(1.0) || bounds.size.width <= px(1.0) {
            return true;
        }

        let view_top = -self.content_scroll_handle.offset().y;
        let view_bottom = view_top + bounds.size.height;
        let row_top = px(metrics.top);
        let row_bottom = row_top + px(metrics.pitch);

        row_bottom > view_top + px(2.0) && row_top < view_bottom - px(2.0)
    }

    /// Geometry of a folder row inside the outer content scroll area.
    fn current_row_metrics(&self) -> Option<RowMetrics> {
        let current = self.current_track.as_ref()?;
        if !matches!(
            (&self.selected_nav, &self.detail_target),
            (NavItem::Folders, Some(DetailTarget::Folder(_)))
        ) {
            return None;
        }
        let index = self.visible_row_of(current.id)?;
        Some(RowMetrics {
            index,
            top: FIRST_ROW_TOP_FOLDER + index as f32 * FOLDER_ROW_PITCH,
            pitch: FOLDER_ROW_PITCH,
        })
    }

    /// Row index of `track_id` in the list rendered by the active view.
    fn visible_row_of(&self, track_id: u64) -> Option<usize> {
        let current_path = self.current_track.as_ref().map(|track| track.path.clone());
        match (&self.selected_nav, &self.detail_target) {
            (NavItem::AllMusic, _) => self
                .filtered_indices
                .iter()
                .position(|&i| self.library.get(i).map(|track| track.id) == Some(track_id)),
            (NavItem::Artists, Some(DetailTarget::Artist(name))) => self
                .library
                .iter()
                .filter(|track| track.display_artist() == name)
                .position(|track| track.id == track_id),
            (NavItem::Albums, Some(DetailTarget::Album(name))) => {
                let mut indices: Vec<usize> = self
                    .library
                    .iter()
                    .enumerate()
                    .filter(|(_, track)| track.display_album() == name)
                    .map(|(index, _)| index)
                    .collect();
                indices.sort_by_key(|&index| self.library[index].track_number);
                indices
                    .iter()
                    .position(|&index| self.library.get(index).map(|t| t.id) == Some(track_id))
            }
            (NavItem::Playlists, Some(DetailTarget::Playlist(playlist_id))) => {
                let playlist = self.db.get_playlist(*playlist_id)?;
                let ids: std::collections::HashSet<i64> = playlist.track_ids.into_iter().collect();
                self.library
                    .iter()
                    .filter(|track| ids.contains(&(track.id as i64)))
                    .position(|track| track.id == track_id)
            }
            (NavItem::Folders, Some(DetailTarget::Folder(dir))) => {
                // Matches the folder view: subdirectories first, then audio
                // files, each group sorted by path.
                let entries: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
                    .ok()?
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .collect();
                let subdirs = entries.iter().filter(|path| path.is_dir()).count();
                let mut files: Vec<&std::path::PathBuf> = entries
                    .iter()
                    .filter(|path| path.is_file())
                    .filter(|path| {
                        path.extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| {
                                matches!(
                                    ext.to_lowercase().as_str(),
                                    "flac" | "mp3" | "wav" | "ogg" | "m4a" | "aac"
                                )
                            })
                            .unwrap_or(false)
                    })
                    .collect();
                files.sort();
                files
                    .iter()
                    .position(|path| current_path.as_deref() == Some(path.as_path()))
                    .map(|index| subdirs + index)
            }
            _ => None,
        }
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
        self.load_lyrics_for_current(cx);
        self.load_cover_for_current(cx);
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
        self.sync_lyrics_line(cx);
        self.follow_lyrics(cx);

        if self.playback_state != PlaybackState::Playing {
            return;
        }

        // Keep the persisted session close to the live position so quitting (or a
        // crash) does not lose the progress made since the track started.
        if self.last_session_save.elapsed() >= Duration::from_secs(5) {
            self.last_session_save = Instant::now();
            self.save_state_now();
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
        if self.show_queue
            && let Some(index) = self.queue_index
        {
            // Opening the drawer should land on the current track instead of
            // the top of a 621-item queue.
            self.queue_scroll_handle
                .scroll_to_item(index, ScrollStrategy::Center);
        }
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
        self.load_lyrics_for_current(cx);
        self.position = 0.0;
        self.duration = 0.0;
        self.persist_session();
        cx.notify();
    }

    // ── Now playing view / lyrics ──

    /// Show the now-playing view: large cover, track details and lyrics.
    pub fn open_now_playing(&mut self, cx: &mut Context<Self>) {
        if self.show_now_playing {
            return;
        }
        self.show_now_playing = true;
        // The lyrics column is laid out fresh; recenter on the active line.
        self.lyrics_scrolled_line = None;
        cx.notify();
    }

    /// Return from the now-playing view to the previously selected list.
    pub fn close_now_playing(&mut self, cx: &mut Context<Self>) {
        if !self.show_now_playing {
            return;
        }
        self.show_now_playing = false;
        self.lyrics_scrolled_line = None;
        cx.notify();
    }

    /// Toggle the now-playing view (player-bar button / `L` shortcut).
    pub fn toggle_now_playing(&mut self, cx: &mut Context<Self>) {
        if self.show_now_playing {
            self.close_now_playing(cx);
        } else {
            self.open_now_playing(cx);
        }
    }

    /// Fetch missing cover art for the current track on the background executor.
    ///
    /// Embedded art (or an earlier online result) wins; each track is attempted
    /// at most once per session so a miss does not hammer the providers.
    fn load_cover_for_current(&mut self, cx: &mut Context<Self>) {
        let Some(track) = self.current_track.clone() else {
            return;
        };
        if !self.config.fetch_covers_online {
            return;
        }
        if crate::paths::get_or_save_cover(&track).is_some() {
            return;
        }
        if !self.cover_attempts.insert(track.id) {
            return;
        }

        let track_id = track.id;
        let path = crate::paths::cover_path(track_id);
        let track_for_fetch = track.clone();
        cx.spawn(async move |entity, cx| {
            let bytes = cx
                .background_executor()
                .spawn(async move { crate::covers::fetch(&track_for_fetch) })
                .await;
            let Some(bytes) = bytes else {
                crate::log_debug!("no online cover found for track {}", track_id);
                return;
            };
            if !crate::netease::is_image(&bytes) {
                crate::log_warn!("online cover for track {} is not an image", track_id);
                return;
            }

            // Write then rename so a crash cannot leave a truncated cover that
            // every later render would try to decode.
            let tmp = path.with_extension("jpg.tmp");
            if let Err(err) =
                std::fs::write(&tmp, &bytes).and_then(|_| std::fs::rename(&tmp, &path))
            {
                crate::log_warn!("failed to cache cover for track {}: {}", track_id, err);
                let _ = std::fs::remove_file(&tmp);
                return;
            }

            let _ = entity.update(cx, |this, cx| {
                crate::log_info!(
                    "cover cached for track {} ({} bytes)",
                    track_id,
                    bytes.len()
                );
                if this.current_track.as_ref().map(|t| t.id) == Some(track_id) {
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Load lyrics for the current track on the background executor.
    ///
    /// Probe + tag decode touch the filesystem, so they never run in render or
    /// on the UI thread. A generation check keeps a slow load for the previous
    /// track from replacing the current one.
    fn load_lyrics_for_current(&mut self, cx: &mut Context<Self>) {
        let Some(track) = self.current_track.clone() else {
            self.lyrics = None;
            self.lyrics_track_id = None;
            self.lyrics_loading = false;
            self.lyrics_active_line = None;
            self.lyrics_scrolled_line = None;
            return;
        };

        // Replaying or seeking the same track keeps the already parsed lyrics.
        if self.lyrics_track_id == Some(track.id) && (self.lyrics.is_some() || self.lyrics_loading)
        {
            return;
        }

        let track_id = track.id;
        self.lyrics = None;
        self.lyrics_track_id = Some(track_id);
        self.lyrics_loading = true;
        self.lyrics_active_line = None;
        self.lyrics_scrolled_line = None;
        let online = self.config.fetch_lyrics_online;

        cx.spawn(async move |entity, cx| {
            let loaded = cx
                .background_executor()
                .spawn(async move { crate::lyrics::load(&track, online) })
                .await;

            let _ = entity.update(cx, |this, cx| {
                if this.lyrics_track_id != Some(track_id) {
                    return;
                }
                this.lyrics_loading = false;
                this.lyrics = loaded.map(Arc::new);
                this.lyrics_active_line = None;
                this.lyrics_scrolled_line = None;
                if let Some(lyrics) = this.lyrics.as_ref() {
                    crate::log_info!(
                        "lyrics loaded: {} lines (synced={}, source={})",
                        lyrics.lines.len(),
                        lyrics.synced,
                        lyrics.source.label()
                    );
                    let position = duration_from_secs(this.position);
                    this.lyrics_active_line = lyrics.active_line(position);
                } else {
                    crate::log_debug!("no lyrics found for track {}", track_id);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Recompute the highlighted lyric line from the playback position.
    fn sync_lyrics_line(&mut self, cx: &mut Context<Self>) {
        let position = duration_from_secs(self.position);
        let next = self
            .lyrics
            .as_ref()
            .and_then(|lyrics| lyrics.active_line(position));
        if next != self.lyrics_active_line {
            self.lyrics_active_line = next;
            cx.notify();
        }
    }

    /// Keep the highlighted line inside the visible lyrics column.
    fn follow_lyrics(&mut self, cx: &mut Context<Self>) {
        if !self.show_now_playing {
            return;
        }
        let Some(index) = self.lyrics_active_line else {
            return;
        };
        if self.lyrics_scrolled_line == Some(index) {
            return;
        }
        if self.scroll_lyrics_to_active(index) {
            self.lyrics_scrolled_line = Some(index);
            cx.notify();
        }
    }

    /// Place the active line about a third of the way down the viewport.
    ///
    /// Returns `false` until the lyrics column has been laid out, so callers
    /// can retry on the next tick (e.g. right after the view is opened).
    fn scroll_lyrics_to_active(&self, index: usize) -> bool {
        let handle = &self.lyrics_scroll_handle;
        let viewport = handle.bounds();
        if viewport.size.height <= px(1.0) || viewport.size.width <= px(1.0) {
            return false;
        }
        let Some(line) = handle.bounds_for_item(index) else {
            return false;
        };

        // Child bounds are content coordinates: the scroll offset is applied
        // during paint, so it must not be added to the target again.
        let target_top = viewport.top() + viewport.size.height * 0.32;
        let y = target_top - line.top();
        let min = -handle.max_offset().y;
        let y = if y < min {
            min
        } else if y > px(0.0) {
            px(0.0)
        } else {
            y
        };
        handle.set_offset(point(px(0.0), y));
        true
    }

    /// Drop the cached NetEase result and look the current track up again.
    pub fn refresh_lyrics(&mut self, cx: &mut Context<Self>) {
        let Some(track) = self.current_track.clone() else {
            return;
        };

        let path = crate::paths::lyrics_cache_path(track.id);
        if let Err(err) = std::fs::remove_file(&path)
            && err.kind() != std::io::ErrorKind::NotFound
        {
            crate::log_warn!("failed to remove lyrics cache {}: {}", path.display(), err);
        }

        self.lyrics_track_id = None;
        self.lyrics_loading = false;
        self.load_lyrics_for_current(cx);
        cx.notify();
    }

    /// Seek to a synced lyric line; start playback if the engine is idle.
    pub fn play_lyric_line(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(seconds) = self
            .lyrics
            .as_ref()
            .and_then(|lyrics| lyrics.line_time(index))
            .map(|time| time.as_secs_f64())
        else {
            return;
        };
        if self.current_track.is_none() {
            return;
        }

        if self.playback_state == PlaybackState::Stopped {
            self.play_current_queue_from(seconds, cx);
        } else {
            self.seek_to(seconds, cx);
        }
        self.sync_lyrics_line(cx);
        self.lyrics_scrolled_line = None;
        self.follow_lyrics(cx);
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
        // Persist immediately: closing the window right after toggling must not
        // read a stale value from disk, and the throttled flush may be skipped.
        if let Err(err) = self.config.save() {
            crate::log_error!("failed to save close_to_tray preference: {err}");
        }
        crate::log_info!("close_to_tray set to {enabled}");
        cx.notify();
    }

    /// Enable or disable NetEase lookup for tracks without local lyrics.
    ///
    /// Cached results stay available either way; only new requests stop.
    pub fn set_fetch_lyrics_online(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.update_config(|config| config.fetch_lyrics_online = enabled);
        crate::log_info!("fetch_lyrics_online set to {enabled}");

        if enabled {
            // Restart the current load: an in-flight local-only lookup could
            // otherwise finish with no lyrics and never retry.
            self.lyrics_track_id = None;
            self.lyrics_loading = false;
            self.load_lyrics_for_current(cx);
        }

        self.flush_config(cx);
        cx.notify();
    }

    /// Enable or disable online cover art (NetEase, then iTunes).
    pub fn set_fetch_covers_online(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.update_config(|config| config.fetch_covers_online = enabled);
        crate::log_info!("fetch_covers_online set to {enabled}");

        if enabled {
            if let Some(track_id) = self.current_track.as_ref().map(|track| track.id) {
                self.cover_attempts.remove(&track_id);
            }
            self.load_cover_for_current(cx);
        }

        self.flush_config(cx);
        cx.notify();
    }

    /// Set the playback mode directly (used by the tray menu).
    pub fn set_playback_mode(&mut self, mode: PlaybackMode, cx: &mut Context<Self>) {
        self.playback_mode = mode;
        self.update_config(|config| config.playback_mode = mode);
        self.flush_config(cx);
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

    // ── Tag / metadata editing ──

    /// Open the metadata editor for `track`.
    ///
    /// Tag parsing is synchronous but skips cover art and audio properties, so
    /// it stays cheap even for files carrying multi-megabyte artwork.
    pub fn open_tag_editor(
        &mut self,
        track: Arc<Track>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (tags, error) = match crate::tags::read(&track.path) {
            Ok(tags) => (tags, None),
            Err(err) => {
                crate::log_warn!("failed to read tags of {}: {err:#}", track.path.display());
                (
                    EditableTags::from_track(&track),
                    Some(format!("读取标签失败，已回退到曲库缓存：{err:#}")),
                )
            }
        };

        let title = tag_input(window, cx);
        let artist = tag_input(window, cx);
        let album = tag_input(window, cx);
        let album_artist = tag_input(window, cx);
        let genre = tag_input(window, cx);
        let year = tag_input(window, cx);
        let track_number = tag_input(window, cx);
        let disc_number = tag_input(window, cx);

        for (input, value) in [
            (&title, tags.title.as_str()),
            (&artist, tags.artist.as_str()),
            (&album, tags.album.as_str()),
            (&album_artist, tags.album_artist.as_str()),
            (&genre, tags.genre.as_str()),
            (&year, tags.year.as_str()),
            (&track_number, tags.track.as_str()),
            (&disc_number, tags.disc.as_str()),
        ] {
            input.update(cx, |state, cx| state.set_value(value, window, cx));
        }

        // Put the caret in the title field so typing works without an extra
        // click. The modal hands root focus back when it closes.
        let title_focus = title.read(cx).focus_handle(cx);
        window.focus(&title_focus, cx);

        self.tag_edit = Some(TagEditState {
            track_id: track.id,
            path: track.path.clone(),
            title,
            artist,
            album,
            album_artist,
            genre,
            year,
            track_number,
            disc_number,
            error,
            saving: false,
        });
        cx.notify();
    }

    /// Write the open editor back to the file on the background executor, then
    /// refresh the track from disk.
    pub fn save_tag_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.tag_edit.as_ref() else {
            return;
        };
        if edit.saving {
            return;
        }

        let tags = EditableTags {
            title: edit.title.read(cx).value().to_string(),
            artist: edit.artist.read(cx).value().to_string(),
            album: edit.album.read(cx).value().to_string(),
            album_artist: edit.album_artist.read(cx).value().to_string(),
            genre: edit.genre.read(cx).value().to_string(),
            year: edit.year.read(cx).value().to_string(),
            track: edit.track_number.read(cx).value().to_string(),
            disc: edit.disc_number.read(cx).value().to_string(),
        };
        let path = edit.path.clone();
        let display_path = path.display().to_string();
        let track_id = edit.track_id;

        if let Some(edit) = self.tag_edit.as_mut() {
            edit.saving = true;
            edit.error = None;
        }
        cx.notify();

        cx.spawn_in(window, async move |entity, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    crate::tags::write(&path, &tags)?;
                    crate::audio::decoder::read_metadata(&path, track_id)
                })
                .await;

            let _ = entity.update_in(cx, |this, window, cx| {
                match result {
                    Ok(track) => {
                        crate::log_info!("updated tags of {display_path}");
                        this.tag_edit = None;
                        this.apply_updated_track(track);
                        // Restore the root focus handle; otherwise global
                        // shortcuts die with the modal's inputs.
                        window.focus(&this.focus_handle, cx);
                    }
                    Err(err) => {
                        crate::log_error!("failed to save tags of {display_path}: {err:#}");
                        if let Some(edit) = this.tag_edit.as_mut() {
                            edit.saving = false;
                            edit.error = Some(format!("保存失败：{err:#}"));
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Dismiss the metadata editor without writing.
    pub fn close_tag_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tag_edit.as_ref().is_some_and(|edit| edit.saving) {
            return;
        }
        self.tag_edit = None;
        // Closing the modal drops its inputs; hand focus back to the root view
        // so global shortcuts keep working.
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    /// Swap in metadata re-read from disk after an edit.
    fn apply_updated_track(&mut self, updated: Arc<Track>) {
        if let Some(index) = self.library.iter().position(|track| track.id == updated.id) {
            self.library[index] = updated.clone();
        }

        // Keep the scan order (album, then track number) after an edit, so
        // All Music and the album detail views stay consistent. Unlike
        // `apply_filter`, this does not jump the list back to the top.
        self.library.sort_by(|a, b| {
            a.album
                .cmp(&b.album)
                .then(a.track_number.cmp(&b.track_number))
        });
        self.filtered_indices = self.compute_filter();

        for track in &mut self.queue {
            if track.id == updated.id {
                *track = updated.clone();
            }
        }
        if self
            .current_track
            .as_ref()
            .is_some_and(|track| track.id == updated.id)
        {
            self.current_track = Some(updated.clone());
        }
        self.selected_track_id = Some(updated.id);
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

    /// Persist the current session synchronously (used on quit and every few
    /// seconds while playing). Bypasses the throttle: quitting must not lose the
    /// last few seconds of progress.
    pub fn save_state_now(&mut self) {
        self.persist_session();
        if let Err(err) = self.config.save() {
            crate::log_error!("failed to save session state: {err}");
        }
        self.config_dirty = false;
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
        self.load_lyrics_for_current(cx);
        self.load_cover_for_current(cx);
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

/// Clamp a playback position before converting it to a `Duration`.
fn duration_from_secs(seconds: f64) -> Duration {
    Duration::try_from_secs_f64(seconds).unwrap_or_default()
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
        self.save_state_now();
    }
}
