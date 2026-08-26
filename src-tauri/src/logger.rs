use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::paths;

const SEVEN_DAYS_SECS: u64 = 7 * 24 * 60 * 60; // 7 days in seconds

static LOG_MUTEX: Mutex<Option<Instant>> = Mutex::new(None);

/// Format system time as "YYYY-MM-DD" and "YYYY-MM-DD HH:MM:SS"
fn format_now() -> (String, String) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let days = (secs / 86400) as i64;
    let time_of_day = (secs % 86400) as u32;

    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    let (year, month, day) = days_to_ymd(days);

    let date_str = format!("{year:04}-{month:02}-{day:02}");
    let time_str = format!("{date_str} {hours:02}:{minutes:02}:{seconds:02}");
    (date_str, time_str)
}

fn days_to_ymd(mut days: i64) -> (i32, u32, u32) {
    let mut year = 1970;
    loop {
        let leap = is_leap_year(year);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap_year(year);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, (days + 1) as u32)
}

fn is_leap_year(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

/// Automatically delete log files older than 7 days in ~/.config/caw/logs/ and ~/.config/caw/
pub fn clean_old_logs() {
    let now = SystemTime::now();

    let scan_dirs = [paths::logs_dir(), paths::caw_dir()];

    for dir in scan_dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();

            // Match log files (e.g. caw-2025-05-18.log, caw.log.old, *.log)
            if !file_name.ends_with(".log")
                && !file_name.contains(".log.")
                && !file_name.ends_with(".old")
            {
                continue;
            }

            // Do not delete active caw.log unless it's an old backup
            if file_name == "caw.log" {
                continue;
            }

            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age.as_secs() > SEVEN_DAYS_SECS {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }
}

/// Write a log line to stdout/stderr and to daily rotated log file ~/.config/caw/logs/caw-YYYY-MM-DD.log
/// as well as the active ~/.config/caw/caw.log
pub fn write_log(level: &str, msg: &str) {
    let mut last_clean = LOG_MUTEX.lock().unwrap();

    // Clean old logs once per hour (or on first invocation)
    let should_clean = match *last_clean {
        Some(instant) => instant.elapsed() >= Duration::from_secs(3600),
        None => true,
    };

    if should_clean {
        *last_clean = Some(Instant::now());
        clean_old_logs();
    }

    let (date_str, timestamp) = format_now();
    let formatted = format!("[{timestamp}] [{level}] {msg}\n");

    // 1. Output to stderr
    eprint!("{formatted}");

    // 2. Append to active log (~/.config/caw/caw.log)
    let main_log = paths::log_path();
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&main_log)
    {
        let _ = file.write_all(formatted.as_bytes());
    }

    // 3. Append to dated log (~/.config/caw/logs/caw-YYYY-MM-DD.log) for rotation
    let daily_log = paths::logs_dir().join(format!("caw-{date_str}.log"));
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&daily_log)
    {
        let _ = file.write_all(formatted.as_bytes());
    }

    // If main caw.log grows beyond 5MB, rotate it to logs/caw-YYYY-MM-DD-backup.log
    if let Ok(meta) = std::fs::metadata(&main_log) {
        if meta.len() > 5 * 1024 * 1024 {
            let backup = paths::logs_dir().join(format!("caw-{date_str}-backup.log"));
            let _ = std::fs::rename(&main_log, backup);
        }
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logger::write_log("INFO", &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::write_log("ERROR", &format!($($arg)*))
    };
}
