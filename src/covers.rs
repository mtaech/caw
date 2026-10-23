//! Online cover art: NetEase first, then iTunes, then MusicBrainz/CAA.

mod itunes;
mod musicbrainz;

use crate::models::track::Track;

/// Fetch cover bytes for `track`. Returns `None` when every provider misses.
///
/// The caller stores the bytes under `~/.config/caw/covers/<track-id>.jpg`
/// and notifies the UI; the existing `get_or_save_cover` then picks it up.
/// `acoustid_client` enables the fingerprint-assisted retry for mis-tagged
/// files; empty means the assist is off.
pub fn fetch(track: &Track, acoustid_client: &str) -> Option<Vec<u8>> {
    if let Some(bytes) = crate::netease::fetch_cover(track)
        && crate::netease::is_image(&bytes)
    {
        return Some(bytes);
    }

    if let Some(bytes) = itunes::fetch_cover(track) {
        return Some(bytes);
    }
    if let Some(bytes) = musicbrainz::fetch_cover(track) {
        return Some(bytes);
    }
    fetch_assisted(track, acoustid_client)
}

/// Retry the providers with metadata recovered from the audio itself: text
/// search is only as good as the tags, and mis-tagged files are exactly the
/// case a fingerprint lookup exists for.
fn fetch_assisted(track: &Track, acoustid_client: &str) -> Option<Vec<u8>> {
    let identified = crate::acoustid::identify(track, acoustid_client)?;

    let corrected = Track {
        title: identified.title.clone(),
        artist: identified.artist.clone(),
        album: identified
            .album
            .clone()
            .unwrap_or_else(|| track.album.clone()),
        ..track.clone()
    };
    if let Some(bytes) = crate::netease::fetch_cover(&corrected)
        && crate::netease::is_image(&bytes)
    {
        return Some(bytes);
    }
    if let Some(bytes) = itunes::fetch_cover(&corrected) {
        return Some(bytes);
    }
    identified
        .release_group
        .as_deref()
        .and_then(musicbrainz::fetch_cover_by_release_group)
}
