<template>
  <div v-if="artistData" class="flex flex-col h-full">
    <!-- Header -->
    <div class="flex gap-6 p-6 pb-4 flex-shrink-0 items-end">
      <div
        class="w-48 h-48 rounded-lg flex items-center justify-center flex-shrink-0 shadow-1"
        :style="{ background: coverGradient(artistData.name) }"
      >
        <span class="text-white text-7xl font-bold select-none">{{ artistData.name.charAt(0) }}</span>
      </div>
      <div class="flex flex-col justify-end gap-2 min-w-0">
        <p class="text-overline">艺术家</p>
        <h1 class="text-display text-foreground">{{ artistData.name }}</h1>
        <p class="text-body-sm text-faint-foreground">
          {{ artistData.trackCount }} tracks · {{ artistData.albumCount }} albums
        </p>
        <div class="flex items-center gap-2 mt-2">
          <button
            class="w-12 h-12 rounded-full bg-primary text-background flex items-center justify-center hover:scale-105 transition-transform active:scale-95 shadow-1"
            @click="playAll"
          >
            <Play class="w-5 h-5 ml-0.5 fill-background" />
          </button>
          <Button variant="ghost" size="sm" @click="view.closeDetail()">
            <ArrowLeft class="w-4 h-4 mr-1" />
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
import { Play, ArrowLeft } from "lucide-vue-next";
import { useViewStore } from "@/stores/view";
import { coverGradient } from "@/lib/utils";
import { usePlaybackStore } from "@/stores/playback";
import TrackTable from "@/components/TrackTable.vue";
import Button from "@/components/ui/Button.vue";

const view = useViewStore();
const playback = usePlaybackStore();

const artistData = computed(() => {
  if (view.detail?.type !== "artist") return null;
  return view.artists.find((a) => a.name === view.detail!.name) ?? null;
});

function playAll() {
  if (!artistData.value) return;
  const ids = artistData.value.trackIds;
  if (ids.length > 0) playback.playTracks(ids, ids[0]);
}
</script>
