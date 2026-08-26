<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import {
  FolderOpen,
  FolderX,
  RefreshCw,
  ChevronDown,
  Sun,
  Moon,
  Laptop,
  Check,
  Disc3,
  Sliders,
  Type,
  Folder,
  Info,
} from "lucide-vue-next";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { usePlaybackStore } from "@/stores/playback";
import { useTheme } from "@/composables/useTheme";
import Button from "@/components/ui/Button.vue";
import {
  applyFont,
  getFontPreference,
  setFontPreference,
  loadSystemFonts,
  getAvailableFonts,
  type FontOption,
} from "@/lib/fonts";
import * as api from "@/lib/tauri";

const playback = usePlaybackStore();
const { theme, setTheme } = useTheme();

const musicDirs = ref<string[]>([]);
const loading = ref(true);
const scanning = ref(false);

// ── Font preference ──
const fontOptions = ref<FontOption[]>([
  { id: "default", label: "系统默认 (System Default)", fontFamily: "" },
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
const dropdownLeft = ref(0);
const dropdownTop = ref(0);

const currentFontLabel = computed(
  () => fontOptions.value.find((o) => o.id === currentFont.value)?.label ?? "系统默认",
);

const filteredFonts = computed(() => {
  if (!fontSearch.value) return fontOptions.value;
  const q = fontSearch.value.toLowerCase();
  return fontOptions.value.filter((o) => o.label.toLowerCase().includes(q));
});

const fontListRef = ref<HTMLElement | null>(null);

const fontVirtualizerOptions = computed(() => ({
  count: filteredFonts.value.length,
  getScrollElement: () => fontListRef.value,
  estimateSize: () => 36,
  overscan: 8,
}));

const fontVirtualizer = useVirtualizer(fontVirtualizerOptions);

const virtualFontRows = computed(() => {
  const v = fontVirtualizer.value;
  if (!v) return [];
  return v.getVirtualItems().map((item: any) => ({
    ...item,
    _font: filteredFonts.value[item.index],
  }));
});

function toggleFontPicker() {
  if (fontPickerOpen.value) {
    fontPickerOpen.value = false;
    return;
  }
  if (fontTriggerRef.value) {
    const r = fontTriggerRef.value.getBoundingClientRect();
    dropdownLeft.value = r.left;
    dropdownTop.value = r.bottom + 6;
  }
  fontPickerOpen.value = true;
  nextTick(() => fontVirtualizer.value?.measure());
}

function selectFont(id: string) {
  currentFont.value = id;
  fontPickerOpen.value = false;
  fontSearch.value = "";
}

watch(fontSearch, () => {
  nextTick(() => {
    if (fontListRef.value) fontListRef.value.scrollTop = 0;
  });
});

let unlistenScanStarted: UnlistenFn | null = null;
let unlistenScanComplete: UnlistenFn | null = null;

async function loadDirs() {
  loading.value = true;
  try {
    musicDirs.value = await api.getMusicDirs();
  } catch (err) {
    console.error("Failed to load music dirs:", err);
  } finally {
    loading.value = false;
  }
}

async function handleAdd() {
  try {
    const selected = await playback.pickMusicFolder();
    if (selected) {
      await loadDirs();
      await playback.init();
    }
  } catch (err) {
    console.error("Failed to add music dir:", err);
  }
}

async function handleRemove(path: string) {
  try {
    await api.removeMusicDir(path);
    await loadDirs();
    await playback.init();
  } catch (err) {
    console.error("Failed to remove music dir:", err);
  }
}

async function handleRescan() {
  scanning.value = true;
  try {
    await api.rescanAll();
  } catch (err) {
    console.error("Failed to trigger rescan:", err);
    scanning.value = false;
  }
}

onMounted(async () => {
  await loadDirs();
  try {
    await loadSystemFonts();
    const loaded = getAvailableFonts();
    if (loaded && loaded.length > 0) {
      fontOptions.value = [
        { id: "default", label: "系统默认 (System Default)", fontFamily: "" },
        ...loaded,
      ];
    }
  } catch {}

  unlistenScanStarted = await listen("library-scan-started", () => {
    scanning.value = true;
  });
  unlistenScanComplete = await listen("library-scan-complete", async () => {
    scanning.value = false;
    await playback.init();
  });
});

onUnmounted(() => {
  unlistenScanStarted?.();
  unlistenScanComplete?.();
});
</script>

<template>
  <div class="h-full overflow-y-auto select-none">
    <div class="max-w-3xl mx-auto p-8 space-y-8">
    <!-- Header -->
    <div>
      <h1 class="text-display font-bold text-foreground tracking-tight">偏好设置</h1>
      <p class="text-body-sm text-muted-foreground mt-1">自定义外观、音乐资料库及播放选项</p>
    </div>

    <!-- 1. 外观与主题 -->
    <section class="space-y-3.5">
      <div class="flex items-center gap-2">
        <Sliders class="w-4 h-4 text-primary" />
        <h2 class="text-title-sm font-semibold text-foreground">外观与主题</h2>
      </div>

      <div class="bg-elevated/70 border border-border/80 rounded-2xl p-5 space-y-4 shadow-1">
        <div>
          <label class="text-sm font-medium text-foreground">颜色模式</label>
          <p class="text-xs text-muted-foreground mt-0.5">选择您喜爱的界面色调</p>
        </div>

        <div class="grid grid-cols-3 gap-3">
          <button
            class="flex flex-col items-center gap-2 p-3.5 rounded-xl border transition-all duration-150 text-center"
            :class="
              theme === 'system'
                ? 'border-primary bg-primary/10 text-primary font-medium shadow-sm ring-1 ring-primary'
                : 'border-border/70 hover:border-border bg-background/50 hover:bg-background text-muted-foreground hover:text-foreground'
            "
            @click="setTheme('system')"
          >
            <Laptop class="w-5 h-5" />
            <span class="text-xs">跟随系统</span>
          </button>

          <button
            class="flex flex-col items-center gap-2 p-3.5 rounded-xl border transition-all duration-150 text-center"
            :class="
              theme === 'light'
                ? 'border-primary bg-primary/10 text-primary font-medium shadow-sm ring-1 ring-primary'
                : 'border-border/70 hover:border-border bg-background/50 hover:bg-background text-muted-foreground hover:text-foreground'
            "
            @click="setTheme('light')"
          >
            <Sun class="w-5 h-5" />
            <span class="text-xs">浅色模式</span>
          </button>

          <button
            class="flex flex-col items-center gap-2 p-3.5 rounded-xl border transition-all duration-150 text-center"
            :class="
              theme === 'dark'
                ? 'border-primary bg-primary/10 text-primary font-medium shadow-sm ring-1 ring-primary'
                : 'border-border/70 hover:border-border bg-background/50 hover:bg-background text-muted-foreground hover:text-foreground'
            "
            @click="setTheme('dark')"
          >
            <Moon class="w-5 h-5" />
            <span class="text-xs">深色模式</span>
          </button>
        </div>
      </div>
    </section>

    <!-- 2. 字体选择 -->
    <section class="space-y-3.5">
      <div class="flex items-center gap-2">
        <Type class="w-4 h-4 text-primary" />
        <h2 class="text-title-sm font-semibold text-foreground">界面字体</h2>
      </div>

      <div class="bg-elevated/70 border border-border/80 rounded-2xl p-5 space-y-3 shadow-1">
        <div class="flex items-center justify-between">
          <div>
            <label class="text-sm font-medium text-foreground">首选字体</label>
            <p class="text-xs text-muted-foreground mt-0.5">选择系统中安装的字体渲染界面文字</p>
          </div>
          
          <button
            ref="fontTriggerRef"
            class="h-9 px-3.5 rounded-lg border border-border bg-background hover:bg-elevated-hover flex items-center justify-between gap-3 text-sm font-medium text-foreground min-w-[200px] transition-colors"
            @click="toggleFontPicker"
          >
            <span class="truncate">{{ currentFontLabel }}</span>
            <ChevronDown class="w-4 h-4 text-muted-foreground flex-shrink-0" />
          </button>
        </div>
      </div>
    </section>

    <!-- 3. 音乐资料库 -->
    <section class="space-y-3.5">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <Folder class="w-4 h-4 text-primary" />
          <h2 class="text-title-sm font-semibold text-foreground">音乐目录</h2>
        </div>
        <Button variant="secondary" size="sm" :disabled="scanning" @click="handleRescan">
          <RefreshCw class="w-3.5 h-3.5 mr-1.5" :class="{ 'animate-spin': scanning }" />
          {{ scanning ? "正在重新扫描……" : "重新扫描" }}
        </Button>
      </div>

      <div class="bg-elevated/70 border border-border/80 rounded-2xl p-5 space-y-4 shadow-1">
        <div v-if="loading" class="text-sm text-muted-foreground py-2">加载中……</div>
        <div v-else-if="musicDirs.length === 0" class="text-sm text-muted-foreground py-2">
          暂无配置的音乐目录
        </div>
        <div v-else class="space-y-2">
          <div
            v-for="dir in musicDirs"
            :key="dir"
            class="flex items-center justify-between p-3 rounded-xl bg-background/80 border border-border/70"
          >
            <div class="flex items-center gap-2.5 min-w-0 pr-3">
              <FolderOpen class="w-4 h-4 text-primary flex-shrink-0" />
              <span class="text-sm font-mono text-foreground truncate">{{ dir }}</span>
            </div>
            <button
              class="text-muted-foreground hover:text-destructive p-1.5 rounded-md hover:bg-destructive/10 transition-colors flex-shrink-0"
              title="移除此目录"
              @click="handleRemove(dir)"
            >
              <FolderX class="w-4 h-4" />
            </button>
          </div>
        </div>

        <Button variant="default" size="md" @click="handleAdd">
          <FolderOpen class="w-4 h-4 mr-2" />
          添加音乐文件夹
        </Button>
      </div>
    </section>

    <!-- 4. 关于 Caw -->
    <section class="space-y-3.5">
      <div class="flex items-center gap-2">
        <Info class="w-4 h-4 text-primary" />
        <h2 class="text-title-sm font-semibold text-foreground">关于</h2>
      </div>

      <div class="bg-elevated/70 border border-border/80 rounded-2xl p-5 space-y-4 shadow-1">
        <div class="flex items-center gap-4">
          <div class="w-12 h-12 rounded-xl bg-primary flex items-center justify-center text-primary-foreground shadow-md">
            <Disc3 class="w-7 h-7" />
          </div>
          <div>
            <h3 class="text-base font-bold text-foreground">Caw Music Player</h3>
            <p class="text-xs text-muted-foreground mt-0.5">版本 0.2.0 · 纯 Rust 音频引擎驱动</p>
          </div>
        </div>

        <div class="flex flex-wrap gap-2 pt-2 border-t border-border/60">
          <span class="px-2.5 py-1 rounded-md bg-background text-xs text-muted-foreground border border-border/60">Rust + Tauri v2</span>
          <span class="px-2.5 py-1 rounded-md bg-background text-xs text-muted-foreground border border-border/60">Symphonia + CPAL</span>
          <span class="px-2.5 py-1 rounded-md bg-background text-xs text-muted-foreground border border-border/60">Vue 3 + TailwindCSS</span>
        </div>
      </div>
    </section>
    </div>
  </div>

  <!-- Virtualized Font Picker Dropdown Modal / Portal -->
  <Teleport to="body">
    <div
      v-if="fontPickerOpen"
      class="fixed inset-0 z-dropdown"
      @click="fontPickerOpen = false"
    >
      <div
        class="absolute bg-elevated border border-border rounded-xl shadow-3 w-80 max-h-80 overflow-hidden flex flex-col backdrop-blur-md"
        :style="{ left: dropdownLeft + 'px', top: dropdownTop + 'px' }"
        @click.stop
      >
        <div class="p-2.5 border-b border-border/70">
          <input
            v-model="fontSearch"
            type="text"
            placeholder="搜索已安装字体……"
            class="w-full px-3 py-1.5 text-xs bg-background border border-border rounded-lg outline-none focus:border-primary text-foreground"
          />
        </div>
        <div ref="fontListRef" class="flex-1 overflow-y-auto p-1 max-h-64">
          <div :style="{ height: fontVirtualizer?.getTotalSize() + 'px' }" class="relative">
            <button
              v-for="row in virtualFontRows"
              :key="row.key"
              class="w-full text-left px-3 py-2 text-xs rounded-lg flex items-center justify-between hover:bg-elevated-hover transition-colors absolute top-0 left-0"
              :style="{
                height: row.size + 'px',
                transform: `translateY(${row.start}px)`,
                fontFamily: row._font.fontFamily,
              }"
              @click="selectFont(row._font.id)"
            >
              <span class="truncate">{{ row._font.label }}</span>
              <Check v-if="currentFont === row._font.id" class="w-3.5 h-3.5 text-primary flex-shrink-0" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>