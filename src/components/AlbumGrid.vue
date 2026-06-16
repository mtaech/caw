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
        class="group cursor-pointer rounded-lg p-3 hover:bg-elevated-hover transition-colors duration-120"
        @click="view.openAlbum(album.title)"
      >
        <CoverArt
          :track-id="album.trackIds[0]"
          :size="160"
          class="w-full aspect-square rounded-lg mb-3"
        />
        <p class="text-body-md text-foreground truncate">{{ album.title }}</p>
        <p class="text-body-sm text-muted-foreground truncate">{{ album.artist }}</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useViewStore } from "@/stores/view";
import CoverArt from "@/components/CoverArt.vue";

const view = useViewStore();
</script>
