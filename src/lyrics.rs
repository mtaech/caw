//! Lyrics loading: sidecar `.lrc` files, embedded audio tags, and online
//! providers (NetEase, then LRCLIB; an optional AcoustID fingerprint lookup
//! assists both when the tags themselves are wrong).
//!
//! Lyrics are loaded lazily for the current track (never during the library
//! scan), so a large collection pays no memory cost for text it does not show.
//! Local sources win (sidecar, then embedded tag), then a cached online result,
//! then a live provider search. Every source goes through the same LRC parser,
//! which also accepts plain-text lyrics.

use std::path::Path;
use std::time::Duration;

use crate::models::track::Track;

/// One displayed lyric line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricLine {
    /// Timestamp for synced lyrics; `None` for a plain-text line.
    pub time: Option<Duration>,
    pub text: String,
}

/// Where the lyrics came from, shown as a quiet hint in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LyricsSource {
    /// A `.lrc` file next to the audio file.
    Sidecar,
    /// The `LYRICS` / `USLT` tag inside the audio file.
    Embedded,
    /// NetEase search result, either live or from the local cache.
    NetEase,
    /// LRCLIB search result, either live or from the local cache.
    LrcLib,
}

impl LyricsSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Sidecar => "本地 .lrc 歌词",
            Self::Embedded => "内嵌歌词",
            Self::NetEase => "网易云歌词",
            Self::LrcLib => "LRCLIB歌词",
        }
    }
}

/// Parsed lyrics for one track.
#[derive(Debug, Clone)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    pub source: LyricsSource,
    /// `true` when every line carries a timestamp and can follow playback.
    pub synced: bool,
}

impl Lyrics {
    /// Index of the line highlighted at `position`, if playback has reached one.
    pub fn active_line(&self, position: Duration) -> Option<usize> {
        if !self.synced || self.lines.is_empty() {
            return None;
        }
        let next = self
            .lines
            .partition_point(|line| line.time.is_some_and(|time| time <= position));
        next.checked_sub(1)
    }

    /// Timestamp of `index`, used when the user clicks a synced line.
    pub fn line_time(&self, index: usize) -> Option<Duration> {
        self.lines.get(index).and_then(|line| line.time)
    }
}

/// Read lyrics for `track`.
///
/// Order: sidecar `.lrc` -> embedded tag -> cached online result -> live
/// search. `online` only controls the live search; a cached result is local
/// data and keeps working with the setting disabled. `acoustid_client`
/// enables the fingerprint-assisted retry for mis-tagged files.
pub fn load(track: &Track, online: bool, acoustid_client: &str) -> Option<Lyrics> {
    if let Some(text) = read_sidecar(&track.path)
        && let Some(lyrics) = parse(&text, LyricsSource::Sidecar)
    {
        return Some(lyrics);
    }

    if let Some(text) = crate::audio::decoder::read_embedded_lyrics(&track.path)
        && let Some(lyrics) = parse(&text, LyricsSource::Embedded)
    {
        return Some(lyrics);
    }

    if let Some((source, text)) = read_cache(track.id)
        && let Some(lyrics) = parse(&text, source)
    {
        return Some(lyrics);
    }

    if !online {
        return None;
    }

    let (source, text) = fetch_online(track, acoustid_client)?;
    let lyrics = parse(&text, source)?;
    write_cache(track.id, source, &text);
    Some(lyrics)
}

/// Live provider chain: NetEase first (strongest for Chinese catalogs), then
/// LRCLIB. When both miss and a client key is configured, AcoustID identifies
/// the recording from audio and the chain retries with the corrected metadata.
fn fetch_online(track: &Track, acoustid_client: &str) -> Option<(LyricsSource, String)> {
    if let Some(text) = crate::netease::fetch_lyrics(track) {
        return Some((LyricsSource::NetEase, text));
    }
    if let Some(text) = crate::lrclib::fetch_lyrics(track) {
        return Some((LyricsSource::LrcLib, text));
    }

    let identified = crate::acoustid::identify(track, acoustid_client)?;
    crate::log_info!(
        "acoustid identified '{}' by {} — retrying lyrics with corrected metadata",
        identified.title,
        identified.artist
    );
    let corrected = Track {
        title: identified.title.clone(),
        artist: identified.artist.clone(),
        ..track.clone()
    };
    if let Some(text) = crate::netease::fetch_lyrics(&corrected) {
        return Some((LyricsSource::NetEase, text));
    }
    if let Some(text) = crate::lrclib::fetch_lyrics(&corrected) {
        return Some((LyricsSource::LrcLib, text));
    }
    None
}

