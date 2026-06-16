<template>
  <button
    :class="
      cn(
        'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-foreground disabled:pointer-events-none disabled:opacity-50',
        variant === 'default' &&
          'bg-primary text-background hover:bg-primary-hover',
        variant === 'ghost' && 'text-muted-foreground hover:text-foreground hover:bg-elevated-hover',
        variant === 'outline' && 'border border-border text-foreground hover:bg-elevated-hover',
        variant === 'icon' &&
          'text-muted-foreground hover:text-foreground hover:bg-elevated-hover',
        size === 'sm' && 'h-8 px-2 text-xs',
        size === 'md' && 'h-9 px-3 text-sm',
        size === 'lg' && 'h-10 px-4',
        size === 'icon-sm' && 'h-8 w-8',
        size === 'icon-md' && 'h-9 w-9',
        circle && 'rounded-full',
        'flex-shrink-0',
      )
    "
    v-bind="$attrs"
  >
    <slot />
  </button>
</template>

<script setup lang="ts">
import { cn } from "@/lib/utils";

export interface ButtonProps {
  variant?: "default" | "ghost" | "icon" | "outline";
  size?: "sm" | "md" | "lg" | "icon-sm" | "icon-md";
  circle?: boolean;
}

withDefaults(defineProps<ButtonProps>(), {
  variant: "default",
  size: "md",
  circle: false,
});
</script>
