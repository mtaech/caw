<template>
  <div
    :class="
      cn(
        'relative flex-shrink-0 overflow-hidden select-none bg-elevated transition-transform duration-200 ring-1 ring-black/5 dark:ring-white/10',
        round ? 'rounded-full' : 'rounded-lg shadow-sm',
      )
    "
    :style="{ width: size + 'px', height: size + 'px' }"
  >
    <!-- Actual Cover Image with cross-fade -->
    <Transition name="fade">
      <img
        v-if="coverUrl"
        :key="coverUrl"
        :src="coverUrl"
        class="w-full h-full object-cover select-none pointer-events-none"
        alt="cover"
        @error="onError"
      />
    </Transition>

    <!-- Loading skeleton -->
    <div
      v-if="loading && !coverUrl"
      class="w-full h-full bg-elevated-hover animate-pulse flex items-center justify-center"
    >
      <Disc3 class="text-muted-foreground/40 animate-spin" :size="size * 0.35" />
    </div>

    <!-- Gradient placeholder (deterministic per track) -->
    <div
      v-else-if="!coverUrl"
      class="w-full h-full flex items-center justify-center relative overflow-hidden"
      :style="{ background: gradient }"
    >
      <!-- Subtle internal gloss / lighting overlay -->
      <div class="absolute inset-0 bg-gradient-to-tr from-black/20 via-transparent to-white/20 pointer-events-none" />
      <Music
        class="text-white/90 drop-shadow relative z-10"
        :size="size * 0.38"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { Music, Disc3 } from "lucide-vue-next";
import { cn, coverGradient } from "@/lib/utils";
import { usePlaybackStore } from "@/stores/playback";

const props = withDefaults(
  defineProps<{
    trackId: number;
    size?: number;
    round?: boolean;
  }>(),
  { size: 56, round: false },
);

const store = usePlaybackStore();
const coverUrl = ref<string | null>(null);
const errored = ref(false);
const loading = ref(false);

const gradient = computed(() => coverGradient(String(props.trackId)));

async function loadCover() {
  if (!props.trackId || errored.value) {
    coverUrl.value = null;
    loading.value = false;
    return;
  }
  loading.value = true;
  try {
    const url = await store.getCoverUrl(props.trackId);
    coverUrl.value = url;
  } catch {
    coverUrl.value = null;
  } finally {
    loading.value = false;
  }
}

function onError() {
  errored.value = true;
  coverUrl.value = null;
}

watch(
  () => props.trackId,
  () => {
    errored.value = false;
    loadCover();
  },
  { immediate: true },
);

watch(
  () => store.library.length,
  () => {
    errored.value = false;
    loadCover();
  },
);
</script>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.25s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
