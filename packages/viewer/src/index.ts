/**
 * `@slideglance/viewer` — React-based PPTX presentation viewer.
 *
 * Originally a Lit-based Web Component shell; now a pure React
 * implementation. The Lit / Custom-Element surface is no longer
 * exported.
 *
 * Public API:
 *
 * - `<PptxPresentation>` — top-level shell with toolbar, stage,
 *   status bar, slide rendering, zoom, keyboard navigation.
 * - `createWorkerController` — Web Worker-backed `SlideController`
 *   for browser hosts. Native hosts (Tauri etc.) can supply their
 *   own `SlideController` implementation that talks to a Rust
 *   backend.
 * - SVG helper utilities (`prepareSvg`, `parseAspect`,
 *   `rewriteMediaRefs`, `extractAndStripFontStyle`) for hosts that
 *   render slides directly without the React shell.
 * - Types: `SlideController`, `RenderedSlide`, `MediaBlob`, `SlideSvg`,
 *   `SlideMeta`, `TextRenderMode`, `FontFallback`.
 */

export { PptxPresentation } from "./PptxPresentation.js";
export type { PptxPresentationProps } from "./PptxPresentation.js";

export { SettingsDialog } from "./ui/SettingsDialog.js";
export type { SettingsDialogProps } from "./ui/SettingsDialog.js";

export { createWorkerController } from "./worker-controller.js";

export {
  parseAspect,
  prepareSvg,
  rewriteMediaRefs,
  extractAndStripFontStyle,
  extractFontStyleCss,
} from "./svg-utils.js";

export type {
  TextRenderMode,
  FontFallback,
  SlideSvg,
  MediaBlob,
  RenderedSlide,
  SlideController,
  SlideMeta,
} from "./types.js";

// i18n surface — re-exported from the ui subpackage so embedding
// hosts (Tauri shell, playgrounds, custom React mounts, the
// `<pptx-viewer>` Web Component) can localize their own UI strings
// using the same catalog as the viewer.
export {
  t,
  setLocale,
  getActiveLocale,
  getDetectedLocale,
  detectLocale,
  subscribeLocale,
  resetLocaleCache,
  SUPPORTED_LOCALES,
} from "./ui/i18n.js";
export type { Locale, ResolvedLocale, MessageKey } from "./ui/i18n.js";
