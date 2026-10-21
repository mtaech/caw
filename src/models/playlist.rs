use std::sync::Arc;
use std::time::Duration;

use crate::ui::icons::CawIcon;

/// Playback mode: sequence, shuffle, repeat list, or repeat one.
///
/// Serialized as part of `AppConfig` (`config.json`), hence the serde derives.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackMode {
    #[default]
    Sequence,
    Shuffle,
    RepeatList,
    RepeatOne,
}

impl PlaybackMode {
    pub fn next(self) -> Self {
        match self {
            Self::Sequence => Self::Shuffle,
            Self::Shuffle => Self::RepeatList,
            Self::RepeatList => Self::RepeatOne,
            Self::RepeatOne => Self::Sequence,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Sequence => "顺序播放",
            Self::Shuffle => "随机播放",
            Self::RepeatList => "列表循环",
            Self::RepeatOne => "单曲循环",
        }
    }

    pub fn icon(self) -> CawIcon {
        match self {
            Self::Sequence => CawIcon::ListMusic,
            Self::Shuffle => CawIcon::Shuffle,
            Self::RepeatList => CawIcon::Repeat,
            Self::RepeatOne => CawIcon::Repeat1,
        }
    }
}

/// Playback repeat mode.
#[allow(dead_code)] // superseded by PlaybackMode; kept until queue history UI lands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    #[default]
    None,
    One,
    All,
}

/// Playback state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

/// A named playlist containing a sequence of track indices.
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
