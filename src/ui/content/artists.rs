//! Artists list and artist detail view.

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::*;

use crate::app::{DetailTarget, PlayerApp};
use crate::models::track::Track;
use crate::ui::icons::CawIcon;

use super::tracks::render_tracks_table;

pub(super) fn render_artists(
    app: &PlayerApp,
    window: &mut Window,
    cx: &mut Context<PlayerApp>,
) -> impl IntoElement {
    let entity = cx.entity().clone();

    // If an artist detail is selected
    if let Some(DetailTarget::Artist(artist_name)) = &app.detail_target {
        let artist_name = artist_name.clone();
        let artist_indices: Vec<usize> = app
            .library
            .iter()
            .enumerate()
            .filter(|(_, t)| t.display_artist() == artist_name)
            .map(|(i, _)| i)
            .collect();

        let tracks: Vec<Arc<Track>> = artist_indices
            .iter()
            .filter_map(|&i| app.library.get(i).cloned())
            .collect();

        return v_flex()
            .w_full()
            .gap_4()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_3()
                    .child({
                        let ent = entity.clone();
                        Button::new("btn-back-artists")
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
                        v_flex()
                            .gap_0()
                            .child(
                                div()
                                    .text_xl()
                                    .font_bold()
                                    .text_color(cx.theme().foreground)
                                    .child(artist_name.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("共 {} 首歌曲", artist_indices.len())),
                            ),
                    )
                    .child(div().flex_1())
                    .child({
                        let ent = entity.clone();
                        Button::new("btn-play-all-artist")
                            .primary()
                            .icon(CawIcon::Play)
                            .label("播放全部")
                            .on_click(move |_, _, cx| {
                                ent.update(cx, |this, cx| {
                                    this.play_all(tracks.clone(), 0, cx);
                                });
                            })
                    }),
            )
            .child(render_tracks_table(
                app,
                &artist_indices,
                &app.library,
                window,
                cx,
            ))
            .into_any_element();
    }

    // Group library by artist
    let mut map: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, t) in app.library.iter().enumerate() {
        let artist = t.display_artist().to_string();
        map.entry(artist).or_default().push(i);
    }

    v_flex()
        .w_full()
        .gap_4()
        .child(
            div()
                .text_2xl()
                .font_bold()
                .text_color(cx.theme().foreground)
                .child(format!("艺术家 ({})", map.len())),
        )
        .child(
            div()
                .grid()
                .grid_cols(4)
                .gap_4()
                .children(map.into_iter().map(|(artist, indices)| {
                    let count = indices.len();
                    let artist_clone = artist.clone();

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
                                this.open_detail(DetailTarget::Artist(artist_clone.clone()), cx);
                            }),
                        )
                        .child(
                            v_flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .size_16()
                                        .rounded_full()
                                        .bg(crate::ui::cover::gradient(&artist, cx.theme()))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            Icon::new(CawIcon::User).size_8().text_color(
                                                crate::ui::cover::icon_color(cx.theme()),
                                            ),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .font_semibold()
                                        .text_color(cx.theme().foreground)
                                        .truncate()
                                        .child(artist),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("{} 首歌曲", count)),
                                ),
                        )
                })),
        )
        .into_any_element()
}
