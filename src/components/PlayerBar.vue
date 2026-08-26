<template>
  <footer
    class="h-20 flex-shrink-0 bg-elevated border-t border-border flex items-center justify-between px-5 gap-4 z-20 select-none relative"
  >
    <!-- Left: Now Playing Metadata (Generous width, no useless locate tooltip) -->
    <div class="flex items-center gap-3.5 w-80 md:w-96 flex-shrink-0 min-w-0">
      <div class="relative flex-shrink-0">
        <CoverArt
          v-if="playback.currentTrackId"
          :track-id="playback.currentTrackId"
          :size="52"
          class="rounded-lg shadow-sm"
        />
        <div
          v-else
          class="w-[52px] h-[52px] rounded-lg bg-elevated-hover/80 border border-border flex items-center justify-center flex-shrink-0"
        >
          <Music class="w-5 h-5 text-muted-foreground/40" />
        </div>
      </div>
      
      <div class="min-w-0 flex-1 flex flex-col justify-center gap-0.5 pr-2">
        <MarqueeText
          class="text-sm font-semibold text-foreground"
          :text="playback.currentTrack?.title ?? '未在播放'"
          :speed="30"
          :pause-on-hover="true"
        />
        <p
          class="text-xs text-muted-foreground truncate"
          :title="
            playback.currentTrack
              ? playback.currentTrack.artist + (playback.currentTrack.album ? ' · ' + playback.currentTrack.album : '')
              : 'Caw 音乐播放器'
          "
        >
          <span v-if="playback.currentTrack">
            {{ playback.currentTrack.artist }}{{ playback.currentTrack.album ? ' · ' + playback.currentTrack.album : '' }}
          </span>
          <span v-else>Caw 音乐播放器</span>
        </p>
      </div>
    </div>

    <!-- Center: Transport & Progress Bar -->
    <div class="flex-1 flex flex-col items-center gap-1.5 min-w-0 max-w-xl mx-auto relative px-2">
      <!-- Mode Toast Popup -->
      <Transition name="toast-fade">
        <div
          v-if="modeToast"
          class="absolute -top-9 left-1/2 -translate-x-1/2 px-3 py-1 rounded-full bg-foreground text-background text-caption whitespace-nowrap shadow-2 pointer-events-none z-30 font-medium"
        >
          {{ modeToast }}
        </div>
      </Transition>

      <!-- Transport Buttons Row -->
      <div class="flex items-center gap-3">
        <!-- Shuffle Toggle Button -->
        <button
          class="w-8 h-8 rounded-full flex items-center justify-center transition-all duration-150"
          :class="
            playback.shuffle
              ? 'text-primary bg-primary/10 hover:bg-primary/20'
              : 'text-muted-foreground hover:text-foreground hover:bg-elevated-hover'
          "
          :title="playback.shuffle ? '随机播放：开启' : '随机播放：关闭'"
          @click="toggleShuffle"
        >
          <Shuffle class="w-4 h-4" />
        </button>

        <!-- Previous Track Button -->
        <button
          class="w-8 h-8 rounded-full flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-all duration-150 active:scale-95"
          title="上一首 (Left Arrow)"
          @click="playback.prev()"
        >
          <SkipBack class="w-4 h-4" />
        </button>

        <!-- Main Play/Pause Button -->
        <button
          class="w-10 h-10 rounded-full bg-primary text-primary-foreground flex items-center justify-center shadow-md hover:scale-105 active:scale-95 hover:bg-primary-hover transition-all duration-150"
          title="播放 / 暂停 (Space)"
          @click="playback.togglePlay()"
        >
          <Play v-if="!playback.isPlaying" class="w-4 h-4 ml-0.5 fill-primary-foreground" />
          <Pause v-else class="w-4 h-4 fill-primary-foreground" />
        </button>

        <!-- Next Track Button -->
        <button
          class="w-8 h-8 rounded-full flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-all duration-150 active:scale-95"
          title="下一首 (Right Arrow)"
          @click="playback.next()"
        >
          <SkipForward class="w-4 h-4" />
        </button>

        <!-- Repeat Mode Button -->
        <button
          class="w-8 h-8 rounded-full flex items-center justify-center transition-all duration-150 relative"
          :class="
            playback.repeat !== 'none'
              ? 'text-primary bg-primary/10 hover:bg-primary/20'
              : 'text-muted-foreground hover:text-foreground hover:bg-elevated-hover'
          "
          :title="repeatTitle"
          @click="cycleRepeat"
        >
          <Repeat1 v-if="playback.repeat === 'one'" class="w-4 h-4" />
          <Repeat v-else class="w-4 h-4" />
        </button>
      </div>

      <!-- Progress Bar Row -->
      <div class="w-full flex items-center gap-2.5">
        <span class="text-caption text-muted-foreground w-10 text-right tabular-nums flex-shrink-0 font-medium font-mono">
          {{ formatDuration(playback.position) }}
        </span>
        <Slider
          class="flex-1"
          :model-value="playback.position"
          :min="0"
          :max="Math.max(playback.duration, 1)"
          :step="0.25"
          @input="onSeek"
        />
        <span class="text-caption text-muted-foreground w-10 tabular-nums flex-shrink-0 font-medium font-mono">
          {{ formatDuration(playback.duration) }}
        </span>
      </div>
    </div>

    <!-- Right: Queue & Volume Controls -->
    <div class="flex items-center gap-3 w-72 md:w-80 flex-shrink-0 justify-end">
      <!-- Queue Panel Toggle Button -->
      <button
        class="relative h-8 px-2.5 rounded-lg flex items-center gap-1.5 text-xs font-medium transition-all duration-150 border border-border"
        :class="
          showQueue
            ? 'bg-primary/15 text-primary border-primary/30'
            : 'text-muted-foreground hover:text-foreground hover:bg-elevated-hover'
        "
        :title="showQueue ? '隐藏播放队列' : '显示播放队列'"
        @click="toggleQueue?.()"
      >
        <ListOrdered class="w-4 h-4" />
        <span v-if="playback.queue.length > 0" class="text-caption tabular-nums">
          {{ playback.queue.length }}
        </span>
      </button>

      <!-- Volume Mute Toggle -->
      <button
        class="w-8 h-8 rounded-lg flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors"
        :title="playback.volume === 0 ? '取消静音' : '静音'"
        @click="playback.toggleMute()"
      >
        <VolumeX v-if="playback.volume === 0" class="w-4 h-4 text-destructive" />
        <Volume1 v-else-if="playback.volume < 0.5" class="w-4 h-4" />
        <Volume2 v-else class="w-4 h-4" />
      </button>

      <!-- Volume Slider -->
      <div class="w-24 flex items-center">
        <Slider
          :model-value="playback.volume"
          :min="0"
          :max="1"
          :step="0.01"
          @input="onVolume"
        />
      </div>
    </div>
  </footer>
