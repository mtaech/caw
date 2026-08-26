import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Deterministic modern gradient for cover-less album art, artist avatars and playlists.
 * Handpicked harmonious gradient pairs with smooth lighting for both Dark and Light themes.
 */
const GRADIENT_PAIRS = [
  ["#6366f1", "#a855f7"], // Indigo -> Purple
  ["#3b82f6", "#2dd4bf"], // Blue -> Teal
  ["#ec4899", "#f43f5e"], // Pink -> Rose
  ["#10b981", "#059669"], // Emerald -> Forest
  ["#f59e0b", "#ea580c"], // Amber -> Orange
  ["#8b5cf6", "#d946ef"], // Violet -> Fuchsia
  ["#06b6d4", "#3b82f6"], // Cyan -> Blue
  ["#14b8a6", "#10b981"], // Teal -> Emerald
  ["#f43f5e", "#fb7185"], // Rose -> Light Rose
  ["#64748b", "#475569"], // Slate -> Dark Slate
  ["#84cc16", "#10b981"], // Lime -> Emerald
  ["#a855f7", "#ec4899"], // Purple -> Pink
];

export function coverGradient(seed: string): string {
  if (!seed) return "linear-gradient(135deg, #4f46e5 0%, #7c3aed 100%)";
  let hash = 0;
  for (let i = 0; i < seed.length; i++) {
    hash = (hash << 5) - hash + seed.charCodeAt(i);
    hash |= 0;
  }
  const index = Math.abs(hash) % GRADIENT_PAIRS.length;
  const [c1, c2] = GRADIENT_PAIRS[index];
  return `linear-gradient(135deg, ${c1} 0%, ${c2} 100%)`;
}

/**
 * Format seconds to M:SS or H:MM:SS
 */
export function formatDuration(sec: number): string {
  if (!sec || isNaN(sec) || sec < 0) return "0:00";
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = Math.floor(sec % 60);
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }
  return `${m}:${s.toString().padStart(2, "0")}`;
}
