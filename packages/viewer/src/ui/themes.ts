/**
 * Theme presets for `<pptx-viewer>` and the `<pptx-presentation>`
 * shell.
 *
 * A theme is a record of CSS custom-property overrides covering both
 * the inner viewer (`--pptx-viewer-*`) and the surrounding chrome
 * (`--pptx-shell-*` — ribbon, sidebar, status bar, drawers, dialogs).
 * Consumers apply a theme by setting the variables on the host
 * element (or any ancestor — they're inheritable):
 *
 * ```ts
 * import { applyTheme, THEMES } from "@slideglance/viewer";
 * applyTheme(viewerEl, THEMES.light);
 * ```
 *
 * The light/dark choice can also follow the OS / browser preference
 * automatically — see {@link detectSystemTheme} and
 * {@link subscribeSystemTheme}.
 */

export type ThemeVars = Record<string, string>;

/**
 * Default dark theme — matches the built-in viewer defaults.
 * Includes both viewer-internal HUD variables and the shell chrome
 * (ribbon, sidebar, status bar, search drawer, settings dialog).
 */
export const dark: ThemeVars = {
  // --- inner viewer (slide stage) ---
  "--pptx-viewer-bg": "#1f1f1f",
  "--pptx-viewer-fg": "#f5f5f5",
  "--pptx-viewer-shadow": "rgba(0, 0, 0, 0.45)",
  "--pptx-viewer-hud-bg": "rgba(0, 0, 0, 0.5)",
  "--pptx-viewer-hud-fg": "#fff",
  "--pptx-viewer-overlay": "rgba(31, 31, 31, 0.85)",
  "--pptx-viewer-error": "#ff8a80",
  // --- shell chrome ---
  "--pptx-shell-bg": "#2b2b2f",
  "--pptx-shell-fg": "#ececec",
  "--pptx-shell-ribbon-bg": "#1f1f23",
  "--pptx-shell-status-bg": "#1f1f23",
  "--pptx-shell-sidebar-bg": "#15151a",
  "--pptx-shell-notes-bg": "#1a1a1f",
  "--pptx-shell-notes-fg": "#ddd",
  "--pptx-shell-notes-heading": "#888",
  "--pptx-shell-status": "#888",
  "--pptx-shell-border": "#2a2a30",
  "--pptx-shell-hover": "#2a2a30",
  "--pptx-shell-active": "#3a3a44",
  "--pptx-shell-accent": "#6aa3ff",
  "--pptx-shell-drawer-bg": "#1f1f23",
  "--pptx-shell-dialog-bg": "#1f1f23",
  "--pptx-shell-dialog-fg": "#ececec",
  "--pptx-shell-dialog-overlay": "rgba(0, 0, 0, 0.55)",
  "--pptx-shell-input-bg": "#15151a",
  "--pptx-shell-input-fg": "#ececec",
  // --- sidebar add-ons (section-nav, thumbnail-strip/panel) ---
  "--pptx-section-bg": "#1a1a1f",
  "--pptx-section-fg": "#ccc",
  "--pptx-section-tile": "#25252b",
  "--pptx-section-tile-hover": "#2f2f36",
  "--pptx-section-active": "#6aa3ff",
  "--pptx-section-active-fg": "#fff",
  "--pptx-thumb-bg": "#15151a",
  "--pptx-thumb-fg": "#ddd",
  "--pptx-thumb-tile": "#1f1f24",
  "--pptx-thumb-active": "#6aa3ff",
  // --- accent-tinted background for selected/active items ---
  "--pptx-shell-accent-soft": "#1d2738",
  // --- kbd / code chip ---
  "--pptx-shell-kbd-bg": "rgba(255, 255, 255, 0.06)",
  // --- shadow color used for floating panels / cards ---
  "--pptx-shell-shadow": "rgba(0, 0, 0, 0.45)",
  // --- info / loading banner ---
  "--pptx-shell-info-bg": "#1f2a3a",
  "--pptx-shell-info-fg": "#cfe1ff",
  "--pptx-shell-info-border": "#3a5a8a",
  // --- error banner ---
  "--pptx-shell-error-bg": "#3a1f1f",
  "--pptx-shell-error-fg": "#ffd1d1",
  "--pptx-shell-error-border": "#ff5566",
  // --- scrollbars / UA color scheme ---
  // `color-scheme` lets the browser pick UA scrollbar / form-control
  // colors that match the theme (dark scrollbars on dark surfaces).
  // `scrollbar-color` gives Firefox finer control. WebKit reads the
  // explicit `--pptx-shell-scrollbar-*` variables from each
  // component's `::-webkit-scrollbar*` rules.
  // Track matches sidebar bg so the gutter blends in; thumb is a
  // subtle border-tone gray that matches PowerPoint / VSCode dark
  // chrome (avoid the over-bright defaults that read as white on
  // OLED panels).
  "--slideglance-color-scheme": "dark",
  "--pptx-shell-scrollbar-track": "#15151a",
  "--pptx-shell-scrollbar-thumb": "#2a2a30",
  "--pptx-shell-scrollbar-thumb-hover": "#3a3a44",
};

