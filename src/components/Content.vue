<template>
  <main class="flex-1 flex flex-col overflow-hidden bg-background relative">
    <!-- Search bar (only in data-browsing views, NOT in settings/placeholders) -->
    <div
      v-if="!view.detail && !showPlaylistDetail && view.nav !== 'settings' && view.nav !== 'folders' && view.nav !== 'playlists'"
      class="flex items-center gap-3 px-5 py-2.5 border-b border-border flex-shrink-0 bg-background/50 backdrop-blur-sm z-10"
    >
      <div class="flex-1 max-w-md flex items-center gap-2.5 px-3 py-1.5 rounded-lg bg-elevated border border-border focus-within:border-primary focus-within:ring-2 focus-within:ring-primary/20 transition-all duration-150">
        <Search class="w-4 h-4 text-muted-foreground flex-shrink-0" />
        <input
          v-model="view.searchQuery"
          placeholder="搜索歌曲、艺人或专辑……"
          class="flex-1 bg-transparent border-none outline-none text-sm text-foreground placeholder:text-muted-foreground/70"
        />
        <button
          v-if="view.searchQuery"
          class="text-muted-foreground hover:text-foreground flex-shrink-0 p-0.5 rounded"
          title="清空搜索"
          @click="view.setSearch('')"
        >
          <X class="w-3.5 h-3.5" />
        </button>
      </div>

      <!-- Result count indicator if searching -->
      <span v-if="view.searchQuery" class="text-xs text-muted-foreground font-medium animate-in fade-in-0 duration-150">
        找到 {{ view.filteredTracks.length }} 首歌曲
      </span>
    </div>

    <!-- Content area -->
    <div class="flex-1 overflow-hidden relative">
      <!-- Playlist detail (takes precedence when a playlist is open) -->
      <PlaylistDetail v-if="showPlaylistDetail" />

      <!-- Album / Artist detail -->
      <AlbumDetail v-else-if="view.detail?.type === 'album'" />
      <ArtistDetail v-else-if="view.detail?.type === 'artist'" />

      <!-- Main views -->
      <AlbumGrid v-else-if="view.nav === 'albums'" />
      <TrackTable v-else-if="view.nav === 'all-music'" />
      <ArtistList v-else-if="view.nav === 'artists'" />
      <Placeholder
        v-else-if="view.nav === 'playlists'"
        title="播放列表"
        desc="在左侧栏创建或选择一个播放列表以查看其中的曲目"
        icon-type="playlists"
      />
      <FolderView
        v-else-if="view.nav === 'folders'"
        :currentPath="currentFolderPath"
        @navigate="currentFolderPath = $event"
      />
      <Settings v-else-if="view.nav === 'settings'" />
      <div v-else class="flex-1 flex items-center justify-center">
        <p class="text-muted-foreground">请选择一个视图</p>
      </div>
    </div>
  </main>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Search, X } from "lucide-vue-next";
import { useViewStore } from "@/stores/view";
import { usePlaylistStore } from "@/stores/playlists";
import TrackTable from "@/components/TrackTable.vue";
import AlbumGrid from "@/components/AlbumGrid.vue";
import AlbumDetail from "@/components/AlbumDetail.vue";
import ArtistDetail from "@/components/ArtistDetail.vue";
import ArtistList from "@/components/ArtistList.vue";
import PlaylistDetail from "@/components/PlaylistDetail.vue";
import Placeholder from "@/components/Placeholder.vue";
import Settings from "@/components/Settings.vue";
import FolderView from "@/components/FolderView.vue";

const view = useViewStore();
const plStore = usePlaylistStore();

const showPlaylistDetail = computed(
  () => view.nav === "playlists" && plStore.currentPlaylistId !== null,
);

const currentFolderPath = ref<string | null>(null);

watch(
  () => view.nav,
  () => {
    if (view.nav !== "folders") currentFolderPath.value = null;
  },
);
</script>
