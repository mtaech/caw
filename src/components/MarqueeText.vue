<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";

/**
 * MarqueeText: auto-scrolls long text horizontally when it overflows its box.
 * Scroll is driven by requestAnimationFrame (not CSS animation) so it can never
 * "restart / freeze" when re-measured. Only animates when the text overflows;
 * otherwise renders a static single line. Shows a native tooltip when truncated.
 */
const props = withDefaults(
  defineProps<{
    text: string;
    /** horizontal gap between looping copies, px */
    gap?: number;
    /** scroll speed, px per second */
    speed?: number;
    /** pause the scroll while hovering */
    pauseOnHover?: boolean;
  }>(),
  { gap: 48, speed: 30, pauseOnHover: true },
);

const wrapRef = ref<HTMLElement | null>(null);
const copyRef = ref<HTMLElement | null>(null);
const trackRef = ref<HTMLElement | null>(null);

const textWidth = ref(0);
const containerWidth = ref(0);
const isOverflowing = computed(() => textWidth.value > containerWidth.value + 1);
const loopDistance = computed(() => textWidth.value + props.gap);

const tooltip = computed(() => (isOverflowing.value ? props.text : ""));

const hovered = ref(false);
const reducedMotion = ref(false);

function onEnter() {
  hovered.value = true;
}
function onLeave() {
  hovered.value = false;
}

// ── Measure widths ──
function measure() {
  if (copyRef.value) textWidth.value = copyRef.value.offsetWidth;
  if (wrapRef.value) containerWidth.value = wrapRef.value.clientWidth;
}

// ── rAF scroll driver (mutates transform directly; never restarts) ──
let rafId = 0;
let lastMs = 0;
let offset = 0;

function tick(ms: number) {
  rafId = requestAnimationFrame(tick);
  const track = trackRef.value;
  if (!track) return;

  if (!isOverflowing.value || props.speed <= 0 || reducedMotion.value) return;
  if (props.pauseOnHover && hovered.value) return;

  if (!lastMs) lastMs = ms;
  const dt = ms - lastMs;
  lastMs = ms;

  const dist = loopDistance.value;
  if (dist <= 0) return;
  offset = (offset + (dt / 1000) * props.speed) % dist;
  track.style.transform = "translateX(" + -offset + "px)";
}

let ro: ResizeObserver | null = null;

onMounted(() => {
  measure();
  reducedMotion.value = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  ro = new ResizeObserver(() => nextTick(measure));
  if (wrapRef.value) ro.observe(wrapRef.value);
  if (copyRef.value) ro.observe(copyRef.value);
  rafId = requestAnimationFrame(tick);
});

onUnmounted(() => {
  ro?.disconnect();
  cancelAnimationFrame(rafId);
});

watch(() => props.text, () => {
  offset = 0;
  if (trackRef.value) trackRef.value.style.transform = "";
  nextTick(measure);
});

watch(isOverflowing, () => {
  // Reset the scroll position whenever overflow state flips,
  // so it never resumes from a stale offset.
  offset = 0;
  lastMs = 0;
  if (trackRef.value) trackRef.value.style.transform = "";
});
</script>

<template>
  <div
    ref="wrapRef"
    class="overflow-hidden whitespace-nowrap w-full min-w-0"
    :title="tooltip"
    :aria-label="text"
    @mouseenter="onEnter"
    @mouseleave="onLeave"
  >
    <div
      ref="trackRef"
      class="inline-flex items-center whitespace-nowrap will-change-transform"
    >
      <span ref="copyRef" :style="{ marginRight: gap + 'px' }">{{ text }}</span>
      <span aria-hidden="true" v-if="isOverflowing">{{ text }}</span>
    </div>
  </div>
</template>
