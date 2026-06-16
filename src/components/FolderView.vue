<template>
  <div class="h-full overflow-y-auto">
    <!-- Breadcrumb header -->
    <div class="flex items-center gap-1 px-6 pt-6 pb-3 border-b border-border">
      <Folder class="w-4 h-4 text-muted-foreground flex-shrink-0" />
      <template v-if="breadcrumbs.length">
        <template v-for="(seg, i) in breadcrumbs" :key="seg.path">
          <ChevronRight class="w-3.5 h-3.5 text-faint-foreground flex-shrink-0" />
          <button
            class="text-body transition-colors hover:text-primary truncate max-w-[200px]"
            :class="i === breadcrumbs.length - 1 ? 'text-foreground font-medium' : 'text-muted-foreground'"
            @click="openDir(seg.path)"
          >{{ seg.label }}</button>
        </template>
      </template>
      <h1 v-else class="text-title text-foreground">文件夹</h1>
    </div>

    <!-- Subdirectories -->
    <div v-if="subDirs.length > 0" class="px-6 pt-4 pb-2">
      <p class="text-overline mb-2">
        文件夹（{{ subDirs.length }}）
      </p>
      <div class="space-y-0.5">
        <button
          v-for="(dir, idx) in subDirs"
          :key="idx"
          class="w-full flex items-center gap-3 px-3 py-2 rounded-md text-sm text-foreground hover:bg-elevated-hover transition-colors text-left"
          @click="openDir(dir)"
        >
          <Folder class="w-4 h-4 text-primary flex-shrink-0" />
          <span class="truncate">{{ dirName(dir) }}</span>
        </button>
      </div>
    </div>

    <!-- Tracks in current folder -->
    <div v-if="folderTracks.length > 0" class="px-6 pt-4 pb-6">
      <p class="text-overline mb-2">
        歌曲（{{ folderTracks.length }}）
      </p>
      <div class="space-y-0.5">
        <div
          v-for="(track, idx) in folderTracks"
          :key="track.id"
          class="flex items-center gap-3 px-3 py-1.5 rounded-md cursor-pointer hover:bg-elevated-hover transition-colors group"
          :class="{ 'bg-primary/10': track.id === playback.currentTrackId }"
          @dblclick="playTrack(track.id)"
        >
          <div class="w-6 flex justify-center flex-shrink-0">
            <Play
              v-if="playback.currentTrackId === track.id"
              class="w-3 h-3 text-primary fill-primary"
            />
            <span v-else class="text-body-sm text-faint-foreground">{{ idx + 1 }}</span>
          </div>
          <div class="flex-1 min-w-0">
            <p class="text-body text-foreground truncate">{{ track.title }}</p>
          </div>
          <span class="text-body-sm text-muted-foreground flex-shrink-0 w-14 text-right">
            {{ fmt(track.duration_secs) }}
          </span>
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-if="!currentPath && musicRoots.length === 0"
      class="flex flex-col items-center justify-center h-full text-muted-foreground gap-2"
    >
      <FolderOpen class="w-12 h-12" />
      <p class="text-body">尚无音乐目录</p>
      <p class="text-body-sm">请先在设置中添加音乐目录</p>
    </div>
    <!-- Scanning / empty library state -->
    <div
      v-else-if="!currentPath && playback.library.length === 0"
      class="flex flex-col items-center justify-center h-full text-muted-foreground gap-2"
    >
      <div class="w-8 h-8 border-2 border-primary border-t-transparent rounded-full animate-spin" />
      <p class="text-body-sm">曲库扫描中……</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { Folder, FolderOpen, ChevronRight, Play } from "lucide-vue-next";
import { usePlaybackStore } from "@/stores/playback";
import * as api from "@/lib/tauri";

const playback = usePlaybackStore();

const props = defineProps<{
  currentPath: string | null;
}>();

const emit = defineEmits<{
  (e: "navigate", path: string | null): void;
}>();

/** Configured music directories from Settings (root level) */
const musicRoots = ref<string[]>([]);

onMounted(async () => {
  try {
    musicRoots.value = await api.getMusicDirs();
  } catch (e) {
    console.error("caw: failed to load music dirs for folder view", e);
  }
});

