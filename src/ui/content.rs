use std::collections::BTreeMap;
use std::sync::Arc;

use egui::{Align2, Color32, Rect, Rounding, Ui, Vec2};

use crate::app::{DetailTarget, NavItem, PlayerApp, SortKey};
use crate::models::playlist::PlaybackState;
use crate::ui::cover;

/// Render the central content area (top bar + body).
pub fn render_content(ui: &mut Ui, app: &mut PlayerApp, search_text: &mut String) {
    let scanning = app.scanning;
    let library_count = app.library.len();
    let filtered_count = app.filtered_count();
    let is_detail = app.detail.is_some();
    let selected_nav = app.selected_nav;

    render_top_bar(ui, app, search_text);

    // Separator
    let sep_rect = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover()).0;
    ui.painter().rect_filled(sep_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(0, 0, 0, 12));

    if scanning {
        render_scanning(ui);
    } else if filtered_count == 0 {
        render_empty(ui, library_count);
    } else if is_detail {
        render_detail_view(ui, app);
    } else {
        match selected_nav {
            NavItem::Albums => render_album_grid(ui, app),
            NavItem::Artists => render_artist_list(ui, app),
            _ => render_track_table(ui, app),
        }
    }
}

// ── Top Bar ──────────────────────────────────────────────────────

fn render_top_bar(ui: &mut Ui, app: &mut PlayerApp, search_text: &mut String) {
    let is_detail = app.detail.is_some();
    let total_secs = app.total_duration_secs();
    let count = app.filtered_count();

    let title = if let Some(ref target) = app.detail {
        match target {
            DetailTarget::Album(name) | DetailTarget::Artist(name) => name.clone(),
        }
    } else {
        match app.selected_nav {
            NavItem::AllMusic => "全部音乐",
            NavItem::Artists => "艺术家",
            NavItem::Albums => "专辑",
            NavItem::Playlists => "播放列表",
            NavItem::Folders => "文件夹",
        }
        .to_string()
    };

    let subtitle = format!("{} 首 · {}", count, format_total_duration(total_secs));

    let tb_rect = ui.allocate_exact_size(Vec2::new(ui.available_width(), 64.0), egui::Sense::hover()).0;
    let painter = ui.painter();

    painter.rect_filled(tb_rect, Rounding::ZERO, Color32::from_rgb(0x12, 0x12, 0x12));

    if is_detail {
        let back_rect = Rect::from_min_max(
            egui::pos2(tb_rect.min.x + 24.0, tb_rect.min.y + 12.0),
            egui::pos2(tb_rect.min.x + 60.0, tb_rect.min.y + 40.0),
        );
        if ui.interact(back_rect, ui.id().with("detail-back"), egui::Sense::click()).clicked() {
            app.close_detail();
        }
        painter.text(
            egui::pos2(back_rect.min.x, tb_rect.center().y),
            Align2::LEFT_CENTER,
            "← 返回",
            egui::FontId::proportional(14.0),
            Color32::from_rgb(0xa7, 0xa7, 0xa7),
        );
        painter.text(
            egui::pos2(tb_rect.min.x + 24.0 + 60.0, tb_rect.center().y),
            Align2::LEFT_CENTER,
            &title,
            egui::FontId::proportional(18.0),
            Color32::WHITE,
        );
    } else {
        painter.text(
            egui::pos2(tb_rect.min.x + 24.0, tb_rect.min.y + 14.0),
            Align2::LEFT_TOP,
            &title,
            egui::FontId::proportional(18.0),
            Color32::WHITE,
        );
        painter.text(
            egui::pos2(tb_rect.min.x + 24.0, tb_rect.min.y + 38.0),
            Align2::LEFT_TOP,
            &subtitle,
            egui::FontId::proportional(13.0),
            Color32::from_rgb(0xa7, 0xa7, 0xa7),
        );

        // Search box
        let search_rect = Rect::from_min_max(
            egui::pos2(tb_rect.max.x - 24.0 - 320.0, tb_rect.min.y + 14.0),
            egui::pos2(tb_rect.max.x - 24.0, tb_rect.min.y + 50.0),
        );
        painter.rect_filled(search_rect, Rounding::same(8), Color32::from_rgb(0x1a, 0x1a, 0x1a));

        let text_id = egui::Id::new("search-input");
        let resp = ui.put(
            search_rect,
            egui::TextEdit::singleline(search_text)
                .id(text_id)
                .text_color(Color32::from_rgb(0xcc, 0xcc, 0xcc))
                .hint_text("搜索音乐..."),
        );
        if resp.changed() {
            app.set_search(search_text.clone());
        }
    }
}

