use std::collections::HashSet;

use egui::{Align2, Color32, Rounding, Ui, Vec2};
use egui_material_icons::MaterialIcon;

use crate::app::{NavItem, PlayerApp};
use crate::ui::icons;

/// Render the left sidebar navigation panel.
pub fn render_sidebar(ui: &mut Ui, app: &mut PlayerApp) {
    let selected = app.selected_nav;
    let library = &app.library;

    let track_count = library.len();
    let artist_count = {
        let mut set = HashSet::new();
        for t in library.iter().take(1024) {
            set.insert(t.display_artist().to_string());
        }
        if library.len() > 1024 {
            library.len() / 4
        } else {
            set.len()
        }
    };
    let album_count = {
        let mut set = HashSet::new();
        for t in library.iter().take(1024) {
            set.insert(t.album.clone());
        }
        if library.len() > 1024 {
            library.len() / 8
        } else {
            set.len()
        }
    };

    // Dark background
    ui.painter().rect_filled(ui.max_rect(), Rounding::ZERO, Color32::from_rgb(0x00, 0x00, 0x00));

    // ── Logo ──────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        let m_rect = ui.allocate_exact_size(Vec2::new(2.0, 20.0), egui::Sense::hover()).0;
        ui.painter()
            .rect_filled(m_rect, Rounding::same(2), Color32::from_rgb(0x1d, 0xb9, 0x54));
        ui.add_space(10.0);
        ui.label(egui::RichText::new("Caw").size(16.0).strong().color(Color32::WHITE));
    });
    ui.add_space(12.0);

    // ── Group label ───────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(egui::RichText::new("我的资料库").size(11.0).color(Color32::from_rgb(0x6a, 0x6a, 0x6a)));
    });
    ui.add_space(4.0);

    // ── Nav items ─────────────────────────────────────────────
    let entries: &[(&str, MaterialIcon, NavItem, Option<usize>)] = &[
        ("全部音乐", icons::ICON_MUSIC_NOTE, NavItem::AllMusic, Some(track_count)),
        ("艺术家", icons::ICON_ARTIST, NavItem::Artists, Some(artist_count)),
        ("专辑", icons::ICON_ALBUM, NavItem::Albums, Some(album_count)),
        ("播放列表", icons::ICON_QUEUE_MUSIC, NavItem::Playlists, None),
        ("文件夹", icons::ICON_FOLDER, NavItem::Folders, None),
    ];

    for &(label, icon, item, count) in entries {
        render_nav_entry(ui, app, icon, label, item, selected, count);
    }

    // Spacer pushing settings to the bottom
    ui.allocate_space(Vec2::new(0.0, ui.available_height().max(0.0)));

    // ── Settings ──────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.add_space(16.0 + 3.0);
        let ir = ui.allocate_exact_size(Vec2::new(16.0, 40.0), egui::Sense::hover()).0;
        ui.painter().text(
            ir.center(),
            Align2::CENTER_CENTER,
            icons::ICON_SETTINGS.codepoint,
            egui::FontId::new(16.0, icons::ICON_SETTINGS.font_family()),
            Color32::from_rgb(0xa7, 0xa7, 0xa7),
        );
        ui.add_space(12.0);
        ui.label(egui::RichText::new("设置").size(13.0).color(Color32::from_rgb(0xa7, 0xa7, 0xa7)));
    });
}

fn render_nav_entry(
    ui: &mut Ui,
    app: &mut PlayerApp,
    icon: MaterialIcon,
    label: &str,
    item: NavItem,
    selected: NavItem,
    count: Option<usize>,
) {
    let is_sel = item == selected;
    let text_color = if is_sel {
        Color32::WHITE
    } else {
        Color32::from_rgb(0xa7, 0xa7, 0xa7)
    };

    let avail_w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, 40.0), egui::Sense::click());
    let painter = ui.painter();

    let resp = ui.interact(rect, ui.id().with(("nav", item)), egui::Sense::click());
    if resp.hovered() || is_sel {
        let alpha: u8 = if is_sel { 16 } else { 8 };
        painter.rect_filled(rect, Rounding::ZERO, Color32::from_rgba_premultiplied(255, 255, 255, alpha));
    }

    // Left selection bar
    let bar = egui::Rect::from_min_max(
        egui::pos2(rect.min.x, rect.min.y + 10.0),
        egui::pos2(rect.min.x + 3.0, rect.min.y + 30.0),
    );
    painter.rect_filled(
        bar,
        Rounding::same(2),
        if is_sel {
            Color32::from_rgb(0x1d, 0xb9, 0x54)
        } else {
            Color32::TRANSPARENT
        },
    );

    // Icon (Material glyph — font-independent, no tofu)
    let icon_center = egui::pos2(rect.min.x + 16.0 + 3.0 + 12.0 + 8.0, rect.center().y);
    painter.text(
        icon_center,
        Align2::CENTER_CENTER,
        icon.codepoint,
        egui::FontId::new(18.0, icon.font_family()),
        text_color,
    );

    // Label
    painter.text(
        egui::pos2(rect.min.x + 16.0 + 3.0 + 12.0 + 16.0 + 12.0 + 4.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        text_color,
    );

    // Count
    if let Some(c) = count {
        let ct = c.to_string();
        painter.text(
            egui::pos2(rect.max.x - 16.0, rect.center().y),
            Align2::RIGHT_CENTER,
            &ct,
            egui::FontId::proportional(11.0),
            Color32::from_rgb(0x6a, 0x6a, 0x6a),
        );
    }

    if resp.clicked() {
        app.set_nav(item);
    }
}
