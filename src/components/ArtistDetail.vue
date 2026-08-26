<template>
  <div v-if="artistData" class="flex flex-col h-full bg-background select-none">
    <!-- Header Hero -->
    <div class="flex gap-6 p-7 pb-6 flex-shrink-0 items-end border-b border-border relative overflow-hidden">
      <!-- Gradient Backdrop Glow -->
      <div
        class="absolute -top-24 -left-24 w-96 h-96 rounded-full blur-3xl opacity-20 pointer-events-none"
        :style="{ background: coverGradient(artistData.name) }"
      />

      <!-- Circular Hero Avatar (Strictly 1:1 circle) -->
      <div class="relative w-40 h-40 rounded-full flex-shrink-0 flex items-center justify-center z-10 shadow-3">
        <CoverArt
          v-if="artistData.trackIds.length > 0"
          :track-id="artistData.trackIds[0]"
          :size="160"
          :round="true"
          class="w-40 h-40 rounded-full object-cover shadow-2 ring-4 ring-border/80"
        />
        <div
          v-else
          class="w-40 h-40 rounded-full flex items-center justify-center text-white text-6xl font-extrabold shadow-2 ring-4 ring-border/80 relative overflow-hidden"
          :style="{ background: coverGradient(artistData.name) }"
        >
          <div class="absolute inset-0 bg-gradient-to-tr from-black/20 via-transparent to-white/20 pointer-events-none" />
          <span class="relative z-10 drop-shadow">
            {{ artistData.name.charAt(0).toUpperCase() }}
          </span>
        </div>
      </div>

      <div class="flex flex-col justify-end gap-2.5 min-w-0 flex-1 relative z-10">
        <div class="flex items-center gap-2">
          <span class="text-overline px-2.5 py-0.5 rounded-full bg-primary/15 text-primary font-semibold">艺术家</span>
          <span class="text-xs text-muted-foreground">
            {{ artistData.trackCount }} 首歌曲 · {{ artistData.albumCount }} 张专辑
          </span>
        </div>

        <h1 class="text-display text-foreground font-bold tracking-tight truncate">
          {{ artistData.name }}
        </h1>

        <!-- Actions -->
        <div class="flex items-center gap-3 mt-2">
          <button
            class="h-10 px-5 rounded-full bg-primary text-primary-foreground flex items-center gap-2 font-medium shadow-md hover:scale-105 active:scale-95 hover:bg-primary-hover transition-all cursor-pointer"
            @click="playAll"
          >
            <Play class="w-4 h-4 fill-primary-foreground" />
            播放全部
          </button>

          <Button variant="secondary" size="md" circle class="h-10 px-4" @click="shuffleAll">
            <Shuffle class="w-4 h-4 mr-1.5" />
            随机播放
          </Button>

          <Button variant="ghost" size="md" @click="view.closeDetail()">
            <ArrowLeft class="w-4 h-4 mr-1.5" />
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
import { Play, Shuffle, ArrowLeft } from "lucide-vue-next";
import { useViewStore } from "@/stores/view";
import { coverGradient } from "@/lib/utils";
import { usePlaybackStore } from "@/stores/playback";
import TrackTable from "@/components/TrackTable.vue";
import CoverArt from "@/components/CoverArt.vue";
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

function shuffleAll() {
  if (!artistData.value) return;
  const ids = [...artistData.value.trackIds];
  if (ids.length === 0) return;
  playback.setShuffle(true);
  const randomIdx = Math.floor(Math.random() * ids.length);
  playback.playTracks(ids, ids[randomIdx]);
}
</script>
