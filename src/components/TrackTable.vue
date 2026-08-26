<template>
  <div class="flex flex-col h-full select-none">
    <!-- Header -->
    <div
      class="flex items-center border-b border-border px-4 h-9 text-caption text-muted-foreground uppercase tracking-wider flex-shrink-0"
    >
      <!-- # / Track Number -->
      <div
        class="relative flex items-center justify-center flex-shrink-0 cursor-pointer hover:text-foreground transition-colors h-full"
        :style="{ width: view.columnWidths.index + 'px' }"
        :class="{ 'text-foreground': view.sortKey === 'track_number' }"
        @click="handleSort('track_number')"
      >
        <span>#</span>
        <template v-if="view.sortKey === 'track_number'">
          <ChevronUp v-if="view.sortDir === 'asc'" class="w-3 h-3 ml-0.5" />
          <ChevronDown v-else class="w-3 h-3 ml-0.5" />
        </template>
        <!-- Resize handle: Index / Title -->
        <div
          class="absolute -right-1.5 top-0 bottom-0 w-3 cursor-col-resize flex items-center justify-center z-10 group/handle"
          title="拖动调整列宽，双击重置"
          @mousedown.stop.prevent="startResize('index-title', $event)"
          @dblclick.stop.prevent="resetCol('index')"
          @click.stop
        >
          <div
            class="w-[1px] h-3.5 transition-colors"
            :class="resizingHandle === 'index-title' ? 'bg-primary w-[2px]' : 'bg-border/60 group-hover/handle:bg-primary group-hover/handle:w-[2px]'"
          />
        </div>
      </div>

      <!-- Title -->
      <div
        class="relative flex items-center gap-1 min-w-0 flex-1 cursor-pointer hover:text-foreground transition-colors pl-3 h-full"
        :class="{ 'text-foreground': view.sortKey === 'title' }"
        @click="handleSort('title')"
      >
        <span class="truncate">标题</span>
        <template v-if="view.sortKey === 'title'">
          <ChevronUp v-if="view.sortDir === 'asc'" class="w-3 h-3 flex-shrink-0" />
          <ChevronDown v-else class="w-3 h-3 flex-shrink-0" />
        </template>
        <!-- Resize handle: Title / Artist -->
        <div
          class="absolute -right-1.5 top-0 bottom-0 w-3 cursor-col-resize flex items-center justify-center z-10 group/handle"
          title="拖动调整列宽，双击重置"
          @mousedown.stop.prevent="startResize('title-artist', $event)"
          @dblclick.stop.prevent="resetCol('artist')"
          @click.stop
        >
          <div
            class="w-[1px] h-3.5 transition-colors"
            :class="resizingHandle === 'title-artist' ? 'bg-primary w-[2px]' : 'bg-border/60 group-hover/handle:bg-primary group-hover/handle:w-[2px]'"
          />
        </div>
      </div>

      <!-- Artist -->
      <div
        class="relative flex items-center gap-1 flex-shrink-0 cursor-pointer hover:text-foreground transition-colors pl-3 h-full"
        :style="{ width: view.columnWidths.artist + 'px' }"
        :class="{ 'text-foreground': view.sortKey === 'artist' }"
        @click="handleSort('artist')"
      >
        <span class="truncate">艺人</span>
        <template v-if="view.sortKey === 'artist'">
          <ChevronUp v-if="view.sortDir === 'asc'" class="w-3 h-3 flex-shrink-0" />
          <ChevronDown v-else class="w-3 h-3 flex-shrink-0" />
        </template>
        <!-- Resize handle: Artist / Album -->
        <div
          class="absolute -right-1.5 top-0 bottom-0 w-3 cursor-col-resize flex items-center justify-center z-10 group/handle"
          title="拖动调整列宽，双击重置"
          @mousedown.stop.prevent="startResize('artist-album', $event)"
          @dblclick.stop.prevent="resetCol('artist-album')"
          @click.stop
        >
          <div
            class="w-[1px] h-3.5 transition-colors"
            :class="resizingHandle === 'artist-album' ? 'bg-primary w-[2px]' : 'bg-border/60 group-hover/handle:bg-primary group-hover/handle:w-[2px]'"
          />
        </div>
      </div>

      <!-- Album -->
      <div
        class="relative flex items-center gap-1 flex-shrink-0 cursor-pointer hover:text-foreground transition-colors pl-3 h-full"
        :style="{ width: view.columnWidths.album + 'px' }"
        :class="{ 'text-foreground': view.sortKey === 'album' }"
        @click="handleSort('album')"
      >
        <span class="truncate">专辑</span>
        <template v-if="view.sortKey === 'album'">
          <ChevronUp v-if="view.sortDir === 'asc'" class="w-3 h-3 flex-shrink-0" />
          <ChevronDown v-else class="w-3 h-3 flex-shrink-0" />
        </template>
        <!-- Resize handle: Album / Duration -->
        <div
          class="absolute -right-1.5 top-0 bottom-0 w-3 cursor-col-resize flex items-center justify-center z-10 group/handle"
          title="拖动调整列宽，双击重置"
          @mousedown.stop.prevent="startResize('album-duration', $event)"
          @dblclick.stop.prevent="resetCol('album-duration')"
          @click.stop
        >
          <div
            class="w-[1px] h-3.5 transition-colors"
            :class="resizingHandle === 'album-duration' ? 'bg-primary w-[2px]' : 'bg-border/60 group-hover/handle:bg-primary group-hover/handle:w-[2px]'"
          />
        </div>
      </div>

      <!-- Duration -->
      <div
        class="relative flex items-center gap-1 flex-shrink-0 cursor-pointer hover:text-foreground transition-colors pl-3 h-full"
        :style="{ width: view.columnWidths.duration + 'px' }"
        :class="{ 'text-foreground': view.sortKey === 'duration' }"
        @click="handleSort('duration')"
      >
        <span class="truncate">时长</span>
        <template v-if="view.sortKey === 'duration'">
          <ChevronUp v-if="view.sortDir === 'asc'" class="w-3 h-3 flex-shrink-0" />
          <ChevronDown v-else class="w-3 h-3 flex-shrink-0" />
        </template>
      </div>
    </div>

    <!-- Virtualized rows -->
    <div ref="parentRef" class="flex-1 overflow-auto select-none">
      <div v-if="virtualizer" class="relative" :style="{ height: virtualizer.getTotalSize() + 'px' }">
        <div
          v-for="row in virtualRows"
          :key="row.key"
          class="group flex items-center px-4 border-b border-border/40 transition-colors duration-120 cursor-pointer absolute top-0 left-0 w-full"
          :style="{
            height: row.size + 'px',
            transform: `translateY(${row.start}px)`,
          }"
          :class="rowClasses(row._track)"
          @dblclick="view.playTrackById(row._track.id)"
          @click="handleRowClick(row._track.id, $event)"
          @contextmenu.prevent="showContextMenu(row._track, $event)"
        >
          <!-- # / now playing indicator -->
          <div
            class="flex items-center justify-center flex-shrink-0"
            :style="{ width: view.columnWidths.index + 'px' }"
          >
            <Play
              v-if="row._track.id === playback.currentTrackId && playback.isPlaying"
              class="w-3 h-3 text-primary fill-primary"
            />
            <span
              v-else
              class="text-caption"
              :class="
                row._track.id === playback.currentTrackId
                  ? 'text-primary font-medium'
                  : 'text-faint-foreground'
              "
            >
              {{ row.index + 1 }}
            </span>
          </div>

          <!-- Title -->
          <div
            class="truncate text-body flex-1 min-w-0 pl-3"
            :class="{ 'text-body-md text-primary': row._track.id === playback.currentTrackId }"
          >
            {{ row._track.title }}
          </div>

          <!-- Artist -->
          <div
            class="truncate text-body-sm text-muted-foreground flex-shrink-0 pl-3"
            :style="{ width: view.columnWidths.artist + 'px' }"
          >
            {{ row._track.artist }}
          </div>

          <!-- Album -->
          <div
            class="truncate text-body-sm text-muted-foreground flex-shrink-0 pl-3"
            :style="{ width: view.columnWidths.album + 'px' }"
          >
            {{ row._track.album }}
          </div>

          <!-- Duration + actions -->
          <div
            class="flex items-center justify-between flex-shrink-0 pl-3 pr-1"
            :style="{ width: view.columnWidths.duration + 'px' }"
          >
            <span class="text-body-sm text-muted-foreground tabular-nums truncate">
              {{ fmt(row._track.duration_secs) }}
            </span>
            <div class="relative" @click.stop>
              <button
                class="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-primary transition-all p-0.5 rounded flex-shrink-0"
                title="添加到播放列表"
                @click="toggleAddMenu(row._track.id, $event)"
              >
                <Plus class="w-3.5 h-3.5" />
              </button>
              <!-- Add-to-playlist dropdown -->
              <div
                v-if="addMenuTrackId === row._track.id"
                class="absolute right-0 top-full mt-1 w-44 bg-elevated border border-border rounded-lg shadow-2 z-dropdown py-1 max-h-64 overflow-auto"
              >
                <p v-if="plStore.playlists.length === 0" class="px-3 py-2 text-xs text-muted-foreground">
                  暂无播放列表
                </p>
                <button
                  v-for="pl in plStore.playlists"
                  :key="pl.id"
                  class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-elevated-hover transition-colors truncate flex items-center"
                  @click.stop="addToPlaylist(pl.id, row._track.id)"
                >
                  <ListMusic class="w-3 h-3 mr-2 text-muted-foreground flex-shrink-0" />
                  <span class="truncate">{{ pl.name }}</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Context menu (right-click) -->
  <div
    v-if="contextMenu"
    class="fixed z-dropdown w-44 bg-elevated border border-border rounded-lg shadow-2 py-1 max-h-64 overflow-auto"
    :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
    @click.stop
  >
    <button
      class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-elevated-hover flex items-center gap-2 transition-colors"
      @click="playContextTrack()"
    >
      <Play class="w-3.5 h-3.5 flex-shrink-0" /> 播放
    </button>
    <button
      class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-elevated-hover flex items-center gap-2 transition-colors"
      @click="playNextContextTrack()"
    >
      <ListOrdered class="w-3.5 h-3.5 flex-shrink-0" /> 下一首播放
    </button>
    <div class="relative">
      <button
        class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-elevated-hover flex items-center gap-2 transition-colors"
        @click.stop="toggleAddFromContext($event)"
      >
        <Plus class="w-3.5 h-3.5 flex-shrink-0" /> 添加到播放列表
      </button>
      <div
        v-if="showContextAddMenu"
        class="absolute left-full top-0 ml-1 w-44 bg-elevated border border-border rounded-lg shadow-2 py-1 z-dropdown max-h-64 overflow-auto"
      >
        <p
          v-if="plStore.playlists.length === 0"
          class="px-3 py-2 text-xs text-muted-foreground"
        >
          暂无播放列表
        </p>
        <button
          v-for="pl in plStore.playlists"
          :key="pl.id"
          class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-elevated-hover transition-colors truncate flex items-center"
          @click.stop="addContextTrackToPlaylist(pl.id)"
        >
          <ListMusic class="w-3 h-3 mr-2 text-muted-foreground flex-shrink-0" />
          <span class="truncate">{{ pl.name }}</span>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { Play, ChevronUp, ChevronDown, Plus, ListMusic, ListOrdered } from "lucide-vue-next";
