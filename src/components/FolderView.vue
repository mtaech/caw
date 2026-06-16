<template>
  <div class="h-full overflow-y-auto">
    <!-- Breadcrumb / header -->
    <div
      v-if="currentPath"
      class="flex items-center gap-2 px-6 pt-6 pb-3 border-b border-border"
    >
      <button
        class="text-muted-foreground hover:text-foreground transition-colors"
        @click="goUp"
      >
        <ChevronLeft class="w-4 h-4" />
      </button>
      <span class="text-body text-foreground">{{ currentLabel }}</span>
    </div>
    <div v-else class="px-6 pt-6 pb-3 border-b border-border">
      <h1 class="text-title text-foreground">文件夹</h1>
    </div>

    <!-- Subdirectories -->
    <div v-if="subDirs.length > 0" class="px-6 pt-4 pb-2">
      <p class="text-caption text-muted-foreground uppercase tracking-wider mb-2">
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
      <p class="text-caption text-muted-foreground uppercase tracking-wider mb-2">
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
          <span class="text-body-sm text-muted-foreground w-6 text-right flex-shrink-0">
            {{ playback.currentTrackId === track.id ? '♫' : idx + 1 }}
          </span>
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
      v-if="!currentPath && subDirs.length === 0 && folderTracks.length === 0"
      class="flex flex-col items-center justify-center h-full text-muted-foreground gap-2"
    >
      <FolderOpen class="w-12 h-12" />
      <p class="text-body">尚无音乐目录</p>
      <p class="text-body-sm">请先在设置中添加音乐目录</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Folder, FolderOpen, ChevronLeft } from "lucide-vue-next";
import { usePlaybackStore } from "@/stores/playback";

const playback = usePlaybackStore();

const props = defineProps<{
  currentPath: string | null;
}>();

const emit = defineEmits<{
  (e: "navigate", path: string | null): void;
}>();

const currentLabel = computed(() => {
  if (!props.currentPath) return "";
  return props.currentPath.replace(/\/+$/, "").split("/").pop() || props.currentPath;
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
 * When currentPath is null, returns top-level dirs (e.g. "/home", "/media").
 */
const subDirs = computed(() => {
  const prefix = props.currentPath ?? "";
  // Normalize: ensure prefix doesn't end with / (except root "/")
  const normalized = prefix === "/" ? "/" : prefix.replace(/\/+$/, "");
  const prefixSlash = normalized === "" ? "" : normalized + "/";

  const children = new Set<string>();
  for (const dir of allDirEntries.value) {
    // Skip if outside our prefix
    if (!dir.startsWith(prefixSlash)) continue;
    if (dir === normalized) continue;
    // Get the next path segment
    const rest = dir.slice(prefixSlash.length);
    const slashIdx = rest.indexOf("/");
    const child = slashIdx === -1 ? rest : rest.slice(0, slashIdx);
    if (child) children.add(normalized ? normalized + "/" + child : child);
  }
  return Array.from(children).sort();
});

/**
 * Tracks whose file is directly inside currentPath (not in subdirectories).
 */
const folderTracks = computed(() => {
  const prefix = props.currentPath;
  const prefixSlash = prefix === null ? "" : (prefix === "/" ? "/" : prefix + "/");
  return playback.library
    .filter((t) => {
      const d = t.path.substring(0, t.path.lastIndexOf("/") + 1);
      if (prefix === null) return d === "" || d === "/";
      return prefixSlash === "" ? d === "" : d.startsWith(prefixSlash) && d.lastIndexOf("/", d.length - 2) === prefixSlash.length - 1;
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

function goUp() {
  if (!props.currentPath) return;
  const normalized = props.currentPath.replace(/\/+$/, "");
  const slashIdx = normalized.lastIndexOf("/");
  if (slashIdx <= 0) {
    emit("navigate", "/");
  } else {
    emit("navigate", normalized.slice(0, slashIdx));
  }
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
