/**
 * Style constants for the React viewer shell.
 *
 * Pulled out of `PptxPresentation.tsx` to keep that file focused on
 * the component logic. Every constant here is a plain `CSSProperties`
 * object that can be passed to a `style={...}` prop, plus one global
 * `<style>` template (`SHELL_GLOBAL_CSS`) the shell mounts once at
 * the top of its tree.
 *
 * Theme tokens (e.g. `var(--pptx-shell-bg, #2b2b2f)`) are resolved by
 * the host through CSS custom properties — see `ui/themes.ts` for the
 * default palette. Inline fallbacks are present on every var so an
 * embedder that forgets to mount the theme still gets a usable dark
 * shell.
 */

import type { CSSProperties } from "react";

/** Width of the ruler gutter that frames the slide stage. Shared with
 *  the ruler corner / horizontal / vertical strip styles. */
export const RULER_SIZE = 24;

// =========================================================================
// Global stylesheet
// =========================================================================

/**
 * One-shot global stylesheet mounted via `<style>{SHELL_GLOBAL_CSS}</style>`
 * at the top of the shell tree. Covers things inline `style=` cannot:
 *
 * - scrollbar theming for the deck container
 * - hover-fade behaviour for the slideshow corner-nav buttons
 * - the keyframes the loading-overlay spinner consumes
 * - reset of native focus / touch chrome on shell buttons
 * - sidebar splitter affordance highlighting
 */
export const SHELL_GLOBAL_CSS = `
[data-pptx-shell] {
  color-scheme: var(--slideglance-color-scheme, dark);
  scrollbar-color: var(--pptx-shell-scrollbar-thumb, #3a3a44) var(--pptx-shell-scrollbar-track, #1a1a1f);
  scrollbar-width: thin;
}
[data-pptx-shell] *::-webkit-scrollbar {
  width: 10px;
  height: 10px;
}
[data-pptx-shell] *::-webkit-scrollbar-track {
  background: var(--pptx-shell-scrollbar-track, #1a1a1f);
}
[data-pptx-shell] *::-webkit-scrollbar-thumb {
  background: var(--pptx-shell-scrollbar-thumb, #3a3a44);
  border-radius: 5px;
  border: 2px solid var(--pptx-shell-scrollbar-track, #1a1a1f);
}
[data-pptx-shell] *::-webkit-scrollbar-thumb:hover {
  background: var(--pptx-shell-scrollbar-thumb-hover, #4d4d58);
}
[data-pptx-shell] *::-webkit-scrollbar-corner {
  background: var(--pptx-shell-scrollbar-track, #1a1a1f);
}
/* Slideshow corner-nav reveal: hovering the bottom-right zone or
   focusing one of its buttons fades the button group in. Keyboard
   users get the same affordance via the focus-within branch. */
[data-pptx-shell] [data-pptx-slideshow-nav]:hover > div,
[data-pptx-shell] [data-pptx-slideshow-nav]:focus-within > div {
  opacity: 1 !important;
}
[data-pptx-shell] [data-pptx-slideshow-nav] button:hover {
  background: rgba(255, 255, 255, 0.12) !important;
}
/* Loading-overlay spinner — used by the centred parse / slide-prepare
   panel rendered when there's no slide SVG to show yet. Keyframes
   live here because inline styles cannot carry @keyframes. */
@keyframes pptx-loading-spin {
  to { transform: rotate(360deg); }
}
[data-pptx-shell] [data-pptx-slideshow-nav] button:disabled {
  opacity: 0.4;
  cursor: default;
}
/* Suppress every form of native focus / touch chrome on shell
   buttons so a click never leaves a persistent ring behind. Keyboard
   users still get a focus indicator because the affected styles
   (icon-button / status-icon / radio cell) flip their background or
   border-color when 'aria-pressed' / 'aria-checked' is true; that
   semantic-state highlight is what marks the active control, not the
   browser's default focus ring. '-webkit-tap-highlight-color:
   transparent' removes the iOS / Android touch flash so the same
   suppression works on touch devices. */
[data-pptx-shell] button,
[data-pptx-shell] [role="radio"] {
  outline: 0 !important;
  -webkit-tap-highlight-color: transparent;
}
/* Some Chromium embeddings (notably the VS Code webview iframe at
   the time of writing) keep painting the user-agent focus ring even
   after \`outline: 0 !important\` because the chrome stems from a
   private \`-internal-focus-ring\` style attached to \`:focus-visible\`,
   not the regular \`outline\` cascade. Belt-and-braces: explicitly
   suppress every commonly-emitted focus channel — outline, outline-
   offset, the legacy \`-webkit-focus-ring-color\`, and any inset
   box-shadow a downstream theme might add for accessibility. */
[data-pptx-shell] button:focus,
[data-pptx-shell] button:focus-visible,
[data-pptx-shell] [role="radio"]:focus,
[data-pptx-shell] [role="radio"]:focus-visible {
  outline: 0 !important;
  outline-offset: 0 !important;
  box-shadow: none !important;
  -webkit-focus-ring-color: transparent;
}
[data-pptx-shell] button::-moz-focus-inner,
[data-pptx-shell] [role="radio"]::-moz-focus-inner {
  border: 0;
}
/* Sidebar splitter — subtle highlight on hover/active so the
   drag affordance is discoverable without visually competing with
   the sidebar's own border at rest. */
[data-pptx-shell] [role="separator"][aria-orientation="vertical"]:hover {
  background: var(--pptx-shell-accent-soft, rgba(106, 163, 255, 0.18));
}
[data-pptx-shell] [role="separator"][aria-orientation="vertical"]:focus-visible {
  outline: 2px solid var(--pptx-shell-accent, #6aa3ff);
  outline-offset: -2px;
}
`;

