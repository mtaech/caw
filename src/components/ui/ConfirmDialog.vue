<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    description?: string;
    confirmText?: string;
    cancelText?: string;
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
        class="fixed inset-0 z-modal flex items-center justify-center p-4 select-none"
      >
        <div class="absolute inset-0 bg-overlay backdrop-blur-sm" @click="close" />
        <Transition name="pop" appear>
          <div
            v-if="props.open"
            class="relative bg-elevated rounded-2xl border border-border/80 shadow-3 p-6 w-full max-w-sm mx-4 backdrop-blur-md"
          >
            <h2 class="text-title text-foreground font-bold mb-2">{{ props.title }}</h2>
            <p v-if="props.description" class="text-body-sm text-muted-foreground leading-relaxed mb-6">
              {{ props.description }}
            </p>
            <div v-else class="mb-6" />
            <div class="flex justify-end gap-2.5">
              <button
                class="px-4 py-2 rounded-xl text-sm font-medium text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors"
                @click="close"
              >
                {{ props.cancelText }}
              </button>
              <button
                class="px-4 py-2 rounded-xl text-sm font-semibold transition-all shadow-sm active:scale-95"
                :class="
                  props.destructive
                    ? 'bg-destructive text-white hover:bg-destructive-hover shadow-destructive/20'
                    : 'bg-primary text-primary-foreground hover:bg-primary-hover shadow-primary/20'
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
