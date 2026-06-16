/**
 * Font preference management.
 * Font lists are fetched from the Rust backend (linux: fontconfig / fc-list)
 * via a Tauri command, then cached.  First call triggers the backend query;
 * subsequent calls return the cached result synchronously.
 */

export interface FontOption {
  id: string;
  /** Human-readable display name */
  label: string;
  /** CSS font-family value. Empty string = use default fallback chain. */
  fontFamily: string;
}

const STORAGE_KEY = "caw-font-preference";

// ── Async loading from backend ──────────────────────────────────────

let _cached: FontOption[] | null = null;
let _loading: Promise<void> | null = null;

function slug(name: string): string {
  return name.replace(/\s+/g, "-").toLowerCase();
}

/** Fetch installed fonts from the Tauri backend and cache them. */
export async function loadSystemFonts(): Promise<void> {
  if (_cached) return;
  if (_loading) return _loading;

  _loading = (async () => {
    try {
      const { getSystemFonts } = await import("@/lib/tauri");
      const families = await getSystemFonts();
      _cached = families.map((f) => ({
        id: slug(f),
        label: f,
        fontFamily: `'${f}'`,
      }));
    } catch (e) {
      console.error("caw: failed to load system fonts", e);
      _cached = [];
    }
  })();

  return _loading;
}

/** Synchronously return already-loaded font options (empty until loaded). */
export function getAvailableFonts(): FontOption[] {
  return _cached ?? [];
}

// ── Persistence ─────────────────────────────────────────────────────

export function getFontPreference(): string {
  return localStorage.getItem(STORAGE_KEY) ?? "default";
}

export function setFontPreference(id: string): void {
  localStorage.setItem(STORAGE_KEY, id);
}

/** Apply a font preference (by id) to the document root CSS variable.
 *  Fonts must be loaded first — call `await loadSystemFonts()` before this. */
export function applyFont(id: string): void {
  if (id === "default") {
    document.documentElement.style.removeProperty("--font-sans");
    return;
  }
  const opt = getAvailableFonts().find((f) => f.id === id);
  if (opt) {
    document.documentElement.style.setProperty("--font-sans", opt.fontFamily);
  }
}
