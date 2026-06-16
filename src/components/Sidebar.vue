<template>
  <aside class="w-60 flex-shrink-0 bg-sidebar flex flex-col border-r border-border">
    <!-- Nav items -->
    <nav class="flex-1 flex flex-col gap-0.5 px-2 py-4">
      <button
        v-for="item in navItems"
        :key="item.id"
        class="flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium transition-colors duration-120 relative text-left"
        :class="
          view.nav === item.id
            ? 'bg-elevated text-foreground'
            : 'text-muted-foreground hover:text-foreground hover:bg-elevated-hover'
        "
        @click="view.setNav(item.id)"
      >
        <!-- Active indicator -->
        <div
          v-if="view.nav === item.id"
          class="absolute left-0 w-0.5 h-5 bg-primary rounded-r-full"
        />
        <component :is="item.icon" class="w-4 h-4 flex-shrink-0 ml-0.5" />
        <span>{{ item.label }}</span>
      </button>
    </nav>

    <!-- Library stats -->
    <div class="px-4 py-3 border-t border-border">
      <p class="text-caption text-faint-foreground">
        {{ view.filteredTracks.length }} tracks
      </p>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { useViewStore } from "@/stores/view";
import { Music, Users, Album, ListMusic, Folder } from "lucide-vue-next";

const view = useViewStore();

const navItems = [
  { id: "all-music" as const, label: "全部音乐", icon: Music },
  { id: "artists" as const, label: "艺术家", icon: Users },
  { id: "albums" as const, label: "专辑", icon: Album },
  { id: "playlists" as const, label: "播放列表", icon: ListMusic },
  { id: "folders" as const, label: "文件夹", icon: Folder },
];
</script>
