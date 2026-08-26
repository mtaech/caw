<template>
  <div class="flex flex-col h-full select-none relative bg-background">
    <!-- Sticky Table Header -->
    <div
      class="flex items-center border-b border-border/70 px-4 h-9 text-caption text-muted-foreground uppercase tracking-wider flex-shrink-0 bg-elevated/40 backdrop-blur-sm z-10"
    >
      <!-- # / Track Number -->
      <div
        class="relative flex items-center justify-center flex-shrink-0 cursor-pointer hover:text-foreground transition-colors h-full"
        :style="{ width: view.columnWidths.index + 'px' }"
        :class="{ 'text-primary font-semibold': view.sortKey === 'track_number' }"
        @click="view.setSort('track_number')"
      >
        <span>#</span>
        <template v-if="view.sortKey === 'track_number'">
          <ChevronUp v-if="view.sortDir === 'asc'" class="w-3 h-3 ml-0.5 text-primary" />
          <ChevronDown v-else class="w-3 h-3 ml-0.5 text-primary" />
        </template>
        <!-- Resize handle: Index / Title -->
        <div
          class="absolute -right-1 top-0 bottom-0 w-2 cursor-col-resize flex items-center justify-center z-10 group/handle"
          title="拖动调整列宽，双击重置"
          @mousedown.stop.prevent="startResize('index-title', $event)"
          @dblclick.stop.prevent="view.resetColumnWidths()"
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
        :class="{ 'text-primary font-semibold': view.sortKey === 'title' }"
        @click="view.setSort('title')"
      >
        <span class="truncate">标题</span>
        <template v-if="view.sortKey === 'title'">
          <ChevronUp v-if="view.sortDir === 'asc'" class="w-3 h-3 flex-shrink-0 text-primary" />
          <ChevronDown v-else class="w-3 h-3 flex-shrink-0 text-primary" />
        </template>
        <!-- Resize handle: Title / Artist -->
        <div
          class="absolute -right-1 top-0 bottom-0 w-2 cursor-col-resize flex items-center justify-center z-10 group/handle"
          title="拖动调整列宽，双击重置"
          @mousedown.stop.prevent="startResize('title-artist', $event)"
          @dblclick.stop.prevent="view.resetColumnWidths()"
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
        :class="{ 'text-primary font-semibold': view.sortKey === 'artist' }"
        @click="view.setSort('artist')"
      >
        <span class="truncate">艺人</span>
        <template v-if="view.sortKey === 'artist'">
          <ChevronUp v-if="view.sortDir === 'asc'" class="w-3 h-3 flex-shrink-0 text-primary" />
          <ChevronDown v-else class="w-3 h-3 flex-shrink-0 text-primary" />
        </template>
        <!-- Resize handle: Artist / Album -->
        <div
          class="absolute -right-1 top-0 bottom-0 w-2 cursor-col-resize flex items-center justify-center z-10 group/handle"
          title="拖动调整列宽，双击重置"
          @mousedown.stop.prevent="startResize('artist-album', $event)"
          @dblclick.stop.prevent="view.resetColumnWidths()"
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
        :class="{ 'text-primary font-semibold': view.sortKey === 'album' }"
        @click="view.setSort('album')"
      >
        <span class="truncate">专辑</span>
        <template v-if="view.sortKey === 'album'">
          <ChevronUp v-if="view.sortDir === 'asc'" class="w-3 h-3 flex-shrink-0 text-primary" />
          <ChevronDown v-else class="w-3 h-3 flex-shrink-0 text-primary" />
        </template>
        <!-- Resize handle: Album / Duration -->
        <div
          class="absolute -right-1 top-0 bottom-0 w-2 cursor-col-resize flex items-center justify-center z-10 group/handle"
          title="拖动调整列宽，双击重置"
          @mousedown.stop.prevent="startResize('album-duration', $event)"
          @dblclick.stop.prevent="view.resetColumnWidths()"
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
        class="relative flex items-center gap-1 flex-shrink-0 cursor-pointer hover:text-foreground transition-colors pl-3 pr-2 h-full justify-end"
        :style="{ width: view.columnWidths.duration + 'px' }"
        :class="{ 'text-primary font-semibold': view.sortKey === 'duration' }"
        @click="view.setSort('duration')"
      >
        <span class="truncate">时长</span>
        <template v-if="view.sortKey === 'duration'">
          <ChevronUp v-if="view.sortDir === 'asc'" class="w-3 h-3 flex-shrink-0 text-primary" />
          <ChevronDown v-else class="w-3 h-3 flex-shrink-0 text-primary" />
        </template>
      </div>
    </div>

    <!-- Virtualized rows -->
    <div ref="parentRef" class="flex-1 overflow-auto select-none px-2 py-1" @scroll="onScroll">
      <div v-if="virtualizer" class="relative" :style="{ height: virtualizer.getTotalSize() + 'px' }">
        <div
          v-for="row in virtualRows"
          :key="row.key"
          class="group flex items-center px-2.5 rounded-lg transition-colors duration-120 cursor-pointer absolute top-0 left-0 w-full"
          :style="{
            height: row.size + 'px',
            transform: `translateY(${row.start}px)`,
          }"
          :class="rowClasses(row._track)"
          @dblclick="view.playTrackById(row._track.id)"
          @click="view.selectTrack(row._track.id, $event)"
          @contextmenu.prevent="showContextMenu(row._track, $event)"
        >
          <!-- # / now playing indicator -->
          <div
            class="flex items-center justify-center flex-shrink-0"
            :style="{ width: view.columnWidths.index + 'px' }"
          >
            <!-- Animated Equalizer if playing -->
            <div
              v-if="row._track.id === playback.currentTrackId && playback.isPlaying"
              class="flex items-center gap-0.5 h-3.5"
            >
              <span class="w-0.5 h-2.5 bg-primary rounded-full eq-bar-1" />
              <span class="w-0.5 h-3.5 bg-primary rounded-full eq-bar-2" />
              <span class="w-0.5 h-2 bg-primary rounded-full eq-bar-3" />
            </div>
            <!-- Static Play Icon if paused current track -->
            <Play
              v-else-if="row._track.id === playback.currentTrackId"
              class="w-3.5 h-3.5 text-primary fill-primary"
            />
            <!-- Ordinary Track Index -->
            <span
              v-else
              class="text-caption text-faint-foreground group-hover:text-foreground font-mono"
            >
              {{ row.index + 1 }}
            </span>
          </div>

          <!-- Title -->
          <div
            class="truncate text-body flex-1 min-w-0 pl-3 font-medium"
            :class="{ 'text-primary font-semibold': row._track.id === playback.currentTrackId }"
            :title="row._track.title"
          >
            {{ row._track.title }}
          </div>

          <!-- Artist (Clickable to jump to artist view) -->
          <div
            class="truncate text-body-sm text-muted-foreground hover:text-foreground flex-shrink-0 pl-3 transition-colors"
            :style="{ width: view.columnWidths.artist + 'px' }"
            :title="row._track.artist"
            @click.stop="view.openArtist(row._track.artist)"
          >
            {{ row._track.artist }}
          </div>

          <!-- Album (Clickable to jump to album view) -->
          <div
            class="truncate text-body-sm text-muted-foreground hover:text-foreground flex-shrink-0 pl-3 transition-colors"
            :style="{ width: view.columnWidths.album + 'px' }"
            :title="row._track.album"
            @click.stop="view.openAlbum(row._track.album)"
          >
            {{ row._track.album }}
          </div>

          <!-- Duration + Action Menu Button -->
          <div
            class="flex items-center justify-between flex-shrink-0 pl-3 pr-1"
            :style="{ width: view.columnWidths.duration + 'px' }"
          >
            <span class="text-body-sm text-muted-foreground tabular-nums truncate font-mono text-xs">
              {{ formatDuration(row._track.duration_secs) }}
            </span>
            
            <div class="relative" @click.stop>
              <button
                class="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-primary hover:bg-elevated-hover transition-all p-1 rounded-md flex-shrink-0"
                title="添加到播放列表"
                @click="toggleAddMenu(row._track.id)"
              >
                <Plus class="w-3.5 h-3.5" />
              </button>
              
              <!-- Add-to-playlist dropdown -->
              <div
                v-if="addMenuTrackId === row._track.id"
                class="absolute right-0 top-full mt-1 w-48 bg-elevated border border-border rounded-xl shadow-2 z-dropdown py-1.5 max-h-64 overflow-auto backdrop-blur-md"
              >
                <p v-if="plStore.playlists.length === 0" class="px-3 py-2 text-xs text-muted-foreground">
                  暂无播放列表
                </p>
                <button
                  v-for="pl in plStore.playlists"
                  :key="pl.id"
                  class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-elevated-hover transition-colors truncate flex items-center gap-2"
                  @click.stop="addToPlaylist(pl.id, row._track.id)"
                >
                  <ListMusic class="w-3.5 h-3.5 text-muted-foreground flex-shrink-0" />
                  <span class="truncate">{{ pl.name }}</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Floating locate button in bottom-right -->
    <Transition name="locate-btn">
      <div
        v-if="showLocateButton"
        class="absolute bottom-5 right-5 z-20"
      >
        <Tooltip :delay="150" side="left" :side-offset="8">
          <template #trigger>
            <button
              class="w-10 h-10 rounded-full flex items-center justify-center bg-elevated/90 hover:bg-elevated text-foreground hover:text-primary border border-border shadow-2 backdrop-blur-md transition-all duration-200 hover:scale-105 active:scale-95 cursor-pointer select-none group"
              aria-label="定位到正在播放歌曲"
              @click.stop="scrollToCurrentTrack"
            >
              <LocateFixed class="w-4 h-4 text-primary group-hover:rotate-45 transition-transform duration-300 flex-shrink-0" />
            </button>
          </template>
          <span>定位到正在播放歌曲</span>
        </Tooltip>
      </div>
    </Transition>
  </div>

  <!-- Context menu (right-click) -->
  <div
    v-if="contextMenu"
    class="fixed z-dropdown w-48 bg-elevated border border-border rounded-xl shadow-3 py-1.5 max-h-64 overflow-auto backdrop-blur-md"
    :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
    @click.stop
  >
    <button
      class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-elevated-hover flex items-center gap-2.5 transition-colors"
      @click="playContextTrack()"
    >
      <Play class="w-3.5 h-3.5 text-primary flex-shrink-0" /> 立即播放
    </button>
    <button
      class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-elevated-hover flex items-center gap-2.5 transition-colors"
      @click="playNextContextTrack()"
    >
      <ListOrdered class="w-3.5 h-3.5 text-muted-foreground flex-shrink-0" /> 下一首播放
    </button>
    <div class="border-t border-border/60 my-1" />
    <div class="relative">
      <button
        class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-elevated-hover flex items-center gap-2.5 transition-colors"
        @click.stop="toggleAddFromContext()"
      >
        <Plus class="w-3.5 h-3.5 text-muted-foreground flex-shrink-0" /> 添加到播放列表
      </button>
      <div
        v-if="showContextAddMenu"
        class="absolute left-full top-0 ml-1 w-48 bg-elevated border border-border rounded-xl shadow-3 py-1.5 z-dropdown max-h-64 overflow-auto"
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
          class="w-full text-left px-3 py-1.5 text-sm text-foreground hover:bg-elevated-hover transition-colors truncate flex items-center gap-2"
          @click.stop="addContextTrackToPlaylist(pl.id)"
        >
          <ListMusic class="w-3.5 h-3.5 text-muted-foreground flex-shrink-0" />
          <span class="truncate">{{ pl.name }}</span>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { Play, ChevronUp, ChevronDown, Plus, ListMusic, ListOrdered, LocateFixed } from "lucide-vue-next";
