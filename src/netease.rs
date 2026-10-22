//! NetEase Cloud Music provider over its unofficial web API.
//!
//! Caw is a personal, non-distributed player, so the unlicensed web endpoints
//! are acceptable here. Requests stay sequential and only run for the current
//! track. Lyrics are cached under `~/.config/caw/lyrics/`, covers under
//! `~/.config/caw/covers/`.

use std::cmp::Reverse;
use std::fmt::Write as _;
use std::time::Duration;

use serde::Deserialize;

use crate::models::track::Track;

const SEARCH_URL: &str = "https://music.163.com/api/search/get/web";
const LYRIC_URL: &str = "https://music.163.com/api/song/lyric";
const DETAIL_URL: &str = "https://music.163.com/api/song/detail";
/// Preferred cover side, appended as `?param=600y600`.
const COVER_PARAM: &str = "param=600y600";
const USER_AGENT: &str = concat!(
    "Caw/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/mtaech/caw)"
);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
/// Cool-down between consecutive requests to the same host.
const REQUEST_GAP: Duration = Duration::from_millis(120);
/// Search hits scoring below this are treated as a different song.
const MIN_SCORE: i32 = 130;
/// Fetch lyrics for at most this many ranked hits.
const MAX_CANDIDATES: usize = 3;

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    result: Option<SearchResult>,
}

#[derive(Debug, Default, Deserialize)]
struct SearchResult {
    #[serde(default)]
    songs: Vec<Song>,
}

