<template>
  <header
    class="h-10 flex-shrink-0 flex items-center justify-between px-3 bg-sidebar border-b border-border select-none relative z-30"
    data-tauri-drag-region
  >
    <!-- Left: App Brand & Icon -->
    <div class="flex items-center gap-2.5 pointer-events-none pl-1">
      <div class="w-5 h-5 rounded-md bg-primary flex items-center justify-center text-primary-foreground shadow-sm">
        <Disc3 class="w-3.5 h-3.5 animate-spin-slow" />
      </div>
      <span class="text-xs font-bold tracking-widest uppercase text-foreground/90">CAW</span>
    </div>

    <!-- Center: Subtle Now-Playing Header pill (if playing) -->
    <div
      v-if="playback.currentTrack"
      class="hidden md:flex items-center gap-2 px-3 py-1 rounded-full bg-elevated border border-border max-w-sm"
    >
      <div class="flex items-center gap-0.5 h-3">
        <span class="w-0.5 h-2.5 bg-primary rounded-full" :class="{ 'eq-bar-1': playback.isPlaying }" />
        <span class="w-0.5 h-3 bg-primary rounded-full" :class="{ 'eq-bar-2': playback.isPlaying }" />
        <span class="w-0.5 h-2 bg-primary rounded-full" :class="{ 'eq-bar-3': playback.isPlaying }" />
      </div>
      <span class="text-xs font-medium text-foreground truncate max-w-[150px]">
        {{ playback.currentTrack.title }}
      </span>
      <span class="text-xs text-muted-foreground truncate max-w-[120px]">
        · {{ playback.currentTrack.artist }}
      </span>
    </div>

    <!-- Right: Theme Quick Toggle & Window controls -->
    <div class="flex items-center gap-1">
      <!-- Quick Theme Switch -->
      <button
        class="h-8 w-8 rounded-md flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors"
        :title="themeTitle"
        @click="cycleTheme"
      >
        <Sun v-if="theme === 'light'" class="w-4 h-4" />
        <Moon v-else-if="theme === 'dark'" class="w-4 h-4" />
        <Laptop v-else class="w-4 h-4" />
      </button>

      <!-- Window controls -->
      <div class="flex items-center ml-1">
        <button
          class="h-8 w-9 rounded-md flex items-center justify-center text-muted-foreground hover:bg-elevated-hover hover:text-foreground transition-colors active:scale-95"
          title="最小化"
          @click="minimize"
        >
          <Minus class="w-3.5 h-3.5" />
        </button>
        <button
          class="h-8 w-9 rounded-md flex items-center justify-center text-muted-foreground hover:bg-elevated-hover hover:text-foreground transition-colors active:scale-95"
          :title="isMaximized ? '还原' : '最大化'"
          @click="toggleMaximize"
        >
          <Square v-if="!isMaximized" class="w-3.5 h-3.5" />
          <Minimize2 v-else class="w-3.5 h-3.5" />
        </button>
        <button
          class="h-8 w-9 rounded-md flex items-center justify-center text-muted-foreground hover:bg-red-500 hover:text-white transition-colors active:scale-95"
          title="关闭"
          @click="closeWindow"
        >
          <X class="w-4 h-4" />
        </button>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed, inject, type Ref, ref } from "vue";
import { Minus, Square, Minimize2, X, Disc3, Sun, Moon, Laptop } from "lucide-vue-next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { usePlaybackStore } from "@/stores/playback";
import { useTheme } from "@/composables/useTheme";

const playback = usePlaybackStore();
const { theme, setTheme } = useTheme();

const isMaximized = inject<Ref<boolean>>("isMaximized", ref(false));

const themeTitle = computed(() => {
  if (theme.value === "light") return "当前：浅色主题（点击切换）";
  if (theme.value === "dark") return "当前：深色主题（点击切换）";
  return "当前：跟随系统（点击切换）";
});

function cycleTheme() {
  if (theme.value === "system") {
    setTheme("light");
  } else if (theme.value === "light") {
    setTheme("dark");
  } else {
    setTheme("system");
  }
}

async function minimize() {
  await getCurrentWindow().minimize();
}

async function toggleMaximize() {
  const w = getCurrentWindow();
  if (await w.isMaximized()) {
    await w.unmaximize();
    isMaximized.value = false;
  } else {
    await w.maximize();
    isMaximized.value = true;
  }
}

async function closeWindow() {
  await getCurrentWindow().close();
}
</script>

<style scoped>
.animate-spin-slow {
  animation: spin 12s linear infinite;
}
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
