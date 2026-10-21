//! MPRIS2 D-Bus service (Linux).
//!
//! Exposes two interfaces on /org/mpris/MediaPlayer2:
//!
//! * org.mpris.MediaPlayer2 -- identity, Raise, Quit
//! * org.mpris.MediaPlayer2.Player -- transport control, metadata, position, volume
//!
//! Runs on its own thread using zbus's blocking API, so no async runtime is needed in
//! the app. Inbound D-Bus calls are turned into PlatformCommand values (channel to the
//! app timer); outbound state comes from the latest NowPlaying snapshot published by
//! that timer.
//!
//! Degradation: if there is no session bus (headless/SSH), the thread logs a warning
//! and exits; the player keeps working and nothing panics.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use zbus::blocking::Connection;
use zbus::zvariant::{Array, ObjectPath, OwnedValue, Str, Value};

use super::{NowPlaying, PlatformCommand, Shared};

const ROOT_PATH: &str = "/org/mpris/MediaPlayer2";
const SERVICE_NAME: &str = "org.mpris.MediaPlayer2.caw";
const PLAYER_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";
const PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";

/// Spawn the MPRIS service thread. Any failure is logged, never propagated.
pub(crate) fn spawn(shared: Arc<Shared>) {
    let spawned = std::thread::Builder::new()
        .name("caw-mpris".to_owned())
        .spawn(move || match run(shared) {
            Ok(()) => crate::log_info!("platform: MPRIS service stopped"),
            Err(err) => crate::log_warn!(
                "platform: MPRIS unavailable ({err}); media keys and desktop media widgets are disabled"
            ),
        });

    if let Err(err) = spawned {
        crate::log_warn!("platform: could not spawn MPRIS thread: {err}");
    }
}

fn run(shared: Arc<Shared>) -> zbus::Result<()> {
    let connection = Connection::session()?;
    connection.object_server().at(
        ROOT_PATH,
        MprisRoot {
            shared: Arc::clone(&shared),
        },
    )?;
    connection.object_server().at(
        ROOT_PATH,
        MprisPlayer {
            shared: Arc::clone(&shared),
        },
    )?;
    connection.request_name(SERVICE_NAME)?;
    crate::log_info!("platform: MPRIS registered as {SERVICE_NAME}");

    // Poll the shared snapshot; publish changed properties. The condvar wakes us as
    // soon as the app timer publishes (~250 ms), the timeout is only a safety net.
    let mut emitted = Emitted::default();
    loop {
        let snapshot = shared.wait_for_change(Duration::from_secs(2));
        let changed = emitted.diff(&snapshot);
        if changed.is_empty() {
            continue;
        }
        let body = (PLAYER_INTERFACE, changed, Vec::<String>::new());
        connection.emit_signal(
            None::<&str>,
            ROOT_PATH,
            PROPERTIES_INTERFACE,
            "PropertiesChanged",
            &body,
        )?;
    }
}

// -- Property diffing ------------------------------------------------------------

/// Last values we announced through PropertiesChanged, used to avoid spamming clients.
#[derive(Default)]
struct Emitted {
    playback_status: &'static str,
    loop_status: &'static str,
    shuffle: bool,
    volume: f32,
    metadata_key: String,
    can_go_next: bool,
    can_go_previous: bool,
    can_play: bool,
    can_pause: bool,
    can_seek: bool,
}

