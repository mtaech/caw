//! Bottom player bar: now-playing info, transport controls, progress, volume.

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::slider::{Slider, SliderState};
use gpui_kit::component::{ActiveTheme, Disableable as _, Icon, Selectable, Sizable};
use gpui_kit::*;

use crate::app::PlayerApp;
use crate::models::playlist::{PlaybackMode, PlaybackState};
use crate::paths::get_or_save_cover;
use crate::ui::icons::CawIcon;
use crate::ui::marquee::MarqueeText;

/// Height of the player bar (old Vue h-20).
const PLAYER_BAR_HEIGHT: Pixels = px(80.);
/// Width of the fixed left (now playing) and right (volume/queue) clusters.
const SIDE_CLUSTER_WIDTH: Pixels = px(280.);
/// The right cluster also hosts locate + queue buttons, so it is wider.
const RIGHT_CLUSTER_WIDTH: Pixels = px(324.);
/// Space left for the title inside the left cluster (cover + gap subtracted).
const TITLE_WIDTH: Pixels = px(216.);

fn format_time(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// Push a programmatic value into a slider only when it actually changed.
///
/// SliderState::set_value calls cx.notify(), so an unconditional update on
/// every render would schedule a frame forever.
fn sync_slider(
    state: &Entity<SliderState>,
    value: f32,
    epsilon: f32,
    window: &mut Window,
    cx: &mut Context<PlayerApp>,
) {
    let current = state.read(cx).value().start();
    if (current - value).abs() > epsilon {
        state.update(cx, |state, cx| state.set_value(value, window, cx));
    }
}

pub fn render_player_bar(
    app: &PlayerApp,
    window: &mut Window,
    cx: &mut Context<PlayerApp>,
) -> impl IntoElement {
    let entity = cx.entity().clone();
    let is_playing = app.playback_state == PlaybackState::Playing;
    let is_muted = app.is_muted || app.volume <= 0.0;

    // Mirror playback state into the sliders (the reverse direction is wired
    // through slider events in PlayerApp).
    if !app.seeking && app.duration > 0.0 {
        let pct = ((app.position / app.duration) * 100.0).clamp(0.0, 100.0) as f32;
        sync_slider(&app.progress_slider, pct, 0.05, window, cx);
    }
    sync_slider(&app.volume_slider, app.volume * 100.0, 0.5, window, cx);

    h_flex()
        .w_full()
        .h(PLAYER_BAR_HEIGHT)
        .flex_shrink_0()
        .border_t_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .items_center()
        .justify_between()
        .px_4()
        .gap_4()
        // ── Left: current track info ──
        .child(
            h_flex()
                .w(SIDE_CLUSTER_WIDTH)
                .flex_shrink_0()
                .items_center()
                .gap_3()
                .child(cover_art(app, cx))
                .child(
                    v_flex()
                        .id("now-playing-info")
                        .flex_1()
                        .min_w(px(0.))
                        .overflow_hidden()
                        .gap_0()
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                this.locate_now_playing(window, cx);
                            }),
                        )
                        .child(
                            // Long titles scroll; short ones stay static and
                            // ellipsize. Fixed height keeps the row stable.
                            MarqueeText::new(now_playing_title(app), TITLE_WIDTH)
                                .height(px(20.))
                                .font_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(cx.theme().foreground),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .truncate()
                                .child(secondary_text(app)),
                        ),
                ),
        )
        // ── Center: transport + progress ──
        .child(
            v_flex()
                .flex_1()
                .min_w(px(0.))
                .max_w(px(560.))
                .items_center()
                .gap_1()
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        // Playback mode: sequence / shuffle / repeat list / repeat one
                        .child({
                            let ent = entity.clone();
                            let mode = app.playback_mode;
                            Button::new("btn-playback-mode")
                                .ghost()
                                .small()
                                .selected(mode != PlaybackMode::Sequence)
                                .toggled(mode != PlaybackMode::Sequence)
                                .icon(mode.icon())
                                .tooltip(format!("{}（点击切换）", mode.label()))
                                .on_click(move |_, _, cx| {
                                    ent.update(cx, |this, cx| this.cycle_playback_mode(cx));
                                })
                        })
                        .child({
                            let ent = entity.clone();
                            Button::new("btn-prev-track")
                                .ghost()
                                .small()
                                .icon(CawIcon::SkipBack)
                                .tooltip("上一首（←）")
                                .on_click(move |_, _, cx| {
                                    ent.update(cx, |this, cx| this.prev_track(cx));
                                })
                        })
                        .child({
                            let ent = entity.clone();
                            Button::new("btn-play-pause")
                                .primary()
                                .icon(if is_playing {
                                    CawIcon::Pause
                                } else {
                                    CawIcon::Play
                                })
                                .tooltip(if is_playing {
                                    "暂停（空格）"
                                } else {
                                    "播放（空格）"
                                })
                                .on_click(move |_, _, cx| {
                                    ent.update(cx, |this, cx| this.toggle_play_pause(cx));
                                })
                        })
                        .child({
                            let ent = entity.clone();
                            Button::new("btn-next-track")
                                .ghost()
                                .small()
                                .icon(CawIcon::SkipForward)
                                .tooltip("下一首（→）")
                                .on_click(move |_, _, cx| {
                                    ent.update(cx, |this, cx| this.next_track(cx));
                                })
                        }),
                )
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .w(px(44.))
                                .flex_shrink_0()
                                .text_right()
                                .text_xs()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_color(cx.theme().muted_foreground)
                                .child(format_time(app.position)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .child(Slider::new(&app.progress_slider)),
                        )
                        .child(
                            div()
                                .w(px(44.))
                                .flex_shrink_0()
                                .text_left()
                                .text_xs()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_color(cx.theme().muted_foreground)
                                .child(format_time(app.duration)),
                        ),
                ),
        )
        // ── Right: queue + volume ──
        .child(
            h_flex()
                .w(RIGHT_CLUSTER_WIDTH)
                .flex_shrink_0()
                .items_center()
                .justify_end()
                .gap_3()
                .child({
                    let ent = entity.clone();
                    let has_track = app.current_track.is_some();
                    Button::new("btn-locate-now-playing")
                        .ghost()
                        .small()
                        .icon(CawIcon::Locate)
                        .tooltip("定位到正在播放")
                        .disabled(!has_track)
                        .on_click(move |_, window, cx| {
                            ent.update(cx, |this, cx| {
                                this.locate_now_playing(window, cx);
                            });
                        })
                })
                .child({
                    let ent = entity.clone();
                    let show_queue = app.show_queue;
                    let queue_len = app.queue.len();
                    Button::new("btn-toggle-queue")
                        .ghost()
                        .small()
                        .selected(show_queue)
                        .toggled(show_queue)
                        .icon(CawIcon::ListMusic)
                        .label(if queue_len > 0 {
                            format!("{queue_len}")
                        } else {
                            String::new()
                        })
                        .tooltip(if show_queue {
                            "隐藏播放队列"
                        } else {
                            "显示播放队列"
                        })
                        .on_click(move |_, _, cx| {
                            ent.update(cx, |this, cx| this.toggle_queue(cx));
                        })
                })
                .child({
                    let ent = entity.clone();
                    Button::new("btn-volume-mute")
                        .ghost()
                        .small()
                        .icon(volume_icon(is_muted, cx))
                        .tooltip(if is_muted {
                            format!("取消静音（{}%）", (app.volume * 100.0).round() as u32)
                        } else {
                            "静音".to_string()
                        })
                        .on_click(move |_, _, cx| {
                            ent.update(cx, |this, cx| this.toggle_mute(cx));
                        })
                })
                .child(
                    div()
                        .w(px(90.))
                        .flex_shrink_0()
                        .child(Slider::new(&app.volume_slider)),
                )
                .child(
                    div()
                        .w(px(38.))
                        .flex_shrink_0()
                        .text_xs()
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_color(cx.theme().muted_foreground)
                        .child(if is_muted {
                            "静音".to_string()
                        } else {
                            format!("{}%", (app.volume * 100.0).round() as u32)
                        }),
                ),
        )
}

