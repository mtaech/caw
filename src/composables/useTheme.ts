import { ref } from "vue";

export type ThemeMode = "system" | "light" | "dark";

const STORAGE_KEY = "caw_theme_mode";

export const currentTheme = ref<ThemeMode>("system");

function applyTheme(mode: ThemeMode) {
  const root = document.documentElement;
  if (mode === "dark") {
    root.classList.add("dark");
  } else if (mode === "light") {
    root.classList.remove("dark");
  } else {
    // system
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    if (prefersDark) {
      root.classList.add("dark");
    } else {
      root.classList.remove("dark");
    }
  }
}

export function useTheme() {
  function setTheme(mode: ThemeMode) {
    currentTheme.value = mode;
    localStorage.setItem(STORAGE_KEY, mode);
    applyTheme(mode);
  }

  function initTheme() {
    const saved = localStorage.getItem(STORAGE_KEY) as ThemeMode | null;
    if (saved && ["system", "light", "dark"].includes(saved)) {
      currentTheme.value = saved;
    } else {
      currentTheme.value = "system";
    }
    applyTheme(currentTheme.value);

    // Listen to system theme changes
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    mediaQuery.addEventListener("change", () => {
      if (currentTheme.value === "system") {
        applyTheme("system");
      }
    });
  }

  return {
    theme: currentTheme,
    setTheme,
    initTheme,
  };
}
