<script setup lang="ts">
import { ref, watch } from "vue";
import { Disc3, Minimize2, Power, Check } from "lucide-vue-next";
import Button from "@/components/ui/Button.vue";
import * as api from "@/lib/tauri";

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "confirm", action: "tray" | "quit"): void;
}>();

const selectedAction = ref<"tray" | "quit">("tray");
const remember = ref(false);

// When dialog opens, initialize from current setting
watch(
  () => props.open,
  async (isOpen) => {
    if (isOpen) {
      try {
        const toTray = await api.getMinimizeToTray();
        selectedAction.value = toTray ? "tray" : "quit";
        remember.value = false;
      } catch {}
    }
  },
  { immediate: true }
);

function handleCancel() {
  emit("close");
}

async function handleConfirm() {
  const isTray = selectedAction.value === "tray";
  try {
    await api.setMinimizeToTray(isTray);
    if (remember.value) {
      await api.setClosePrompt(false);
    }
  } catch (e) {
    console.error("Failed to save close preferences:", e);
  }

  emit("confirm", selectedAction.value);

  if (isTray) {
    await api.hideWindow();
  } else {
    await api.exitApp();
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="overlay">
      <div
        v-if="props.open"
        class="fixed inset-0 z-modal flex items-center justify-center p-4 select-none"
      >
        <div class="absolute inset-0 bg-overlay backdrop-blur-sm" @click="handleCancel" />
        <Transition name="pop" appear>
          <div
            v-if="props.open"
            class="relative bg-elevated rounded-2xl border border-border/80 shadow-3 p-6 w-full max-w-md mx-4 backdrop-blur-md space-y-5"
          >
            <!-- Header -->
            <div class="flex items-start gap-3.5">
              <div class="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center text-primary flex-shrink-0 shadow-sm border border-primary/20">
                <Disc3 class="w-5 h-5 animate-spin-slow" />
              </div>
              <div>
                <h2 class="text-title text-foreground font-bold tracking-tight">关闭 Caw</h2>
                <p class="text-body-sm text-muted-foreground mt-0.5">
                  请选择点击关闭按钮时的操作：
                </p>
              </div>
            </div>

            <!-- Selection Options -->
            <div class="space-y-2.5">
              <!-- Option 1: Minimize to tray -->
              <button
                type="button"
                class="w-full flex items-center gap-3.5 p-3.5 rounded-xl border text-left transition-all duration-150 relative overflow-hidden"
                :class="
                  selectedAction === 'tray'
                    ? 'border-primary bg-primary/10 text-foreground ring-1 ring-primary shadow-sm'
                    : 'border-border/70 hover:border-border bg-background/50 hover:bg-background text-muted-foreground hover:text-foreground'
                "
                @click="selectedAction = 'tray'"
              >
                <div
                  class="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 transition-colors"
                  :class="selectedAction === 'tray' ? 'bg-primary text-primary-foreground shadow-sm' : 'bg-elevated border border-border text-muted-foreground'"
                >
                  <Minimize2 class="w-4 h-4" />
                </div>
                <div class="flex-1 min-w-0">
                  <div class="text-sm font-semibold" :class="selectedAction === 'tray' ? 'text-foreground' : 'text-foreground/90'">
                    最小化到系统托盘
                  </div>
                  <div class="text-xs text-muted-foreground mt-0.5 truncate">
                    程序在后台静默运行，音乐不中断
                  </div>
                </div>
                <div
                  class="w-4 h-4 rounded-full border flex items-center justify-center transition-colors flex-shrink-0"
                  :class="selectedAction === 'tray' ? 'border-primary bg-primary text-primary-foreground' : 'border-muted-foreground/40'"
                >
                  <div v-if="selectedAction === 'tray'" class="w-1.5 h-1.5 rounded-full bg-primary-foreground" />
                </div>
              </button>

              <!-- Option 2: Exit app -->
              <button
                type="button"
                class="w-full flex items-center gap-3.5 p-3.5 rounded-xl border text-left transition-all duration-150 relative overflow-hidden"
                :class="
                  selectedAction === 'quit'
                    ? 'border-primary bg-primary/10 text-foreground ring-1 ring-primary shadow-sm'
                    : 'border-border/70 hover:border-border bg-background/50 hover:bg-background text-muted-foreground hover:text-foreground'
                "
                @click="selectedAction = 'quit'"
              >
                <div
                  class="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 transition-colors"
                  :class="selectedAction === 'quit' ? 'bg-destructive text-white shadow-sm' : 'bg-elevated border border-border text-muted-foreground'"
                >
                  <Power class="w-4 h-4" />
                </div>
                <div class="flex-1 min-w-0">
                  <div class="text-sm font-semibold" :class="selectedAction === 'quit' ? 'text-foreground' : 'text-foreground/90'">
                    直接退出 Caw
                  </div>
                  <div class="text-xs text-muted-foreground mt-0.5 truncate">
                    停止音频播放并完全退出应用程序
                  </div>
                </div>
                <div
                  class="w-4 h-4 rounded-full border flex items-center justify-center transition-colors flex-shrink-0"
                  :class="selectedAction === 'quit' ? 'border-primary bg-primary text-primary-foreground' : 'border-muted-foreground/40'"
                >
                  <div v-if="selectedAction === 'quit'" class="w-1.5 h-1.5 rounded-full bg-primary-foreground" />
                </div>
              </button>
            </div>

            <!-- Remember choice Checkbox -->
            <label class="flex items-center gap-2.5 cursor-pointer py-1 group select-none">
              <div
                class="w-4 h-4 rounded border transition-all flex items-center justify-center flex-shrink-0"
                :class="
                  remember
                    ? 'bg-primary border-primary text-primary-foreground shadow-sm'
                    : 'border-border/90 bg-background group-hover:border-primary/60'
                "
                @click="remember = !remember"
              >
                <Check v-if="remember" class="w-3 h-3 stroke-[3]" />
              </div>
              <span class="text-xs text-muted-foreground group-hover:text-foreground transition-colors" @click="remember = !remember">
                记住我的选择，下次不再提示（可在偏好设置中修改）
              </span>
            </label>

            <!-- Buttons -->
            <div class="flex justify-end gap-2.5 pt-2 border-t border-border/60">
              <Button variant="secondary" size="md" @click="handleCancel">
                取消
              </Button>
              <Button variant="default" size="md" @click="handleConfirm">
                确定
              </Button>
            </div>
          </div>
        </Transition>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.animate-spin-slow {
  animation: spin 12s linear infinite;
}
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
