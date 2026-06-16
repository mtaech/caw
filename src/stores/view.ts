/**
 * Frontend-only view Pinia store — nav, search, sort, selection.
 * This state lives entirely in the frontend; the backend never sees it.
 */
import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { usePlaybackStore } from "./playback";

export type NavItem = "all-music" | "artists" | "albums" | "playlists" | "folders";
export type SortKey = "title" | "artist" | "album" | "duration";
export type DetailTarget =
  | { type: "album"; name: string }
  | { type: "artist"; name: string }
  | null;

export const useViewStore = defineStore("view", () => {
  const nav = ref<NavItem>("all-music");
  const searchQuery = ref("");
  const sortKey = ref<SortKey>("album");
  const sortDir = ref<"asc" | "desc">("asc");
  const detail = ref<DetailTarget>(null);
  const selectedTrackId = ref<number | null>(null);
  const columnWidths = ref({
    index: 40,
    title: 400,
    artist: 240,
    album: 240,
    duration: 80,
  });

  const playback = computed(() => usePlaybackStore());

  // ── Aggregation computed ──

  const albums = computed(() => {
    const map = new Map<
      string,
      {
        title: string;
        artist: string;
        trackIds: number[];
        trackCount: number;
        durationSecs: number;
      }
    >();
    for (const t of playback.value.library) {
      const key = t.album;
      const existing = map.get(key);
      if (existing) {
        existing.trackIds.push(t.id);
        existing.trackCount++;
        existing.durationSecs += t.duration_secs;
      } else {
        map.set(key, {
          title: t.album,
          artist: t.artist,
          trackIds: [t.id],
          trackCount: 1,
          durationSecs: t.duration_secs,
        });
      }
    }
    return Array.from(map.values()).sort((a, b) =>
      a.title.localeCompare(b.title),
    );
  });

  const artists = computed(() => {
    // First pass: collect unique albums per artist
    const artistAlbums = new Map<string, Set<string>>();
    const artistTracks = new Map<string, number[]>();
    const artistCounts = new Map<string, number>();

    for (const t of playback.value.library) {
      const a = t.artist;
      // Track albums per artist
      if (!artistAlbums.has(a)) artistAlbums.set(a, new Set());
      artistAlbums.get(a)!.add(t.album);
      // Track IDs per artist
      if (!artistTracks.has(a)) artistTracks.set(a, []);
      artistTracks.get(a)!.push(t.id);
      // Count tracks
      artistCounts.set(a, (artistCounts.get(a) ?? 0) + 1);
    }

    return Array.from(artistAlbums.keys())
      .map((name) => ({
        name,
        trackIds: artistTracks.get(name) ?? [],
        trackCount: artistCounts.get(name) ?? 0,
        albumCount: artistAlbums.get(name)?.size ?? 0,
      }))
      .sort((a, b) => a.name.localeCompare(b.name));
  });

  // ── Filtered / sorted tracks ──

  const filteredTracks = computed(() => {
    const lib = playback.value.library;
    if (!detail.value) {
      return applySearch(sortTracks(lib, sortKey.value, sortDir.value));
    }
    const dt = detail.value;
    if (dt.type === "album") {
      return sortTracks(
        lib.filter((t) => t.album === dt.name),
        "track_number",
        "asc",
      );
    }
    if (dt.type === "artist") {
      return sortTracks(
        lib.filter((t) => t.artist === dt.name),
        sortKey.value,
        sortDir.value,
      );
    }
    return sortTracks(lib, sortKey.value, sortDir.value);
  });

  function applySearch(tracks: typeof playback.value.library) {
    if (!searchQuery.value) return tracks;
    const q = searchQuery.value.toLowerCase();
    return tracks.filter(
      (t) =>
        t.title.toLowerCase().includes(q) ||
        t.artist.toLowerCase().includes(q) ||
        t.album.toLowerCase().includes(q),
    );
  }

  function sortTracks(
    tracks: typeof playback.value.library,
    key: string,
    dir: "asc" | "desc",
  ) {
    const sorted = [...tracks];
    const cmp = dir === "asc" ? 1 : -1;
    sorted.sort((a, b) => {
      let v = 0;
      switch (key) {
        case "title":
          v = a.title.localeCompare(b.title);
          break;
        case "artist":
          v = a.artist.localeCompare(b.artist);
          break;
        case "album":
          v = a.album.localeCompare(b.album);
          break;
        case "duration":
          v = a.duration_secs - b.duration_secs;
          break;
        case "track_number":
          v = a.track_number - b.track_number;
          break;
        default:
          v = 0;
      }
      return v * cmp;
    });
    return sorted;
  }

  // ── Actions ──

  function setNav(item: NavItem) {
    nav.value = item;
    searchQuery.value = "";
    detail.value = null;
    selectedTrackId.value = null;
  }

  function setSearch(q: string) {
    searchQuery.value = q;
    detail.value = null;
  }

  function setSort(key: SortKey) {
    if (sortKey.value === key) {
      sortDir.value = sortDir.value === "asc" ? "desc" : "asc";
    } else {
      sortKey.value = key;
      sortDir.value = "asc";
    }
  }

  function openAlbum(name: string) {
    detail.value = { type: "album", name };
  }

  function openArtist(name: string) {
    detail.value = { type: "artist", name };
  }

  function closeDetail() {
    detail.value = null;
  }

  function setSelectedTrack(id: number | null) {
    selectedTrackId.value = id;
  }

  function playAll() {
    const ids = filteredTracks.value.map((t) => t.id);
    if (ids.length > 0) {
      playback.value.playTracks(ids, ids[0]);
    }
  }

  function playTrackById(id: number) {
    const ids = filteredTracks.value.map((t) => t.id);
    playback.value.playTracks(ids, id);
  }

  return {
    nav,
    searchQuery,
    sortKey,
    sortDir,
    detail,
    selectedTrackId,
    columnWidths,
    albums,
    artists,
    filteredTracks,
    setNav,
    setSearch,
    setSort,
    openAlbum,
    openArtist,
    closeDetail,
    setSelectedTrack,
    playAll,
    playTrackById,
  };
});
