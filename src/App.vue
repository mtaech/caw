<script setup lang="ts">
import { onMounted, onUnmounted, provide, ref } from "vue";
import { usePlaybackStore } from "@/stores/playback";
import { useNeteaseStore } from "@/stores/netease";
import { useKeyboardShortcuts } from "@/composables/useKeyboardShortcuts";
import { applyFont, getFontPreference, loadSystemFonts } from "@/lib/fonts";
import { useTheme } from "@/composables/useTheme";
import { getCurrentWindow } from "@tauri-apps/api/window";

import TitleBar from "@/components/TitleBar.vue";
import Sidebar from "@/components/Sidebar.vue";
import Content from "@/components/Content.vue";
import PlayerBar from "@/components/PlayerBar.vue";
import QueuePanel from "@/components/QueuePanel.vue";
import CloseDialog from "@/components/CloseDialog.vue";
import Button from "@/components/ui/Button.vue";
import { Disc3, FolderOpen } from "lucide-vue-next";
import * as api from "@/lib/tauri";

useKeyboardShortcuts();

const { initTheme } = useTheme();
const playback = usePlaybackStore();
const netease = useNeteaseStore();
const showQueue = ref(false);
const isMaximized = ref(false);
const showCloseDialog = ref(false);

async function handleRequestClose() {
  try {
    const prompt = await api.getClosePrompt();
    if (prompt) {
      showCloseDialog.value = true;
    } else {
      const toTray = await api.getMinimizeToTray();
      if (toTray) {
        await api.hideWindow();
      } else {
        await api.exitApp();
      }
    }
  } catch (err) {
    console.error("Failed to handle close request:", err);
    showCloseDialog.value = true;
  }
}

provide("toggleQueue", () => {
  showQueue.value = !showQueue.value;
});
provide("showQueue", showQueue);
provide("isMaximized", isMaximized);
provide("requestClose", handleRequestClose);

let unlistenResize: (() => void) | null = null;
let unlistenCloseRequested: (() => void) | null = null;

onMounted(async () => {
  initTheme();
  await loadSystemFonts();
  applyFont(getFontPreference());
  await Promise.all([playback.init(), netease.loadConfig()]);

  try {
    const w = getCurrentWindow();
    isMaximized.value = await w.isMaximized();
    unlistenResize = await w.onResized(async () => {
      isMaximized.value = await w.isMaximized();
    });
  } catch {}

  try {
    unlistenCloseRequested = await api.onWindowCloseRequested(() => {
      showCloseDialog.value = true;
    });
  } catch {}
});

onUnmounted(() => {
  playback.cleanup();
  unlistenResize?.();
  unlistenCloseRequested?.();
});

async function handlePickFolder() {
  const path = await playback.pickMusicFolder();
  if (path) {
    // library_updated event will re-fetch
  }
}

// Custom resize handles (needed because decorations: false)
function startResize(dir: string) {
  getCurrentWindow().startResizeDragging(dir as any);
}
</script>

<template>
  <div
    class="h-screen w-screen flex flex-col overflow-hidden bg-background text-foreground select-none relative transition-colors duration-200"
    :class="[
      isMaximized
        ? 'border-0 rounded-none'
        : 'border border-window-border window-frame rounded-xl'
    ]"
  >
    <!-- Resize handles (client-side, only active when not maximized) -->
    <div v-if="!isMaximized" class="absolute inset-0 pointer-events-none z-50 rounded-xl overflow-hidden">
      <div class="absolute top-0 left-1 right-1 h-1 pointer-events-auto cursor-n-resize" @mousedown="startResize('North')" />
      <div class="absolute bottom-0 left-1 right-1 h-1 pointer-events-auto cursor-s-resize" @mousedown="startResize('South')" />
      <div class="absolute left-0 top-1 bottom-1 w-1 pointer-events-auto cursor-w-resize" @mousedown="startResize('West')" />
      <div class="absolute right-0 top-1 bottom-1 w-1 pointer-events-auto cursor-e-resize" @mousedown="startResize('East')" />
      <div class="absolute top-0 left-0 w-3 h-3 pointer-events-auto cursor-nw-resize" @mousedown="startResize('NorthWest')" />
      <div class="absolute top-0 right-0 w-3 h-3 pointer-events-auto cursor-ne-resize" @mousedown="startResize('NorthEast')" />
      <div class="absolute bottom-0 left-0 w-3 h-3 pointer-events-auto cursor-sw-resize" @mousedown="startResize('SouthWest')" />
      <div class="absolute bottom-0 right-0 w-3 h-3 pointer-events-auto cursor-se-resize" @mousedown="startResize('SouthEast')" />
    </div>

    <!-- Custom title bar -->
    <TitleBar />

    <!-- Empty state: no library -->
    <template v-if="playback.library.length === 0 && !playback.loading">
      <div class="flex-1 flex flex-col items-center justify-center gap-6 p-8 relative overflow-hidden">
        <!-- Decorative Ambient Background Glow -->
        <div class="absolute w-96 h-96 rounded-full bg-primary/10 blur-3xl pointer-events-none -z-0" />
        
        <div class="w-24 h-24 rounded-3xl bg-elevated border border-border shadow-2 flex items-center justify-center relative z-10 transition-transform duration-300 hover:scale-105">
          <Disc3 class="w-12 h-12 text-primary" />
        </div>
        
        <div class="text-center max-w-sm space-y-2 relative z-10">
          <h1 class="text-display text-foreground font-bold">欢迎使用 Caw</h1>
          <p class="text-body text-muted-foreground leading-relaxed">
            极速、纯粹的本地音乐播放器。添加您的音乐目录即可开始享受音乐。
          </p>
        </div>

        <Button size="lg" class="relative z-10 shadow-glow" @click="handlePickFolder">
          <FolderOpen class="w-4 h-4 mr-2" />
          选择音乐文件夹
        </Button>
      </div>
    </template>

    <!-- Loading state -->
    <template v-else-if="playback.loading">
      <div class="flex-1 flex items-center justify-center">
        <div class="flex flex-col items-center gap-4">
          <div class="w-10 h-10 border-3 border-primary border-t-transparent rounded-full animate-spin" />
          <p class="text-body-sm text-muted-foreground font-medium animate-pulse">扫描曲库中……</p>
        </div>
      </div>
    </template>

    <!-- Main layout: sidebar + content + playerbar -->
    <template v-else>
      <div class="flex-1 flex overflow-hidden relative">
        <Sidebar />
        <Content />
        <QueuePanel :visible="showQueue" @close="showQueue = false" />
      </div>
      <PlayerBar />
    </template>

    <!-- Close confirmation dialog -->
    <CloseDialog
      :open="showCloseDialog"
      @close="showCloseDialog = false"
      @confirm="showCloseDialog = false"
    />
  </div>
</template>
