use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;

use crate::audio::decoder::read_metadata;
use crate::models::track::Track;

/// Supported audio file extensions.
const SUPPORTED_EXTENSIONS: &[&str] = &["flac", "mp3", "wav", "ogg", "m4a", "aac", "alac"];

/// Scan multiple directories recursively for audio files, deduplicating by canonical path.
pub fn scan_directories(paths: &[PathBuf]) -> Result<Vec<Arc<Track>>> {
    let mut seen_paths = HashSet::new();
    let mut candidates = Vec::new();

    for dir in paths {
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            if let Some(ext) = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                && SUPPORTED_EXTENSIONS.contains(&ext.as_str())
            {
                // Canonicalize so the same file reached through two music
                // directories (or a symlink) is scanned once, and so its id
                // does not depend on how the directory was spelled.
                let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                if seen_paths.insert(canonical.clone()) {
                    candidates.push(canonical);
                }
            }
        }
    }

    let mut tracks = Vec::with_capacity(candidates.len());
    for path in &candidates {
        match read_metadata(path, stable_track_id(path)) {
            Ok(track) => tracks.push(track),
            Err(e) => eprintln!("caw: failed to read metadata from {:?}: {}", path, e),
        }
    }

    tracks.sort_by(|a, b| {
        a.album
            .cmp(&b.album)
            .then(a.track_number.cmp(&b.track_number))
    });

    Ok(tracks)
}

/// Stable 64-bit track id derived from the canonical file path (FNV-1a).
///
/// Playlists in SQLite reference tracks by this id, so it must not depend on
/// scan order: adding one file must not renumber every track after it (which
/// would silently corrupt existing playlists).
fn stable_track_id(path: &Path) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = FNV_OFFSET_BASIS;
    for byte in path.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Scan a single directory recursively for audio files.
#[allow(dead_code)]
pub fn scan_directory(path: &Path) -> Result<Vec<Arc<Track>>> {
    scan_directories(&[path.to_path_buf()])
}

/// Find all default music directories under common roots.
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

    dirs.dedup();
    dirs
}
