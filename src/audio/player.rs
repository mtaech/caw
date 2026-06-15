use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

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
        let host = cpal::default_host();

        let Some(device) = host.default_output_device() else {
            eprintln!("caw: no audio output device available");
            Self::drain_commands(rx);
            return;
        };

        let Ok(stream_config) = device.default_output_config() else {
            eprintln!("caw: failed to get default output config");
            Self::drain_commands(rx);
            return;
        };

        let sample_rate = stream_config.sample_rate();
        let channels: u16 = stream_config.channels();

        // Shared audio buffer for the CPAL callback to read from.
        let shared_audio: Arc<std::sync::RwLock<AudioStreamState>> =
            Arc::new(std::sync::RwLock::new(AudioStreamState {
                samples: Vec::new(),
                position: 0,
                source_channels: 2,
                source_rate: 44100,
            }));

        let shared_audio_cb = shared_audio.clone();
        let state_cb = state.clone();

        let Ok(stream) = device.build_output_stream(
            stream_config.config(),
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                let is_playing = state_cb.is_playing.load(Ordering::Relaxed);
                if !is_playing {
                    for s in data.iter_mut() {
                        *s = 0.0;
                    }
                    return;
                }

                let vol = state_cb.volume.load(Ordering::Relaxed) as f32 / 1000.0;
                let guard = match shared_audio_cb.read() {
                    Ok(g) => g,
                    Err(_) => return,
                };

                let src_channels = guard.source_channels as usize;
                let total_src_frames = guard.samples.len() / src_channels;
                let src_pos = guard.position;

                // Advance position
                let out_channels = channels as usize;
                let out_frames = data.len() / out_channels;
                let frames_to_copy = out_frames.min(total_src_frames.saturating_sub(src_pos));

                let mut src_idx = src_pos * src_channels;
                let mut out_idx = 0;

                for _ in 0..frames_to_copy {
                    let mut sample: f32 = 0.0;
                    for ch in 0..src_channels.min(guard.samples.len().saturating_sub(src_idx)) {
                        sample += guard.samples[src_idx + ch];
                    }
                    sample = sample / src_channels.max(1) as f32 * vol;

                    for ch in 0..out_channels.min(data.len().saturating_sub(out_idx)) {
                        data[out_idx + ch] = sample;
                    }
                    out_idx += out_channels;
                    src_idx += src_channels;
                }

                // Fill remaining with silence
                for s in data.iter_mut().skip(out_idx) {
                    *s = 0.0;
                }

                // Advance playback position
                let new_pos = src_pos + frames_to_copy;
                state_cb.position_frames.store(new_pos, Ordering::Relaxed);
                // Also update RwLock position so next callback starts from here
                drop(guard);
                if let Ok(mut wguard) = shared_audio_cb.write() {
                    wguard.position = new_pos;
                }
                // If track finished, stop playing
                if total_src_frames > 0 && new_pos >= total_src_frames {
                    state_cb.is_playing.store(false, Ordering::Relaxed);
                }
            },
            |err| eprintln!("caw: audio output error: {}", err),
            None,
        ) else {
            eprintln!("caw: failed to build output stream");
            Self::drain_commands(rx);
            return;
        };

        stream.play().ok();
        eprintln!(
            "caw: audio output stream started ({}Hz, {}ch)",
            sample_rate, channels
        );

        // Command processing loop
        for cmd in rx {
            match cmd {
                PlayerCommand::Play(audio, seek_pos) => {
                    let src_rate = audio.sample_rate;
                    let src_ch = audio.channels;
                    let total_frames = audio.samples.len() / src_ch as usize;

                    state.total_frames.store(total_frames, Ordering::Relaxed);

                    let seek_frames = if seek_pos > 0.0 {
                        (seek_pos * src_rate as f64 * src_ch as f64) as usize
                    } else {
                        0
                    };
                    let clamped_seek = seek_frames.min(audio.samples.len());

                    {
                        let mut guard = shared_audio.write().unwrap();
                        guard.samples = audio.samples;
                        guard.position = clamped_seek / src_ch as usize;
                        guard.source_channels = src_ch;
                        guard.source_rate = src_rate;
                    }

                    state
                        .position_frames
                        .store(clamped_seek / src_ch as usize, Ordering::Relaxed);
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
                    let mut guard = shared_audio.write().unwrap();
                    guard.samples.clear();
                    guard.position = 0;
                }
                PlayerCommand::Seek(pos) => {
                    if let Ok(guard) = shared_audio.read() {
                        let src_ch = guard.source_channels as usize;
                        let src_rate = guard.source_rate;
                        let seek_frames = (pos * src_rate as f64) as usize * src_ch;
                        let clamped = seek_frames.min(guard.samples.len());
                        let frame_pos = clamped / src_ch;
                        state.position_frames.store(frame_pos, Ordering::Relaxed);
                        drop(guard);
                        let mut guard = shared_audio.write().unwrap();
                        guard.position = frame_pos;
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

    fn drain_commands(rx: Receiver<PlayerCommand>) {
        for cmd in rx {
            if matches!(cmd, PlayerCommand::Shutdown) {
                return;
            }
        }
    }
}

/// Shared audio buffer state for the CPAL callback.
struct AudioStreamState {
    samples: Vec<f32>,
    position: usize, // in frames (not samples)
    source_channels: u16,
    source_rate: u32,
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.send(PlayerCommand::Shutdown);
    }
}
