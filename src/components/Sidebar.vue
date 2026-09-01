<template>
  <aside
    class="flex-shrink-0 bg-sidebar flex flex-col border-r border-border transition-all duration-200 overflow-hidden relative select-none"
    :class="collapsed ? 'w-16' : 'w-56'"
  >
    <!-- Collapse toggle header -->
    <div class="flex items-center justify-between px-3 pt-3 pb-1" :class="collapsed ? 'justify-center px-0' : ''">
      <p v-if="!collapsed" class="text-overline text-faint-foreground pl-1">资料库</p>
      <button
        class="flex items-center justify-center w-7 h-7 rounded-lg text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors"
        :title="collapsed ? '展开侧边栏' : '收起侧边栏'"
        @click="collapsed = !collapsed"
      >
        <PanelLeftClose v-if="!collapsed" class="w-4 h-4" />
        <PanelLeftOpen v-else class="w-4 h-4" />
      </button>
    </div>

    <!-- Main Nav items -->
    <nav class="flex flex-col gap-1 px-2.5 py-1">
      <button
        v-for="item in navItems"
        :key="item.id"
        class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-150 relative text-left group"
        :class="[
          view.nav === item.id && !isPlaylistOpen
            ? 'bg-elevated text-primary font-semibold shadow-sm ring-1 ring-border'
            : 'text-muted-foreground hover:text-foreground hover:bg-elevated-hover/80',
          collapsed ? 'justify-center px-0' : ''
        ]"
        :title="collapsed ? item.label : undefined"
        @click="onNav(item.id)"
      >
        <!-- Active indicator bar -->
        <div
          v-if="view.nav === item.id && !isPlaylistOpen"
          class="absolute left-1 top-1/2 -translate-y-1/2 w-1 h-4 bg-primary rounded-full shadow-sm"
        />
        <component
          :is="item.icon"
          class="w-4 h-4 flex-shrink-0 transition-transform group-hover:scale-110"
          :class="view.nav === item.id && !isPlaylistOpen ? 'text-primary' : ''"
        />
        <span v-if="!collapsed" class="truncate">{{ item.label }}</span>
      </button>
    </nav>

    <!-- Playlist Section Divider & Header (hidden when collapsed) -->
    <template v-if="!collapsed">
      <div class="px-3 pt-4 pb-1.5 flex items-center justify-between border-t border-border mt-2">
        <p class="text-overline text-faint-foreground pl-1">播放列表</p>
        <button
          class="w-5 h-5 rounded-md flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors"
          title="新建播放列表"
          @click="showCreateDialog = true"
        >
          <Plus class="w-3.5 h-3.5" />
        </button>
      </div>

      <!-- Playlist items list -->
      <div class="flex-1 overflow-y-auto px-2.5 space-y-0.5">
        <div
          v-if="plStore.playlists.length === 0"
          class="px-3 py-3 text-xs text-faint-foreground text-center"
        >
          暂无播放列表
        </div>
        <div
          v-for="pl in plStore.playlists"
          :key="pl.id"
          class="flex items-center gap-2 px-3 py-1.5 rounded-lg cursor-pointer text-sm transition-all duration-120 group relative"
          :class="
            plStore.currentPlaylistId === pl.id && view.nav === 'playlists'
              ? 'bg-elevated text-primary font-semibold shadow-sm ring-1 ring-border'
              : 'text-muted-foreground hover:text-foreground hover:bg-elevated-hover/80'
          "
          @click="plStore.openPlaylist(pl.id)"
        >
          <!-- Active bar -->
          <div
            v-if="plStore.currentPlaylistId === pl.id && view.nav === 'playlists'"
            class="absolute left-1 top-1/2 -translate-y-1/2 w-1 h-3.5 bg-primary rounded-full"
          />
          <ListMusic
            class="w-3.5 h-3.5 flex-shrink-0"
            :class="plStore.currentPlaylistId === pl.id && view.nav === 'playlists' ? 'text-primary' : 'text-muted-foreground'"
          />
          <span class="truncate flex-1">{{ pl.name }}</span>
          <span class="text-caption text-faint-foreground flex-shrink-0 px-1.5 py-0.5 rounded bg-elevated-hover/60">
            {{ pl.trackCount }}
          </span>
        </div>
      </div>
    </template>
    <div v-else class="flex-1" />

    <!-- Bottom info footer -->
    <div class="p-3 border-t border-border flex items-center justify-between" :class="collapsed ? 'justify-center p-2' : ''">
      <template v-if="!collapsed">
        <span class="text-caption text-faint-foreground truncate">
          {{ view.filteredTracks.length }} 首歌曲
        </span>
        <button
          class="w-6 h-6 rounded-md flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors"
          title="快速新建列表"
          @click="showCreateDialog = true"
        >
          <Plus class="w-3.5 h-3.5" />
        </button>
      </template>
      <button
        v-else
        class="w-8 h-8 rounded-lg flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors"
        title="新建播放列表"
        @click="showCreateDialog = true"
      >
        <Plus class="w-4 h-4" />
      </button>
    </div>

    <!-- Create dialog -->
    <PlaylistDialog
      :open="showCreateDialog"
      title="新建播放列表"
      placeholder="输入播放列表名称"
      confirm-text="创建"
      @close="showCreateDialog = false"
      @confirm="handleCreate"
    />
  </aside>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import {
  Music,
  Users,
  Album,
  ListMusic,
  Folder,
  Plus,
  Settings,
  Cloud,
  PanelLeftClose,
  PanelLeftOpen,
} from "lucide-vue-next";
import { useViewStore } from "@/stores/view";
import { usePlaylistStore } from "@/stores/playlists";
import { useNeteaseStore } from "@/stores/netease";
import PlaylistDialog from "@/components/PlaylistDialog.vue";

const view = useViewStore();
const plStore = usePlaylistStore();
const netease = useNeteaseStore();

const collapsed = ref(false);
const showCreateDialog = ref(false);

const isPlaylistOpen = computed(
  () => view.nav === "playlists" && plStore.currentPlaylistId !== null,
);

const navItems = computed(() => {
  const items = [
    { id: "all-music", label: "全部音乐", icon: Music },
    { id: "artists", label: "艺术家", icon: Users },
    { id: "albums", label: "专辑", icon: Album },
    { id: "folders", label: "文件夹", icon: Folder },
  ];
  if (netease.enabled) {
    items.push({ id: "netease", label: "网易云音乐", icon: Cloud });
  }
  items.push({ id: "settings", label: "设置", icon: Settings });
  return items;
});

function onNav(id: string) {
  if (id === "playlists") {
    view.setNav("playlists");
  } else {
    plStore.closePlaylist();
    view.setNav(id as any);
  }
}

async function handleCreate(name: string) {
  showCreateDialog.value = false;
  if (!name.trim()) return;
  await plStore.create(name.trim());
}

onMounted(() => {
  plStore.refresh();
});
</script>
