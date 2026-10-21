//! Window geometry persistence.
//!
//! Saves the main window position, size and maximized flag to
//! ~/.config/caw/window.json and restores it on the next start.
//!
//! PLAN.md originally proposed storing this inside config.json; that was changed to a
//! separate file so window state does not depend on core-audio's AppConfig. Writes are
//! atomic (tmp file + rename), so a crash mid-save cannot leave a truncated file.
//!
//! The file is read once at startup (cheap) and written on window close. Corrupt or
//! implausible geometry is ignored and the window falls back to its default size.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use gpui_kit::{Bounds, Point, Size, Window, WindowBounds, px};

/// Smallest geometry we accept; below this the file is considered corrupt.
const MIN_WIDTH: f32 = 640.0;
const MIN_HEIGHT: f32 = 400.0;
/// Upper bound to reject absurd values from a corrupt file.
const MAX_DIMENSION: f32 = 16_384.0;

/// Persisted main-window geometry, in logical pixels (GPUI coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub maximized: bool,
}

impl WindowState {
    /// Capture the current geometry of a gpui window.
    pub fn capture(window: &Window) -> Self {
        let bounds = window.window_bounds().get_bounds();
        Self {
            x: bounds.origin.x.as_f32(),
            y: bounds.origin.y.as_f32(),
            width: bounds.size.width.as_f32(),
            height: bounds.size.height.as_f32(),
            maximized: window.is_maximized(),
        }
    }

    /// Convert to the type expected by WindowOptions.window_bounds.
    pub fn to_window_bounds(self) -> WindowBounds {
        let bounds = Bounds {
            origin: Point {
                x: px(self.x),
                y: px(self.y),
            },
            size: Size {
                width: px(self.width.clamp(MIN_WIDTH, MAX_DIMENSION)),
                height: px(self.height.clamp(MIN_HEIGHT, MAX_DIMENSION)),
            },
        };
        if self.maximized {
            WindowBounds::Maximized(bounds)
        } else {
            WindowBounds::Windowed(bounds)
        }
    }

    /// Read from the default location; None when missing, corrupt or implausible.
    #[allow(dead_code)] // mirrored by the free fn load(); either may be used by main.rs
    pub fn load() -> Option<Self> {
        Self::load_from(&state_path())
    }

    /// Read from an explicit path (used by tests/diagnostics).
    pub fn load_from(path: &Path) -> Option<Self> {
        let contents = fs::read_to_string(path).ok()?;
        let state: Self = serde_json::from_str(&contents).ok()?;
        state.sanitized()
    }

    /// Write to the default location, ignoring any error (window state is best-effort).
    #[allow(dead_code)] // mirrored by the free fn save(); either may be used by main.rs
    pub fn save(self) {
        if let Err(err) = self.save_to(&state_path()) {
            crate::log_warn!("platform: could not save window state: {err}");
        }
    }

    /// Write to an explicit path atomically (tmp file + rename).
    pub fn save_to(self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        let tmp_path = tmp_path(path);
        fs::write(&tmp_path, json)?;
        fs::rename(&tmp_path, path)
    }

    fn sanitized(self) -> Option<Self> {
        let finite = self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite();
        if !finite || self.width < MIN_WIDTH || self.height < MIN_HEIGHT {
            return None;
        }
        Some(Self {
            width: self.width.min(MAX_DIMENSION),
            height: self.height.min(MAX_DIMENSION),
            ..self
        })
    }
}

/// Load the persisted window geometry from ~/.config/caw/window.json.
#[allow(dead_code)] // public helper for main.rs; may be unused while both forms exist
pub fn load() -> Option<WindowState> {
    WindowState::load()
}

/// Save the given geometry (see also save_now).
#[allow(dead_code)] // public helper for main.rs; may be unused while both forms exist
pub fn save(state: &WindowState) {
    state.save();
}

/// Capture and persist the current window geometry in one call (use on window close).
pub fn save_now(window: &Window) {
    WindowState::capture(window).save();
}

fn state_path() -> PathBuf {
    crate::paths::caw_dir().join("window.json")
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    PathBuf::from(tmp)
}
