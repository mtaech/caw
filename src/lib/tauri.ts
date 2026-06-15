/**
 * Typed Tauri invoke/listen wrappers.
 * Filled in as commands are added in P1+.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ── Payload types ──────────────────────────────────────────────────

export interface TrackDto {
  id: number;
  path: string;
  title: string;
  artist: string;
  album: string;
  duration_secs: number;
  track_number: number;
  has_cover: boolean;
}

export interface PlayerStateDto {
  is_playing: boolean;
  current_track_id: number | null;
  position: number;
  duration: number;
  volume: number;
  shuffle: boolean;
  repeat: string;
}

// ── Commands ───────────────────────────────────────────────────────

export async function getLibrary(): Promise<TrackDto[]> {
  return invoke<TrackDto[]>("get_library");
}

// ── Events ─────────────────────────────────────────────────────────

export function onPositionUpdate(
  callback: (payload: { current: number; total: number }) => void,
): Promise<UnlistenFn> {
  return listen<{ current: number; total: number }>("position", (event) => {
    callback(event.payload);
  });
}

export function onLibraryUpdated(
  callback: () => void,
): Promise<UnlistenFn> {
  return listen("library_updated", () => callback());
}

export function onPlaybackState(
  callback: (payload: PlayerStateDto) => void,
): Promise<UnlistenFn> {
  return listen<PlayerStateDto>("playback_state", (event) => {
    callback(event.payload);
  });
}