// ── Detail View ─────────────────────────────────────────────────

fn render_detail_view(ui: &mut Ui, app: &mut PlayerApp) {
    let first_track = app.get_filtered_track(0);
    let detail_name = match &app.detail {
        Some(DetailTarget::Album(name)) | Some(DetailTarget::Artist(name)) => name.clone(),
        None => return,
    };
    let is_album = matches!(app.detail, Some(DetailTarget::Album(_)));
    let (cover_data, artist_name, track_id) = if let Some(t) = first_track {
        (t.cover_data.clone(), t.display_artist().to_string(), t.id)
    } else {
        (None, String::new(), 0)
    };
    let total_secs = app.total_duration_secs();
    let count = app.filtered_count();

    ui.horizontal(|ui| {
        ui.add_space(24.0);
        let (c_rect, _) = ui.allocate_exact_size(Vec2::splat(180.0), egui::Sense::hover());
        cover::cover(cover_data.as_ref(), &detail_name, track_id, ui, c_rect);
        ui.add_space(24.0);

        ui.vertical(|ui| {
            ui.add_space(20.0);
            ui.label(egui::RichText::new(&detail_name).size(24.0).strong().color(Color32::WHITE));
            if is_album {
                ui.label(egui::RichText::new(&artist_name).size(13.0).color(Color32::from_rgb(0xa7, 0xa7, 0xa7)));
            }
            ui.label(egui::RichText::new(format!("{} 首 · {}", count, format_total_duration(total_secs))).size(13.0).color(Color32::from_rgb(0xa7, 0xa7, 0xa7)));

            if ui.add(
                egui::Button::new(egui::RichText::new("▶ 播放").size(14.0).color(Color32::WHITE))
                    .fill(Color32::from_rgb(0x1d, 0xb9, 0x54))
                    .min_size(Vec2::new(120.0, 40.0)),
            ).clicked() {
                app.play_at(0);
            }
        });
    });

    ui.add_space(12.0);
    let sep = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover()).0;
    ui.painter().rect_filled(sep, Rounding::ZERO, Color32::from_rgba_premultiplied(255, 255, 255, 10));

    render_track_table(ui, app);
}

// ── States ───────────────────────────────────────────────────────

fn render_scanning(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(180.0);
        ui.add(egui::Spinner::new());
        ui.add_space(12.0);
        ui.label(egui::RichText::new("正在扫描音乐文件…").size(18.0).color(Color32::WHITE));
        ui.add_space(8.0);
        ui.label(egui::RichText::new("正在浏览 ~/Music 等目录").size(13.0).color(Color32::from_rgb(0xa7, 0xa7, 0xa7)));
    });
}

fn render_empty(ui: &mut Ui, library_count: usize) {
    ui.vertical_centered(|ui| {
        ui.add_space(180.0);
        ui.label(egui::RichText::new("🎵").size(36.0));
        ui.add_space(12.0);
        let msg = if library_count == 0 { "没有找到音乐文件" } else { "无匹配结果" };
        ui.label(egui::RichText::new(msg).size(18.0).color(Color32::WHITE));
        ui.add_space(8.0);
        let sub = if library_count == 0 {
            "请将音乐文件放在 ~/Music 目录后重启应用"
        } else {
            "尝试其他搜索关键词或切换导航分类"
        };
        ui.label(egui::RichText::new(sub).size(13.0).color(Color32::from_rgb(0xa7, 0xa7, 0xa7)));
    });
}

// ── Album Grid ──────────────────────────────────────────────────