/** Light theme — for embedding in white-page docs. */
export const light: ThemeVars = {
  // --- inner viewer ---
  "--pptx-viewer-bg": "#fafafa",
  "--pptx-viewer-fg": "#1a1a1a",
  "--pptx-viewer-shadow": "rgba(0, 0, 0, 0.18)",
  "--pptx-viewer-hud-bg": "rgba(255, 255, 255, 0.85)",
  "--pptx-viewer-hud-fg": "#111",
  "--pptx-viewer-overlay": "rgba(250, 250, 250, 0.85)",
  "--pptx-viewer-error": "#c62828",
  // --- shell chrome ---
  "--pptx-shell-bg": "#f3f3f5",
  "--pptx-shell-fg": "#1a1a1f",
  "--pptx-shell-ribbon-bg": "#ffffff",
  "--pptx-shell-status-bg": "#ffffff",
  "--pptx-shell-sidebar-bg": "#fafafa",
  "--pptx-shell-notes-bg": "#fafafa",
  "--pptx-shell-notes-fg": "#222",
  "--pptx-shell-notes-heading": "#666",
  "--pptx-shell-status": "#555",
  "--pptx-shell-border": "#dcdce0",
  "--pptx-shell-hover": "#eceef2",
  "--pptx-shell-active": "#dde7f7",
  "--pptx-shell-accent": "#1f6feb",
  "--pptx-shell-drawer-bg": "#ffffff",
  "--pptx-shell-dialog-bg": "#ffffff",
  "--pptx-shell-dialog-fg": "#1a1a1f",
  "--pptx-shell-dialog-overlay": "rgba(40, 40, 50, 0.45)",
  "--pptx-shell-input-bg": "#ffffff",
  "--pptx-shell-input-fg": "#1a1a1f",
  // --- sidebar add-ons ---
  "--pptx-section-bg": "#fafafa",
  "--pptx-section-fg": "#1a1a1f",
  "--pptx-section-tile": "#eef0f3",
  "--pptx-section-tile-hover": "#dde4ee",
  "--pptx-section-active": "#1f6feb",
  "--pptx-section-active-fg": "#ffffff",
  "--pptx-thumb-bg": "#fafafa",
  "--pptx-thumb-fg": "#1a1a1f",
  "--pptx-thumb-tile": "#ffffff",
  "--pptx-thumb-active": "#1f6feb",
  // --- accent-tinted background for selected/active items ---
  "--pptx-shell-accent-soft": "#dde7f7",
  // --- kbd / code chip ---
  "--pptx-shell-kbd-bg": "rgba(0, 0, 0, 0.04)",
  // --- shadow color used for floating panels / cards ---
  "--pptx-shell-shadow": "rgba(0, 0, 0, 0.18)",
  // --- info / loading banner ---
  "--pptx-shell-info-bg": "#eaf3ff",
  "--pptx-shell-info-fg": "#1a3a6a",
  "--pptx-shell-info-border": "#7ea8d6",
  // --- error banner ---
  "--pptx-shell-error-bg": "#ffecec",
  "--pptx-shell-error-fg": "#8a1f1f",
  "--pptx-shell-error-border": "#d6555a",
  // --- scrollbars / UA color scheme ---
  "--slideglance-color-scheme": "light",
  "--pptx-shell-scrollbar-track": "#eeeef1",
  "--pptx-shell-scrollbar-thumb": "#c2c5cc",
  "--pptx-shell-scrollbar-thumb-hover": "#a6abb5",
};

