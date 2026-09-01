<template>
  <div class="h-full flex flex-col bg-background select-none overflow-hidden">
    <div class="flex items-center justify-between gap-4 px-6 py-4 border-b border-border bg-background/60 backdrop-blur-md flex-shrink-0">
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-xl bg-primary/10 text-primary flex items-center justify-center border border-primary/20 shadow-sm">
          <Cloud class="w-5 h-5" />
        </div>
        <div>
          <div class="flex items-center gap-2">
            <h1 class="text-title-sm font-bold text-foreground">网易云音乐</h1>
            <span class="px-2 py-0.5 rounded-full text-2xs font-medium bg-primary/10 text-primary border border-primary/20">OpenAPI</span>
          </div>
          <p class="text-xs text-muted-foreground mt-0.5">
            <template v-if="netease.loggedIn && netease.userName">已登录 · {{ netease.userName }}</template>
            <template v-else>在线推荐、搜索我的曲库</template>
          </p>
        </div>
      </div>

      <nav class="flex items-center gap-1 p-1 rounded-xl bg-elevated/60 border border-border/60 flex-shrink-0">
        <button v-for="tab in tabs" :key="tab.id"
          class="px-3 py-1.5 text-xs font-medium rounded-lg transition-colors"
          :class="netease.activeTab === tab.id ? 'bg-primary text-primary-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground hover:bg-elevated-hover'"
          @click="switchTab(tab.id)">{{ tab.label }}</button>
      </nav>

      <button
        class="flex items-center gap-1.5 px-3 py-1.5 text-xs text-muted-foreground hover:text-foreground rounded-lg hover:bg-elevated border border-border/60 transition-colors"
        title="网易云音乐配置" @click="view.setNav('settings')">
        <Settings class="w-3.5 h-3.5" /><span>配置</span>
      </button>
    </div>

    <div class="flex-1 overflow-y-auto p-6">
      <div v-if="!netease.deviceComplete" class="mb-6 p-4 rounded-2xl bg-destructive/5 border border-destructive/20 flex items-center justify-between gap-4">
        <div class="flex items-start gap-3">
          <Key class="w-5 h-5 text-destructive flex-shrink-0 mt-0.5" />
          <div>
            <h3 class="text-sm font-semibold text-foreground">device 参数未配置</h3>
            <p class="text-xs text-muted-foreground mt-0.5">开放平台校验 device 的 os / channel / brand，必须与 appId 注册一致。请在 <code class="font-mono">.env</code> 配置 <code class="font-mono">CAW_NETEASE_OS</code> / <code class="font-mono">_CHANNEL</code> / <code class="font-mono">_BRAND</code>。</p>
          </div>
        </div>
      </div>

      <div v-if="!netease.loggedIn && netease.activeTab === 'recommend'"
        class="h-96 flex flex-col items-center justify-center gap-4 text-center">
        <div class="w-16 h-16 rounded-3xl bg-elevated border border-border flex items-center justify-center text-muted-foreground shadow-sm">
          <QrCode class="w-8 h-8 text-primary/70" />
        </div>
        <div class="space-y-1">
          <h2 class="text-sm font-semibold text-foreground">扫码登录后查看每日推荐</h2>
          <p class="text-xs text-muted-foreground max-w-sm">每日推荐、心动模式、我的歌单与最近播放都需要登录你的网易云账号。</p>
        </div>
        <Button variant="default" size="sm" @click="openLogin">立即登录</Button>
      </div>

      <div v-else-if="netease.activeTab === 'recommend'" class="space-y-8">
        <div class="flex items-end justify-between">
          <div>
            <h2 class="text-lg font-bold text-foreground">每日推荐</h2>
            <p class="text-xs text-muted-foreground mt-0.5">根据你的听歌喜好，每天为你换一批</p>
          </div>
          <Button variant="ghost" size="sm" :disabled="netease.isLoadingRecs" @click="netease.loadRecommendations()">
            <RefreshCw class="w-3.5 h-3.5 mr-1.5" />刷新
          </Button>
        </div>

        <div v-if="netease.isLoadingRecs" class="h-48 flex flex-col items-center justify-center gap-3 text-muted-foreground">
          <Loader2 class="w-7 h-7 animate-spin text-primary" /><p class="text-xs">正在加载推荐……</p>
        </div>
        <div v-else-if="netease.recError" class="flex flex-col items-center justify-center gap-3 text-destructive py-12">
          <AlertCircle class="w-7 h-7" />
          <p class="text-xs text-center whitespace-pre-line max-w-md">{{ netease.recError }}</p>
        </div>
        <template v-else>
          <div v-if="netease.dailyRecs.length" class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
            <div v-for="song in netease.dailyRecs" :key="song.id"
              class="group rounded-xl bg-elevated/40 border border-border/60 p-3 hover:bg-elevated-hover/60 transition-all cursor-pointer" @dblclick="playOnline(song)">
              <div class="relative aspect-square rounded-lg overflow-hidden bg-background border border-border/40 mb-2.5">
                <img v-if="song.pic_url" :src="song.pic_url + '?param=200y200'" class="w-full h-full object-cover" loading="lazy" />
                <Music v-else class="w-8 h-8 text-muted-foreground/40 absolute inset-0 m-auto" />
                <div class="absolute inset-0 flex items-center justify-center bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity">
                  <button class="w-9 h-9 rounded-full bg-primary text-primary-foreground flex items-center justify-center shadow-lg hover:scale-105 transition-transform" title="播放" @click.stop="playOnline(song)">
                    <Play class="w-4 h-4 ml-0.5" />
                  </button>
                </div>
              </div>
              <p class="text-xs font-medium text-foreground truncate">{{ song.title }}</p>
              <p class="text-2xs text-muted-foreground truncate mt-0.5">{{ song.artist }}</p>
            </div>
          </div>
          <div v-else class="flex flex-col items-center justify-center gap-3 py-16 text-muted-foreground">
            <Disc3 class="w-8 h-8 animate-spin-slow text-primary/60" /><p class="text-xs">暂无推荐，点击「刷新」重试</p>
          </div>

          <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
            <section>
              <div class="flex items-center justify-between mb-3"><h3 class="text-sm font-semibold text-foreground">最近播放</h3></div>
              <div v-if="netease.playHistory.length" class="space-y-1">
                <div v-for="song in netease.playHistory.slice(0, 10)" :key="song.id"
                  class="flex items-center gap-3 px-2 py-2 rounded-lg hover:bg-elevated-hover/60 cursor-pointer group" @dblclick="playOnline(song)">
                  <div class="w-9 h-9 rounded-md overflow-hidden bg-background border border-border/40 flex-shrink-0">
                    <img v-if="song.pic_url" :src="song.pic_url + '?param=72y72'" class="w-full h-full object-cover" />
                    <Music v-else class="w-4 h-4 text-muted-foreground/40 m-auto" />
                  </div>
                  <div class="min-w-0 flex-1">
                    <p class="text-xs font-medium text-foreground truncate">{{ song.title }}</p>
                    <p class="text-2xs text-muted-foreground truncate">{{ song.artist }} · {{ song.album }}</p>
                  </div>
                  <Play class="w-3.5 h-3.5 text-muted-foreground opacity-0 group-hover:opacity-100" />
                </div>
              </div>
              <p v-else class="text-xs text-muted-foreground py-4">暂无播放记录</p>
            </section>
            <section>
              <div class="flex items-center justify-between mb-3"><h3 class="text-sm font-semibold text-foreground">我的歌单</h3></div>
              <div v-if="netease.myPlaylists.length" class="grid grid-cols-2 gap-3">
                <div v-for="pl in netease.myPlaylists.slice(0, 6)" :key="pl.id"
                  class="rounded-xl bg-elevated/40 border border-border/60 p-2.5 hover:bg-elevated-hover/60 cursor-pointer" @dblclick="openPlaylist(pl)">
                  <div class="w-full aspect-square rounded-lg overflow-hidden bg-background border border-border/40 mb-2">
                    <img v-if="pl.cover_url" :src="pl.cover_url + '?param=160y160'" class="w-full h-full object-cover" />
                    <ListMusic v-else class="w-7 h-7 text-muted-foreground/40 m-auto" />
                  </div>
                  <p class="text-2xs text-foreground truncate font-medium">{{ pl.name }}</p>
                  <p class="text-3xs text-muted-foreground mt-0.5">{{ pl.track_count }} 首</p>
                </div>
              </div>
              <p v-else class="text-xs text-muted-foreground py-4">暂无歌单</p>
            </section>
          </div>
        </template>
      </div>

      <div v-else-if="netease.activeTab === 'search'" class="space-y-4">
        <div class="flex items-center gap-2.5 px-3.5 py-2 rounded-xl bg-elevated border border-border focus-within:border-primary focus-within:ring-2 focus-within:ring-primary/20 transition-all duration-150">
          <Search class="w-4 h-4 text-muted-foreground flex-shrink-0" />
          <input v-model="searchInput" placeholder="搜索网易云单曲、歌手、专辑……"
            class="flex-1 bg-transparent border-none outline-none text-xs text-foreground placeholder:text-muted-foreground/70" @keydown.enter="handleSearch" />
          <button v-if="searchInput" class="text-muted-foreground hover:text-foreground p-0.5 rounded" title="清空"
            @click="searchInput = ''; netease.searchResults = []"><X class="w-3.5 h-3.5" /></button>
          <button class="px-2.5 py-1 text-2xs font-medium bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors flex-shrink-0"
            :disabled="netease.isSearching || !searchInput.trim()" @click="handleSearch">
            <Loader2 v-if="netease.isSearching" class="w-3 h-3 animate-spin" /><span v-else>搜索</span>
          </button>
        </div>

        <div v-if="netease.isSearching" class="h-48 flex flex-col items-center justify-center gap-3 text-muted-foreground">
          <Loader2 class="w-7 h-7 animate-spin text-primary" /><p class="text-xs">正在检索网易云曲库……</p>
        </div>
        <div v-else-if="netease.searchError" class="flex flex-col items-center justify-center gap-3 text-destructive py-12">
          <AlertCircle class="w-7 h-7" />
          <p class="text-xs text-center whitespace-pre-line max-w-md">{{ netease.searchError }}</p>
          <Button variant="secondary" size="sm" @click="handleSearch">重试</Button>
        </div>
        <div v-else-if="netease.searchResults.length" class="rounded-2xl border border-border bg-elevated/40 overflow-hidden shadow-1">
          <table class="w-full text-left text-xs">
            <thead class="bg-elevated/80 border-b border-border text-muted-foreground font-medium">
              <tr>
                <th class="w-12 py-3 pl-4">#</th><th class="py-3 px-3">歌曲</th><th class="py-3 px-3">歌手</th>
                <th class="py-3 px-3">专辑</th><th class="w-20 py-3 px-3 text-right">时长</th><th class="w-24 py-3 pr-4 text-right">操作</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-border/50">
              <tr v-for="(song, idx) in netease.searchResults" :key="song.id"
                class="hover:bg-elevated-hover/80 transition-colors group cursor-pointer" @dblclick="playOnline(song)">
                <td class="py-3 pl-4 text-muted-foreground font-mono">{{ idx + 1 }}</td>
                <td class="py-3 px-3 font-medium text-foreground flex items-center gap-2.5">
                  <div class="w-8 h-8 rounded-md bg-background overflow-hidden flex-shrink-0 border border-border/50 flex items-center justify-center">
                    <img v-if="song.pic_url" :src="song.pic_url + '?param=64y64'" class="w-full h-full object-cover" loading="lazy" />
                    <Music v-else class="w-4 h-4 text-muted-foreground/60" />
                  </div>
                  <div class="min-w-0">
                    <p class="truncate font-medium text-foreground">{{ song.title }}</p>
                    <span v-if="song.vip_only" class="text-3xs px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-500 font-mono">VIP</span>
                  </div>
                </td>
                <td class="py-3 px-3 text-muted-foreground truncate">{{ song.artist }}</td>
                <td class="py-3 px-3 text-muted-foreground truncate">{{ song.album }}</td>
                <td class="py-3 px-3 text-right text-muted-foreground font-mono">{{ formatDuration(song.duration_secs) }}</td>
                <td class="py-3 pr-4 text-right"><div class="flex items-center justify-end gap-1">
                  <button class="p-1.5 rounded-lg text-muted-foreground hover:text-primary hover:bg-primary/10 transition-colors" title="查看歌词" @click.stop="openLyricDialog(song)"><FileText class="w-3.5 h-3.5" /></button>
                  <button class="p-1.5 rounded-lg text-muted-foreground hover:text-primary hover:bg-primary/10 transition-colors" title="播放" @click.stop="playOnline(song)"><Play class="w-3.5 h-3.5" /></button>
                </div></td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-else class="h-64 flex flex-col items-center justify-center gap-3 text-center text-muted-foreground">
          <Search class="w-8 h-8 text-muted-foreground/50" /><p class="text-xs">输入关键词开始搜索</p>
        </div>
      </div>

      <div v-else class="space-y-4">
        <div class="flex items-center justify-between">
          <h2 class="text-sm font-semibold text-foreground">我的歌单</h2>
          <Button variant="ghost" size="sm" :disabled="netease.isLoadingRecs" @click="netease.loadRecommendations()"><RefreshCw class="w-3.5 h-3.5 mr-1.5" />刷新</Button>
        </div>
        <div v-if="netease.myPlaylists.length" class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6 gap-4">
          <div v-for="pl in netease.myPlaylists" :key="pl.id"
            class="group rounded-xl bg-elevated/40 border border-border/60 p-3 hover:bg-elevated-hover/60 transition-all cursor-pointer" @dblclick="openPlaylist(pl)">
            <div class="w-full aspect-square rounded-lg overflow-hidden bg-background border border-border/40 mb-2.5">
              <img v-if="pl.cover_url" :src="pl.cover_url + '?param=200y200'" class="w-full h-full object-cover" loading="lazy" />
              <ListMusic v-else class="w-8 h-8 text-muted-foreground/40 m-auto" />
            </div>
            <p class="text-xs text-foreground truncate font-medium">{{ pl.name }}</p>
            <p class="text-3xs text-muted-foreground mt-0.5">{{ pl.track_count }} 首</p>
          </div>
        </div>
        <div v-else-if="!netease.isLoadingRecs" class="h-48 flex flex-col items-center justify-center gap-3 text-muted-foreground">
          <ListMusic class="w-8 h-8 text-muted-foreground/40" /><p class="text-xs">暂无歌单</p>
        </div>
      </div>
    </div>

    <Teleport to="body">
      <div v-if="showLogin" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-150" @click="closeLogin">
        <div class="bg-elevated border border-border rounded-2xl w-full max-w-md p-6 flex flex-col items-center gap-4 shadow-3" @click.stop>
          <div class="text-center">
            <h3 class="text-sm font-bold text-foreground">扫码登录网易云音乐</h3>
            <p class="text-xs text-muted-foreground mt-1">使用手机网易云音乐 App 扫码</p>
          </div>
          <div v-if="!netease.qrSession && !netease.qrPoll" class="py-6 text-muted-foreground"><Loader2 class="w-7 h-7 animate-spin text-primary" /></div>
          <div v-else-if="netease.qrPoll?.status === 'error'" class="py-4 flex flex-col items-center gap-3 text-destructive">
            <AlertCircle class="w-7 h-7" />
            <p class="text-xs text-center whitespace-pre-line">{{ netease.qrPoll.message }}</p>
            <Button variant="secondary" size="sm" @click="netease.startLogin()">重试</Button>
          </div>
          <div v-else class="flex flex-col items-center gap-3">
            <img v-if="qrCodeDataUrl" :src="qrCodeDataUrl" class="w-52 h-52 rounded-lg border border-border/60 bg-white p-2" alt="登录二维码" />
            <p class="text-xs text-muted-foreground">{{ netease.qrPoll?.message || '等待扫码……' }}</p>
          </div>
          <div class="flex items-center gap-2">
            <Button variant="ghost" size="sm" @click="closeLogin">关闭</Button>
            <Button variant="secondary" size="sm" :disabled="netease.isQrPolling" @click="netease.startLogin()">刷新二维码</Button>
          </div>
        </div>
      </div>
    </Teleport>

    <Teleport to="body">
      <div v-if="showLyricModal" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-150" @click="showLyricModal = false">
        <div class="bg-elevated border border-border rounded-2xl w-full max-w-lg max-h-[75vh] flex flex-col shadow-3 overflow-hidden" @click.stop>
          <div class="p-4 border-b border-border flex items-center justify-between">
            <div class="min-w-0 pr-4">
              <h3 class="text-sm font-bold text-foreground truncate">{{ currentSong?.title }}</h3>
              <p class="text-xs text-muted-foreground truncate">{{ currentSong?.artist }} · {{ currentSong?.album }}</p>
            </div>
            <button class="p-1 rounded-lg text-muted-foreground hover:text-foreground hover:bg-elevated-hover" @click="showLyricModal = false"><X class="w-4 h-4" /></button>
          </div>
          <div class="flex-1 overflow-y-auto p-6 space-y-2 text-center select-text">
            <div v-if="loadingLyric" class="py-12 flex flex-col items-center gap-2 text-muted-foreground"><Loader2 class="w-6 h-6 animate-spin text-primary" /><span class="text-xs">加载歌词中……</span></div>
            <div v-else-if="currentLyric" class="text-xs text-foreground/90 whitespace-pre-line leading-relaxed font-sans">{{ currentLyric }}</div>
            <div v-else class="py-12 text-xs text-muted-foreground">暂无歌词</div>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from "vue";
