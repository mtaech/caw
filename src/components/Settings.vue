<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from "vue";
import { FolderOpen, FolderX, Music, RefreshCw, X, ChevronDown } from "lucide-vue-next";
import { SwitchRoot, SwitchThumb } from "radix-vue";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { usePlaybackStore } from "@/stores/playback";
import { invoke } from "@tauri-apps/api/core";
import Button from "@/components/ui/Button.vue";
import { applyFont, getFontPreference, setFontPreference, getAvailableFonts, loadSystemFonts } from "@/lib/fonts";
import type { FontOption } from "@/lib/fonts";
import * as api from "@/lib/tauri";

const playback = usePlaybackStore();
const musicDirs = ref<string[]>([]);
const loading = ref(true);
const scanning = ref(false);
const minimizeToTray = ref(false);
const queueReplaceMode = ref(true);

// ── Font preference ──
const fontOptions = ref<FontOption[]>([
  { id: "default", label: "系统默认", fontFamily: "" },
]);
const currentFont = ref(getFontPreference());
watch(currentFont, (id) => {
  setFontPreference(id);
  applyFont(id);
});

// ── Font picker combobox ──
const fontPickerOpen = ref(false);
const fontSearch = ref("");
const fontTriggerRef = ref<HTMLElement | null>(null);
const fontDropdownRef = ref<HTMLElement | null>(null);
const dropdownLeft = ref(0);
const dropdownTop = ref(0);

const currentFontLabel = computed(() =>
  fontOptions.value.find((o) => o.id === currentFont.value)?.label ?? "系统默认",
);

const filteredFonts = computed(() => {
  if (!fontSearch.value) return fontOptions.value;
  const q = fontSearch.value.toLowerCase();
  return fontOptions.value.filter((o) => o.label.toLowerCase().includes(q));
});

function toggleFontPicker() {
  if (fontPickerOpen.value) {
    fontPickerOpen.value = false;
    return;
  }
  if (fontTriggerRef.value) {
    const r = fontTriggerRef.value.getBoundingClientRect();
    dropdownLeft.value = r.left;
    dropdownTop.value = r.bottom + 4;
  }
  fontPickerOpen.value = true;
}

function selectFont(id: string) {
  currentFont.value = id;
  fontPickerOpen.value = false;
  fontSearch.value = "";
}

function onDocumentClick(e: MouseEvent) {
  if (!fontPickerOpen.value) return;
  const target = e.target as Node;
  if (fontTriggerRef.value?.contains(target)) return;
  if (fontDropdownRef.value?.contains(target)) return;
  fontPickerOpen.value = false;
}

