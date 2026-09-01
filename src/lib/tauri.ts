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
  return invoke<void>("add_to_playlist", { playlistId, trackIds });
}

export async function removeFromPlaylist(playlistId: number, trackIds: number[]): Promise<void> {
  return invoke<void>("remove_from_playlist", { playlistId, trackIds });
}

export async function reorderPlaylist(playlistId: number, trackId: number, newPosition: number): Promise<void> {
  return invoke<void>("reorder_playlist", { playlistId, trackId, newPosition });
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

export function onRemoteTrack(
  callback: (payload: RemoteTrackDto) => void,
): Promise<UnlistenFn> {
  return listen<RemoteTrackDto>("remote_track", (event) => {
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

export async function getMinimizeToTray(): Promise<boolean> {
  return invoke<boolean>("get_minimize_to_tray");
}

export async function setMinimizeToTray(enable: boolean): Promise<void> {
  return invoke<void>("set_minimize_to_tray", { enable });
}

export async function getClosePrompt(): Promise<boolean> {
  return invoke<boolean>("get_close_prompt");
}

export async function setClosePrompt(enable: boolean): Promise<void> {
  return invoke<void>("set_close_prompt", { enable });
}

export async function exitApp(): Promise<void> {
  return invoke<void>("exit_app");
}

export async function hideWindow(): Promise<void> {
  return invoke<void>("hide_window");
}

export async function showWindow(): Promise<void> {
  return invoke<void>("show_window");
}

export function onWindowCloseRequested(
  callback: () => void,
): Promise<UnlistenFn> {
  return listen("window_close_requested", () => {
    callback();
  });
}

export async function playNext(id: number): Promise<void> {
  return invoke<void>("play_next", { id });
}

export async function rescanAll(): Promise<void> {
  return invoke<void>("rescan_all");
}

/** Fetch all system-installed fonts (family names) from the Rust backend. */
export async function getSystemFonts(): Promise<string[]> {
  return invoke<string[]>("get_system_fonts");
}

// ─── Queue Commands ──────────────────────────────────────────────────

export async function addToQueue(id: number): Promise<void> {
  return invoke<void>("add_to_queue", { id });
}

export async function removeFromQueue(index: number): Promise<void> {
  return invoke<void>("remove_from_queue", { index });
}

export async function reorderQueue(from: number, to: number): Promise<void> {
  return invoke<void>("reorder_queue", { from, to });
}

export async function clearQueue(): Promise<void> {
  return invoke<void>("clear_queue");
}

export interface QueueStateDto {
  queue: number[];
  current_track_id: number | null;
  history: number[];
}

export async function getQueueState(): Promise<QueueStateDto> {
  return invoke<QueueStateDto>("get_queue_state");
}

export async function saveQueueAsPlaylist(name: string): Promise<number> {
  return invoke<number>("save_queue_as_playlist", { name });
}

/** Queue change event listener */
export function onQueueChanged(
  callback: (payload: PlayerStateDto) => void,
): Promise<UnlistenFn> {
  return listen<PlayerStateDto>("queue_changed", (event) => {
    callback(event.payload);
  });
}

// ─── Queue Replace Mode ──────────────────────────────────────────

export async function playTracksInsert(ids: number[], startId: number): Promise<void> {
  return invoke<void>("play_tracks_insert", { ids, startId });
}

export async function getQueueReplaceMode(): Promise<boolean> {
  return invoke<boolean>("get_queue_replace_mode");
}

export async function setQueueReplaceMode(enabled: boolean): Promise<void> {
  return invoke<void>("set_queue_replace_mode", { enabled });
}

// ─── NetEase Cloud Music ───────────────────────────────────────────────

export interface NeteaseConfigDto {
  enabled: boolean;
  app_id: string;
  has_private_key: boolean;
  anonymous_mode: boolean;
  api_host: string;
  device_complete: boolean;
  logged_in: boolean;
  user_name: string | null;
}

export interface UpdateNeteaseConfigDto {
  enabled: boolean;
  app_id?: string;
  private_key?: string;
  anonymous_mode?: boolean;
}

export interface RemoteTrackDto {
  id: string;
  title: string;
  artist: string;
  album: string;
  pic_url: string | null;
  duration_secs: number;
}

export interface NeteaseSongDto {
  id: string;
  original_id: number | null;
  title: string;
  artist: string;
  album: string;
  album_id: string | null;
  duration_secs: number;
  pic_url: string | null;
  liked: boolean;
  vip_only: boolean;
  playable: boolean;
  gain: number | null;
  peak: number | null;
}

export interface NeteasePlaylistDto {
  id: string;
  original_id: number | null;
  name: string;
  cover_url: string | null;
  track_count: number;
  description: string | null;
  creator: string | null;
}

export interface NeteaseSearchResultDto {
  songs: NeteaseSongDto[];
  total: number;
}

export interface NeteasePlayUrlDto {
  url: string;
  format: string;
  bitrate: number;
  size: number;
  level: string | null;
  gain: number | null;
  peak: number | null;
}

export interface NeteaseLyricDto {
  lyric: string;
  tlyric: string | null;
}

export interface NeteaseConnectionStatus {
  ok: boolean;
  message: string;
  user_name: string | null;
}

export interface NeteaseQrSessionDto {
  uni_key: string;
  qr_code_url: string;
}

export interface NeteaseQrPollDto {
  status: 'waiting' | 'scanned' | 'confirmed' | 'expired' | 'error';
  message: string;
  user: { id: string; original_id: number | null; nickname: string; avatar_url: string | null; vip: boolean } | null;
}

export interface NeteaseUserDto {
  id: string;
  original_id: number | null;
  nickname: string;
  avatar_url: string | null;
  vip: boolean;
}

export async function getNeteaseConfig(): Promise<NeteaseConfigDto> {
  return invoke<NeteaseConfigDto>('get_netease_config');
}

export async function setNeteaseConfig(config: UpdateNeteaseConfigDto): Promise<NeteaseConfigDto> {
  return invoke<NeteaseConfigDto>('set_netease_config', { config });
}

export async function testNeteaseConnection(): Promise<NeteaseConnectionStatus> {
  return invoke<NeteaseConnectionStatus>('test_netease_connection');
}

// Auth
export async function neteaseStartQrLogin(): Promise<NeteaseQrSessionDto> {
  return invoke<NeteaseQrSessionDto>('netease_start_qr_login');
}

export async function neteasePollQrLogin(uniKey: string): Promise<NeteaseQrPollDto> {
  return invoke<NeteaseQrPollDto>('netease_poll_qr_login', { uniKey });
}

export async function neteaseLogout(): Promise<void> {
  return invoke<void>('netease_logout');
}

export async function neteaseCurrentUser(): Promise<NeteaseUserDto | null> {
  return invoke<NeteaseUserDto | null>('netease_current_user');
}

// Content
export async function neteaseRecommendDaily(limit = 30): Promise<NeteaseSongDto[]> {
  return invoke<NeteaseSongDto[]>('netease_recommend_daily', { limit });
}

export async function neteaseHeartbeat(playlistId: string, songId: string, count = 20): Promise<NeteaseSongDto[]> {
  return invoke<NeteaseSongDto[]>('netease_heartbeat', { playlistId, songId, count });
}

export async function neteasePlayHistory(limit = 30): Promise<NeteaseSongDto[]> {
  return invoke<NeteaseSongDto[]>('netease_play_history', { limit });
}

export async function neteaseMyPlaylists(limit = 50): Promise<NeteasePlaylistDto[]> {
  return invoke<NeteasePlaylistDto[]>('netease_my_playlists', { limit });
}

export async function neteasePlaylistTracks(playlistId: string, limit = 100, offset = 0): Promise<NeteaseSongDto[]> {
  return invoke<NeteaseSongDto[]>('netease_playlist_tracks', { playlistId, limit, offset });
}

export async function neteaseSetLike(songId: string, like: boolean): Promise<void> {
  return invoke<void>('netease_set_like', { songId, like });
}

export async function neteaseSearchSongs(
  query: string,
  page = 1,
  limit = 30,
): Promise<NeteaseSearchResultDto> {
  return invoke<NeteaseSearchResultDto>('netease_search_songs', { query, page, limit });
}

export async function neteaseGetSongUrl(songId: string): Promise<NeteasePlayUrlDto | null> {
  return invoke<NeteasePlayUrlDto | null>('netease_get_song_url', { songId });
}

export async function neteaseGetLyric(songId: string): Promise<NeteaseLyricDto> {
  return invoke<NeteaseLyricDto>('netease_get_lyric', { songId });
}

export async function neteasePlayOnline(song: NeteaseSongDto): Promise<void> {
  return invoke<void>('netease_play_online', { song });
}
