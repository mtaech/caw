//! AcoustID fingerprint lookup for mis-tagged files.
//!
//! Text search cannot find "track01.mp3" or a file whose garbled tags name a
//! different song. This module identifies the recording from audio instead:
//! it decodes up to two minutes, computes a chromaprint fingerprint (pure
//! Rust port; the same TEST2 algorithm `fpcalc` uses by default) and asks
//! the AcoustID web service which recording it is. Callers retry their normal
//! providers with the corrected title/artist and can fetch the cover straight
//! from the matched release group.
//!
//! Opt-in: the service needs a per-application client key. Set
//! `acoustid_client` in `~/.config/caw/config.json` (register instantly at
//! https://acoustid.org/new-application). An empty key disables everything
//! here, including the audio decode.

use std::cmp::Ordering;
use std::time::Duration;

use base64::Engine as _;
use rusty_chromaprint::{Configuration, FingerprintCompressor, Fingerprinter};
use serde::Deserialize;

use crate::audio::decoder::DecodedAudio;
use crate::models::track::Track;
use crate::netease::encode;

const LOOKUP_URL: &str = "https://api.acoustid.org/v2/lookup";
const USER_AGENT: &str = concat!(
    "Caw/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/mtaech/caw)"
);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// How much audio feeds the fingerprint; AcoustID matches fine on a prefix.
const FINGERPRINT_DURATION: Duration = Duration::from_secs(120);
/// Below this lookup score the "match" is more likely a different song.
const MIN_RESULT_SCORE: f32 = 0.7;

/// Metadata recovered from the audio itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Identified {
    pub title: String,
    pub artist: String,
    /// Album/release-group title when the lookup exposed one.
    pub album: Option<String>,
    /// MusicBrainz release-group MBID, for direct Cover Art Archive fetches.
    pub release_group: Option<String>,
}

/// Identify `track` by audio. `client_key` comes from the config; when it is
/// empty this returns `None` without decoding anything.
pub(crate) fn identify(track: &Track, client_key: &str) -> Option<Identified> {
    let client_key = client_key.trim();
    if client_key.is_empty() {
        return None;
    }
    // Too-short clips fingerprint poorly and would match randomly.
    if track.duration < Duration::from_secs(10) {
        return None;
    }

    let audio = crate::audio::decoder::decode_prefix(&track.path, FINGERPRINT_DURATION).ok()?;
    let fingerprint = fingerprint_string(&audio)?;
    let seconds =
        audio.samples.len() as u64 / audio.channels.max(1) as u64 / audio.sample_rate.max(1) as u64;
    if seconds < 10 {
        return None;
    }

    let url = format!(
        "{LOOKUP_URL}?client={}&duration={seconds}&meta=recordings+releasegroups&format=json&fingerprint={}",
        encode(client_key),
        encode(&fingerprint)
    );

    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build(),
    );
    let mut response = agent
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/json")
        .call()
        .ok()?;
    let body = response.body_mut().read_to_string().ok()?;
    let parsed: LookupResponse = serde_json::from_str(&body).ok()?;
    best_identified(parsed)
}

/// Pick the highest-scored result and turn it into metadata.
fn best_identified(response: LookupResponse) -> Option<Identified> {
    let best = response
        .results
        .into_iter()
        .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal))?;
    if best.score < MIN_RESULT_SCORE {
        crate::log_debug!("acoustid: best score {:.2} below threshold", best.score);
        return None;
    }
    let recording = best
        .recordings
        .into_iter()
        .find(|recording| !recording.title.trim().is_empty())?;
    let artist = recording
        .artist_credit
        .first()
        .map(|credit| credit.resolved_name())
        .filter(|name| !name.trim().is_empty())?
        .to_string();
    let group = recording.releasegroups.first();
    Some(Identified {
        title: recording.title,
        artist,
        album: group
            .filter(|group| !group.title.trim().is_empty())
            .map(|group| group.title.clone()),
        release_group: group
            .filter(|group| is_mbid(&group.id))
            .map(|group| group.id.clone()),
    })
}

/// A trimmed MBID: 36-char hyphenated UUID shape. Loose on purpose — the CAA
/// request simply 404s when the id is bogus.
fn is_mbid(id: &str) -> bool {
    id.len() == 36 && id.matches('-').count() == 4
}

