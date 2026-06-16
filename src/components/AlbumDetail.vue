<template>
  <div v-if="albumData" class="flex flex-col h-full">
    <!-- Header -->
    <div class="flex gap-6 p-6 pb-4 flex-shrink-0">
      <CoverArt :track-id="albumData.trackIds[0]" :size="200" />
      <div class="flex flex-col justify-end gap-2 min-w-0">
        <p class="text-caption text-muted-foreground uppercase tracking-wider">专辑</p>
        <h1 class="text-display text-foreground truncate">{{ albumData.title }}</h1>
        <p class="text-body text-muted-foreground">{{ albumData.artist }}</p>
        <p class="text-body-sm text-faint-foreground">
          {{ albumData.trackCount }} tracks · {{ fmt(albumData.durationSecs) }}
        </p>
        <div class="flex items-center gap-2 mt-2">
          <button
            class="w-12 h-12 rounded-full bg-primary text-background flex items-center justify-center hover:scale-105 transition-transform active:scale-95"
            @click="playAll"
          >
            <Play class="w-5 h-5 ml-0.5 fill-background" />
          </button>
          <Button variant="ghost" size="sm" @click="view.closeDetail()">
            返回
          </Button>
        </div>
      </div>
    </div>

    <!-- Track list -->
    <div class="flex-1 overflow-hidden">
      <TrackTable />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Play } from "lucide-vue-next";
import { useViewStore } from "@/stores/view";
import { usePlaybackStore } from "@/stores/playback";
import CoverArt from "@/components/CoverArt.vue";
import TrackTable from "@/components/TrackTable.vue";
import Button from "@/components/ui/Button.vue";

const view = useViewStore();
const playback = usePlaybackStore();

const albumData = computed(() => {
  if (view.detail?.type !== "album") return null;
  return view.albums.find((a) => a.title === view.detail!.name) ?? null;
});

function playAll() {
  if (!albumData.value) return;
  const ids = albumData.value.trackIds;
  if (ids.length > 0) playback.playTracks(ids, ids[0]);
}

function fmt(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}
</script>
