<template>
  <div
    :class="
      cn(
        'flex-shrink-0 overflow-hidden',
        round ? 'rounded-full' : 'rounded-lg',
        'bg-elevated-hover',
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
    <div v-else class="w-full h-full flex items-center justify-center">
      <Music class="text-muted-foreground" :size="size * 0.35" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { Music } from "lucide-vue-next";
import { cn } from "@/lib/utils";
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

async function loadCover() {
  if (!props.trackId || errored.value) {
    coverUrl.value = null;
    return;
  }
  try {
    const url = await store.getCoverUrl(props.trackId);
    coverUrl.value = url;
  } catch {
    coverUrl.value = null;
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
