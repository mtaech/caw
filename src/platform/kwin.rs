//! KDE-specific window hints (hide from taskbar).
//!
//! Wayland has no protocol for "remove this window from the taskbar"; apps normally
//! unmap their surface, which GPUI cannot do without destroying the window (and the
//! PlayerApp state with it). KWin exposes `skipTaskbar`/`skipPager` on its window
//! objects through its scripting D-Bus interface, and Plasma's task manager honors
//! them, so on KDE we flip those flags from a short-lived helper thread.
//!
//! On other desktops there is no equivalent: the call logs once and does nothing,
//! and close-to-tray degrades to a plain minimize.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Ask KWin to include or exclude Caw's windows from the taskbar and pager.
///
/// When `visible` is true the window is also unminimized (Wayland has no
/// unminimize call in GPUI), so restoring from the tray brings it back.
/// Runs on a detached thread: the D-Bus round trips must never block the UI.
pub fn set_taskbar_visible(visible: bool) {
    if !is_kde() {
        return;
    }

    let pid = std::process::id();
    std::thread::Builder::new()
        .name("caw-kwin-taskbar".to_owned())
        .spawn(move || {
            if let Err(err) = apply(pid, visible) {
                crate::log_debug!("kwin: could not set taskbar visible={visible}: {err}");
            } else {
                crate::log_debug!("kwin: taskbar visible={visible} for pid {pid}");
            }
        })
        .ok();
}

fn is_kde() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|desktop| desktop.to_uppercase().contains("KDE"))
        .unwrap_or(false)
}

fn apply(pid: u32, visible: bool) -> zbus::Result<()> {
    let skip = !visible;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let plugin = format!("caw-taskbar-{pid}-{stamp}");
    let script_path = script_path(stamp);

    let unminimize = if visible {
        // KWin is the compositor, so this also works when Wayland refuses a
        // plain activate request from the application.
        " w.minimized = false; workspace.activeWindow = w;"
    } else {
        ""
    };
    let script = format!(
        "workspace.windowList().forEach(function (w) {{ \
             if (w.pid === {pid}) {{ w.skipTaskbar = {skip}; w.skipPager = {skip};{unminimize} }} \
         }});\n"
    );
    std::fs::write(&script_path, script)
        .map_err(|err| zbus::Error::Failure(format!("write script: {err}")))?;

    let result = (|| -> zbus::Result<()> {
        let connection = zbus::blocking::Connection::session()?;
        let path = script_path.to_string_lossy().to_string();
        connection.call_method(
            Some("org.kde.KWin"),
            "/Scripting",
            Some("org.kde.kwin.Scripting"),
            "loadScript",
            &(path.as_str(), plugin.as_str()),
        )?;
        connection.call_method(
            Some("org.kde.KWin"),
            "/Scripting",
            Some("org.kde.kwin.Scripting"),
            "start",
            &(),
        )?;
        connection.call_method(
            Some("org.kde.KWin"),
            "/Scripting",
            Some("org.kde.kwin.Scripting"),
            "unloadScript",
            &(plugin.as_str(),),
        )?;
        Ok(())
    })();

    let _ = std::fs::remove_file(&script_path);
    result
}

fn script_path(stamp: u128) -> PathBuf {
    std::env::temp_dir().join(format!("caw-kwin-taskbar-{stamp}.js"))
}
