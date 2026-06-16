<script setup lang="ts">
/**
 * Generic confirmation dialog with enter/leave animation.
 * Replaces the ad-hoc inline confirm markup previously inlined in views.
 */
const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    description?: string;
    confirmText?: string;
    cancelText?: string;
    /** Destructive variant tints the confirm button red. */
    destructive?: boolean;
  }>(),
  {
    description: "",
    confirmText: "确定",
    cancelText: "取消",
    destructive: false,
  },
);

const emit = defineEmits<{
  (e: "close"): void;
  (e: "confirm"): void;
}>();

function close() {
  emit("close");
}

function confirm() {
  emit("confirm");
}
</script>

<template>
  <Teleport to="body">
    <Transition name="overlay">
      <div
        v-if="props.open"
        class="fixed inset-0 z-modal flex items-center justify-center"
      >
        <div class="absolute inset-0 bg-overlay" @click="close" />
        <Transition name="pop" appear>
          <div
            v-if="props.open"
            class="relative bg-elevated rounded-xl border border-border shadow-3 p-6 w-full max-w-sm mx-4"
          >
            <h2 class="text-title text-foreground mb-2">{{ props.title }}</h2>
            <p v-if="props.description" class="text-body text-muted-foreground mb-5">
              {{ props.description }}
            </p>
            <div v-else class="mb-5" />
            <div class="flex justify-end gap-2">
              <button
                class="px-3 py-1.5 rounded-md text-sm text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors"
                @click="close"
              >
                {{ props.cancelText }}
              </button>
              <button
                class="px-3 py-1.5 rounded-md text-sm text-white transition-colors"
                :class="
                  props.destructive
                    ? 'bg-destructive hover:bg-destructive-hover'
                    : 'bg-primary hover:bg-primary-hover'
                "
                @click="confirm"
              >
                {{ props.confirmText }}
              </button>
            </div>
          </div>
        </Transition>
      </div>
    </Transition>
  </Teleport>
</template>
