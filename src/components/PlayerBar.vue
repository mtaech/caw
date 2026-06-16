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
          :class="{ 'text-primary': playback.shuffle }"
          @click="playback.setShuffle(!playback.shuffle)"
        >
          <Shuffle class="w-4 h-4" />
        </Button>

        <Button variant="ghost" size="icon-sm" @click="playback.prev()">
          <SkipBack class="w-4 h-4" />
        </Button>

        <button
          class="w-8 h-8 rounded-full bg-foreground text-background flex items-center justify-center hover:scale-105 transition-transform active:scale-95"
          @click="playback.togglePlay()"
        >
          <Play v-if="!playback.isPlaying" class="w-4 h-4 ml-0.5 fill-background" />
          <Pause v-else class="w-4 h-4 fill-background" />
        </button>

        <button
          class="text-caption text-muted-foreground hover:text-foreground transition-colors px-1.5 tabular-nums select-none whitespace-nowrap"
          :title="modeTooltip"
          @click="cyclePlayMode"
        >
          {{ currentModeLabel }}
        </button>

        <Button variant="ghost" size="icon-sm" @click="playback.next()">
          <SkipForward class="w-4 h-4" />
        </Button>

        <Button
          variant="ghost"
          size="icon-sm"
          :class="{
            'text-primary': playback.repeat !== 'none',
          }"
          @click="cycleRepeat"
        >
          <Repeat v-if="playback.repeat === 'all'" class="w-4 h-4" />
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

const repeatModes = ["none", "all", "one"] as const;
function cycleRepeat() {
  const idx = repeatModes.indexOf(playback.repeat as "none" | "all" | "one");
  const next = repeatModes[(idx + 1) % repeatModes.length];
  playback.setRepeat(next);
}

// Combined playback mode label
// 顺序播放 (repeat none, no shuffle) → 单曲循环 (repeat one) →
// 随机播放 (shuffle on) → 列表循环 (repeat all, no shuffle) →
const playModes = [
  { shuffle: false, repeat: "none", label: "顺序播放" },
  { shuffle: false, repeat: "one", label: "单曲循环" },
  { shuffle: true, repeat: "all", label: "随机播放" },
  { shuffle: false, repeat: "all", label: "列表循环" },
] as const;

const currentModeLabel = computed(() => {
  const s = playback.shuffle;
  const r = playback.repeat;
  for (const m of playModes) {
    if (m.shuffle === s && m.repeat === r) return m.label;
  }
  if (r !== "none") {
    return r === "one" ? "单曲循环" : "列表循环";
  }
  return "顺序播放";
});

const modeTooltip = computed(() => {
  return `${currentModeLabel.value} — 点击切换`;
});

function cyclePlayMode() {
  const s = playback.shuffle;
  const r = playback.repeat;
  const idx = playModes.findIndex((m) => m.shuffle === s && m.repeat === r);
  const next = playModes[(idx + 1) % playModes.length];
  if (next.shuffle !== s) playback.setShuffle(next.shuffle);
  if (next.repeat !== r) playback.setRepeat(next.repeat);
}
</script>
