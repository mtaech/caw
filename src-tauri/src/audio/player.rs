use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};

use crate::audio::decoder::DecodedAudio;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Commands sent to the audio player thread.
#[derive(Debug)]
#[allow(dead_code)]
pub enum PlayerCommand {
    /// Start playing the given decoded audio. Optionally seek to position (seconds).
    Play(DecodedAudio, f64),
    Pause,
    Resume,
    Stop,
    /// Seek to position in seconds.
    Seek(f64),
    /// Set volume 0.0 – 1.0.
    SetVolume(f32),
    /// Quit the player thread.
    Shutdown,
}

/// Playback state shared between UI and audio threads.
#[derive(Debug, Clone)]
pub struct SharedState {
    pub is_playing: Arc<AtomicBool>,
    pub position_frames: Arc<AtomicUsize>,
    pub total_frames: Arc<AtomicUsize>,
    pub volume: Arc<AtomicU32>, // 0..1000 (milli-scale for finer control)
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            is_playing: Arc::new(AtomicBool::new(false)),
            position_frames: Arc::new(AtomicUsize::new(0)),
            total_frames: Arc::new(AtomicUsize::new(0)),
            volume: Arc::new(AtomicU32::new(800)), // 0.8 * 1000
        }
    }

    /// Get current volume as f32 0.0 – 1.0.
    #[allow(dead_code)]
    pub fn volume_f32(&self) -> f32 {
        self.volume.load(Ordering::Relaxed) as f32 / 1000.0
    }
}

/// The audio player engine.
/// Runs decoding in a separate thread and outputs via CPAL.
pub struct Player {
    command_tx: Sender<PlayerCommand>,
    pub state: SharedState,
}

impl Player {
    pub fn new() -> Self {
        let (tx, rx) = crossbeam_channel::bounded::<PlayerCommand>(16);
        let state = SharedState::new();
        let state_clone = state.clone();

        std::thread::spawn(move || {
            Self::run_loop(rx, state_clone);
        });

        Self {
            command_tx: tx,
            state,
        }
    }

