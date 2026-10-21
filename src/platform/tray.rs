//! System tray icon (KDE/freedesktop StatusNotifierItem + com.canonical.dbusmenu).
//!
//! Implemented directly on zbus 5 because `ksni 0.3.7` does not compile against
//! zbus 5.19 (its macro-generated `DBUS_CONTEXT` was removed upstream). The item
//! publishes a music-player menu (now playing header, transport, volume and mode
//! submenus) and maps clicks onto `PlatformCommand`s, so the tray never touches
//! gpui entities.
//!
//! Degradation: without a StatusNotifierWatcher (no tray host) the thread logs a
//! warning and exits; the app keeps running.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Str, StructureBuilder, Value};

use crate::models::playlist::{PlaybackMode, PlaybackState};

use super::{NowPlaying, PlatformCommand, Shared};

const ITEM_PATH: &str = "/StatusNotifierItem";
const MENU_PATH: &str = "/MenuBar";
const KDE_WATCHER: &str = "org.kde.StatusNotifierWatcher";
const FDO_WATCHER: &str = "org.freedesktop.StatusNotifierWatcher";
const ICON_NAME: &str = "audio-x-generic";

// Menu item ids.
const ITEM_NOW_PLAYING: i32 = 1;
const ITEM_SEP_TOP: i32 = 2;
const ITEM_PLAY_PAUSE: i32 = 3;
const ITEM_PREVIOUS: i32 = 4;
const ITEM_NEXT: i32 = 5;
const ITEM_SEP_MID: i32 = 6;
const ITEM_VOLUME: i32 = 7;
const ITEM_MODE: i32 = 8;
const ITEM_SEP_BOTTOM: i32 = 9;
const ITEM_SHOW: i32 = 10;
const ITEM_QUIT: i32 = 11;
const VOLUME_BASE: i32 = 100;
const MODE_BASE: i32 = 200;

const VOLUME_LEVELS: [i32; 5] = [0, 25, 50, 75, 100];
const MODES: [PlaybackMode; 4] = [
    PlaybackMode::Sequence,
    PlaybackMode::Shuffle,
    PlaybackMode::RepeatList,
    PlaybackMode::RepeatOne,
];

type IconPixmap = Vec<(i32, i32, Vec<u8>)>;
type ToolTip = (String, IconPixmap, String, String);
type MenuProps = HashMap<String, OwnedValue>;
type MenuChildren = Vec<Value<'static>>;
type MenuLayoutNode = (i32, MenuProps, MenuChildren);
type MenuLayout = (u32, MenuLayoutNode);

/// Spawn the tray thread. Any failure is logged, never propagated.
pub(crate) fn spawn(shared: Arc<Shared>) {
    let spawned = std::thread::Builder::new()
        .name("caw-tray".to_owned())
        .spawn(move || match run(shared) {
            Ok(()) => crate::log_info!("platform: tray service stopped"),
            Err(err) => crate::log_warn!(
                "platform: tray unavailable ({err}); close-to-tray will only minimize"
            ),
        });

    if let Err(err) = spawned {
        crate::log_warn!("platform: could not spawn tray thread: {err}");
    }
}

