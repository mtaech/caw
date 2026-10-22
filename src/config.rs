//! Persisted user preferences: `~/.config/caw/config.json`.
//!
//! One small JSON document holds every setting that must survive a restart.
//! Playlists and music folders live in SQLite (`crate::db`), not here.

use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::App;
use gpui_kit::component::{Theme, ThemeMode};
use serde::{Deserialize, Serialize};

use crate::models::playlist::PlaybackMode;

/// Default UI font (matches the system font setup documented in AGENTS.md).
pub const DEFAULT_FONT_FAMILY: &str = "Noto Sans CJK SC";
/// Default monospace font family.
pub const DEFAULT_MONO_FONT_FAMILY: &str = "DejaVu Sans Mono";
/// Default playback volume (0.0 - 1.0).
pub const DEFAULT_VOLUME: f32 = 0.8;

/// Serializes config writes: the debounced flush (background executor) and the
/// final save on teardown must not interleave on the same temp file.
static SAVE_LOCK: Mutex<()> = Mutex::new(());

/// Maximum number of entries kept in the play history.
pub const MAX_HISTORY: usize = 100;

/// Last known window geometry.
///
/// Stored as plain floats so the JSON stays readable; the platform layer
/// converts to/from GPUI's `WindowBounds`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub maximized: bool,
}

/// All preferences persisted to `~/.config/caw/config.json`.
///
/// Every field has a default and unknown fields are ignored, so config files
/// written by an older/newer build still load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Light or dark. gpui-kit has no "follow system" mode; callers resolve the
    /// OS appearance to one of the two.
    pub theme_mode: ThemeMode,
    /// UI sans-serif font family.
    pub font_family: String,
    /// Monospace font family.
    pub mono_font_family: String,
    /// Playback volume, 0.0 - 1.0.
    pub volume: f32,
    /// Sequence / shuffle / repeat-list / repeat-one.
    pub playback_mode: PlaybackMode,
    /// Close button hides to the tray instead of quitting (used once a tray exists).
    pub close_to_tray: bool,
    /// Fetch lyrics from NetEase when the file has no local lyrics.
    pub fetch_lyrics_online: bool,
    /// Fetch missing cover art online (NetEase, then iTunes).
    pub fetch_covers_online: bool,
    /// Last window geometry, restored on startup when present.
    pub window_bounds: Option<WindowBounds>,
    /// Unix timestamp (seconds) of the last completed library scan.
    pub last_scan_time: Option<u64>,

    // ── Session restore (P3) ──
    /// Path of the track that was playing when the app last closed.
    pub last_track_path: Option<String>,
    /// Paths of the playback queue, in play order.
    pub last_queue_paths: Vec<String>,
    /// Playback position (seconds) within `last_track_path`.
    pub last_position: f64,
    /// Index into `last_queue_paths` of the current track.
    pub last_queue_index: Option<usize>,
    /// Recently played track paths, most recent first (bounded by `MAX_HISTORY`).
    pub history_paths: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            // Caw is a dark-first player; gpui-kit's own default is light.
            theme_mode: ThemeMode::Dark,
            font_family: DEFAULT_FONT_FAMILY.to_string(),
            mono_font_family: DEFAULT_MONO_FONT_FAMILY.to_string(),
            volume: DEFAULT_VOLUME,
            playback_mode: PlaybackMode::default(),
            // No tray exists until the platform phase, so hiding on close would
            // strand the app in the background; default to a normal close.
            close_to_tray: false,
            // Personal builds want lyrics without extra setup; the setting can
            // still be turned off for offline-only use.
            fetch_lyrics_online: true,
            fetch_covers_online: true,
            window_bounds: None,
            last_scan_time: None,
            last_track_path: None,
            last_queue_paths: Vec::new(),
            last_position: 0.0,
            last_queue_index: None,
            history_paths: Vec::new(),
        }
    }
}

