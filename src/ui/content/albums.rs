//! Album grid and album detail view.

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::*;

use crate::app::{DetailTarget, PlayerApp};
use crate::models::track::Track;
use crate::paths::get_or_save_cover;
use crate::ui::icons::CawIcon;

use super::tracks::render_tracks_table;

pub(super) fn render_albums(
    app: &PlayerApp,
    window: &mut Window,
    cx: &mut Context<PlayerApp>,
) -> impl IntoElement {
    let entity = cx.entity().clone();

    // If an album detail is selected
    if let Some(DetailTarget::Album(album_name)) = &app.detail_target {
        let album_name = album_name.clone();
        let mut album_indices: Vec<usize> = app
            .library
            .iter()
            .enumerate()
            .filter(|(_, t)| t.display_album() == album_name)
            .map(|(i, _)| i)
            .collect();

        // Sort by track number
        album_indices.sort_by_key(|&i| app.library[i].track_number);

        let artist_name = album_indices
            .first()
            .and_then(|&i| app.library.get(i))
            .map(|t| t.display_artist().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let cover_path = album_indices
            .first()
            .and_then(|&i| app.library.get(i))
            .and_then(|t| get_or_save_cover(t));

        let tracks: Vec<Arc<Track>> = album_indices
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
                    .gap_4()
                    .child({
                        let ent = entity.clone();
                        Button::new("btn-back-albums")
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
                    .child(if let Some(path) = cover_path {
                        div()
                            .size(px(72.))
                            .rounded_md()
                            .overflow_hidden()
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(img(path).size_full().object_fit(ObjectFit::Cover))
                            .into_any_element()
                    } else {
                        div()
                            .size(px(72.))
                            .rounded_md()
                            .bg(crate::ui::cover::gradient(&album_name, cx.theme()))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(CawIcon::Disc)
                                    .size_8()
                                    .text_color(crate::ui::cover::icon_color(cx.theme())),
                            )
                            .into_any_element()
                    })
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xl()
                                    .font_bold()
                                    .text_color(cx.theme().foreground)
                                    .child(album_name.clone()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!(
                                        "{} · 共 {} 首歌曲",
                                        artist_name,
                                        album_indices.len()
                                    )),
                            ),
                    )
                    .child(div().flex_1())
                    .child({
                        let ent = entity.clone();
                        Button::new("btn-play-all-album")
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
                &album_indices,
                &app.library,
                window,
                cx,
            ))
            .into_any_element();
    }

    // Group library by album
    let mut map: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, t) in app.library.iter().enumerate() {
        let album = t.display_album().to_string();
        map.entry(album).or_default().push(i);
    }

    v_flex()
        .w_full()
        .gap_4()
        .child(
            div()
                .text_2xl()
                .font_bold()
                .text_color(cx.theme().foreground)
                .child(format!("专辑 ({})", map.len())),
        )
        .child(
            div()
                .grid()
                .grid_cols(4)
                .gap_4()
                .children(map.into_iter().map(|(album, indices)| {
                    let count = indices.len();
                    let album_clone = album.clone();

                    let first_track = indices.first().and_then(|&i| app.library.get(i));
                    let artist = first_track
                        .map(|t| t.display_artist().to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let cover_path = first_track.and_then(|t| get_or_save_cover(t));
                    let cover_background: Background = if cover_path.is_some() {
                        cx.theme().muted.into()
                    } else {
                        crate::ui::cover::gradient(&album, cx.theme())
                    };

                    div()
                        .p_3()
                        .rounded_lg()
                        .border_1()
                        .border_color(cx.theme().border)
                        .bg(cx.theme().secondary)
                        .cursor_pointer()
                        .hover(|s| s.bg(cx.theme().accent))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.open_detail(DetailTarget::Album(album_clone.clone()), cx);
                            }),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(
                                    div()
                                        .w_full()
                                        .h(px(140.))
                                        .rounded_md()
                                        .overflow_hidden()
                                        .bg(cover_background)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(if let Some(path) = cover_path {
                                            img(path)
                                                .size_full()
                                                .object_fit(ObjectFit::Cover)
                                                .into_any_element()
                                        } else {
                                            Icon::new(CawIcon::Disc)
                                                .size_12()
                                                .text_color(crate::ui::cover::icon_color(
                                                    cx.theme(),
                                                ))
                                                .into_any_element()
                                        }),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .font_semibold()
                                        .text_color(cx.theme().foreground)
                                        .truncate()
                                        .child(album),
                                )
                                .child(
                                    h_flex()
                                        .w_full()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .truncate()
                                                .child(artist),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(format!("{} 首", count)),
                                        ),
                                ),
                        )
                })),
        )
        .into_any_element()
}
