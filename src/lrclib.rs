//! LRCLIB lyrics provider over its free, keyless API (https://lrclib.net).
//!
//! LRCLIB serves synced (LRC) and plain lyrics and is strongest for Western
//! and Japanese catalogs; NetEase stays the primary provider for Chinese
//! ones. Requests stay sequential and only run for the current track; the
//! caller parses and caches the result.

use std::time::Duration;

use serde::Deserialize;

use crate::models::track::Track;
use crate::netease::{encode, match_artist, match_title, score_candidate};

const API_URL: &str = "https://lrclib.net/api";
const USER_AGENT: &str = concat!(
    "Caw/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/mtaech/caw)"
);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
/// Cool-down between consecutive requests; the public API rate limits hard.
const REQUEST_GAP: Duration = Duration::from_millis(300);
/// Search hits scoring below this are treated as a different song.
const MIN_SCORE: i32 = 130;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    #[serde(default)]
    track_name: String,
    #[serde(default)]
    artist_name: String,
    #[serde(default)]
    album_name: String,
    /// Duration in seconds (may be fractional).
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    instrumental: bool,
    #[serde(default)]
    plain_lyrics: String,
    #[serde(default)]
    synced_lyrics: String,
}

/// Look up synced or plain lyrics on LRCLIB.
///
/// Returns the raw LRC/plain text; the caller parses and caches it.
pub(crate) fn fetch_lyrics(track: &Track) -> Option<String> {
    if track.display_title().trim().is_empty() {
        return None;
    }

    let agent = new_agent();
    let record = best_record(&agent, track)?;
    let text = record_text(&record)?;
    crate::log_info!(
        "lrclib lyrics: '{}' by {} -> '{}' ({} , {} bytes)",
        track.display_title(),
        track.display_artist(),
        record.track_name,
        if record.synced_lyrics.trim().is_empty() {
            "plain"
        } else {
            "synced"
        },
        text.len()
    );
    Some(text)
}

fn new_agent() -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build(),
    )
}

/// Best record for `track`: a server-side signature lookup first, then a
/// ranked search. Returns `None` when nothing scores as the same song.
fn best_record(agent: &ureq::Agent, track: &Track) -> Option<Record> {
    let title = match_title(track);
    let artist = match_artist(track);
    let artist_known = !artist.is_empty() && !artist.eq_ignore_ascii_case("unknown artist");

    // `/get` matches on the exact signature; a miss is normal and cheap.
    if artist_known {
        let album = track.album.trim();
        let mut url = format!(
            "{API_URL}/get?track_name={}&artist_name={}",
            encode(&title),
            encode(&artist)
        );
        if !album.is_empty() {
            url.push_str(&format!("&album_name={}", encode(album)));
        }
        if !track.duration.is_zero() {
            url.push_str(&format!("&duration={}", track.duration.as_secs()));
        }
        match request_json::<Record>(agent, &url) {
            Ok(record) => {
                if score_record(&record, track) >= MIN_SCORE {
                    return Some(record);
                }
            }
            Err(err) => crate::log_debug!("lrclib get failed: {err}"),
        }
        std::thread::sleep(REQUEST_GAP);
    }

    let url = if artist_known {
        format!(
            "{API_URL}/search?track_name={}&artist_name={}",
            encode(&title),
            encode(&artist)
        )
    } else {
        format!("{API_URL}/search?q={}", encode(&title))
    };
    let records: Vec<Record> = request_json(agent, &url).ok()?;

    let mut best: Option<(i32, Record)> = None;
    for record in records {
        let score = score_record(&record, track);
        if score < MIN_SCORE {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|(best_score, _)| score > *best_score)
        {
            best = Some((score, record));
        }
    }
    best.map(|(_, record)| record)
}

fn score_record(record: &Record, track: &Track) -> i32 {
    score_candidate(
        &record.track_name,
        &record.artist_name,
        Some(&record.album_name),
        (record.duration * 1000.0) as i64,
        track,
    )
}

/// Synced lyrics win over plain text; instrumental tracks carry neither.
fn record_text(record: &Record) -> Option<String> {
    if record.instrumental {
        return None;
    }
    let synced = record.synced_lyrics.trim();
    if !synced.is_empty() {
        return Some(synced.to_string());
    }
    let plain = record.plain_lyrics.trim();
    (!plain.is_empty()).then(|| plain.to_string())
}

fn request_json<T: serde::de::DeserializeOwned>(
    agent: &ureq::Agent,
    url: &str,
) -> anyhow::Result<T> {
    let mut response = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/json")
        .call()?;
    let body = response.body_mut().read_to_string()?;
    serde_json::from_str(&body).map_err(|err| anyhow::anyhow!("invalid LRCLIB JSON: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::models::track::AudioFormat;

    fn track(title: &str, artist: &str, secs: u64) -> Track {
        Track {
            id: 1,
            path: PathBuf::from("/tmp/test.mp3"),
            title: title.to_string(),
            artist: artist.to_string(),
            album: "Thriller".to_string(),
            duration: Duration::from_secs(secs),
            track_number: 0,
            format: AudioFormat::Mp3,
            cover_data: None,
        }
    }

    fn record(name: &str, artist: &str, album: &str, secs: f64) -> Record {
        Record {
            track_name: name.to_string(),
            artist_name: artist.to_string(),
            album_name: album.to_string(),
            duration: secs,
            instrumental: false,
            plain_lyrics: String::new(),
            synced_lyrics: String::new(),
        }
    }

    #[test]
    fn parses_record_and_prefers_synced_lyrics() {
        let parsed: Record = serde_json::from_str(
            r#"{"id":130407,"trackName":"Billie Jean","artistName":"Michael Jackson",
                "albumName":"Thriller","duration":294.0,"instrumental":false,
                "plainLyrics":"plain text",
                "syncedLyrics":"[00:29.35] She was more like a beauty queen"}"#,
        )
        .expect("record");
        assert_eq!(parsed.track_name, "Billie Jean");
        assert_eq!(
            record_text(&parsed).as_deref(),
            Some("[00:29.35] She was more like a beauty queen")
        );
    }

    #[test]
    fn falls_back_to_plain_lyrics() {
        let parsed: Record = serde_json::from_str(
            r#"{"trackName":"Song","plainLyrics":"just text","syncedLyrics":""}"#,
        )
        .expect("record");
        assert_eq!(record_text(&parsed).as_deref(), Some("just text"));
    }

    #[test]
    fn instrumental_records_have_no_text() {
        let parsed: Record = serde_json::from_str(
            r#"{"trackName":"Mute","instrumental":true,"plainLyrics":"","syncedLyrics":""}"#,
        )
        .expect("record");
        assert!(record_text(&parsed).is_none());
    }

    #[test]
    fn scores_the_studio_recording_above_a_live_version() {
        let wanted = track("Billie Jean", "Michael Jackson", 294);
        let exact = record("Billie Jean", "Michael Jackson", "Thriller", 294.0);
        let live = record("Billie Jean (Live)", "Michael Jackson", "Live", 380.0);

        assert!(score_record(&exact, &wanted) >= MIN_SCORE);
        assert!(score_record(&exact, &wanted) > score_record(&live, &wanted));
        assert!(score_record(&live, &wanted) < MIN_SCORE);
    }

    #[test]
    fn rejects_an_unrelated_record() {
        let wanted = track("Billie Jean", "Michael Jackson", 294);
        let other = record("Human Nature", "Michael Jackson", "Thriller", 245.0);
        assert!(score_record(&other, &wanted) < MIN_SCORE);
    }
}