/** Accessible high-contrast theme. */
export const highContrast: ThemeVars = {
  // --- inner viewer ---
  "--pptx-viewer-bg": "#000",
  "--pptx-viewer-fg": "#fff",
  "--pptx-viewer-shadow": "transparent",
  "--pptx-viewer-hud-bg": "#fff",
  "--pptx-viewer-hud-fg": "#000",
  "--pptx-viewer-overlay": "rgba(0, 0, 0, 0.92)",
  "--pptx-viewer-error": "#ffeb3b",
  // --- shell chrome ---
  "--pptx-shell-bg": "#000000",
  "--pptx-shell-fg": "#ffffff",
  "--pptx-shell-ribbon-bg": "#000000",
  "--pptx-shell-status-bg": "#000000",
  "--pptx-shell-sidebar-bg": "#000000",
  "--pptx-shell-notes-bg": "#000000",
  "--pptx-shell-notes-fg": "#ffffff",
  "--pptx-shell-notes-heading": "#ffeb3b",
  "--pptx-shell-status": "#ffffff",
  "--pptx-shell-border": "#ffffff",
  "--pptx-shell-hover": "#222222",
  "--pptx-shell-active": "#ffeb3b",
  "--pptx-shell-accent": "#ffeb3b",
  "--pptx-shell-drawer-bg": "#000000",
  "--pptx-shell-dialog-bg": "#000000",
  "--pptx-shell-dialog-fg": "#ffffff",
  "--pptx-shell-dialog-overlay": "rgba(0, 0, 0, 0.8)",
  "--pptx-shell-input-bg": "#000000",
  "--pptx-shell-input-fg": "#ffffff",
  // --- sidebar add-ons ---
  "--pptx-section-bg": "#000000",
  "--pptx-section-fg": "#ffffff",
  "--pptx-section-tile": "#000000",
  "--pptx-section-tile-hover": "#222222",
  "--pptx-section-active": "#ffeb3b",
  "--pptx-section-active-fg": "#000000",
  "--pptx-thumb-bg": "#000000",
  "--pptx-thumb-fg": "#ffffff",
  "--pptx-thumb-tile": "#000000",
  "--pptx-thumb-active": "#ffeb3b",
  // --- accent-tinted background for selected/active items ---
  "--pptx-shell-accent-soft": "#332e00",
  // --- kbd / code chip ---
  "--pptx-shell-kbd-bg": "#000000",
  // --- shadow color used for floating panels / cards ---
  "--pptx-shell-shadow": "transparent",
  // --- info / loading banner ---
  "--pptx-shell-info-bg": "#000000",
  "--pptx-shell-info-fg": "#ffeb3b",
  "--pptx-shell-info-border": "#ffeb3b",
  // --- error banner ---
  "--pptx-shell-error-bg": "#000000",
  "--pptx-shell-error-fg": "#ffeb3b",
  "--pptx-shell-error-border": "#ff8888",
  // --- scrollbars / UA color scheme ---
  "--slideglance-color-scheme": "dark",
  "--pptx-shell-scrollbar-track": "#000000",
  "--pptx-shell-scrollbar-thumb": "#ffeb3b",
  "--pptx-shell-scrollbar-thumb-hover": "#fff176",
};

/** Built-in theme registry. */
export const THEMES = { dark, light, highContrast } as const;

/** Apply a theme to an element by setting CSS custom properties. */
export function applyTheme(el: HTMLElement, vars: ThemeVars): void {
  for (const [k, v] of Object.entries(vars)) {
    el.style.setProperty(k, v);
  }
}

/**
 * Inspect the OS / browser color-scheme preference. Returns `"dark"`
 * when `prefers-color-scheme: dark` matches, else `"light"`. SSR /
 * non-window environments (worker, jsdom without matchMedia) get
 * `"light"` as a stable default.
 */
export function detectSystemTheme(): "light" | "dark" {
  if (
    typeof window === "undefined" ||
    typeof window.matchMedia !== "function"
  ) {
    return "light";
  }
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

/**
 * Subscribe to OS / browser color-scheme changes. The callback fires
 * with `"light"` or `"dark"` whenever `prefers-color-scheme` flips.
 * Returns a teardown function that removes the listener.
 */
export function subscribeSystemTheme(
  cb: (mode: "light" | "dark") => void,
): () => void {
  if (
    typeof window === "undefined" ||
    typeof window.matchMedia !== "function"
  ) {
    return () => {};
  }
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = (ev: MediaQueryListEvent): void =>
    cb(ev.matches ? "dark" : "light");
  // `addEventListener` on MediaQueryList is the modern API; older
  // Safari (< 14) only had `addListener`. Both are supported via
  // feature detection so we don't break in legacy browser embeds.
  if (typeof mq.addEventListener === "function") {
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }
  // Legacy fallback path.
  const legacy = mq as MediaQueryList & {
    addListener: (cb: (ev: MediaQueryListEvent) => void) => void;
    removeListener: (cb: (ev: MediaQueryListEvent) => void) => void;
  };
  legacy.addListener(handler);
  return () => legacy.removeListener(handler);
}