fn run(shared: Arc<Shared>) -> zbus::Result<()> {
    let connection = Connection::session()?;
    connection.object_server().at(
        ITEM_PATH,
        StatusNotifierItem {
            shared: Arc::clone(&shared),
        },
    )?;
    connection.object_server().at(
        MENU_PATH,
        DbusMenu {
            shared: Arc::clone(&shared),
        },
    )?;

    // SNI items are addressed through their own well-known bus name.
    let item_service = format!("org.kde.StatusNotifierItem-{}-1", std::process::id());
    connection.request_name(item_service.as_str())?;

    if let Err(err) = register(&connection, KDE_WATCHER, &item_service) {
        crate::log_debug!(
            "platform: KDE tray watcher unavailable ({err}); trying freedesktop watcher"
        );
        register(&connection, FDO_WATCHER, &item_service)?;
    }
    crate::log_info!("platform: tray registered as {item_service}");

    // Incoming D-Bus calls are served by zbus's internal executor; keep the thread
    // (and therefore the connection) alive.
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

fn register(connection: &Connection, watcher: &str, item_service: &str) -> zbus::Result<()> {
    connection.call_method(
        Some(watcher),
        "/StatusNotifierWatcher",
        Some(watcher),
        "RegisterStatusNotifierItem",
        &(item_service,),
    )?;
    Ok(())
}

// -- Menu layout ----------------------------------------------------------------

fn text(label: &str, enabled: bool, icon: &str) -> MenuProps {
    let mut props = HashMap::new();
    props.insert("label".to_owned(), string_value(label));
    props.insert("enabled".to_owned(), OwnedValue::from(enabled));
    props.insert("visible".to_owned(), OwnedValue::from(true));
    props.insert("icon-name".to_owned(), string_value(icon));
    props
}

fn radio(label: &str, icon: &str, selected: bool) -> MenuProps {
    let mut props = text(label, true, icon);
    props.insert("toggle-type".to_owned(), string_value("radio"));
    props.insert(
        "toggle-state".to_owned(),
        OwnedValue::from(if selected { 1i32 } else { 0i32 }),
    );
    props
}

fn separator() -> MenuProps {
    let mut props = HashMap::new();
    props.insert("type".to_owned(), string_value("separator"));
    props.insert("visible".to_owned(), OwnedValue::from(true));
    props
}

fn submenu(label: &str, enabled: bool, icon: &str) -> MenuProps {
    let mut props = text(label, enabled, icon);
    props.insert("children-display".to_owned(), string_value("submenu"));
    props
}

fn layout_node(id: i32, props: MenuProps, children: MenuChildren) -> Value<'static> {
    let structure = StructureBuilder::new()
        .append_field(Value::from(id))
        .append_field(Value::from(props))
        .append_field(Value::from(children))
        .build()
        .expect("menu item structure is never empty");
    Value::Structure(structure)
}

fn fmt_time(seconds: f64) -> String {
    let seconds = seconds.max(0.0).round() as u64;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

fn now_playing_label(snapshot: &NowPlaying) -> String {
    if !snapshot.has_track() {
        return "未在播放".to_owned();
    }
    let mut label = snapshot.title.clone();
    if !snapshot.artist.is_empty() {
        label.push_str(" — ");
        label.push_str(&snapshot.artist);
    }
    format!(
        "{}   {} / {}",
        label,
        fmt_time(snapshot.position_secs),
        fmt_time(snapshot.duration_secs)
    )
}

fn transport_icon(snapshot: &NowPlaying) -> &'static str {
    match snapshot.playback_state {
        PlaybackState::Playing => "media-playback-pause-symbolic",
        PlaybackState::Paused => "media-playback-start-symbolic",
        PlaybackState::Stopped => "media-playback-stop-symbolic",
    }
}

fn volume_icon(volume: f32) -> &'static str {
    if volume <= 0.001 {
        "audio-volume-muted-symbolic"
    } else if volume < 0.34 {
        "audio-volume-low-symbolic"
    } else if volume < 0.67 {
        "audio-volume-medium-symbolic"
    } else {
        "audio-volume-high-symbolic"
    }
}

fn volume_level_index(volume: f32) -> usize {
    let percent = (volume.clamp(0.0, 1.0) * 100.0).round() as i32;
    VOLUME_LEVELS
        .iter()
        .enumerate()
        .min_by_key(|(_, level)| (percent - **level).abs())
        .map(|(index, _)| index)
        .unwrap_or(VOLUME_LEVELS.len() - 1)
}

fn mode_index(mode: PlaybackMode) -> usize {
    MODES
        .iter()
        .position(|candidate| *candidate == mode)
        .unwrap_or(0)
}

fn mode_icon(mode: PlaybackMode) -> &'static str {
    match mode {
        PlaybackMode::Sequence => "view-media-playlist-symbolic",
        PlaybackMode::Shuffle => "media-playlist-shuffle-symbolic",
        PlaybackMode::RepeatList => "media-playlist-repeat-symbolic",
        PlaybackMode::RepeatOne => "media-playlist-repeat-song-symbolic",
    }
}