/**
 * Full clickable path segments for the breadcrumb. When the current path
 * sits under a configured music root, the first segment is that root.
 */
const breadcrumbs = computed(() => {
  const path = props.currentPath;
  if (!path) return [];
  const norm = path.replace(/\/+$/, "");
  const root = musicRoots.value
    .map((r) => r.replace(/\/+$/, ""))
    .find((r) => norm === r || norm.startsWith(r + "/"));
  const segments: { label: string; path: string }[] = [];
  if (root) {
    segments.push({ label: dirName(root), path: root });
    const rest = norm.slice(root.length).replace(/^\/+/, "");
    if (rest) {
      let acc = root;
      for (const part of rest.split("/")) {
        if (!part) continue;
        acc += "/" + part;
        segments.push({ label: part, path: acc });
      }
    }
  } else {
    const parts = norm.replace(/^\/+$/, "").split("/");
    let acc = "";
    for (const part of parts) {
      if (!part) continue;
      acc += "/" + part;
      segments.push({ label: part, path: acc });
    }
  }
  return segments;
});

/**
 * Walk EVERY track's path and collect every unique directory entry
 * (all ancestors, not just the immediate parent). Returns sorted list
 * of full directory paths like ["/a", "/a/b", "/a/b/c", ...].
 */
const allDirEntries = computed(() => {
  const dirs = new Set<string>();
  for (const t of playback.library) {
    const p = t.path;
    // Collect every ancestor directory of this track's path
    let idx = p.lastIndexOf("/");
    while (idx > 0) {
      dirs.add(p.slice(0, idx));
      idx = p.lastIndexOf("/", idx - 1);
    }
    // Also add the root "/" if there's a leading slash
    if (p.startsWith("/")) dirs.add("/");
  }
  return Array.from(dirs).sort();
});

/**
 * Immediate subdirectories under the currentPath.
 * When currentPath is null, returns the user's configured music directories.
 */
const subDirs = computed(() => {
  const prefix = props.currentPath;
  if (prefix === null) {
    // Root level: show configured music directories
    return musicRoots.value.slice().sort();
  }
  if (prefix === "/") {
    // Under filesystem root: show top-level dirs from track paths
    const top = new Set<string>();
    for (const dir of allDirEntries.value) {
      const parts = dir.replace(/^\/+/, "").split("/");
      if (parts.length > 0 && parts[0]) top.add("/" + parts[0]);
    }
    return Array.from(top).sort();
  }
  const prefixSlash = prefix.replace(/\/+$/, "") + "/";
  const children = new Set<string>();
  for (const dir of allDirEntries.value) {
    if (!dir.startsWith(prefixSlash) || dir === prefix.replace(/\/+$/, "")) continue;
    const rest = dir.slice(prefixSlash.length);
    const slashIdx = rest.indexOf("/");
    const child = slashIdx === -1 ? rest : rest.slice(0, slashIdx);
    if (child) children.add(prefixSlash + child);
  }
  return Array.from(children).sort();
});

/**
 * Tracks whose file is directly inside currentPath (not in subdirectories).
 */
const folderTracks = computed(() => {
  const prefix = props.currentPath;
  const prefixSlash = prefix === null ? null : (prefix === "/" ? "/" : prefix + "/");
  return playback.library
    .filter((t) => {
      const parentDir = t.path.substring(0, t.path.lastIndexOf("/") + 1);
      // At root: no tracks directly at the filesystem root
      if (prefix === null || prefix === "/") return false;
      return parentDir === prefixSlash;
    })
    .sort((a, b) => a.track_number - b.track_number || a.title.localeCompare(b.title));
});

function dirName(fullPath: string): string {
  if (fullPath === "/") return "/";
  return fullPath.replace(/\/+$/, "").split("/").pop() || fullPath;
}

function openDir(path: string) {
  emit("navigate", path);
}

function playTrack(id: number) {
  const ids = playback.library.map((t) => t.id);
  playback.playTracks(ids, id);
}

function fmt(sec: number): string {
  if (!sec || sec < 0) return "0:00";
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}
</script>
