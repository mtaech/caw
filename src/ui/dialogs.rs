//! Reusable confirmation dialogs built on gpui-kit's overlay layer.
//!
//! These helpers only build and open the modal; the Root wrapper in the window
//! tree owns rendering. Callers pass the PlayerApp entity so the actions stay
//! in app state.

use gpui_kit::component::WindowExt;
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::*;

use crate::app::PlayerApp;

/// Confirm quitting the application.
///
/// Persists pending config changes before quitting. A future close-to-tray
/// flow can chain through this dialog or offer its own variant.
pub fn open_close_confirm(window: &mut Window, cx: &mut App, entity: Entity<PlayerApp>) {
    window.open_alert_dialog(cx, move |alert, _, _| {
        let entity = entity.clone();
        alert
            .title("退出 Caw？")
            .description("退出后播放会立即停止，下次启动会重新扫描曲库。")
            .confirm()
            .ok_text("退出")
            .ok_variant(ButtonVariant::Danger)
            .cancel_text("取消")
            .on_ok(move |_, _, cx| {
                entity.update(cx, |app, cx| app.flush_config(cx));
                cx.quit();
                true
            })
    });
}

/// Confirm deleting a playlist. Tracks stay in the library.
///
/// Not wired yet: the playlist views currently delete without confirmation
/// and will adopt this helper once they add a delete affordance.
#[allow(dead_code)]
pub fn open_delete_playlist_confirm(
    window: &mut Window,
    cx: &mut App,
    entity: Entity<PlayerApp>,
    playlist_id: i64,
    name: String,
) {
    window.open_alert_dialog(cx, move |alert, _, _| {
        let entity = entity.clone();
        alert
            .title("删除播放列表？")
            .description(format!(
                "将永久删除「{name}」，其中的歌曲仍会保留在曲库中。"
            ))
            .confirm()
            .ok_text("删除")
            .ok_variant(ButtonVariant::Danger)
            .cancel_text("取消")
            .on_ok(move |_, _, cx| {
                entity.update(cx, |app, cx| app.delete_playlist(playlist_id, cx));
                true
            })
    });
}
