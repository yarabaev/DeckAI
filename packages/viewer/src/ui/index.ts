/**
 * Viewer UI — React components and helpers that wrap the core
 * `<PptxPresentation>` slide stage.
 *
 * Split into:
 *
 * - **UI components** — `Ruler`, `SettingsDialog`, `SectionNav`.
 * - **Action modules** — `search`, `print`, `pdf`.
 * - **Infrastructure** — `i18n`, `themes`, `settings`, `media-inline`.
 *
 * Everything in this folder is host-agnostic: usable from the
 * built-in shell, custom React hosts, or the `<pptx-viewer>` Web
 * Component the bundle registers for vanilla / non-React mounts.
 */

export { Ruler } from "./Ruler.js";
export type { RulerProps } from "./Ruler.js";
export { SettingsDialog } from "./SettingsDialog.js";
export type { SettingsDialogProps } from "./SettingsDialog.js";
export { SectionNav } from "./SectionNav.js";
export type { SectionNavProps } from "./SectionNav.js";
export { SelectionOverlay } from "./SelectionOverlay.js";
export type {
  SelectionOverlayProps,
  SelectionBox,
  RubberBandRect,
} from "./SelectionOverlay.js";
export { ShortcutsDialog } from "./ShortcutsDialog.js";
export type { ShortcutsDialogProps } from "./ShortcutsDialog.js";
export { FontUsageIndicator } from "./FontUsageIndicator.js";
export type {
  FontUsageIndicatorProps,
  FontMappingRow,
} from "./FontUsageIndicator.js";

export { searchSlides, splitMatches, stripMatchMarkers } from "./search.js";
export type { SearchHit } from "./search.js";

export {
  THEMES,
  dark,
  light,
  highContrast,
  applyTheme,
  detectSystemTheme,
  subscribeSystemTheme,
} from "./themes.js";
export type { ThemeVars } from "./themes.js";

export { printDeck } from "./print.js";
export type { PrintOptions } from "./print.js";

export { exportToPdf } from "./pdf.js";
export type { ExportPdfOptions } from "./pdf.js";

export {
  loadSettings,
  saveSettings,
  subscribeSettings,
  STORAGE_KEY as SETTINGS_STORAGE_KEY,
  DEFAULT_SETTINGS,
} from "./settings.js";
export type { ViewerSettings, ThemeMode, RulerUnit } from "./settings.js";

export {
  t,
  setLocale,
  getActiveLocale,
  getDetectedLocale,
  detectLocale,
  subscribeLocale,
  resetLocaleCache,
  SUPPORTED_LOCALES,
} from "./i18n.js";
export type { Locale, ResolvedLocale, MessageKey } from "./i18n.js";

export { inlineMediaAsDataUrls } from "./media-inline.js";
