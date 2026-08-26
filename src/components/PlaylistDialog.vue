<template>
  <Teleport to="body">
    <Transition name="overlay">
      <div v-if="open" class="fixed inset-0 z-modal flex items-center justify-center p-4 select-none">
        <!-- Overlay -->
        <div class="absolute inset-0 bg-overlay backdrop-blur-sm" @click="close" />
        <!-- Dialog panel -->
        <Transition name="pop" appear>
          <div v-if="open" class="relative bg-elevated rounded-2xl border border-border/80 shadow-3 p-6 w-full max-w-sm mx-4 backdrop-blur-md">
            <h2 class="text-title text-foreground font-bold mb-4">{{ title }}</h2>
            <input
              ref="inputRef"
              v-model="text"
              type="text"
              class="w-full rounded-xl border border-border bg-background px-3.5 py-2.5 text-sm text-foreground placeholder:text-muted-foreground/70 focus:outline-none focus:ring-2 focus:ring-primary/20 focus:border-primary mb-6 transition-all"
              :placeholder="placeholder"
              @keydown.enter="confirm"
              @keydown.escape="close"
            />
            <div class="flex justify-end gap-2.5">
              <button
                class="px-4 py-2 rounded-xl text-sm font-medium text-muted-foreground hover:text-foreground hover:bg-elevated-hover transition-colors"
                @click="close"
              >
                取消
              </button>
              <button
                class="px-4 py-2 rounded-xl text-sm font-semibold bg-primary text-primary-foreground hover:bg-primary-hover transition-all shadow-sm active:scale-95 disabled:opacity-40"
                :disabled="!text.trim()"
                @click="confirm"
              >
                {{ confirmText }}
              </button>
            </div>
          </div>
        </Transition>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from "vue";

export interface DialogProps {
  open: boolean;
  title: string;
  placeholder?: string;
  confirmText?: string;
  initialText?: string;
}

const props = withDefaults(defineProps<DialogProps>(), {
  placeholder: "",
  confirmText: "确定",
  initialText: "",
});

const emit = defineEmits<{
  (e: "close"): void;
  (e: "confirm", text: string): void;
}>();

const text = ref(props.initialText);
const inputRef = ref<HTMLInputElement | null>(null);

watch(
  () => props.open,
  (val) => {
    if (val) {
      text.value = props.initialText;
      nextTick(() => inputRef.value?.focus());
    }
  },
);

function close() {
  emit("close");
}

function confirm() {
  if (text.value.trim()) {
    emit("confirm", text.value.trim());
  }
}
</script>