import { usePlaybackStore } from "@/stores/playback";
import { useViewStore } from "@/stores/view";
import { usePlaylistStore } from "@/stores/playlists";

const playback = usePlaybackStore();
const view = useViewStore();
const plStore = usePlaylistStore();

const addMenuTrackId = ref<number | null>(null);
const parentRef = ref<HTMLDivElement | null>(null);

const tracks = computed(() => view.filteredTracks);

// v3 takes a reactive options object (computed Ref)
const virtualizerOptions = computed(() => ({
  count: tracks.value.length,
  getScrollElement: () => parentRef.value,
  estimateSize: () => 40,
  overscan: 10,
}));

const virtualizer = useVirtualizer(virtualizerOptions);

const virtualRows = computed(() => {
  const v = virtualizer.value;
  if (!v) return [];
  const range = v.getVirtualItems();
  return range.map((item: any) => ({
    ...item,
    _track: tracks.value[item.index],
  }));
});

// ── Column Resizing ──────────────────────────────────────────────────────────

type ResizeHandle = "index-title" | "title-artist" | "artist-album" | "album-duration";

const resizingHandle = ref<ResizeHandle | null>(null);
let startX = 0;
let startWidths = { index: 40, artist: 240, album: 240, duration: 80 };
let hasMoved = false;

