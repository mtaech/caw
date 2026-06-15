//! Icon helpers built on top of `egui_material_icons`.
//!
//! `egui_material_icons` ships Google Material Symbols as an egui font, so every
//! glyph is a real vector outline — no tofu (□ / ?) risk, regardless of which
//! CJK/UI font is loaded. We just re-export the constants we use and provide a
//! tiny `icon_button` wrapper that lets callers control size + colour (the
//! crate's own `icon_button` is hardcoded to size 18 and has no colour arg).
//!
//! All icons come from `egui_material_icons::icons`; see that module for the
//! full catalogue. The Material Symbol font family is registered in `main.rs`
//! via `egui_material_icons::initialize(&ctx)`.

use egui::{Color32, RichText, Ui, Vec2};
use egui_material_icons::icons;
use egui_material_icons::MaterialIcon;

// ── Re-exported icon constants used across the app ──────────────
pub use icons::{
    ICON_ALBUM, ICON_ARTIST, ICON_FOLDER, ICON_MUSIC_NOTE, ICON_PAUSE, ICON_PLAY_ARROW,
    ICON_QUEUE_MUSIC, ICON_REPEAT, ICON_REPEAT_ONE, ICON_SETTINGS, ICON_SHUFFLE,
    ICON_SKIP_NEXT, ICON_SKIP_PREVIOUS, ICON_VOLUME_DOWN, ICON_VOLUME_OFF, ICON_VOLUME_UP,
};

/// Render an icon as a `RichText` at the given size and colour.
pub fn rich(icon: MaterialIcon, size: f32, color: Color32) -> RichText {
    icon.rich_text().size(size).color(color)
}

/// A frameless, square clickable icon button.
///
/// `size` is the clickable square; the glyph is drawn a bit smaller.
/// `id` is currently unused (egui's `Button` auto-ids by content), but kept
/// in the signature for future explicit-id needs. Returns the button `Response`.
#[allow(unused_variables)]
pub fn icon_button(ui: &mut Ui, id: impl std::hash::Hash, icon: MaterialIcon, size: f32, color: Color32) -> egui::Response {
    let btn = egui::Button::new(rich(icon, size * 0.62, color))
        .frame(false)
        .min_size(Vec2::splat(size));
    ui.add(btn)
}

/// A filled circular button (used for the primary play/pause control).
pub fn filled_circle_button(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    icon: MaterialIcon,
    diameter: f32,
    fill: Color32,
) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(diameter), egui::Sense::click());
    let resp = ui.interact(rect, ui.id().with(id), egui::Sense::click());
    let actual_fill = if resp.hovered() {
        Color32::from_rgb(fill.r().saturating_add(12), fill.g().saturating_add(20), fill.b().saturating_add(12))
    } else {
        fill
    };
    ui.painter().circle_filled(rect.center(), diameter * 0.5, actual_fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon.codepoint,
        egui::FontId::new(diameter * 0.5, icon.font_family()),
        Color32::WHITE,
    );
    resp
}
