//! Now-playing view: large cover, track details and lyrics.

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::{FluentBuilder as _, StatefulInteractiveElement as _};
use gpui_kit::*;

use crate::app::PlayerApp;
use crate::lyrics::{Lyrics, LyricsSource};
use crate::models::track::Track;
use crate::paths::get_or_save_cover;
use crate::ui::icons::CawIcon;

/// Cover column width; it shrinks when the window (or queue drawer) gets tight.
const COVER_COLUMN_WIDTH: Pixels = px(280.);
const COVER_COLUMN_MIN_WIDTH: Pixels = px(160.);

pub(super) fn render_now_playing(
    app: &PlayerApp,
    _window: &mut Window,
    cx: &mut Context<PlayerApp>,
) -> impl IntoElement {
    let entity = cx.entity().clone();

    v_flex()
        .size_full()
        .gap_4()
        .child(
            h_flex()
                .w_full()
                .h(px(56.))
                .flex_shrink_0()
                .items_center()
                .gap_3()
                .child({
                    let ent = entity.clone();
                    Button::new("btn-close-now-playing")
                        .ghost()
                        .small()
                        .icon(CawIcon::ArrowLeft)
                        .label("返回")
                        .on_click(move |_, _, cx| {
                            ent.update(cx, |this, cx| this.close_now_playing(cx));
                        })
                })
                .child(
                    div()
                        .text_2xl()
                        .font_bold()
                        .text_color(cx.theme().foreground)
                        .child("正在播放"),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .min_w(px(0.))
                        .truncate()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("L 切换封面与歌词"),
                ),
        )
        .child(match app.current_track.as_ref() {
            Some(track) => render_track(app, track, cx).into_any_element(),
            None => render_empty(entity, cx).into_any_element(),
        })
}

/// Cover column on the left, details and lyrics on the right.
fn render_track(app: &PlayerApp, track: &Track, cx: &mut Context<PlayerApp>) -> impl IntoElement {
    h_flex()
        .w_full()
        .flex_1()
        .min_h(px(0.))
        .items_stretch()
        .gap_8()
        .child(
            v_flex()
                .w(COVER_COLUMN_WIDTH)
                .min_w(COVER_COLUMN_MIN_WIDTH)
                .items_start()
                .child(cover_art(track, cx)),
        )
        .child(
            v_flex()
                .flex_1()
                .min_w(px(0.))
                .min_h(px(0.))
                .gap_4()
                .child(track_details(track, cx))
                .child(div().w_full().h(px(1.)).bg(cx.theme().border))
                .child(lyrics_section(app, cx)),
        )
}

fn cover_art(track: &Track, cx: &mut Context<PlayerApp>) -> AnyElement {
    let frame = div()
        .w_full()
        .aspect_square()
        .rounded_lg()
        .overflow_hidden()
        .border_1()
        .border_color(cx.theme().border);

    if let Some(path) = get_or_save_cover(track) {
        return frame
            .child(img(path).size_full().object_fit(ObjectFit::Cover))
            .into_any_element();
    }

    frame
        .bg(crate::ui::cover::gradient(
            track.display_album(),
            cx.theme(),
        ))
        .flex()
        .items_center()
        .justify_center()
        .child(
            Icon::new(CawIcon::Disc)
                .size_16()
                .text_color(crate::ui::cover::icon_color(cx.theme())),
        )
        .into_any_element()
}

fn track_details(track: &Track, cx: &mut Context<PlayerApp>) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_1()
        .child(
            div()
                .text_2xl()
                .font_bold()
                .text_color(cx.theme().foreground)
                .truncate()
                .child(track.display_title().to_string()),
        )
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .truncate()
                .child(track.display_artist().to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .truncate()
                .child(format!(
                    "{} · {}",
                    track.display_album(),
                    track.duration_formatted()
                )),
        )
}

