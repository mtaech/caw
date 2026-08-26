<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";

/**
 * MarqueeText: auto-scrolls long text horizontally when it overflows its box.
 * Also shows a native tooltip with the full text when overflowing.
 * Renders a static single line (no animation) when the text fits.
 */
const props = withDefaults(
  defineProps<{
    text: string;
    /** horizontal gap between looping copies, px */
    gap?: number;
    /** scroll speed, px per second */
    speed?: number;
    /** seconds to wait before the scroll starts (after overflow) */
    delay?: number;
    /** pause the scroll while hovering */
    pauseOnHover?: boolean;
  }>(),
  { gap: 48, speed: 30, delay: 0.6, pauseOnHover: true },
);

const wrapRef = ref<HTMLElement | null>(null);
const copyRef = ref<HTMLElement | null>(null);

const textWidth = ref(0);
const containerWidth = ref(0);

// Width of ONE text copy vs. available container width.
const isOverflowing = computed(() => textWidth.value > containerWidth.value + 1);

// Distance to shift so the 2nd copy lands exactly where the 1st started -> seamless loop.
const loopDistance = computed(() => textWidth.value + props.gap);

const trackStyle = computed(
  () =>
    ({
      animationDuration: isOverflowing.value && props.speed > 0 ? (loopDistance.value / props.speed) + "s" : "0s",
      animationDelay: props.delay + "s",
      "--marquee-dist": "-" + loopDistance.value + "px",
    }) as Record<string, string>,
);

const tooltip = computed(() => (isOverflowing.value ? props.text : ""));

function measure() {
  if (copyRef.value) textWidth.value = copyRef.value.offsetWidth;
  if (wrapRef.value) containerWidth.value = wrapRef.value.clientWidth;
}

let ro: ResizeObserver | null = null;

onMounted(() => {
  measure();
  ro = new ResizeObserver(() => nextTick(measure));
  if (wrapRef.value) ro.observe(wrapRef.value);
  if (copyRef.value) ro.observe(copyRef.value);
});

onUnmounted(() => ro?.disconnect());

watch(() => props.text, () => nextTick(measure));
</script>

<template>
  <div ref="wrapRef" class="overflow-hidden whitespace-nowrap w-full min-w-0" :title="tooltip" :aria-label="text">
    <div
      class="inline-flex items-center whitespace-nowrap will-change-transform"
      :class="{ 'marquee-anim': isOverflowing, 'marquee-pause-hover': pauseOnHover }"
      :style="trackStyle"
    >
      <span ref="copyRef" :style="{ marginRight: gap + 'px' }">{{ text }}</span>
      <span aria-hidden="true">{{ text }}</span>
    </div>
  </div>
</template>

<style scoped>
@keyframes marquee-scroll {
  from {
    transform: translateX(0);
  }
  to {
    transform: translateX(var(--marquee-dist));
  }
}
.marquee-anim {
  animation-name: marquee-scroll;
  animation-timing-function: linear;
  animation-iteration-count: infinite;
}
.marquee-pause-hover:hover {
  animation-play-state: paused;
}
@media (prefers-reduced-motion: reduce) {
  .marquee-anim {
    animation: none;
  }
}
</style>
