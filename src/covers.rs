//! Online cover art: NetEase first, iTunes as a no-key fallback.

mod itunes;

use crate::models::track::Track;

/// Fetch cover bytes for `track`. Returns `None` when both providers miss.
///
/// The caller stores the bytes under `~/.config/caw/covers/<track-id>.jpg`
/// and notifies the UI; the existing `get_or_save_cover` then picks it up.
pub fn fetch(track: &Track) -> Option<Vec<u8>> {
    if let Some(bytes) = crate::netease::fetch_cover(track)
        && crate::netease::is_image(&bytes)
    {
        return Some(bytes);
    }

    itunes::fetch_cover(track)
}
