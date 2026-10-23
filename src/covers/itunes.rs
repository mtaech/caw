//! iTunes Search API cover fallback.

use std::cmp::Reverse;
use std::time::Duration;

use serde::Deserialize;

use crate::models::track::Track;
use crate::netease::{encode, is_image, match_artist, match_title};

const SEARCH_URL: &str = "https://itunes.apple.com/search";
const USER_AGENT: &str = concat!(
    "Caw/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/mtaech/caw)"
);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const MIN_SCORE: i32 = 130;
const MAX_CANDIDATES: usize = 3;

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    results: Vec<Item>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    #[serde(default)]
    track_name: String,
    #[serde(default)]
    artist_name: String,
    #[serde(default)]
    collection_name: String,
    #[serde(default)]
    track_time_millis: Option<u64>,
    #[serde(default)]
    artwork_url100: String,
}

/// Search the US store first, then the Taiwan store for Chinese catalogs.
/// The latter often uses traditional titles, so NetEase remains the primary
/// provider; iTunes mainly covers Western tracks that NetEase misses.
pub(super) fn fetch_cover(track: &Track) -> Option<Vec<u8>> {
    if track.display_title().trim().is_empty() {
        return None;
    }

    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build(),
    );
    let term = search_term(track);

    let mut items: Vec<Item> = Vec::new();
    for country in ["us", "tw"] {
        match search(&agent, &term, country) {
            Ok(mut found) => items.append(&mut found),
            Err(err) => crate::log_debug!("itunes search failed ({}): {}", country, err),
        }
        if items.iter().any(|item| score(item, track) >= MIN_SCORE) {
            break;
        }
        std::thread::sleep(Duration::from_millis(150));
    }

    items.sort_by_key(|item| Reverse(score(item, track)));
    let mut seen = std::collections::HashSet::new();
    items.retain(|item| seen.insert(item.artwork_url100.clone()));

    for item in items.iter().take(MAX_CANDIDATES) {
        let candidate_score = score(item, track);
        if candidate_score < MIN_SCORE {
            continue;
        }
        let Some(url) = artwork_url(item) else {
            continue;
        };
        match download(&agent, &url) {
            Ok(bytes) => {
                crate::log_info!(
                    "itunes cover: '{}' by {} -> '{}' ({} bytes)",
                    track.display_title(),
                    track.display_artist(),
                    item.collection_name,
                    bytes.len()
                );
                return Some(bytes);
            }
            Err(err) => crate::log_debug!("itunes cover download failed: {}", err),
        }
    }

    None
}

fn search_term(track: &Track) -> String {
    let title = match_title(track);
    let artist = match_artist(track);
    if artist.is_empty() || artist.eq_ignore_ascii_case("unknown artist") {
        title
    } else {
        format!("{title} {artist}")
    }
}

fn search(agent: &ureq::Agent, term: &str, country: &str) -> anyhow::Result<Vec<Item>> {
    let url = format!(
        "{SEARCH_URL}?term={}&entity=song&limit=10&country={country}",
        encode(term)
    );
    let mut response = agent
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/json")
        .call()?;
    let body = response.body_mut().read_to_string()?;
    let parsed: SearchResponse = serde_json::from_str(&body)
        .map_err(|err| anyhow::anyhow!("invalid iTunes search JSON: {err}"))?;
    Ok(parsed.results)
}

fn artwork_url(item: &Item) -> Option<String> {
    if item.artwork_url100.trim().is_empty() {
        return None;
    }
    Some(
        item.artwork_url100
            .replace("100x100bb", "600x600bb")
            .replace("100x100-75", "600x600-75"),
    )
}

fn download(agent: &ureq::Agent, url: &str) -> anyhow::Result<Vec<u8>> {
    let mut response = agent.get(url).header("User-Agent", USER_AGENT).call()?;
    let bytes = response.body_mut().read_to_vec()?;
    if !is_image(&bytes) {
        anyhow::bail!("response is not an image");
    }
    Ok(bytes)
}

fn score(item: &Item, track: &Track) -> i32 {
    crate::netease::score_candidate(
        &item.track_name,
        &item.artist_name,
        Some(&item.collection_name),
        item.track_time_millis.map(|ms| ms as i64).unwrap_or(0),
        track,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, artist: &str, album: &str, secs: f64) -> Item {
        Item {
            track_name: name.to_string(),
            artist_name: artist.to_string(),
            collection_name: album.to_string(),
            track_time_millis: Some((secs * 1000.0) as u64),
            artwork_url100: "https://example.com/100x100bb.jpg".to_string(),
        }
    }

    fn track(name: &str, artist: &str) -> Track {
        Track {
            id: 1,
            path: std::path::PathBuf::from("/tmp/test.mp3"),
            title: name.to_string(),
            artist: artist.to_string(),
            album: String::new(),
            duration: Duration::from_secs(300),
            track_number: 0,
            format: crate::models::track::AudioFormat::Mp3,
            cover_data: None,
        }
    }

    #[test]
    fn upgrades_artwork_url_to_600px() {
        let item = item("Song", "Artist", "Album", 300.0);
        assert_eq!(
            artwork_url(&item).as_deref(),
            Some("https://example.com/600x600bb.jpg")
        );
    }

    #[test]
    fn prefers_the_exact_song() {
        let wanted = track("Song", "Artist");
        let exact = item("Song", "Artist", "Album", 300.0);
        let live = item("Song (Live)", "Artist", "Album", 380.0);
        assert!(score(&exact, &wanted) >= MIN_SCORE);
        assert!(score(&exact, &wanted) > score(&live, &wanted));
    }
}
