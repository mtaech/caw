<template>
  <div class="h-full overflow-y-auto select-none bg-background p-6">
    <!-- Breadcrumb header -->
    <div class="flex items-center gap-1.5 pb-4 border-b border-border/60 flex-wrap">
      <div class="w-8 h-8 rounded-lg bg-elevated border border-border/60 flex items-center justify-center flex-shrink-0 mr-1">
        <Folder class="w-4 h-4 text-primary" />
      </div>

      <template v-if="breadcrumbs.length">
        <template v-for="(seg, i) in breadcrumbs" :key="seg.path">
          <ChevronRight class="w-3.5 h-3.5 text-faint-foreground flex-shrink-0" />
          <button
            class="text-sm px-2 py-1 rounded-md transition-colors hover:bg-elevated hover:text-primary truncate max-w-[220px]"
            :class="i === breadcrumbs.length - 1 ? 'text-foreground font-semibold bg-elevated/70' : 'text-muted-foreground'"
            @click="openDir(seg.path)"
          >
            {{ seg.label }}
          </button>
        </template>
      </template>
      <h1 v-else class="text-title text-foreground font-bold">音乐文件夹</h1>
    </div>

    <!-- Subdirectories Grid/List -->
    <div v-if="subDirs.length > 0" class="pt-5 pb-3">
      <p class="text-overline mb-3">
        子文件夹（{{ subDirs.length }}）
      </p>
      <div class="grid gap-2.5" :style="{ gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))' }">
        <button
          v-for="(dir, idx) in subDirs"
          :key="idx"
          class="flex items-center gap-3 px-3.5 py-2.5 rounded-xl text-sm text-foreground bg-elevated/40 hover:bg-elevated border border-border/40 hover:border-border transition-all duration-150 text-left group hover:shadow-1"
          @click="openDir(dir)"
        >
          <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0 group-hover:scale-110 transition-transform">
            <Folder class="w-4 h-4 text-primary" />
          </div>
          <span class="truncate font-medium flex-1">{{ dirName(dir) }}</span>
        </button>
      </div>
    </div>

    <!-- Tracks in current folder -->
    <div v-if="folderTracks.length > 0" class="pt-5 pb-6">
      <p class="text-overline mb-3">
        当前目录歌曲（{{ folderTracks.length }}）
      </p>
      <div class="space-y-1">
        <div
          v-for="(track, idx) in folderTracks"
          :key="track.id"
          class="flex items-center gap-3 px-3 py-2 rounded-lg cursor-pointer hover:bg-elevated-hover transition-colors group"
          :class="{ 'bg-primary/10 text-foreground font-medium': track.id === playback.currentTrackId }"
          @dblclick="playTrack(track.id)"
        >
          <div class="w-6 flex justify-center flex-shrink-0">
            <div
              v-if="playback.currentTrackId === track.id && playback.isPlaying"
              class="flex items-center gap-0.5 h-3.5"
            >
              <span class="w-0.5 h-2.5 bg-primary rounded-full eq-bar-1" />
              <span class="w-0.5 h-3.5 bg-primary rounded-full eq-bar-2" />
              <span class="w-0.5 h-2 bg-primary rounded-full eq-bar-3" />
            </div>
            <Play
              v-else-if="playback.currentTrackId === track.id"
              class="w-3.5 h-3.5 text-primary fill-primary"
            />
            <span v-else class="text-caption text-faint-foreground font-mono">{{ idx + 1 }}</span>
          </div>

          <div class="flex-1 min-w-0">
            <p class="text-sm truncate" :class="{ 'text-primary font-semibold': track.id === playback.currentTrackId }">
              {{ track.title }}
            </p>
            <p class="text-xs text-muted-foreground truncate">{{ track.artist }}</p>
          </div>

          <span class="text-xs text-muted-foreground flex-shrink-0 w-14 text-right font-mono">
            {{ formatDuration(track.duration_secs) }}
          </span>
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-if="!currentPath && musicRoots.length === 0"
      class="flex flex-col items-center justify-center h-72 text-muted-foreground gap-3"
    >
      <div class="w-16 h-16 rounded-2xl bg-elevated flex items-center justify-center border border-border/60">
        <FolderOpen class="w-8 h-8 text-muted-foreground/50" />
      </div>
      <p class="text-body font-medium">尚无音乐目录</p>
      <p class="text-xs text-faint-foreground">请先在设置中添加您的音乐文件夹</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { Folder, FolderOpen, ChevronRight, Play } from "lucide-vue-next";
import { usePlaybackStore } from "@/stores/playback";
import { formatDuration } from "@/lib/utils";
import * as api from "@/lib/tauri";

const playback = usePlaybackStore();

const props = defineProps<{
  currentPath: string | null;
}>();

const emit = defineEmits<{
  (e: "navigate", path: string | null): void;
}>();

const musicRoots = ref<string[]>([]);

onMounted(async () => {
  musicRoots.value = await api.getMusicDirs();
});

const breadcrumbs = computed(() => {
  if (!props.currentPath) return [];
  const parts = props.currentPath.split("/").filter(Boolean);
  const res: { label: string; path: string }[] = [];
  let cur = "";
  for (const p of parts) {
    cur += "/" + p;
    res.push({ label: p, path: cur });
  }
  return res;
});

const subDirs = computed(() => {
  const dirs = new Set<string>();
  for (const t of playback.library) {
    if (!props.currentPath) {
      for (const root of musicRoots.value) {
        if (t.path.startsWith(root)) {
          const rel = t.path.slice(root.length).replace(/^\//, "");
          const seg = rel.split("/")[0];
          if (seg && rel.includes("/")) dirs.add(root + "/" + seg);
        }
      }
    } else if (t.path.startsWith(props.currentPath + "/")) {
      const rel = t.path.slice(props.currentPath.length + 1);
      const seg = rel.split("/")[0];
      if (seg && rel.includes("/")) dirs.add(props.currentPath + "/" + seg);
    }
  }
  return Array.from(dirs).sort();
});

const folderTracks = computed(() => {
  if (!props.currentPath) return [];
  return playback.library.filter((t) => {
    const dir = t.path.substring(0, t.path.lastIndexOf("/"));
    return dir === props.currentPath;
  });
});

function dirName(path: string) {
  return path.split("/").pop() || path;
}

function openDir(path: string) {
  emit("navigate", path);
}

function playTrack(trackId: number) {
  const ids = folderTracks.value.map((t) => t.id);
  playback.playTracks(ids, trackId);
}
</script>
