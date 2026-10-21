//! Playback queue drawer: current queue, save-as-playlist and play history.

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, WindowExt};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::app::PlayerApp;
use crate::ui::icons::CawIcon;

pub(super) fn render_queue_drawer(
    app: &PlayerApp,
    _window: &mut Window,
    cx: &mut Context<PlayerApp>,
) -> impl IntoElement {
    let entity = cx.entity().clone();
    let cur_idx = app.queue_index;
    let theme = cx.theme();
    let history = app.history_tracks();

    div()
        .w(px(320.))
        .h_full()
        .border_l_1()
        .border_color(theme.border)
        .bg(theme.secondary)
        .flex()
        .flex_col()
        .child(
            h_flex()
                .w_full()
                .p_3()
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(theme.border)
                .child(
                    h_flex().gap_2().items_center().child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(theme.foreground)
                            .child(format!("播放队列 ({})", app.queue.len())),
                    ),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .child({
                            let ent = entity.clone();
                            Button::new("btn-save-queue")
                                .ghost()
                                .xsmall()
                                .label("保存")
                                .on_click(move |_, window, cx| {
                                    open_save_queue_dialog(window, cx, ent.clone());
                                })
                        })
                        .child({
                            let ent = entity.clone();
                            Button::new("btn-clear-queue")
                                .ghost()
                                .xsmall()
                                .label("清空")
                                .on_click(move |_, _, cx| {
                                    ent.update(cx, |this, cx| {
                                        this.clear_queue(cx);
                                    });
                                })
                        })
                        .child({
                            let ent = entity.clone();
                            Button::new("btn-close-queue")
                                .ghost()
                                .xsmall()
                                .icon(CawIcon::Close)
                                .on_click(move |_, _, cx| {
                                    ent.update(cx, |this, cx| {
                                        this.toggle_queue(cx);
                                    });
                                })
                        }),
                ),
        )
        .child(
            div()
                .id("queue-scroll")
                .flex_1()
                .overflow_y_scrollbar()
                .p_2()
                .child(
                    v_flex()
                        .gap_1()
                        .children(app.queue.iter().enumerate().map(|(ix, t)| {
                            let is_cur = cur_idx == Some(ix);

                            h_flex()
                                .w_full()
                                .p_2()
                                .rounded_md()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .bg(if is_cur {
                                    theme.accent
                                } else {
                                    gpui::transparent_black()
                                })
                                .hover(|s| s.bg(theme.accent))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        this.play_queue_index(ix, cx);
                                    }),
                                )
                                .child(if is_cur {
                                    Icon::new(CawIcon::Volume2)
                                        .size_4()
                                        .text_color(theme.primary)
                                        .into_any_element()
                                } else {
                                    div()
                                        .w(px(16.))
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(format!("{}", ix + 1))
                                        .into_any_element()
                                })
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .overflow_hidden()
                                        .gap_0()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_medium()
                                                .truncate()
                                                .text_color(if is_cur {
                                                    theme.primary
                                                } else {
                                                    theme.foreground
                                                })
                                                .child(t.display_title().to_string()),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .truncate()
                                                .child(t.display_artist().to_string()),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .font_family(theme.mono_font_family.clone())
                                        .text_color(theme.muted_foreground)
                                        .child(t.duration_formatted()),
                                )
                        }))
                        // ── History ──
                        .child(
                            h_flex()
                                .w_full()
                                .mt_3()
                                .pt_3()
                                .border_t_1()
                                .border_color(theme.border)
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_semibold()
                                        .text_color(theme.muted_foreground)
                                        .child("最近播放"),
                                )
                                .child({
                                    let ent = entity.clone();
                                    Button::new("btn-clear-history")
                                        .ghost()
                                        .xsmall()
                                        .label("清除")
                                        .on_click(move |_, _, cx| {
                                            ent.update(cx, |this, cx| {
                                                this.clear_history(cx);
                                            });
                                        })
                                }),
                        )
                        .when(history.is_empty(), |this| {
                            this.child(
                                div()
                                    .px_2()
                                    .py_2()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("暂无播放历史"),
                            )
                        })
                        .children(
                            history
                                .into_iter()
                                .enumerate()
                                .take(20)
                                .map(|(idx, track)| {
                                    h_flex()
                                        .w_full()
                                        .p_2()
                                        .rounded_md()
                                        .items_center()
                                        .gap_2()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.accent))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, _, _, cx| {
                                                this.play_history_index(idx, cx);
                                            }),
                                        )
                                        .child(
                                            Icon::new(CawIcon::Music)
                                                .size_3()
                                                .text_color(theme.muted_foreground),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_xs()
                                                .truncate()
                                                .text_color(theme.foreground)
                                                .child(track.display_title().to_string()),
                                        )
                                }),
                        ),
                ),
        )
}

/// Prompt for a playlist name and snapshot the current queue into it.
fn open_save_queue_dialog(window: &mut Window, cx: &mut App, entity: Entity<PlayerApp>) {
    let input = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("播放列表名称")
            .clean_on_escape()
    });

    window.open_dialog(cx, move |dialog, _, _| {
        let input = input.clone();
        let entity = entity.clone();
        dialog
            .title("保存队列为播放列表")
            .child(Input::new(&input).cleanable(true))
            .button_props(
                DialogButtonProps::default()
                    .ok_text("保存")
                    .cancel_text("取消")
                    .show_cancel(true),
            )
            .on_ok(move |_, _, cx| {
                let name = input.read(cx).value().trim().to_string();
                if name.is_empty() {
                    return false;
                }
                entity.update(cx, |app, cx| app.save_queue_as_playlist(&name, cx));
                true
            })
    });
}