#[derive(Debug, Clone, Deserialize)]
struct Song {
    id: i64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    artists: Vec<SongArtist>,
    #[serde(default)]
    album: Option<SongAlbum>,
    /// Duration in milliseconds.
    #[serde(default)]
    duration: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct SongArtist {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SongAlbum {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct LyricResponse {
    #[serde(default)]
    lrc: Option<LyricText>,
    /// Set by NetEase for tracks that have no lyrics at all.
    #[serde(default)]
    nolyric: bool,
    /// Set when nobody has submitted lyrics yet.
    #[serde(default)]
    uncollected: bool,
}

#[derive(Debug, Deserialize)]
struct LyricText {
    #[serde(default)]
    lyric: String,
}

#[derive(Debug, Deserialize)]
struct SongDetailResponse {
    #[serde(default)]
    songs: Vec<SongDetail>,
}

#[derive(Debug, Deserialize)]
struct SongDetail {
    #[serde(default)]
    album: Option<AlbumDetail>,
}

#[derive(Debug, Deserialize)]
struct AlbumDetail {
    #[serde(default, rename = "picUrl")]
    pic_url: String,
}

fn new_agent() -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build(),
    )
}

/// Search NetEase and return candidates sorted by match score.
fn search_candidates(agent: &ureq::Agent, track: &Track) -> Vec<Song> {
    // Try the exact query first; only widen to the qualifier-stripped title if
    // no hit is good enough. That keeps the common case at one search request.
    let mut songs: Vec<Song> = Vec::new();
    for query in search_queries(track) {
        match search(agent, &query) {
            Ok(mut found) => songs.append(&mut found),
            Err(err) => crate::log_debug!("netease search failed for '{}': {}", query, err),
        }
        if songs.iter().any(|song| score(song, track) >= MIN_SCORE) {
            break;
        }
        std::thread::sleep(REQUEST_GAP);
    }

    songs.sort_by_key(|song| Reverse(score(song, track)));
    let mut seen = std::collections::HashSet::new();
    songs.retain(|song| seen.insert(song.id));
    songs
}

/// Look up synced or plain lyrics on NetEase.
///
/// Returns the raw LRC/plain text; the caller parses and caches it.
pub(crate) fn fetch_lyrics(track: &Track) -> Option<String> {
    if track.display_title().trim().is_empty() {
        return None;
    }

    let agent = new_agent();
    let songs = search_candidates(&agent, track);

    for song in songs.iter().take(MAX_CANDIDATES) {
        let candidate_score = score(song, track);
        if candidate_score < MIN_SCORE {
            continue;
        }

        match lyrics(&agent, song.id) {
            Ok(Some(text)) => {
                crate::log_info!(
                    "netease lyrics: '{}' by {} -> '{}' ({:.0}s, score {})",
                    track.display_title(),
                    track.display_artist(),
                    song.name,
                    song.duration as f64 / 1000.0,
                    candidate_score
                );
                return Some(text);
            }
            Ok(None) => crate::log_debug!("netease: no lyrics for song id {}", song.id),
            Err(err) => crate::log_debug!("netease lyric request failed for {}: {}", song.id, err),
        }
        std::thread::sleep(REQUEST_GAP);
    }

    crate::log_debug!(
        "netease: no lyrics match for '{}' by {}",
        track.display_title(),
        track.display_artist()
    );
    None
}

/// Look up album art on NetEase and return the raw image bytes.
///
/// Tries the best-ranked candidates and falls back to the next one when a
/// record has no album picture. The image is requested at 600x600.
pub(crate) fn fetch_cover(track: &Track) -> Option<Vec<u8>> {
    if track.display_title().trim().is_empty() {
        return None;
    }

    let agent = new_agent();
    let songs = search_candidates(&agent, track);

    for song in songs.iter().take(MAX_CANDIDATES) {
        let candidate_score = score(song, track);
        if candidate_score < MIN_SCORE {
            continue;
        }

        match cover_url(&agent, song.id) {
            Ok(Some(url)) => match download_image(&agent, &url) {
                Ok(bytes) => {
                    crate::log_info!(
                        "netease cover: '{}' by {} -> '{}' ({} bytes)",
                        track.display_title(),
                        track.display_artist(),
                        song.name,
                        bytes.len()
                    );
                    return Some(bytes);
                }
                Err(err) => {
                    crate::log_debug!("netease cover download failed for {}: {}", song.id, err)
                }
            },
            Ok(None) => crate::log_debug!("netease: no cover for song id {}", song.id),
            Err(err) => crate::log_debug!("netease cover request failed for {}: {}", song.id, err),
        }
        std::thread::sleep(REQUEST_GAP);
    }

    None
}

/// Fetch a song's album cover URL from the detail endpoint.
fn cover_url(agent: &ureq::Agent, song_id: i64) -> anyhow::Result<Option<String>> {
    let url = format!("{DETAIL_URL}?ids=%5B{song_id}%5D");
    let body = request(agent, &url)?;
    let parsed: SongDetailResponse = serde_json::from_str(&body)
        .map_err(|err| anyhow::anyhow!("invalid song detail JSON: {err}"))?;

    let pic = parsed
        .songs
        .into_iter()
        .find_map(|song| song.album)
        .map(|album| album.pic_url)
        .unwrap_or_default();
    if pic.trim().is_empty() {
        return Ok(None);
    }

    let url = if pic.contains('?') {
        format!("{pic}&{COVER_PARAM}")
    } else {
        format!("{pic}?{COVER_PARAM}")
    };
    Ok(Some(url))
}

fn download_image(agent: &ureq::Agent, url: &str) -> anyhow::Result<Vec<u8>> {
    let mut response = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Referer", "https://music.163.com/")
        .call()?;
    let bytes = response.body_mut().read_to_vec()?;
    if !is_image(&bytes) {
        anyhow::bail!("response is not a JPEG/PNG/WebP image");
    }
    Ok(bytes)
}

/// Cheap signature check; avoids caching an HTML error page as a cover.
pub(crate) fn is_image(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47])
        || (bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WEBP")
}

fn is_unknown_artist(track: &Track) -> bool {
    let artist = track.artist.trim();
    artist.is_empty() || artist.eq_ignore_ascii_case("unknown artist")
}

/// Strip a leading track number like `001.` / `12_`, but not a title that
/// genuinely starts with digits (`24K Magic`, `7 Rings`).
fn strip_leading_track_number(text: &str) -> String {
    let text = text.trim();
    let digits_end = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    if digits_end == 0 || digits_end == text.len() {
        return text.to_string();
    }
    let separator = text[digits_end..].chars().next();
    if !matches!(separator, Some('.' | '-' | '_')) {
        return text.to_string();
    }
    text[digits_end + separator.unwrap().len_utf8()..]
        .trim()
        .to_string()
}

