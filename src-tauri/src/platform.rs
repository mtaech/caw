/// Wayland / platform detection utilities.
///
/// Tauri v2's tray-icon feature uses GTK StatusIcon internally, which is an
/// X11 protocol and silently fails on Wayland. These helpers let us gracefully
/// skip features that don't work outside X11.
///
/// When Tauri adds native Wayland tray support (StatusNotifierItem), delete
/// this file and revert to unconditional tray setup.

/// Returns `true` when the compositor is Wayland.
pub fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .ok()
        .map(|s| s.to_lowercase() == "wayland")
        .unwrap_or(false)
}

/// Brief description of why a feature was skipped on the current platform.
pub fn skip_reason(name: &str) -> Option<String> {
    if is_wayland() {
        Some(format!(
            "{}: not supported on Wayland (X11 protocol)",
            name
        ))
    } else {
        None
    }
}