import QRCode from "qrcode";
import {
  Cloud, Search, X, Play, FileText, Key, Disc3, Music,
  Settings, AlertCircle, Loader2, RefreshCw, QrCode, ListMusic,
} from "lucide-vue-next";
import { useNeteaseStore } from "@/stores/netease";
import { useViewStore } from "@/stores/view";
import Button from "@/components/ui/Button.vue";
import * as api from "@/lib/tauri";
import type { NeteasePlaylistDto, NeteaseSongDto } from "@/lib/tauri";

const netease = useNeteaseStore();
const view = useViewStore();

const tabs = [
  { id: "recommend" as const, label: "推荐" },
  { id: "search" as const, label: "搜索" },
  { id: "playlists" as const, label: "歌单" },
];

const searchInput = ref(netease.searchQuery);
const showLyricModal = ref(false);
const currentSong = ref<NeteaseSongDto | null>(null);
const currentLyric = ref("");
const loadingLyric = ref(false);
const showLogin = ref(false);

const qrCodeDataUrl = ref("");

function switchTab(id: "recommend" | "search" | "playlists") {
  netease.activeTab = id;
  if (id === "recommend" && !netease.dailyRecs.length) netease.loadRecommendations();
}

function openLogin() {
  showLogin.value = true;
  netease.startLogin();
}
function closeLogin() {
  showLogin.value = false;
  if (netease.qrSession) netease.qrSession = null;
}

