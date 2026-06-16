/**
 * Typed Tauri invoke/listen wrappers.
 * Every backend command has a corresponding async function here.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ── Payload shapes ─────────────────────────────────────────────────

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
  queue: number[];
}

export interface PositionPayload {
  current: number;
  total: number;
  is_playing: boolean;
  track_id: number | null;
}

// ─── Commands ──────────────────────────────────────────────────────

export async function getLibrary(): Promise<TrackDto[]> {
  return invoke<TrackDto[]>("get_library");
}

export async function getCover(id: number): Promise<number[] | null> {
  return invoke<number[] | null>("get_cover", { id });
}

export async function getState(): Promise<PlayerStateDto> {
  return invoke<PlayerStateDto>("get_state");
}

export async function playTracks(ids: number[], startId: number): Promise<void> {
  return invoke<void>("play_tracks", { ids, startId });
}

export async function togglePlay(): Promise<void> {
  return invoke<void>("toggle_play");
}

export async function pause(): Promise<void> {
  return invoke<void>("pause");
}

export async function resume(): Promise<void> {
  return invoke<void>("resume");
}

export async function nextTrack(): Promise<void> {
  return invoke<void>("next_track");
}

export async function prevTrack(): Promise<void> {
  return invoke<void>("prev_track");
}

export async function seek(sec: number): Promise<void> {
  return invoke<void>("seek", { sec });
}

export async function setVolume(vol: number): Promise<void> {
  return invoke<void>("set_volume", { vol });
}

export async function toggleMute(): Promise<void> {
  return invoke<void>("toggle_mute");
}

export async function setShuffle(on: boolean): Promise<void> {
  return invoke<void>("set_shuffle", { on });
}

export async function setRepeat(mode: string): Promise<void> {
  return invoke<void>("set_repeat", { mode });
}

export async function pickMusicFolder(): Promise<string | null> {
  return invoke<string | null>("pick_music_folder");
}

// ─── Playlist Commands ────────────────────────────────────────────────

export interface PlaylistRow {
  id: number;
  name: string;
  track_count: number;
}

export interface PlaylistWithTracks {
  id: number;
  name: string;
  track_ids: number[];
}

export async function listPlaylists(): Promise<PlaylistRow[]> {
  return invoke<PlaylistRow[]>("list_playlists");
}

export async function getPlaylist(id: number): Promise<PlaylistWithTracks | null> {
  return invoke<PlaylistWithTracks | null>("get_playlist", { id });
}

export async function createPlaylist(name: string): Promise<number> {
  return invoke<number>("create_playlist", { name });
}

export async function renamePlaylist(id: number, name: string): Promise<void> {
  return invoke<void>("rename_playlist", { id, name });
}

export async function deletePlaylist(id: number): Promise<void> {
  return invoke<void>("delete_playlist", { id });
}

export async function addToPlaylist(playlistId: number, trackIds: number[]): Promise<void> {
  return invoke<void>("add_to_playlist", { playlist_id: playlistId, track_ids: trackIds });
}

export async function removeFromPlaylist(playlistId: number, trackIds: number[]): Promise<void> {
  return invoke<void>("remove_from_playlist", { playlist_id: playlistId, track_ids: trackIds });
}

export async function reorderPlaylist(playlistId: number, trackId: number, newPosition: number): Promise<void> {
  return invoke<void>("reorder_playlist", { playlist_id: playlistId, track_id: trackId, new_position: newPosition });
}

// ─── Events ────────────────────────────────────────────────────────

export function onPosition(
  callback: (payload: PositionPayload) => void,
): Promise<UnlistenFn> {
  return listen<PositionPayload>("position", (event) => {
    callback(event.payload);
  });
}

export function onPlaybackState(
  callback: (payload: PlayerStateDto) => void,
): Promise<UnlistenFn> {
  return listen<PlayerStateDto>("playback_state", (event) => {
    callback(event.payload);
  });
}

export function onTrackChanged(
  callback: (payload: { track_id: number }) => void,
): Promise<UnlistenFn> {
  return listen<{ track_id: number }>("track_changed", (event) => {
    callback(event.payload);
  });
}

export function onLibraryUpdated(callback: () => void): Promise<UnlistenFn> {
  return listen("library_updated", () => callback());
}

export function onScanProgress(
  callback: (payload: { scanned?: number; done?: boolean }) => void,
): Promise<UnlistenFn> {
  return listen<{ scanned?: number; done?: boolean }>("scan_progress", (event) => {
    callback(event.payload);
  });
}

export function onPlaylistChanged(
  callback: (payload: { id?: number }) => void,
): Promise<UnlistenFn> {
  return listen<{ id?: number }>("playlist_changed", (event) => {
    callback(event.payload);
  });
}

// ── Settings Commands ────────────────────────────────────────────────

export async function getMusicDirs(): Promise<string[]> {
  return invoke<string[]>("get_music_dirs");
}

export async function removeMusicDir(path: string): Promise<void> {
  return invoke<void>("remove_music_dir", { path });
}
