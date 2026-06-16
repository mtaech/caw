<template>
  <div :class="cn('relative flex items-center w-full h-5 group cursor-pointer', $attrs.class as string)">
    <!-- Track background -->
    <div class="absolute left-0 right-0 h-1 rounded-full bg-border group-hover:h-1.5 transition-all duration-120" />
    <!-- Filled track -->
    <div
      class="absolute left-0 h-1 rounded-full bg-primary group-hover:h-1.5 transition-all duration-120"
      :style="{ width: `${(modelValue / (max || 1)) * 100}%` }"
    />
    <!-- Thumb -->
    <input
      type="range"
      :min="min"
      :max="max"
      :step="step"
      :value="modelValue"
      class="absolute inset-0 w-full h-full opacity-0 cursor-pointer z-10"
      @input="onInput"
    />
    <div
      v-if="showThumb"
      class="absolute w-3 h-3 rounded-full bg-foreground shadow-md opacity-0 group-hover:opacity-100 transition-opacity duration-120 pointer-events-none"
      :style="{ left: `calc(${(modelValue / (max || 1)) * 100}% - 6px)` }"
    />
  </div>
</template>

<script setup lang="ts">
import { cn } from "@/lib/utils";

export interface SliderProps {
  modelValue: number;
  min?: number;
  max?: number;
  step?: number;
  showThumb?: boolean;
}

withDefaults(defineProps<SliderProps>(), {
  min: 0,
  max: 1,
  step: 0.01,
  showThumb: true,
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