/// Mute button icon: tinted with the danger color while muted.
fn volume_icon(muted: bool, cx: &mut Context<PlayerApp>) -> Icon {
    let icon = if muted {
        Icon::new(CawIcon::VolumeX)
    } else {
        Icon::new(CawIcon::Volume2)
    };
    icon.text_color(if muted {
        cx.theme().danger
    } else {
        cx.theme().muted_foreground
    })
}

/// Title shown in the player bar left cluster.
fn now_playing_title(app: &PlayerApp) -> String {
    app.current_track
        .as_ref()
        .map(|t| t.display_title().to_string())
        .unwrap_or_else(|| "未播放歌曲".to_string())
}

/// "artist · album" line under the title, with sensible empty states.
fn secondary_text(app: &PlayerApp) -> String {
    match app.current_track.as_ref() {
        Some(track) => {
            let artist = track.display_artist();
            let album = track.display_album();
            if album.is_empty() {
                artist.to_string()
            } else {
                format!("{artist} · {album}")
            }
        }
        None => "选择歌曲开始播放".to_string(),
    }
}

/// Cover thumbnail, or a themed placeholder when the track has no artwork.
fn cover_art(app: &PlayerApp, cx: &mut Context<PlayerApp>) -> AnyElement {
    if let Some(track) = app.current_track.as_ref()
        && let Some(cover_path) = get_or_save_cover(track)
    {
        return div()
            .size(px(52.))
            .flex_shrink_0()
            .rounded_md()
            .overflow_hidden()
            .border_1()
            .border_color(cx.theme().border)
            .child(
                img(cover_path.to_string_lossy().to_string())
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            )
            .into_any_element();
    }

    let icon = if app.current_track.is_some() {
        CawIcon::Music
    } else {
        CawIcon::Disc
    };

    div()
        .size(px(52.))
        .flex_shrink_0()
        .rounded_md()
        .bg(cx.theme().muted)
        .border_1()
        .border_color(cx.theme().border)
        .flex()
        .items_center()
        .justify_center()
        .child(
            Icon::new(icon)
                .size_5()
                .text_color(cx.theme().muted_foreground),
        )
        .into_any_element()
}