onMounted(async () => {
  // Load system fonts for the font picker.
  await loadSystemFonts();
  fontOptions.value = [
    { id: "default", label: "系统默认", fontFamily: "" },
    ...getAvailableFonts(),
  ];

  document.addEventListener("click", onDocumentClick);

  try {
    minimizeToTray.value = await api.getMinimizeToTray();
    try {
      queueReplaceMode.value = await api.getQueueReplaceMode();
    } catch {}
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
  document.removeEventListener("click", onDocumentClick);
  for (const fn of unlisteners) fn();
});

async function handleQueueReplaceToggle(checked: boolean) {
  queueReplaceMode.value = checked;
  try {
    await playback.setQueueReplaceMode(checked);
  } catch (e) {
    console.error("caw: failed to set queue replace mode", e);
    queueReplaceMode.value = !checked;
  }
}

async function handleMinimizeToggle(checked: boolean) {
  minimizeToTray.value = checked;
  try {
    await api.setMinimizeToTray(checked);
  } catch (e) {
    console.error("caw: failed to set minimize_to_tray", e);
    minimizeToTray.value = !checked;
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
            class="flex-shrink-0 p-1.5 rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
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
        <SwitchRoot
          :checked="minimizeToTray"
          @update:checked="handleMinimizeToggle"
          class="w-10 h-5 rounded-full data-[state=checked]:bg-primary data-[state=unchecked]:bg-border transition-colors duration-200 flex-shrink-0"
        >
          <SwitchThumb class="block w-4 h-4 rounded-full bg-foreground shadow mx-0.5 transition-transform duration-200 data-[state=checked]:translate-x-5 data-[state=unchecked]:translate-x-0" />
        </SwitchRoot>
      </div>
      <div class="flex items-center justify-between px-4 py-3 rounded-lg bg-elevated border border-border">
        <div class="space-y-0.5">
          <p class="text-body text-foreground">替换当前播放队列</p>
          <p class="text-body-sm text-muted-foreground">点击专辑/艺人/播放列表播放时替换当前队列，关闭则追加</p>
        </div>
        <SwitchRoot
          :checked="queueReplaceMode"
          @update:checked="handleQueueReplaceToggle"
          class="w-10 h-5 rounded-full data-[state=checked]:bg-primary data-[state=unchecked]:bg-border transition-colors duration-200 flex-shrink-0"
        >
          <SwitchThumb class="block w-4 h-4 rounded-full bg-foreground shadow mx-0.5 transition-transform duration-200 data-[state=checked]:translate-x-5 data-[state=unchecked]:translate-x-0" />
        </SwitchRoot>
      </div>
    </section>

    <!-- Font section -->
    <section class="space-y-3">
      <h2 class="text-body-md text-foreground">字体</h2>

      <!-- Combobox trigger + dropdown -->
      <div class="relative">
        <button
          ref="fontTriggerRef"
          class="flex h-9 w-64 items-center justify-between gap-2 rounded-md border border-border bg-elevated px-3 text-sm text-foreground hover:bg-elevated-hover transition-colors"
          @click.stop="toggleFontPicker"
        >
          <span class="truncate">{{ currentFontLabel }}</span>
          <ChevronDown class="w-4 h-4 text-muted-foreground flex-shrink-0" />
        </button>

        <Teleport to="body">
          <Transition name="pop">
            <div
              v-if="fontPickerOpen"
              ref="fontDropdownRef"
              class="fixed z-dropdown w-72 rounded-lg border border-border bg-elevated shadow-2 overflow-hidden"
              :style="{ left: dropdownLeft + 'px', top: dropdownTop + 'px' }"
            >
              <!-- Search -->
              <div class="p-2 border-b border-border">
                <input
                  v-model="fontSearch"
                  class="w-full h-8 rounded-md border border-border bg-background px-2.5 text-sm text-foreground placeholder:text-muted-foreground outline-none transition-colors focus:border-primary"
                  placeholder="搜索字体…"
                  @keydown.escape="fontPickerOpen = false"
                />
              </div>

              <!-- Options list -->
              <div class="overflow-y-auto max-h-60 py-1">
                <button
                  v-for="opt in filteredFonts"
                  :key="opt.id"
                  class="w-full flex items-center px-3 py-2 text-sm text-left hover:bg-elevated-hover transition-colors"
                  :class="{ 'bg-primary/10': opt.id === currentFont }"
                  :style="{ fontFamily: opt.fontFamily || undefined }"
                  @click.stop="selectFont(opt.id)"
                >
                  {{ opt.label }}
                </button>

                <div
                  v-if="filteredFonts.length === 0"
                  class="px-3 py-6 text-sm text-muted-foreground text-center"
                >
                  未找到匹配字体
                </div>
              </div>

              <!-- Footer -->
              <div
                class="px-3 py-1.5 border-t border-border text-caption text-faint-foreground text-center"
              >
                {{ fontOptions.length - 1 }} 个可用
              </div>
            </div>
          </Transition>
        </Teleport>
      </div>

      <p class="text-body-sm text-faint-foreground">
        自动检测系统已安装字体。可能需要重启应用才能完全生效。
      </p>
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
