/**
 * Public types shared between the React `PptxPresentation` shell, the
 * worker-backed slide controller, and external host applications
 * (the Tauri shell, the playground, third-party integrations).
 *
 * No framework imports — these are deliberately portable so a
 * consumer can plug in its own controller (e.g. native Rust IPC) or
 * its own React UI without depending on the browser-side worker.
 */

/**
 * Text rendering mode forwarded to `@slideglance/core`.
 *
 * - `text` — emit `<text>` / `<tspan>` (selectable, accessible).
 * - `path` — emit `<path>` glyph outlines (deterministic, no font
 *   dependency at render time).
 * - `auto` — let the renderer pick per-shape.
 */
export type TextRenderMode = "text" | "path" | "auto";

/** Behaviour when a font referenced by the deck is not registered. */
export type FontFallback = "first-available" | "system" | "none";

/** Per-slide SVG payload returned by `convertPptxToSvg`. */
export interface SlideSvg {
  slide_number: number;
  svg: string;
  notes?: string;
  layout_name?: string;
  section_name?: string;
}

/** One unique media blob referenced by `pptx-media://{hash}`. */
export interface MediaBlob {
  mime: string;
  bytes: Uint8Array;
}

/** Result of a single-slide on-demand render. */
export interface RenderedSlide {
  slide: number;
  svg: string;
  media: Map<string, MediaBlob>;
  notes?: string;
  layoutName?: string;
  sectionName?: string;
}

/**
 * One typeface referenced by the deck and the SVG `font-family`
 * fallback chain emitted for it. Hosts walk `fallbackChain` against
 * `document.fonts.check()` to identify the actually-rendered font and
 * surface the mapping (e.g. "맑은 고딕 → Noto Sans KR") to the user.
 *
 * `resolvedFamily` is populated only by native (non-browser) hosts
 * that supply a path-mode font resolver — it is always `null` from
 * the WASM browser viewer worker because the browser, not Rust, picks
 * the effective font in text mode.
 */
export interface TypefaceUsage {
  /** Authored typeface name (`<a:latin typeface="…"/>` value). */
  requested: string;
  /**
   * Ordered list of font-family names the SVG `font-family` attr
   * lists, before the trailing generic (`sans-serif` / `serif`).
   * The browser walks this list and uses the first installed font.
   */
  fallbackChain: string[];
  /**
   * Family name returned by the path-mode font resolver, or `null`
   * when no resolver was supplied / none of the chain entries hit.
   */
  resolvedFamily: string | null;
}

/**
 * Slide-rendering engine. The viewer talks to one of these so the
 * actual conversion can be a Web Worker, an in-process WASM module,
 * a native IPC bridge, or a test stub. Each method returns a Promise
 * so the worker path doesn't have to bend its message-driven API.
 */
export interface SlideController {
  /**
   * Parse a PPTX byte stream, return deck metadata.
   *
   * `options.extraFontDefsCss` is an optional host-supplied stylesheet
   * whose `@font-face` rules are loaded into the worker's
   * `FontFaceSet` alongside the deck's embedded fonts. Hosts that ship
   * pre-bundled fallback fonts (e.g. the chrome-extension's bundled
   * Google Fonts) pass it here so canvas-based wrap measurement uses
   * the same metrics as the eventual paint.
   */
  open(
    bytes: Uint8Array,
    options?: { extraFontDefsCss?: string },
  ): Promise<{
    slideCount: number;
    fontDefs: string;
    fontUsage: TypefaceUsage[];
    /**
     * Per-face report of embedded `@font-face` declarations that the
     * browser refused to load (typically MicroType-Express-compressed
     * payloads our pipeline can't decode, but also subset / permission
     * issues). Empty in the steady state. The host shell may surface
     * these in the status bar so users know why a deck looks
     * different from the original.
     */
    fontLoadFailures: Array<{ family: string; reason: string }>;
    /**
     * Raw TTF byte buffers for `<p:embeddedFont>` payloads that arrived
     * as MicroType-Express compressed and which the worker decoded in
     * JS via `mtx-decompressor`. The shell registers these on
     * `document.fonts` via the FontFace API (skipping the CSS
     * `@font-face` data-URI path that would otherwise trigger
     * Chromium's eager OTS validation and console warnings). Empty
     * when the deck has no MTX-compressed embedded fonts or every
     * decode failed.
     */
    decodedFonts: Array<{
      family: string;
      weight: string;
      style: string;
      bytes: Uint8Array;
    }>;
  }>;
  /** Render one 1-based slide on demand. */
  renderSlide(slide: number): Promise<RenderedSlide>;
  /** Release the underlying document. */
  close(): void;
}

/** Per-slide metadata surfaced by the shell's status bar / overlays. */
export interface SlideMeta {
  layout_name?: string | null;
  section_name?: string | null;
  notes?: string | null;
}
