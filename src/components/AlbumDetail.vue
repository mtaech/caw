<template>
  <div v-if="albumData" class="flex flex-col h-full">
    <!-- Header -->
    <div class="relative flex gap-6 p-6 pb-4 flex-shrink-0 overflow-hidden">
      <!-- Blurred cover backdrop -->
      <div v-if="coverUrl" class="absolute inset-0 pointer-events-none">
        <img
          :src="coverUrl"
          class="w-full h-full object-cover blur-3xl scale-125 opacity-40"
          alt=""
        />
        <div
          class="absolute inset-0 bg-gradient-to-b from-background/70 via-background/85 to-background"
        />
      </div>
      <CoverArt :track-id="albumData.trackIds[0]" :size="200" class="relative z-[1]" />
      <div class="relative z-[1] flex flex-col justify-end gap-2 min-w-0">
        <p class="text-overline">专辑</p>
        <h1 class="text-display text-foreground truncate">{{ albumData.title }}</h1>
        <p class="text-body text-muted-foreground">{{ albumData.artist }}</p>
        <p class="text-body-sm text-faint-foreground">
          {{ albumData.trackCount }} tracks · {{ fmt(albumData.durationSecs) }}
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
import { computed, ref, watch } from "vue";
import { Play, ArrowLeft } from "lucide-vue-next";
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

// Cover URL for the blurred header backdrop.
const coverUrl = ref<string | null>(null);
watch(
  () => albumData.value?.trackIds[0],
  async (id) => {
    coverUrl.value = id ? await playback.getCoverUrl(id) : null;
  },
  { immediate: true },
);

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
