<template>
  <div class="flex flex-col h-full bg-background select-none">
    <!-- Header Hero -->
    <div class="flex items-center justify-between p-6 pb-5 border-b border-border/60 flex-shrink-0 bg-elevated/30">
      <div class="flex items-center gap-4">
        <!-- Back button -->
        <button
          class="w-9 h-9 rounded-full flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-elevated transition-colors"
          title="返回播放列表"
          @click="plStore.closePlaylist(); view.setNav('playlists')"
        >
          <ArrowLeft class="w-5 h-5" />
        </button>

        <!-- Playlist Icon / Cover Art -->
        <div
          class="w-14 h-14 rounded-xl flex items-center justify-center text-white shadow-sm flex-shrink-0"
          :style="{ background: coverGradient(playlistName) }"
        >
          <ListMusic class="w-7 h-7" />
        </div>

        <div>
          <div class="flex items-center gap-2">
            <span class="text-overline px-2 py-0.5 rounded-full bg-primary/10 text-primary">播放列表</span>
            <span class="text-xs text-muted-foreground">{{ plTracks.length }} 首歌曲</span>
          </div>
          <h1 class="text-title text-foreground font-bold mt-0.5">{{ playlistName }}</h1>
        </div>
      </div>

      <!-- Action buttons -->
      <div class="flex items-center gap-2">
        <button
          class="h-9 px-3 rounded-lg text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors flex items-center gap-1.5 border border-border/60"
          title="重命名播放列表"
          @click="editingName = true; editText = playlistName"
        >
          <Pencil class="w-3.5 h-3.5" />
          <span>重命名</span>
        </button>

        <button
          class="h-9 px-3 rounded-lg text-xs font-medium text-destructive hover:text-destructive-hover hover:bg-destructive/10 transition-colors flex items-center gap-1.5 border border-destructive/20"
          title="删除播放列表"
          @click="confirmDelete = true"
        >
          <Trash2 class="w-3.5 h-3.5" />
          <span>删除</span>
        </button>

        <button
          class="h-10 px-5 rounded-full bg-primary text-primary-foreground flex items-center gap-2 font-medium shadow-md hover:scale-105 active:scale-95 hover:bg-primary-hover transition-all ml-1"
          @click="playPlaylist"
        >
          <Play class="w-4 h-4 fill-primary-foreground" />
          <span>播放全部</span>
        </button>
      </div>
    </div>

    <!-- Track list -->
    <div class="flex-1 overflow-y-auto px-4 py-2">
      <div v-if="plTracks.length === 0" class="flex flex-col items-center justify-center h-full gap-3 py-12">
        <div class="w-16 h-16 rounded-2xl bg-elevated flex items-center justify-center border border-border/60">
          <ListMusic class="w-8 h-8 text-muted-foreground/50" />
        </div>
        <p class="text-body font-medium text-muted-foreground">播放列表为空</p>
        <p class="text-xs text-faint-foreground">在全部音乐、专辑或艺术家详情中将歌曲添加到此播放列表</p>
      </div>

      <div v-else class="space-y-1">
        <div
          v-for="(track, idx) in plTracks"
          :key="track.id"
          draggable="true"
          class="flex items-center gap-3 px-3 py-2 rounded-lg cursor-pointer hover:bg-elevated-hover/80 transition-colors group"
          :class="{
            'bg-primary/10 text-foreground font-medium': track.id === playback.currentTrackId,
            'ring-1 ring-primary ring-inset': dropIndex === idx
          }"
          @dblclick="playTrack(track.id)"
          @dragstart="onDragStart($event, idx)"
          @dragover="onDragOver($event, idx)"
          @drop="onDrop($event, idx)"
          @dragend="onDragEnd"
        >
          <!-- Drag handle -->
          <GripVertical class="w-3.5 h-3.5 text-faint-foreground flex-shrink-0 cursor-grab active:cursor-grabbing opacity-0 group-hover:opacity-100 transition-opacity" />

          <!-- Playing indicator or Index -->
          <div class="w-6 flex justify-center flex-shrink-0">
            <div
              v-if="track.id === playback.currentTrackId && playback.isPlaying"
              class="flex items-center gap-0.5 h-3.5"
            >
              <span class="w-0.5 h-2.5 bg-primary rounded-full eq-bar-1" />
              <span class="w-0.5 h-3.5 bg-primary rounded-full eq-bar-2" />
              <span class="w-0.5 h-2 bg-primary rounded-full eq-bar-3" />
            </div>
            <Play v-else-if="track.id === playback.currentTrackId" class="w-3.5 h-3.5 text-primary fill-primary" />
            <span v-else class="text-caption text-faint-foreground group-hover:text-foreground font-mono">{{ idx + 1 }}</span>
          </div>

          <!-- Info -->
          <div class="flex-1 min-w-0">
            <p class="text-body truncate" :class="{ 'text-primary font-semibold': track.id === playback.currentTrackId }">
              {{ track.title }}
            </p>
            <p class="text-xs text-muted-foreground truncate">{{ track.artist }} — {{ track.album }}</p>
          </div>

          <!-- Duration + remove -->
          <span class="text-xs text-muted-foreground tabular-nums flex-shrink-0 mr-2 font-mono">
            {{ formatDuration(track.duration_secs) }}
          </span>

          <button
            class="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive transition-all p-1 rounded-md"
            title="从列表中移除"
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
      :description="`确定删除「${playlistName}」？此操作不会删除本地音乐文件。`"
      confirm-text="删除"
      destructive
      @close="confirmDelete = false"
      @confirm="handleDelete"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { ArrowLeft, Play, Pencil, Trash2, ListMusic, GripVertical, X } from "lucide-vue-next";