function startResize(handle: ResizeHandle, e: MouseEvent) {
  resizingHandle.value = handle;
  startX = e.clientX;
  startWidths = {
    index: view.columnWidths.index,
    artist: view.columnWidths.artist,
    album: view.columnWidths.album,
    duration: view.columnWidths.duration,
  };
  hasMoved = false;

  document.body.style.cursor = "col-resize";
  document.body.style.userSelect = "none";

  window.addEventListener("mousemove", onMouseMove);
  window.addEventListener("mouseup", onMouseUp);
}

function onMouseMove(e: MouseEvent) {
  if (!resizingHandle.value) return;
  const dx = e.clientX - startX;
  if (Math.abs(dx) > 2) {
    hasMoved = true;
  }

  const handle = resizingHandle.value;

  if (handle === "index-title") {
    view.columnWidths.index = Math.max(32, Math.min(120, Math.round(startWidths.index + dx)));
  } else if (handle === "title-artist") {
    // Moving right shrinks artist, expanding title
    view.columnWidths.artist = Math.max(80, Math.min(600, Math.round(startWidths.artist - dx)));
  } else if (handle === "artist-album") {
    const minW = 80;
    const maxW = 600;
    const minDx = Math.max(minW - startWidths.artist, startWidths.album - maxW);
    const maxDx = Math.min(maxW - startWidths.artist, startWidths.album - minW);
    const clampedDx = Math.max(minDx, Math.min(maxDx, dx));
    view.columnWidths.artist = Math.round(startWidths.artist + clampedDx);
    view.columnWidths.album = Math.round(startWidths.album - clampedDx);
  } else if (handle === "album-duration") {
    const minAlb = 80;
    const maxAlb = 600;
    const minDur = 60;
    const maxDur = 200;
    const minDx = Math.max(minAlb - startWidths.album, startWidths.duration - maxDur);
    const maxDx = Math.min(maxAlb - startWidths.album, startWidths.duration - minDur);
    const clampedDx = Math.max(minDx, Math.min(maxDx, dx));
    view.columnWidths.album = Math.round(startWidths.album + clampedDx);
    view.columnWidths.duration = Math.round(startWidths.duration - clampedDx);
  }
}

