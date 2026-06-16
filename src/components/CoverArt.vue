<template>
  <div
    :class="
      cn(
        'flex-shrink-0 overflow-hidden',
        round ? 'rounded-full' : 'rounded-lg',
      )
    "
    :style="{ width: size + 'px', height: size + 'px' }"
  >
    <img
      v-if="coverUrl"
      :src="coverUrl"
      class="w-full h-full object-cover"
      alt="cover"
      @error="onError"
    />
    <!-- Loading skeleton -->
    <div
      v-else-if="loading"
      class="w-full h-full bg-elevated-hover animate-pulse"
    />
    <!-- Gradient placeholder (deterministic per track) -->
    <div
      v-else
      class="w-full h-full flex items-center justify-center"
      :style="{ background: gradient }"
    >
      <Music
        :class="round ? 'text-white/85' : 'text-white/90'"
        :size="size * 0.35"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { Music } from "lucide-vue-next";
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

// Stable gradient keyed on the track id so the placeholder never flickers.
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
