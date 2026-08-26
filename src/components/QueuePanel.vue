<script setup lang="ts">
import { ref, computed } from "vue";
import { usePlaybackStore } from "@/stores/playback";
import type { TrackDto } from "@/lib/tauri";
import { formatDuration } from "@/lib/utils";
import {
  X,
  GripVertical,
  Clock,
  ListOrdered,
} from "lucide-vue-next";

defineProps<{ visible: boolean }>();
const emit = defineEmits<{ close: [] }>();

const playback = usePlaybackStore();

const activeTab = ref<"upcoming" | "history">("upcoming");
const dragIdx = ref<number | null>(null);

const libraryMap = computed(() => {
  const map = new Map<number, TrackDto>();
  for (const t of playback.library) {
    map.set(t.id, t);
  }
  return map;
});

const queueCurrentIndex = computed(() => {
  if (playback.currentTrackId === null) return -1;
  return playback.queue.indexOf(playback.currentTrackId);
});

const nowPlaying = computed(() => {
  if (playback.currentTrackId === null) return null;
  return libraryMap.value.get(playback.currentTrackId) ?? null;
});

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

const historyTracks = computed(() => {
  return playback.history
    .map((id) => libraryMap.value.get(id))
    .filter(Boolean) as TrackDto[];
});

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
</script>

<template>
  <Transition name="slide">
    <aside
      v-if="visible"
      class="w-80 border-l border-border/80 bg-elevated/95 backdrop-blur-md flex flex-col z-30 select-none shadow-3 relative"
    >
      <!-- Header -->
      <div class="h-12 px-4 border-b border-border/70 flex items-center justify-between flex-shrink-0">
        <div class="flex items-center gap-2">
          <ListOrdered class="w-4 h-4 text-primary" />
          <h2 class="text-sm font-bold text-foreground">播放队列</h2>
          <span class="text-caption text-faint-foreground px-1.5 py-0.5 rounded-full bg-elevated-hover">
            {{ playback.queue.length }}
          </span>
        </div>

        <button
          class="w-7 h-7 rounded-lg flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors"
          @click="emit('close')"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Tabs: Upcoming / History -->
      <div class="flex p-2 gap-1 border-b border-border/60 bg-background/40">
        <button
          class="flex-1 py-1.5 text-xs font-medium rounded-md transition-colors"
          :class="
            activeTab === 'upcoming'
              ? 'bg-elevated text-primary font-semibold shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          "
          @click="activeTab = 'upcoming'"
        >
          即将播放 ({{ upcomingTracks.length }})
        </button>
        <button
          class="flex-1 py-1.5 text-xs font-medium rounded-md transition-colors"
          :class="
            activeTab === 'history'
              ? 'bg-elevated text-primary font-semibold shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          "
          @click="activeTab = 'history'"
        >
          历史记录 ({{ historyTracks.length }})
        </button>
      </div>

      <!-- Now Playing Section (if upcoming active) -->
      <div v-if="activeTab === 'upcoming' && nowPlaying" class="p-3 border-b border-border/60 bg-primary/5">
        <p class="text-caption text-primary font-bold uppercase tracking-wider mb-2">正在播放</p>
        <div class="flex items-center gap-2.5">
          <div class="w-3 flex justify-center">
            <div class="flex items-center gap-0.5 h-3">
              <span class="w-0.5 h-2 bg-primary rounded-full eq-bar-1" />
              <span class="w-0.5 h-3 bg-primary rounded-full eq-bar-2" />
              <span class="w-0.5 h-1.5 bg-primary rounded-full eq-bar-3" />
            </div>
          </div>
          <div class="flex-1 min-w-0">
            <p class="text-xs font-semibold text-foreground truncate">{{ nowPlaying.title }}</p>
            <p class="text-caption text-muted-foreground truncate">{{ nowPlaying.artist }}</p>
          </div>
          <span class="text-caption text-muted-foreground font-mono">
            {{ formatDuration(nowPlaying.duration_secs) }}
          </span>
        </div>
      </div>

      <!-- Queue list items -->
      <div class="flex-1 overflow-y-auto p-2 space-y-0.5">
        <template v-if="activeTab === 'upcoming'">
          <div v-if="upcomingTracks.length === 0" class="py-12 text-center text-xs text-muted-foreground">
            暂无后续播放曲目
          </div>
          <div
            v-for="(track, idx) in upcomingTracks"
            :key="track.id"
            draggable="true"
            class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg cursor-pointer hover:bg-elevated-hover transition-colors group"
            @dblclick="playback.playTracks(playback.queue, track.id)"
            @dragstart="onDragStart($event, idx)"
            @dragover.prevent
            @drop="onDrop($event, idx)"
            @dragend="onDragEnd"
          >
            <GripVertical class="w-3.5 h-3.5 text-faint-foreground opacity-0 group-hover:opacity-100 cursor-grab flex-shrink-0" />
            <div class="flex-1 min-w-0">
              <p class="text-xs font-medium text-foreground truncate">{{ track.title }}</p>
              <p class="text-caption text-muted-foreground truncate">{{ track.artist }}</p>
            </div>
            <span class="text-caption text-muted-foreground font-mono flex-shrink-0">
              {{ formatDuration(track.duration_secs) }}
            </span>
          </div>
        </template>

        <template v-else>
          <div v-if="historyTracks.length === 0" class="py-12 text-center text-xs text-muted-foreground">
            暂无播放历史
          </div>
          <div
            v-for="track in historyTracks"
            :key="track.id"
            class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg cursor-pointer hover:bg-elevated-hover transition-colors"
            @dblclick="playback.playTracks([track.id], track.id)"
          >
            <Clock class="w-3 h-3 text-faint-foreground flex-shrink-0" />
            <div class="flex-1 min-w-0">
              <p class="text-xs font-medium text-foreground truncate">{{ track.title }}</p>
              <p class="text-caption text-muted-foreground truncate">{{ track.artist }}</p>
            </div>
            <span class="text-caption text-muted-foreground font-mono flex-shrink-0">
              {{ formatDuration(track.duration_secs) }}
            </span>
          </div>
        </template>
      </div>
    </aside>
  </Transition>
</template>

<style scoped>
.slide-enter-active,
.slide-leave-active {
  transition: transform 0.25s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.25s ease;
}
.slide-enter-from,
.slide-leave-to {
  transform: translateX(100%);
  opacity: 0;
}
</style>
