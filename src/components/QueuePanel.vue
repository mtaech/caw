<script setup lang="ts">
import { ref, computed } from "vue";
import { usePlaybackStore } from "@/stores/playback";
import PlaylistDialog from "@/components/PlaylistDialog.vue";
import type { TrackDto } from "@/lib/tauri";
import {
  X,
  Play,
  GripVertical,
  Clock,
  ChevronDown,
  ListOrdered,
} from "lucide-vue-next";

defineProps<{ visible: boolean }>();
defineEmits<{ close: [] }>();

const playback = usePlaybackStore();

const showHistory = ref(true);
const showSaveDialog = ref(false);
const dragIdx = ref<number | null>(null);

// Map for O(1) track lookup
const libraryMap = computed(() => {
  const map = new Map<number, TrackDto>();
  for (const t of playback.library) {
    map.set(t.id, t);
  }
  return map;
});

// Index of current track in the full queue (-1 if not found or null)
const queueCurrentIndex = computed(() => {
  if (playback.currentTrackId === null) return -1;
  return playback.queue.indexOf(playback.currentTrackId);
});

// Currently playing track (null if nothing playing)
const nowPlaying = computed(() => {
  if (playback.currentTrackId === null) return null;
  return libraryMap.value.get(playback.currentTrackId) ?? null;
});

// Upcoming tracks (everything after current index in the full queue)
const upcomingTracks = computed(() => {
  const q = playback.queue;
  const curIdx = queueCurrentIndex.value;
  if (curIdx === -1) {
    return q.map((id) => libraryMap.value.get(id)).filter(Boolean) as TrackDto[];
  }
  return q
    .slice(curIdx + 1)
    .map((id) => libraryMap.value.get(id))
    .filter(Boolean) as TrackDto[];
});

// History tracks (most recent first)
const historyTracks = computed(() => {
  return playback.history
    .map((id) => libraryMap.value.get(id))
    .filter(Boolean) as TrackDto[];
});

// Total queue length (for header count)
const queueCount = computed(() => playback.queue.length);

// ── Drag reorder (maps upcoming-tracks slice index to full-queue index) ──

function sliceIdxToFullIdx(sliceIndex: number): number {
  const ci = queueCurrentIndex.value;
  return (ci >= 0 ? ci + 1 : 0) + sliceIndex;
}

function onDragStart(e: DragEvent, sliceIndex: number) {
  const fullIdx = sliceIdxToFullIdx(sliceIndex);
  e.dataTransfer!.setData("text/plain", String(fullIdx));
  e.dataTransfer!.effectAllowed = "move";
  dragIdx.value = fullIdx;
}

function onDrop(e: DragEvent, sliceIndex: number) {
  e.preventDefault();
  const from = dragIdx.value;
  if (from === null) return;
  dragIdx.value = null;
  const to = sliceIdxToFullIdx(sliceIndex);
  if (from !== to) {
    playback.reorderQueue(from, to);
  }
}

function onDragEnd() {
  dragIdx.value = null;
}

// ── Save-as-playlist ──

async function onSaveConfirm(name: string) {
  showSaveDialog.value = false;
  try {
    await playback.saveQueueAsPlaylist(name);
  } catch (e) {
    console.error("caw: save queue as playlist failed", e);
  }
}
</script>

