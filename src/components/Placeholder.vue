<template>
  <div class="flex-1 flex flex-col items-center justify-center gap-3 text-center p-8">
    <component :is="iconComponent" class="w-12 h-12 text-faint-foreground" />
    <h2 class="text-title text-foreground">{{ title }}</h2>
    <p class="text-body text-muted-foreground max-w-md">{{ desc }}</p>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { ListMusic, Folder } from "lucide-vue-next";

const props = defineProps<{
  title: string;
  desc: string;
  iconType?: "playlists" | "folders";
}>();

const iconComponent = computed(() => {
  if (props.iconType === "folders") return Folder;
  return ListMusic;
});
</script>
