//! Editable tag metadata.
//!
//! Symphonia (used for scanning and playback) is read-only, so tag **writes**
//! go through Lofty. Lofty can rewrite ID3v2.3/2.4 (MP3, WAV, AIFF, AAC),
//! Vorbis comments (FLAC, Ogg) and MP4 ilst, which covers every format Caw
//! scans. Text fields are read through the same library so the editor shows
//! exactly what will be written back.
//!
//! The editor deliberately exposes a small, audio-player-shaped field set:
//! title, artist, album, album artist, genre, year, track and disc number.
//! Everything else in the file (cover art, unknown frames, ReplayGain, ...) is
//! parsed and written back untouched.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use lofty::config::{ParseOptions, WriteOptions};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{ItemKey, Tag};

use crate::audio::decoder::fix_mojibake;

/// Tag values shown in the metadata editor.
///
/// Numeric fields stay strings: the UI edits text and tolerates junk input
/// (an empty year clears the field, "3/12" sets track + total), and parsing
/// happens once in [`write`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditableTags {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: String,
    /// Four-digit year. Month/day already in the file are preserved on write.
    pub year: String,
    /// `"3"` or `"3/12"` (number / total).
    pub track: String,
    /// `"1"` or `"1/2"` (number / total).
    pub disc: String,
}

impl EditableTags {
    /// True when every editable field is blank.
    ///
    /// Saving an empty form over a file that has no tags would otherwise create
    /// an empty tag block; `write` treats that as a no-op.
    pub fn is_empty(&self) -> bool {
        self.title.trim().is_empty()
            && self.artist.trim().is_empty()
            && self.album.trim().is_empty()
            && self.album_artist.trim().is_empty()
            && self.genre.trim().is_empty()
            && self.year.trim().is_empty()
            && self.track.trim().is_empty()
            && self.disc.trim().is_empty()
    }

    /// Fill the fields the scanner already knows, used when the file has no
    /// readable tag yet (or reading it failed).
    pub fn from_track(track: &crate::models::track::Track) -> Self {
        Self {
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            track: number_or_total((track.track_number > 0).then_some(track.track_number), None),
            ..Self::default()
        }
    }
}

/// Read the editable fields from `path`.
///
/// Cover art and audio properties are skipped: the editor only needs text and
/// this keeps a click responsive on files with multi-megabyte artwork.
pub fn read(path: &Path) -> Result<EditableTags> {
    let parse_options = ParseOptions::new()
        .read_properties(false)
        .read_cover_art(false);
    let tagged = Probe::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?
        .options(parse_options)
        .read()
        .with_context(|| format!("failed to parse tags of {}", path.display()))?;

    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return Ok(EditableTags::default());
    };

    let text = |value: Option<std::borrow::Cow<'_, str>>| {
        value
            .map(|v| fix_mojibake(v.into_owned()))
            .unwrap_or_default()
    };

    Ok(EditableTags {
        title: text(tag.title()),
        artist: text(tag.artist()),
        album: text(tag.album()),
        album_artist: text(
            tag.get_string(ItemKey::AlbumArtist)
                .map(std::borrow::Cow::Borrowed),
        ),
        genre: text(tag.genre()),
        year: tag
            .date()
            .filter(|date| date.year > 0)
            .map(|date| date.year.to_string())
            .unwrap_or_default(),
        track: number_or_total(tag.track(), tag.track_total()),
        disc: number_or_total(tag.disk(), tag.disk_total()),
    })
}

