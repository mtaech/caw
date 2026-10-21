//! All-music view and the shared track table used by every detail view.

use std::sync::Arc;

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::*;

use crate::app::PlayerApp;
use crate::models::playlist::PlaybackState;
use crate::models::track::Track;
use crate::ui::icons::CawIcon;

pub(super) fn render_all_music(
    app: &PlayerApp,
    window: &mut Window,
    cx: &mut Context<PlayerApp>,
) -> impl IntoElement {
    let entity = cx.entity().clone();
    let total_tracks = app.library.len();
    let shown_tracks = app.filtered_indices.len();

    // Calculate total duration of filtered tracks
    let total_secs: u64 = app
        .filtered_indices
        .iter()
        .filter_map(|&i| app.library.get(i))
        .map(|t| t.duration.as_secs())
        .sum();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let duration_str = if hours > 0 {
        format!("{} 小时 {} 分钟", hours, mins)
    } else {
        format!("{} 分钟", mins)
    };

    // Empty states: nothing scanned yet, or the search matched no track.
    let empty_message = if total_tracks == 0 {
        Some("曲库为空，请先在「设置」中添加音乐文件夹")
    } else if shown_tracks == 0 {
        Some("没有找到匹配的歌曲")
    } else {
        None
    };

    v_flex()
        .w_full()
        .gap_4()
        // Top Toolbar
        .child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .gap_4()
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_2xl()
                                .font_bold()
                                .text_color(cx.theme().foreground)
                                .child("全部音乐"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!(
                                    "共 {} 首歌曲 (筛选后 {} 首) · {}",
                                    total_tracks, shown_tracks, duration_str
                                )),
                        ),
                )
                .child(
                    h_flex()
                        .items_center()
                        .gap_3()
                        // Search Input
                        .child(
                            div()
                                .w(px(260.))
                                .child(Input::new(&app.search_input).cleanable(true)),
                        )
                        // Play All Button
                        .child({
                            let ent = entity.clone();
                            let tracks: Vec<Arc<Track>> = app
                                .filtered_indices
                                .iter()
                                .filter_map(|&i| app.library.get(i).cloned())
                                .collect();
                            Button::new("btn-play-all-tracks")
                                .primary()
                                .icon(CawIcon::Play)
                                .label("播放全部")
                                .on_click(move |_, _, cx| {
                                    ent.update(cx, |this, cx| {
                                        this.play_all(tracks.clone(), 0, cx);
                                    });
                                })
                        }),
                ),
        )
        // Tracks Table (or an empty state when there is nothing to list)
        .child(match empty_message {
            Some(message) => v_flex()
                .w_full()
                .items_center()
                .justify_center()
                .py_16()
                .gap_2()
                .child(
                    Icon::new(CawIcon::Music)
                        .size_8()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(message),
                )
                .into_any_element(),
            None => render_tracks_table(app, &app.filtered_indices, &app.library, window, cx)
                .into_any_element(),
        })
}

pub(super) fn render_tracks_table(
    app: &PlayerApp,
    indices: &[usize],
    library: &[Arc<Track>],
    _window: &mut Window,
    cx: &mut Context<PlayerApp>,
) -> impl IntoElement {
    let entity = cx.entity().clone();
    let current_track_id = app.current_track.as_ref().map(|t| t.id);
    let is_playing = app.playback_state == PlaybackState::Playing;

    // Shared once per render instead of cloning the whole track list for every row.
    let queued: Arc<Vec<Arc<Track>>> = Arc::new(
        indices
            .iter()
            .filter_map(|&i| library.get(i).cloned())
            .collect(),
    );

    v_flex()
        .w_full()
        .gap_1()
        // Table Header
        .child(
            h_flex()
                .w_full()
                .px_3()
                .py_2()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .border_b_1()
                .border_color(cx.theme().border)
                .child(div().w(px(40.)).child("#"))
                .child(div().flex_1().child("标题"))
                .child(div().w(px(180.)).child("歌手"))
                .child(div().w(px(180.)).child("专辑"))
                .child(div().w(px(70.)).text_right().child("时长"))
                .child(div().w(px(92.)).text_center().child("")),
        )
        // Rows
        .child(
            v_flex()
                .w_full()
                .children(indices.iter().enumerate().map(|(row_idx, &lib_idx)| {
                    let track = match library.get(lib_idx) {
                        Some(t) => t.clone(),
                        None => return div().into_any_element(),
                    };

                    let is_current = current_track_id == Some(track.id);
                    let track_clone = track.clone();
                    let queued = queued.clone();
                    let track_for_menu = track.clone();

                    h_flex()
                        .w_full()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .items_center()
                        .cursor_pointer()
                        .bg(if is_current {
                            cx.theme().accent
                        } else {
                            gpui::transparent_black()
                        })
                        .hover(|s| s.bg(cx.theme().accent))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.selected_track_id = Some(track_clone.id);
                                cx.notify();
                            }),
                        )
                        // # / Playing Indicator
                        .child(div().w(px(40.)).child(if is_current && is_playing {
                            Icon::new(CawIcon::Volume2)
                                .size_4()
                                .text_color(cx.theme().primary)
                                .into_any_element()
                        } else {
                            div()
                                .text_xs()
                                .text_color(if is_current {
                                    cx.theme().primary
                                } else {
                                    cx.theme().muted_foreground
                                })
                                .child(format!("{}", row_idx + 1))
                                .into_any_element()
                        }))
                        // Title
                        .child(
                            h_flex()
                                .flex_1()
                                .items_center()
                                .gap_2()
                                .overflow_hidden()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_medium()
                                        .truncate()
                                        .text_color(if is_current {
                                            cx.theme().primary
                                        } else {
                                            cx.theme().foreground
                                        })
                                        .child(track.display_title().to_string()),
                                ),
                        )
                        // Artist
                        .child(
                            div()
                                .w(px(180.))
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .truncate()
                                .child(track.display_artist().to_string()),
                        )
                        // Album
                        .child(
                            div()
                                .w(px(180.))
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .truncate()
                                .child(track.display_album().to_string()),
                        )
                        // Duration
                        .child(
                            div()
                                .w(px(70.))
                                .text_right()
                                .text_xs()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_color(cx.theme().muted_foreground)
                                .child(track.duration_formatted()),
                        )
                        // Actions: play this row / add it to a playlist
                        .child(
                            h_flex()
                                .w(px(92.))
                                .justify_center()
                                .gap_1()
                                .child({
                                    let ent = entity.clone();
                                    Button::new(format!("btn-play-row-{}", track.id))
                                        .ghost()
                                        .xsmall()
                                        .icon(if is_current && is_playing {
                                            CawIcon::Pause
                                        } else {
                                            CawIcon::Play
                                        })
                                        .on_click(move |_, _, cx| {
                                            let tracks = (*queued).clone();
                                            ent.update(cx, |this, cx| {
                                                if is_current {
                                                    this.toggle_play_pause(cx);
                                                } else {
                                                    this.play_all(tracks, row_idx, cx);
                                                }
                                            });
                                        })
                                })
                                .child({
                                    let ent = entity.clone();
                                    Button::new(format!("btn-add-row-to-playlist-{}", track.id))
                                        .ghost()
                                        .xsmall()
                                        .icon(CawIcon::Plus)
                                        .on_click(move |_, _, cx| {
                                            let track = track_for_menu.clone();
                                            ent.update(cx, |this, cx| {
                                                this.add_to_playlist_track = Some(track);
                                                cx.notify();
                                            });
                                        })
                                }),
                        )
                        .into_any_element()
                })),
        )
}
