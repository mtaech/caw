<template>
  <main class="flex-1 flex flex-col overflow-hidden">
    <!-- Search bar (shown in list/playlist views, hidden in album/artist detail) -->
    <div
      v-if="!view.detail && !showPlaylistDetail"
      class="flex items-center gap-2 px-4 py-2 border-b border-border flex-shrink-0"
    >
      <Search class="w-4 h-4 text-muted-foreground flex-shrink-0" />
      <Input
        v-model="view.searchQuery"
        placeholder="搜索标题、艺术家或专辑……"
        class="border-none bg-transparent px-0 focus-visible:ring-0"
      />
      <button
        v-if="view.searchQuery"
        class="text-muted-foreground hover:text-foreground flex-shrink-0"
        @click="view.setSearch('')"
      >
        <X class="w-4 h-4" />
      </button>
    </div>

    <!-- Content area -->
    <div class="flex-1 overflow-y-auto">
      <!-- Playlist detail (takes precedence when a playlist is open) -->
      <PlaylistDetail v-if="showPlaylistDetail" />

      <!-- Album / Artist detail -->
      <AlbumDetail v-else-if="view.detail?.type === 'album'" />
      <ArtistDetail v-else-if="view.detail?.type === 'artist'" />

      <!-- Main views -->
      <AlbumGrid v-else-if="view.nav === 'albums'" />
      <TrackTable v-else-if="view.nav === 'all-music'" />
      <ArtistList v-else-if="view.nav === 'artists'" />
      <Placeholder v-else-if="view.nav === 'playlists'" title="播放列表" desc="在侧栏创建或选择一个播放列表" icon-type="playlists" />
      <Placeholder v-else-if="view.nav === 'folders'" title="文件夹浏览" desc="即将推出" icon-type="folders" />
      <Settings v-else-if="view.nav === 'settings'" />
      <div v-else class="flex-1 flex items-center justify-center">
        <p class="text-muted-foreground">请选择一个视图</p>
      </div>
    </div>
  </main>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Search, X } from "lucide-vue-next";
import { useViewStore } from "@/stores/view";
import { usePlaylistStore } from "@/stores/playlists";
import Input from "@/components/ui/Input.vue";
import TrackTable from "@/components/TrackTable.vue";
import AlbumGrid from "@/components/AlbumGrid.vue";
import AlbumDetail from "@/components/AlbumDetail.vue";
import ArtistDetail from "@/components/ArtistDetail.vue";
import ArtistList from "@/components/ArtistList.vue";
import PlaylistDetail from "@/components/PlaylistDetail.vue";
import Placeholder from "@/components/Placeholder.vue";
import Settings from "@/components/Settings.vue";

const view = useViewStore();
const plStore = usePlaylistStore();

const showPlaylistDetail = computed(
  () => view.nav === "playlists" && plStore.currentPlaylistId !== null,
);
</script>
