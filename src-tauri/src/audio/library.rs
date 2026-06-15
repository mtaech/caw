use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::Result;

use crate::audio::decoder::read_metadata;
use crate::models::track::Track;

/// Supported audio file extensions.
const SUPPORTED_EXTENSIONS: &[&str] = &["flac", "mp3", "wav", "ogg", "m4a", "aac", "alac"];

/// Scan a directory recursively for audio files, extracting metadata.
/// Returns `Vec<Arc<Track>>` to avoid cloning track data across the UI.
pub fn scan_directory(path: &Path) -> Result<Vec<Arc<Track>>> {
    let id_counter = AtomicU64::new(1);

    // Collect candidate files first (fast), then decode metadata.
    // This separates I/O from CPU work and makes the scan more cache-friendly.
    let candidates: Vec<PathBuf> = walkdir::WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let ext = e.path().extension()?.to_str()?.to_lowercase();
            if SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                Some(e.path().to_path_buf())
            } else {
                None
            }
        })
        .collect();

    // Scan metadata for all candidates.
    let mut tracks = Vec::with_capacity(candidates.len());
    for path in &candidates {
        match read_metadata(path, id_counter.fetch_add(1, Ordering::Relaxed)) {
            Ok(track) => tracks.push(track),
            Err(e) => eprintln!("Failed to read metadata from {:?}: {}", path, e),
        }
    }

    tracks.sort_by(|a, b| a.album.cmp(&b.album).then(a.track_number.cmp(&b.track_number)));

    Ok(tracks)
}

/// Find all music directories under common roots.
pub fn find_music_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(music_dir) = std::env::var("XDG_MUSIC_DIR") {
        let p = PathBuf::from(music_dir);
        if p.exists() {
            dirs.push(p);
        }
    }

    if let Some(home) = dirs::home_dir() {
        let music = home.join("Music");
        if music.exists() {
            dirs.push(music);
        }
        let downloads = home.join("Downloads");
        if downloads.exists() {
            dirs.push(downloads);
        }
    }

    // Deduplicate
    dirs.dedup();
    dirs
}
