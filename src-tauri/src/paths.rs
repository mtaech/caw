use std::path::PathBuf;

/// Canonical directory for Caw configuration, database, and logs: `~/.config/caw/`
pub fn caw_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("caw");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Directory for rotated logs: `~/.config/caw/logs/`
pub fn logs_dir() -> PathBuf {
    let dir = caw_dir().join("logs");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Path to config.json: `~/.config/caw/config.json`
pub fn config_path() -> PathBuf {
    let new_path = caw_dir().join("config.json");
    // Migrate from old default identifier location (~/.config/cn.cyber-nest.caw/config.json) if needed
    if !new_path.exists() {
        if let Some(base) = dirs::config_dir() {
            let old_path = base.join("cn.cyber-nest.caw").join("config.json");
            if old_path.exists() {
                let _ = std::fs::copy(&old_path, &new_path);
            }
        }
    }
    new_path
}

/// Path to database: `~/.config/caw/caw.db`
pub fn db_path() -> PathBuf {
    let new_path = caw_dir().join("caw.db");
    // Migrate from old ~/.local/share/caw/caw.db if needed
    if !new_path.exists() {
        if let Some(old_base) = dirs::data_dir() {
            let old_path = old_base.join("caw").join("caw.db");
            if old_path.exists() {
                let _ = std::fs::copy(&old_path, &new_path);
            }
        }
    }
    new_path
}

/// Path to current log file: `~/.config/caw/caw.log`
pub fn log_path() -> PathBuf {
    caw_dir().join("caw.log")
}
