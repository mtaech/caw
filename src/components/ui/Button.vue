<template>
  <button
    :class="
      cn(
        'inline-flex items-center justify-center gap-2 whitespace-nowrap text-sm font-medium transition-all duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 disabled:pointer-events-none disabled:opacity-40 select-none active:scale-[0.97]',
        variant === 'default' &&
          'bg-primary text-primary-foreground shadow-sm hover:bg-primary-hover hover:shadow',
        variant === 'secondary' &&
          'bg-elevated text-foreground border border-border/80 shadow-sm hover:bg-elevated-hover hover:border-border',
        variant === 'ghost' &&
          'text-muted-foreground hover:text-foreground hover:bg-elevated-hover',
        variant === 'outline' &&
          'border border-border text-foreground hover:bg-elevated-hover hover:border-foreground/20',
        variant === 'icon' &&
          'text-muted-foreground hover:text-foreground hover:bg-elevated-hover',
        size === 'xs' && 'h-7 px-2 text-xs rounded-md',
        size === 'sm' && 'h-8 px-2.5 text-xs rounded-md',
        size === 'md' && 'h-9 px-3.5 text-sm rounded-lg',
        size === 'lg' && 'h-10 px-5 text-sm font-semibold rounded-lg',
        size === 'icon-sm' && 'h-8 w-8 rounded-lg',
        size === 'icon-md' && 'h-9 w-9 rounded-lg',
        size === 'icon-lg' && 'h-11 w-11 rounded-full',
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
  variant?: "default" | "secondary" | "ghost" | "icon" | "outline";
  size?: "xs" | "sm" | "md" | "lg" | "icon-sm" | "icon-md" | "icon-lg";
  circle?: boolean;
}

withDefaults(defineProps<ButtonProps>(), {
  variant: "default",
  size: "md",
  circle: false,
});
</script>