/// Read a previously fetched online lyric file together with its origin.
fn read_cache(track_id: u64) -> Option<(LyricsSource, String)> {
    let path = crate::paths::lyrics_cache_path(track_id);
    let bytes = std::fs::read(path).ok()?;
    let text = decode_text(&bytes)?;
    Some(split_cache_marker(&text))
}

/// Persist a network result so the next play works offline. A first-line
/// `[source:...]` marker keeps the origin label correct after a restart.
fn write_cache(track_id: u64, source: LyricsSource, text: &str) {
    let path = crate::paths::lyrics_cache_path(track_id);
    let marked = format!("{}\n{}", source_marker(source), text);
    if let Err(err) = std::fs::write(&path, marked) {
        crate::log_warn!("failed to cache lyrics at {}: {}", path.display(), err);
    }
}

/// First-line marker tag for a source. The LRC parser skips `[key:value]`
/// tags, so the marker never reaches the screen.
fn source_marker(source: LyricsSource) -> &'static str {
    match source {
        LyricsSource::NetEase => "[source:netease]",
        LyricsSource::LrcLib => "[source:lrclib]",
        LyricsSource::Sidecar | LyricsSource::Embedded => "",
    }
}

/// Split a leading `[source:...]` marker; caches written before the marker
/// existed are NetEase results.
fn split_cache_marker(text: &str) -> (LyricsSource, String) {
    if let Some((first, rest)) = text.split_once('\n') {
        let source = match first.trim() {
            "[source:lrclib]" => Some(LyricsSource::LrcLib),
            "[source:netease]" => Some(LyricsSource::NetEase),
            _ => None,
        };
        if let Some(source) = source {
            return (source, rest.to_string());
        }
    }
    (LyricsSource::NetEase, text.to_string())
}

/// Parse LRC or plain-text lyrics.
///
/// Supported LRC forms: `[mm:ss]`, `[mm:ss.xx]`, `[mm:ss.xxx]`, several
/// timestamps on one line, `[offset:+/-ms]`, and the usual `[ti:]` / `[ar:]`
/// metadata (ignored). Lines without a timestamp are kept as plain text, so
/// unsynchronised lyrics from a tag still display.
pub fn parse(text: &str, source: LyricsSource) -> Option<Lyrics> {
    let mut lines: Vec<LyricLine> = Vec::new();
    let mut offset_ms: i64 = 0;
    let mut saw_timestamp = false;
    let mut saw_plain_text = false;

    for raw in text.lines() {
        let mut rest = raw.trim_start();
        let mut times: Vec<i64> = Vec::new();

        // Consume leading bracket tags. A bracket without a timestamp or a
        // `key:` pair (e.g. "[Chorus]") is lyric text, not metadata.
        while let Some(after_bracket) = rest.strip_prefix('[') {
            let Some((tag, after)) = after_bracket.split_once(']') else {
                break;
            };

            if let Some(ms) = parse_timestamp(tag) {
                times.push(ms);
                rest = after.trim_start();
            } else if let Some(value) = tag.strip_prefix("offset:") {
                if let Ok(ms) = value.trim().parse::<i64>() {
                    offset_ms = ms;
                }
                rest = after.trim_start();
            } else if tag.contains(':') {
                // Other metadata tags ([ti:], [ar:], [by:], ...) are ignored.
                rest = after.trim_start();
            } else {
                break;
            }
        }

        let text = collapse_spaces(rest.trim_end());
        if text.is_empty() {
            // Metadata-only line, or a timed blank used as a gap. Keeping the
            // blank would give the active line an empty highlighted row.
            continue;
        }
        if times.is_empty() {
            saw_plain_text = true;
            lines.push(LyricLine {
                time: None,
                text: text.clone(),
            });
        } else {
            saw_timestamp = true;
            for ms in times {
                // Positive LRC offsets make lyrics appear earlier.
                let adjusted = (ms - offset_ms).max(0);
                lines.push(LyricLine {
                    time: Some(Duration::from_millis(adjusted as u64)),
                    text: text.clone(),
                });
            }
        }
    }

    if lines.is_empty() {
        return None;
    }

    let synced = saw_timestamp && !saw_plain_text;
    if synced {
        lines.sort_by_key(|line| line.time);
    } else {
        // Collapse runs of blank lines and drop the ones at the edges.
        let mut cleaned: Vec<LyricLine> = Vec::with_capacity(lines.len());
        for line in lines {
            let blank = line.text.is_empty();
            if blank && cleaned.last().is_none_or(|prev| prev.text.is_empty()) {
                continue;
            }
            cleaned.push(line);
        }
        while cleaned.last().is_some_and(|line| line.text.is_empty()) {
            cleaned.pop();
        }
        lines = cleaned;
        if lines.is_empty() {
            return None;
        }
    }

    Some(Lyrics {
        lines,
        source,
        synced,
    })
}

