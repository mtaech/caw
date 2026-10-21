//! Left navigation sidebar: library views, playlists, and scan status.

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::InputState;
use gpui_kit::component::sidebar::{
    Sidebar, SidebarFooter, SidebarGroup, SidebarMenu, SidebarMenuItem,
};
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::*;

use crate::app::{DetailTarget, NavItem, PlayerApp};
use crate::ui::icons::CawIcon;

/// Width of the sidebar, matching the old Vue w-56 (224px).
const SIDEBAR_WIDTH: Pixels = px(224.);

pub fn render_sidebar(
    app: &PlayerApp,
    _window: &mut Window,
    cx: &mut Context<PlayerApp>,
) -> impl IntoElement {
    let entity = cx.entity().clone();
    let selected_nav = app.selected_nav;
    let detail_target = app.detail_target.clone();
    let track_count = app.library.len();
    let scanning = app.scanning;

    Sidebar::new("caw-sidebar")
        .w(SIDEBAR_WIDTH)
        .collapsible(false)
        .header(
            h_flex()
                .w_full()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .size_7()
                        .flex_shrink_0()
                        .rounded_md()
                        .bg(rgb(0xffffff))
                        .border_1()
                        .border_color(cx.theme().border)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(CawIcon::App).size_5()),
                )
                .child(
                    v_flex()
                        .gap_0()
                        .overflow_hidden()
                        .child(
                            div()
                                .text_sm()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("Caw"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .truncate()
                                .child("本地音乐播放器"),
                        ),
                ),
        )
        .child(
            SidebarGroup::new("资料库").child(
                SidebarMenu::new()
                    .child(nav_item(
                        "全部音乐",
                        CawIcon::App,
                        NavItem::AllMusic,
                        selected_nav,
                        &entity,
                    ))
                    .child(nav_item(
                        "艺术家",
                        CawIcon::User,
                        NavItem::Artists,
                        selected_nav,
                        &entity,
                    ))
                    .child(nav_item(
                        "专辑",
                        CawIcon::Disc,
                        NavItem::Albums,
                        selected_nav,
                        &entity,
                    ))
                    .child(nav_item(
                        "文件夹",
                        CawIcon::Folder,
                        NavItem::Folders,
                        selected_nav,
                        &entity,
                    ))
                    .child(nav_item(
                        "设置",
                        CawIcon::Settings,
                        NavItem::Settings,
                        selected_nav,
                        &entity,
                    )),
            ),
        )
        .child(SidebarGroup::new("播放列表").child(playlist_menu(
            app,
            &entity,
            selected_nav,
            &detail_target,
        )))
        .footer(
            SidebarFooter::new().child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .truncate()
                            .child(if scanning {
                                "正在扫描曲库...".to_string()
                            } else {
                                format!("{track_count} 首歌曲")
                            }),
                    )
                    .child({
                        let ent = entity.clone();
                        Button::new("btn-rescan")
                            .ghost()
                            .xsmall()
                            .loading(scanning)
                            .icon(CawIcon::Refresh)
                            .tooltip(if scanning {
                                "正在扫描曲库..."
                            } else {
                                "重新扫描曲库"
                            })
                            .on_click(move |_, _, cx| {
                                ent.update(cx, |this, cx| this.rescan(cx));
                            })
                    }),
            ),
        )
}

/// A single top-level navigation row.
fn nav_item(
    label: &'static str,
    icon: CawIcon,
    nav: NavItem,
    selected_nav: NavItem,
    entity: &Entity<PlayerApp>,
) -> SidebarMenuItem {
    let ent = entity.clone();
    SidebarMenuItem::new(label)
        .icon(icon)
        .active(selected_nav == nav)
        .on_click(move |_, _, cx| {
            ent.update(cx, |this, cx| this.select_nav(nav, cx));
        })
}

/// Playlist section: "new playlist" action, then one row per saved playlist.
fn playlist_menu(
    app: &PlayerApp,
    entity: &Entity<PlayerApp>,
    selected_nav: NavItem,
    detail_target: &Option<DetailTarget>,
) -> SidebarMenu {
    let mut menu = SidebarMenu::new().child({
        let ent = entity.clone();
        SidebarMenuItem::new("新建播放列表")
            .icon(CawIcon::Plus)
            .on_click(move |_, window, cx| {
                // The create-playlist overlay lives in root_view and renders
                // from this input state.
                ent.update(cx, |this, cx| {
                    let input = cx.new(|cx| {
                        InputState::new(window, cx)
                            .placeholder("播放列表名称")
                            .clean_on_escape()
                    });
                    this.create_playlist_input = Some(input);
                    cx.notify();
                });
            })
    });

    if app.playlists.is_empty() {
        menu = menu.child(
            SidebarMenuItem::new("暂无歌单")
                .icon(CawIcon::ListMusic)
                .disable(true),
        );
        return menu;
    }

    for playlist in &app.playlists {
        let playlist_id = playlist.id;
        let track_count = playlist.track_count;
        let is_active = selected_nav == NavItem::Playlists
            && matches!(detail_target, Some(DetailTarget::Playlist(id)) if *id == playlist_id);
        let ent = entity.clone();

        menu = menu.child(
            SidebarMenuItem::new(playlist.name.clone())
                .icon(CawIcon::ListMusic)
                .active(is_active)
                .suffix(move |_, cx| {
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{track_count}"))
                        .into_any_element()
                })
                .on_click(move |_, _, cx| {
                    ent.update(cx, |this, cx| {
                        this.select_nav(NavItem::Playlists, cx);
                        this.open_detail(DetailTarget::Playlist(playlist_id), cx);
                    });
                }),
        );
    }

    menu
}