impl Emitted {
    fn diff(&mut self, snapshot: &NowPlaying) -> HashMap<String, OwnedValue> {
        let mut changed = HashMap::new();

        let playback_status = snapshot.mpris_playback_status();
        if self.playback_status != playback_status {
            self.playback_status = playback_status;
            changed.insert("PlaybackStatus".to_owned(), string_value(playback_status));
        }

        let loop_status = snapshot.mpris_loop_status();
        if self.loop_status != loop_status {
            self.loop_status = loop_status;
            changed.insert("LoopStatus".to_owned(), string_value(loop_status));
        }

        let shuffle = snapshot.mpris_shuffle();
        if self.shuffle != shuffle {
            self.shuffle = shuffle;
            changed.insert("Shuffle".to_owned(), OwnedValue::from(shuffle));
        }

        if self.volume != snapshot.volume {
            self.volume = snapshot.volume;
            changed.insert(
                "Volume".to_owned(),
                OwnedValue::from(snapshot.volume as f64),
            );
        }

        let metadata_key = metadata_key(snapshot);
        if self.metadata_key != metadata_key {
            self.metadata_key = metadata_key;
            changed.insert(
                "Metadata".to_owned(),
                OwnedValue::from(metadata_map(snapshot)),
            );
        }

        let can_go_next = snapshot.can_go_next && snapshot.has_track();
        if self.can_go_next != can_go_next {
            self.can_go_next = can_go_next;
            changed.insert("CanGoNext".to_owned(), OwnedValue::from(can_go_next));
        }
        let can_go_previous = snapshot.can_go_previous && snapshot.has_track();
        if self.can_go_previous != can_go_previous {
            self.can_go_previous = can_go_previous;
            changed.insert(
                "CanGoPrevious".to_owned(),
                OwnedValue::from(can_go_previous),
            );
        }
        let can_play = true;
        if self.can_play != can_play {
            self.can_play = can_play;
            changed.insert("CanPlay".to_owned(), OwnedValue::from(can_play));
        }
        let can_pause = snapshot.has_track();
        if self.can_pause != can_pause {
            self.can_pause = can_pause;
            changed.insert("CanPause".to_owned(), OwnedValue::from(can_pause));
        }
        let can_seek = snapshot.has_track();
        if self.can_seek != can_seek {
            self.can_seek = can_seek;
            changed.insert("CanSeek".to_owned(), OwnedValue::from(can_seek));
        }

        changed
    }
}

/// Values that, when changed, require a new Metadata announcement.
fn metadata_key(snapshot: &NowPlaying) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        snapshot.track_id.unwrap_or(0),
        snapshot.title,
        snapshot.artist,
        snapshot.album,
        snapshot.track_number,
        snapshot.duration_secs,
        snapshot.file_url.as_deref().unwrap_or(""),
        snapshot.art_url.as_deref().unwrap_or(""),
    )
}

/// Build the MPRIS Metadata dictionary (a{sv}).
fn metadata_map(snapshot: &NowPlaying) -> HashMap<String, OwnedValue> {
    let mut metadata = HashMap::new();
    let Some(track_id) = snapshot.track_id else {
        return metadata;
    };

    let track_path = format!("{ROOT_PATH}/Track/{track_id}");
    if let Ok(path) = ObjectPath::try_from(track_path) {
        metadata.insert("mpris:trackid".to_owned(), OwnedValue::from(path));
    }

    let length_micros = (snapshot.duration_secs.max(0.0) * 1_000_000.0).round() as i64;
    metadata.insert("mpris:length".to_owned(), OwnedValue::from(length_micros));
    metadata.insert("xesam:title".to_owned(), string_value(&snapshot.title));
    metadata.insert("xesam:album".to_owned(), string_value(&snapshot.album));
    metadata.insert(
        "xesam:trackNumber".to_owned(),
        OwnedValue::from(snapshot.track_number as i32),
    );

    if !snapshot.artist.is_empty() {
        let artists = Array::from(vec![Value::from(snapshot.artist.as_str())]);
        let artists = OwnedValue::try_from(Value::Array(artists))
            .unwrap_or_else(|_| string_value(&snapshot.artist));
        metadata.insert("xesam:artist".to_owned(), artists);
    }
    if let Some(url) = snapshot.file_url.as_deref() {
        metadata.insert("xesam:url".to_owned(), string_value(url));
    }
    if let Some(art_url) = snapshot.art_url.as_deref() {
        metadata.insert("mpris:artUrl".to_owned(), string_value(art_url));
    }

    metadata
}

fn string_value(text: &str) -> OwnedValue {
    OwnedValue::from(Str::from(text))
}

// -- org.mpris.MediaPlayer2 ------------------------------------------------------

struct MprisRoot {
    shared: Arc<Shared>,
}

