<template>
  <footer
    class="h-20 flex-shrink-0 bg-elevated border-t border-border flex items-center px-4 gap-4 z-10"
  >
    <!-- Left: now playing info -->
    <div class="flex items-center gap-3 w-56 flex-shrink-0 min-w-0">
      <CoverArt
        v-if="playback.currentTrackId"
        :track-id="playback.currentTrackId"
        :size="56"
      />
      <div v-else class="w-14 h-14 rounded-lg bg-elevated-hover flex-shrink-0" />
      <div class="min-w-0 flex flex-col">
        <p class="text-body-md text-foreground truncate">
          {{ playback.currentTrack?.title ?? "—" }}
        </p>
        <p class="text-body-sm text-muted-foreground truncate">
          {{ playback.currentTrack?.artist ?? "" }}
        </p>
      </div>
    </div>

    <!-- Center: transport + progress -->
    <div class="flex-1 flex flex-col items-center gap-1 min-w-0 max-w-2xl mx-auto relative">
      <!-- Mode toast -->
      <Transition name="toast-fade">
        <div
          v-if="modeToast"
          class="absolute -top-8 left-1/2 -translate-x-1/2 px-3 py-0.5 rounded-full bg-foreground text-background text-caption whitespace-nowrap pointer-events-none z-20"
        >
          {{ modeToast }}
        </div>
      </Transition>

      <!-- Transport buttons -->
      <div class="flex items-center gap-2">
        <Button variant="ghost" size="icon-sm" @click="playback.prev()" title="上一首">
          <SkipBack class="w-4 h-4" />
        </Button>

        <button
          class="w-9 h-9 rounded-full bg-primary text-background flex items-center justify-center hover:scale-105 transition-transform active:scale-95 shadow-1"
          @click="playback.togglePlay()"
          title="播放/暂停"
        >
          <Play v-if="!playback.isPlaying" class="w-4 h-4 ml-0.5 fill-background" />
          <Pause v-else class="w-4 h-4 fill-background" />
        </button>

        <Button variant="ghost" size="icon-sm" @click="playback.next()" title="下一首">
          <SkipForward class="w-4 h-4" />
        </Button>

        <!-- Play mode: 顺序→随机→列表循环→单曲循环 -->
        <Button
          variant="ghost"
          size="icon-sm"
          :class="{ 'text-primary': playback.shuffle || playback.repeat !== 'none' }"
          :title="playModeLabel"
          @click="cyclePlayMode"
        >
          <Shuffle v-if="playback.shuffle" class="w-4 h-4" />
          <Repeat1 v-else-if="playback.repeat === 'one'" class="w-4 h-4" />
          <Repeat v-else class="w-4 h-4" />
        </Button>
      </div>

      <!-- Progress bar -->
      <div class="w-full flex items-center gap-2">
        <span class="text-caption text-muted-foreground w-8 text-right tabular-nums flex-shrink-0">
          {{ fmt(playback.position) }}
        </span>
        <Slider
          class="flex-1"
          :model-value="playback.position"
          :min="0"
          :max="Math.max(playback.duration, 1)"
          :step="0.5"
          @input="onSeek"
        />
        <span class="text-caption text-muted-foreground w-8 tabular-nums flex-shrink-0">
          {{ fmt(playback.duration) }}
        </span>
      </div>
    </div>

    <!-- Right: queue toggle + volume -->
    <div class="flex items-center gap-2 w-48 flex-shrink-0 justify-end">
      <Button
        variant="ghost"
        size="icon-sm"
        @click="toggleQueue?.()"
        :title="showQueue ? '隐藏队列' : '显示队列'"
      >
        <ListOrdered class="w-4 h-4" :class="{ 'text-primary': showQueue }" />
      </Button>
      <Button variant="ghost" size="icon-sm" @click="playback.toggleMute()">
        <VolumeX v-if="playback.volume === 0" class="w-4 h-4" />
        <Volume1 v-else-if="playback.volume < 0.5" class="w-4 h-4" />
        <Volume2 v-else class="w-4 h-4" />
      </Button>
      <Slider
        class="w-24"
        :model-value="playback.volume"
        :min="0"
        :max="1"
        :step="0.01"
        @input="onVolume"
      />
    </div>
  </footer>
</template>

<script setup lang="ts">
import { computed, inject, type Ref, ref } from "vue";
import { usePlaybackStore } from "@/stores/playback";
import Button from "@/components/ui/Button.vue";
import Slider from "@/components/ui/Slider.vue";
import CoverArt from "@/components/CoverArt.vue";
import {
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Shuffle,
  Repeat,
  Repeat1,
  ListOrdered,
  Volume1,
  Volume2,
  VolumeX,
} from "lucide-vue-next";

const playback = usePlaybackStore();
const toggleQueue = inject<() => void>("toggleQueue", () => {});
const showQueue = inject<Ref<boolean>>("showQueue", ref(false));

function fmt(sec: number): string {
  if (!sec || sec < 0) return "0:00";
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function onSeek(val: number) {
  playback.seekTo(val);
}

function onVolume(val: number) {
  playback.setVolume(val);
}

const playModeLabel = computed(() => {
  if (playback.shuffle) return '随机播放';
  if (playback.repeat === 'one') return '单曲循环';
  if (playback.repeat === 'all') return '列表循环';
  return '顺序播放';
});

const modeToast = ref<string | null>(null);
let modeToastTimer: ReturnType<typeof setTimeout> | null = null;

function showModeToast(label: string) {
  modeToast.value = label;
  if (modeToastTimer) clearTimeout(modeToastTimer);
  modeToastTimer = setTimeout(() => { modeToast.value = null; }, 1500);
}

function cyclePlayMode() {
  // 顺序 → 随机 → 列表循环 → 单曲循环
  if (!playback.shuffle && playback.repeat === 'none') {
    playback.setShuffle(true);
  } else if (playback.shuffle) {
    playback.setShuffle(false);
    playback.setRepeat('all');
  } else if (playback.repeat === 'all') {
    playback.setRepeat('one');
  } else {
    playback.setRepeat('none');
  }
  showModeToast(playModeLabel.value);
}
</script>

<style>
.toast-fade-enter-active,
.toast-fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.toast-fade-enter-from {
  opacity: 0;
  transform: translate(-50%, 6px);
}
.toast-fade-leave-to {
  opacity: 0;
  transform: translate(-50%, -6px);
}
</style>