function formatDuration(secs: number): string {
  if (!secs || isNaN(secs)) return "0:00";
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

async function handleSearch() {
  if (!searchInput.value.trim()) return;
  netease.searchQuery = searchInput.value.trim();
  await netease.search();
}

const playingOnlineId = ref<string | null>(null);

async function playOnline(song: NeteaseSongDto) {
  if (playingOnlineId.value) return;
  playingOnlineId.value = song.id;
  try {
    await api.neteasePlayOnline(song);
  } catch (err: any) {
    console.error("caw: play online song error", err);
  } finally {
    playingOnlineId.value = null;
  }
}

async function openLyricDialog(song: NeteaseSongDto) {
  currentSong.value = song;
  showLyricModal.value = true;
  loadingLyric.value = true;
  currentLyric.value = "";
  try {
    const res = await api.neteaseGetLyric(song.id);
    currentLyric.value = res.lyric;
  } catch (err) {
    console.error("caw: get lyric error", err);
    currentLyric.value = "获取歌词失败";
  } finally {
    loadingLyric.value = false;
  }
}

async function openPlaylist(pl: NeteasePlaylistDto) {
  try {
    const tracks = await api.neteasePlaylistTracks(pl.id);
    console.log("caw: playlist tracks:", pl.name, tracks.length);
  } catch (err) {
    console.error("caw: open playlist error", err);
  }
}

let pollTimer: ReturnType<typeof setInterval> | null = null;
async function generateQr(url: string) {
  try {
    qrCodeDataUrl.value = await QRCode.toDataURL(url, { width: 260, margin: 2 });
  } catch (e) {
    console.error("caw: qr generate error", e);
    qrCodeDataUrl.value = "";
  }
}
watch(() => netease.qrSession, (session) => {
  stopPoll();
  if (session) {
    generateQr(session.qr_code_url);
    if (showLogin.value) {
      pollTimer = setInterval(async () => {
        const confirmed = await netease.pollLogin();
        if (confirmed) {
          await netease.loadRecommendations();
          stopPoll();
          closeLogin();
        }
      }, 2500);
    }
  }
});
function stopPoll() {
  if (pollTimer) { clearInterval(pollTimer); pollTimer = null; }
}

onMounted(async () => {
  await netease.loadConfig();
  if (netease.activeTab === "recommend" && netease.loggedIn) await netease.loadRecommendations();
});
onBeforeUnmount(() => stopPoll());
</script>

<style scoped>
.animate-spin-slow { animation: spin 8s linear infinite; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
</style>
