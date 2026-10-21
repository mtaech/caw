//! Folder browser: configured music roots and their subdirectories.

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::app::{DetailTarget, PlayerApp};
use crate::ui::icons::CawIcon;

pub(super) fn render_folders(
    app: &PlayerApp,
    _window: &mut Window,
    cx: &mut Context<PlayerApp>,
) -> impl IntoElement {
    let entity = cx.entity().clone();

    // Determine current folder path
    let current_dir = if let Some(DetailTarget::Folder(path)) = &app.detail_target {
        Some(path.clone())
    } else {
        None
    };

    v_flex()
        .w_full()
        .gap_4()
        .child(
            h_flex()
                .items_center()
                .gap_3()
                .when(current_dir.is_some(), |this| {
                    let ent = entity.clone();
                    let cur = current_dir.clone().unwrap();
                    let parent = cur.parent().map(|p| p.to_path_buf());
                    this.child(
                        Button::new("btn-back-folder")
                            .ghost()
                            .small()
                            .icon(CawIcon::ArrowLeft)
                            .label("上一级")
                            .on_click(move |_, _, cx| {
                                ent.update(cx, |this, cx| {
                                    if let Some(p) = parent.clone() {
                                        this.open_detail(DetailTarget::Folder(p), cx);
                                    } else {
                                        this.back_to_list(cx);
                                    }
                                });
                            }),
                    )
                })
                .child(
                    div()
                        .text_2xl()
                        .font_bold()
                        .text_color(cx.theme().foreground)
                        .child(if let Some(ref d) = current_dir {
                            d.file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| "文件夹".to_string())
                        } else {
                            "音乐文件夹".to_string()
                        }),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(if let Some(ref d) = current_dir {
                    d.to_string_lossy().to_string()
                } else {
                    "浏览配置的音乐目录与本地文件".to_string()
                }),
        )
        // Subdirectories and audio files list
        .child({
            let mut list = v_flex().gap_2();

            if let Some(dir) = current_dir {
                // List items in this directory
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    let mut subdirs = Vec::new();
                    let mut audio_files = Vec::new();

                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_dir() {
                            subdirs.push(path);
                        } else if let Some(ext) = path.extension().and_then(|e| e.to_str())
                            && ["flac", "mp3", "wav", "ogg", "m4a", "aac"]
                                .contains(&ext.to_lowercase().as_str())
                        {
                            audio_files.push(path);
                        }
                    }

                    subdirs.sort();
                    audio_files.sort();

                    // Render subdirs
                    for sd in subdirs {
                        let sd_name = sd
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let sd_clone = sd.clone();

                        list = list.child(
                            h_flex()
                                .p_2()
                                .rounded_md()
                                .items_center()
                                .gap_3()
                                .cursor_pointer()
                                .hover(|s| s.bg(cx.theme().accent))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        this.open_detail(
                                            DetailTarget::Folder(sd_clone.clone()),
                                            cx,
                                        );
                                    }),
                                )
                                .child(
                                    Icon::new(CawIcon::Folder)
                                        .size_5()
                                        .text_color(cx.theme().primary),
                                )
                                .child(div().text_sm().font_medium().child(sd_name)),
                        );
                    }

                    // Render audio files
                    for af in audio_files {
                        let af_name = af
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let track_match = app.library.iter().find(|t| t.path == af).cloned();

                        list = list.child(
                            h_flex()
                                .p_2()
                                .rounded_md()
                                .items_center()
                                .gap_3()
                                .cursor_pointer()
                                .hover(|s| s.bg(cx.theme().accent))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        if let Some(t) = track_match.clone() {
                                            this.play_track(t, cx);
                                        }
                                    }),
                                )
                                .child(
                                    Icon::new(CawIcon::Music)
                                        .size_5()
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .child(div().text_sm().child(af_name)),
                        );
                    }
                }
            } else {
                // Show root music directories
                for root in &app.music_dirs {
                    let root_clone = root.clone();
                    let name = root.to_string_lossy().to_string();

                    list = list.child(
                        h_flex()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().secondary)
                            .items_center()
                            .gap_3()
                            .cursor_pointer()
                            .hover(|s| s.bg(cx.theme().accent))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.open_detail(DetailTarget::Folder(root_clone.clone()), cx);
                                }),
                            )
                            .child(
                                Icon::new(CawIcon::Folder)
                                    .size_6()
                                    .text_color(cx.theme().primary),
                            )
                            .child(
                                v_flex()
                                    .gap_0()
                                    .child(
                                        div().text_sm().font_semibold().child(
                                            root.file_name()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                                .to_string(),
                                        ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(name),
                                    ),
                            ),
                    );
                }
            }

            list
        })
}
