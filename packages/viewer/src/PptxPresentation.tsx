import {
  type CSSProperties,
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { parseAspect } from "./svg-utils.js";
import type {
  SlideController,
  SlideMeta,
  SlideSvg,
  TypefaceUsage,
} from "./types.js";
import { GridView } from "./presentation/GridView.js";
import { NotesPanel } from "./presentation/NotesPanel.js";
import { ThumbnailSidebar } from "./presentation/Thumbnail.js";
import { SlideshowOverlay } from "./presentation/SlideshowOverlay.js";
import { StatusBar } from "./presentation/StatusBar.js";
import { Toolbar } from "./presentation/Toolbar.js";
import { useDeckLoader } from "./presentation/use-deck-loader.js";
import { useEmbeddedFontStyle } from "./presentation/use-embedded-font-style.js";
import { useKeyboardShortcuts } from "./presentation/use-keyboard-shortcuts.js";
import { usePrintPdfExport } from "./presentation/use-print-pdf-export.js";
import { useRulerGeometry } from "./presentation/use-ruler-geometry.js";
import { useSelectionStateMachine } from "./presentation/use-selection-state-machine.js";
import { useSlideCache } from "./presentation/use-slide-cache.js";
import { useSlideDomMount } from "./presentation/use-slide-dom-mount.js";
import { useWheelZoomNav } from "./presentation/use-wheel-zoom-nav.js";
import {
  RULER_SIZE,
  SHELL_GLOBAL_CSS,
  bodyStyle,
  iconButtonStyle,
  loadingOverlayStyle,
  loadingSpinnerStyle,
  loadingTextStyle,
  overlayStyle,
  progressBackdropStyle,
  progressBarFillStyle,
  progressBarIndeterminateStyle,
  progressBarTrackStyle,
  progressCounterStyle,
  progressHostStyle,
  progressPanelStyle,
  progressStepStyle,
  progressTitleStyle,
  rootStyle,
  rulerCornerStyle,
  rulerHStyle,
  rulerVStyle,
  searchDrawerStyle,
  searchEmptyStyle,
  searchHeaderStyle,
  searchHitNumStyle,
  searchInputStyle,
  searchItemStyle,
  searchListStyle,
  sidebarResizerStyle,
  sidebarStyle,
  stageAreaStyle,
  stageStyle,
  stageWrapStyle,
} from "./presentation/styles.js";
import type { CachedSlide } from "./presentation/types.js";
import { Ruler } from "./ui/Ruler.js";
import { SettingsDialog } from "./ui/SettingsDialog.js";
import { SectionNav } from "./ui/SectionNav.js";
import { SelectionOverlay } from "./ui/SelectionOverlay.js";
import { ShortcutsDialog } from "./ui/ShortcutsDialog.js";
// FontUsageIndicator is now mounted inside `presentation/StatusBar.tsx`.
import { searchSlides, type SearchHit } from "./ui/search.js";
// Print + PDF export top-level handlers live in
// `presentation/use-print-pdf-export.ts`.
import {
  applyTheme,
  detectSystemTheme,
  subscribeSystemTheme,
  dark,
  light,
  highContrast,
  type ThemeVars,
} from "./ui/themes.js";
import {
  clampSidebarWidth,
  loadSettings,
  saveSettings,
  SIDEBAR_WIDTH_MAX,
  SIDEBAR_WIDTH_MIN,
  subscribeSettings,
  type ThemeMode,
  type ViewerSettings,
} from "./ui/settings.js";
import { subscribeLocale, t } from "./ui/i18n.js";
import { X } from "@phosphor-icons/react";

const ZOOM_MIN = 0.25;
const ZOOM_MAX = 8;
/** Width of the sidebar resize handle (CSS px). The full body grid
 * dedicates exactly this much horizontal space to the splitter so the
 * stage area's width tracks `sidebarWidth + RESIZER_WIDTH`. */
const SIDEBAR_RESIZER_WIDTH = 6;
// `RULER_SIZE` and the rest of the shell's CSS-in-JS constants live
// in `presentation/styles.ts`. The same module also exports
// `SHELL_GLOBAL_CSS` — the global stylesheet the shell mounts once
// per render to drive scrollbar theming, slideshow corner-nav fade-
// in, the loading-overlay spinner keyframes, and reset of native
// focus / touch chrome on shell buttons.
//
// Sub-components (`Thumbnail`, `NotesPanel`, `GridView`) live in
// `presentation/{Thumbnail,NotesPanel,GridView}.tsx` and import
// from `presentation/styles.ts` directly.

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

/**
 * Build the tooltip for deck-wide actions (Print / PDF / Slideshow)
 * so the user can see *why* the button is disabled — empty deck vs.
 * still-prefetching — instead of just a dead control.
 */
function deckGateTitle(
  base: string,
  ready: boolean,
  slideCount: number,
): string {
  if (slideCount === 0) return t("output.gateLoadFirst");
  if (!ready)
    return t("output.gatePreparing", { current: 0, total: slideCount });
  return base;
}

type ThemeName = "dark" | "light" | "high-contrast";
const THEME_TABLE: Record<ThemeName, ThemeVars> = {
  dark,
  light,
  "high-contrast": highContrast,
};
function resolveTheme(mode: ThemeMode): ThemeName {
  if (mode === "auto") return detectSystemTheme();
  return mode;
}

interface SlideMetaResolver {
  (slide: number): Promise<SlideMeta | null> | SlideMeta | null;
}

export interface PptxPresentationProps {
  controller: SlideController | null;
  name?: string | null;
  slideCount?: number;
  src?: Uint8Array | ArrayBuffer | string | null;
  className?: string;
  style?: CSSProperties;
  toolbarStart?: ReactNode;
  toolbarEnd?: ReactNode;
  resolveMeta?: SlideMetaResolver;
  /**
   * Disable the deck-wide background prefetch.
   *
   * The viewer normally walks every slide in the background after the
   * deck loads so deck-wide actions (Print / PDF / Slideshow / Search)
   * are instant. On native hosts (Tauri viewer-desktop) every render
   * is a synchronous IPC roundtrip plus a JSON-string serialization of
   * the SVG, so prefetching all 100+ slides eagerly stalls the shell
   * for a long time before the first slide even paints.
   *
   * When `noPrefetch` is true:
   * - Slides are rendered lazily — only when navigated to.
   * - Print / PDF / Slideshow / Search trigger their own
   *   `ensureAllSlidesRendered()` on demand (the gate that hides the
   *   buttons until the prefetch finishes is removed).
   *
   * Browser hosts (worker-backed) keep prefetching by default because
   * the worker is concurrent with the main thread.
   */
  noPrefetch?: boolean;
  /**
   * Fired exactly once after the first slide's SVG has been appended
   * to the DOM (the moment a user can see content). Hosts use this to
   * dismiss their own loading overlays without having to guess at a
   * delay — a fast deck open shouldn't dwell behind a spinner that
   * outlasts the actual wait, and a slow first slide shouldn't drop
   * the spinner while the stage is still blank.
   *
   * Re-fires when the deck is replaced (i.e. the host re-keys this
   * component to remount it for a new file) — internal first-render
   * tracking is reset on mount.
   */
  onReady?: () => void;
  /**
   * Optional host-supplied stylesheet whose `@font-face` rules are
   * loaded into the worker's `FontFaceSet` alongside the deck's
   * embedded fonts. The chrome-extension passes its bundled Google
   * Fonts CSS here so decks that name `Anton`, `Alata`, etc. resolve
   * to the bundled face during canvas measurement (and not just at
   * paint time via the document-level stylesheet).
   *
   * Without this, decks that ship MTX-compressed embedded fonts —
   * which our renderer drops — would measure with the worker's OS
   * fallback metrics and produce wider lines than the browser will
   * paint.
   */
  bundledFontDefsCss?: string;
  /**
   * When `true`, treat every `src` change as an in-place edit-cycle
   * update of the same logical deck rather than a brand-new deck
   * open: the current slide index, zoom level, and pan offsets are
   * preserved across the reload. Hosts that drive a live editing
   * surface (e.g. the pom VS Code preview, where every keystroke
   * produces a fresh PPTX byte buffer) set this so the viewer
   * doesn't snap back to slide 1 / zoom 1 after each edit.
   *
   * Cache treatment is gated by `invalidatedSlides`: see that prop.
   */
  incrementalUpdate?: boolean;
  /**
   * 1-based indices of slides whose cache entries should be flushed
   * on the next `src` change. Only consulted when
   * `incrementalUpdate` is true. Pair this with a host-side
   * per-slide hash diff to achieve true surgical updates: the host
   * computes which slides' source actually changed between edits
   * and forwards that list here, so the viewer keeps cached SVGs
   * for everything else.
   *
   *  - `undefined`: viewer flushes the entire cache (safe default
   *    when the host has no diff).
   *  - `[]`: keep the entire cache (host asserts nothing changed
   *    visually).
   *  - `[3, 5, …]`: drop only those entries; navigation refetches
   *    them, the rest stay cached.
   */
  invalidatedSlides?: number[];
}

// `CachedSlide` lives in `presentation/types.ts` so the sub-component
// modules can import it without circling back through this file.

/**
 * Top-level presentation shell. React port of the original Lit
 * `<pptx-presentation>` Web Component, mirroring the same chrome:
 *
 *     ┌───────────────────────────────────────────────┐
 *     │ ribbon (filename / nav / search / print / …)  │
 *     ├──────────┬────────────────────────────────────┤
 *     │ thumb    │ stage (slide rendering with ruler) │
 *     │ + sects  │                                    │
 *     │          ├────────────────────────────────────┤
 *     │          │ notes (collapsible)                │
 *     ├──────────┴────────────────────────────────────┤
 *     │ status bar (slide / view modes / zoom slider) │
 *     └───────────────────────────────────────────────┘
 */
export function PptxPresentation(props: PptxPresentationProps): JSX.Element {
  const {
    controller,
    name,
    slideCount: externalSlideCount,
    src,
    className,
    style,
    toolbarStart,
    toolbarEnd,
    resolveMeta,
    noPrefetch = false,
    onReady,
  } = props;
  // One-shot guard for `onReady` — the SVG mount effect re-runs on
  // every layout / sidebar / notes / view-mode change, but the host
  // wants the callback fired exactly once per deck. Re-mounting the
  // component (host re-keys on file swap) zeroes the ref naturally.
  const onReadyFiredRef = useRef<boolean>(false);

  // ---- Settings + theme + locale -------------------------------------------
  const [settings, setSettings] = useState<ViewerSettings>(() =>
    loadSettings(),
  );
  const [theme, setTheme] = useState<ThemeName>(() =>
    resolveTheme(loadSettings().themeMode),
  );
  const [, setLocaleTick] = useState(0);
  const rootRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const unsub = subscribeSettings((next) => {
      setSettings(next);
      setTheme(resolveTheme(next.themeMode));
      // Mirror persisted width updates from another surface (Settings
      // dialog, second viewer instance) into local drag state. Drags
      // write to local state directly, so this is a no-op for the
      // event the drag itself emitted.
      setSidebarWidth((prev) =>
        prev === next.sidebarWidth ? prev : next.sidebarWidth,
      );
    });
    return unsub;
  }, []);
  useEffect(() => {
    if (settings.themeMode !== "auto") return;
    const unsub = subscribeSystemTheme(() => setTheme(detectSystemTheme()));
    return unsub;
  }, [settings.themeMode]);
  useEffect(() => {
    const vars = THEME_TABLE[theme];
    if (rootRef.current) applyTheme(rootRef.current, vars);
    // Also propagate to <html> so chrome outside the shell (host page
    // body, fixed-position overlays / native scrollbar gutters in
    // Tauri / Electron windows) picks up the same theme variables.
    if (typeof document !== "undefined" && document.documentElement) {
      applyTheme(document.documentElement, vars);
    }
  }, [theme]);
  useEffect(() => {
    const unsub = subscribeLocale(() => setLocaleTick((n) => n + 1));
    return unsub;
  }, []);

  // ---- Slide state ---------------------------------------------------------
  const [slideCount, setSlideCount] = useState<number>(externalSlideCount ?? 0);
  // Per-typeface fallback report from controller.open(). Empty until a
  // deck is loaded; reset on every reopen so the status-bar indicator
  // reflects the current deck.
  const [fontUsage, setFontUsage] = useState<TypefaceUsage[]>([]);
  // Bare CSS body of the deck's `@font-face` declarations
  // (`<p:embeddedFontLst>` faces from PPTX), produced by
  // `slideglance-wasm`'s `fontDefs()` and stripped of its SVG `<defs>`
  // wrapper. Mounted into a deck-scoped `<style>` element in
  // `document.head` (see effect below) so that browser-side SVG
  // rendering can resolve embedded family names like "Noto Sans Bold"
  // — without this mount they would only live in the worker's
  // FontFaceSet (which is invisible to the document) and the SVG would
  // silently fall through the chain to a system font.
  const [fontDefsCss, setFontDefsCss] = useState<string>("");
  const [currentSlide, setCurrentSlide] = useState<number>(1);
  const [zoom, setZoom] = useState<number>(1);
  const [panX, setPanX] = useState<number>(0);
  const [panY, setPanY] = useState<number>(0);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [phase, setPhase] = useState<string>("");
  // Long-running export pipelines (Print / PDF) post structured progress
  // here so the host can show a centred overlay instead of relying on
  // the easy-to-miss status bar at the bottom of the shell. `null`
  // means the deck is idle.
  const [progress, setProgress] = useState<{
    title: string;
    step: string;
    current?: number;
    total?: number;
  } | null>(null);
  const [stageSize, setStageSize] = useState<{ w: number; h: number }>({
    w: 0,
    h: 0,
  });
  const [viewMode, setViewMode] = useState<"normal" | "grid">("normal");
  const [notesOpen, setNotesOpen] = useState<boolean>(false);
  // Sidebar width is user-resizable via the splitter handle and
  // persisted across sessions (clamped server-side in `loadSettings`).
  // Local state tracks the live value during a drag for low-latency
  // updates; the persisted copy in `settings.sidebarWidth` is rewritten
  // on pointer-up to avoid spamming localStorage from a 60Hz drag loop.
  const [sidebarWidth, setSidebarWidth] = useState<number>(
    () => loadSettings().sidebarWidth,
  );
  const sidebarResizeStartRef = useRef<{
    pointerId: number;
    startX: number;
    startWidth: number;
  } | null>(null);
  const [searchOpen, setSearchOpen] = useState<boolean>(false);
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [searchHits, setSearchHits] = useState<SearchHit[]>([]);
  const [settingsOpen, setSettingsOpen] = useState<boolean>(false);
  const [shortcutsOpen, setShortcutsOpen] = useState<boolean>(false);
  const [slideshow, setSlideshow] = useState<boolean>(false);
  // `allSlidesReady` gates the deck-wide actions (Print / PDF /
  // Slideshow). When the background prefetch (or a manual
  // `ensureAllSlidesRendered`) finishes, the gate opens so those
  // actions stop showing partial output. Mirrors the historic Lit
  // shell's `allSlidesReady` flag.
  const [allSlidesReady, setAllSlidesReady] = useState<boolean>(false);
  // Selection model — set of `data-sp-id` strings for shapes selected
  // on the active slide, plus a live rubber-band rect (viewport coords)
  // when the user is dragging on empty stage. Both are derived data the
  // SelectionOverlay turns into stage-relative px boxes via `bboxMap`.
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  // Status-bar selection-font popover. Toggled by clicking the
  // "폰트: …" / "Font: …" label so users with multi-typeface
  // selections can see the full list instead of a "+N more" summary
  // they can't drill into.
  const [selectionFontsOpen, setSelectionFontsOpen] = useState<boolean>(false);
  const selectionFontsRef = useRef<HTMLDivElement | null>(null);
  const [rubberBand, setRubberBand] = useState<{
    x0: number;
    y0: number;
    x1: number;
    y1: number;
  } | null>(null);
  // Viewport bbox cache, keyed by sp-id. Built once per slide mount
  // (or when the SVG host re-renders) so hit-testing during pan/zoom
  // doesn't have to call getBBox/getCTM per shape per frame.
  const bboxMapRef = useRef<
    Map<string, { x: number; y: number; w: number; h: number }>
  >(new Map());
  // Pointer-down snapshot used by the selection state machine. Stored
  // as a ref so re-renders during a drag don't reset it.
  const pointerDownAtRef = useRef<{
    x: number;
    y: number;
    target: HTMLElement | null;
  } | null>(null);
  const [textEditId, setTextEditId] = useState<string | null>(null);
  const [spaceHeld, setSpaceHeld] = useState<boolean>(false);
  const panStartRef = useRef<{
    x: number;
    y: number;
    panX: number;
    panY: number;
  } | null>(null);
  // `slideCache` + `requestSlide` + `ensureAllSlidesRendered` live in
  // `presentation/use-slide-cache.ts` — the hook is wired up further
  // down once the deck-epoch and pending-task refs have been declared.

  const stageRef = useRef<HTMLDivElement | null>(null);
  const slideRef = useRef<HTMLDivElement | null>(null);
  // The slideshow overlay mounts its own `<main>` and slide
  // container. Sharing a single `stageRef` / `slideRef` between
  // them caused a use-after-free when slideshow exits: React calls
  // the slideshow ref-callback with `null`, which clears the slot,
  // but the persistent normal `<main>`'s ref-callback never re-runs
  // because that element didn't remount. Result: stale `stageSize`
  // from the fullscreen viewport leaking into the post-exit render
  // → scrollbars + visibly larger slide. Two separate refs keep the
  // mode-specific pointers independent.
  const slideshowStageRef = useRef<HTMLDivElement | null>(null);
  const slideshowSlideRef = useRef<HTMLDivElement | null>(null);
  // Ref-handle for Cmd/Ctrl+P. Captured before the keyboard handler
  // is registered so we don't have to thread `handlePrint` through
  // the effect's dependency array (which would re-bind the listener
  // on every render).
  const handlePrintRef = useRef<(() => Promise<void>) | null>(null);
  const pendingRef = useRef<Map<number, Promise<CachedSlide | null>>>(
    new Map(),
  );
  // Monotonic deck-epoch counter. Incremented on every deck swap
  // (`externalSlideCount` / `name` change). Each `requestSlide` task
  // captures the epoch when it starts; if the epoch advances while
  // the IPC roundtrip is in flight, the stale resolution is dropped
  // before it stamps the new deck's cache. Without this, a deck swap
  // triggered while slide N is mid-render leaves the *old* SVG in
  // the new deck's `slideCache.get(N)` — which is exactly the
  // "previous deck's thumbnail" symptom.
  const deckEpochRef = useRef(0);

  // Sync external slideCount + flush slide cache on deck swap.
  //
  // Triggered by either `slideCount` *or* `name` changing — relying
  // only on `slideCount` lets a new deck with the same length re-use
  // the previous deck's cached SVG (and thus stale thumbnails).
  // `name` is the host-supplied display label; for native and worker
  // hosts that's always the file basename, so a new file = new name
  // = cache flush. The cache cleanup also revokes any lingering blob
  // URLs from earlier renders for safety, even though the current
  // pipeline inlines media as data URLs.
  useEffect(() => {
    if (typeof externalSlideCount !== "number") return;
    deckEpochRef.current += 1; // invalidate every in-flight task
    setSlideCount(externalSlideCount);
    setCurrentSlide(1);
    setAllSlidesReady(false);
    setSelectedIds(new Set());
    setTextEditId(null);
    setRubberBand(null);
    pendingRef.current.clear();
    setSlideCache((prev) => {
      for (const c of prev.values()) {
        for (const u of c.blobUrls) URL.revokeObjectURL(u);
      }
      return new Map();
    });
  }, [externalSlideCount, name]);

  // Reset transient deck state on file change for the src-driven path.
  //
  // The externalSlideCount-gated effect above handles file change for
  // hosts that own deck loading. The src-driven auto-open path
  // (vscode-extension preview, web-playground file picker) has no
  // equivalent: useDeckLoader is designed around either in-place edits
  // (incrementalUpdate=true preserves currentSlide / zoom / pan) or
  // initial mount, neither of which clears stale state when the host
  // points the viewer at a different file.
  //
  // Without this, switching files preserves currentSlide from the old
  // deck. If the new deck has fewer slides, the active-slide effect
  // immediately fires `renderSlide(stale_index)` against the worker,
  // which returns "slide N not found" — visible as an empty preview
  // or an error overlay. deckEpochRef also stays at 0 in this path,
  // so the stale-resolution guards in useSlideCache silently no-op
  // and in-flight renders from the old deck stamp their results into
  // the new deck's cache.
  useEffect(() => {
    if (typeof externalSlideCount === "number") return;
    deckEpochRef.current += 1;
    setSlideCount(0);
    setCurrentSlide(1);
    setZoom(1);
    setPanX(0);
    setPanY(0);
    setAllSlidesReady(false);
    setSelectedIds(new Set());
    setTextEditId(null);
    setRubberBand(null);
    pendingRef.current.clear();
    setSlideCache((prev) => {
      for (const c of prev.values()) {
        for (const u of c.blobUrls) URL.revokeObjectURL(u);
      }
      return new Map();
    });
  }, [name]);

  // Stage sizing.
  //
  // Re-attached whenever `slideshow` flips because we mount two
  // different `<main>` elements (normal vs slideshow overlay) and
  // both share the same `stageRef`. Without re-binding the
  // ResizeObserver to the *active* element, the observer keeps
  // watching whichever main mounted first — which is `display: none`
  // in the inactive mode and reports size 0, collapsing `fit` and
  // hiding the slide.
  useEffect(() => {
    // Pick the active stage based on the current mode. In slideshow
    // mode the overlay's `<main>` is the truth; in normal mode the
    // gridded body's `<main>` is.
    const stage = slideshow ? slideshowStageRef.current : stageRef.current;
    if (!stage) return;
    const seed = stage.getBoundingClientRect();
    if (seed.width > 0 && seed.height > 0) {
      setStageSize({ w: seed.width, h: seed.height });
    } else if (slideshow) {
      // Fullscreen transition often hasn't laid out yet on the same
      // tick — fall back to viewport so the first frame still gets
      // a non-zero `fit`.
      setStageSize({ w: window.innerWidth, h: window.innerHeight });
    }
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      const r = entry.contentRect;
      setStageSize({ w: Math.max(0, r.width), h: Math.max(0, r.height) });
    });
    observer.observe(stage);
    // Window resize matters only in slideshow (fullscreen) — the
    // wrapping body handles resize for the windowed case.
    const onWinResize = (): void => {
      const cur = slideshow ? slideshowStageRef.current : stageRef.current;
      if (!cur) return;
      const r = cur.getBoundingClientRect();
      setStageSize({ w: Math.max(0, r.width), h: Math.max(0, r.height) });
    };
    if (slideshow) window.addEventListener("resize", onWinResize);
    return () => {
      observer.disconnect();
      if (slideshow) window.removeEventListener("resize", onWinResize);
    };
  }, [slideshow]);

  // Slide rendering + caching pipeline — see
  // `presentation/use-slide-cache.ts`. Declared before the
  // deck-loader hook so the loader's `setSlideCache` argument
  // refers to the live setter.
  const { slideCache, setSlideCache, requestSlide, ensureAllSlidesRendered } =
    useSlideCache({
      controller,
      slideCount,
      currentSlide,
      noPrefetch,
      resolveMeta,
      allSlidesReady,
      setAllSlidesReady,
      setErrorMsg,
      setPhase,
      deckEpochRef,
      pendingRef,
    });

  // Optional auto-open from `src` (browser path) — see
  // `presentation/use-deck-loader.ts`.
  useDeckLoader({
    controller,
    src,
    externalSlideCount,
    bundledFontDefsCss: props.bundledFontDefsCss,
    incrementalUpdate: props.incrementalUpdate,
    invalidatedSlides: props.invalidatedSlides,
    setPhase,
    setSlideCount,
    setFontUsage,
    setFontDefsCss,
    setCurrentSlide,
    setZoom,
    setPanX,
    setPanY,
    setErrorMsg,
    setSlideCache,
  });

  // Mount the deck's embedded `@font-face` declarations into a
  // document-scoped `<style>` — see
  // `presentation/use-embedded-font-style.ts`.
  useEmbeddedFontStyle(fontDefsCss);

  // Cache cleanup intentionally left unmanaged on unmount.
  //
  // Earlier this effect revoked every cached blob URL when the
  // component tore down. That is correct in production but races
  // catastrophically with React 18 StrictMode (development) and HMR:
  // both double-mount the component, so the *first* mount's cleanup
  // revokes the URLs that the *second* mount has already stamped
  // into its slideCache. The second-mount thumbnails then load the
  // stale URLs from `cached.preparedSvg` and fail with
  // `net::ERR_FILE_NOT_FOUND` — exactly the symptom users see when
  // the grid view paints with random tiles missing.
  //
  // We rely on the browser's natural blob lifetime: blob URLs become
  // unreachable when the document is torn down (window close,
  // navigation), at which point the GC reclaims their backing bytes.
  // The brief window between component unmount and document teardown
  // leaks a few MB of media that the browser will collect anyway —
  // a fair trade for never showing broken thumbnails in dev.

  const activeSlide = slideCache.get(currentSlide);
  const slideSvg = activeSlide?.preparedSvg ?? "";
  const slideMeta = activeSlide?.meta ?? null;

  // Mount SVG into the slide host + rebuild bbox cache — see
  // `presentation/use-slide-dom-mount.ts`.
  useSlideDomMount({
    slideSvg,
    currentSlide,
    slideshow,
    slideRef,
    slideshowSlideRef,
    bboxMapRef,
    onReadyFiredRef,
    onReady,
    setErrorMsg,
    sidebarWidth,
    notesOpen,
    viewMode,
    stageW: stageSize.w,
    stageH: stageSize.h,
  });

  // Clear selection when navigating away (sp-ids are slide-scoped).
  useEffect(() => {
    setSelectedIds(new Set());
    setTextEditId(null);
    setRubberBand(null);
  }, [currentSlide]);

  // Keyboard shortcuts — see `presentation/use-keyboard-shortcuts.ts`
  // for the full binding table and rationale.
  useKeyboardShortcuts({
    slideCount,
    allSlidesReady,
    searchOpen,
    slideshow,
    selectedIds,
    textEditId,
    handlePrintRef,
    bboxMapRef,
    slideRef,
    setSearchOpen,
    setSlideshow,
    setSelectedIds,
    setTextEditId,
    setRubberBand,
    setSpaceHeld,
    setShortcutsOpen,
    setCurrentSlide,
    setZoom,
    setPanX,
    setPanY,
  });

  // Pinch / Cmd-wheel zoom + plain-wheel slide navigation.
  //
  // Browsers deliver three flavours of wheel through the same event:
  //   1. Plain scroll (mouse wheel, two-finger trackpad scroll) —
  //      `ctrlKey: false`, large `deltaY` (often > 50 per notch).
  //   2. Trackpad pinch — synthesised as `wheel` with `ctrlKey: true`
  //      and small `deltaY` (~1-10 per frame) at high frequency.
  //   3. Cmd-wheel — explicit modifier, large `deltaY`.
  //
  // (2) and (3) are zoom intents; (1) drives slide navigation when
  // the stage scroll has reached its top / bottom edge — a
  // PowerPoint-style "scroll past the edge to flip pages" gesture.
  // While the slide is taller than the stage (zoomed in) the wheel
  // scrolls within it as usual; the navigation only kicks in once
  // the user has hit the boundary. To stop trackpad inertia from
  // skipping multiple slides per gesture, the boundary requires a
  // configurable extra travel (`BOUNDARY_THRESHOLD`) before the
  // jump fires, plus a brief post-jump cooldown so the tail end of
  // an inertial gesture can't immediately advance again.
  // Wheel + pinch handler — see `presentation/use-wheel-zoom-nav.ts`.
  useWheelZoomNav({
    stageRef,
    slideshow,
    viewMode,
    slideCount,
    setZoom,
    setCurrentSlide,
  });

  // Fullscreen change handling for slideshow exit.
  useEffect(() => {
    const onFs = (): void => {
      if (!document.fullscreenElement && slideshow) setSlideshow(false);
    };
    document.addEventListener("fullscreenchange", onFs);
    return () => document.removeEventListener("fullscreenchange", onFs);
  }, [slideshow]);

  // ---- Selection state machine --------------------------------------------
  // See `presentation/use-selection-state-machine.ts` — owns the
  // pointer-down/move/up dispatch, rubber-band hit-test, double-click
  // text-edit entry, selection-bbox projection, authored-font
  // derivation, and the status-bar font-popover lifecycle.
  const {
    onStagePointerDown,
    onStagePointerMove,
    onStagePointerUp,
    onStageDoubleClick,
    selectionBoxes,
    selectionFonts,
    rubberBandRect,
  } = useSelectionStateMachine({
    selectedIds,
    setSelectedIds,
    rubberBand,
    setRubberBand,
    textEditId,
    setTextEditId,
    spaceHeld,
    panX,
    panY,
    setPanX,
    setPanY,
    zoom,
    stageW: stageSize.w,
    stageH: stageSize.h,
    slideSvg,
    panStartRef,
    pointerDownAtRef,
    bboxMapRef,
    stageRef,
    slideRef,
    selectionFontsOpen,
    setSelectionFontsOpen,
    selectionFontsRef,
  });

  // ---- Layout math ---------------------------------------------------------
  const aspect = useMemo(() => parseAspect(slideSvg) ?? 16 / 9, [slideSvg]);
  const fit = useMemo(() => {
    if (stageSize.w <= 0 || stageSize.h <= 0) return 0;
    return Math.min(stageSize.w, stageSize.h * aspect);
  }, [aspect, stageSize.w, stageSize.h]);
  const slideW = fit * zoom;
  const slideH = fit > 0 ? (fit / aspect) * zoom : 0;
  const canvasW = Math.max(slideW, stageSize.w);
  const canvasH = Math.max(slideH, stageSize.h);
  const zoomPct = Math.round(zoom * 100);

  const reset = useCallback(() => {
    setZoom(1);
    setPanX(0);
    setPanY(0);
  }, []);
  const setZoomFromPct = useCallback((pct: number) => {
    setZoom(clamp(pct / 100, ZOOM_MIN, ZOOM_MAX));
  }, []);

  // ---- Search --------------------------------------------------------------
  const runSearch = useCallback(
    async (q: string): Promise<void> => {
      if (!q.trim()) {
        setSearchHits([]);
        return;
      }
      const slides = await ensureAllSlidesRendered();
      setSearchHits(searchSlides(slides, q));
    },
    [ensureAllSlidesRendered],
  );

  // ---- Print / PDF ---------------------------------------------------------
  // Both handlers live in `usePrintPdfExport` — they share the same
  // pre-flight (force-render every slide, surface progress, inline
  // media as data URIs) and only differ on the final dispatch
  // (`printDeck` vs `exportToPdf`). The `handlePrintRef` indirection
  // stays here because the keyboard handler at line ~1011 needs to
  // call into `handlePrint` before its `useCallback` body has been
  // initialised on the first render.
  const { handlePrint, handleExportPdf } = usePrintPdfExport({
    name,
    ensureAllSlidesRendered,
    setProgress,
    setErrorMsg,
    setPhase,
  });
  useEffect(() => {
    handlePrintRef.current = handlePrint;
  }, [handlePrint]);

  const handleSlideshow = useCallback(async () => {
    setSlideshow(true);
    setCurrentSlide(1);
    try {
      await rootRef.current?.requestFullscreen();
    } catch {
      /* host disallowed fullscreen — soft slideshow */
    }
  }, []);

  // Section nav data.
  const sectionSlides: SlideSvg[] = useMemo(() => {
    const entries: SlideSvg[] = [];
    for (const [n, c] of slideCache) {
      entries.push({
        slide_number: n,
        svg: c.svg,
        notes: c.meta.notes ?? undefined,
        layout_name: c.meta.layout_name ?? undefined,
        section_name: c.meta.section_name ?? undefined,
      });
    }
    entries.sort((a, b) => a.slide_number - b.slide_number);
    return entries;
  }, [slideCache]);
  const hasSections = sectionSlides.some((s) => !!s.section_name);

  // Ruler geometry — see `presentation/use-ruler-geometry.ts`.
  const rulerOn =
    settings.showRuler && !slideshow && viewMode === "normal" && !!slideSvg;
  const { intrinsic, intrinsicY, rulerRect } = useRulerGeometry({
    rulerOn,
    slideSvg,
    stageRef,
    slideRef,
    slideW,
    slideH,
    panX,
    panY,
    stageW: stageSize.w,
    stageH: stageSize.h,
  });

  return (
    <div
      ref={rootRef}
      data-pptx-shell=""
      className={className}
      style={{ ...rootStyle, ...style }}
    >
      <style>{SHELL_GLOBAL_CSS}</style>
      {/* ---- Ribbon ---- */}
      <Toolbar
        toolbarStart={toolbarStart}
        toolbarEnd={toolbarEnd}
        name={name}
        slideCount={slideCount}
        currentSlide={currentSlide}
        setCurrentSlide={setCurrentSlide}
        searchOpen={searchOpen}
        setSearchOpen={setSearchOpen}
        allSlidesReady={allSlidesReady}
        noPrefetch={noPrefetch}
        deckGateTitle={deckGateTitle}
        handlePrint={handlePrint}
        handleExportPdf={handleExportPdf}
        handleSlideshow={handleSlideshow}
        shortcutsOpen={shortcutsOpen}
        setShortcutsOpen={setShortcutsOpen}
        settingsOpen={settingsOpen}
        setSettingsOpen={setSettingsOpen}
      />

      {/* ---- Body: sidebar + resizer + stage area ----
        Sidebar width is user-resizable; the splitter `<div>` sits on
        the column boundary. All three grid items keep their identity
        across renders so the imperatively-mounted slide SVG and
        thumbnail DOM cache survive layout changes. */}
      <div
        style={{
          ...bodyStyle,
          gridTemplateColumns: `${sidebarWidth}px ${SIDEBAR_RESIZER_WIDTH}px minmax(0, 1fr)`,
          display: slideshow ? "none" : "grid",
        }}
      >
        <aside
          key="sidebar"
          style={{
            ...sidebarStyle,
            gridTemplateRows: hasSections ? "auto 1fr" : "1fr",
          }}
        >
          {hasSections && (
            <SectionNav
              slides={sectionSlides}
              currentSlide={currentSlide}
              onJump={(n) => setCurrentSlide(n)}
            />
          )}
          <ThumbnailSidebar
            slideCount={slideCount}
            currentSlide={currentSlide}
            onSelect={setCurrentSlide}
            getThumbnail={requestSlide}
            aspectFallback={aspect}
            deckKey={name ?? ""}
          />
        </aside>

        {/* Splitter — drag horizontally to resize the sidebar.
            `setPointerCapture` keeps the grip even when a fast drag
            leaves the 6 px hit-zone. ARIA marks this as a vertical
            separator with min/max/now so assistive tech announces the
            current width. */}
        <div
          key="sidebar-resizer"
          role="separator"
          aria-orientation="vertical"
          aria-label={t("status.resizeSidebar")}
          aria-valuemin={SIDEBAR_WIDTH_MIN}
          aria-valuemax={SIDEBAR_WIDTH_MAX}
          aria-valuenow={sidebarWidth}
          tabIndex={0}
          style={sidebarResizerStyle}
          onPointerDown={(ev) => {
            ev.preventDefault();
            sidebarResizeStartRef.current = {
              pointerId: ev.pointerId,
              startX: ev.clientX,
              startWidth: sidebarWidth,
            };
            ev.currentTarget.setPointerCapture(ev.pointerId);
          }}
          onPointerMove={(ev) => {
            const start = sidebarResizeStartRef.current;
            if (!start || start.pointerId !== ev.pointerId) return;
            const next = clampSidebarWidth(
              start.startWidth + (ev.clientX - start.startX),
            );
            setSidebarWidth(next);
          }}
          onPointerUp={(ev) => {
            const start = sidebarResizeStartRef.current;
            if (!start || start.pointerId !== ev.pointerId) return;
            sidebarResizeStartRef.current = null;
            try {
              ev.currentTarget.releasePointerCapture(ev.pointerId);
            } catch {
              /* capture may already be released by the platform */
            }
            // Persist only on release — a 60 Hz drag would otherwise
            // hammer localStorage and re-notify every settings
            // subscriber for each frame.
            saveSettings({ sidebarWidth: clampSidebarWidth(sidebarWidth) });
          }}
          onPointerCancel={() => {
            sidebarResizeStartRef.current = null;
          }}
          onKeyDown={(ev) => {
            // Keyboard a11y: ←/→ nudge by 10 px, Home/End jump to
            // bounds. Persist on each step.
            const STEP = 10;
            let next: number | null = null;
            if (ev.key === "ArrowLeft") next = sidebarWidth - STEP;
            else if (ev.key === "ArrowRight") next = sidebarWidth + STEP;
            else if (ev.key === "Home") next = SIDEBAR_WIDTH_MIN;
            else if (ev.key === "End") next = SIDEBAR_WIDTH_MAX;
            if (next == null) return;
            ev.preventDefault();
            const clamped = clampSidebarWidth(next);
            setSidebarWidth(clamped);
            saveSettings({ sidebarWidth: clamped });
          }}
        />

        <div
          key="stage-area"
          style={{
            ...stageAreaStyle,
            gridTemplateRows: notesOpen
              ? "minmax(0, 1fr) auto"
              : "minmax(0, 1fr)",
          }}
        >
          <div
            style={{
              ...stageWrapStyle,
              padding: rulerOn ? `${RULER_SIZE}px 0 0 ${RULER_SIZE}px` : 0,
            }}
          >
            {rulerOn && (
              <>
                <div style={rulerCornerStyle} />
                <Ruler
                  orientation="horizontal"
                  unit={settings.rulerUnit}
                  slideOriginPx={rulerRect.originX}
                  slideExtentPx={rulerRect.extentX || slideW}
                  slideExtentCm={intrinsic.cm}
                  slideIntrinsicPx={intrinsic.px}
                  style={rulerHStyle}
                />
                <Ruler
                  orientation="vertical"
                  unit={settings.rulerUnit}
                  slideOriginPx={rulerRect.originY}
                  slideExtentPx={rulerRect.extentY || slideH}
                  slideExtentCm={intrinsicY.cm}
                  slideIntrinsicPx={intrinsicY.px}
                  style={rulerVStyle}
                />
              </>
            )}
            <main
              ref={stageRef}
              style={{
                ...stageStyle,
                cursor: spaceHeld
                  ? panStartRef.current
                    ? "grabbing"
                    : "grab"
                  : undefined,
              }}
              onPointerDown={onStagePointerDown}
              onPointerMove={onStagePointerMove}
              onPointerUp={onStagePointerUp}
              onPointerCancel={onStagePointerUp}
              onDoubleClick={onStageDoubleClick}
            >
              {viewMode === "normal" ? (
                <>
                  {/*
                    Slide canvas is always mounted so the parsed SVG
                    survives layout changes (sidebar toggle, zoom, …).
                    If we conditionally unmount the host, the
                    DOMParser-injected `<svg>` is lost; the
                    `[slideSvg]` effect doesn't refire because the
                    state didn't change, so the re-mounted host stays
                    blank until the next slide load.
                   */}
                  <div
                    style={{
                      width: canvasW,
                      height: canvasH,
                      position: "relative",
                      visibility: slideSvg ? "visible" : "hidden",
                    }}
                  >
                    <div
                      ref={slideRef}
                      style={{
                        width: slideW,
                        height: slideH,
                        position: "absolute",
                        left: "50%",
                        top: "50%",
                        transform: `translate(calc(-50% + ${panX}px), calc(-50% + ${panY}px))`,
                        background: "white",
                        boxShadow:
                          "0 4px 12px var(--pptx-shell-shadow, rgba(0, 0, 0, 0.45))",
                      }}
                    />
                    <SelectionOverlay
                      boxes={selectionBoxes}
                      rubberBand={rubberBandRect}
                    />
                  </div>
                  {!slideSvg &&
                    (errorMsg ? (
                      <div style={overlayStyle}>{errorMsg}</div>
                    ) : phase ? (
                      // `phase` is set while we're parsing the deck or
                      // preparing the next slide — surface a prominent
                      // centred loading panel with a spinner so the
                      // user doesn't have to scan the bottom-left
                      // status bar to know something is happening.
                      <div style={loadingOverlayStyle} role="status">
                        <div style={loadingSpinnerStyle} aria-hidden="true" />
                        <div style={loadingTextStyle}>
                          {t("viewer.loading")}
                        </div>
                      </div>
                    ) : slideCount === 0 ? (
                      <div style={overlayStyle}>{t("viewer.empty")}</div>
                    ) : (
                      <div style={loadingOverlayStyle} role="status">
                        <div style={loadingSpinnerStyle} aria-hidden="true" />
                        <div style={loadingTextStyle}>
                          {t("viewer.loading")}
                        </div>
                      </div>
                    ))}
                </>
              ) : (
                <GridView
                  slideCount={slideCount}
                  currentSlide={currentSlide}
                  cache={slideCache}
                  aspect={aspect}
                  onSelect={(n) => {
                    setCurrentSlide(n);
                    setViewMode("normal");
                  }}
                  getThumbnail={requestSlide}
                  deckKey={name ?? ""}
                />
              )}
            </main>
          </div>
          {notesOpen && (
            <NotesPanel currentSlide={currentSlide} meta={slideMeta} />
          )}
        </div>

        {/* Search drawer (overlay panel anchored to body). */}
        {searchOpen && (
          <div style={searchDrawerStyle}>
            <header style={searchHeaderStyle}>
              <span>{t("search.title")}</span>
              <button
                style={iconButtonStyle}
                onClick={() => setSearchOpen(false)}
                title={t("common.close")}
                aria-label={t("common.close")}
              >
                <X size={14} weight="bold" />
              </button>
            </header>
            <input
              type="search"
              placeholder={t("search.placeholder")}
              value={searchQuery}
              onChange={(e) => {
                setSearchQuery(e.target.value);
                void runSearch(e.target.value);
              }}
              style={searchInputStyle}
              autoFocus
            />
            {searchHits.length === 0 ? (
              <div style={searchEmptyStyle}>
                {searchQuery ? t("search.noMatches") : t("search.typeToSearch")}
              </div>
            ) : (
              <ul style={searchListStyle}>
                {searchHits.map((hit) => (
                  <li
                    key={`${hit.slide_number}-${hit.excerpt}`}
                    style={searchItemStyle}
                    onClick={() => {
                      setCurrentSlide(hit.slide_number);
                      setSearchOpen(false);
                    }}
                  >
                    <span style={searchHitNumStyle}>#{hit.slide_number}</span>
                    <span>{hit.excerpt.replace(/\[\/?match\]/g, "")}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </div>

      {/* ---- Slideshow overlay (presentation/SlideshowOverlay.tsx) ---- */}
      <SlideshowOverlay
        open={slideshow}
        currentSlide={currentSlide}
        slideCount={slideCount}
        setSlideshow={setSlideshow}
        setCurrentSlide={setCurrentSlide}
        fit={fit}
        slideSvg={slideSvg}
        canvasW={canvasW}
        canvasH={canvasH}
        slideW={slideW}
        slideH={slideH}
        slideshowStageRef={slideshowStageRef}
        slideshowSlideRef={slideshowSlideRef}
      />

      {/* ---- Status bar (presentation/StatusBar.tsx) ---- */}
      <StatusBar
        slideshow={slideshow}
        phase={phase}
        errorMsg={errorMsg}
        slideMeta={slideMeta}
        slideCount={slideCount}
        currentSlide={currentSlide}
        selectionFonts={selectionFonts}
        selectionFontsOpen={selectionFontsOpen}
        setSelectionFontsOpen={setSelectionFontsOpen}
        selectionFontsRef={selectionFontsRef}
        fontUsage={fontUsage}
        notesOpen={notesOpen}
        setNotesOpen={setNotesOpen}
        viewMode={viewMode}
        setViewMode={setViewMode}
        zoom={zoom}
        zoomPct={zoomPct}
        setZoom={setZoom}
        setZoomFromPct={setZoomFromPct}
        reset={reset}
      />

      {/* ---- Settings dialog ---- */}
      <SettingsDialog
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        onSettingsChange={(next) => setSettings(next)}
      />

      {/* ---- Shortcuts help dialog (?) ---- */}
      <ShortcutsDialog
        open={shortcutsOpen}
        onClose={() => setShortcutsOpen(false)}
      />

      {/* ---- Long-running export progress overlay ----
          The status bar at the bottom is too easy to miss on a 100+
          slide deck where Print / PDF can take many seconds before
          the OS print dialog or download fires. A centred modal
          surface confirms the click was received and gives a live
          counter so users don't suspect the click was lost. */}
      {progress && (
        <div
          style={progressHostStyle}
          role="status"
          aria-live="polite"
          aria-busy="true"
        >
          <div style={progressBackdropStyle} />
          <div style={progressPanelStyle}>
            <div style={progressTitleStyle}>{progress.title}</div>
            <div style={progressStepStyle}>{progress.step}</div>
            <div style={progressBarTrackStyle}>
              <div
                style={{
                  ...progressBarFillStyle,
                  ...(progress.total && progress.total > 0
                    ? {
                        width: `${Math.min(100, ((progress.current ?? 0) / progress.total) * 100)}%`,
                      }
                    : progressBarIndeterminateStyle),
                }}
              />
            </div>
            {progress.total != null && progress.total > 0 ? (
              <div style={progressCounterStyle}>
                {progress.current ?? 0} / {progress.total}
              </div>
            ) : (
              <div style={progressCounterStyle}>&nbsp;</div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// Subcomponents (`ThumbnailSidebar`, `Thumbnail`, `NotesPanel`,
// `GridView`) live in `presentation/{Thumbnail,NotesPanel,GridView}.tsx`.
