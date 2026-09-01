import { defineStore } from "pinia";
import { ref } from "vue";
import * as api from "@/lib/tauri";
import type {
  NeteaseConfigDto,
  NeteaseConnectionStatus,
  NeteasePlaylistDto,
  NeteaseQrPollDto,
  NeteaseQrSessionDto,
  NeteaseSongDto,
  UpdateNeteaseConfigDto,
} from "@/lib/tauri";

export const useNeteaseStore = defineStore("netease", () => {
  // Config / connection state
  const enabled = ref(false);
  const appId = ref("");
  const hasPrivateKey = ref(false);
  const anonymousMode = ref(true);
  const apiHost = ref("");
  const deviceComplete = ref(false);
  const loggedIn = ref(false);
  const userName = ref<string | null>(null);

  const loaded = ref(false);
  const isTesting = ref(false);
  const testStatus = ref<NeteaseConnectionStatus | null>(null);

  // Active tab inside NetEase View: 'recommend' | 'search' | 'playlists'
  const activeTab = ref<"recommend" | "search" | "playlists">("recommend");

  // Search state
  const searchQuery = ref("");
  const searchResults = ref<NeteaseSongDto[]>([]);
  const isSearching = ref(false);
  const searchError = ref<string | null>(null);

  // Recommend state
  const dailyRecs = ref<NeteaseSongDto[]>([]);
  const playHistory = ref<NeteaseSongDto[]>([]);
  const myPlaylists = ref<NeteasePlaylistDto[]>([]);
  const isLoadingRecs = ref(false);
  const recError = ref<string | null>(null);

  // QR login state
  const qrSession = ref<NeteaseQrSessionDto | null>(null);
  const qrPoll = ref<NeteaseQrPollDto | null>(null);
  const isQrPolling = ref(false);

  function applyConfig(cfg: NeteaseConfigDto) {
    enabled.value = cfg.enabled;
    appId.value = cfg.app_id;
    hasPrivateKey.value = cfg.has_private_key;
    anonymousMode.value = cfg.anonymous_mode;
    apiHost.value = cfg.api_host;
    deviceComplete.value = cfg.device_complete;
    loggedIn.value = cfg.logged_in;
    userName.value = cfg.user_name;
  }

  async function loadConfig() {
    try {
      const cfg = await api.getNeteaseConfig();
      applyConfig(cfg);
      loaded.value = true;
    } catch (err) {
      console.error("caw: failed to load netease config", err);
    }
  }

  async function updateConfig(update: UpdateNeteaseConfigDto) {
    try {
      const res = await api.setNeteaseConfig(update);
      applyConfig(res);
      testStatus.value = null;
      return res;
    } catch (err) {
      console.error("caw: failed to update netease config", err);
      throw err;
    }
  }

  async function testConnection() {
    isTesting.value = true;
    testStatus.value = null;
    try {
      const res = await api.testNeteaseConnection();
      testStatus.value = res;
      return res;
    } catch (err: any) {
      testStatus.value = {
        ok: false,
        message: err?.toString() || "连接测试失败",
        user_name: null,
      };
      return testStatus.value;
    } finally {
      isTesting.value = false;
    }
  }

  async function search(query?: string) {
    const q = query ?? searchQuery.value;
    if (!q.trim()) {
      searchResults.value = [];
      return;
    }
    isSearching.value = true;
    searchError.value = null;
    try {
      const res = await api.neteaseSearchSongs(q.trim());
      searchResults.value = res.songs;
    } catch (err: any) {
      console.error("caw: netease search error", err);
      searchError.value = err?.toString() || "搜索发生错误";
      searchResults.value = [];
    } finally {
      isSearching.value = false;
    }
  }

  async function loadRecommendations() {
    isLoadingRecs.value = true;
    recError.value = null;
    try {
      const [daily, history, playlists] = await Promise.all([
        api.neteaseRecommendDaily(30),
        api.neteasePlayHistory(20),
        api.neteaseMyPlaylists(12),
      ]);
      dailyRecs.value = daily;
      playHistory.value = history;
      myPlaylists.value = playlists;
    } catch (err: any) {
      console.error("caw: netease recommendations error", err);
      recError.value = err?.toString() || "加载推荐失败";
    } finally {
      isLoadingRecs.value = false;
    }
  }

  async function startLogin() {
    try {
      qrSession.value = await api.neteaseStartQrLogin();
      qrPoll.value = null;
    } catch (err: any) {
      qrPoll.value = { status: "error", message: err?.toString() || "获取二维码失败", user: null };
    }
  }

  async function pollLogin(): Promise<boolean> {
    if (!qrSession.value) return false;
    isQrPolling.value = true;
    try {
      const res = await api.neteasePollQrLogin(qrSession.value.uni_key);
      qrPoll.value = res;
      if (res.status === "confirmed") {
        loggedIn.value = true;
        userName.value = res.user?.nickname ?? userName.value;
        qrSession.value = null;
        return true;
      }
      return false;
    } catch (err: any) {
      qrPoll.value = { status: "error", message: err?.toString() || "轮询失败", user: null };
      return false;
    } finally {
      isQrPolling.value = false;
    }
  }

  async function logout() {
    try {
      await api.neteaseLogout();
      loggedIn.value = false;
      userName.value = null;
    } catch (err) {
      console.error("caw: netease logout error", err);
    }
  }

  return {
    enabled,
    appId,
    hasPrivateKey,
    anonymousMode,
    apiHost,
    deviceComplete,
    loggedIn,
    userName,
    loaded,
    isTesting,
    testStatus,
    activeTab,
    searchQuery,
    searchResults,
    isSearching,
    searchError,
    dailyRecs,
    playHistory,
    myPlaylists,
    isLoadingRecs,
    recError,
    qrSession,
    qrPoll,
    isQrPolling,
    applyConfig,
    loadConfig,
    updateConfig,
    testConnection,
    search,
    loadRecommendations,
    startLogin,
    pollLogin,
    logout,
  };
});
