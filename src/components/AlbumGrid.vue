<template>
  <div class="p-6 overflow-y-auto h-full select-none">
    <div
      class="grid gap-5"
      :style="{
        gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))',
      }"
    >
      <div
        v-for="album in view.albums"
        :key="album.title"
        class="group relative cursor-pointer rounded-xl p-3 bg-elevated/40 hover:bg-elevated border border-border/40 hover:border-border hover:shadow-2 -translate-y-0 hover:-translate-y-1 transition-all duration-200"
        @click="view.openAlbum(album.title)"
      >
        <div class="relative w-full aspect-square mb-3 rounded-lg overflow-hidden shadow-sm">
          <CoverArt
            :track-id="album.trackIds[0]"
            :size="240"
            class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
          />
          <!-- Hover play button overlay -->
          <button
            class="absolute bottom-2.5 right-2.5 w-11 h-11 rounded-full bg-primary text-primary-foreground flex items-center justify-center shadow-lg opacity-0 translate-y-2 group-hover:opacity-100 group-hover:translate-y-0 hover:scale-105 active:scale-95 transition-all duration-200 z-10"
            title="播放专辑"
            @click.stop="playAlbum(album.title, album.trackIds)"
          >
            <Play class="w-5 h-5 ml-0.5 fill-primary-foreground" />
          </button>
        </div>
        
        <p class="text-sm font-semibold text-foreground truncate group-hover:text-primary transition-colors">
          {{ album.title }}
        </p>
        <p class="text-xs text-muted-foreground truncate mt-0.5 hover:text-foreground transition-colors" @click.stop="view.openArtist(album.artist)">
          {{ album.artist }}
        </p>
        <p class="text-caption text-faint-foreground mt-1">
          {{ album.trackIds.length }} 首歌曲
        </p>
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