// =========================================================================
// Layout — ribbon, body, sidebar, stage
// =========================================================================

export const rootStyle: CSSProperties = {
  display: "grid",
  gridTemplateRows: "auto minmax(0, 1fr) auto",
  width: "100%",
  height: "100%",
  background: "var(--pptx-shell-bg, #2b2b2f)",
  color: "var(--pptx-shell-fg, #ececec)",
  font: "13px system-ui, -apple-system, sans-serif",
  overflow: "hidden",
  position: "relative",
};

export const ribbonStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 6,
  padding: "8px 12px",
  background: "var(--pptx-shell-ribbon-bg, #1f1f23)",
  borderBottom: "1px solid var(--pptx-shell-border, #2a2a30)",
  flexWrap: "wrap",
};

export const filenameStyle: CSSProperties = {
  fontWeight: 600,
  maxWidth: 240,
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
  marginLeft: 8,
};

export const spacerStyle: CSSProperties = { flex: "1 1 auto" };

export const dividerStyle: CSSProperties = {
  width: 1,
  alignSelf: "stretch",
  background: "var(--pptx-shell-border, #2a2a30)",
  margin: "0 2px",
};

export const counterStyle: CSSProperties = {
  minWidth: 70,
  textAlign: "center",
  fontVariantNumeric: "tabular-nums",
};

export const bodyStyle: CSSProperties = {
  display: "grid",
  gridTemplateRows: "minmax(0, 1fr)",
  minHeight: 0,
  minWidth: 0,
  overflow: "hidden",
  position: "relative",
};

export const sidebarStyle: CSSProperties = {
  display: "grid",
  borderRight: "1px solid var(--pptx-shell-border, #2a2a30)",
  background: "var(--pptx-shell-sidebar-bg, #15151a)",
  overflow: "hidden",
  minHeight: 0,
};

// Splitter handle between the sidebar and the stage area. Painted as a
// transparent strip so the sidebar's own right border is the only
// visible separator at rest; on hover/active the accent colour fades
// in to advertise the drag affordance.
export const sidebarResizerStyle: CSSProperties = {
  cursor: "col-resize",
  background: "transparent",
  // A thin strip is hard to grab with the mouse; the underlying grid
  // column is `SIDEBAR_RESIZER_WIDTH` wide so this `<div>` already
  // fills it. `touch-action: none` blocks the browser's pan gesture
  // from stealing pointer events during a drag on touch devices.
  touchAction: "none",
  userSelect: "none",
};