/// Chromaprint fingerprint, compressed and base64-encoded like `fpcalc` does.
fn fingerprint_string(audio: &DecodedAudio) -> Option<String> {
    let config = Configuration::preset_test2();
    let mut printer = Fingerprinter::new(&config);
    printer
        .start(audio.sample_rate, audio.channels as u32)
        .ok()?;
    for chunk in audio.samples.chunks(8192) {
        let scaled: Vec<i16> = chunk
            .iter()
            .map(|sample| (sample.clamp(-1.0, 1.0) * 32_767.0) as i16)
            .collect();
        printer.consume(&scaled);
    }
    printer.finish();
    let compressed = FingerprintCompressor::from(&config).compress(printer.fingerprint());
    Some(base64::engine::general_purpose::STANDARD.encode(compressed))
}

#[derive(Debug, Deserialize)]
struct LookupResponse {
    #[serde(default)]
    results: Vec<LookupResult>,
}

#[derive(Debug, Deserialize)]
struct LookupResult {
    #[serde(default)]
    score: f32,
    #[serde(default)]
    recordings: Vec<AcoustRecording>,
}

#[derive(Debug, Deserialize)]
struct AcoustRecording {
    #[serde(default)]
    title: String,
    #[serde(default, rename = "artist-credit")]
    artist_credit: Vec<Credit>,
    #[serde(default)]
    releasegroups: Vec<AcoustReleaseGroup>,
}

#[derive(Debug, Deserialize)]
struct Credit {
    #[serde(default)]
    name: String,
    #[serde(default)]
    artist: Option<CreditArtist>,
}

impl Credit {
    fn resolved_name(&self) -> &str {
        if self.name.is_empty() {
            self.artist
                .as_ref()
                .map(|artist| artist.name.as_str())
                .unwrap_or_default()
        } else {
            &self.name
        }
    }
}

#[derive(Debug, Deserialize)]
struct CreditArtist {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct AcoustReleaseGroup {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::models::track::AudioFormat;

    fn track() -> Track {
        Track {
            id: 1,
            path: PathBuf::from("/definitely/not/there.flac"),
            title: "Unknown".to_string(),
            artist: String::new(),
            album: String::new(),
            duration: Duration::from_secs(200),
            track_number: 0,
            format: AudioFormat::Flac,
            cover_data: None,
        }
    }

    #[test]
    fn empty_client_key_disables_identification_without_decoding() {
        // The path does not exist; reaching the decoder would fail the test.
        assert!(identify(&track(), "").is_none());
        assert!(identify(&track(), "   ").is_none());
    }

    #[test]
    fn parses_lookup_and_picks_the_best_recording() {
        let body = r#"{"status":"ok","results":[
            {"score":0.60,"id":"low","recordings":[{"title":"Wrong","artist-credit":[{"name":"Nope"}]}]},
            {"score":0.98,"id":"high","recordings":[
                {"title":"Billie Jean","duration":294,
                 "artist-credit":[{"name":"Michael Jackson"}],
                 "releasegroups":[{"id":"a8d45c46-9f1d-4961-9804-230659d4e948","title":"The Essential"}]}]}]}"#;
        let parsed: LookupResponse = serde_json::from_str(body).expect("json");
        let identified = best_identified(parsed).expect("identified");
        assert_eq!(identified.title, "Billie Jean");
        assert_eq!(identified.artist, "Michael Jackson");
        assert_eq!(identified.album.as_deref(), Some("The Essential"));
        assert_eq!(
            identified.release_group.as_deref(),
            Some("a8d45c46-9f1d-4961-9804-230659d4e948")
        );
    }

    #[test]
    fn low_scores_are_rejected() {
        let body = r#"{"status":"ok","results":[{"score":0.42,"id":"low","recordings":[
            {"title":"Guess","artist-credit":[{"name":"Who"}]}]}]}"#;
        let parsed: LookupResponse = serde_json::from_str(body).expect("json");
        assert!(best_identified(parsed).is_none());
    }

    #[test]
    fn rejects_bogus_mbids() {
        assert!(is_mbid("a8d45c46-9f1d-4961-9804-230659d4e948"));
        assert!(!is_mbid("not-an-mbid"));
        assert!(!is_mbid(""));
    }
}