fn lyrics_section(app: &PlayerApp, cx: &mut Context<PlayerApp>) -> impl IntoElement {
    let entity = cx.entity().clone();

    let source = match app.lyrics.as_ref() {
        Some(lyrics) if lyrics.synced => format!("{} · 同步", lyrics.source.label()),
        Some(lyrics) => lyrics.source.label().to_string(),
        None if app.lyrics_loading => "正在读取…".to_string(),
        None => String::new(),
    };

    let online = app.config.fetch_lyrics_online;
    let is_network = app.lyrics.as_ref().is_some_and(|lyrics| {
        matches!(lyrics.source, LyricsSource::NetEase | LyricsSource::LrcLib)
    });
    let loading_detail = if online {
        "本地没有时会尝试网易云 / LRCLIB，结果会缓存"
    } else {
        "支持同名 .lrc 文件与音频内嵌歌词"
    };
    let empty_detail = if online {
        "本地没有歌词，网易云和 LRCLIB 都没有匹配（或网络不可用）。可点刷新重试，或放入同名 .lrc。"
    } else {
        "已关闭在线歌词。可在「设置 → 歌词」中开启在线获取，或放入同名 .lrc。"
    };

    let body: AnyElement = if app.lyrics_loading {
        lyrics_placeholder(CawIcon::MicVocal, "正在查找歌词…", loading_detail, cx)
    } else {
        match app.lyrics.as_ref() {
            Some(lyrics) if !lyrics.lines.is_empty() => {
                lyrics_lines(app, lyrics, entity.clone(), cx)
            }
            _ => lyrics_placeholder(CawIcon::MicVocal, "这首歌没有歌词", empty_detail, cx),
        }
    };

    v_flex()
        .w_full()
        .flex_1()
        .min_h(px(0.))
        .gap_2()
        .child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .text_color(cx.theme().foreground)
                        .child("歌词"),
                )
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(source),
                        )
                        .when(
                            online && !app.lyrics_loading && (is_network || app.lyrics.is_none()),
                            |this| {
                                this.child({
                                    let ent = entity.clone();
                                    Button::new("btn-refresh-lyrics")
                                        .ghost()
                                        .xsmall()
                                        .icon(CawIcon::Refresh)
                                        .tooltip("重新在线获取")
                                        .on_click(move |_, _, cx| {
                                            ent.update(cx, |this, cx| {
                                                this.refresh_lyrics(cx);
                                            });
                                        })
                                })
                            },
                        ),
                ),
        )
        .child(body)
}

fn lyrics_lines(
    app: &PlayerApp,
    lyrics: &Lyrics,
    entity: Entity<PlayerApp>,
    cx: &mut Context<PlayerApp>,
) -> AnyElement {
    let synced = lyrics.synced;
    let active = app.lyrics_active_line;

    // Rows are direct children of the scroll container: ScrollHandle records
    // one child bound per row, which is what auto-scroll positions against.
    // The scrollbar is appended after the rows so row index == child index.
    let mut scroll = div()
        .id("lyrics-scroll")
        .w_full()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .gap_0p5();

    for (index, line) in lyrics.lines.iter().enumerate() {
        let is_active = synced && active == Some(index);
        let text = if line.text.is_empty() {
            " ".to_string()
        } else {
            line.text.clone()
        };

        let mut row = div()
            .w_full()
            .flex_shrink_0()
            .px_3()
            .py_1()
            .rounded_md()
            .whitespace_normal()
            .text_sm()
            .child(text);

        if is_active {
            row = row
                .bg(cx.theme().primary.opacity(0.12))
                .font_semibold()
                .text_color(cx.theme().primary);
        } else {
            row = row.text_color(cx.theme().muted_foreground);
        }

        if synced && line.time.is_some() {
            let ent = entity.clone();
            row = row
                .cursor_pointer()
                .hover(|style| style.bg(cx.theme().accent))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    ent.update(cx, |this, cx| this.play_lyric_line(index, cx));
                });
        }

        scroll = scroll.child(row);
    }

    let scroll = scroll
        .overflow_y_scroll()
        .track_scroll(&app.lyrics_scroll_handle)
        .vertical_scrollbar(&app.lyrics_scroll_handle);

    scroll.into_any_element()
}

fn lyrics_placeholder(
    icon: CawIcon,
    title: &str,
    detail: &str,
    cx: &mut Context<PlayerApp>,
) -> AnyElement {
    v_flex()
        .w_full()
        .flex_1()
        .min_h(px(0.))
        .items_center()
        .justify_center()
        .gap_2()
        .px_6()
        .child(
            Icon::new(icon)
                .size_8()
                .text_color(cx.theme().muted_foreground),
        )
        .child(
            div()
                .text_sm()
                .font_medium()
                .text_color(cx.theme().foreground)
                .child(title.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .text_center()
                .child(detail.to_string()),
        )
        .into_any_element()
}

fn render_empty(entity: Entity<PlayerApp>, cx: &mut Context<PlayerApp>) -> AnyElement {
    v_flex()
        .w_full()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_3()
        .child(
            Icon::new(CawIcon::Disc)
                .size_12()
                .text_color(cx.theme().muted_foreground),
        )
        .child(
            div()
                .text_base()
                .font_medium()
                .text_color(cx.theme().foreground)
                .child("还没有正在播放的歌曲"),
        )
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("在曲库中双击任意歌曲开始播放"),
        )
        .child(
            Button::new("btn-now-playing-browse")
                .outline()
                .small()
                .icon(CawIcon::ArrowLeft)
                .label("返回曲库")
                .on_click(move |_, _, cx| {
                    entity.update(cx, |this, cx| this.close_now_playing(cx));
                }),
        )
        .into_any_element()
}
