use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

use egui::{Color32, ColorImage, Rect, Rounding, TextureHandle, TextureOptions};

/// Process-global cache for decoded cover images, keyed by `Track.id`.
static COVER_CACHE: std::sync::OnceLock<RwLock<HashMap<u64, TextureHandle>>> =
    std::sync::OnceLock::new();

fn cover_cache() -> &'static RwLock<HashMap<u64, TextureHandle>> {
    COVER_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Draw a square cover at the given `rect` (caller allocates space).
pub fn cover(
    cover_data: Option<&Arc<Vec<u8>>>,
    seed: &str,
    id: u64,
    ui: &egui::Ui,
    rect: Rect,
) {
    let cache = cover_cache();
    let rounding = Rounding::same((rect.size().x * 0.09) as u8);

    let have_texture = cache.read().unwrap().contains_key(&id)
        || cover_data.map_or(false, |bytes| {
            try_decode_cover(bytes, ui.ctx())
                .map(|handle| cache.write().unwrap().insert(id, handle))
                .is_some()
        });

    if have_texture {
        if let Some(handle) = cache.read().unwrap().get(&id) {
            ui.painter().image(
                handle.id(),
                rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            return;
        }
    }

    // Fallback placeholder
    let (bg_color, mute_color) = seed_color(seed);
    let painter = ui.painter();
    painter.rect_filled(rect, rounding, bg_color);

    // Draw a Material music_note glyph (vector font, no tofu).
    let size = rect.size().x;
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        crate::ui::icons::ICON_MUSIC_NOTE.codepoint,
        egui::FontId::new(size * 0.5, crate::ui::icons::ICON_MUSIC_NOTE.font_family()),
        mute_color,
    );
}

/// Decode raw image bytes into an egui `TextureHandle`.
fn try_decode_cover(bytes: &[u8], ctx: &egui::Context) -> Option<TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let pixels = rgba.into_vec();
    let color_image = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels);
    Some(ctx.load_texture(
        format!("caw-cover-{}", fast_id(&pixels)),
        color_image,
        TextureOptions::LINEAR,
    ))
}

fn fast_id(data: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish() & 0xFFFFFFFF
}

fn seed_color(seed: &str) -> (Color32, Color32) {
    if seed.is_empty() {
        return (Color32::from_rgb(160, 130, 110), Color32::from_rgb(60, 60, 60));
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    let hash = hasher.finish();
    let h = (hash % 360) as f32 / 360.0;
    let (r, g, b) = hsl_to_rgb(h, 0.45, 0.55);
    (Color32::from_rgb(r, g, b), Color32::from_rgb((r as f32 * 0.5) as u8, (g as f32 * 0.5) as u8, (b as f32 * 0.5) as u8))
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    if s == 0.0 {
        let v = (l * 255.0) as u8;
        return (v, v, v);
    }
    let hue2rgb = |t: f32| -> f32 {
        let t = if t < 0.0 { t + 1.0 } else if t > 1.0 { t - 1.0 } else { t };
        if t < 1.0 / 6.0 { l + (s - l) * 6.0 * t }
        else if t < 0.5 { s }
        else if t < 2.0 / 3.0 { l + (s - l) * (2.0 / 3.0 - t) * 6.0 }
        else { l }
    };
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let _p = 2.0 * l - q;
    (
        (hue2rgb(h + 1.0 / 3.0) * 255.0) as u8,
        (hue2rgb(h) * 255.0) as u8,
        (hue2rgb(h - 1.0 / 3.0) * 255.0) as u8,
    )
}