export const stageAreaStyle: CSSProperties = {
  display: "grid",
  gridTemplateRows: "minmax(0, 1fr)",
  minHeight: 0,
  minWidth: 0,
  overflow: "hidden",
};

export const stageWrapStyle: CSSProperties = {
  position: "relative",
  minHeight: 0,
  minWidth: 0,
  overflow: "hidden",
  boxSizing: "border-box",
};

export const stageStyle: CSSProperties = {
  position: "relative",
  width: "100%",
  height: "100%",
  overflow: "auto",
  background: "var(--pptx-shell-bg, #2b2b2f)",
  display: "block",
};

// =========================================================================
// Buttons — base / icon / text variants
// =========================================================================

export const baseButtonStyle: CSSProperties = {
  background: "transparent",
  color: "inherit",
  border: "1px solid var(--pptx-shell-border, #2a2a30)",
  borderRadius: 4,
  padding: "4px 10px",
  font: "inherit",
  cursor: "pointer",
  minHeight: 28,
};

export const iconButtonStyle: CSSProperties = {
  ...baseButtonStyle,
  padding: "4px 8px",
  minWidth: 28,
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  gap: 4,
};

export const activeIconStyle: CSSProperties = {
  ...iconButtonStyle,
  background: "var(--pptx-shell-active, #3a3a44)",
  // See `activeIconSmStyle`: active toolbar buttons signal state via
  // background only; the accent border was producing a perceived
  // "white outline" on dark themes and is dropped here for parity.
};

export const textButtonStyle: CSSProperties = {
  ...baseButtonStyle,
  display: "inline-flex",
  alignItems: "center",
  gap: 6,
};

export const disabledTextButtonStyle: CSSProperties = {
  ...textButtonStyle,
  opacity: 0.45,
  cursor: "not-allowed",
};

// =========================================================================
// Thumbnails — sidebar row variant + grid tile variant
// =========================================================================

export const thumbStripStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 8,
  padding: 10,
  overflowY: "auto",
};

export const sidebarEmptyStyle: CSSProperties = {
  textAlign: "center",
  color: "var(--pptx-shell-status, #666)",
  fontSize: 12,
  padding: "24px 8px",
  fontStyle: "italic",
};

export const thumbnailButtonStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 8,
  padding: 4,
  background: "transparent",
  // Border longhand split. Using `border: "2px solid transparent"` here
  // works in isolation but a Chromium quirk in the VS Code webview
  // serializes the shorthand to `border-width / border-style /
  // border-image: initial` and drops the `border-color` longhand —
  // which then falls back to `currentColor` (the parent's
  // `--pptx-shell-fg`, ~#ececec) and renders as a visible light-grey
  // 2px stroke around every inactive thumbnail. Setting the three
  // longhands explicitly avoids the serialization round-trip.
  borderWidth: 2,
  borderStyle: "solid",
  borderColor: "transparent",
  borderRadius: 4,
  cursor: "pointer",
  color: "inherit",
  font: "inherit",
  textAlign: "left",
};

export const thumbnailButtonActiveStyle: CSSProperties = {
  ...thumbnailButtonStyle,
  borderColor: "var(--pptx-shell-accent, #6aa3ff)",
  background: "var(--pptx-shell-accent-soft, rgba(106, 163, 255, 0.12))",
};

export const thumbnailIndexStyle: CSSProperties = {
  width: 24,
  textAlign: "center",
  fontVariantNumeric: "tabular-nums",
  fontSize: 12,
  color: "var(--pptx-shell-status, #888)",
};

export const thumbnailFrameStyle: CSSProperties = {
  flex: "1 1 auto",
  background: "white",
  borderRadius: 3,
  overflow: "hidden",
  boxShadow: "0 1px 3px var(--pptx-shell-shadow, rgba(0, 0, 0, 0.4))",
};