/// Collapse runs of spaces. Some providers (NetEase) pad lyrics with double
/// spaces, which looks broken in a proportional UI font.
fn collapse_spaces(text: &str) -> String {
    if !text.contains("  ") && !text.contains('\t') {
        return text.to_string();
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse `[mm:ss.xx]` style content into milliseconds.
fn parse_timestamp(tag: &str) -> Option<i64> {
    let (minutes, seconds) = tag.split_once(':')?;
    // `[hh:mm:ss]` and `[mm:ss:ff]` are not part of the common LRC dialect.
    if seconds.contains(':') {
        return None;
    }

    let minutes: u64 = minutes.trim().parse().ok()?;
    let seconds: f64 = seconds.trim().parse().ok()?;
    if !seconds.is_finite() || seconds < 0.0 || minutes > 10_000 {
        return None;
    }

    Some((minutes as f64 * 60_000.0 + seconds * 1_000.0).round() as i64)
}

/// Look for a `.lrc` file beside the audio file.
fn read_sidecar(audio_path: &Path) -> Option<String> {
    for extension in ["lrc", "LRC", "Lrc"] {
        let path = audio_path.with_extension(extension);
        if let Ok(bytes) = std::fs::read(&path) {
            return decode_text(&bytes);
        }
    }
    None
}

/// Decode a text file, tolerating the encodings local lyric files use.
fn decode_text(bytes: &[u8]) -> Option<String> {
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16(rest, true);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16(rest, false);
    }

    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Some(text.to_owned());
    }

    // Many Chinese lyric files are GBK rather than UTF-8.
    let (decoded, _, had_errors) = encoding_rs::GBK.decode(bytes);
    (!had_errors).then(|| decoded.into_owned())
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> Option<String> {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes([pair[0], pair[1]])
            } else {
                u16::from_be_bytes([pair[0], pair[1]])
            }
        })
        .collect();
    String::from_utf16(&units).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_synced_lrc_with_metadata_and_offset() {
        let text = "[ti:Song]\n[ar:Artist]\n[offset:500]\n[00:01.00]one\n[00:02.50][00:05.00]two\n";
        let lyrics = parse(text, LyricsSource::Sidecar).expect("lyrics");

        assert!(lyrics.synced);
        assert_eq!(lyrics.lines.len(), 3);
        assert_eq!(lyrics.lines[0].time, Some(Duration::from_millis(500)));
        assert_eq!(lyrics.lines[1].text, "two");
        assert_eq!(lyrics.lines[1].time, Some(Duration::from_millis(2000)));
        assert_eq!(lyrics.lines[2].time, Some(Duration::from_millis(4500)));
        assert_eq!(lyrics.active_line(Duration::from_millis(2000)), Some(1));
    }

    #[test]
    fn keeps_plain_text_and_section_markers() {
        let text = "line one\n\n[Chorus]\nline two\n";
        let lyrics = parse(text, LyricsSource::Embedded).expect("lyrics");

        assert!(!lyrics.synced);
        assert_eq!(
            lyrics
                .lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            ["line one", "[Chorus]", "line two"]
        );
    }

    #[test]
    fn rejects_empty_or_metadata_only_input() {
        assert!(parse("\n[ti:Only metadata]\n", LyricsSource::Sidecar).is_none());
    }

    #[test]
    fn collapses_provider_padding_spaces() {
        let lyrics =
            parse("[00:01.00]I'm  still  all  alone\n", LyricsSource::NetEase).expect("lyrics");
        assert_eq!(lyrics.lines[0].text, "I'm still all alone");
    }

    #[test]
    fn drops_timed_blank_gaps() {
        let lyrics = parse(
            "[00:01.00]first\n[00:02.00]\n[00:03.00]second\n",
            LyricsSource::NetEase,
        )
        .expect("lyrics");
        assert_eq!(lyrics.lines.len(), 2);
        assert_eq!(lyrics.active_line(Duration::from_millis(2500)), Some(0));
    }

    #[test]
    fn cache_marker_selects_the_source_and_is_stripped() {
        let (source, text) = split_cache_marker("[source:lrclib]\n[00:01.00]hello\n");
        assert_eq!(source, LyricsSource::LrcLib);
        assert_eq!(text, "[00:01.00]hello\n");
        assert_eq!(parse(&text, source).expect("lyrics").lines[0].text, "hello");

        let (source, text) = split_cache_marker("[source:netease]\nplain\n");
        assert_eq!(source, LyricsSource::NetEase);
        assert_eq!(text, "plain\n");
    }

    #[test]
    fn unmarked_caches_default_to_netease() {
        let (source, text) = split_cache_marker("[00:01.00]legacy\n");
        assert_eq!(source, LyricsSource::NetEase);
        assert_eq!(text, "[00:01.00]legacy\n");
    }
}