</template>

<script setup lang="ts">
import { computed, inject, type Ref, ref } from "vue";
import { usePlaybackStore } from "@/stores/playback";
import Slider from "@/components/ui/Slider.vue";
import CoverArt from "@/components/CoverArt.vue";
import MarqueeText from "@/components/MarqueeText.vue";
import { formatDuration } from "@/lib/utils";
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
  Music,
} from "lucide-vue-next";

const playback = usePlaybackStore();
const toggleQueue = inject<() => void>("toggleQueue", () => {});
const showQueue = inject<Ref<boolean>>("showQueue", ref(false));

const modeToast = ref<string | null>(null);
let modeToastTimer: ReturnType<typeof setTimeout> | null = null;

function showToast(label: string) {
  modeToast.value = label;
  if (modeToastTimer) clearTimeout(modeToastTimer);
  modeToastTimer = setTimeout(() => {
    modeToast.value = null;
  }, 1400);
}

function onSeek(val: number) {
  playback.seekTo(val);
}

function onVolume(val: number) {
  playback.setVolume(val);
}

function toggleShuffle() {
  const next = !playback.shuffle;
  playback.setShuffle(next);
  showToast(next ? "随机播放：开启" : "随机播放：关闭");
}

const repeatTitle = computed(() => {
  if (playback.repeat === "one") return "单曲循环";
  if (playback.repeat === "all") return "列表循环";
  return "顺序播放（不循环）";
});

function cycleRepeat() {
  if (playback.repeat === "none") {
    playback.setRepeat("all");
    showToast("列表循环");
  } else if (playback.repeat === "all") {
    playback.setRepeat("one");
    showToast("单曲循环");
  } else {
    playback.setRepeat("none");
    showToast("顺序播放");
  }
}
</script>

<style scoped>
.toast-fade-enter-active,
.toast-fade-leave-active {
  transition: opacity 0.2s cubic-bezier(0.16, 1, 0.3, 1), transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
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