<template>
  <Transition name="slide-right">
    <aside
      v-if="visible"
      class="w-72 flex-shrink-0 bg-elevated border-l border-border flex flex-col h-full overflow-hidden"
    >
      <!-- Header -->
      <div
        class="flex items-center justify-between px-4 py-2 border-b border-border flex-shrink-0 gap-2"
      >
        <div class="flex items-center gap-2 min-w-0">
          <ListOrdered class="w-4 h-4 text-muted-foreground flex-shrink-0" />
          <h2 class="text-body-md text-foreground font-medium truncate">播放队列</h2>
          <span class="text-caption text-muted-foreground flex-shrink-0">{{ queueCount }}</span>
        </div>
        <div class="flex items-center gap-1 flex-shrink-0">
          <button
            v-if="upcomingTracks.length > 0"
            class="text-caption text-muted-foreground hover:text-destructive transition-colors px-1"
            title="清空队列"
            @click="playback.clearQueue()"
          >
            清空
          </button>
          <button
            class="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors"
            title="关闭队列面板"
            @click="$emit('close')"
          >
            <X class="w-4 h-4" />
          </button>
        </div>
      </div>

      <!-- Queue content -->
      <div class="flex-1 overflow-y-auto">
        <!-- Currently playing -->
        <div v-if="nowPlaying" class="px-3 py-2 border-b border-border/40">
          <p class="text-caption text-muted-foreground mb-1.5 font-medium">正在播放</p>
          <div class="flex items-center gap-2 p-2 rounded-lg bg-primary/10">
            <Play class="w-3 h-3 text-primary fill-primary flex-shrink-0" />
            <div class="min-w-0">
              <p class="text-body-sm text-foreground truncate">{{ nowPlaying.title }}</p>
              <p class="text-caption text-muted-foreground truncate">{{ nowPlaying.artist }}</p>
            </div>
          </div>
        </div>

        <!-- Upcoming section header -->
        <div
          v-if="upcomingTracks.length > 0"
          class="flex items-center justify-between px-3 pt-2 pb-1"
        >
          <p class="text-caption text-muted-foreground font-medium">接下来播放</p>
        </div>

        <!-- Upcoming tracks (draggable) -->
        <div v-if="upcomingTracks.length > 0" class="px-1">
          <div
            v-for="(track, idx) in upcomingTracks"
            :key="track.id"
            class="group flex items-center gap-2 px-2 py-1.5 rounded-md cursor-grab active:cursor-grabbing hover:bg-elevated-hover transition-colors"
            :class="{ 'opacity-30': dragIdx === sliceIdxToFullIdx(idx) }"
            draggable="true"
            @dragstart="onDragStart($event, idx)"
            @dragover.prevent
            @drop="onDrop($event, idx)"
            @dragend="onDragEnd"
          >
            <GripVertical
              class="w-3 h-3 text-faint-foreground flex-shrink-0 opacity-50 group-hover:opacity-100 transition-opacity"
            />
            <div class="min-w-0 flex-1">
              <p class="text-body-sm text-foreground truncate">{{ track.title }}</p>
              <p class="text-caption text-muted-foreground truncate">{{ track.artist }}</p>
            </div>
            <button
              class="p-0.5 rounded opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive transition-all flex-shrink-0"
              @click.stop="playback.removeFromQueue(sliceIdxToFullIdx(idx))"
              title="从队列移除"
            >
              <X class="w-3 h-3" />
            </button>
          </div>
        </div>

        <!-- Empty upcoming state -->
        <div
          v-else-if="!nowPlaying"
          class="flex items-center justify-center py-8"
        >
          <p class="text-body-sm text-muted-foreground">队列为空</p>
        </div>

        <!-- History section -->
        <div v-if="historyTracks.length > 0" class="border-t border-border mt-2">
          <button
            class="flex items-center justify-between w-full px-3 py-2 text-caption text-muted-foreground font-medium hover:text-foreground transition-colors"
            @click="showHistory = !showHistory"
          >
            <span>播放历史 ({{ historyTracks.length }})</span>
            <ChevronDown
              class="w-3.5 h-3.5 transition-transform duration-200"
              :class="{ 'rotate-180': !showHistory }"
            />
          </button>
          <div v-if="showHistory" class="px-1 pb-2">
            <div
              v-for="(track, idx) in historyTracks"
              :key="track.id + '-hist-' + idx"
              class="flex items-center gap-2 px-2 py-1.5 rounded-md"
            >
              <Clock class="w-3 h-3 text-faint-foreground flex-shrink-0" />
              <div class="min-w-0 flex-1">
                <p class="text-body-sm text-foreground truncate">{{ track.title }}</p>
                <p class="text-caption text-muted-foreground truncate">{{ track.artist }}</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Bottom: save button -->
      <div class="px-4 py-3 border-t border-border flex-shrink-0">
        <button
          class="w-full px-3 py-1.5 rounded-md text-sm bg-primary text-background hover:bg-primary-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="upcomingTracks.length === 0"
          @click="showSaveDialog = true"
        >
          保存为播放列表
        </button>
      </div>
    </aside>
  </Transition>

  <!-- Save-as-playlist dialog -->
  <PlaylistDialog
    :open="showSaveDialog"
    title="保存队列为播放列表"
    placeholder="播放列表名称"
    confirm-text="保存"
    :initial-text="'队列 ' + new Date().toLocaleDateString('zh-CN')"
    @close="showSaveDialog = false"
    @confirm="onSaveConfirm"
  />
</template>

<style>
/* ── Slide-in from right transition (global, needed for Vue Transition) ── */
.slide-right-enter-active,
.slide-right-leave-active {
  transition:
    transform 0.2s ease,
    opacity 0.2s ease;
}
.slide-right-enter-from,
.slide-right-leave-to {
  transform: translateX(100%);
  opacity: 0;
}
</style>
