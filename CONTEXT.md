# Caw — Local Music Player

A pure-Rust desktop music player: GPUI (gpui-kit) UI, Symphonia decoding, cpal
output, SQLite for playlists and settings. `PlayerApp` is the single window root
view and owns every piece of mutable state; UI modules are pure view functions over
it. Audio decode/playback runs on its own thread and is polled by a UI timer.

## Language

**Nav item**:
A first-class section in the sidebar that switches the main content view.
Current items: 全部音乐, 艺术家, 专辑, 文件夹, 播放列表 (list at the top), 设置.
_Avoid_: Tab, page, view mode

**PlayerApp**:
The single top-level `Entity` and the window root view (`impl Render for PlayerApp`).
Owns the library, filtered index view, transport/queue, volume, playlists, music
directories and `AppConfig`.
_Avoid_: App, state, controller, view model

**Root view**:
`src/root_view.rs`, the `Render` implementation that composes TitleBar + Sidebar +
Content + PlayerBar and hosts global shortcuts and modal overlays.
_Avoid_: shell, frame, layout wrapper

**Content view**:
One of the six main-area renderers declared under `src/ui/content/` and dispatched by
`render_content`. Drilling into an artist/album/playlist/folder opens a detail view.
_Avoid_: page, screen, route

**Settings**:
A content view persisted by `AppConfig` (theme mode, font family, close-to-tray,
music directories) plus the About section.
_Avoid_: Preferences, Options

**Music directory**:
A filesystem path where music files are stored. Multiple directories are supported;
the library is built by scanning all configured directories and deduplicating by
canonical file path.
_Avoid_: Folder path, library path

**Cover art**:
Embedded art extracted during metadata scan, cached under `~/.config/caw/covers/`.
Always passed to `img()` as a `PathBuf` (a `String` would be looked up in the asset
bundle instead of the filesystem).
_Avoid_: Artwork URL, thumbnail

**Platform integration**:
Linux-only side effects (MPRIS, window bounds capture) in `src/platform/`. They read a
`NowPlaying` snapshot from public `PlayerApp` fields and send `PlatformCommand`s back
through a channel consumed by the 250 ms UI timer; they never mutate entities.
_Avoid_: Backend, service layer, sidecar
