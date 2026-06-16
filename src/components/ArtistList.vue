<template>
  <div class="p-6 overflow-y-auto h-full">
    <div class="grid gap-3"
      :style="{ gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))' }"
    >
      <div
        v-for="artist in view.artists"
        :key="artist.name"
        class="flex items-center gap-3 p-3 rounded-lg cursor-pointer hover:bg-elevated-hover transition-colors duration-120"
        @click="view.openArtist(artist.name)"
      >
        <div
          class="w-12 h-12 rounded-full flex items-center justify-center flex-shrink-0 text-white text-lg font-semibold select-none"
          :style="{ background: coverGradient(artist.name) }"
        >
          {{ artist.name.charAt(0) }}
        </div>
        <div class="min-w-0">
          <p class="text-body-md text-foreground truncate">{{ artist.name }}</p>
          <p class="text-body-sm text-muted-foreground">{{ artist.trackCount }} tracks · {{ artist.albumCount }} albums</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useViewStore } from "@/stores/view";
import { coverGradient } from "@/lib/utils";

const view = useViewStore();
</script>
