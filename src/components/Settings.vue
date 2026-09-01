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
  AppWindow,
  Minimize2,
  Power,
  ListMusic,
  Cloud,
  Key,
  ExternalLink,
  ShieldCheck,
  AlertCircle,
  Loader2,
  QrCode,
} from "lucide-vue-next";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { usePlaybackStore } from "@/stores/playback";
import { useNeteaseStore } from "@/stores/netease";
import { useViewStore } from "@/stores/view";
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
const netease = useNeteaseStore();
const view = useViewStore();
const { theme, setTheme } = useTheme();

// ── NetEase Cloud Music settings ──
const neteaseAppIdInput = ref("");
const neteasePrivateKeyInput = ref("");
const savingNetease = ref(false);
const saveSuccess = ref(false);

async function handleToggleNetease() {
  const next = !netease.enabled;
  try {
    await netease.updateConfig({ enabled: next });
  } catch (err) {
    console.error("Failed to toggle netease:", err);
  }
}

async function handleSaveNeteaseCredentials() {
  savingNetease.value = true;
  saveSuccess.value = false;
  try {
    await netease.updateConfig({
      enabled: netease.enabled,
      app_id: neteaseAppIdInput.value.trim(),
      private_key: neteasePrivateKeyInput.value.trim() ? neteasePrivateKeyInput.value.trim() : undefined,
    });
    neteasePrivateKeyInput.value = "";
    saveSuccess.value = true;
    setTimeout(() => {
      saveSuccess.value = false;
    }, 3000);
  } catch (err) {
    console.error("Failed to save netease credentials:", err);
  } finally {
    savingNetease.value = false;
  }
}

async function handleTestNetease() {
  await netease.testConnection();
}

async function handleStartNeteaseLogin() {
  view.setNav("netease");
}

async function handleNeteaseLogout() {
  await netease.logout();
}

const musicDirs = ref<string[]>([]);
const loading = ref(true);
const scanning = ref(false);

// ── Window & Behavior settings ──
const minimizeToTray = ref(true);
const closePrompt = ref(true);
const queueReplaceMode = ref(true);

async function loadWindowSettings() {
  try {
    minimizeToTray.value = await api.getMinimizeToTray();
    closePrompt.value = await api.getClosePrompt();
    queueReplaceMode.value = await api.getQueueReplaceMode();
  } catch (err) {
    console.error("Failed to load window settings:", err);
  }
}

async function handleSetMinimizeToTray(val: boolean) {
  minimizeToTray.value = val;
  try {
    await api.setMinimizeToTray(val);
  } catch (err) {
    console.error("Failed to set minimize to tray:", err);
  }
}

async function handleToggleClosePrompt() {
  closePrompt.value = !closePrompt.value;
  try {
    await api.setClosePrompt(closePrompt.value);
  } catch (err) {
    console.error("Failed to set close prompt:", err);
  }
}

