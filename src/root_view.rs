use std::sync::Arc;

use gpui_kit::base::{StyledExt, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{ActiveTheme, Icon, Sizable, TitleBar};
use gpui_kit::*;

use crate::app::PlayerApp;
use crate::models::track::Track;
use crate::ui::content::render_content;
use crate::ui::icons::CawIcon;
use crate::ui::player_bar::render_player_bar;
use crate::ui::sidebar::render_sidebar;

impl Render for PlayerApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().clone();
        let app: &PlayerApp = self;

        let create_playlist_overlay = app
            .create_playlist_input
            .clone()
            .map(|input| build_create_playlist_overlay(entity.clone(), input, cx));

        // Floating shortcut back to the now-playing row, shown only while that
        // row is scrolled out of view.
        let floating_locate = (app.current_track.is_some() && !app.now_playing_row_visible())
            .then(|| build_floating_locate(entity.clone(), cx));

        let add_to_playlist_overlay = app.add_to_playlist_track.clone().map(|track| {
            let playlists = app
                .playlists
                .iter()
                .map(|playlist| (playlist.id, playlist.name.clone()))
                .collect::<Vec<_>>();
            build_add_to_playlist_overlay(entity.clone(), track, playlists, cx)
        });

        div()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .track_focus(&app.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                handle_shortcut(this, event, window, cx);
            }))
            .child(
                v_flex()
                    .size_full()
                    .child({
                        let close_entity = entity.clone();
                        TitleBar::new()
                            .on_close_window(move |_, window, cx| {
                                crate::platform::window_state::save_now(window);
                                let close_to_tray = close_entity.read(cx).config.close_to_tray;
                                crate::log_info!(
                                    "titlebar close requested (close_to_tray={close_to_tray})"
                                );
                                if close_to_tray {
                                    close_entity.update(cx, |app, cx| app.flush_config(cx));
                                    // Remove the taskbar entry too, not just minimize.
                                    crate::platform::kwin::set_taskbar_visible(false);
                                    window.minimize_window();
                                } else {
                                    crate::ui::dialogs::open_close_confirm(
                                        window,
                                        cx,
                                        close_entity.clone(),
                                    );
                                }
                            })
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        Icon::new(CawIcon::Music)
                                            .size_4()
                                            .text_color(cx.theme().primary),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_semibold()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Caw"),
                                    ),
                            )
                    })
                    .child(
                        h_flex()
                            .flex_1()
                            .size_full()
                            .overflow_hidden()
                            .child(render_sidebar(app, window, cx))
                            .child(render_content(app, window, cx)),
                    )
                    .child(render_player_bar(app, window, cx)),
            )
            .children(floating_locate)
            .children(create_playlist_overlay)
            .children(add_to_playlist_overlay)
    }
}

/// Round floating button that jumps back to the now-playing row.
fn build_floating_locate(entity: Entity<PlayerApp>, cx: &mut Context<PlayerApp>) -> AnyElement {
    let background = cx.theme().primary;
    let foreground = cx.theme().primary_foreground;

    h_flex()
        .id("floating-locate")
        .absolute()
        .right(px(24.))
        .bottom(px(104.))
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .rounded_full()
        .bg(background)
        .shadow_lg()
        .cursor_pointer()
        .hover(|style| style.opacity(0.92))
        .child(Icon::new(CawIcon::Locate).size_4().text_color(foreground))
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(foreground)
                .child("正在播放"),
        )
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            entity.update(cx, |app, cx| app.locate_now_playing(window, cx));
        })
        .into_any_element()
}

fn handle_shortcut(
    app: &mut PlayerApp,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut Context<PlayerApp>,
) {
    // Never steal keys from a text field or an open modal.
    if app.create_playlist_input.is_some() || app.add_to_playlist_track.is_some() {
        return;
    }
    if app
        .search_input
        .read(cx)
        .focus_handle(cx)
        .is_focused(window)
    {
        return;
    }

    match event.keystroke.key.as_str() {
        "space" | " " => app.toggle_play_pause(cx),
        "left" | "arrowleft" => app.prev_track(cx),
        "right" | "arrowright" => app.next_track(cx),
        "up" | "arrowup" => {
            let vol = (app.volume + 0.05).min(1.0);
            app.set_volume(vol, cx);
            sync_volume_slider(app, vol, window, cx);
        }
        "down" | "arrowdown" => {
            let vol = (app.volume - 0.05).max(0.0);
            app.set_volume(vol, cx);
            sync_volume_slider(app, vol, window, cx);
        }
        _ => {}
    }
}

