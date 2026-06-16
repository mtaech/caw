<script setup lang="ts">
import { ref, onMounted } from "vue";
import { FolderOpen, FolderX, Music, X } from "lucide-vue-next";
import { invoke } from "@tauri-apps/api/core";
import Button from "@/components/ui/Button.vue";
import * as api from "@/lib/tauri";

const musicDirs = ref<string[]>([]);
const loading = ref(true);

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
      await loadDirs();
    }
  } catch (e) {
    console.error("caw: pick folder failed", e);
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

onMounted(() => {
  loadDirs();
});
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
      <Button variant="outline" size="md" @click="handleAddDir">
        <FolderOpen class="w-4 h-4 mr-2" />
        添加目录
      </Button>
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