/// Filenames and untagged files often look like `王力宏 - 爱得得体`.
/// Split that into artist/title for matching; display text stays untouched.
fn split_filename_title(title: &str) -> Option<(String, String)> {
    let (artist, song) = title.split_once(" - ")?;
    let artist = strip_leading_track_number(artist);
    let song = song.trim();
    if artist.is_empty() || song.is_empty() {
        return None;
    }
    Some((artist, song.to_string()))
}

/// Title used for provider matching, with filename prefixes removed.
pub(crate) fn match_title(track: &Track) -> String {
    let title = track.display_title();
    if is_unknown_artist(track)
        && let Some((_, song)) = split_filename_title(title)
    {
        return song;
    }
    strip_leading_track_number(title)
}

/// Artist used for provider matching; derives it from `Artist - Title`
/// filenames when the tag is missing.
pub(crate) fn match_artist(track: &Track) -> String {
    if is_unknown_artist(track)
        && let Some((artist, _)) = split_filename_title(track.display_title())
    {
        return artist;
    }
    track.display_artist().to_string()
}

fn search_queries(track: &Track) -> Vec<String> {
    let title = match_title(track);
    let artist = match_artist(track);
    let artist_known = !artist.is_empty() && !artist.eq_ignore_ascii_case("unknown artist");

    let first = if artist_known {
        format!("{title} {artist}")
    } else {
        title.clone()
    };
    let mut queries = vec![first];

    let base = strip_qualifiers(&title);
    if !base.is_empty() && normalize(&base) != normalize(&title) {
        let widened = if artist_known {
            format!("{base} {artist}")
        } else {
            base
        };
        if !queries.contains(&widened) {
            queries.push(widened);
        }
    }
    queries
}

fn search(agent: &ureq::Agent, query: &str) -> anyhow::Result<Vec<Song>> {
    let url = format!(
        "{SEARCH_URL}?s={}&type=1&offset=0&total=true&limit=10",
        encode(query)
    );
    let body = request(agent, &url)?;
    let parsed: SearchResponse =
        serde_json::from_str(&body).map_err(|err| anyhow::anyhow!("invalid search JSON: {err}"))?;
    Ok(parsed.result.map(|result| result.songs).unwrap_or_default())
}

fn lyrics(agent: &ureq::Agent, song_id: i64) -> anyhow::Result<Option<String>> {
    let url = format!("{LYRIC_URL}?id={song_id}&lv=-1&kv=-1&tv=-1");
    let body = request(agent, &url)?;
    let parsed: LyricResponse =
        serde_json::from_str(&body).map_err(|err| anyhow::anyhow!("invalid lyric JSON: {err}"))?;

    if parsed.nolyric || parsed.uncollected {
        return Ok(None);
    }
    let text = parsed.lrc.map(|lrc| lrc.lyric).unwrap_or_default();
    if text.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(text))
}

fn request(agent: &ureq::Agent, url: &str) -> anyhow::Result<String> {
    let mut response = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Referer", "https://music.163.com/")
        .header("Accept", "application/json")
        .call()?;
    Ok(response.body_mut().read_to_string()?)
}

/// Rank a search hit against the local track.
///
/// Exact title beats a base title match; duration is the strongest guard
/// against picking a live/remix/cover version. A hit with a mismatched artist
/// needs a near-exact duration to pass `MIN_SCORE`.
fn score(song: &Song, track: &Track) -> i32 {
    let wanted_title = normalize(&match_title(track));
    let wanted_base = normalize_base(&match_title(track));
    let title = normalize(&song.name);
    let title_base = normalize_base(&song.name);

    let mut score = if title == wanted_title {
        100
    } else if !wanted_base.is_empty() && title_base == wanted_base {
        60
    } else if !wanted_base.is_empty()
        && !title_base.is_empty()
        && (title_base.contains(&wanted_base) || wanted_base.contains(&title_base))
    {
        20
    } else {
        return i32::MIN;
    };

    if artist_matches(song, track) {
        score += 60;
    }

    if let Some(album) = &song.album {
        let wanted_album = normalize(track.display_album());
        if !wanted_album.is_empty() && normalize(&album.name) == wanted_album {
            score += 15;
        }
    }

    if !track.duration.is_zero() {
        let diff = (song.duration as f64 / 1000.0 - track.duration.as_secs_f64()).abs();
        score += if diff <= 1.0 {
            40
        } else if diff <= 2.0 {
            35
        } else if diff <= 5.0 {
            15
        } else if diff <= 10.0 {
            -20
        } else if diff <= 20.0 {
            -80
        } else {
            -200
        };
    }

    score
}

