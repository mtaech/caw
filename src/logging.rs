//! File logger for Caw.
//!
//! Every log line is mirrored to two sinks:
//!
//! 1. stderr -- keeps the historical "caw: " prefix so terminal users can keep grepping
//!    (cargo run 2>&1 | grep 'caw:'), followed by the timestamp and level tag.
//! 2. file -- ~/.config/caw/caw.log (see paths::log_path), with size-based rotation:
//!    at most 2 MiB per file, 5 rotated files kept (caw.log.1 ... caw.log.5, oldest
//!    dropped first).
//!
//! Thread-safe (a single Mutex around the open file), and it never panics: if the log
//! file cannot be opened the logger degrades to console-only output.
//!
//! # Usage
//!
//! Call init_logging once at the very start of main, then use the macros:
//!
//! ```text
//! // src/main.rs
//! #[macro_use]
//! mod logging;
//!
//! fn main() {
//!     logging::init_logging();
//!     log_info!("scan complete: {} tracks", count);
//!     log_warn!("no music directories configured");
//!     log_error!("failed to open database: {err}");
//!     log_debug!("queue index = {}", idx);
//! }
//! ```
//!
//! The macros are #[macro_export]ed, so they are reachable from every module as
//! crate::log_info!(...) even without the #[macro_use] attribute. They accept the same
//! arguments as format!.
//!
//! Timestamps are UTC (2026-10-21T02:30:00.123Z); formatting them locally would need a
//! timezone database, which is not worth a dependency here.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths;

/// Rotate once the active log file would exceed this size (2 MiB).
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

/// Number of rotated files to keep (caw.log.1 is newest, caw.log.5 oldest).
const MAX_BACKUPS: usize = 5;

/// Severity tag written into each line.
///
/// allow(dead_code): variants are constructed by the exported log_*! macros, which
/// dead-code analysis cannot see; not every level is used at every stage.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub const fn as_str(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

/// Open log file plus the size we believe it has, shared across threads.
struct Sink {
    file: Option<File>,
    size: u64,
    path: PathBuf,
}

static LOGGER: OnceLock<Mutex<Sink>> = OnceLock::new();

/// Initialize the file logger. Idempotent; safe to call more than once.
///
/// Creates ~/.config/caw/ if missing and appends to caw.log. Failure to open the
/// file is reported on stderr and logging continues to the console only.
pub fn init_logging() {
    let path = paths::log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let (file, size) = match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => {
            let size = file.metadata().map(|m| m.len()).unwrap_or(0);
            (Some(file), size)
        }
        Err(err) => {
            eprintln!(
                "caw: cannot open log file {}: {err}; logging to console only",
                path.display()
            );
            (None, 0)
        }
    };

    let already_open = LOGGER
        .set(Mutex::new(Sink {
            file,
            size,
            path: path.clone(),
        }))
        .is_err();

    // Route panics into the log file as well; the default hook still runs below.
    if !already_open {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let location = info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_else(|| "<unknown>".to_owned());
            let payload = panic_payload(info.payload());
            log_line(Level::Error, &format!("panic at {location}: {payload}"));
            default_hook(info);
        }));
    }

    log_line(Level::Info, "logging initialized");
}

/// Write one line at the given level. This is the primitive the macros use.
///
/// Never panics: an unusable mutex (poisoned by a previous panic) is recovered.
pub fn log_line(level: Level, message: &str) {
    let timestamp = timestamp_now();
    let tag = level.as_str();

    // Console first: even a broken log file must not lose the message.
    for line in message.lines() {
        eprintln!("caw: [{timestamp}] [{tag}] {line}");
    }

    let Some(lock) = LOGGER.get() else {
        return; // init_logging() not called yet: console output only
    };
    let mut sink = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if sink.file.is_none() {
        return;
    }

    for line in message.lines() {
        let record = format!("[{timestamp}] [{tag}] {line}\n");
        let record_len = record.len() as u64;

        if sink.size > 0 && sink.size + record_len > MAX_LOG_BYTES {
            rotate(&mut sink);
            if sink.file.is_none() {
                return;
            }
        }

        match sink.file.as_mut() {
            Some(file) => match file.write_all(record.as_bytes()) {
                Ok(()) => sink.size += record_len,
                Err(err) => {
                    eprintln!("caw: log write failed for {}: {err}", sink.path.display());
                    sink.file = None; // stop trying; console output continues
                    return;
                }
            },
            None => return,
        }
    }
}

/// Flush file writes. The file is unbuffered in practice (one write_all per line),
/// but this keeps shutdown paths explicit and cheap.
#[allow(dead_code)] // called from shutdown paths once platform integration lands
pub fn flush() {
    if let Some(lock) = LOGGER.get() {
        let mut sink = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(file) = sink.file.as_mut() {
            let _ = file.flush();
        }
    }
}

/// Rotate caw.log -> caw.log.1, shifting older backups up and dropping the oldest.
fn rotate(sink: &mut Sink) {
    sink.file = None; // close before renaming

    let base = sink.path.clone();
    let _ = std::fs::remove_file(backup_path(&base, MAX_BACKUPS));
    for index in (1..MAX_BACKUPS).rev() {
        let from = backup_path(&base, index);
        if from.exists() {
            let _ = std::fs::rename(&from, backup_path(&base, index + 1));
        }
    }
    let _ = std::fs::rename(&base, backup_path(&base, 1));

    match OpenOptions::new().create(true).append(true).open(&base) {
        Ok(file) => {
            sink.file = Some(file);
            sink.size = 0;
        }
        Err(err) => {
            eprintln!(
                "caw: cannot reopen log file after rotation {}: {err}",
                base.display()
            );
            sink.size = 0;
        }
    }
}

/// caw.log + 1 -> caw.log.1.
fn backup_path(base: &Path, index: usize) -> PathBuf {
    let name = base
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "caw.log".to_owned());
    base.with_file_name(format!("{name}.{index}"))
}

fn panic_payload(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_owned()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "<non-string panic payload>".to_owned()
    }
}

/// YYYY-MM-DDTHH:MM:SS.mmmZ in UTC, without pulling in a date/time crate.
fn timestamp_now() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();

    let days = (secs / 86_400) as i64;
    let time_of_day = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    let (hour, minute, second) = (
        time_of_day / 3_600,
        (time_of_day % 3_600) / 60,
        time_of_day % 60,
    );

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// Days since 1970-01-01 -> (year, month, day). Howard Hinnant's civil_from_days.
fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Log at Level::Debug.
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logging::log_line($crate::logging::Level::Debug, &format!($($arg)*))
    };
}

/// Log at Level::Info.
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logging::log_line($crate::logging::Level::Info, &format!($($arg)*))
    };
}

/// Log at Level::Warn.
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logging::log_line($crate::logging::Level::Warn, &format!($($arg)*))
    };
}

/// Log at Level::Error.
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logging::log_line($crate::logging::Level::Error, &format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates_round_trip() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1)); // 2024-01-01
        assert_eq!(civil_from_days(20_747), (2026, 10, 21));
    }

    #[test]
    fn backup_names_are_suffixed() {
        let base = PathBuf::from("/tmp/caw.log");
        assert_eq!(backup_path(&base, 1), PathBuf::from("/tmp/caw.log.1"));
        assert_eq!(backup_path(&base, 5), PathBuf::from("/tmp/caw.log.5"));
    }
}