impl AppConfig {
    /// Load preferences from `~/.config/caw/config.json`.
    ///
    /// A missing, unreadable, or corrupt file yields defaults instead of
    /// failing: bad preferences must never prevent the app from starting.
    pub fn load() -> Self {
        Self::load_from(&crate::paths::config_path())
    }

    pub(crate) fn load_from(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(raw) => match serde_json::from_str(&raw) {
                Ok(config) => config,
                Err(err) => {
                    crate::log_warn!(
                        "invalid config at {}: {} — using defaults",
                        path.display(),
                        err
                    );
                    Self::default()
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(err) => {
                crate::log_warn!(
                    "cannot read config {}: {} — using defaults",
                    path.display(),
                    err
                );
                Self::default()
            }
        }
    }

    /// Persist preferences to `~/.config/caw/config.json`.
    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to(&crate::paths::config_path())
    }

    pub(crate) fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        // A poisoned lock only means a previous save panicked; the file write
        // itself is still safe to attempt.
        let _guard = SAVE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        // Write-then-rename keeps the previous file intact if we crash midway.
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Apply the persisted theme mode and fonts to the global gpui-kit theme.
    ///
    /// Call after `gpui_kit::init(cx)`. `Theme::update` (unlike
    /// `Theme::global_mut`) also re-resolves the default fonts and refreshes
    /// every open window, which is required for a font change to take effect.
    pub fn apply_theme(&self, cx: &mut App) {
        Theme::change(self.theme_mode, None, cx);

        let font_family = self.font_family.clone();
        let mono_font_family = self.mono_font_family.clone();
        if font_family.is_empty() && mono_font_family.is_empty() {
            return;
        }
        Theme::update(cx, |theme| {
            if !font_family.is_empty() {
                theme.font_family = font_family.into();
            }
            if !mono_font_family.is_empty() {
                theme.mono_font_family = mono_font_family.into();
            }
        });
    }
}

/// Current Unix timestamp in seconds.
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("caw-config-{}-{}.json", tag, std::process::id()));
        path
    }

    #[test]
    fn config_round_trips_through_json_file() {
        let path = temp_path("roundtrip");
        let config = AppConfig {
            theme_mode: ThemeMode::Light,
            font_family: "Test Sans".to_string(),
            mono_font_family: "Test Mono".to_string(),
            volume: 0.35,
            playback_mode: PlaybackMode::Shuffle,
            close_to_tray: true,
            fetch_lyrics_online: false,
            fetch_covers_online: false,
            window_bounds: Some(WindowBounds {
                x: 10.0,
                y: 20.0,
                width: 1080.0,
                height: 720.0,
                maximized: true,
            }),
            last_scan_time: Some(1_700_000_000),
            last_track_path: Some("/music/a.flac".to_string()),
            last_queue_paths: vec!["/music/a.flac".to_string(), "/music/b.flac".to_string()],
            last_position: 42.5,
            last_queue_index: Some(1),
            history_paths: vec!["/music/b.flac".to_string()],
        };

        config.save_to(&path).expect("save config");
        let loaded = AppConfig::load_from(&path);
        let _ = fs::remove_file(&path);

        assert_eq!(loaded, config);
    }

    #[test]
    fn missing_and_partial_config_fall_back_to_defaults() {
        let missing = temp_path("missing");
        assert_eq!(AppConfig::load_from(&missing), AppConfig::default());

        let partial = temp_path("partial");
        fs::write(&partial, r#"{ "theme_mode": "light" }"#).expect("write partial config");
        let loaded = AppConfig::load_from(&partial);
        let _ = fs::remove_file(&partial);

        assert_eq!(loaded.theme_mode, ThemeMode::Light);
        assert_eq!(loaded.volume, AppConfig::default().volume);
        assert_eq!(loaded.playback_mode, AppConfig::default().playback_mode);
    }

    #[test]
    fn corrupt_config_falls_back_to_defaults() {
        let path = temp_path("corrupt");
        fs::write(&path, "not json").expect("write corrupt config");
        let loaded = AppConfig::load_from(&path);
        let _ = fs::remove_file(&path);
        assert_eq!(loaded, AppConfig::default());
    }
}
