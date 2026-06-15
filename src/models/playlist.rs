use std::sync::Arc;
use std::time::Duration;

/// Playback repeat mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    #[default]
    None,
    One,
    All,
}

/// Playback state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

/// A named playlist containing a sequence of track indices.
/// Stores indices into the library rather than owned `Track`s
/// to avoid redundant allocations.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct Playlist {
    pub name: String,
    pub track_indices: Vec<usize>,
}

#[allow(dead_code)]
impl Playlist {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            track_indices: Vec::new(),
        }
    }

    pub fn duration(&self, tracks: &[Arc<crate::models::track::Track>]) -> Duration {
        self.track_indices
            .iter()
            .filter_map(|&i| tracks.get(i))
            .fold(Duration::ZERO, |acc, t| acc + t.duration)
    }
}
