use egui::{Color32, Rounding, Ui};
use egui_material_icons::MaterialIcon;

use crate::app::PlayerApp;
use crate::models::playlist::{PlaybackState, RepeatMode};
use crate::ui::icons;

/// Render the bottom playback control bar.
///
/// Layout: progress slider row + control row (track info | buttons | volume).
pub fn render_player_bar(
    ui: &mut Ui,
    app: &mut PlayerApp,
    volume_slider: &mut f32,
    progress_slider: &mut f64,
) {
    let state = app.playback_state;
    let position = app.audio_position();
    let duration = app.duration_secs();
    let volume = app.volume;
    let shuffle_on = app.shuffle;
    let repeat_mode = app.repeat_mode;

    // Sync sliders
    *progress_slider = if duration > 0.0 {
        (position / duration).min(1.0).max(0.0)
    } else {
        0.0
    };
    *volume_slider = volume;

    let bg = Color32::from_rgb(0x00, 0x00, 0x00);
    let primary = Color32::from_rgb(0x1d, 0xb9, 0x54);
    let fg = Color32::WHITE;
    let muted = Color32::from_rgb(0xa7, 0xa7, 0xa7);

    // Full background
    ui.painter().rect_filled(ui.max_rect(), Rounding::ZERO, bg);

    // ── Progress row ──────────────────────────────────────────
    let pos_str = format_time(position);
    let dur_str = format_time(duration);

    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(egui::RichText::new(pos_str).size(11.0).color(muted));

        let avail_w = ui.available_width().max(50.0) - 80.0;
        let mut prog = *progress_slider as f32;
        let slid = ui.add_sized(
            egui::vec2(avail_w, 20.0),
            egui::Slider::new(&mut prog, 0.0..=1.0).show_value(false),
        );
        if slid.changed() && duration > 0.0 {
            app.seek_to(prog as f64 * duration);
        }
        *progress_slider = prog as f64;

        ui.add_space(16.0);
        ui.label(egui::RichText::new(dur_str).size(11.0).color(muted));
    });

    // ── Control row ───────────────────────────────────────────
    ui.horizontal(|ui| {
        // Left: track info
        let track = app.current_track().cloned();
        let (title, artist) = match &track {
            Some(t) => (t.display_title().to_string(), t.display_artist().to_string()),
            None => ("未选择歌曲".into(), String::new()),
        };

        ui.vertical(|ui| {
            ui.label(egui::RichText::new(title).size(14.0).color(fg));
            ui.label(egui::RichText::new(artist).size(11.0).color(muted));
        });

        // Push controls to the centre.
        let spacer = (ui.available_width() - 300.0).max(0.0) / 2.0;
        ui.add_space(spacer);

        // ── Controls (Material icons) ──────────────────────────
        let s_color = if shuffle_on { primary } else { muted };
        if btn(ui, "shuffle", icons::ICON_SHUFFLE, 30.0, s_color).clicked() {
            app.shuffle = !app.shuffle;
        }

        if btn(ui, "prev", icons::ICON_SKIP_PREVIOUS, 30.0, fg).clicked() {
            app.prev_track();
        }

        // Play / Pause — filled primary circle
        let pp_icon: MaterialIcon = if state == PlaybackState::Playing {
            icons::ICON_PAUSE
        } else {
            icons::ICON_PLAY_ARROW
        };
        if icons::filled_circle_button(ui, "playpause", pp_icon, 42.0, primary).clicked() {
            app.toggle_play();
        }

        if btn(ui, "next", icons::ICON_SKIP_NEXT, 30.0, fg).clicked() {
            app.next_track();
        }

        let r_color = if repeat_mode != RepeatMode::None { primary } else { muted };
        let r_icon = if repeat_mode == RepeatMode::One {
            icons::ICON_REPEAT_ONE
        } else {
            icons::ICON_REPEAT
        };
        if btn(ui, "repeat", r_icon, 30.0, r_color).clicked() {
            app.repeat_mode = match app.repeat_mode {
                RepeatMode::None => RepeatMode::All,
                RepeatMode::All => RepeatMode::One,
                RepeatMode::One => RepeatMode::None,
            };
        }

        ui.add_space(spacer);

        // ── Volume + Queue ─────────────────────────────────────
        let vol_icon = if volume <= 0.0 {
            icons::ICON_VOLUME_OFF
        } else if volume <= 0.5 {
            icons::ICON_VOLUME_DOWN
        } else {
            icons::ICON_VOLUME_UP
        };
        if btn(ui, "vol", vol_icon, 30.0, muted).clicked() {
            app.toggle_mute();
        }

        let mut vol = *volume_slider;
        ui.add_sized(
            egui::vec2(100.0, 20.0),
            egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false),
        );
        if (vol - *volume_slider).abs() > 0.005 {
            app.set_volume(vol);
        }
        *volume_slider = vol;

        if btn(ui, "queue", icons::ICON_QUEUE_MUSIC, 30.0, muted).clicked() {
            eprintln!("caw: queue panel not yet implemented");
        }
    });
}

/// Frameless square icon button (Material glyph) with caller-controlled size + colour.
fn btn(ui: &mut Ui, id: &str, icon: MaterialIcon, size: f32, color: Color32) -> egui::Response {
    icons::icon_button(ui, id, icon, size, color)
}

fn format_time(secs: f64) -> String {
    if secs.is_nan() || secs.is_infinite() || secs < 0.0 {
        "00:00".into()
    } else {
        let total = secs as u64;
        format!("{:02}:{:02}", total / 60, total % 60)
    }
}
