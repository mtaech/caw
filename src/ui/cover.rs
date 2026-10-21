//! Deterministic cover placeholders for albums and artists without embedded art.
//!
//! A cover-less item gets a stable two-stop gradient derived from its name, so
//! the same album or artist keeps the same placeholder across restarts. Colors
//! are built from a hue rather than a fixed palette, and their lightness follows
//! the active theme so the placeholder reads correctly in both modes.

use gpui_kit::component::Theme;
use gpui_kit::*;

/// Stable hue in `0..1` derived from a text key (FNV-1a: stable across runs).
fn hue(seed: &str) -> f32 {
    let mut hash: u32 = 2_166_136_261;
    for byte in seed.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    (hash % 360) as f32 / 360.0
}

/// Gradient background for a cover placeholder seeded by `seed`.
pub fn gradient(seed: &str, theme: &Theme) -> Background {
    let hue = hue(seed);
    let (start_lightness, end_lightness, saturation) = if theme.mode.is_dark() {
        (0.34, 0.22, 0.36)
    } else {
        (0.84, 0.70, 0.42)
    };

    linear_gradient(
        135.0,
        linear_color_stop(hsla(hue, saturation, start_lightness, 1.0), 0.0),
        linear_color_stop(
            hsla((hue + 0.12) % 1.0, saturation, end_lightness, 1.0),
            1.0,
        ),
    )
}

/// Foreground color for the glyph drawn on a placeholder.
pub fn icon_color(theme: &Theme) -> Hsla {
    theme.foreground.alpha(0.6)
}
