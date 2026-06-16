<template>
  <input
    type="text"
    :value="modelValue"
    :class="
      cn(
        'flex h-9 w-full rounded-md border border-border bg-elevated px-3 py-1 text-sm text-foreground shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50',
        $attrs.class as string,
      )
    "
    v-bind="$attrs"
    @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  />
</template>

<script setup lang="ts">
import { cn } from "@/lib/utils";

defineProps<{
  modelValue?: string;
}>();

defineEmits<{
  (e: 'update:modelValue', value: string): void;
}>();
</script>
