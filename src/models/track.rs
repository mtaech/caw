use std::path::PathBuf;
use std::time::Duration;

/// Supported audio formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioFormat {
    Flac,
    Mp3,
    Wav,
    OggVorbis,
    Aac,
    Alac,
    Other(String),
}

impl AudioFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "flac" => Some(Self::Flac),
            "mp3" => Some(Self::Mp3),
            "wav" => Some(Self::Wav),
            "ogg" => Some(Self::OggVorbis),
            "m4a" | "aac" => Some(Self::Aac),
            "alac" => Some(Self::Alac),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn extension(&self) -> &str {
        match self {
            Self::Flac => "flac",
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::OggVorbis => "ogg",
            Self::Aac => "m4a",
            Self::Alac => "alac",
            Self::Other(ext) => ext,
        }
    }
}

/// Represents a single music track with its metadata.
#[derive(Debug, Clone)]
pub struct Track {
    #[allow(dead_code)]
    pub id: u64,
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: Duration,
    pub track_number: u32,
    #[allow(dead_code)]
    pub format: AudioFormat,
    /// Cover art data — use `Arc<Vec<u8>>` to avoid expensive clones in lists.
    #[allow(dead_code)]
    pub cover_data: Option<std::sync::Arc<Vec<u8>>>,
}

impl Track {
    /// Format duration as "MM:SS".
    #[inline]
    pub fn duration_formatted(&self) -> String {
        let secs = self.duration.as_secs();
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }

    /// Display title, falling back to filename.
    #[inline]
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            self.path
                .file_stem()
                .map(|s| s.to_str().unwrap_or("Unknown"))
                .unwrap_or("Unknown")
        } else {
            &self.title
        }
    }

    /// Display artist, falling back to "Unknown Artist".
    #[inline]
    pub fn display_artist(&self) -> &str {
        if self.artist.is_empty() {
            "Unknown Artist"
        } else {
            &self.artist
        }
    }
}
