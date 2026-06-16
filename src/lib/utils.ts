import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Deterministic gradient for cover-less album art and artist avatars.
 * Same seed always yields the same hue pair, so placeholders stay stable
 * across renders (no flicker). Tuned for legibility on a light background.
 */
export function coverGradient(seed: string): string {
  let hash = 0;
  for (let i = 0; i < seed.length; i++) {
    hash = (hash << 5) - hash + seed.charCodeAt(i);
    hash |= 0;
  }
  const hue = Math.abs(hash) % 360;
  const hue2 = (hue + 38) % 360;
  return `linear-gradient(135deg, hsl(${hue} 52% 62%), hsl(${hue2} 48% 46%))`;
}
