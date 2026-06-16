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
    <div class="flex-1 flex flex-col items-center gap-1 min-w-0 max-w-2xl mx-auto">
      <!-- Transport buttons -->
      <div class="flex items-center gap-2">
        <Button
          variant="ghost"
          size="icon-sm"
          :class="{ 'text-primary': playback.shuffle || playback.repeat !== 'none' }"
          :title="modeTitle"
          @click="cyclePlayMode"
        >
          <Shuffle v-if="playback.shuffle" class="w-4 h-4" />
          <Repeat1 v-else-if="playback.repeat === 'one'" class="w-4 h-4" />
          <Repeat v-else-if="playback.repeat === 'all'" class="w-4 h-4" />
          <ListOrdered v-else class="w-4 h-4" />
        </Button>

        <Button variant="ghost" size="icon-sm" @click="playback.prev()" title="上一首">
          <SkipBack class="w-4 h-4" />
        </Button>

        <button
          class="w-8 h-8 rounded-full bg-foreground text-background flex items-center justify-center hover:scale-105 transition-transform active:scale-95"
          @click="playback.togglePlay()"
          title="播放/暂停"
        >
          <Play v-if="!playback.isPlaying" class="w-4 h-4 ml-0.5 fill-background" />
          <Pause v-else class="w-4 h-4 fill-background" />
        </button>

        <Button variant="ghost" size="icon-sm" @click="playback.next()" title="下一首">
          <SkipForward class="w-4 h-4" />
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

    <!-- Right: volume -->
    <div class="flex items-center gap-2 w-36 flex-shrink-0 justify-end">
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
import { computed } from "vue";
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

const modeTitle = computed(() => {
  if (playback.shuffle) return "随机播放";
  if (playback.repeat === "one") return "单曲循环";
  if (playback.repeat === "all") return "列表循环";
  return "顺序播放";
});

function cyclePlayMode() {
  if (playback.shuffle) {
    // 随机播放 → 顺序播放
    playback.setShuffle(false);
  } else if (playback.repeat === "one") {
    // 单曲循环 → 列表循环
    playback.setRepeat("all");
  } else if (playback.repeat === "all") {
    // 列表循环 → 顺序播放
    playback.setRepeat("none");
  } else {
    // 顺序播放 → 随机播放
    playback.setShuffle(true);
    playback.setRepeat("all");
  }
}
</script>
