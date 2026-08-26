<template>
  <div class="p-6 overflow-y-auto h-full select-none">
    <div
      class="grid gap-5"
      :style="{
        gridTemplateColumns: 'repeat(auto-fill, minmax(190px, 1fr))',
      }"
    >
      <div
        v-for="artist in view.artists"
        :key="artist.name"
        class="group relative cursor-pointer rounded-2xl p-4 bg-elevated/40 hover:bg-elevated border border-border/70 hover:border-border hover:shadow-2 -translate-y-0 hover:-translate-y-1 transition-all duration-200 flex flex-col items-center text-center"
        @click="view.openArtist(artist.name)"
      >
        <!-- Circular Avatar Container (Strictly 1:1 circular ratio) -->
        <div class="relative w-36 h-36 mb-3.5 rounded-full flex-shrink-0 flex items-center justify-center">
          <CoverArt
            v-if="artist.trackIds.length > 0"
            :track-id="artist.trackIds[0]"
            :size="144"
            :round="true"
            class="w-36 h-36 rounded-full object-cover shadow-md ring-2 ring-border group-hover:ring-primary/50 group-hover:scale-105 transition-all duration-300"
          />
          <div
            v-else
            class="w-36 h-36 rounded-full flex items-center justify-center text-white text-3xl font-bold shadow-md ring-2 ring-border group-hover:ring-primary/50 group-hover:scale-105 transition-all duration-300 relative overflow-hidden"
            :style="{ background: coverGradient(artist.name) }"
          >
            <div class="absolute inset-0 bg-gradient-to-tr from-black/20 via-transparent to-white/20 pointer-events-none" />
            <span class="relative z-10 drop-shadow">{{ artist.name.charAt(0).toUpperCase() }}</span>
          </div>

          <!-- Quick Play Button on Hover -->
          <button
            v-if="artist.trackIds.length > 0"
            class="absolute bottom-1 right-1 w-11 h-11 rounded-full bg-primary text-primary-foreground flex items-center justify-center shadow-lg opacity-0 translate-y-2 group-hover:opacity-100 group-hover:translate-y-0 hover:scale-110 active:scale-95 transition-all duration-200 z-10 cursor-pointer"
            title="播放该艺术家所有歌曲"
            @click.stop="playArtist(artist.trackIds)"
          >
            <Play class="w-5 h-5 ml-0.5 fill-primary-foreground" />
          </button>
        </div>

        <!-- Artist Info -->
        <p class="w-full text-sm font-bold text-foreground truncate group-hover:text-primary transition-colors px-1">
          {{ artist.name }}
        </p>
        <p class="text-xs text-muted-foreground truncate mt-1">
          {{ artist.trackCount }} 首歌曲 · {{ artist.albumCount }} 张专辑
        </p>
        <span class="text-caption text-faint-foreground mt-1.5 px-2.5 py-0.5 rounded-full bg-elevated-hover font-medium">
          艺术家
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Play } from "lucide-vue-next";
import { useViewStore } from "@/stores/view";
import { usePlaybackStore } from "@/stores/playback";
import { coverGradient } from "@/lib/utils";
import CoverArt from "@/components/CoverArt.vue";

const view = useViewStore();
const playback = usePlaybackStore();

function playArtist(trackIds: number[]) {
  if (trackIds.length > 0) {
    playback.playTracks(trackIds, trackIds[0]);
  }
}
</script>
