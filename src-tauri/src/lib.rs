// Allow dead code — many PlayerApp / audio methods are wired in P1 onward.
#![allow(dead_code)]

mod app;
mod audio;
mod models;

use std::path::PathBuf;
use std::sync::Mutex;


use crate::app::PlayerApp;

// ── App state shared across commands ────────────────────────────────

pub struct AppState {
    pub app: Mutex<PlayerApp>,
    pub music_dir: Mutex<Option<PathBuf>>,
}

// ── Commands ───────────────────────────────────────────────────────

/// Health-check command — proves the invoke pathway works.
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! Caw Tauri backend is alive.")
}

/// Return the current library as a JSON-serializable vector of minimal DTOs.
/// P0 stub: returns empty vec. P1 will wire real data.
#[tauri::command]
fn get_library(state: tauri::State<AppState>) -> Vec<TrackDto> {
    let _app = state.app.lock().unwrap();
    // P1: map library tracks to TrackDto
    Vec::new()
}

// ── DTOs ───────────────────────────────────────────────────────────

/// Minimal public track data — never includes cover bytes on the list endpoint.
#[derive(serde::Serialize)]
pub struct TrackDto {
    pub id: u64,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: f64,
    pub track_number: u32,
    pub has_cover: bool,
}

// ── App entry ──────────────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(AppState {
            app: Mutex::new(PlayerApp::new()),
            music_dir: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![greet, get_library])
        .run(tauri::generate_context!())
        .expect("error while running Caw Tauri application");
}