export const thumbnailInnerStyle: CSSProperties = {
  width: "100%",
  height: "100%",
};

export const thumbnailPlaceholderStyle: CSSProperties = {
  width: "100%",
  height: "100%",
  display: "grid",
  placeItems: "center",
  color: "var(--pptx-shell-status, #aaa)",
  fontSize: 14,
  background: "var(--pptx-thumb-tile, #1a1a1f)",
};

// Frame stacks above caption; tile stretches to fill its grid cell so
// columns line up at a fixed cell width. Frame uses width: 100% +
// aspect-ratio so every tile reserves the same on-screen footprint.
export const thumbnailTileStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "stretch",
  gap: 8,
  padding: 8,
  width: "100%",
  background: "transparent",
  // Border longhand split — see the comment on `thumbnailButtonStyle`
  // above. Same Chromium-in-webview quirk: `border` shorthand drops
  // the color longhand on serialization and the resulting
  // `border-color: currentColor` paints a light-grey ring around
  // every inactive grid tile.
  borderWidth: 2,
  borderStyle: "solid",
  borderColor: "transparent",
  borderRadius: 6,
  cursor: "pointer",
  color: "inherit",
  font: "inherit",
  textAlign: "center",
  boxSizing: "border-box",
};

export const thumbnailTileActiveStyle: CSSProperties = {
  ...thumbnailTileStyle,
  borderColor: "var(--pptx-shell-accent, #6aa3ff)",
  background: "var(--pptx-shell-accent-soft, rgba(106, 163, 255, 0.12))",
};

export const thumbnailTileFrameStyle: CSSProperties = {
  width: "100%",
  background: "white",
  borderRadius: 4,
  overflow: "hidden",
  boxShadow: "0 2px 6px var(--pptx-shell-shadow, rgba(0, 0, 0, 0.45))",
  alignSelf: "stretch",
};

export const thumbnailCaptionStyle: CSSProperties = {
  fontSize: 12,
  fontVariantNumeric: "tabular-nums",
  color: "var(--pptx-shell-status, #aaa)",
  lineHeight: 1.2,
};

export const gridViewStyle: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))",
  // `gridAutoRows: min-content` forces every row track to size to its
  // content rather than stretching to fill the container's leftover
  // height; combined with `alignContent: start`, that anchors a
  // single-row deck to the top of the stage instead of letting the
  // single row expand to half-height and centring vertically.
  gridAutoRows: "min-content",
  alignContent: "start",
  alignItems: "start",
  justifyItems: "stretch",
  gap: 20,
  padding: 24,
  overflow: "auto",
  width: "100%",
  height: "100%",
  boxSizing: "border-box",
};

// =========================================================================
// Stage overlays — empty / loading / progress modals
// =========================================================================

export const overlayStyle: CSSProperties = {
  position: "absolute",
  inset: 0,
  display: "grid",
  placeItems: "center",
  color: "var(--pptx-shell-status, #888)",
  fontSize: 14,
};

// Prominent centred loading panel for the parse / slide-prepare
// window. Differs from `overlayStyle` (which is used for the
// "empty" / error states) by stacking a spinner above the label
// and using a slightly higher-contrast colour so the user can spot
// it without scanning the status bar at the bottom-left.
export const loadingOverlayStyle: CSSProperties = {
  position: "absolute",
  inset: 0,
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  justifyContent: "center",
  gap: 16,
  color: "var(--pptx-shell-fg, #ececec)",
  fontSize: 15,
  pointerEvents: "none",
};

export const loadingSpinnerStyle: CSSProperties = {
  width: 36,
  height: 36,
  borderRadius: "50%",
  border: "3px solid var(--pptx-shell-border, rgba(255, 255, 255, 0.18))",
  borderTopColor: "var(--pptx-shell-accent, #6aa3ff)",
  animation: "pptx-loading-spin 0.9s linear infinite",
};

