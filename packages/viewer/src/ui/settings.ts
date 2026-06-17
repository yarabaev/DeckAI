/**
 * Persistent viewer settings.
 *
 * The settings object is stored in `localStorage` under
 * {@link STORAGE_KEY}. Every property has a built-in default so a
 * brand-new install or a wiped browser still produces a valid,
 * fully-functional viewer state. Future settings should be added to
 * the {@link ViewerSettings} interface and {@link DEFAULT_SETTINGS}
 * registry — the load() / save() / subscribe() machinery picks them
 * up automatically.
 *
 * Why a hand-rolled subscriber list instead of using a Lit reactive
 * controller? The settings store is shared across multiple
 * components (the presentation shell, the settings dialog, theme
 * helpers) and must work outside any Lit element too — e.g. when
 * the host page wants to read the persisted theme synchronously
 * before mounting the viewer.
 */

import { setLocale, type Locale } from "./i18n.js";

/** localStorage key under which the settings JSON is persisted. */
export const STORAGE_KEY = "slideglance-viewer-settings:v1";

/**
 * Theme-mode preference. `auto` follows the OS / browser
 * `prefers-color-scheme` setting and reacts to runtime changes.
 */
export type ThemeMode = "auto" | "dark" | "light" | "high-contrast";

/**
 * Ruler measurement unit — `cm` mirrors PowerPoint's default with the
 * slide centred on `0` and counts outwards; `px` shows the on-screen
 * pixel coordinate space starting at the slide's left / top edge.
 */
export type RulerUnit = "cm" | "px";

/**
 * The persisted viewer settings shape. Add new fields here when a
 * new dialog control needs to remember its value across sessions.
 */
export interface ViewerSettings {
  /** Active theme. Defaults to `auto` so a fresh install adapts to the OS. */
  themeMode: ThemeMode;
  /** Whether the PowerPoint-style ruler overlay is visible. */
  showRuler: boolean;
  /** Unit used by the ruler ticks / labels. */
  rulerUnit: RulerUnit;
  /**
   * UI language. `"auto"` resolves via the host's locale (browser
   * `navigator.languages` / Node `LC_*` env). Any explicit value
   * forces that language regardless of the host setting.
   */
  locale: Locale;
  /**
   * Width of the thumbnail sidebar in CSS pixels. Persisted across
   * sessions so the user's resize is remembered. Clamped at runtime
   * to {@link SIDEBAR_WIDTH_MIN}..{@link SIDEBAR_WIDTH_MAX}.
   */
  sidebarWidth: number;
}

/** Lower bound on the sidebar width — anything narrower hides the
 * thumbnail labels and makes the slide-number column unreadable. */
export const SIDEBAR_WIDTH_MIN = 140;
/** Upper bound on the sidebar width — past this point the stage area
 * starts to feel cramped on a typical 1024px viewport. */
export const SIDEBAR_WIDTH_MAX = 480;
/** Default sidebar width — matches the previous hard-coded value. */
export const SIDEBAR_WIDTH_DEFAULT = 220;

/** Clamp `width` to the supported sidebar range. */
export function clampSidebarWidth(width: number): number {
  if (!Number.isFinite(width)) return SIDEBAR_WIDTH_DEFAULT;
  return Math.max(
    SIDEBAR_WIDTH_MIN,
    Math.min(SIDEBAR_WIDTH_MAX, Math.round(width)),
  );
}

/** Defaults applied when no value is found in storage. */
export const DEFAULT_SETTINGS: Readonly<ViewerSettings> = Object.freeze({
  themeMode: "auto" as ThemeMode,
  // Ruler is on by default — first-time viewers benefit from the
  // visual reference for slide measurements without having to find
  // it in the settings dialog. Existing users keep whatever value
  // they previously persisted (settings merge: stored over default).
  showRuler: true,
  // Pixel ticks match what the user sees on screen; this aligns the
  // ruler's reading with browser zoom / window dimensions instead of
  // PowerPoint's centimetres-from-centre coordinate space.
  rulerUnit: "px" as RulerUnit,
  locale: "auto" as Locale,
  sidebarWidth: SIDEBAR_WIDTH_DEFAULT,
});

type Listener = (settings: ViewerSettings) => void;
const listeners = new Set<Listener>();
let cached: ViewerSettings | null = null;

/**
 * Read the persisted settings. Reads from `localStorage` once, then
 * memoizes — subsequent calls are O(1) until a `save()` invalidates
 * the cache.
 */
export function loadSettings(): ViewerSettings {
  if (cached) return cached;
  cached = readFromStorage();
  // Push the persisted locale into the i18n module so the viewer's
  // first paint already reflects the user's stored choice (or the
  // host-detected default when the value is `"auto"`).
  setLocale(cached.locale);
  return cached;
}

/**
 * Persist a partial update. Triggers every subscriber registered via
 * {@link subscribeSettings}. Storage failures (private mode, quota,
 * disabled localStorage) are swallowed — the in-memory copy still
 * updates so the current session reflects the change even if the
 * persistence layer is unavailable.
 */
export function saveSettings(patch: Partial<ViewerSettings>): ViewerSettings {
  const current = loadSettings();
  const next: ViewerSettings = { ...current, ...patch };
  cached = next;
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
    }
  } catch {
    /* persistence failed — keep the in-memory copy. */
  }
  if (patch.locale !== undefined) setLocale(next.locale);
  for (const listener of listeners) {
    try {
      listener(next);
    } catch {
      /* one bad listener should not break the others. */
    }
  }
  return next;
}

/**
 * Subscribe to settings changes. Returns a teardown function that
 * removes the listener.
 */
export function subscribeSettings(cb: Listener): () => void {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

/** Reset the in-memory cache — primarily for tests. */
export function resetSettingsCache(): void {
  cached = null;
}

function readFromStorage(): ViewerSettings {
  if (typeof localStorage === "undefined") return { ...DEFAULT_SETTINGS };
  let raw: string | null = null;
  try {
    raw = localStorage.getItem(STORAGE_KEY);
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
  if (!raw) return { ...DEFAULT_SETTINGS };
  try {
    const parsed = JSON.parse(raw) as Partial<ViewerSettings>;
    const merged = { ...DEFAULT_SETTINGS, ...parsed };
    // Re-clamp persisted numeric ranges — older payloads may predate
    // a tightened bound (e.g. SIDEBAR_WIDTH_MIN raised in a future
    // release) and would otherwise survive the merge unchecked.
    merged.sidebarWidth = clampSidebarWidth(merged.sidebarWidth);
    return merged;
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}
