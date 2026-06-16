/**
 * Pinia store for playlist management.
 * The authoritative data stays in SQLite (backend); this store caches
 * the list of playlists and provides actions that call the backend.
 */
import { defineStore } from "pinia";
import { ref } from "vue";
import * as api from "@/lib/tauri";

export interface PlaylistItem {
  id: number;
  name: string;
  trackCount: number;
}

export const usePlaylistStore = defineStore("playlists", () => {
  const playlists = ref<PlaylistItem[]>([]);
  const currentPlaylistId = ref<number | null>(null);
  const currentPlaylistTracks = ref<number[]>([]);

  async function refresh() {
    const rows = await api.listPlaylists();
    playlists.value = rows.map((r) => ({
      id: r.id,
      name: r.name,
      trackCount: r.track_count,
    }));
  }

  async function openPlaylist(id: number) {
    currentPlaylistId.value = id;
    const pl = await api.getPlaylist(id);
    currentPlaylistTracks.value = pl?.track_ids ?? [];
  }

  function closePlaylist() {
    currentPlaylistId.value = null;
    currentPlaylistTracks.value = [];
  }

  async function create(name: string) {
    await api.createPlaylist(name);
    await refresh();
  }

  async function rename(id: number, name: string) {
    await api.renamePlaylist(id, name);
    await refresh();
  }

  async function remove(id: number) {
    if (currentPlaylistId.value === id) {
      closePlaylist();
    }
    await api.deletePlaylist(id);
    await refresh();
  }

  async function addTracks(id: number, trackIds: number[]) {
    await api.addToPlaylist(id, trackIds);
    if (currentPlaylistId.value === id) {
      await openPlaylist(id);
    }
    await refresh();
  }

  async function removeTracks(id: number, trackIds: number[]) {
    await api.removeFromPlaylist(id, trackIds);
    if (currentPlaylistId.value === id) {
      await openPlaylist(id);
    }
    await refresh();
  }

  async function reorder(id: number, trackId: number, newPos: number) {
    await api.reorderPlaylist(id, trackId, newPos);
    if (currentPlaylistId.value === id) {
      await openPlaylist(id);
    }
  }

  return {
    playlists,
    currentPlaylistId,
    currentPlaylistTracks,
    refresh,
    openPlaylist,
    closePlaylist,
    create,
    rename,
    remove,
    addTracks,
    removeTracks,
    reorder,
  };
});
