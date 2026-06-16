<template>
  <aside class="w-60 flex-shrink-0 bg-sidebar flex flex-col border-r border-border">
    <!-- Section label -->
    <p class="text-overline px-3 pt-4 pb-1">资料库</p>
    <!-- Nav items -->
    <nav class="flex flex-col gap-0.5 px-2 pb-2">
      <button
        v-for="item in navItems"
        :key="item.id"
        class="flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium transition-colors duration-120 relative text-left"
        :class="
          view.nav === item.id
            ? 'bg-elevated text-foreground'
            : 'text-muted-foreground hover:text-foreground hover:bg-elevated-hover'
        "
        @click="onNav(item.id)"
      >
        <div
          v-if="view.nav === item.id"
          class="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-5 bg-primary rounded-r-full"
        />
        <component :is="item.icon" class="w-4 h-4 flex-shrink-0" />
        <span>{{ item.label }}</span>
      </button>
    </nav>

    <!-- Saved playlists (under 播放列表) -->
    <div
      v-if="view.nav === ('playlists' as string) || (plStore.playlists.length > 0 && view.nav !== ('playlists' as string))"
      class="border-t border-border mx-3 my-1"
    />
    <div v-if="plStore.playlists.length > 0 && view.nav === ('playlists' as string)" class="flex-1 overflow-auto px-2">
      <div
        v-for="pl in plStore.playlists"
        :key="pl.id"
        class="flex items-center gap-2 px-3 py-1.5 rounded-md cursor-pointer text-sm transition-colors duration-120 group"
        :class="
          pl.id === plStore.currentPlaylistId
            ? 'bg-elevated text-foreground'
            : 'text-muted-foreground hover:text-foreground hover:bg-elevated-hover'
        "
        @click="plStore.openPlaylist(pl.id)"
      >
        <ListMusic class="w-3.5 h-3.5 flex-shrink-0" />
        <span class="truncate flex-1">{{ pl.name }}</span>
        <span class="text-caption text-faint-foreground flex-shrink-0">{{ pl.trackCount }}</span>
      </div>
    </div>

    <!-- Bottom controls -->
    <div class="px-3 py-3 border-t border-border flex flex-col gap-2">
      <button
        v-if="view.nav === ('playlists' as string)"
        class="flex items-center gap-2 px-3 py-1.5 rounded-md text-sm text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors"
        @click="showCreateDialog = true"
      >
        <Plus class="w-4 h-4" />
        <span>新建播放列表</span>
      </button>
      <p class="text-caption text-faint-foreground">
        {{ view.filteredTracks.length }} tracks
      </p>
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
import { ref, onMounted } from "vue";
import { Music, Users, Album, ListMusic, Folder, Plus, Settings } from "lucide-vue-next";
import { useViewStore } from "@/stores/view";
import { usePlaylistStore } from "@/stores/playlists";
import PlaylistDialog from "@/components/PlaylistDialog.vue";

const view = useViewStore();
const plStore = usePlaylistStore();

const showCreateDialog = ref(false);

const navItems = [
  { id: "all-music" as const, label: "全部音乐", icon: Music },
  { id: "artists" as const, label: "艺术家", icon: Users },
  { id: "albums" as const, label: "专辑", icon: Album },
  { id: "playlists" as const, label: "播放列表", icon: ListMusic },
  { id: "folders" as const, label: "文件夹", icon: Folder },
  { id: "settings" as const, label: "设置", icon: Settings },
];

function onNav(id: any) {
  if (id !== 'playlists') {
    plStore.closePlaylist();
  }
  view.setNav(id);
  if (id === 'playlists') {
    plStore.refresh();
  }
}

async function handleCreate(name: string) {
  await plStore.create(name);
  showCreateDialog.value = false;
}

onMounted(() => {
  plStore.refresh();
});
</script>