import { usePlaybackStore } from "@/stores/playback";
import { useViewStore } from "@/stores/view";
import { usePlaylistStore } from "@/stores/playlists";
import { formatDuration } from "@/lib/utils";
import Tooltip from "@/components/ui/Tooltip.vue";

const playback = usePlaybackStore();
const view = useViewStore();
const plStore = usePlaylistStore();

const parentRef = ref<HTMLElement | null>(null);

const virtualizerOptions = computed(() => ({
  count: view.filteredTracks.length,
  getScrollElement: () => parentRef.value,
  estimateSize: () => 40,
  overscan: 10,
}));

const virtualizer = useVirtualizer(virtualizerOptions);

const virtualRows = computed(() => {
  const v = virtualizer.value;
  if (!v) return [];
  return v.getVirtualItems().map((item: any) => ({
    ...item,
    _track: view.filteredTracks[item.index],
  }));
});

function rowClasses(track: any) {
  const isPlaying = track.id === playback.currentTrackId;
  const isSelected = view.selectedTrackIds.has(track.id);

  if (isPlaying) {
    return "bg-primary/10 text-foreground hover:bg-primary/15 font-medium";
  }
  if (isSelected) {
    return "bg-elevated-hover text-foreground";
  }
  return "hover:bg-elevated-hover/70 text-foreground";
}

