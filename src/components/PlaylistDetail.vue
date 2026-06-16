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
          class="px-2 py-1 rounded-md text-xs text-red-400 hover:text-red-300 hover:bg-elevated-hover transition-colors"
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
          class="flex items-center gap-3 px-2 py-2 rounded-md cursor-pointer hover:bg-elevated-hover transition-colors group"
          :class="{ 'bg-primary/10': track.id === playback.currentTrackId }"
          @dblclick="playTrack(track.id)"
        >
          <!-- Drag handle (placeholder for reorder) -->
          <GripVertical class="w-3.5 h-3.5 text-faint-foreground flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity" />

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
            class="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-red-400 transition-all"
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
    <Teleport to="body">
      <div v-if="confirmDelete" class="fixed inset-0 z-50 flex items-center justify-center">
        <div class="absolute inset-0 bg-overlay" @click="confirmDelete = false" />
        <div class="relative bg-elevated rounded-xl border border-border shadow-xl p-6 w-full max-w-sm mx-4 z-10">
          <p class="text-body text-foreground mb-4">确定删除「{{ playlistName }}」？</p>
          <div class="flex justify-end gap-2">
            <button class="px-3 py-1.5 rounded-md text-sm text-muted-foreground hover:text-foreground transition-colors" @click="confirmDelete = false">取消</button>
            <button class="px-3 py-1.5 rounded-md text-sm bg-red-500 text-white hover:opacity-90 transition-opacity" @click="handleDelete">删除</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { ArrowLeft, Play, Pencil, Trash2, X, ListMusic, GripVertical } from "lucide-vue-next";
import { usePlaylistStore } from "@/stores/playlists";
import { usePlaybackStore } from "@/stores/playback";
import { useViewStore } from "@/stores/view";
import PlaylistDialog from "@/components/PlaylistDialog.vue";

const plStore = usePlaylistStore();
const playback = usePlaybackStore();
const view = useViewStore();

const editingName = ref(false);
const editText = ref("");
const confirmDelete = ref(false);

const playlistName = computed(() => {
  const pl = plStore.playlists.find((p) => p.id === plStore.currentPlaylistId);
  return pl?.name ?? "未知播放列表";
});

const tracksLookup = computed(() => {
  const lib = playback.library;
  const ids = new Set(plStore.currentPlaylistTracks);
  return lib.filter((t) => ids.has(t.id));
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