fn item_props(id: i32, snapshot: &NowPlaying) -> Option<MenuProps> {
    Some(match id {
        ITEM_NOW_PLAYING => {
            let mut props = text(
                &now_playing_label(snapshot),
                false,
                transport_icon(snapshot),
            );
            props.insert("visible".to_owned(), OwnedValue::from(true));
            props
        }
        ITEM_SEP_TOP | ITEM_SEP_MID | ITEM_SEP_BOTTOM => separator(),
        ITEM_PLAY_PAUSE => text(
            if snapshot.playback_state == PlaybackState::Playing {
                "暂停"
            } else {
                "播放"
            },
            true,
            transport_icon(snapshot),
        ),
        ITEM_PREVIOUS => text(
            "上一首",
            snapshot.has_track() && snapshot.can_go_previous,
            "media-skip-backward-symbolic",
        ),
        ITEM_NEXT => text(
            "下一首",
            snapshot.has_track() && snapshot.can_go_next,
            "media-skip-forward-symbolic",
        ),
        ITEM_VOLUME => submenu(
            &format!(
                "音量  {}%",
                (snapshot.volume.clamp(0.0, 1.0) * 100.0).round() as i32
            ),
            true,
            volume_icon(snapshot.volume),
        ),
        ITEM_MODE => submenu(
            &format!("播放模式  ·  {}", snapshot.playback_mode.label()),
            true,
            mode_icon(snapshot.playback_mode),
        ),
        ITEM_SHOW => text("显示 Caw", true, "view-visible-symbolic"),
        ITEM_QUIT => text("退出 Caw", true, "application-exit-symbolic"),
        _ => {
            let volume_index = volume_level_index(snapshot.volume);
            if (VOLUME_BASE..VOLUME_BASE + VOLUME_LEVELS.len() as i32).contains(&id) {
                let index = (id - VOLUME_BASE) as usize;
                return Some(radio(
                    &format!("{}%", VOLUME_LEVELS[index]),
                    volume_icon(VOLUME_LEVELS[index] as f32 / 100.0),
                    index == volume_index,
                ));
            }
            if (MODE_BASE..MODE_BASE + MODES.len() as i32).contains(&id) {
                let index = (id - MODE_BASE) as usize;
                let mode = MODES[index];
                return Some(radio(
                    mode.label(),
                    mode_icon(mode),
                    index == mode_index(snapshot.playback_mode),
                ));
            }
            return None;
        }
    })
}

fn volume_children(snapshot: &NowPlaying) -> MenuChildren {
    (0..VOLUME_LEVELS.len())
        .map(|index| {
            let id = VOLUME_BASE + index as i32;
            layout_node(id, item_props(id, snapshot).unwrap_or_default(), Vec::new())
        })
        .collect()
}

fn mode_children(snapshot: &NowPlaying) -> MenuChildren {
    (0..MODES.len())
        .map(|index| {
            let id = MODE_BASE + index as i32;
            layout_node(id, item_props(id, snapshot).unwrap_or_default(), Vec::new())
        })
        .collect()
}