export const loadingTextStyle: CSSProperties = {
  color: "var(--pptx-shell-fg, #ececec)",
  fontWeight: 500,
  letterSpacing: "0.01em",
};

// Centred export-progress overlay — visually mirrors the existing
// SettingsDialog modal so users perceive it as a system-level
// confirmation that the click landed.
export const progressHostStyle: CSSProperties = {
  position: "fixed",
  inset: 0,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  zIndex: 1100,
  font: "13px system-ui, -apple-system, sans-serif",
  pointerEvents: "auto",
};

export const progressBackdropStyle: CSSProperties = {
  position: "absolute",
  inset: 0,
  background: "var(--pptx-shell-dialog-overlay, rgba(0, 0, 0, 0.45))",
};

export const progressPanelStyle: CSSProperties = {
  position: "relative",
  width: "min(420px, 86vw)",
  display: "flex",
  flexDirection: "column",
  gap: 10,
  padding: "20px 24px",
  background: "var(--pptx-shell-dialog-bg, #1f1f23)",
  color: "var(--pptx-shell-dialog-fg, #ececec)",
  border: "1px solid var(--pptx-shell-border, #2a2a30)",
  borderRadius: 10,
  boxShadow: "0 16px 48px var(--pptx-shell-shadow, rgba(0, 0, 0, 0.5))",
};

export const progressTitleStyle: CSSProperties = {
  fontSize: 15,
  fontWeight: 600,
  letterSpacing: 0.1,
};

export const progressStepStyle: CSSProperties = {
  fontSize: 13,
  color: "var(--pptx-shell-status, #b8b8c0)",
  minHeight: 18,
  fontVariantNumeric: "tabular-nums",
};

export const progressBarTrackStyle: CSSProperties = {
  position: "relative",
  width: "100%",
  height: 6,
  borderRadius: 3,
  background: "var(--pptx-shell-track, rgba(255, 255, 255, 0.08))",
  overflow: "hidden",
};

export const progressBarFillStyle: CSSProperties = {
  position: "absolute",
  top: 0,
  bottom: 0,
  left: 0,
  background: "var(--pptx-shell-accent, #6aa3ff)",
  borderRadius: 3,
  transition: "width 120ms ease-out",
};

// Indeterminate fallback when the host hasn't supplied a current/total
// pair yet — paints a thin sliver so users still see the bar exists.
// Inline styles can't define keyframes, so we keep it static rather
// than animated; the live "step" text already conveys motion.
export const progressBarIndeterminateStyle: CSSProperties = {
  width: "30%",
  opacity: 0.65,
};

export const progressCounterStyle: CSSProperties = {
  fontSize: 12,
  color: "var(--pptx-shell-status, #888)",
  fontVariantNumeric: "tabular-nums",
  textAlign: "right",
};

// =========================================================================
// Ruler corner / strip
// =========================================================================

export const rulerCornerStyle: CSSProperties = {
  position: "absolute",
  top: 0,
  left: 0,
  width: RULER_SIZE,
  height: RULER_SIZE,
  background: "var(--pptx-shell-status-bg, #1f1f23)",
  borderRight: "1px solid var(--pptx-shell-border, #2a2a30)",
  borderBottom: "1px solid var(--pptx-shell-border, #2a2a30)",
  zIndex: 6,
  pointerEvents: "none",
};

export const rulerHStyle: CSSProperties = {
  position: "absolute",
  top: 0,
  right: 0,
  left: RULER_SIZE,
  height: RULER_SIZE,
  zIndex: 5,
  borderBottom: "1px solid var(--pptx-shell-border, #2a2a30)",
};

export const rulerVStyle: CSSProperties = {
  position: "absolute",
  top: RULER_SIZE,
  left: 0,
  bottom: 0,
  width: RULER_SIZE,
  zIndex: 5,
  borderRight: "1px solid var(--pptx-shell-border, #2a2a30)",
};

// =========================================================================
// Notes panel
// =========================================================================