async function handleToggleQueueReplaceMode() {
  queueReplaceMode.value = !queueReplaceMode.value;
  playback.queueReplaceMode = queueReplaceMode.value;
  try {
    await api.setQueueReplaceMode(queueReplaceMode.value);
  } catch (err) {
    console.error("Failed to set queue replace mode:", err);
  }
}

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
  await Promise.all([loadDirs(), loadWindowSettings(), netease.loadConfig()]);
  neteaseAppIdInput.value = netease.appId;
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

    <!-- 3. 常规与窗口行为 -->
    <section class="space-y-3.5">
      <div class="flex items-center gap-2">
        <AppWindow class="w-4 h-4 text-primary" />
        <h2 class="text-title-sm font-semibold text-foreground">常规与窗口</h2>
      </div>

      <div class="bg-elevated/70 border border-border/80 rounded-2xl p-5 space-y-4 shadow-1">
        <div>
          <label class="text-sm font-medium text-foreground">关闭窗口时</label>
          <p class="text-xs text-muted-foreground mt-0.5">选择点击窗口右上角关闭按钮时的默认操作</p>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <button
            type="button"
            class="flex items-start gap-3.5 p-3.5 rounded-xl border text-left transition-all duration-150"
            :class="
              minimizeToTray
                ? 'border-primary bg-primary/10 text-foreground ring-1 ring-primary shadow-sm'
                : 'border-border/70 hover:border-border bg-background/50 hover:bg-background text-muted-foreground hover:text-foreground'
            "
            @click="handleSetMinimizeToTray(true)"
          >
            <div
              class="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 transition-colors"
              :class="minimizeToTray ? 'bg-primary text-primary-foreground shadow-sm' : 'bg-elevated border border-border text-muted-foreground'"
            >
              <Minimize2 class="w-4 h-4" />
            </div>
            <div class="flex-1 min-w-0">
              <div class="text-xs font-semibold text-foreground">最小化到系统托盘</div>
              <p class="text-2xs text-muted-foreground mt-0.5">后台保持运行并继续播放音乐</p>
            </div>
          </button>

          <button
            type="button"
            class="flex items-start gap-3.5 p-3.5 rounded-xl border text-left transition-all duration-150"
            :class="
              !minimizeToTray
                ? 'border-primary bg-primary/10 text-foreground ring-1 ring-primary shadow-sm'
                : 'border-border/70 hover:border-border bg-background/50 hover:bg-background text-muted-foreground hover:text-foreground'
            "
            @click="handleSetMinimizeToTray(false)"
          >
            <div
              class="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 transition-colors"
              :class="!minimizeToTray ? 'bg-destructive text-white shadow-sm' : 'bg-elevated border border-border text-muted-foreground'"
            >
              <Power class="w-4 h-4" />
            </div>
            <div class="flex-1 min-w-0">
              <div class="text-xs font-semibold text-foreground">直接退出 Caw</div>
              <p class="text-2xs text-muted-foreground mt-0.5">停止所有播放并完全退出应用程序</p>
            </div>
          </button>
        </div>

        <div class="pt-3 border-t border-border/60 flex items-center justify-between">
          <div>
            <label class="text-xs font-medium text-foreground">关闭窗口时弹出提示确认</label>
            <p class="text-2xs text-muted-foreground mt-0.5">开启后每次点击关闭都会弹出操作询问窗口</p>
          </div>
          <button
            type="button"
            class="relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none"
            :class="closePrompt ? 'bg-primary' : 'bg-muted-foreground/30'"
            @click="handleToggleClosePrompt"
          >
            <span
              class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out"
              :class="closePrompt ? 'translate-x-5' : 'translate-x-0'"
            />
          </button>
        </div>
      </div>
    </section>

    <!-- 4. 播放行为 -->
    <section class="space-y-3.5">
      <div class="flex items-center gap-2">
        <ListMusic class="w-4 h-4 text-primary" />
        <h2 class="text-title-sm font-semibold text-foreground">播放偏好</h2>
      </div>

      <div class="bg-elevated/70 border border-border/80 rounded-2xl p-5 space-y-3 shadow-1">
        <div class="flex items-center justify-between">
          <div>
            <label class="text-sm font-medium text-foreground">双击歌曲时替换播放队列</label>
            <p class="text-xs text-muted-foreground mt-0.5">开启后双击曲目将清空原队列并从该曲目开始播放；关闭则仅在当前歌曲后插入</p>
          </div>
          <button
            type="button"
            class="relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none"
            :class="queueReplaceMode ? 'bg-primary' : 'bg-muted-foreground/30'"
            @click="handleToggleQueueReplaceMode"
          >
            <span
              class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out"
              :class="queueReplaceMode ? 'translate-x-5' : 'translate-x-0'"
            />
          </button>
        </div>
      </div>
    </section>

    <!-- 5. 音乐资料库 -->
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

    <!-- 6. 网易云音乐扩展 -->
    <section class="space-y-3.5">
      <div class="flex items-center gap-2">
        <Cloud class="w-4 h-4 text-primary" />
        <h2 class="text-title-sm font-semibold text-foreground">网易云音乐</h2>
      </div>

      <div class="bg-elevated/70 border border-border/80 rounded-2xl p-5 space-y-4 shadow-1">
        <!-- Master Toggle -->
        <div class="flex items-center justify-between">
          <div>
            <label class="text-sm font-medium text-foreground">启用网易云音乐服务</label>
            <p class="text-xs text-muted-foreground mt-0.5">
              开启后可在侧边栏访问在线搜索、推荐曲库与云端歌单；关闭时 Caw 仅作为纯本地播放器运行
            </p>
          </div>
          <button
            type="button"
            class="relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none"
            :class="netease.enabled ? 'bg-primary' : 'bg-muted-foreground/30'"
            @click="handleToggleNetease"
          >
            <span
              class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out"
              :class="netease.enabled ? 'translate-x-5' : 'translate-x-0'"
            />
          </button>
        </div>

        <!-- Credentials Form (Only shown when enabled) -->
        <div v-if="netease.enabled" class="pt-4 border-t border-border/60 space-y-4 animate-in fade-in-50 duration-200">
          <div class="flex items-center justify-between pb-1">
            <div class="flex items-center gap-1.5 text-xs text-muted-foreground">
              <Key class="w-3.5 h-3.5 text-primary" />
              <span>开放平台 API 凭证</span>
            </div>
            <a
              href="https://developer.music.163.com/st/developer/apply/account?type=INDIVIDUAL"
              target="_blank"
              rel="noopener"
              class="text-xs text-primary hover:underline flex items-center gap-1"
            >
              申请 API Key
              <ExternalLink class="w-3 h-3" />
            </a>
          </div>

          <!-- App ID Input -->
          <div class="space-y-1.5">
            <label class="text-xs font-medium text-foreground">App ID</label>
            <input
              v-model="neteaseAppIdInput"
              type="text"
              placeholder="例如：1000000000"
              class="w-full px-3 py-2 text-xs bg-background/90 border border-border rounded-xl outline-none focus:border-primary focus:ring-1 focus:ring-primary text-foreground font-mono transition-all"
            />
          </div>

          <!-- Private Key Input -->
          <div class="space-y-1.5">
            <div class="flex items-center justify-between">
              <label class="text-xs font-medium text-foreground">Private Key (私钥)</label>
              <span v-if="netease.hasPrivateKey" class="text-2xs text-emerald-500 font-medium flex items-center gap-1">
                <ShieldCheck class="w-3 h-3" />
                已存储有效私钥
              </span>
            </div>
            <textarea
              v-model="neteasePrivateKeyInput"
              :placeholder="netease.hasPrivateKey ? '私钥已保存。若需更新，请粘贴新的 RSA 私钥文本……' : '粘贴网易云开放平台颁发的 RSA Private Key……'"
              rows="3"
              class="w-full px-3 py-2 text-xs bg-background/90 border border-border rounded-xl outline-none focus:border-primary focus:ring-1 focus:ring-primary text-foreground font-mono resize-none transition-all"
            />
          </div>

          <!-- Test Connection Result Badge -->
          <div
            v-if="netease.testStatus"
            class="p-3 rounded-xl border text-xs flex items-start gap-2.5"
            :class="
              netease.testStatus.ok
                ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400'
                : 'bg-destructive/10 border-destructive/30 text-destructive'
            "
          >
            <ShieldCheck v-if="netease.testStatus.ok" class="w-4 h-4 flex-shrink-0 mt-0.5" />
            <AlertCircle v-else class="w-4 h-4 flex-shrink-0 mt-0.5" />
            <div class="flex-1">
              <p class="font-medium">{{ netease.testStatus.message }}</p>
              <p v-if="netease.testStatus.user_name" class="text-2xs opacity-80 mt-0.5">
                账号状态: {{ netease.testStatus.user_name }}
              </p>
            </div>
          </div>

  <!-- NetEase login rows -->
    <div class="flex items-center grow justify-between gap-2 rounded-xl border bg-elevated/50 px-3 py-2.5">
      <div class="flex items-center gap-2.5">
        <QrCode class="w-4 h-4 text-foreground flex-shrink-0" />
        <div>
          <p class="text-xs font-medium text-foreground">登录网易云账号</p>
          <p class="text-2xs text-muted-foreground mt-0.5">登录后才能获取每日推荐、我的歌单与最近播放</p>
        </div>
      </div>
      <div class="flex items-center gap-2 flex-shrink-0">
        <span v-if="netease.loggedIn" class="text-2xs text-emerald-400 font-medium">{{ netease.userName }}</span>
        <Button v-if="!netease.loggedIn" variant="outline" size="sm" :disabled="netease.isQrPolling" @click="handleStartNeteaseLogin">立即登录</Button>
        <Button v-else variant="ghost" size="sm" @click="handleNeteaseLogout">退出</Button>
      </div>
    </div>

    <div v-if="!netease.deviceComplete" class="p-3 rounded-xl border border-amber-500/30 bg-amber-500/5 text-xs text-amber-400 flex items-start gap-2.5">
      <AlertCircle class="w-4 h-4 flex-shrink-0 mt-0.5" />
      <div>
        <p class="font-medium">device 参数未配置</p>
        <p class="text-2xs opacity-90 mt-0.5">需在项目根目录 <code class="font-mono">.env</code> 设置
        <code class="font-mono">CAW_NETEASE_OS</code> / <code class="font-mono">_CHANNEL</code> / <code class="font-mono">_BRAND</code>，否则调用会被平台以 code 400 拒绝。</p>
      </div>
    </div>
          <!-- Action Buttons -->
          <div class="flex items-center gap-3 pt-1">
            <Button
              variant="default"
              size="sm"
              :disabled="savingNetease"
              @click="handleSaveNeteaseCredentials"
            >
              <Loader2 v-if="savingNetease" class="w-3.5 h-3.5 mr-1.5 animate-spin" />
              <Check v-else-if="saveSuccess" class="w-3.5 h-3.5 mr-1.5 text-emerald-400" />
              {{ saveSuccess ? "保存成功" : "保存配置" }}
            </Button>

            <Button
              variant="secondary"
              size="sm"
              :disabled="netease.isTesting || !netease.appId"
              @click="handleTestNetease"
            >
              <Loader2 v-if="netease.isTesting" class="w-3.5 h-3.5 mr-1.5 animate-spin" />
              {{ netease.isTesting ? "正在测试连接……" : "测试连接" }}
            </Button>
          </div>
        </div>
      </div>
    </section>

    <!-- 7. 关于 Caw -->
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
            <p class="text-xs text-muted-foreground mt-0.5">版本 0.3.3 · 纯 Rust 音频引擎驱动</p>
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