/**
 * Pinia playback store — synchronised with the Rust backend via
 * Tauri invoke commands + events.
 */
import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import * as api from "../lib/tauri";

export const usePlaybackStore = defineStore("playback", () => {
  // ── State ──

  const library = ref<api.TrackDto[]>([]);
  const currentTrackId = ref<number | null>(null);
  const isPlaying = ref(false);
  const position = ref(0);
  const duration = ref(0);
  const volume = ref(0.8);
  const shuffle = ref(false);
  const repeat = ref("none");
  const queue = ref<number[]>([]);
  const history = ref<number[]>([]);
  const loading = ref(true);
  const queueReplaceMode = ref(true);

  // ── Derived ──

  const currentTrack = computed(() =>
    library.value.find((t) => t.id === currentTrackId.value) ?? null,
  );

  // ── Cover cache ──

  const coverCache = new Map<number, string>();

  async function getCoverUrl(trackId: number): Promise<string | null> {
    const cached = coverCache.get(trackId);
    if (cached) return cached;

    const bytes = await api.getCover(trackId);
    if (!bytes) return null;

    const blob = new Blob([new Uint8Array(bytes)]);
    const url = URL.createObjectURL(blob);
    coverCache.set(trackId, url);
    return url;
  }

  function clearCoverCache() {
    for (const url of coverCache.values()) {
      URL.revokeObjectURL(url);
    }
    coverCache.clear();
  }

  // ── Event listeners ──

  let unlistens: UnlistenFn[] = [];

  async function registerListeners() {
    unlistens.push(
      await api.onPosition((p) => {
        position.value = p.current;
        duration.value = p.total;
        isPlaying.value = p.is_playing;
        if (p.track_id) currentTrackId.value = p.track_id;
      }),
    );

    unlistens.push(
      await api.onPlaybackState((s) => {
        isPlaying.value = s.is_playing;
        currentTrackId.value = s.current_track_id;
        position.value = s.position;
        duration.value = s.duration;
        volume.value = s.volume;
        shuffle.value = s.shuffle;
        repeat.value = s.repeat;
      }),
    );

    unlistens.push(
      await api.onTrackChanged((p) => {
        currentTrackId.value = p.track_id;
      }),
    );

    unlistens.push(
      await api.onLibraryUpdated(async () => {
        library.value = await api.getLibrary();
      }),
    );

    unlistens.push(
      await api.onScanProgress((p) => {
        if (p.done) loading.value = false;
      }),
    );

    unlistens.push(
      await api.onQueueChanged((p) => {
        queue.value = p.queue;
        currentTrackId.value = p.current_track_id;
      }),
    );
  }

  function unregisterListeners() {
    unlistens.forEach((fn) => fn());
    unlistens = [];
  }

  // ── Init / Cleanup ──

  async function init() {
    loading.value = true;
    try {
      const [lib, st] = await Promise.all([api.getLibrary(), api.getState()]);
      library.value = lib;
      currentTrackId.value = st.current_track_id;
      isPlaying.value = st.is_playing;
      position.value = st.position;
      duration.value = st.duration;
      volume.value = st.volume;
      shuffle.value = st.shuffle;
      repeat.value = st.repeat;
      queue.value = st.queue;
    } catch (e) {
      console.error("caw: store init error", e);
    }
    loading.value = false;
    await registerListeners();

    // Fetch history on startup
    const qs = await api.getQueueState();
    history.value = qs.history;

    // Load queue replace mode
    queueReplaceMode.value = await api.getQueueReplaceMode();
  }

  function cleanup() {
    unregisterListeners();
    clearCoverCache();
  }

  // ── Transport actions ──

  async function playTracks(ids: number[], startId: number) {
    if (queueReplaceMode.value) {
      await api.playTracks(ids, startId);
    } else {
      await api.playTracksInsert(ids, startId);
    }
  }

  async function playNextTrack(id: number) {
    await api.playNext(id);
  }

  async function togglePlay() {
    await api.togglePlay();
  }

  async function next() {
    await api.nextTrack();
  }

  async function prev() {
    await api.prevTrack();
  }

  async function seekTo(sec: number) {
    position.value = sec; // optimistic
    await api.seek(sec);
  }

  async function setVolume(vol: number) {
    volume.value = vol;
    await api.setVolume(vol);
  }

  async function toggleMute() {
    await api.toggleMute();
  }

  async function setShuffle(on: boolean) {
    shuffle.value = on;
    await api.setShuffle(on);
  }

  async function setRepeat(mode: string) {
    repeat.value = mode;
    await api.setRepeat(mode);
  }

  async function pickMusicFolder() {
    const path = await api.pickMusicFolder();
    if (path !== null) {
      // library_updated event will refresh the list
      return path;
    }
    return null;
  }

  // ── Queue actions ──

  async function addToQueue(id: number) {
    await api.addToQueue(id);
  }

  async function removeFromQueue(index: number) {
    await api.removeFromQueue(index);
  }

  async function reorderQueue(from: number, to: number) {
    await api.reorderQueue(from, to);
  }

  async function clearQueue() {
    await api.clearQueue();
  }

  async function saveQueueAsPlaylist(name: string) {
    return await api.saveQueueAsPlaylist(name);
  }

  async function fetchQueueState() {
    const state = await api.getQueueState();
    queue.value = state.queue;
    history.value = state.history;
  }

  return {
    library,
    currentTrackId,
    isPlaying,
    position,
    duration,
    volume,
    shuffle,
    repeat,
    queue,
    loading,
    currentTrack,
    getCoverUrl,
    init,
    cleanup,
    playTracks,
    playNextTrack,
    togglePlay,
    next,
    prev,
    seekTo,
    setVolume,
    toggleMute,
    setShuffle,
    setRepeat,
    pickMusicFolder,
    history,
    addToQueue,
    removeFromQueue,
    reorderQueue,
    clearQueue,
    saveQueueAsPlaylist,
    fetchQueueState,
    queueReplaceMode,
    async setQueueReplaceMode(enabled: boolean) {
      queueReplaceMode.value = enabled;
      await api.setQueueReplaceMode(enabled);
    },
  };
});