// ── Context menu ──
const contextMenu = ref<{ x: number; y: number; track: any } | null>(null);
const showContextAddMenu = ref(false);

function showContextMenu(track: any, event: MouseEvent) {
  contextMenu.value = {
    x: Math.min(event.clientX, window.innerWidth - 200),
    y: Math.min(event.clientY, window.innerHeight - 200),
    track,
  };
  showContextAddMenu.value = false;
}

function closeContextMenu() {
  contextMenu.value = null;
  showContextAddMenu.value = false;
  addMenuTrackId.value = null;
}

function playContextTrack() {
  if (contextMenu.value) {
    view.playTrackById(contextMenu.value.track.id);
  }
  closeContextMenu();
}

function playNextContextTrack() {
  if (contextMenu.value) {
    playback.playNextTrack(contextMenu.value.track.id);
  }
  closeContextMenu();
}

function toggleAddFromContext() {
  showContextAddMenu.value = !showContextAddMenu.value;
}

async function addContextTrackToPlaylist(playlistId: number) {
  if (contextMenu.value) {
    await plStore.addTracks(playlistId, [contextMenu.value.track.id]);
  }
  closeContextMenu();
}

// ── Add to playlist quick dropdown ──
const addMenuTrackId = ref<number | null>(null);

function toggleAddMenu(trackId: number) {
  if (addMenuTrackId.value === trackId) {
    addMenuTrackId.value = null;
  } else {
    addMenuTrackId.value = trackId;
  }
}

