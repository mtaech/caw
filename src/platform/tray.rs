//! System tray icon (KDE/freedesktop StatusNotifierItem + com.canonical.dbusmenu).
//!
//! Implemented directly on zbus 5 because `ksni 0.3.7` does not compile against
//! zbus 5.19 (its macro-generated `DBUS_CONTEXT` was removed upstream). The item
//! publishes a small menu and maps clicks onto `PlatformCommand`s, so the tray does
//! not touch gpui entities.
//!
//! Degradation: without a StatusNotifierWatcher (no tray host) the thread logs a
//! warning and exits; the app keeps running.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Str, StructureBuilder, Value};

use crate::models::playlist::PlaybackState;

use super::{NowPlaying, PlatformCommand, Shared};

const ITEM_PATH: &str = "/StatusNotifierItem";
const MENU_PATH: &str = "/MenuBar";
const KDE_WATCHER: &str = "org.kde.StatusNotifierWatcher";
const FDO_WATCHER: &str = "org.freedesktop.StatusNotifierWatcher";
const ICON_NAME: &str = "audio-x-generic";

const MENU_SHOW: i32 = 1;
const MENU_PLAY_PAUSE: i32 = 2;
const MENU_NEXT: i32 = 3;
const MENU_PREVIOUS: i32 = 4;
const MENU_SEPARATOR: i32 = 5;
const MENU_QUIT: i32 = 6;

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

// -- Menu layout helpers ---------------------------------------------------------

fn menu_props(label: &str, enabled: bool) -> MenuProps {
    let mut props = HashMap::new();
    props.insert("label".to_owned(), OwnedValue::from(Str::from(label)));
    props.insert("enabled".to_owned(), OwnedValue::from(enabled));
    props.insert("visible".to_owned(), OwnedValue::from(true));
    props
}

fn separator_props() -> MenuProps {
    let mut props = HashMap::new();
    props.insert("type".to_owned(), OwnedValue::from(Str::from("separator")));
    props.insert("visible".to_owned(), OwnedValue::from(true));
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

fn children(snapshot: &NowPlaying) -> MenuChildren {
    let play_label = if snapshot.playback_state == PlaybackState::Playing {
        "暂停"
    } else {
        "播放"
    };
    let has_track = snapshot.has_track();

    vec![
        layout_node(MENU_SHOW, menu_props("显示 Caw", true), Vec::new()),
        layout_node(
            MENU_PLAY_PAUSE,
            menu_props(play_label, has_track),
            Vec::new(),
        ),
        layout_node(
            MENU_NEXT,
            menu_props("下一首", has_track && snapshot.can_go_next),
            Vec::new(),
        ),
        layout_node(
            MENU_PREVIOUS,
            menu_props("上一首", has_track && snapshot.can_go_previous),
            Vec::new(),
        ),
        layout_node(MENU_SEPARATOR, separator_props(), Vec::new()),
        layout_node(MENU_QUIT, menu_props("退出 Caw", true), Vec::new()),
    ]
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
        _parent_id: i32,
        _recursion_depth: i32,
        _property_names: Vec<String>,
    ) -> MenuLayout {
        let snapshot = self.shared.read();
        let mut root_props = HashMap::new();
        root_props.insert(
            "children-display".to_owned(),
            OwnedValue::from(Str::from("submenu")),
        );
        (1, (0, root_props, children(&snapshot)))
    }

    fn get_group_properties(
        &self,
        ids: Vec<i32>,
        _property_names: Vec<String>,
    ) -> Vec<(i32, MenuProps)> {
        let snapshot = self.shared.read();
        let all = all_props(&snapshot);
        ids.into_iter()
            .filter_map(|id| all.get(&id).map(|props| (id, props.clone())))
            .collect()
    }

    fn get_property(&self, id: i32, name: String) -> OwnedValue {
        let snapshot = self.shared.read();
        all_props(&snapshot)
            .get(&id)
            .and_then(|props| props.get(&name).cloned())
            .unwrap_or_else(|| OwnedValue::from(Str::from("")))
    }

    fn event(&self, id: i32, event_id: String, _data: Value<'_>, _timestamp: u32) {
        if event_id != "clicked" {
            return;
        }
        let command = match id {
            MENU_SHOW => Some(PlatformCommand::ShowWindow),
            MENU_PLAY_PAUSE => Some(PlatformCommand::TogglePlayPause),
            MENU_NEXT => Some(PlatformCommand::Next),
            MENU_PREVIOUS => Some(PlatformCommand::Previous),
            MENU_QUIT => Some(PlatformCommand::Quit),
            _ => None,
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

fn all_props(snapshot: &NowPlaying) -> HashMap<i32, MenuProps> {
    let huge_label = if snapshot.playback_state == PlaybackState::Playing {
        "暂停"
    } else {
        "播放"
    };
    let has_track = snapshot.has_track();

    HashMap::from([
        (MENU_SHOW, menu_props("显示 Caw", true)),
        (MENU_PLAY_PAUSE, menu_props(huge_label, has_track)),
        (
            MENU_NEXT,
            menu_props("下一首", has_track && snapshot.can_go_next),
        ),
        (
            MENU_PREVIOUS,
            menu_props("上一首", has_track && snapshot.can_go_previous),
        ),
        (MENU_SEPARATOR, separator_props()),
        (MENU_QUIT, menu_props("退出 Caw", true)),
    ])
}
