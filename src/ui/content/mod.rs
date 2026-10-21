//! Content area: dispatches the active nav view and hosts the queue drawer.

mod albums;
mod artists;
mod folders;
mod playlists;
mod queue;
mod settings;
mod tracks;

use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::prelude::{FluentBuilder as _, StatefulInteractiveElement as _};
use gpui_kit::*;

use crate::app::{NavItem, PlayerApp};

use albums::render_albums;
use artists::render_artists;
use folders::render_folders;
use playlists::render_playlists;
use queue::render_queue_drawer;
use settings::render_settings;
use tracks::render_all_music;

pub fn render_content(
    app: &PlayerApp,
    window: &mut Window,
    cx: &mut Context<PlayerApp>,
) -> impl IntoElement {
    div()
        .flex_1()
        .size_full()
        .overflow_hidden()
        .bg(cx.theme().background)
        .child(
            h_flex()
                .size_full()
                .child(
                    div()
                        .id("content-scroll")
                        .flex_1()
                        .size_full()
                        .overflow_y_scroll()
                        .track_scroll(&app.content_scroll_handle)
                        .vertical_scrollbar(&app.content_scroll_handle)
                        .p_6()
                        .child(match app.selected_nav {
                            NavItem::AllMusic => {
                                render_all_music(app, window, cx).into_any_element()
                            }
                            NavItem::Artists => render_artists(app, window, cx).into_any_element(),
                            NavItem::Albums => render_albums(app, window, cx).into_any_element(),
                            NavItem::Playlists => {
                                render_playlists(app, window, cx).into_any_element()
                            }
                            NavItem::Folders => render_folders(app, window, cx).into_any_element(),
                            NavItem::Settings => {
                                render_settings(app, window, cx).into_any_element()
                            }
                        }),
                )
                // Queue Drawer on right if open
                .when(app.show_queue, |this| {
                    this.child(render_queue_drawer(app, window, cx))
                }),
        )
}