async function addToPlaylist(playlistId: number, trackId: number) {
  await plStore.addTracks(playlistId, [trackId]);
  addMenuTrackId.value = null;
}

// ── Column resizing ──
const resizingHandle = ref<string | null>(null);
let startX = 0;
let initialWidth = 0;

function startResize(handle: string, event: MouseEvent) {
  resizingHandle.value = handle;
  startX = event.clientX;
  if (handle === "index-title") {
    initialWidth = view.columnWidths.index;
  } else if (handle === "title-artist") {
    initialWidth = view.columnWidths.artist;
  } else if (handle === "artist-album") {
    initialWidth = view.columnWidths.album;
  } else if (handle === "album-duration") {
    initialWidth = view.columnWidths.duration;
  }

  const onMouseMove = (e: MouseEvent) => {
    const delta = e.clientX - startX;
    if (handle === "index-title") {
      view.columnWidths.index = Math.max(32, Math.min(80, initialWidth + delta));
    } else if (handle === "title-artist") {
      view.columnWidths.artist = Math.max(100, Math.min(400, initialWidth - delta));
    } else if (handle === "artist-album") {
      view.columnWidths.album = Math.max(100, Math.min(400, initialWidth - delta));
    } else if (handle === "album-duration") {
      view.columnWidths.duration = Math.max(60, Math.min(150, initialWidth - delta));
    }
  };

  const onMouseUp = () => {
    resizingHandle.value = null;
    view.saveColumnWidths();
    window.removeEventListener("mousemove", onMouseMove);
    window.removeEventListener("mouseup", onMouseUp);
  };

  window.addEventListener("mousemove", onMouseMove);
  window.addEventListener("mouseup", onMouseUp);
}

// ── Locate button ──
const showLocateButton = ref(false);

function onScroll() {
  if (!parentRef.value || !playback.currentTrackId) {
    showLocateButton.value = false;
    return;
  }
  const currentIndex = view.filteredTracks.findIndex((t) => t.id === playback.currentTrackId);
  if (currentIndex === -1) {
    showLocateButton.value = false;
    return;
  }
  const v = virtualizer.value;
  if (!v) return;
  const items = v.getVirtualItems();
  if (items.length === 0) return;
  const firstVisible = items[0].index;
  const lastVisible = items[items.length - 1].index;
  showLocateButton.value = currentIndex < firstVisible || currentIndex > lastVisible;
}

function scrollToCurrentTrack() {
  if (!playback.currentTrackId) return;
  const idx = view.filteredTracks.findIndex((t) => t.id === playback.currentTrackId);
  if (idx !== -1 && virtualizer.value) {
    virtualizer.value.scrollToIndex(idx, { align: "center", behavior: "smooth" });
  }
}

onMounted(() => {
  window.addEventListener("click", closeContextMenu);
});

onUnmounted(() => {
  window.removeEventListener("click", closeContextMenu);
});
</script>