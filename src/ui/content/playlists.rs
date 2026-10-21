//! Playlists grid and playlist detail view.

use std::collections::HashSet;
use std::sync::Arc;

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::*;

use crate::app::{DetailTarget, PlayerApp};
use crate::models::track::Track;
use crate::ui::icons::CawIcon;

use super::tracks::render_tracks_table;

pub(super) fn render_playlists(
    app: &PlayerApp,
    window: &mut Window,
    cx: &mut Context<PlayerApp>,
) -> impl IntoElement {
    let entity = cx.entity().clone();

    if let Some(DetailTarget::Playlist(pl_id)) = app.detail_target {
        let pl_data = app.db.get_playlist(pl_id);
        if let Some(pl) = pl_data {
            let track_id_set: HashSet<i64> = pl.track_ids.iter().cloned().collect();
            let pl_indices: Vec<usize> = app
                .library
                .iter()
                .enumerate()
                .filter(|(_, t)| track_id_set.contains(&(t.id as i64)))
                .map(|(i, _)| i)
                .collect();

            let tracks: Vec<Arc<Track>> = pl_indices
                .iter()
                .filter_map(|&i| app.library.get(i).cloned())
                .collect();

            let pl_name = pl.name.clone();
            let pl_name_for_delete = pl.name.clone();

            return v_flex()
                .w_full()
                .gap_4()
                .child(
                    h_flex()
                        .w_full()
                        .h(px(72.))
                        .items_center()
                        .gap_4()
                        .child({
                            let ent = entity.clone();
                            Button::new("btn-back-pl")
                                .ghost()
                                .small()
                                .icon(CawIcon::ArrowLeft)
                                .label("返回")
                                .on_click(move |_, _, cx| {
                                    ent.update(cx, |this, cx| {
                                        this.back_to_list(cx);
                                    });
                                })
                        })
                        .child(
                            div()
                                .size(px(64.))
                                .rounded_md()
                                .bg(cx.theme().primary)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    Icon::new(CawIcon::ListMusic)
                                        .size_8()
                                        .text_color(cx.theme().primary_foreground),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xl()
                                        .font_bold()
                                        .text_color(cx.theme().foreground)
                                        .child(pl_name),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("共 {} 首歌曲", pl_indices.len())),
                                ),
                        )
                        .child(div().flex_1())
                        .child({
                            let ent = entity.clone();
                            Button::new("btn-play-all-pl")
                                .primary()
                                .icon(CawIcon::Play)
                                .label("播放全部")
                                .on_click(move |_, _, cx| {
                                    ent.update(cx, |this, cx| {
                                        this.play_all(tracks.clone(), 0, cx);
                                    });
                                })
                        })
                        .child({
                            let ent = entity.clone();
                            let name = pl_name_for_delete.clone();
                            Button::new("btn-delete-pl")
                                .outline()
                                .icon(CawIcon::Trash)
                                .label("删除歌单")
                                .on_click(move |_, window, cx| {
                                    crate::ui::dialogs::open_delete_playlist_confirm(
                                        window,
                                        cx,
                                        ent.clone(),
                                        pl_id,
                                        name.clone(),
                                    );
                                })
                        }),
                )
                .child(render_tracks_table(
                    app,
                    &pl_indices,
                    &app.library,
                    window,
                    cx,
                ))
                .into_any_element();
        }
    }

    // Playlists list
    v_flex()
        .w_full()
        .gap_4()
        .child(
            h_flex()
                .w_full()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_2xl()
                        .font_bold()
                        .text_color(cx.theme().foreground)
                        .child("播放列表"),
                )
                .child({
                    let ent = entity.clone();
                    Button::new("btn-create-pl")
                        .primary()
                        .icon(CawIcon::Plus)
                        .label("新建播放列表")
                        .on_click(move |_, window, cx| {
                            ent.update(cx, |this, cx| {
                                let input = cx.new(|cx| {
                                    gpui_kit::component::input::InputState::new(window, cx)
                                        .placeholder("输入播放列表名称...")
                                        .clean_on_escape()
                                });
                                this.create_playlist_input = Some(input);
                                cx.notify();
                            });
                        })
                }),
        )
        .child(
            div()
                .grid()
                .grid_cols(3)
                .gap_4()
                .children(app.playlists.iter().map(|pl| {
                    let pl_id = pl.id;
                    let pl_name = pl.name.clone();
                    let count = pl.track_count;

                    div()
                        .p_4()
                        .rounded_lg()
                        .border_1()
                        .border_color(cx.theme().border)
                        .bg(cx.theme().secondary)
                        .cursor_pointer()
                        .hover(|s| s.bg(cx.theme().accent))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.open_detail(DetailTarget::Playlist(pl_id), cx);
                            }),
                        )
                        .child(
                            h_flex()
                                .gap_3()
                                .items_center()
                                .child(
                                    div()
                                        .size_12()
                                        .rounded_md()
                                        .bg(cx.theme().primary)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            Icon::new(CawIcon::ListMusic)
                                                .size_6()
                                                .text_color(cx.theme().primary_foreground),
                                        ),
                                )
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_semibold()
                                                .text_color(cx.theme().foreground)
                                                .child(pl_name),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(format!("{} 首歌曲", count)),
                                        ),
                                ),
                        )
                })),
        )
        .into_any_element()
}