fn sync_volume_slider(
    app: &PlayerApp,
    volume: f32,
    window: &mut Window,
    cx: &mut Context<PlayerApp>,
) {
    app.volume_slider.update(cx, |slider, cx| {
        slider.set_value(volume * 100.0, window, cx);
    });
}

fn build_create_playlist_overlay(
    entity: Entity<PlayerApp>,
    input: Entity<InputState>,
    cx: &mut Context<PlayerApp>,
) -> AnyElement {
    let background = cx.theme().popover;
    let border = cx.theme().border;
    let foreground = cx.theme().foreground;
    let cancel_entity = entity.clone();
    let confirm_entity = entity;
    let confirm_input = input.clone();

    div()
        .absolute()
        .inset_0()
        .bg(rgba(0x00000088))
        .flex()
        .items_center()
        .justify_center()
        .child(
            v_flex()
                .w(px(360.))
                .p_5()
                .rounded_lg()
                .bg(background)
                .border_1()
                .border_color(border)
                .shadow_lg()
                .gap_4()
                .child(
                    div()
                        .text_base()
                        .font_semibold()
                        .text_color(foreground)
                        .child("新建播放列表"),
                )
                .child(Input::new(&input))
                .child(
                    h_flex()
                        .justify_end()
                        .gap_2()
                        .child(
                            Button::new("btn-cancel-create-playlist")
                                .ghost()
                                .small()
                                .label("取消")
                                .on_click(move |_, _, cx| {
                                    cancel_entity.update(cx, |app, cx| {
                                        app.create_playlist_input = None;
                                        cx.notify();
                                    });
                                }),
                        )
                        .child(
                            Button::new("btn-confirm-create-playlist")
                                .primary()
                                .small()
                                .label("创建")
                                .on_click(move |_, _, cx| {
                                    let name = confirm_input.read(cx).value().to_string();
                                    confirm_entity.update(cx, |app, cx| {
                                        app.create_playlist(&name, cx);
                                    });
                                }),
                        ),
                ),
        )
        .into_any_element()
}

fn build_add_to_playlist_overlay(
    entity: Entity<PlayerApp>,
    track: Arc<Track>,
    playlists: Vec<(i64, String)>,
    cx: &mut Context<PlayerApp>,
) -> AnyElement {
    let background = cx.theme().popover;
    let border = cx.theme().border;
    let foreground = cx.theme().foreground;
    let muted = cx.theme().muted_foreground;
    let cancel_entity = entity.clone();

    let mut list = v_flex().gap_1().w_full();
    if playlists.is_empty() {
        list = list.child(
            div()
                .px_2()
                .py_3()
                .text_sm()
                .text_color(muted)
                .child("暂无播放列表，请先在侧栏创建。"),
        );
    } else {
        for (playlist_id, name) in playlists {
            let ent = entity.clone();
            let track = track.clone();
            list = list.child(
                Button::new(format!("btn-add-track-to-playlist-{playlist_id}"))
                    .ghost()
                    .small()
                    .w_full()
                    .label(name)
                    .on_click(move |_, _, cx| {
                        let track_id = track.id;
                        ent.update(cx, |app, cx| {
                            app.add_track_to_playlist(playlist_id, track_id, cx);
                        });
                    }),
            );
        }
    }

    let track_label = format!("{} — {}", track.display_title(), track.display_artist());

    div()
        .absolute()
        .inset_0()
        .bg(rgba(0x00000088))
        .flex()
        .items_center()
        .justify_center()
        .child(
            v_flex()
                .w(px(380.))
                .p_5()
                .rounded_lg()
                .bg(background)
                .border_1()
                .border_color(border)
                .shadow_lg()
                .gap_4()
                .child(
                    div()
                        .text_base()
                        .font_semibold()
                        .text_color(foreground)
                        .child("添加到播放列表"),
                )
                .child(div().text_xs().text_color(muted).child(track_label))
                .child(list)
                .child(
                    h_flex().justify_end().child(
                        Button::new("btn-cancel-add-to-playlist")
                            .ghost()
                            .small()
                            .label("取消")
                            .on_click(move |_, _, cx| {
                                cancel_entity.update(cx, |app, cx| {
                                    app.add_to_playlist_track = None;
                                    cx.notify();
                                });
                            }),
                    ),
                ),
        )
        .into_any_element()
}