fn menu_children(snapshot: &NowPlaying, include_submenus: bool) -> MenuChildren {
    vec![
        layout_node(
            ITEM_NOW_PLAYING,
            item_props(ITEM_NOW_PLAYING, snapshot).unwrap_or_default(),
            Vec::new(),
        ),
        layout_node(ITEM_SEP_TOP, separator(), Vec::new()),
        layout_node(
            ITEM_PLAY_PAUSE,
            item_props(ITEM_PLAY_PAUSE, snapshot).unwrap_or_default(),
            Vec::new(),
        ),
        layout_node(
            ITEM_PREVIOUS,
            item_props(ITEM_PREVIOUS, snapshot).unwrap_or_default(),
            Vec::new(),
        ),
        layout_node(
            ITEM_NEXT,
            item_props(ITEM_NEXT, snapshot).unwrap_or_default(),
            Vec::new(),
        ),
        layout_node(ITEM_SEP_MID, separator(), Vec::new()),
        layout_node(
            ITEM_VOLUME,
            item_props(ITEM_VOLUME, snapshot).unwrap_or_default(),
            if include_submenus {
                volume_children(snapshot)
            } else {
                Vec::new()
            },
        ),
        layout_node(
            ITEM_MODE,
            item_props(ITEM_MODE, snapshot).unwrap_or_default(),
            if include_submenus {
                mode_children(snapshot)
            } else {
                Vec::new()
            },
        ),
        layout_node(ITEM_SEP_BOTTOM, separator(), Vec::new()),
        layout_node(
            ITEM_SHOW,
            item_props(ITEM_SHOW, snapshot).unwrap_or_default(),
            Vec::new(),
        ),
        layout_node(
            ITEM_QUIT,
            item_props(ITEM_QUIT, snapshot).unwrap_or_default(),
            Vec::new(),
        ),
    ]
}

/// Menu revision derived from the values that shape the layout, so clients refresh
/// labels/icons when playback state changes.
fn revision(snapshot: &NowPlaying) -> u32 {
    let mut hasher = DefaultHasher::new();
    snapshot.playback_state.hash(&mut hasher);
    snapshot.playback_mode.hash(&mut hasher);
    snapshot.track_id.hash(&mut hasher);
    snapshot.title.hash(&mut hasher);
    snapshot.artist.hash(&mut hasher);
    (snapshot.position_secs.round() as i64).hash(&mut hasher);
    (snapshot.duration_secs.round() as i64).hash(&mut hasher);
    ((snapshot.volume * 100.0).round() as i32).hash(&mut hasher);
    snapshot.can_go_next.hash(&mut hasher);
    snapshot.can_go_previous.hash(&mut hasher);
    hasher.finish() as u32
}

// -- org.kde.StatusNotifierItem --------------------------------------------------

struct StatusNotifierItem {
    shared: Arc<Shared>,
}

