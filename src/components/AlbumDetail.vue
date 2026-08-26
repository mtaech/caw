<template>
  <div v-if="albumData" class="flex flex-col h-full bg-background select-none">
    <!-- Header with Dynamic Blur Background -->
    <div class="relative flex gap-6 p-7 pb-5 flex-shrink-0 overflow-hidden border-b border-border/60">
      <!-- Blurred cover backdrop -->
      <div v-if="coverUrl" class="absolute inset-0 pointer-events-none -z-0">
        <img
          :src="coverUrl"
          class="w-full h-full object-cover blur-3xl scale-125 opacity-25 dark:opacity-20"
          alt=""
        />
        <div class="absolute inset-0 bg-gradient-to-b from-background/60 via-background/85 to-background" />
      </div>

      <!-- Large Cover Art -->
      <CoverArt
        :track-id="albumData.trackIds[0]"
        :size="190"
        class="relative z-[1] shadow-2 rounded-xl ring-1 ring-black/10 dark:ring-white/10"
      />

      <!-- Album Info & Actions -->
      <div class="relative z-[1] flex flex-col justify-end gap-2 min-w-0 flex-1">
        <div class="flex items-center gap-2">
          <span class="text-overline px-2 py-0.5 rounded-full bg-primary/10 text-primary">专辑</span>
          <span class="text-xs text-muted-foreground">{{ albumData.trackCount }} 首歌曲 · {{ formatDuration(albumData.durationSecs) }}</span>
        </div>

        <h1 class="text-display text-foreground font-bold truncate tracking-tight">
          {{ albumData.title }}
        </h1>

        <p
          class="text-body font-medium text-muted-foreground hover:text-primary cursor-pointer transition-colors truncate"
          @click="view.openArtist(albumData.artist)"
        >
          {{ albumData.artist }}
        </p>

        <!-- Action buttons -->
        <div class="flex items-center gap-3 mt-3">
          <button
            class="h-10 px-5 rounded-full bg-primary text-primary-foreground flex items-center gap-2 font-medium shadow-md hover:scale-105 active:scale-95 hover:bg-primary-hover transition-all"
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
import { computed, ref, watch } from "vue";
import { Play, Shuffle, ArrowLeft } from "lucide-vue-next";
import { useViewStore } from "@/stores/view";
import { usePlaybackStore } from "@/stores/playback";
import { formatDuration } from "@/lib/utils";
import CoverArt from "@/components/CoverArt.vue";
import TrackTable from "@/components/TrackTable.vue";
import Button from "@/components/ui/Button.vue";

const view = useViewStore();
const playback = usePlaybackStore();

const albumData = computed(() => {
  if (view.detail?.type !== "album") return null;
  return view.albums.find((a) => a.title === view.detail!.name) ?? null;
});

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

function shuffleAll() {
  if (!albumData.value) return;
  const ids = [...albumData.value.trackIds];
  if (ids.length === 0) return;
  playback.setShuffle(true);
  const randomIdx = Math.floor(Math.random() * ids.length);
  playback.playTracks(ids, ids[randomIdx]);
}
</script>
