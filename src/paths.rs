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

/// Save cover art bytes to disk if present, returning the file path.
pub fn get_or_save_cover(track: &crate::models::track::Track) -> Option<PathBuf> {
    let cover_data = track.cover_data.as_ref()?;
    let covers_dir = covers_dir();
    let file_path = covers_dir.join(format!("{}.jpg", track.id));
    if !file_path.exists() {
        let _ = std::fs::write(&file_path, &**cover_data);
    }
    Some(file_path)
}
