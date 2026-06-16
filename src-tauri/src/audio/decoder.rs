use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::models::track::{AudioFormat, Track};

/// Decoded audio data: interleaved f32 PCM samples.
#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
    #[allow(dead_code)]
    pub duration: Duration,
}

fn read_tags_from_revision(
    rev: &symphonia::core::meta::MetadataRevision,
    title: &mut String,
    artist: &mut String,
    album: &mut String,
    track_number: &mut u32,
    cover_data: &mut Option<Arc<Vec<u8>>>,
) {
    for tag in rev.tags() {
        match tag.std_key {
            Some(symphonia::core::meta::StandardTagKey::TrackTitle) => {
                if title.is_empty() {
                    *title = fix_mojibake(tag.value.to_string());
                }
            }
            Some(symphonia::core::meta::StandardTagKey::Artist) => {
                if artist.is_empty() {
                    *artist = fix_mojibake(tag.value.to_string());
                }
            }
            Some(symphonia::core::meta::StandardTagKey::Album) => {
                if album.is_empty() {
                    *album = fix_mojibake(tag.value.to_string());
                }
            }
            Some(symphonia::core::meta::StandardTagKey::TrackNumber) => {
                if *track_number == 0 {
                    *track_number = tag.value.to_string().parse().unwrap_or(0);
                }
            }
            _ => {}
        }
    }

    if cover_data.is_none() {
        if let Some(visual) = rev.visuals().first() {
            *cover_data = Some(Arc::new(visual.data.to_vec()));
        }
    }
}

/// Detect and repair double-mojibake from GBK-encoded tags.
///
/// Some rippers (mostly Chinese) store GBK bytes in FLAC Vorbis comments / ID3
/// tags instead of the UTF-8 required by the spec. Symphonia then reads those
/// bytes byte-by-byte as Latin-1 and packs them into a Rust `String` via UTF-8,
/// producing a *double* mojibake: e.g. GBK `c4 e3 ...` (你以为我是谁) becomes
/// the UTF-8 encoding of `ÄãÒÔÎªÎÒÊÇË` (`c3 84 c3 a3 ...`).
///
/// Heuristic (only touches suspicious strings; clean ASCII / valid UTF-8 CJK /
/// real Latin-1 text is left untouched):
///   1. All-ASCII  -> return as-is (fast path for the common case).
///   2. Any char >= U+00FF -> it's genuine UTF-8 (real CJK or extended Latin);
///      return as-is. (Mojibake can never produce chars above U+00FF because
///      each original byte maps to exactly one Latin-1 code point.)
///   3. Otherwise the string lives entirely in 0x00-0xFF and contains high
///      bytes -> likely mojibake. Reverse the UTF-8 encoding to recover the
///      original bytes, decode as GBK; if it yields valid text with CJK,
///      use it, else fall back to the original.
fn fix_mojibake(s: String) -> String {
    // 1. Pure ASCII fast path.
    if s.is_ascii() {
        return s;
    }
    // 2. Genuine UTF-8 with characters above the Latin-1 range.
    if s.chars().any(|c| c as u32 >= 0x100) {
        return s;
    }
    // 3. Suspected mojibake: each char's code point is one original byte.
    let raw: Vec<u8> = s.chars().map(|c| c as u8).collect();
    let (decoded, _, had_errors) = encoding_rs::GBK.decode(&raw);
    if had_errors {
        return s;
    }
    // Only accept if the result actually contains CJK (avoid mangling real
    // Latin-1 text like français).
    if decoded.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)) {
        return decoded.into_owned();
    }
    s
}

/// Extract metadata from an audio file, returning an `Arc<Track>`.
pub fn read_metadata(path: &Path, id: u64) -> Result<Arc<Track>> {
    let src = File::open(path).context("Failed to open audio file")?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .context("Failed to probe audio format")?;

    let mut title = String::new();
    let mut artist = String::new();
    let mut album = String::new();
    let mut track_number = 0u32;
    let mut cover_data: Option<Arc<Vec<u8>>> = None;

    // Read probed metadata.
    if let Some(meta) = probed.metadata.get() {
        if let Some(rev) = meta.current() {
            read_tags_from_revision(rev, &mut title, &mut artist, &mut album, &mut track_number, &mut cover_data);
        }
    }

    // Also check container metadata.
    let mut format = probed.format;
    if let Some(rev) = format.metadata().current() {
        read_tags_from_revision(rev, &mut title, &mut artist, &mut album, &mut track_number, &mut cover_data);
    }

    // Find audio track and get duration.
    let audio_track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("No audio track found")?;

    let codec_params = &audio_track.codec_params;
    let n_frames = codec_params.n_frames;
    let sample_rate = codec_params.sample_rate.unwrap_or(44100);

    let duration = n_frames
        .map(|n| {
            if sample_rate > 0 {
                Duration::from_secs(n / sample_rate as u64)
            } else {
                Duration::ZERO
            }
        })
        .unwrap_or(Duration::ZERO);

    let format_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();

    let audio_format =
        AudioFormat::from_extension(&format_ext).unwrap_or(AudioFormat::Other(format_ext));

    Ok(Arc::new(Track {
        id,
        path: path.to_path_buf(),
        title,
        artist,
        album,
        duration,
        track_number,
        format: audio_format,
        cover_data,
    }))
}

/// Fully decode an audio file into PCM f32 samples.
pub fn decode_file(path: &Path) -> Result<DecodedAudio> {
    let src = File::open(path).context("Failed to open audio file")?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .context("Failed to probe audio format")?;

    let mut format = probed.format;

    let audio_track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("No audio track found")?;

    let codec_params = &audio_track.codec_params;
    let track_id = audio_track.id;
    let sample_rate = codec_params.sample_rate.unwrap_or(44100);
    let channels = codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(2);

    let dec_opts = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&audio_track.codec_params, &dec_opts)
        .context("Failed to create decoder")?;

    let mut all_samples: Vec<f32> = Vec::new();
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        match format.next_packet() {
            Ok(packet) => {
                if packet.track_id() != track_id {
                    continue;
                }
                match decoder.decode(&packet) {
                    Ok(decoded) => {
                        let spec = *decoded.spec();
                        let frames = decoded.frames();

                        if sample_buf.is_none() {
                            sample_buf = Some(SampleBuffer::<f32>::new(
                                frames as u64 * spec.channels.count() as u64,
                                spec,
                            ));
                        }

                        if let Some(buf) = &mut sample_buf {
                            buf.copy_interleaved_ref(decoded);
                            all_samples.extend_from_slice(buf.samples());
                        }
                    }
                    Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
                    Err(symphonia::core::errors::Error::IoError(_)) => continue,
                    Err(e) => return Err(e.into()),
                }
            }
            Err(symphonia::core::errors::Error::ResetRequired) => {}
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(e.into()),
        }
    }

    let total_frames = all_samples.len() as u64 / channels as u64;
    let duration = if sample_rate > 0 {
        Duration::from_secs(total_frames / sample_rate as u64)
    } else {
        Duration::ZERO
    };

    Ok(DecodedAudio {
        sample_rate,
        channels,
        samples: all_samples,
        duration,
    })
}