export const notesPanelStyle: CSSProperties = {
  padding: "10px 16px",
  background: "var(--pptx-shell-notes-bg, #1a1a1f)",
  borderTop: "1px solid var(--pptx-shell-border, #2a2a30)",
  overflow: "auto",
  maxHeight: 200,
  whiteSpace: "pre-wrap",
};

export const notesHeadingStyle: CSSProperties = {
  margin: "0 0 6px",
  fontSize: 11,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--pptx-shell-notes-heading, #888)",
};

export const notesBodyStyle: CSSProperties = {
  fontSize: 12,
  color: "var(--pptx-shell-notes-fg, #ddd)",
};

export const notesEmptyStyle: CSSProperties = {
  color: "var(--pptx-shell-status, #666)",
  fontSize: 12,
};

export const notesMetaStyle: CSSProperties = {
  fontSize: 11,
  color: "var(--pptx-shell-accent, #6aa3ff)",
  marginBottom: 4,
};

// =========================================================================
// Search drawer
// =========================================================================

export const searchDrawerStyle: CSSProperties = {
  position: "absolute",
  top: 12,
  right: 12,
  width: 280,
  maxHeight: "calc(100% - 24px)",
  background: "var(--pptx-shell-drawer-bg, #1f1f23)",
  border: "1px solid var(--pptx-shell-border, #2a2a30)",
  borderRadius: 6,
  boxShadow: "0 6px 24px var(--pptx-shell-shadow, rgba(0, 0, 0, 0.4))",
  overflow: "hidden",
  display: "flex",
  flexDirection: "column",
  zIndex: 10,
};

export const searchHeaderStyle: CSSProperties = {
  padding: "8px 10px",
  borderBottom: "1px solid var(--pptx-shell-border, #2a2a30)",
  fontSize: 11,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--pptx-shell-status, #aaa)",
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
};

export const searchInputStyle: CSSProperties = {
  margin: 8,
  padding: "4px 8px",
  background: "transparent",
  color: "inherit",
  border: "1px solid var(--pptx-shell-border, #2a2a30)",
  borderRadius: 4,
  font: "inherit",
};

export const searchEmptyStyle: CSSProperties = {
  padding: 12,
  color: "var(--pptx-shell-status, #666)",
  fontStyle: "italic",
};

export const searchListStyle: CSSProperties = {
  listStyle: "none",
  margin: 0,
  padding: 0,
  overflowY: "auto",
};

export const searchItemStyle: CSSProperties = {
  padding: "8px 10px",
  cursor: "pointer",
  borderBottom: "1px solid var(--pptx-shell-border, rgba(255, 255, 255, 0.05))",
  fontSize: 12,
};

export const searchHitNumStyle: CSSProperties = {
  fontVariantNumeric: "tabular-nums",
  color: "var(--pptx-shell-accent, #6aa3ff)",
  marginRight: 6,
};

// =========================================================================
// Slideshow — fullscreen + corner-nav buttons
// =========================================================================

export const slideshowStyle: CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "#000",
  zIndex: 100,
};

// Bottom-right hover zone for slideshow nav buttons. The zone itself
// is invisible (no background) but reserves a 220×100 region for the
// hover trigger. The buttons inside fade in via the CSS rule in
// `SHELL_GLOBAL_CSS` (`[data-pptx-slideshow-nav]:hover > div`).
export const slideshowNavZoneStyle: CSSProperties = {
  position: "absolute",
  right: 0,
  bottom: 0,
  width: 220,
  height: 100,
  zIndex: 110,
  // No background so the zone is invisible until the user hovers in.
  // `pointer-events: auto` is the default; explicit so a future
  // change to the parent doesn't inherit `none` and break clicks.
  pointerEvents: "auto",
};

export const slideshowNavGroupStyle: CSSProperties = {
  position: "absolute",
  right: 16,
  bottom: 16,
  display: "flex",
  gap: 6,
  padding: 6,
  borderRadius: 8,
  background: "rgba(20, 20, 24, 0.7)",
  backdropFilter: "blur(8px)",
  // Fade controlled by CSS hover rule; keep buttons reachable for
  // keyboard focus by retaining pointer-events even while invisible.
  opacity: 0,
  transition: "opacity 120ms ease-out",
};

