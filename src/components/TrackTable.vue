<template>
  <div class="flex flex-col h-full">
    <!-- Header -->
    <div class="flex items-center border-b border-border px-4 h-9 text-caption text-muted-foreground uppercase tracking-wider flex-shrink-0">
      <div
        v-for="col in columns"
        :key="col.key"
        :style="{ width: col.width + 'px' }"
        class="flex items-center gap-1 cursor-pointer select-none hover:text-foreground transition-colors"
        :class="{ 'text-foreground': view.sortKey === col.key }"
        @click="view.setSort(col.key as any)"
      >
        <span>{{ col.label }}</span>
        <template v-if="view.sortKey === col.key">
          <ChevronUp v-if="view.sortDir === 'asc'" class="w-3 h-3" />
          <ChevronDown v-else class="w-3 h-3" />
        </template>
      </div>
    </div>

    <!-- Virtualized rows -->
    <div ref="parentRef" class="flex-1 overflow-auto">
      <div v-if="virtualizer" class="relative" :style="{ height: virtualizer.getTotalSize() + 'px' }">
        <div
          v-for="row in virtualRows"
          :key="row.key"
          class="flex items-center px-4 border-b border-border/40 transition-colors duration-120 cursor-pointer absolute top-0 left-0 w-full"
          :style="{
            height: row.size + 'px',
            transform: `translateY(${row.start}px)`,
          }"
          :class="rowClasses(row._track)"
          @dblclick="view.playTrackById(row._track.id)"
          @click="view.setSelectedTrack(row._track.id)"
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
                  ? 'text-primary'
                  : 'text-faint-foreground'
              "
            >
              {{ row.index + 1 }}
            </span>
          </div>

          <!-- Playing indicator bar -->
          <div
            v-if="row._track.id === playback.currentTrackId"
            class="w-0.5 h-4 rounded-full bg-primary flex-shrink-0 mr-3"
          />

          <!-- Title -->
          <div
            class="truncate text-body"
            :style="{ width: view.columnWidths.title + 'px' }"
            :class="{ 'text-body-md': row._track.id === playback.currentTrackId }"
          >
            {{ row._track.title }}
          </div>

          <!-- Artist -->
          <div
            class="truncate text-body-sm text-muted-foreground"
            :style="{ width: view.columnWidths.artist + 'px' }"
          >
            {{ row._track.artist }}
          </div>

          <!-- Album -->
          <div
            class="truncate text-body-sm text-muted-foreground"
            :style="{ width: view.columnWidths.album + 'px' }"
          >
            {{ row._track.album }}
          </div>

          <!-- Duration -->
          <div
            class="text-body-sm text-muted-foreground tabular-nums text-right flex-shrink-0"
            :style="{ width: view.columnWidths.duration + 'px' }"
          >
            {{ fmt(row._track.duration_secs) }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { Play, ChevronUp, ChevronDown } from "lucide-vue-next";
import { usePlaybackStore } from "@/stores/playback";
import { useViewStore } from "@/stores/view";

const playback = usePlaybackStore();
const view = useViewStore();

const columns = [
  { key: "index", label: "#", width: 40 },
  { key: "title", label: "Title", width: 400 },
  { key: "artist", label: "Artist", width: 240 },
  { key: "album", label: "Album", width: 240 },
  { key: "duration", label: "Duration", width: 80 },
];

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

function rowClasses(track: any) {
  const isCurrent = track.id === playback.currentTrackId;
  const isSelected = track.id === view.selectedTrackId;
  return {
    "bg-primary/10": isCurrent,
    "bg-elevated-hover": isSelected && !isCurrent,
    "hover:bg-elevated-hover": !isCurrent,
    "text-foreground": true,
  };
}

function fmt(sec: number): string {
  if (!sec || sec < 0) return "0:00";
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

// Re-measure when playing track changes (for the indicator)
watch(
  () => playback.currentTrackId,
  () => {
    if (virtualizer.value) {
      virtualizer.value.measure();
    }
  },
);
</script>
