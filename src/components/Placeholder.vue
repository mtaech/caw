<template>
  <div class="flex flex-col items-center justify-center h-full gap-4 text-center p-8 select-none">
    <div class="w-20 h-20 rounded-2xl bg-elevated border border-border/80 shadow-1 flex items-center justify-center">
      <ListMusic v-if="iconType === 'playlists'" class="w-10 h-10 text-primary" />
      <Music v-else class="w-10 h-10 text-primary" />
    </div>
    <div class="space-y-1.5 max-w-sm">
      <h2 class="text-title text-foreground font-bold">{{ title }}</h2>
      <p class="text-body-sm text-muted-foreground leading-relaxed">{{ desc }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Music, ListMusic } from "lucide-vue-next";

defineProps<{
  title: string;
  desc: string;
  iconType?: string;
}>();
</script>