fn artist_matches(song: &Song, track: &Track) -> bool {
    let wanted = normalize(&match_artist(track));
    if wanted.is_empty() || wanted == "unknownartist" {
        return false;
    }
    song.artists.iter().any(|artist| {
        let name = normalize(&artist.name);
        !name.is_empty() && (name == wanted || name.contains(&wanted) || wanted.contains(&name))
    })
}

/// Lowercase and keep only letters/digits, so punctuation and spacing from
/// different taggers do not break a match. CJK characters are alphanumeric.
pub(crate) fn normalize(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Normalize the title with bracketed qualifiers removed.
pub(crate) fn normalize_base(text: &str) -> String {
    normalize(&strip_qualifiers(text))
}

/// Remove `(Live)` / `[Remastered]` / `feat. ...` style qualifiers.
fn strip_qualifiers(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0u32;
    for c in text.chars() {
        match c {
            '(' | '[' | '（' | '【' => depth += 1,
            ')' | ']' | '）' | '】' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }

    let lower = out.to_lowercase();
    for marker in [" feat. ", " ft. ", " - remastered", " - live"] {
        if let Some(index) = lower.find(marker) {
            // Lowercasing can change byte lengths, so cut on a char boundary.
            let cut = (0..=index)
                .rev()
                .find(|&i| out.is_char_boundary(i))
                .unwrap_or(0);
            out.truncate(cut);
            break;
        }
    }
    out
}

/// Percent-encode a query string value (RFC 3986 unreserved characters).
pub(crate) fn encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            b' ' => out.push_str("%20"),
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::models::track::AudioFormat;

    fn track(title: &str, artist: &str, secs: u64) -> Track {
        Track {
            id: 1,
            path: PathBuf::from("/tmp/test.mp3"),
            title: title.to_string(),
            artist: artist.to_string(),
            album: String::new(),
            duration: Duration::from_secs(secs),
            track_number: 0,
            format: AudioFormat::Mp3,
            cover_data: None,
        }
    }

    fn song(id: i64, name: &str, artist: &str, secs: f64) -> Song {
        Song {
            id,
            name: name.to_string(),
            artists: vec![SongArtist {
                name: artist.to_string(),
            }],
            album: None,
            duration: (secs * 1000.0) as i64,
        }
    }

    #[test]
    fn strips_bracketed_qualifiers() {
        assert_eq!(normalize_base("In The Closet (7\" Edit)"), "inthecloset");
        assert_eq!(normalize_base("爱，很简单（Live）"), "爱很简单");
        assert_eq!(normalize_base("Song feat. Someone"), "song");
    }

    #[test]
    fn prefers_the_exact_studio_recording() {
        let wanted = track("爱，很简单", "陶喆", 269);
        let exact = song(1, "爱，很简单", "陶喆", 269.5);
        let live = song(2, "爱，很简单 (Live)", "陶喆", 320.0);

        assert!(score(&exact, &wanted) >= MIN_SCORE);
        assert!(score(&exact, &wanted) > score(&live, &wanted));
        assert!(score(&live, &wanted) < MIN_SCORE);
    }

    #[test]
    fn rejects_an_unrelated_title() {
        let wanted = track("Dangerous", "Michael Jackson", 407);
        let other = song(3, "Black or White", "Michael Jackson", 407.0);
        assert!(score(&other, &wanted) < MIN_SCORE);
    }

    #[test]
    fn derives_artist_and_track_from_filename() {
        let wanted = track("008.陶喆 - 宫保鸡丁", "Unknown Artist", 260);
        assert_eq!(match_title(&wanted), "宫保鸡丁");
        assert_eq!(match_artist(&wanted), "陶喆");

        let matched = song(4, "宫保鸡丁", "陶喆", 260.5);
        assert!(score(&matched, &wanted) >= MIN_SCORE);
    }

    #[test]
    fn keeps_numeric_song_titles() {
        let wanted = track("24K Magic", "Unknown Artist", 226);
        assert_eq!(match_title(&wanted), "24K Magic");
        assert_eq!(match_artist(&wanted), "Unknown Artist");
    }
}