fn render_album_grid(ui: &mut Ui, app: &mut PlayerApp) {
    let mut album_map: BTreeMap<String, (String, usize, u64, Option<Arc<Vec<u8>>>)> = BTreeMap::new();
    for &idx in &app.filtered.indices {
        if let Some(track) = app.library.get(idx) {
            let entry = album_map
                .entry(track.album.clone())
                .or_insert_with(|| (track.display_artist().to_string(), 0, track.id, track.cover_data.clone()));
            entry.1 += 1;
        }
    }

    if album_map.is_empty() {
        render_empty(ui, 0);
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.add_space(24.0);
            let card_w = 180.0;
            let gap = 20.0;
            let card_full = card_w + gap;
            let avail_w = ui.available_width();
            let margin = 24.0;
            let cols = ((avail_w - margin) / card_full).floor() as usize;

            let mut col = 0usize;
            for (album_name, (artist, track_count, first_track_id, cover_data)) in album_map {
                if col >= cols {
                    ui.add_space(8.0);
                    col = 0;
                }

                let (c_rect, _) = ui.allocate_exact_size(Vec2::new(card_full, 240.0), egui::Sense::click());
                let p = ui.painter();

                let cover_rect = Rect::from_min_max(
                    egui::pos2(c_rect.min.x + 8.0, c_rect.min.y + 8.0),
                    egui::pos2(c_rect.min.x + card_w - 8.0, c_rect.min.y + card_w - 8.0),
                );
                cover::cover(cover_data.as_ref(), &album_name, first_track_id, ui, cover_rect);

                p.text(
                    egui::pos2(cover_rect.min.x, c_rect.min.y + card_w + 4.0),
                    Align2::LEFT_TOP,
                    &album_name,
                    egui::FontId::proportional(14.0),
                    Color32::WHITE,
                );
                p.text(
                    egui::pos2(cover_rect.min.x, c_rect.min.y + card_w + 24.0),
                    Align2::LEFT_TOP,
                    &format!("{} · {} 首", artist, track_count),
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(0xa7, 0xa7, 0xa7),
                );

                if ui.interact(c_rect, ui.id().with(("album-card", album_name.as_str())), egui::Sense::click()).clicked() {
                    app.open_album(album_name.clone());
                }

                col += 1;
            }
        });
}

// ── Artist List ─────────────────────────────────────────────────

fn render_artist_list(ui: &mut Ui, app: &mut PlayerApp) {
    let mut artist_map: BTreeMap<String, (usize, u64, Option<Arc<Vec<u8>>>)> = BTreeMap::new();
    for &idx in &app.filtered.indices {
        if let Some(track) = app.library.get(idx) {
            let artist = track.display_artist().to_string();
            let entry = artist_map.entry(artist).or_insert_with(|| (0, track.id, track.cover_data.clone()));
            entry.0 += 1;
        }
    }

    if artist_map.is_empty() {
        render_empty(ui, 0);
        return;
    }

    // Header
    let h_rect = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), egui::Sense::hover()).0;
    ui.painter().rect_filled(h_rect, Rounding::ZERO, Color32::from_rgb(0x1a, 0x1a, 0x1a));
    ui.painter().text(
        egui::pos2(h_rect.min.x + 24.0, h_rect.center().y),
        Align2::LEFT_CENTER,
        "艺术家",
        egui::FontId::proportional(11.0),
        Color32::from_rgb(0xa7, 0xa7, 0xa7),
    );

    for (artist_name, (track_count, artist_id, cover_data)) in artist_map {
        let (row_r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.0), egui::Sense::click());
        let p = ui.painter();
        let resp = ui.interact(row_r, ui.id().with(("artist-row", artist_name.as_str())), egui::Sense::click());
        if resp.hovered() {
            p.rect_filled(row_r, Rounding::ZERO, Color32::from_rgba_premultiplied(255, 255, 255, 8));
        }

        let av_rect = Rect::from_min_max(
            egui::pos2(row_r.min.x + 24.0, row_r.min.y + 8.0),
            egui::pos2(row_r.min.x + 64.0, row_r.min.y + 48.0),
        );
        cover::cover(cover_data.as_ref(), &artist_name, artist_id, ui, av_rect);

        p.text(
            egui::pos2(av_rect.max.x + 12.0, row_r.center().y),
            Align2::LEFT_CENTER,
            &artist_name,
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );
        p.text(
            egui::pos2(row_r.max.x - 80.0, row_r.center().y),
            Align2::LEFT_CENTER,
            &format!("{} 首", track_count),
            egui::FontId::proportional(13.0),
            Color32::from_rgb(0xa7, 0xa7, 0xa7),
        );
        p.text(
            egui::pos2(row_r.max.x - 24.0, row_r.center().y),
            Align2::CENTER_CENTER,
            "›",
            egui::FontId::proportional(18.0),
            Color32::from_rgb(0xa7, 0xa7, 0xa7),
        );

        if resp.clicked() {
            app.open_artist(artist_name.clone());
        }
    }
}

