<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const backendStatus = ref("checking...");

onMounted(async () => {
  try {
    const msg = await invoke<string>("greet", { name: "caw" });
    backendStatus.value = msg;
    console.log("Backend ready:", msg);

    // Verify the get_library command works
    const lib = await invoke("get_library");
    console.log("Library tracks:", lib);
  } catch (err) {
    console.error("Backend error:", err);
    backendStatus.value = `error: ${err}`;
  }
});
</script>

<template>
  <div class="h-screen w-screen flex flex-col overflow-hidden">
    <!-- Three-zone placeholder layout -->
    <div class="flex flex-1 overflow-hidden">
      <!-- Sidebar -->
      <aside
        class="w-60 flex-shrink-0 bg-sidebar border-r border-border flex flex-col"
      >
        <div class="p-4 text-faint-foreground text-xs uppercase tracking-wider">
          Navigation
        </div>
      </aside>

      <!-- Content -->
      <main class="flex-1 bg-background flex flex-col overflow-auto p-4">
        <h1 class="text-title text-foreground mb-2">Caw Music Player</h1>
        <p class="text-body-sm text-muted-foreground">
          Backend: {{ backendStatus }}
        </p>
      </main>
    </div>

    <!-- Player Bar -->
    <footer
      class="h-20 flex-shrink-0 bg-elevated border-t border-border flex items-center px-4"
    >
      <span class="text-body-sm text-muted-foreground">Player bar (P2)</span>
    </footer>
  </div>
</template>