#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl StatusNotifierItem {
    #[zbus(property(emits_changed_signal = "const"))]
    fn category(&self) -> String {
        "ApplicationStatus".to_owned()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn id(&self) -> String {
        "caw".to_owned()
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn title(&self) -> String {
        "Caw".to_owned()
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn status(&self) -> String {
        "Active".to_owned()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn icon_name(&self) -> String {
        ICON_NAME.to_owned()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn icon_pixmap(&self) -> IconPixmap {
        Vec::new()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn overlay_icon_name(&self) -> String {
        String::new()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn attention_icon_name(&self) -> String {
        String::new()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn attention_icon_pixmap(&self) -> IconPixmap {
        Vec::new()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn attention_movie_name(&self) -> String {
        String::new()
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn tool_tip(&self) -> ToolTip {
        let snapshot = self.shared.read();
        let subtitle = if snapshot.title.is_empty() {
            "本地音乐播放器".to_owned()
        } else if snapshot.artist.is_empty() {
            snapshot.title.clone()
        } else {
            format!("{} — {}", snapshot.title, snapshot.artist)
        };
        (ICON_NAME.to_owned(), Vec::new(), "Caw".to_owned(), subtitle)
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn item_is_menu(&self) -> bool {
        false
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn menu(&self) -> OwnedObjectPath {
        OwnedObjectPath::try_from(MENU_PATH).expect("valid menu object path")
    }

    fn activate(&self, _x: i32, _y: i32) {
        self.shared.send(PlatformCommand::ShowWindow);
    }

    fn secondary_activate(&self, _x: i32, _y: i32) {
        self.shared.send(PlatformCommand::TogglePlayPause);
    }

    fn context_menu(&self, _x: i32, _y: i32) {}

    fn scroll(&self, delta: i32, orientation: String) {
        if orientation != "vertical" || delta == 0 {
            return;
        }
        let current = self.shared.read().volume;
        let next = (current + if delta > 0 { 0.05 } else { -0.05 }).clamp(0.0, 1.0);
        self.shared.send(PlatformCommand::SetVolume(next));
    }
}

// -- com.canonical.dbusmenu -------------------------------------------------------

struct DbusMenu {
    shared: Arc<Shared>,
}

#[zbus::interface(name = "com.canonical.dbusmenu")]
impl DbusMenu {
    #[zbus(property(emits_changed_signal = "const"))]
    fn version(&self) -> u32 {
        3
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn text_direction(&self) -> String {
        "ltr".to_owned()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn status(&self) -> String {
        "normal".to_owned()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn icon_theme_path(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        _property_names: Vec<String>,
    ) -> MenuLayout {
        let snapshot = self.shared.read();
        let revision = revision(&snapshot);
        // -1 means "everything"; 1 means immediate children only. Respecting this
        // avoids sending nested submenu items twice (which made Plasma show a
        // duplicate menu on hover).
        let include_children = recursion_depth != 0;
        let include_submenus = !(0..2).contains(&recursion_depth);

        // Clients request a submenu by asking for the layout of its parent id.
        // Returning the root layout for every id made Plasma render a second,
        // duplicate menu when hovering 「音量」/「播放模式」.
        match parent_id {
            0 => {
                let mut root_props = HashMap::new();
                root_props.insert("children-display".to_owned(), string_value("submenu"));
                (
                    revision,
                    (0, root_props, menu_children(&snapshot, include_submenus)),
                )
            }
            ITEM_VOLUME => (
                revision,
                (
                    ITEM_VOLUME,
                    item_props(ITEM_VOLUME, &snapshot).unwrap_or_default(),
                    if include_children {
                        volume_children(&snapshot)
                    } else {
                        Vec::new()
                    },
                ),
            ),
            ITEM_MODE => (
                revision,
                (
                    ITEM_MODE,
                    item_props(ITEM_MODE, &snapshot).unwrap_or_default(),
                    if include_children {
                        mode_children(&snapshot)
                    } else {
                        Vec::new()
                    },
                ),
            ),
            other => (revision, (other, HashMap::new(), Vec::new())),
        }
    }

    fn get_group_properties(
        &self,
        ids: Vec<i32>,
        _property_names: Vec<String>,
    ) -> Vec<(i32, MenuProps)> {
        let snapshot = self.shared.read();
        ids.into_iter()
            .filter_map(|id| item_props(id, &snapshot).map(|props| (id, props)))
            .collect()
    }

    fn get_property(&self, id: i32, name: String) -> OwnedValue {
        let snapshot = self.shared.read();
        item_props(id, &snapshot)
            .and_then(|props| props.get(&name).cloned())
            .unwrap_or_else(|| string_value(""))
    }

    fn event(&self, id: i32, event_id: String, _data: Value<'_>, _timestamp: u32) {
        if event_id != "clicked" {
            return;
        }
        let command = if id == ITEM_PLAY_PAUSE {
            Some(PlatformCommand::TogglePlayPause)
        } else if id == ITEM_PREVIOUS {
            Some(PlatformCommand::Previous)
        } else if id == ITEM_NEXT {
            Some(PlatformCommand::Next)
        } else if id == ITEM_SHOW {
            Some(PlatformCommand::ShowWindow)
        } else if id == ITEM_QUIT {
            Some(PlatformCommand::Quit)
        } else if (VOLUME_BASE..VOLUME_BASE + VOLUME_LEVELS.len() as i32).contains(&id) {
            let index = (id - VOLUME_BASE) as usize;
            Some(PlatformCommand::SetVolume(
                VOLUME_LEVELS[index] as f32 / 100.0,
            ))
        } else if (MODE_BASE..MODE_BASE + MODES.len() as i32).contains(&id) {
            let index = (id - MODE_BASE) as usize;
            Some(PlatformCommand::SetPlaybackMode(MODES[index]))
        } else {
            None
        };
        if let Some(command) = command {
            self.shared.send(command);
        }
    }

    fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    fn about_to_show_group(&self, _ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        (Vec::new(), Vec::new())
    }
}

fn string_value(text: &str) -> OwnedValue {
    OwnedValue::from(Str::from(text))
}
