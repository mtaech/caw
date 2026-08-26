<template>
  <TooltipProvider :delay-duration="delay">
    <TooltipRoot>
      <TooltipTrigger as-child>
        <slot name="trigger" />
      </TooltipTrigger>
      <TooltipPortal>
        <TooltipContent
          :class="
            cn(
              'z-dropdown rounded-md bg-foreground px-2.5 py-1.5 text-xs text-background shadow-2 animate-in fade-in-0 zoom-in-95 select-none',
            )
          "
          :side="side"
          :side-offset="sideOffset"
        >
          <slot />
          <TooltipArrow class="fill-foreground" />
        </TooltipContent>
      </TooltipPortal>
    </TooltipRoot>
  </TooltipProvider>
</template>

<script setup lang="ts">
import { cn } from "@/lib/utils";
import {
  TooltipProvider,
  TooltipRoot,
  TooltipTrigger,
  TooltipPortal,
  TooltipContent,
  TooltipArrow,
} from "radix-vue";

export interface TooltipProps {
  delay?: number;
  sideOffset?: number;
  side?: "top" | "right" | "bottom" | "left";
}

withDefaults(defineProps<TooltipProps>(), {
  delay: 200,
  sideOffset: 6,
  side: "top",
});
</script>