    pub fn send(&self, cmd: PlayerCommand) -> Result<()> {
        self.command_tx.send(cmd)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn shutdown(&self) -> Result<()> {
        self.command_tx.send(PlayerCommand::Shutdown)?;
        Ok(())
    }

    fn run_loop(rx: Receiver<PlayerCommand>, state: SharedState) {
        // Shared PCM buffer the CPAL callback reads from.
        let shared_audio: Arc<std::sync::RwLock<AudioStreamState>> =
            Arc::new(std::sync::RwLock::new(AudioStreamState {
                samples: Vec::new(),
                source_channels: 2,
                source_rate: 44100,
            }));

        // Fractional read cursor in *source* frames, shared lock-free with the
        // CPAL callback (f64 bits in an AtomicU64). It must be fractional because
        // the source and device rates usually differ (44.1kHz file -> 48kHz device).
        let cursor = Arc::new(AtomicU64::new(0));

        let Some(open) = Self::open_stream(&shared_audio, &cursor, &state) else {
            eprintln!("caw: no usable audio output device — playback disabled");
            Self::drain_commands(rx);
            return;
        };
        let OpenStream {
            stream,
            sample_rate: out_rate,
            channels: out_channels,
            device_name,
        } = open;

        stream.play().ok();
        eprintln!(
            "caw: audio output stream started on '{}' ({}Hz, {}ch)",
            device_name, out_rate, out_channels
        );

        // Command processing loop
        for cmd in rx {
            match cmd {
                PlayerCommand::Play(audio, seek_pos) => {
                    let src_rate = audio.sample_rate;
                    let src_ch = audio.channels.max(1);
                    let total_frames = audio.samples.len() / src_ch as usize;

                    state.total_frames.store(total_frames, Ordering::Relaxed);

                    let start_frame = (seek_pos.max(0.0) * src_rate as f64)
                        .min(total_frames as f64)
                        .floor();

                    {
                        let mut guard = match shared_audio.write() {
                            Ok(g) => g,
                            Err(poisoned) => poisoned.into_inner(),
                        };
                        guard.samples = audio.samples;
                        guard.source_channels = src_ch;
                        guard.source_rate = src_rate;
                    }

                    cursor.store(start_frame.to_bits(), Ordering::Relaxed);
                    state
                        .position_frames
                        .store(start_frame as usize, Ordering::Relaxed);
                    state.is_playing.store(true, Ordering::Relaxed);
                }
                PlayerCommand::Pause => {
                    state.is_playing.store(false, Ordering::Relaxed);
                }
                PlayerCommand::Resume => {
                    state.is_playing.store(true, Ordering::Relaxed);
                }
                PlayerCommand::Stop => {
                    state.is_playing.store(false, Ordering::Relaxed);
                    state.position_frames.store(0, Ordering::Relaxed);
                    state.total_frames.store(0, Ordering::Relaxed);
                    cursor.store(0f64.to_bits(), Ordering::Relaxed);
                    if let Ok(mut guard) = shared_audio.write() {
                        guard.samples.clear();
                    }
                }
                PlayerCommand::Seek(pos) => {
                    if let Ok(guard) = shared_audio.read() {
                        let src_ch = guard.source_channels.max(1) as usize;
                        let src_rate = guard.source_rate;
                        let total_frames = guard.samples.len() / src_ch;
                        let frame = (pos.max(0.0) * src_rate as f64)
                            .min(total_frames as f64)
                            .floor();
                        drop(guard);
                        cursor.store(frame.to_bits(), Ordering::Relaxed);
                        state
                            .position_frames
                            .store(frame as usize, Ordering::Relaxed);
                    }
                }
                PlayerCommand::SetVolume(vol) => {
                    let v = (vol.clamp(0.0, 1.0) * 1000.0) as u32;
                    state.volume.store(v, Ordering::Relaxed);
                }
                PlayerCommand::Shutdown => {
                    drop(stream);
                    return;
                }
            }
        }
    }

    /// Try to open an output stream, falling back across devices.
    ///
    /// The default ALSA PCM is frequently unusable on Linux: without the
    /// `pipewire-alsa` / `libasound2-plugins` bridge, `default` resolves to a
    /// `dmix` plugin on whichever card is first (often an unplugged HDMI output),
    /// and `snd_pcm_dmix_open` fails with "unable to open slave". Previously that
    /// killed the audio thread outright and every later command was silently
    /// dropped, so the UI showed "playing" with no sound. Now we walk the
    /// enumerated devices until one actually opens.
    /// ALSA card id (e.g. "TypeC") backing the sound server's default sink.
    ///
    /// Only used to *rank* fallback candidates. ALSA on its own has no idea
    /// which sink the user picked, so without this the fallback can happily
    /// open some unrelated HDMI or onboard card and appear silent. Returns
    /// `None` when pactl is unavailable — ranking then falls back to names.
    fn default_sink_card() -> Option<String> {
        use std::process::Command;

        let out = Command::new("pactl")
            .arg("get-default-sink")
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let sink = String::from_utf8(out.stdout).ok()?.trim().to_string();
        if sink.is_empty() {
            return None;
        }

        // Find that sink in `pactl list sinks` and read its `alsa.id` property,
        // which is exactly the CARD= token cpal reports in its device ids.
        let list = Command::new("pactl")
            .args(["list", "sinks"])
            .output()
            .ok()?;
        if !list.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&list.stdout);

        let mut in_target = false;
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(name) = trimmed.strip_prefix("Name: ") {
                in_target = name.trim() == sink;
                continue;
            }
            if in_target && let Some(v) = trimmed.strip_prefix("alsa.id = ") {
                return Some(v.trim().trim_matches('"').to_string());
            }
        }
        None
    }

    fn open_stream(
        shared_audio: &Arc<std::sync::RwLock<AudioStreamState>>,
        cursor: &Arc<AtomicU64>,
        state: &SharedState,
    ) -> Option<OpenStream> {
        let host = cpal::default_host();

        let mut candidates: Vec<cpal::Device> = Vec::new();
        if let Some(dev) = host.default_output_device() {
            candidates.push(dev);
        }
        match host.output_devices() {
            Ok(devs) => candidates.extend(devs),
            Err(e) => eprintln!("caw: failed to enumerate output devices: {}", e),
        }

        // The compositor/sound-server default sink is the device the user
        // actually chose; prefer whichever ALSA card backs it.
        let preferred_card = Self::default_sink_card();
        if let Some(ref card) = preferred_card {
            eprintln!(
                "caw: preferred output card from default sink: CARD={}",
                card
            );
        }

        // Rank by device id, which on ALSA carries the PCM name ("plughw:CARD=..").
        // "null" discards audio silently, so it must never be chosen.
        let mut ranked: Vec<(i32, String, cpal::Device)> = candidates
            .into_iter()
            .filter_map(|dev| {
                let id = dev
                    .id()
                    .map(|i| i.to_string())
                    .unwrap_or_else(|_| "<unknown>".into());
                // Sound-server routes always rank first: they hand audio to
                // PipeWire/Pulse, which owns the hardware and shares it, so
                // the device stays visible to every other app. Direct-hardware
                // PCMs (sysdefault:/plughw:/front:/hw:/dmix:) open the ALSA
                // device themselves and hold it exclusively — the sound server
                // then fails with "device busy" and the device disappears from
                // the system mixer (seen with sysdefault:CARD=TypeC grabbing
                // /dev/snd/pcmC3D0p). They remain last-resort fallbacks for
                // systems without the pipewire-alsa/pulse bridge, ranked among
                // themselves only.
                let mut score = match id.as_str() {
                    n if n.ends_with(":null") || n.contains(":null:") => return None,
                    n if n.contains("pipewire") => 0,
                    n if n.contains("pulse") => 1,
                    n if n.ends_with(":default") => 2,
                    // Hardware tier: values must stay above the bridge tier
                    // (with the -20 bonus below, min 80) so nothing here can
                    // ever outrank the sound-server routes.
                    n if n.contains("sysdefault") => 100,
                    n if n.contains("plughw:") => 101,
                    n if n.contains("front:") => 102,
                    n if n.contains("hw:") => 103,
                    n if n.contains("dmix") => 109,
                    _ => 107,
                };
                // Prefer the card backing the default sink, but only to order
                // the hardware fallback tier — the sound-server routes above
                // already follow the user's chosen sink by construction.
                if let Some(card) = &preferred_card
                    && score >= 100
                    && id.contains(&format!("CARD={}", card))
                {
                    score -= 20;
                }
                Some((score, id, dev))
            })
            .collect();
        ranked.sort_by_key(|(score, _, _)| *score);

        let mut seen: Vec<String> = Vec::new();
        for (_, name, device) in ranked {
            if seen.contains(&name) {
                continue;
            }
            seen.push(name.clone());

            let config = match device.default_output_config() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("caw: device '{}' has no usable config: {}", name, e);
                    continue;
                }
            };

            let out_rate = config.sample_rate();
            let out_channels = config.channels();
            let shared_cb = shared_audio.clone();
            let cursor_cb = cursor.clone();
            let state_cb = state.clone();

            let built = device.build_output_stream(
                config.config(),
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    Self::fill(
                        data,
                        out_channels,
                        out_rate,
                        &shared_cb,
                        &cursor_cb,
                        &state_cb,
                    );
                },
                |err| eprintln!("caw: audio output error: {}", err),
                None,
            );

            match built {
                Ok(stream) => {
                    return Some(OpenStream {
                        stream,
                        sample_rate: out_rate,
                        channels: out_channels,
                        device_name: name,
                    });
                }
                Err(e) => eprintln!("caw: cannot open output on '{}': {}", name, e),
            }
        }
        None
    }

    /// CPAL callback body: resample + map channels from the source buffer.
    ///
    /// Real-time safe: no allocation, no blocking lock (`try_read` only — the
    /// writer holds the lock just long enough to swap buffers on Play/Stop).
    fn fill(
        data: &mut [f32],
        out_channels: u16,
        out_rate: u32,
        shared: &Arc<std::sync::RwLock<AudioStreamState>>,
        cursor: &Arc<AtomicU64>,
        state: &SharedState,
    ) {
        let silence = |d: &mut [f32]| d.iter_mut().for_each(|s| *s = 0.0);

        if !state.is_playing.load(Ordering::Relaxed) {
            silence(data);
            return;
        }

        let Ok(guard) = shared.try_read() else {
            // Buffer is being swapped; emit a frame of silence rather than block.
            silence(data);
            return;
        };

        let src_ch = guard.source_channels.max(1) as usize;
        let out_ch = out_channels.max(1) as usize;
        let total_src_frames = guard.samples.len() / src_ch;
        if total_src_frames == 0 {
            silence(data);
            return;
        }

        // Resample ratio: how many source frames advance per output frame.
        let step = guard.source_rate as f64 / out_rate as f64;
        let vol = state.volume.load(Ordering::Relaxed) as f32 / 1000.0;
        let mut pos = f64::from_bits(cursor.load(Ordering::Relaxed));

        let mut finished = false;
        for frame in data.chunks_mut(out_ch) {
            let i = pos.floor() as usize;
            if i >= total_src_frames {
                frame.iter_mut().for_each(|s| *s = 0.0);
                finished = true;
                continue;
            }

            // Linear interpolation between source frame i and i+1.
            let frac = (pos - i as f64) as f32;
            let base = i * src_ch;
            let next = if i + 1 < total_src_frames {
                base + src_ch
            } else {
                base
            };

            for (ch, out) in frame.iter_mut().enumerate() {
                // Map output channel -> source channel. Mono sources feed every
                // output channel; wider sources wrap (stereo -> 5.1 etc.).
                let sc = if src_ch == 1 { 0 } else { ch % src_ch };
                let a = guard.samples[base + sc];
                let b = guard.samples[next + sc];
                *out = (a + (b - a) * frac) * vol;
            }
            pos += step;
        }

        cursor.store(pos.to_bits(), Ordering::Relaxed);
        state.position_frames.store(
            (pos.floor() as usize).min(total_src_frames),
            Ordering::Relaxed,
        );
        if finished {
            state.is_playing.store(false, Ordering::Relaxed);
        }
    }

    fn drain_commands(rx: Receiver<PlayerCommand>) {
        for cmd in rx {
            if matches!(cmd, PlayerCommand::Shutdown) {
                return;
            }
        }
    }
}

/// A successfully opened output stream and its negotiated format.
struct OpenStream {
    stream: cpal::Stream,
    sample_rate: u32,
    channels: u16,
    device_name: String,
}

/// Shared audio buffer state for the CPAL callback.
struct AudioStreamState {
    samples: Vec<f32>,
    source_channels: u16,
    source_rate: u32,
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.send(PlayerCommand::Shutdown);
    }
}
