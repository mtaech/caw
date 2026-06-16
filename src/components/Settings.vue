<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { FolderOpen, FolderX, Music, RefreshCw, X } from "lucide-vue-next";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import Button from "@/components/ui/Button.vue";
import * as api from "@/lib/tauri";

const musicDirs = ref<string[]>([]);
const loading = ref(true);
const scanning = ref(false);
const minimizeToTray = ref(false);

onMounted(async () => {
  try {
    minimizeToTray.value = await api.getMinimizeToTray();
  } catch {}
  await loadDirs();

  // Listen for scan completion to reset scanning state
  unlisteners.push(
    await listen("library_updated", () => {
      scanning.value = false;
    }),
  );
  unlisteners.push(
    await listen("scan_progress", () => {
      // Keep scanning true until we see library_updated
    }),
  );
});

const unlisteners: UnlistenFn[] = [];
onUnmounted(() => {
  for (const fn of unlisteners) fn();
});

async function handleMinimizeToggle() {
  const newVal = !minimizeToTray.value;
  minimizeToTray.value = newVal;
  try {
    await api.setMinimizeToTray(newVal);
  } catch (e) {
    console.error("caw: failed to set minimize_to_tray", e);
    minimizeToTray.value = !newVal;
  }
}

async function loadDirs() {
  loading.value = true;
  try {
    musicDirs.value = await api.getMusicDirs();
  } catch (e) {
    console.error("caw: failed to load music dirs", e);
  } finally {
    loading.value = false;
  }
}

async function handleAddDir() {
  try {
    const result = await invoke<string | null>("pick_music_folder");
    if (result !== null) {
      scanning.value = true;
      await loadDirs();
    }
  } catch (e) {
    console.error("caw: pick folder failed", e);
  }
}

async function handleRescan() {
  scanning.value = true;
  try {
    await api.rescanAll();
    // scan_progress / library_updated events will update state
  } catch (e) {
    console.error("caw: rescan failed", e);
    scanning.value = false;
  }
}

async function handleRemoveDir(path: string) {
  try {
    await api.removeMusicDir(path);
    await loadDirs();
  } catch (e) {
    console.error("caw: remove music dir failed", e);
  }
}
</script>

<template>
  <div class="h-full overflow-y-auto p-6 space-y-8">
    <!-- Page title -->
    <h1 class="text-title text-foreground">设置</h1>

    <!-- Music directories section -->
    <section class="space-y-3">
      <h2 class="text-body-md text-foreground">音乐目录</h2>
      <p class="text-body-sm text-muted-foreground">
        添加您存放音乐文件的文件夹。支持多个目录。
      </p>

      <!-- Directory list -->
      <div v-if="musicDirs.length > 0" class="space-y-2">
        <div
          v-for="dir in musicDirs"
          :key="dir"
          class="flex items-center justify-between gap-3 px-4 py-3 rounded-lg bg-elevated border border-border"
        >
          <div class="flex items-center gap-3 min-w-0">
            <Music class="w-4 h-4 text-primary flex-shrink-0" />
            <span class="text-body text-foreground truncate">{{ dir }}</span>
          </div>
          <button
            class="flex-shrink-0 p-1.5 rounded-md text-muted-foreground hover:text-red-500 hover:bg-red-500/10 transition-colors"
            :title="'移除 ' + dir"
            @click="handleRemoveDir(dir)"
          >
            <X class="w-4 h-4" />
          </button>
        </div>
      </div>

      <!-- Empty state -->
      <div
        v-else-if="!loading"
        class="flex items-center gap-3 px-4 py-6 rounded-lg bg-elevated border border-border text-muted-foreground"
      >
        <FolderX class="w-5 h-5 flex-shrink-0" />
        <p class="text-body-sm">尚未添加音乐目录</p>
      </div>

      <!-- Add button -->
      <div class="flex items-center gap-2">
        <Button variant="outline" size="md" @click="handleAddDir">
          <FolderOpen class="w-4 h-4 mr-2" />
          添加目录
        </Button>
        <Button variant="outline" size="md" @click="handleRescan" :disabled="scanning">
          <RefreshCw class="w-4 h-4 mr-2" :class="{ 'animate-spin': scanning }" />
          重新扫描
        </Button>
      </div>
    </section>

    <!-- Behavior section -->
    <section class="space-y-3">
      <h2 class="text-body-md text-foreground">行为</h2>
      <div class="flex items-center justify-between px-4 py-3 rounded-lg bg-elevated border border-border">
        <div class="space-y-0.5">
          <p class="text-body text-foreground">关闭时最小化到托盘</p>
          <p class="text-body-sm text-muted-foreground">关闭窗口时隐藏到系统托盘而非退出</p>
        </div>
        <button
          class="relative w-10 h-5 rounded-full transition-colors duration-200 flex-shrink-0"
          :class="minimizeToTray ? 'bg-primary' : 'bg-border'"
          @click="handleMinimizeToggle"
        >
          <span
            class="absolute top-0.5 w-4 h-4 rounded-full bg-foreground shadow transition-transform duration-200"
            :class="minimizeToTray ? 'translate-x-[18px]' : 'translate-x-[2px]'"
          />
        </button>
      </div>
    </section>

    <!-- About section -->
    <section class="space-y-3">
      <h2 class="text-body-md text-foreground">关于</h2>
      <div class="px-4 py-4 rounded-lg bg-elevated border border-border space-y-1">
        <p class="text-body text-foreground">Caw v0.1.0</p>
        <p class="text-body-sm text-muted-foreground">
          基于 Rust + Tauri 开发的开源音乐播放器
        </p>
      </div>
    </section>
  </div>
</template>
