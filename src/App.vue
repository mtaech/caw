<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { usePlaybackStore } from "./stores/playback";

const store = usePlaybackStore();

onMounted(async () => {
  await store.init();
});

onUnmounted(() => {
  store.cleanup();
});

function onSeekChange(e: Event) {
  const input = e.target as HTMLInputElement;
  store.seekTo(parseFloat(input.value));
}

function onVolumeChange(e: Event) {
  const input = e.target as HTMLInputElement;
  store.setVolume(parseFloat(input.value));
}

function playTrack(trackId: number) {
  const ids = store.library.map((t) => t.id);
  store.playTracks(ids, trackId);
}

function formatTime(sec: number): string {
  if (!sec || sec < 0) return "0:00";
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function handleDblClick(trackId: number) {
  playTrack(trackId);
}

function handlePickFolder() {
  store.pickMusicFolder();
}
</script>

<template>
  <div class="h-screen w-screen flex flex-col overflow-hidden bg-background">
    <!-- Library scan prompt -->
    <template v-if="store.library.length === 0 && !store.loading">
      <div class="flex-1 flex flex-col items-center justify-center gap-4">
        <p class="text-foreground text-lg">No music library found.</p>
        <button
          class="px-6 py-2 rounded-md bg-primary text-white font-medium hover:opacity-90"
          @click="handlePickFolder"
        >
          Choose Music Folder
        </button>
      </div>
    </template>

    <!-- Track list -->
    <template v-else>
      <div class="flex-1 overflow-y-auto p-4">
        <table class="w-full text-left">
          <thead class="text-muted-foreground text-xs uppercase tracking-wider">
            <tr>
              <th class="px-2 py-1 w-8">#</th>
              <th class="px-2 py-1">Title</th>
              <th class="px-2 py-1">Artist</th>
              <th class="px-2 py-1">Album</th>
              <th class="px-2 py-1 w-16 text-right">Duration</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(track, idx) in store.library"
              :key="track.id"
              class="cursor-pointer hover:bg-elevated-hover transition-colors"
              :class="{
                'bg-primary/10': track.id === store.currentTrackId,
              }"
              @dblclick="handleDblClick(track.id)"
            >
              <td class="px-2 py-1 text-muted-foreground text-sm">{{ idx + 1 }}</td>
              <td class="px-2 py-1 text-foreground text-sm">{{ track.title }}</td>
              <td class="px-2 py-1 text-muted-foreground text-sm">{{ track.artist }}</td>
              <td class="px-2 py-1 text-muted-foreground text-sm">{{ track.album }}</td>
              <td class="px-2 py-1 text-muted-foreground text-sm text-right">
                {{ formatTime(track.duration_secs) }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>

    <!-- Player bar -->
    <footer class="h-20 flex-shrink-0 bg-elevated border-t border-border flex items-center px-4 gap-4">
      <!-- Now playing info -->
      <div class="flex items-center gap-3 w-48 flex-shrink-0">
        <div class="w-12 h-12 rounded-md bg-elevated-hover flex-shrink-0 overflow-hidden">
          <!-- Cover placeholder — P2 -->
        </div>
        <div class="min-w-0">
          <p class="text-foreground text-sm font-medium truncate">
            {{ store.currentTrack?.title ?? "—" }}
          </p>
          <p class="text-muted-foreground text-xs truncate">
            {{ store.currentTrack?.artist ?? "" }}
          </p>
        </div>
      </div>

      <!-- Transport controls -->
      <div class="flex items-center gap-3">
        <button class="text-muted-foreground hover:text-foreground text-lg" @click="store.prev()">⏮</button>
        <button
          class="w-10 h-10 rounded-full bg-foreground text-background flex items-center justify-center text-lg font-bold hover:opacity-80"
          @click="store.togglePlay()"
        >
          {{ store.isPlaying ? "⏸" : "▶" }}
        </button>
        <button class="text-muted-foreground hover:text-foreground text-lg" @click="store.next()">⏭</button>
      </div>

      <!-- Seek bar -->
      <div class="flex-1 flex items-center gap-2">
        <span class="text-xs text-muted-foreground w-8 text-right">{{
          formatTime(store.position)
        }}</span>
        <input
          type="range"
          min="0"
          :max="Math.max(store.duration, 1)"
          step="0.1"
          :value="store.position"
          class="flex-1 accent-primary h-1"
          @input="onSeekChange"
        />
        <span class="text-xs text-muted-foreground w-8">{{
          formatTime(store.duration)
        }}</span>
      </div>

      <!-- Volume -->
      <div class="flex items-center gap-2 w-32 flex-shrink-0">
        <button class="text-muted-foreground hover:text-foreground text-sm" @click="store.toggleMute()">
          {{ store.volume > 0 ? "🔊" : "🔇" }}
        </button>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          :value="store.volume"
          class="flex-1 accent-primary h-1"
          @input="onVolumeChange"
        />
      </div>
    </footer>
  </div>
</template>

<style scoped>
input[type="range"] {
  -webkit-appearance: none;
  appearance: none;
  background: rgba(255, 255, 255, 0.15);
  border-radius: 2px;
  outline: none;
  cursor: pointer;
}
input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #fff;
  border: none;
  cursor: pointer;
}
input[type="range"]:focus {
  outline: none;
}
</style>