export const slideshowNavButtonStyle: CSSProperties = {
  width: 36,
  height: 36,
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  background: "transparent",
  color: "#ececec",
  border: "1px solid rgba(255, 255, 255, 0.18)",
  borderRadius: 6,
  cursor: "pointer",
  padding: 0,
};

// =========================================================================
// Status bar
// =========================================================================

export const statusBarStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 8,
  padding: "4px 12px",
  fontSize: 11,
  color: "var(--pptx-shell-status, #888)",
  background: "var(--pptx-shell-status-bg, #1f1f23)",
  borderTop: "1px solid var(--pptx-shell-border, #2a2a30)",
  minHeight: 28,
};

export const phaseStyle: CSSProperties = { fontVariantNumeric: "tabular-nums" };
export const metaStyle: CSSProperties = { fontSize: 11 };

export const statusSepStyle: CSSProperties = {
  width: 1,
  height: 16,
  background: "var(--pptx-shell-border, #2a2a30)",
  margin: "0 4px",
};

export const selectionFontsContainerStyle: CSSProperties = {
  position: "relative",
  display: "inline-flex",
  alignItems: "center",
};

export const selectionFontsButtonStyle: CSSProperties = {
  background: "transparent",
  color: "inherit",
  border: "1px solid transparent",
  borderRadius: 3,
  padding: "2px 6px",
  cursor: "pointer",
  font: "inherit",
  fontSize: 11,
  minHeight: 22,
  display: "inline-flex",
  alignItems: "center",
};

export const selectionFontsButtonActiveStyle: CSSProperties = {
  ...selectionFontsButtonStyle,
  background: "var(--pptx-shell-active, #3a3a44)",
};

export const selectionFontsPopoverStyle: CSSProperties = {
  position: "absolute",
  // Status bar sits at the bottom of the shell, so the popover
  // expands upward (mirrors `FontUsageIndicator`).
  bottom: "calc(100% + 4px)",
  left: 0,
  minWidth: 220,
  maxWidth: 360,
  maxHeight: 280,
  overflow: "auto",
  background: "var(--pptx-shell-bg, #1f1f24)",
  color: "var(--pptx-shell-fg, #e6e6ea)",
  border: "1px solid var(--pptx-shell-border, #2c2c34)",
  borderRadius: 6,
  boxShadow: "0 8px 24px rgba(0, 0, 0, 0.4)",
  zIndex: 1000,
  fontSize: 12,
  padding: "4px 0",
};

export const selectionFontsListStyle: CSSProperties = {
  listStyle: "none",
  margin: 0,
  padding: 0,
};

export const selectionFontsListItemStyle: CSSProperties = {
  padding: "5px 12px",
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};

export const statusIconStyle: CSSProperties = {
  background: "transparent",
  color: "inherit",
  border: "1px solid transparent",
  borderRadius: 3,
  padding: "2px 6px",
  cursor: "pointer",
  font: "inherit",
  minHeight: 22,
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
};

export const activeIconSmStyle: CSSProperties = {
  ...statusIconStyle,
  background: "var(--pptx-shell-active, #3a3a44)",
  // Active state is signalled by background only — earlier versions
  // also flipped `borderColor` to the accent, but that 1px line was
  // perceived as a stray "white outline" against dark themes (the
  // anti-aliased thin accent stroke desaturates to off-white in JPEG
  // screenshots and on lower-DPI displays). Background-only matches
  // PowerPoint's own toolbar convention and avoids the artefact.
};

export const zoomSliderStyle: CSSProperties = {
  width: 120,
  margin: "0 6px",
};

export const zoomPctStyle: CSSProperties = {
  minWidth: 40,
  textAlign: "right",
  fontVariantNumeric: "tabular-nums",
  cursor: "pointer",
  userSelect: "none",
};
