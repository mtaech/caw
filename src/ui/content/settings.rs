//! Settings view: appearance, playback behavior, music directories and about.

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::combobox::Combobox;
use gpui_kit::component::{ActiveTheme, Icon, Selectable, Sizable, ThemeMode};
use gpui_kit::*;

use crate::app::PlayerApp;
use crate::ui::icons::CawIcon;

/// One option in an exclusive choice row.
///
/// The active option uses the primary variant (filled), the rest stay outlined.
/// `Button::toggled` only sets accessible state, it does not paint a selected
/// background, so selection has to be expressed through the variant as well.
fn choice_button(id: &'static str, label: &'static str, active: bool) -> Button {
    if active {
        Button::new(id)
            .primary()
            .small()
            .label(label)
            .selected(true)
    } else {
        Button::new(id)
            .outline()
            .small()
            .label(label)
            .selected(false)
    }
}

pub(super) fn render_settings(
    app: &PlayerApp,
    _window: &mut Window,
    cx: &mut Context<PlayerApp>,
) -> impl IntoElement {
    let entity = cx.entity().clone();
    let theme = cx.theme();

    let section = |title: &'static str, description: &'static str| {
        v_flex()
            .gap_1()
            .child(
                div()
                    .text_base()
                    .font_semibold()
                    .text_color(theme.foreground)
                    .child(title),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(description),
            )
    };

    v_flex()
        .w_full()
        .max_w(px(720.))
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_bold()
                .text_color(theme.foreground)
                .child("设置"),
        )
        // ── Appearance ──
        .child(
            v_flex()
                .gap_3()
                .p_4()
                .rounded_lg()
                .border_1()
                .border_color(theme.border)
                .bg(theme.secondary)
                .child(section("外观", "选择界面配色；切换后立即生效并写入配置"))
                .child(
                    h_flex()
                        .gap_2()
                        .child({
                            let ent = entity.clone();
                            choice_button(
                                "btn-theme-light",
                                "亮色",
                                app.config.theme_mode == ThemeMode::Light,
                            )
                            .on_click(move |_, _, cx| {
                                ent.update(cx, |this, cx| {
                                    this.set_theme_mode(ThemeMode::Light, cx);
                                });
                            })
                        })
                        .child({
                            let ent = entity.clone();
                            choice_button(
                                "btn-theme-dark",
                                "暗色",
                                app.config.theme_mode == ThemeMode::Dark,
                            )
                            .on_click(move |_, _, cx| {
                                ent.update(cx, |this, cx| {
                                    this.set_theme_mode(ThemeMode::Dark, cx);
                                });
                            })
                        }),
                )
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("界面字体（列出系统全部字体，可搜索）"),
                        )
                        .child(
                            div().w(px(320.)).child(
                                Combobox::new(&app.font_picker)
                                    .placeholder("选择字体")
                                    .search_placeholder("搜索字体…")
                                    .cleanable(false)
                                    .menu_max_h(px(320.)),
                            ),
                        ),
                ),
        )
        // ── Behavior ──
        .child(
            v_flex()
                .gap_3()
                .p_4()
                .rounded_lg()
                .border_1()
                .border_color(theme.border)
                .bg(theme.secondary)
                .child(section(
                    "行为",
                    "关闭窗口时最小化到系统托盘（从托盘图标恢复窗口），或直接退出",
                ))
                .child(
                    h_flex()
                        .gap_2()
                        .child({
                            let ent = entity.clone();
                            choice_button(
                                "btn-close-quit",
                                "直接退出",
                                !app.config.close_to_tray,
                            )
                            .on_click(move |_, _, cx| {
                                ent.update(cx, |this, cx| {
                                    this.set_close_to_tray(false, cx);
                                });
                            })
                        })
                        .child({
                            let ent = entity.clone();
                            choice_button(
                                "btn-close-tray",
                                "最小化到托盘",
                                app.config.close_to_tray,
                            )
                            .on_click(move |_, _, cx| {
                                ent.update(cx, |this, cx| {
                                    this.set_close_to_tray(true, cx);
                                });
                            })
                        }),
                ),
        )
        // ── Music directories ──
        .child(
            v_flex()
                .gap_3()
                .p_4()
                .rounded_lg()
                .border_1()
                .border_color(theme.border)
                .bg(theme.secondary)
                .child(section(
                    "音乐文件夹",
                    "配置 Caw 扫描的音乐目录，支持多个文件夹去重合并",
                ))
                .child(
                    v_flex()
                        .gap_2()
                        .children(app.music_dirs.iter().map(|dir| {
                            let ent = entity.clone();
                            let dir_clone = dir.clone();

                            h_flex()
                                .w_full()
                                .p_2()
                                .rounded_md()
                                .bg(theme.muted)
                                .items_center()
                                .justify_between()
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(Icon::new(CawIcon::Folder).size_4())
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_family(theme.mono_font_family.clone())
                                                .child(dir.to_string_lossy().to_string()),
                                        ),
                                )
                                .child(
                                    Button::new(format!("btn-rm-dir-{}", dir.to_string_lossy()))
                                        .ghost()
                                        .xsmall()
                                        .icon(CawIcon::Trash)
                                        .on_click(move |_, _, cx| {
                                            ent.update(cx, |this, cx| {
                                                this.remove_music_dir(&dir_clone, cx);
                                            });
                                        }),
                                )
                        })),
                )
                .child(
                    h_flex()
                        .gap_3()
                        .child({
                            let ent = entity.clone();
                            Button::new("btn-add-music-dir")
                                .outline()
                                .small()
                                .icon(CawIcon::Plus)
                                .label("添加文件夹…")
                                .on_click(move |_, _, cx| {
                                    ent.update(cx, |this, cx| this.pick_music_folder(cx));
                                })
                        })
                        .child({
                            let ent = entity.clone();
                            Button::new("btn-settings-rescan")
                                .primary()
                                .small()
                                .icon(CawIcon::Refresh)
                                .label("立即重新扫描")
                                .on_click(move |_, _, cx| {
                                    ent.update(cx, |this, cx| {
                                        this.rescan(cx);
                                    });
                                })
                        }),
                ),
        )
        // ── About ──
        .child(
            v_flex()
                .gap_3()
                .p_4()
                .rounded_lg()
                .border_1()
                .border_color(theme.border)
                .bg(theme.secondary)
                .child(section("关于 Caw", "纯 Rust 本地音乐播放器"))
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("GPU 加速渲染 GPUI / gpui-kit，Symphonia 解码，cpal 输出，SQLite 歌单与配置持久化。"),
                )
                .child(
                    h_flex()
                        .gap_4()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("版本: v0.5.1"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("框架: GPUI Kit (v0.7.0)"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("许可: MIT License"),
                        ),
                ),
        )
}
