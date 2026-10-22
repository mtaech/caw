//! Content area: dispatches the active nav view and hosts the queue drawer.

mod albums;
mod artists;
mod folders;
mod now_playing;
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
use now_playing::render_now_playing;
use playlists::render_playlists;
use queue::render_queue_drawer;
use settings::render_settings;
use tracks::render_all_music;

pub fn render_content(
    app: &PlayerApp,
    window: &mut Window,
    cx: &mut Context<PlayerApp>,
) -> impl IntoElement {
    // Three shells:
    // - now-playing owns its own lyrics scroll;
    // - track-list views (All Music + details) own a virtualized list scroll;
    // - everything else uses the shared outer content scroll.
    let view = if app.show_now_playing {
        div()
            .flex_1()
            .size_full()
            .overflow_hidden()
            .p_6()
            .child(render_now_playing(app, window, cx))
            .into_any_element()
    } else if app.is_track_list_view() {
        div()
            .flex_1()
            .size_full()
            .overflow_hidden()
            .p_6()
            .child(active_view(app, window, cx))
            .into_any_element()
    } else {
        div()
            .id("content-scroll")
            .flex_1()
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&app.content_scroll_handle)
            .vertical_scrollbar(&app.content_scroll_handle)
            .p_6()
            .child(active_view(app, window, cx))
            .into_any_element()
    };

    h_flex()
        .flex_1()
        .size_full()
        .overflow_hidden()
        .bg(cx.theme().background)
        .child(view)
        // Queue Drawer on right if open
        .when(app.show_queue, |this| {
            this.child(render_queue_drawer(app, window, cx))
        })
}

fn active_view(app: &PlayerApp, window: &mut Window, cx: &mut Context<PlayerApp>) -> AnyElement {
    match app.selected_nav {
        NavItem::AllMusic => render_all_music(app, window, cx).into_any_element(),
        NavItem::Artists => render_artists(app, window, cx).into_any_element(),
        NavItem::Albums => render_albums(app, window, cx).into_any_element(),
        NavItem::Playlists => render_playlists(app, window, cx).into_any_element(),
        NavItem::Folders => render_folders(app, window, cx).into_any_element(),
        NavItem::Settings => render_settings(app, window, cx).into_any_element(),
    }
}
