<template>
  <div :class="cn('relative flex items-center w-full h-6 group cursor-pointer select-none', $attrs.class as string)">
    <!-- Track background -->
    <div class="absolute left-0 right-0 h-1.5 rounded-full bg-border/80 dark:bg-border group-hover:h-2 transition-all duration-150 overflow-hidden" />
    
    <!-- Filled track with vibrant gradient -->
    <div
      class="absolute left-0 h-1.5 rounded-full bg-primary group-hover:h-2 transition-all duration-150"
      :style="{ width: `${progressPercent}%` }"
    />

    <!-- Range input -->
    <input
      type="range"
      :min="min"
      :max="max"
      :step="step"
      :value="modelValue"
      class="absolute inset-0 w-full h-full opacity-0 cursor-pointer z-10"
      @input="onInput"
    />

    <!-- Modern Thumb Indicator -->
    <div
      v-if="showThumb"
      class="absolute w-3.5 h-3.5 rounded-full bg-foreground shadow-md ring-2 ring-background opacity-0 scale-75 group-hover:opacity-100 group-hover:scale-100 transition-all duration-150 pointer-events-none"
      :style="{ left: `calc(${progressPercent}% - 7px)` }"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { cn } from "@/lib/utils";

export interface SliderProps {
  modelValue: number;
  min?: number;
  max?: number;
  step?: number;
  showThumb?: boolean;
}

const props = withDefaults(defineProps<SliderProps>(), {
  min: 0,
  max: 1,
  step: 0.01,
  showThumb: true,
});

const progressPercent = computed(() => {
  const range = (props.max || 1) - props.min;
  if (range <= 0) return 0;
  const pct = ((props.modelValue - props.min) / range) * 100;
  return Math.max(0, Math.min(100, pct));
});

const emit = defineEmits<{
  (e: "update:modelValue", val: number): void;
  (e: "input", val: number): void;
}>();

function onInput(event: Event) {
  const target = event.target as HTMLInputElement;
  const val = parseFloat(target.value);
  emit("update:modelValue", val);
  emit("input", val);
}
</script>
