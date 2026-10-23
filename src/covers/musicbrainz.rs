//! MusicBrainz + Cover Art Archive cover fallback.
//!
//! MusicBrainz is the open CC0 music encyclopedia; the Cover Art Archive (a
//! joint project with the Internet Archive) serves its release-group front
//! art. The MusicBrainz web API is rate limited to one request per second and
//! requires a meaningful User-Agent, so caw spends exactly one request on the
//! recording search and then pulls covers from CAA, a separate host.

use std::time::Duration;

use serde::Deserialize;

use crate::models::track::Track;
use crate::netease::{encode, is_image, match_artist, match_title, normalize, score_candidate};

const SEARCH_URL: &str = "https://musicbrainz.org/ws/2/recording";
const CAA_URL: &str = "https://coverartarchive.org/release-group";
const USER_AGENT: &str = concat!(
    "Caw/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/mtaech/caw)"
);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
/// A recording must plausibly be the same song: exact/base title match plus
/// an artist or duration agreement.
const MIN_SCORE: i32 = 100;
/// CAA has no art for some release groups (404); try a few before giving up.
const MAX_RELEASE_GROUPS: usize = 3;

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    recordings: Vec<Recording>,
}

#[derive(Debug, Clone, Deserialize)]
struct Recording {
    #[serde(default)]
    title: String,
    /// Recording length in milliseconds.
    #[serde(default)]
    length: Option<i64>,
    #[serde(default, rename = "artist-credit")]
    artist_credit: Vec<Credit>,
    #[serde(default)]
    releases: Vec<Release>,
}

#[derive(Debug, Clone, Deserialize)]
struct Credit {
    #[serde(default)]
    name: String,
    #[serde(default)]
    artist: Option<CreditArtist>,
}

impl Credit {
    /// Credits fall back to the artist entity name when no display name is set.
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

#[derive(Debug, Clone, Deserialize)]
struct CreditArtist {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Release {
    #[serde(default)]
    title: String,
    #[serde(default, rename = "release-group")]
    release_group: Option<ReleaseGroup>,
}

#[derive(Debug, Clone, Deserialize)]
struct ReleaseGroup {
    #[serde(default)]
    id: String,
}

/// Fetch cover bytes for `track` from the Cover Art Archive.
pub(crate) fn fetch_cover(track: &Track) -> Option<Vec<u8>> {
    if track.display_title().trim().is_empty() {
        return None;
    }

    let agent = new_agent();
    let groups = ranked_release_groups(&agent, track);
    for group in groups.iter().take(MAX_RELEASE_GROUPS) {
        match fetch_front(&agent, &group.id) {
            Ok(bytes) => {
                crate::log_info!(
                    "musicbrainz cover: '{}' by {} -> release group {} ({} bytes)",
                    track.display_title(),
                    track.display_artist(),
                    group.id,
                    bytes.len()
                );
                return Some(bytes);
            }
            Err(err) => crate::log_debug!("caa front for {} failed: {}", group.id, err),
        }
    }
    None
}

/// Fetch the front cover of one release group directly (fingerprint path).
pub(crate) fn fetch_cover_by_release_group(release_group_id: &str) -> Option<Vec<u8>> {
    let agent = new_agent();
    fetch_front(&agent, release_group_id).ok()
}

fn new_agent() -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build(),
    )
}

/// Release groups of same-song recordings, album matches first.
fn ranked_release_groups(agent: &ureq::Agent, track: &Track) -> Vec<ReleaseGroup> {
    let recordings = match search(agent, track) {
        Ok(found) => found,
        Err(err) => {
            crate::log_debug!("musicbrainz search failed: {err}");
            return Vec::new();
        }
    };

    let mut groups: Vec<(bool, ReleaseGroup)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for recording in recordings {
        let artist = recording
            .artist_credit
            .iter()
            .map(|credit| credit.resolved_name())
            .collect::<Vec<_>>()
            .join("");
        if score_candidate(
            &recording.title,
            &artist,
            None,
            recording.length.unwrap_or(0),
            track,
        ) < MIN_SCORE
        {
            continue;
        }
        // A release group matching the local album tag beats the compilations
        // CAA knows nothing about; stable sort keeps the server's relevance
        // order inside each tier.
        for release in &recording.releases {
            let Some(group) = release.release_group.as_ref() else {
                continue;
            };
            if group.id.trim().is_empty() || !seen.insert(group.id.clone()) {
                continue;
            }
            let album_match = !track.album.trim().is_empty()
                && normalize(&release.title) == normalize(track.display_album());
            groups.push((album_match, group.clone()));
        }
    }
    groups.sort_by_key(|(album_match, _)| std::cmp::Reverse(*album_match));
    groups.into_iter().map(|(_, group)| group).collect()
}

/// Search recordings via Lucene query syntax: quoted phrases, ANDed fields.
fn search(agent: &ureq::Agent, track: &Track) -> anyhow::Result<Vec<Recording>> {
    let title = match_title(track).replace('"', "");
    let artist = match_artist(track);
    let artist_known = !artist.is_empty() && !artist.eq_ignore_ascii_case("unknown artist");

    let query = if artist_known {
        format!(
            "recording:\"{title}\" AND artist:\"{}\"",
            artist.replace('"', "")
        )
    } else {
        format!("\"{title}\"")
    };
    let url = format!("{SEARCH_URL}?fmt=json&limit=10&query={}", encode(&query));
    let mut response = agent
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/json")
        .call()?;
    let body = response.body_mut().read_to_string()?;
    let parsed: SearchResponse = serde_json::from_str(&body)
        .map_err(|err| anyhow::anyhow!("invalid MusicBrainz JSON: {err}"))?;
    Ok(parsed.recordings)
}

/// Download a release group's front art at ~600px.
fn fetch_front(agent: &ureq::Agent, release_group_id: &str) -> anyhow::Result<Vec<u8>> {
    let url = format!("{CAA_URL}/{release_group_id}/front-600");
    let mut response = agent.get(&url).header("User-Agent", USER_AGENT).call()?;
    if !response.status().is_success() {
        anyhow::bail!("CAA returned {}", response.status());
    }
    let bytes = response.body_mut().read_to_vec()?;
    if !is_image(&bytes) {
        anyhow::bail!("response is not an image");
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_response_with_release_groups() {
        let body = r#"{"created":"2026-01-01T00:00:00.000Z","count":1,"offset":0,"recordings":[
            {"id":"r1","score":100,"title":"Billie Jean","length":297000,
             "artist-credit":[{"name":"Michael Jackson"}],
             "releases":[
                {"title":"Thriller","release-group":{"id":"rg-album","title":"Thriller","primary-type":"Album"}},
                {"title":"Some Compilation","release-group":{"id":"rg-comp","title":"Some Compilation"}}]}]}"#;
        let parsed: SearchResponse = serde_json::from_str(body).expect("json");
        assert_eq!(parsed.recordings.len(), 1);
        let releases = &parsed.recordings[0].releases;
        assert_eq!(releases.len(), 2);
        assert_eq!(
            releases[0].release_group.as_ref().expect("group").id,
            "rg-album"
        );
        assert!(releases[1].release_group.is_some());
    }

    #[test]
    fn resolves_credit_fallback_artist_name() {
        let credit: Credit =
            serde_json::from_str(r#"{"artist":{"name":"Michael Jackson"}}"#).expect("credit");
        assert_eq!(credit.resolved_name(), "Michael Jackson");

        let named: Credit = serde_json::from_str(r#"{"name":"MJ"}"#).expect("credit");
        assert_eq!(named.resolved_name(), "MJ");
    }
}
