# Caw — Local Music Player

A pure-Rust desktop music player with a Tauri v2 + Vue 3 frontend. Backend
handles audio playback (symphonia + cpal), library scanning, and a global
controller that owns the queue/transport/auto-advance. Frontend is a pure view
layer (Vue 3 + shadcn-vue + Tailwind).

## Language

**Nav item**:
A first-class section in the sidebar that switches the main content view.
Current items: 全部音乐, 艺术家, 专辑, 播放列表, 文件夹, 设置.
_Avoid_: Tab, page, view mode

**Settings**:
A user-configurable panel in the sidebar that persists preferences via the
backend store. Currently contains music directory management (multi-directory
add/remove) and an About section.
_Avoid_: Preferences, Options

**Music directory**:
A filesystem path where music files are stored. Multiple directories are
supported; the library is built by scanning all configured directories and
deduplicating by file path.
_Avoid_: Folder path, library path

**Tauri v2 command parameter naming**:
`#[tauri::command]` defaults to **camelCase** for IPC JSON keys
(`ArgumentCase::Camel` in tauri-macros). Rust function params like
`playlist_id: i64` expect `{ playlistId: ... }` from the frontend
`invoke()`, NOT `{ playlist_id: ... }`. Single-word params (`id`,
`name`, `path`) are unaffected because snake_case and camelCase
happen to be identical.
_Avoid_: Sending snake_case keys for multi-word parameters.