/// Write `tags` back to `path`, preserving everything the editor does not own.
///
/// The file is rewritten in place (same inode, permissions and hard links), so
/// callers must not run this on the UI thread. Lofty keeps the existing tag
/// version when it can: an ID3v2.3 file stays 2.3, a 2.4 file stays 2.4. MP3s
/// without a tag get 2.3, which every player/scanner we care about understands.
pub fn write(path: &Path, tags: &EditableTags) -> Result<()> {
    // Cover art must be parsed here: the tag is re-encoded from this object,
    // so skipping pictures would silently delete embedded artwork.
    let parse_options = ParseOptions::new().read_properties(false);
    let mut tagged = Probe::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?
        .options(parse_options)
        .read()
        .with_context(|| format!("failed to parse tags of {}", path.display()))?;

    if tagged.primary_tag().is_none() {
        // Nothing to write and nothing to clear: don't add an empty tag block.
        if tags.is_empty() {
            return Ok(());
        }
        let tag_type = tagged.primary_tag_type();
        tagged.insert_tag(Tag::new(tag_type));
    }
    let tag = tagged.primary_tag_mut().context("no writable tag")?;

    // Text fields: an empty value removes the frame instead of writing "".
    set_text(tag, tags.title.trim(), ItemKey::TrackTitle);
    set_text(tag, tags.artist.trim(), ItemKey::TrackArtist);
    set_text(tag, tags.album.trim(), ItemKey::AlbumTitle);
    set_text(tag, tags.album_artist.trim(), ItemKey::AlbumArtist);
    set_text(tag, tags.genre.trim(), ItemKey::Genre);

    // Year only: keep the month/day already stored in the file.
    match parse_year(&tags.year) {
        Some(year) => {
            let mut date = tag.date().unwrap_or_default();
            date.year = year;
            tag.set_date(date);
        }
        None => tag.remove_date(),
    }

    let (track, track_total) = parse_number_pair(&tags.track);
    match track {
        Some(number) => tag.set_track(number),
        None => {
            tag.remove_track();
            tag.remove_track_total();
        }
    }
    match track_total {
        Some(total) => tag.set_track_total(total),
        None => tag.remove_track_total(),
    }

    let (disc, disc_total) = parse_number_pair(&tags.disc);
    match disc {
        Some(number) => tag.set_disk(number),
        None => {
            tag.remove_disk();
            tag.remove_disk_total();
        }
    }
    match disc_total {
        Some(total) => tag.set_disk_total(total),
        None => tag.remove_disk_total(),
    }

    let write_options = WriteOptions::new().use_id3v23(use_id3v23(path));
    tag.save_to_path(path, write_options)
        .with_context(|| format!("failed to write tags of {}", path.display()))?;
    Ok(())
}

fn set_text(tag: &mut Tag, value: &str, key: ItemKey) {
    if value.is_empty() {
        // `remove_key` covers every field we edit, including album artist,
        // which has no Accessor helper.
        tag.remove_key(key);
    } else {
        tag.insert_text(key, value.to_string());
    }
}

/// `"3/12"` -> (`3`, `12`), `"3"` -> (`3`, `None`), junk -> (`None`, `None`).
fn parse_number_pair(input: &str) -> (Option<u32>, Option<u32>) {
    let mut parts = input.trim().split('/');
    let number = parts.next().and_then(|part| part.trim().parse().ok());
    let total = parts.next().and_then(|part| part.trim().parse().ok());
    (number, total)
}

fn number_or_total(number: Option<u32>, total: Option<u32>) -> String {
    match (number, total) {
        (Some(number), Some(total)) => format!("{number}/{total}"),
        (Some(number), None) => number.to_string(),
        _ => String::new(),
    }
}

/// Accept a bare year, or a date whose leading component is the year.
fn parse_year(input: &str) -> Option<u16> {
    let digits: String = input.trim().chars().take(4).collect();
    digits.parse().ok().filter(|year| *year > 0)
}

/// Whether an ID3v2 tag should be written as 2.3.
///
/// Existing 2.3 tags stay 2.3 (some car stereos and scanners ignore 2.4), 2.4
/// stays 2.4, and a new MP3 tag defaults to 2.3 for the same compatibility
/// reason. Other containers ignore this option.
fn use_id3v23(path: &Path) -> bool {
    let mut header = [0u8; 5];
    let version = File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .ok()
        .filter(|_| &header[0..3] == b"ID3")
        .map(|_| header[3]);

    match version {
        Some(3) => true,
        Some(4) => false,
        _ => path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mp3")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_number_pairs() {
        assert_eq!(parse_number_pair("3/12"), (Some(3), Some(12)));
        assert_eq!(parse_number_pair(" 7 "), (Some(7), None));
        assert_eq!(parse_number_pair(""), (None, None));
        assert_eq!(parse_number_pair("a/b"), (None, None));
        // Only the first two components matter.
        assert_eq!(parse_number_pair("1/2/3"), (Some(1), Some(2)));
    }

    #[test]
    fn formats_number_pairs() {
        assert_eq!(number_or_total(Some(3), Some(12)), "3/12");
        assert_eq!(number_or_total(Some(3), None), "3");
        assert_eq!(number_or_total(None, Some(12)), "");
        assert_eq!(number_or_total(None, None), "");
    }

    #[test]
    fn parses_years() {
        assert_eq!(parse_year("2001"), Some(2001));
        assert_eq!(parse_year(" 2001-05-03 "), Some(2001));
        assert_eq!(parse_year(""), None);
        assert_eq!(parse_year("0"), None);
        assert_eq!(parse_year("abcd"), None);
    }
}