// ── Track Table ─────────────────────────────────────────────────

fn render_track_table(ui: &mut Ui, app: &mut PlayerApp) {
    let primary = Color32::from_rgb(0x1d, 0xb9, 0x54);
    let fg = Color32::WHITE;
    let muted = Color32::from_rgb(0xa7, 0xa7, 0xa7);
    let bg_odd = Color32::from_rgba_premultiplied(255, 255, 255, 6);

    let count = app.filtered_count();
    let current_sort = app.filtered.sort_key;

    // Table header
    let h_rect = ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), egui::Sense::hover()).0;
    ui.painter().rect_filled(h_rect, Rounding::ZERO, Color32::from_rgb(0x1a, 0x1a, 0x1a));

    // Column geometry — computed from available width so columns never overlap.
    // Layout (left→right): [pad][playing-bar 3][num 28][cover 32 + gap][title flex][artist flex][album flex][duration 56][pad]
    let pad = 16.0;
    let inner_w = (h_rect.width() - 2.0 * pad).max(200.0);
    let bar_w = 3.0;
    let num_w = 28.0;
    let cover_w = 32.0;
    let cover_gap = 12.0;
    let dur_w = 56.0;
    let fixed = bar_w + num_w + cover_w + cover_gap + dur_w;
    let flex_total = (inner_w - fixed).max(120.0);
    // title gets ~half of the flex space, artist & album split the rest.
    let title_w = flex_total * 0.46;
    let artist_w = flex_total * 0.27;
    let album_w = (flex_total - title_w - artist_w).max(80.0);

    // X positions (left edge of each column), starting after pad + bar + num + cover + gap
    let x_after_cover = h_rect.min.x + pad + bar_w + num_w + cover_w + cover_gap;
    let x_title = x_after_cover;
    let x_artist = x_title + title_w;
    let x_album = x_artist + artist_w;
    let x_duration = h_rect.max.x - pad - dur_w;

    let sort_cols: &[(&str, SortKey, f32, f32)] = &[
        ("标题", SortKey::Title, x_title, title_w),
        ("艺术家", SortKey::Artist, x_artist, artist_w),
        ("专辑", SortKey::Album, x_album, album_w),
        ("时长", SortKey::Duration, x_duration, dur_w),
    ];
    for &(lab, sk, cx, cw) in sort_cols {
        let is_active = current_sort == sk;
        let txt = if is_active { format!("{} ▾", lab) } else { lab.to_string() };
        let col_rect = Rect::from_min_max(egui::pos2(cx, h_rect.min.y), egui::pos2(cx + cw, h_rect.max.y));
        if ui.interact(col_rect, ui.id().with(("sort-col", lab)), egui::Sense::click()).clicked() {
            app.set_sort(sk);
        }
        ui.painter().text(
            egui::pos2(cx + 4.0, h_rect.center().y),
            Align2::LEFT_CENTER,
            &txt,
            egui::FontId::proportional(11.0),
            if is_active { fg } else { muted },
        );
    }

    // Helper to draw a text cell clipped to its column width (no overlap, no wrap).
    // Uses the ctx (cheap clone) for font layout and the painter for drawing so
    // it never borrows `ui` in a way that conflicts with row painting.
    let draw_clipped = |ctx: &egui::Context, painter: &egui::Painter, text: &str, x: f32, y: f32, w: f32, font_size: f32, color: Color32| {
        let font = egui::FontId::proportional(font_size);
        let max_w = (w - 8.0).max(8.0);
        let mut shown: String = text.to_string();
        let mut galley = ctx.fonts_mut(|f| f.layout_no_wrap(shown.clone(), font.clone(), color));
        let mut count = shown.chars().count();
        while galley.size().x > max_w && count > 1 {
            count -= 1;
            shown = shown.chars().take(count).collect::<String>() + "…";
            galley = ctx.fonts_mut(|f| f.layout_no_wrap(shown.clone(), font.clone(), color));
        }
        painter.galley(egui::pos2(x, y - galley.size().y * 0.5), galley, color);
    };

    // Track rows (scrollable)
    egui::ScrollArea::vertical().show(ui, |ui| {
        for i in 0..count {
            let track = match app.get_filtered_track(i) {
                Some(t) => (*t).clone(),
                None => continue,
            };

            let is_playing = app.current_index == Some(i)
                && app.playback_state == PlaybackState::Playing;
            let is_paused = app.current_index == Some(i)
                && app.playback_state == PlaybackState::Paused;
            let is_sel = app.selected_index == Some(i);

            let (row_r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 48.0), egui::Sense::click());
            let p = ui.painter();

            let bg = if is_sel {
                Color32::from_rgba_premultiplied(0, 0, 0, 15)
            } else if i % 2 == 0 {
                Color32::from_rgb(0x12, 0x12, 0x12)
            } else {
                bg_odd
            };
            p.rect_filled(row_r, Rounding::ZERO, bg);

            let resp = ui.interact(row_r, ui.id().with(("track-row", i)), egui::Sense::click());

            if resp.hovered() && !is_sel {
                p.rect_filled(row_r, Rounding::ZERO, Color32::from_rgba_premultiplied(255, 255, 255, 10));
            }

            if resp.clicked() {
                app.select_at(i);
            }
            if resp.double_clicked() {
                app.play_at(i);
            }

            // Left playing bar
            if is_playing || is_paused {
                p.rect_filled(
                    Rect::from_min_max(
                        egui::pos2(row_r.min.x, row_r.min.y),
                        egui::pos2(row_r.min.x + bar_w, row_r.max.y),
                    ),
                    Rounding::ZERO,
                    primary,
                );
            }

            // Number / play-pause indicator (Material glyph, font-independent)
            let num_color = if is_playing || is_paused { primary } else if is_sel { fg } else { muted };
            let num_center_x = row_r.min.x + pad + bar_w + num_w * 0.5;
            let icon_cp = if is_playing {
                Some(crate::ui::icons::ICON_PLAY_ARROW)
            } else if is_paused {
                Some(crate::ui::icons::ICON_PAUSE)
            } else {
                None
            };
            if let Some(ic) = icon_cp {
                p.text(
                    egui::pos2(num_center_x, row_r.center().y),
                    Align2::CENTER_CENTER,
                    ic.codepoint,
                    egui::FontId::new(16.0, ic.font_family()),
                    primary,
                );
            } else {
                p.text(
                    egui::pos2(num_center_x, row_r.center().y),
                    Align2::CENTER_CENTER,
                    &(i + 1).to_string(),
                    egui::FontId::proportional(11.0),
                    num_color,
                );
            }

            // Cover 32x32
            let thumb = Rect::from_min_max(
                egui::pos2(row_r.min.x + pad + bar_w + num_w, row_r.min.y + 8.0),
                egui::pos2(row_r.min.x + pad + bar_w + num_w + cover_w, row_r.min.y + 40.0),
            );
            cover::cover(track.cover_data.as_ref(), &track.album, track.id, ui, thumb);

            let cy = row_r.center().y;
            // Title (clipped)
            draw_clipped(ui.ctx(), p, track.display_title(), x_title, cy, title_w, 14.0, fg);
            // Artist (clipped)
            draw_clipped(ui.ctx(), p, track.display_artist(), x_artist, cy, artist_w, 13.0, muted);
            // Album (clipped)
            draw_clipped(ui.ctx(), p, &track.album, x_album, cy, album_w, 13.0, muted);
            // Duration
            p.text(
                egui::pos2(x_duration + dur_w * 0.5, cy),
                Align2::CENTER_CENTER,
                &track.duration_formatted(),
                egui::FontId::proportional(13.0),
                muted,
            );
        }
    });
}

// ── Helper ───────────────────────────────────────────────────────

fn format_total_duration(secs: f64) -> String {
    let total = secs as u64;
    let hours = total / 3600;
    let mins = (total % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}
