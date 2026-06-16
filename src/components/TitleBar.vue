<template>
  <header
    class="h-9 flex-shrink-0 flex items-center justify-between px-3 bg-sidebar select-none"
    data-tauri-drag-region
  >
    <!-- Left: app name -->
    <div class="flex items-center gap-2">
      <Disc3 class="w-4 h-4 text-primary" />
      <span class="text-caption text-muted-foreground tracking-wider uppercase">Caw</span>
    </div>

    <!-- Right: window controls -->
    <div class="flex items-center gap-1">
      <button
        class="h-9 w-12 flex items-center justify-center text-muted-foreground hover:bg-elevated-hover hover:text-foreground transition-colors"
        @click="minimize"
      >
        <Minus class="w-3.5 h-3.5" />
      </button>
      <button
        class="h-9 w-12 flex items-center justify-center text-muted-foreground hover:bg-elevated-hover hover:text-foreground transition-colors"
        @click="toggleMaximize"
      >
        <Square v-if="!maximized" class="w-3.5 h-3.5" />
        <Minimize2 v-else class="w-3.5 h-3.5" />
      </button>
      <button
        class="h-9 w-12 flex items-center justify-center text-muted-foreground hover:bg-red-500 hover:text-white transition-colors"
        @click="closeWindow"
      >
        <X class="w-3.5 h-3.5" />
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { Minus, Square, Minimize2, X, Disc3 } from "lucide-vue-next";
import { getCurrentWindow } from "@tauri-apps/api/window";

const maximized = ref(false);

async function minimize() {
  await getCurrentWindow().minimize();
}

async function toggleMaximize() {
  const w = getCurrentWindow();
  if (await w.isMaximized()) {
    await w.unmaximize();
    maximized.value = false;
  } else {
    await w.maximize();
    maximized.value = true;
  }
}

async function closeWindow() {
  await getCurrentWindow().close();
}

onMounted(async () => {
  try {
    maximized.value = await getCurrentWindow().isMaximized();
  } catch {}
});
</script>
