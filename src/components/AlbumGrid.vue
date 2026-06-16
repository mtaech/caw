<template>
  <div class="p-6 overflow-y-auto h-full">
    <div
      class="grid gap-4"
      :style="{
        gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))',
      }"
    >
      <div
        v-for="album in view.albums"
        :key="album.title"
        class="group relative cursor-pointer rounded-lg p-3 hover:bg-elevated-hover hover:shadow-1 transition-all duration-120"
        @click="view.openAlbum(album.title)"
      >
        <div class="relative">
          <CoverArt
            :track-id="album.trackIds[0]"
            :size="160"
            class="w-full aspect-square rounded-lg mb-3"
          />
          <!-- Hover play button -->
          <button
            class="absolute bottom-2 right-2 w-11 h-11 rounded-full bg-primary text-background flex items-center justify-center shadow-2 opacity-0 translate-y-1 group-hover:opacity-100 group-hover:translate-y-0 transition-all duration-120"
            title="播放"
            @click.stop="playAlbum(album.title, album.trackIds)"
          >
            <Play class="w-5 h-5 ml-0.5 fill-background" />
          </button>
        </div>
        <p class="text-body-md text-foreground truncate">{{ album.title }}</p>
        <p class="text-body-sm text-muted-foreground truncate">{{ album.artist }}</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Play } from "lucide-vue-next";
import { useViewStore } from "@/stores/view";
import { usePlaybackStore } from "@/stores/playback";
import CoverArt from "@/components/CoverArt.vue";

const view = useViewStore();
const playback = usePlaybackStore();

function playAlbum(_title: string, trackIds: number[]) {
  if (trackIds.length > 0) playback.playTracks(trackIds, trackIds[0]);
}
</script>
