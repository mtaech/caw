use std::path::PathBuf;

/// Canonical directory for Caw configuration, database, and logs: `~/.config/caw/`
pub fn caw_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("caw");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Directory for rotated logs: `~/.config/caw/logs/`
#[allow(dead_code)] // file logger currently keeps a single rotated caw.log
pub fn logs_dir() -> PathBuf {
    let dir = caw_dir().join("logs");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Directory for extracted album art covers: `~/.config/caw/covers/`
pub fn covers_dir() -> PathBuf {
    let dir = caw_dir().join("covers");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Directory for online lyrics cached as `.lrc`: `~/.config/caw/lyrics/`
pub fn lyrics_dir() -> PathBuf {
    let dir = caw_dir().join("lyrics");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Cache path for one track's online lyrics.
pub fn lyrics_cache_path(track_id: u64) -> PathBuf {
    lyrics_dir().join(format!("{track_id}.lrc"))
}

/// Path to config.json: `~/.config/caw/config.json`
pub fn config_path() -> PathBuf {
    caw_dir().join("config.json")
}

/// Path to database: `~/.config/caw/caw.db`
pub fn db_path() -> PathBuf {
    caw_dir().join("caw.db")
}

/// Path to current log file: `~/.config/caw/caw.log`
pub fn log_path() -> PathBuf {
    caw_dir().join("caw.log")
}

/// Cache path for one track's cover (embedded or downloaded).
pub fn cover_path(track_id: u64) -> PathBuf {
    covers_dir().join(format!("{track_id}.jpg"))
}

/// Save embedded cover art on first use, or return an already cached cover.
///
/// Online covers are written to the same path, so this finds both embedded and
/// downloaded art; callers keep using this one function.
pub fn get_or_save_cover(track: &crate::models::track::Track) -> Option<PathBuf> {
    let file_path = cover_path(track.id);
    if file_path.exists() {
        return Some(file_path);
    }

    let cover_data = track.cover_data.as_ref()?;
    let _ = std::fs::write(&file_path, &**cover_data);
    Some(file_path)
}
