<template>
  <div class="flex flex-col h-full">
    <!-- Header -->
    <div class="flex items-center justify-between px-6 py-4 border-b border-border flex-shrink-0">
      <div class="flex items-center gap-3">
        <button
          class="text-muted-foreground hover:text-foreground transition-colors"
          @click="plStore.closePlaylist(); view.setNav('playlists')"
        >
          <ArrowLeft class="w-5 h-5" />
        </button>
        <div>
          <h1 class="text-title text-foreground">{{ playlistName }}</h1>
          <p class="text-body-sm text-muted-foreground">{{ plStore.currentPlaylistTracks.length }} tracks</p>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <button
          class="px-2 py-1 rounded-md text-xs text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors"
          @click="editingName = true; editText = playlistName"
        >
          <Pencil class="w-3.5 h-3.5" />
        </button>
        <button
          class="px-2 py-1 rounded-md text-xs text-destructive hover:text-destructive-hover hover:bg-destructive/10 transition-colors"
          @click="confirmDelete = true"
        >
          <Trash2 class="w-3.5 h-3.5" />
        </button>
        <button
          class="w-10 h-10 rounded-full bg-primary text-background flex items-center justify-center hover:scale-105 transition-transform"
          @click="playPlaylist"
        >
          <Play class="w-4 h-4 ml-0.5 fill-background" />
        </button>
      </div>
    </div>

    <!-- Track list (simple, not virtualized — playlists tend to be smaller) -->
    <div class="flex-1 overflow-auto">
      <div v-if="tracks.length === 0" class="flex flex-col items-center justify-center h-full gap-2">
        <ListMusic class="w-10 h-10 text-faint-foreground" />
        <p class="text-body text-muted-foreground">播放列表为空</p>
        <p class="text-body-sm text-faint-foreground">在曲目录中添加曲目</p>
      </div>
      <div v-else class="px-4">
        <div
          v-for="(track, idx) in tracks"
          :key="track.id"
          draggable="true"
          class="flex items-center gap-3 px-2 py-2 rounded-md cursor-pointer hover:bg-elevated-hover transition-colors group"
          :class="{ 'bg-primary/10': track.id === playback.currentTrackId, 'ring-1 ring-primary ring-inset': dropIndex === idx }"
          @dblclick="playTrack(track.id)"
          @dragstart="onDragStart($event, idx)"
          @dragover="onDragOver($event, idx)"
          @drop="onDrop($event, idx)"
          @dragend="onDragEnd"
        >
          <!-- Drag handle -->
          <GripVertical class="w-3.5 h-3.5 text-faint-foreground flex-shrink-0 cursor-grab active:cursor-grabbing transition-opacity" />

          <!-- Playing indicator -->
          <Play v-if="track.id === playback.currentTrackId && playback.isPlaying" class="w-3 h-3 text-primary fill-primary flex-shrink-0" />
          <span v-else class="text-caption text-faint-foreground w-3 text-center flex-shrink-0">{{ idx + 1 }}</span>

          <!-- Info -->
          <div class="flex-1 min-w-0">
            <p class="text-body truncate" :class="{ 'text-body-md text-primary': track.id === playback.currentTrackId }">
              {{ track.title }}
            </p>
            <p class="text-body-sm text-muted-foreground truncate">{{ track.artist }} — {{ track.album }}</p>
          </div>

          <!-- Duration + remove -->
          <span class="text-body-sm text-muted-foreground tabular-nums flex-shrink-0 mr-2">{{ fmt(track.duration_secs) }}</span>
          <button
            class="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive transition-all"
            @click.stop="removeTrack(track.id)"
          >
            <X class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>

    <!-- Rename dialog -->
    <PlaylistDialog
      :open="editingName"
      title="重命名播放列表"
      :initial-text="editText"
      confirm-text="保存"
      @close="editingName = false"
      @confirm="handleRename"
    />

    <!-- Delete confirmation -->
    <ConfirmDialog
      :open="confirmDelete"
      title="删除播放列表"
      :description="`确定删除「${playlistName}」？`"
      confirm-text="删除"
      destructive
      @close="confirmDelete = false"
      @confirm="handleDelete"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { ArrowLeft, Play, Pencil, Trash2, X, ListMusic, GripVertical } from "lucide-vue-next";
import { usePlaylistStore } from "@/stores/playlists";
import { usePlaybackStore } from "@/stores/playback";
import { useViewStore } from "@/stores/view";
import type { TrackDto } from "@/lib/tauri";
import PlaylistDialog from "@/components/PlaylistDialog.vue";
import ConfirmDialog from "@/components/ui/ConfirmDialog.vue";

const plStore = usePlaylistStore();
const playback = usePlaybackStore();
const view = useViewStore();

const editingName = ref(false);
const editText = ref("");
const confirmDelete = ref(false);

// ── Drag-to-reorder ──
const dragIndex = ref<number | null>(null);
const dropIndex = ref<number | null>(null);

function onDragStart(e: DragEvent, idx: number) {
  dragIndex.value = idx;
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", String(idx));
  }
}

function onDragOver(e: DragEvent, idx: number) {
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
  if (dragIndex.value !== null && dragIndex.value !== idx) {
    dropIndex.value = idx;
  }
}

async function onDrop(e: DragEvent, idx: number) {
  e.preventDefault();
  const from = dragIndex.value;
  dragIndex.value = null;
  dropIndex.value = null;
  if (from === null || from === idx) return;
  const pl = plStore.currentPlaylistId;
  if (pl === null) return;
  const trackId = tracks.value[from].id;
  // Backend position is 1-indexed.
  await plStore.reorder(pl, trackId, idx + 1);
}

function onDragEnd() {
  dragIndex.value = null;
  dropIndex.value = null;
}

const playlistName = computed(() => {
  const pl = plStore.playlists.find((p) => p.id === plStore.currentPlaylistId);
  return pl?.name ?? "未知播放列表";
});

const tracksLookup = computed(() => {
  // currentPlaylistTracks is the ordered array of track IDs (playlist order).
  // Map them back to track objects IN playlist order — filtering the library
  // by membership would discard the order and make reorder appear to do nothing.
  const byId = new Map(playback.library.map((t) => [t.id, t]));
  return plStore.currentPlaylistTracks
    .map((id) => byId.get(id))
    .filter((t): t is TrackDto => Boolean(t));
});

// Override filteredTracks to show playlist tracks when in playlist detail
const tracks = computed(() => {
  return tracksLookup.value;
});

function fmt(sec: number): string {
  if (!sec || sec < 0) return "0:00";
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function playPlaylist() {
  const ids = tracks.value.map((t) => t.id);
  if (ids.length > 0) {
    playback.playTracks(ids, ids[0]);
  }
}

function playTrack(id: number) {
  const ids = tracks.value.map((t) => t.id);
  playback.playTracks(ids, id);
}

async function removeTrack(id: number) {
  if (plStore.currentPlaylistId)
    await plStore.removeTracks(plStore.currentPlaylistId, [id]);
}

async function handleRename(name: string) {
  if (plStore.currentPlaylistId)
    await plStore.rename(plStore.currentPlaylistId, name);
  editingName.value = false;
}

async function handleDelete() {
  confirmDelete.value = false;
  await plStore.remove(plStore.currentPlaylistId!);
}
</script>
