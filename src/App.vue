<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { usePlaybackStore } from "@/stores/playback";
import { useKeyboardShortcuts } from "@/composables/useKeyboardShortcuts";

import TitleBar from "@/components/TitleBar.vue";
import Sidebar from "@/components/Sidebar.vue";
import Content from "@/components/Content.vue";
import PlayerBar from "@/components/PlayerBar.vue";
import Button from "@/components/ui/Button.vue";
import { Disc3, FolderOpen } from "lucide-vue-next";

useKeyboardShortcuts();

const playback = usePlaybackStore();
onMounted(async () => {
  await playback.init();
});

onUnmounted(() => {
  playback.cleanup();
});

async function handlePickFolder() {
  const path = await playback.pickMusicFolder();
  if (path) {
    // library_updated event will re-fetch
  }
}
</script>

<template>
  <div class="h-screen w-screen flex flex-col overflow-hidden bg-background select-none">
    <!-- Custom title bar -->
    <TitleBar />

    <!-- Empty state: no library -->
    <template v-if="playback.library.length === 0 && !playback.loading">
      <div class="flex-1 flex flex-col items-center justify-center gap-4">
        <div class="w-20 h-20 rounded-2xl bg-elevated flex items-center justify-center">
          <Disc3 class="w-10 h-10 text-primary" />
        </div>
        <h1 class="text-title text-foreground">欢迎使用 Caw</h1>
        <p class="text-body text-muted-foreground">选择您的音乐文件夹开始</p>
        <Button size="lg" @click="handlePickFolder">
          <FolderOpen class="w-4 h-4 mr-2" />
          选择音乐文件夹
        </Button>
      </div>
    </template>

    <!-- Loading state -->
    <template v-else-if="playback.loading">
      <div class="flex-1 flex items-center justify-center">
        <div class="flex flex-col items-center gap-3">
          <div class="w-8 h-8 border-2 border-primary border-t-transparent rounded-full animate-spin" />
          <p class="text-body-sm text-muted-foreground">扫描曲库中……</p>
        </div>
      </div>
    </template>

    <!-- Main layout: sidebar + content + playerbar -->
    <template v-else>
      <div class="flex-1 flex overflow-hidden">
        <Sidebar />
        <Content />
      </div>
      <PlayerBar />
    </template>
  </div>
</template>

<style>
/* Global scrollbar styling */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.18);
  border-radius: 4px;
}
::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.3);
}

/* Smooth font rendering */
body {
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
</style>