import { usePlaylistStore } from "@/stores/playlists";
import { usePlaybackStore } from "@/stores/playback";
import { useViewStore } from "@/stores/view";
import { coverGradient, formatDuration } from "@/lib/utils";
import PlaylistDialog from "@/components/PlaylistDialog.vue";
import ConfirmDialog from "@/components/ui/ConfirmDialog.vue";

const plStore = usePlaylistStore();
const playback = usePlaybackStore();
const view = useViewStore();

const editingName = ref(false);
const editText = ref("");
const confirmDelete = ref(false);
const dropIndex = ref<number | null>(null);
const dragIndex = ref<number | null>(null);

const playlistName = computed(() => {
  return plStore.playlists.find((p) => p.id === plStore.currentPlaylistId)?.name ?? "播放列表";
});

const plTracks = computed(() => {
  const map = new Map(playback.library.map((t) => [t.id, t]));
  return plStore.currentPlaylistTracks
    .map((id) => map.get(id))
    .filter(Boolean) as (typeof playback.library);
});

function playPlaylist() {
  const ids = plStore.currentPlaylistTracks;
  if (ids.length > 0) playback.playTracks(ids, ids[0]);
}

function playTrack(trackId: number) {
  const ids = plStore.currentPlaylistTracks;
  playback.playTracks(ids, trackId);
}

function removeTrack(trackId: number) {
  if (plStore.currentPlaylistId !== null) {
    plStore.removeTracks(plStore.currentPlaylistId, [trackId]);
  }
}

async function handleRename(name: string) {
  editingName.value = false;
  if (!name.trim() || plStore.currentPlaylistId === null) return;
  await plStore.rename(plStore.currentPlaylistId, name.trim());
}

async function handleDelete() {
  confirmDelete.value = false;
  if (plStore.currentPlaylistId !== null) {
    const id = plStore.currentPlaylistId;
    plStore.closePlaylist();
    view.setNav("playlists");
    await plStore.remove(id);
  }
}

// Drag & drop reorder
function onDragStart(e: DragEvent, idx: number) {
  dragIndex.value = idx;
  e.dataTransfer!.effectAllowed = "move";
}

function onDragOver(e: DragEvent, idx: number) {
  e.preventDefault();
  dropIndex.value = idx;
}

function onDrop(e: DragEvent, idx: number) {
  e.preventDefault();
  if (dragIndex.value !== null && dragIndex.value !== idx && plStore.currentPlaylistId !== null) {
    const trackId = plStore.currentPlaylistTracks[dragIndex.value];
    if (trackId !== undefined) {
      plStore.reorder(plStore.currentPlaylistId, trackId, idx);
    }
  }
  dropIndex.value = null;
  dragIndex.value = null;
}

function onDragEnd() {
  dropIndex.value = null;
  dragIndex.value = null;
}
</script>