#[zbus::interface(name = "org.mpris.MediaPlayer2")]
impl MprisRoot {
    #[zbus(property(emits_changed_signal = "const"))]
    fn can_raise(&self) -> bool {
        true
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn can_quit(&self) -> bool {
        true
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn identity(&self) -> String {
        "Caw".to_owned()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn desktop_entry(&self) -> String {
        "caw".to_owned()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn supported_uri_schemes(&self) -> Vec<String> {
        Vec::new()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn supported_mime_types(&self) -> Vec<String> {
        vec![
            "audio/flac".to_owned(),
            "audio/mpeg".to_owned(),
            "audio/wav".to_owned(),
            "audio/ogg".to_owned(),
            "audio/aac".to_owned(),
            "audio/mp4".to_owned(),
            "audio/x-flac".to_owned(),
            "audio/x-mpeg".to_owned(),
        ]
    }

    /// Bring the player window to the foreground.
    fn raise(&self) {
        self.shared.send(PlatformCommand::ShowWindow);
    }

    /// Ask the application to quit.
    fn quit(&self) {
        self.shared.send(PlatformCommand::Quit);
    }
}

// -- org.mpris.MediaPlayer2.Player ----------------------------------------------

struct MprisPlayer {
    shared: Arc<Shared>,
}

#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    // ---- Properties (read-only getters) ----

    #[zbus(property(emits_changed_signal = "false"))]
    fn playback_status(&self) -> String {
        self.shared.read().mpris_playback_status().to_owned()
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn loop_status(&self) -> String {
        self.shared.read().mpris_loop_status().to_owned()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn shuffle(&self) -> bool {
        self.shared.read().mpris_shuffle()
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        metadata_map(&self.shared.read())
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn volume(&self) -> f64 {
        self.shared.read().volume as f64
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn position(&self) -> i64 {
        // MPRIS wants microseconds. The snapshot is refreshed ~4x per second by the
        // app timer, which is fine for media widgets (they interpolate).
        let snapshot = self.shared.read();
        (snapshot.position_secs.max(0.0) * 1_000_000.0).round() as i64
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn minimum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn maximum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn can_go_next(&self) -> bool {
        let snapshot = self.shared.read();
        snapshot.can_go_next && snapshot.has_track()
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn can_go_previous(&self) -> bool {
        let snapshot = self.shared.read();
        snapshot.can_go_previous && snapshot.has_track()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn can_play(&self) -> bool {
        true
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn can_pause(&self) -> bool {
        self.shared.read().has_track()
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn can_seek(&self) -> bool {
        self.shared.read().has_track()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn can_control(&self) -> bool {
        true
    }

    // ---- Properties (writable) ----

    /// MPRIS Volume is 0.0 - 1.0. Commands are applied by the app timer.
    #[zbus(property)]
    fn set_volume(&self, volume: f64) -> zbus::fdo::Result<()> {
        if !volume.is_finite() {
            return Err(zbus::fdo::Error::InvalidArgs(
                "volume must be a finite number".to_owned(),
            ));
        }
        self.shared
            .send(PlatformCommand::SetVolume(volume.clamp(0.0, 1.0) as f32));
        Ok(())
    }

    // ---- Transport methods ----

    fn next(&self) {
        self.shared.send(PlatformCommand::Next);
    }

    fn previous(&self) {
        self.shared.send(PlatformCommand::Previous);
    }

    fn pause(&self) {
        self.shared.send(PlatformCommand::Pause);
    }

    fn play_pause(&self) {
        self.shared.send(PlatformCommand::TogglePlayPause);
    }

    /// Caw has no separate stopped transport state; Stop pauses playback.
    fn stop(&self) {
        self.shared.send(PlatformCommand::Pause);
    }

    fn play(&self) {
        self.shared.send(PlatformCommand::Play);
    }

    /// Seek by a relative offset in microseconds (MPRIS sign convention).
    fn seek(&self, offset: i64) {
        self.shared
            .send(PlatformCommand::SeekBy(offset as f64 / 1_000_000.0));
    }

    /// Seek to an absolute position in microseconds.
    fn set_position(&self, _track_id: ObjectPath<'_>, position: i64) {
        self.shared
            .send(PlatformCommand::SeekTo(position as f64 / 1_000_000.0));
    }

    fn open_uri(&self, _uri: &str) -> zbus::fdo::Result<()> {
        Err(zbus::fdo::Error::NotSupported(
            "OpenUri is not supported by Caw".to_owned(),
        ))
    }
}