function onMouseUp() {
  if (resizingHandle.value) {
    resizingHandle.value = null;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    window.removeEventListener("mousemove", onMouseMove);
    window.removeEventListener("mouseup", onMouseUp);
    view.saveColumnWidths();
    setTimeout(() => {
      hasMoved = false;
    }, 50);
  }
}

function handleSort(key: any) {
  if (hasMoved || resizingHandle.value) return;
  view.setSort(key);
}

function resetCol(target: "index" | "artist" | "artist-album" | "album-duration") {
  if (target === "index") {
    view.columnWidths.index = 40;
  } else if (target === "artist") {
    view.columnWidths.artist = 240;
  } else if (target === "artist-album") {
    view.columnWidths.artist = 240;
    view.columnWidths.album = 240;
  } else if (target === "album-duration") {
    view.columnWidths.album = 240;
    view.columnWidths.duration = 80;
  }
  view.saveColumnWidths();
}

function rowClasses(track: any) {
  const isCurrent = track.id === playback.currentTrackId;
  const isSelected = view.selectedTrackIds.has(track.id);
  return {
    "bg-primary/10": isCurrent,
    "bg-elevated-hover": isSelected && !isCurrent,
    "hover:bg-elevated-hover": !isCurrent && !isSelected,
    "text-foreground": true,
  };
}

function fmt(sec: number): string {
  if (!sec || sec < 0) return "0:00";
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function handleRowClick(trackId: number, event?: MouseEvent) {
  view.selectTrack(trackId, event);
  addMenuTrackId.value = null;
  closeContextMenu();
}

function toggleAddMenu(trackId: number, event: MouseEvent) {
  event.stopPropagation();
  if (addMenuTrackId.value === trackId) {
    addMenuTrackId.value = null;
  } else {
    addMenuTrackId.value = trackId;
  }
}

function addToPlaylist(playlistId: number, trackId: number) {
  plStore.addTracks(playlistId, [trackId]);
  addMenuTrackId.value = null;
}

// ── Context menu (right-click) ──
const contextMenu = ref<{ x: number; y: number; track: any } | null>(null);
const showContextAddMenu = ref(false);

function showContextMenu(track: any, event: MouseEvent) {
  let x = event.clientX;
  let y = event.clientY;
  const ww = window.innerWidth;
  const wh = window.innerHeight;
  const mw = 176;
  const mh = 150;
  if (x + mw + 4 > ww) x = ww - mw - 4;
  if (y + mh + 4 > wh) y = wh - mh - 4;

  contextMenu.value = { x, y, track };
  showContextAddMenu.value = false;
}

function closeContextMenu() {
  contextMenu.value = null;
  showContextAddMenu.value = false;
}

function playContextTrack() {
  if (contextMenu.value) {
    const ids = tracks.value.map((t: any) => t.id);
    playback.playTracks(ids, contextMenu.value.track.id);
  }
  closeContextMenu();
}

async function playNextContextTrack() {
  if (contextMenu.value) {
    await playback.playNextTrack(contextMenu.value.track.id);
    await playback.fetchQueueState();
  }
  closeContextMenu();
}

function toggleAddFromContext(_event: MouseEvent) {
  showContextAddMenu.value = !showContextAddMenu.value;
}

async function addContextTrackToPlaylist(playlistId: number) {
  if (contextMenu.value) {
    try {
      await plStore.addTracks(playlistId, [contextMenu.value.track.id]);
    } catch (e) {
      console.error("caw: add to playlist failed", e);
    }
  }
  closeContextMenu();
}

// Close menus on click outside
function onClickDocument() {
  closeContextMenu();
}

function onKeyDown(e: KeyboardEvent) {
  if (e.key === "Escape") closeContextMenu();
}

onMounted(() => {
  document.addEventListener("click", onClickDocument);
  document.addEventListener("keydown", onKeyDown);
});

onUnmounted(() => {
  document.removeEventListener("click", onClickDocument);
  document.removeEventListener("keydown", onKeyDown);
  window.removeEventListener("mousemove", onMouseMove);
  window.removeEventListener("mouseup", onMouseUp);
});

// Close menu on list change
watch(
  () => tracks.value.length,
  () => { addMenuTrackId.value = null; }
);
</script>
