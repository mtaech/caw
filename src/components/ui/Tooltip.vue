<template>
  <TooltipRoot :delay-duration="delay">
    <TooltipTrigger as-child>
      <slot name="trigger" />
    </TooltipTrigger>
    <TooltipPortal>
      <TooltipContent
        :class="
          cn(
            'z-dropdown rounded-md bg-foreground px-2.5 py-1.5 text-xs text-background shadow-2 animate-in fade-in-0 zoom-in-95',
            sideOffset ?? 4,
          )
        "
        :side-offset="sideOffset ?? 4"
      >
        <slot />
        <TooltipArrow class="fill-foreground" />
      </TooltipContent>
    </TooltipPortal>
  </TooltipRoot>
</template>

<script setup lang="ts">
import { cn } from "@/lib/utils";
import {
  TooltipRoot,
  TooltipTrigger,
  TooltipPortal,
  TooltipContent,
  TooltipArrow,
} from "radix-vue";

export interface TooltipProps {
  delay?: number;
  sideOffset?: number;
}

withDefaults(defineProps<TooltipProps>(), {
  delay: 400,
  sideOffset: 4,
});
</script>
