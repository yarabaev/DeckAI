/**
 * Web Worker that hosts a `PptxDocument` for incremental slide
 * rendering. Keeps the WASM heap and the parsed deck off the main
 * thread so the UI stays responsive while large decks (hundreds of
 * slides, hundreds of MB of media) parse and render.
 *
 * Protocol (every message carries an `id` so callers can correlate
 * concurrent requests):
 *
 *   main → worker:
 *     { type: "open", id, bytes }
 *     { type: "render", id, slide }
 *     { type: "close", id }
 *
 *   worker → main:
 *     { type: "opened", id, slideCount, fontDefs }
 *     { type: "rendered", id, slide, svg, media, notes?, layoutName?, sectionName? }
 *     { type: "error", id, message }
 *
 * Media bytes are transferred as `Uint8Array` instances; the renderer
 * supplies `pptx-media://{hash}` placeholders in the SVG so the main
 * thread can wrap each blob in a `URL.createObjectURL` and rewrite the
 * references before mount.
 */

// `@slideglance/core` ships several wasm-pack flavors. We resolve at
// runtime via dynamic `import()` so the viewer's library bundle
// doesn't have to inline the WASM blob — consumer bundlers handle
// the actual resolution. The `/* @vite-ignore */` directive tells
// Vite not to follow the import during the viewer's own build (it
// would otherwise try to bundle the wasm and fail).
type CoreMod = {
  default?: () => Promise<unknown>;
  PptxDocument: PptxDocumentCtor;
};

interface PptxDocumentCtor {
  new (
    bytes: Uint8Array,
    measurementFonts: Uint8Array[],
    useCanvasMeasurer?: boolean,
  ): PptxDocumentInstance;
}

// `__slideglanceMeasureText` is the JS-side hook the wasm module imports
// when the host enabled canvas-backed measurement. Installing it on
// the worker `self` global makes it visible to the wasm-bindgen
// glue's `self.__slideglanceMeasureText(...)` call site.
//
// The implementation runs measurement through an `OffscreenCanvas`
// using the SAME font / weight / kerning settings the renderer will
// emit on the SVG `<text>` element — which guarantees the wrap
// position the wasm picks lines up with the actual rendered glyph
// positions in the browser. Cross-script CJK detection chooses the
// EA font when present.
//
// We tag a few common Korean / CJK families onto the fallback chain
// so a system that has them gets accurate measurement even when the
// deck didn't embed the authored font (the `<text>` element emits
// the SAME chain via `font-family`).
// Last-resort fallback CJK families: only reached when Rust passes no
// font-family chain (pre-D3 WASM or empty segment). After D3 lands,
// Rust supplies the full chain and this list is bypassed for normal renders.
const MEASURE_FALLBACK_CJK = [
  "Pretendard",
  "Apple SD Gothic Neo",
  "맑은 고딕",
  "Malgun Gothic",
  "Noto Sans CJK KR",
  "Noto Sans KR",
];

const PT_TO_PX = 96 / 72;
const CJK_RE = /[　-鿿가-힯豈-﫿！-｠ᄀ-ᇿꥠ-꥿ힰ-퟿]/;

let measureCanvasCtx: OffscreenCanvasRenderingContext2D | null = null;
function getMeasureCtx(): OffscreenCanvasRenderingContext2D {
  if (measureCanvasCtx === null) {
    const canvas = new OffscreenCanvas(1, 1);
    measureCanvasCtx = canvas.getContext("2d", {
      willReadFrequently: false,
    }) as OffscreenCanvasRenderingContext2D;
  }
  return measureCanvasCtx;
}

const measureCache = new Map<string, number>();

function measureTextWidth(
  text: string,
  fontFamily: string | null,
  fontFamilyEa: string | null,
  fontFamilyChain: string | null,
  fontSizePt: number,
  bold: boolean,
): number {
  const isCjk = CJK_RE.test(text);
  let families: string;
  if (fontFamilyChain != null && fontFamilyChain.length > 0) {
    // KDD-15: Rust-renderer supplied the exact CSS font-family chain
    // that the SVG <text> element will declare. Use verbatim; append
    // sans-serif terminal only if the chain doesn't already end with it.
    families = fontFamilyChain.endsWith("sans-serif")
      ? fontFamilyChain
      : `${fontFamilyChain}, sans-serif`;
  } else if (fontFamily != null && fontFamily.length > 0) {
    // Rust-supplied single family: use verbatim, append terminal sans-serif if absent.
    families = fontFamily.endsWith("sans-serif")
      ? fontFamily
      : `${fontFamily}, sans-serif`;
  } else if (fontFamilyEa != null && fontFamilyEa.length > 0 && isCjk) {
    // EA font supplied but no full chain yet (pre-D3 WASM path).
    const fallbackList = MEASURE_FALLBACK_CJK.map((f) => `'${f}'`);
    families = [
      `'${fontFamilyEa.replace(/'/g, "\\'")}'`,
      ...fallbackList,
      "sans-serif",
    ].join(", ");
  } else {
    // Last-resort: no chain from Rust (pre-D3 WASM or empty segment).
    const fallbackList = isCjk ? MEASURE_FALLBACK_CJK.map((f) => `'${f}'`) : [];
    families = [...fallbackList, "sans-serif"].join(", ");
  }
  const px = fontSizePt * PT_TO_PX;
  const weight = bold ? "bold" : "normal";
  const fontDecl = `${weight} ${px}px ${families}`;
  const cacheKey = `${fontDecl}\x1f${text}`;
  const cached = measureCache.get(cacheKey);
  if (cached !== undefined) return cached;
  const ctx = getMeasureCtx();
  ctx.font = fontDecl;
  // Match the SVG <text> defaults the renderer emits (commit b537f24).
  // `fontKerning` / `letterSpacing` exist on most modern OffscreenCanvas
  // 2D contexts; cast through `any` because TS lib.dom predates them
  // in some environments.
  const c = ctx as unknown as {
    fontKerning?: string;
    letterSpacing?: string;
    wordSpacing?: string;
  };
  if (c.fontKerning !== undefined) c.fontKerning = "none";
  if (c.letterSpacing !== undefined) c.letterSpacing = "0px";
  if (c.wordSpacing !== undefined) c.wordSpacing = "0px";
  const w = ctx.measureText(text).width;
  // Cap cache to keep worker memory predictable across large decks.
  if (measureCache.size > 50_000) measureCache.clear();
  measureCache.set(cacheKey, w);
  return w;
}

// Install on the worker self global so wasm-bindgen's
// `self.__slideglanceMeasureText(...)` call resolves to this fn.
(self as unknown as Record<string, unknown>).__slideglanceMeasureText =
  measureTextWidth;

function measureLineMetrics(fontDecl: string): {
  ascent: number;
  descent: number;
  lineGap: number;
} {
  const ctx = getMeasureCtx();
  ctx.font = fontDecl;
  const m = ctx.measureText("Mg");
  // L2 unification (KDD-9 alignment): prefer fontBoundingBox*, fall back
  // to actualBoundingBox*. Path-mode line-height uses font design metrics
  // (`face.ascender / upem`); fontBoundingBoxAscent/Descent are the canvas
  // equivalent (font-wide, em-aligned). actualBoundingBox* is the ink box
  // of the literal "Mg" string and produces ratios that diverge from
  // path-mode's em-relative formula.
  const fontMetrics = m as unknown as {
    fontBoundingBoxAscent?: number;
    fontBoundingBoxDescent?: number;
  };
  const ascent =
    fontMetrics.fontBoundingBoxAscent ?? m.actualBoundingBoxAscent ?? 0;
  const descent =
    fontMetrics.fontBoundingBoxDescent ?? m.actualBoundingBoxDescent ?? 0;
  return { ascent, descent, lineGap: 0 };
}

// Install on the worker self global so wasm-bindgen's
// `self.__slideglanceMeasureLineMetrics(...)` call resolves to this fn.
(self as unknown as Record<string, unknown>).__slideglanceMeasureLineMetrics =
  measureLineMetrics;

interface PptxDocumentInstance {
  free?(): void;
  slideCount(): number;
  fontDefs(): string;
  fontUsage(): Array<{
    requested: string;
    fallback_chain: string[];
    resolved_family: string | null;
  }>;
  // Optional — older WASM builds shipped before MTX decompression
  // landed. The worker probes for it before calling.
  mtxCompressedFonts?(): Array<{
    family: string;
    weight: string;
    style: string;
    payload: Uint8Array;
  }>;
  renderSlide(
    slide: number,
    externalMedia: boolean,
    includeFontDefs: boolean,
  ): {
    slide_number: number;
    svg: string;
    notes?: string;
    layout_name?: string;
    section_name?: string;
    media: Map<string, { mime: string; bytes: Uint8Array }>;
  } | null;
}

type RenderResult = {
  slide_number: number;
  svg: string;
  notes?: string;
  layout_name?: string;
  section_name?: string;
  media: Map<string, { mime: string; bytes: Uint8Array }>;
};

let coreModulePromise: Promise<CoreMod> | null = null;
async function loadCore(): Promise<CoreMod> {
  if (coreModulePromise === null) {
    coreModulePromise = (async () => {
      // No `/* @vite-ignore */` here: the viewer's own build already
      // externalises `@slideglance/core` via `rollupOptions.external`
      // (vite.config.ts), so the bare specifier survives into the
      // published `dist/pptx-worker.js`. Consumer bundlers — the web
      // playground's Vite, the vscode-extension webview's Vite, etc.
      // — re-process this file and re-resolve the specifier against
      // their own `node_modules`. Adding the ignore directive made
      // Vite skip that re-resolution and the bare specifier leaked
      // into the runtime worker chunk, surfacing as "Failed to
      // resolve module specifier '@slideglance/core'" the first time
      // a slide rendered in the playground.
      const mod = (await import("@slideglance/core")) as unknown as CoreMod;
      if (typeof mod.default === "function") {
        try {
          await mod.default();
        } catch {
          // bundler / nodejs targets reject double-init; ignore.
        }
      }
      return mod;
    })();
  }
  return coreModulePromise;
}

let doc: PptxDocumentInstance | null = null;

interface ParsedFontFace {
  family: string;
  src: string;
}

function parseFontFacesFromCss(css: string): ParsedFontFace[] {
  const faces: ParsedFontFace[] = [];
  const blockRe = /@font-face\s*\{([^}]*)\}/g;
  let match: RegExpExecArray | null;
  while ((match = blockRe.exec(css)) !== null) {
    const block = match[1];
    const familyMatch = /font-family\s*:\s*['"]([^'"]+)['"]/i.exec(block);
    const srcMatch = /src\s*:\s*(url\([^)]+\))/i.exec(block);
    if (familyMatch != null && srcMatch != null) {
      faces.push({ family: familyMatch[1], src: srcMatch[1] });
    }
  }
  return faces;
}

/**
 * Pre-validate a TTF byte buffer's cmap table against the same
 * constraints Chromium's OTS sanitizer enforces. Returns
 * `{ok: false, reason}` when the font would be rejected by the
 * browser's font loader so the caller can skip handing it to
 * `FontFace.load()`.
 *
 * Why this is necessary even with the FontFace API: Chromium emits
 * `OTS parsing error: …` console warnings at the C++ level
 * (`WebFontLoader::ots_message_func`) regardless of whether the
 * loading path is CSS `@font-face` or `new FontFace().load()`. JS
 * has no hook to suppress these warnings — the only way to keep the
 * console clean is to reject the font before it ever reaches OTS.
 *
 * Validates format-4 subtables — the dominant subtable kind for
 * CJK fonts and the format `mtx-decompressor` produces with stale
 * `idRangeOffset` pointers / 0xFFFF glyph-id sentinels for some
 * Hangul faces:
 *  1. Each subtable's declared length stays inside the cmap table.
 *  2. Each non-zero `idRangeOffset` walks to an address still
 *     inside the subtable.
 *  3. Computed glyph IDs (via `idDelta + char` or `glyphIdArray`)
 *     never exceed `maxp.numGlyphs` — mirrors OTS `cmap.cc`
 *     ParseFormat4 "Range glyph reference too high".
 *
 * Other cmap formats (0/2/6/12/14) are not exhaustively validated —
 * we trust those when the table directory is structurally sound.
 *
 * This is bounded reactive maintenance: cmap is the dominant MTX
 * failure mode. If new fonts surface OTHER OTS table-level
 * rejections (head/maxp/glyf/loca/post/...) we'd add validators
 * for those alongside this one. The fallback chain (Google Fonts
 * bundle + metric-match catalog) handles every rejected face so
 * deck rendering is never blocked.
 */
function validateCmap(
  ttf: Uint8Array,
): { ok: true } | { ok: false; reason: string } {
  if (ttf.length < 12) return { ok: false, reason: "header too short" };
  const view = new DataView(ttf.buffer, ttf.byteOffset, ttf.byteLength);
  const numTables = view.getUint16(4);
  let cmapOff = -1;
  let cmapLen = 0;
  let maxpOff = -1;
  for (let i = 0; i < numTables; i++) {
    const recOff = 12 + i * 16;
    if (recOff + 16 > ttf.length)
      return { ok: false, reason: "table directory truncated" };
    const tag = String.fromCharCode(
      ttf[recOff],
      ttf[recOff + 1],
      ttf[recOff + 2],
      ttf[recOff + 3],
    );
    const off = view.getUint32(recOff + 8);
    const len = view.getUint32(recOff + 12);
    if (tag === "cmap") {
      cmapOff = off;
      cmapLen = len;
    } else if (tag === "maxp") {
      maxpOff = off;
    }
  }
  if (cmapOff < 0 || maxpOff < 0) {
    return { ok: true };
  }
  if (maxpOff + 6 > ttf.length) return { ok: false, reason: "maxp truncated" };
  const numGlyphs = view.getUint16(maxpOff + 4);
  if (cmapOff + cmapLen > ttf.length || cmapOff + 4 > ttf.length) {
    return { ok: false, reason: "cmap range out of bounds" };
  }
  const cmapEnd = cmapOff + cmapLen;
  const numSubtables = view.getUint16(cmapOff + 2);
  for (let i = 0; i < numSubtables; i++) {
    const recOff = cmapOff + 4 + i * 8;
    if (recOff + 8 > cmapEnd)
      return { ok: false, reason: "encoding record overflow" };
    const subOffFromCmap = view.getUint32(recOff + 4);
    const sub = cmapOff + subOffFromCmap;
    if (sub + 6 > cmapEnd)
      return { ok: false, reason: "subtable header overflow" };
    const fmt = view.getUint16(sub);
    if (fmt !== 4) continue;
    const length = view.getUint16(sub + 2);
    const subEnd = sub + length;
    if (subEnd > cmapEnd)
      return { ok: false, reason: "subtable length exceeds cmap" };
    const segCount = view.getUint16(sub + 6) >>> 1;
    const endCodesOff = sub + 14;
    const startCodesOff = endCodesOff + segCount * 2 + 2;
    const idDeltasOff = startCodesOff + segCount * 2;
    const idRangeOffsetsOff = idDeltasOff + segCount * 2;
    if (idRangeOffsetsOff + segCount * 2 > subEnd) {
      return { ok: false, reason: "format-4 segment arrays exceed subtable" };
    }
    for (let s = 0; s < segCount; s++) {
      const start = view.getUint16(startCodesOff + s * 2);
      const end = view.getUint16(endCodesOff + s * 2);
      const delta = view.getInt16(idDeltasOff + s * 2);
      const rangeOff = view.getUint16(idRangeOffsetsOff + s * 2);
      if (start > end) return { ok: false, reason: "segment start > end" };
      if (rangeOff === 0) {
        const span = end - start;
        const limit = Math.min(span, 0xffff) + 1;
        for (let j = 0; j < limit; j++) {
          const cp = (start + j) & 0xffff;
          const glyph = (cp + delta) & 0xffff;
          if (glyph >= numGlyphs) {
            return {
              ok: false,
              reason: `seg${s} cp U+${cp.toString(16)} → gid ${glyph} >= numGlyphs ${numGlyphs}`,
            };
          }
        }
      } else {
        const idRangeOffsetOffset = idRangeOffsetsOff + s * 2;
        for (let cp = start; cp <= end; cp++) {
          const rangeDelta = cp - start;
          const glyphIdOffset = idRangeOffsetOffset + rangeOff + rangeDelta * 2;
          if (glyphIdOffset + 1 >= subEnd) {
            return {
              ok: false,
              reason: `seg${s} cp U+${cp.toString(16)}: glyph_id_offset ${glyphIdOffset} past subtable end`,
            };
          }
          const glyphRaw = view.getUint16(glyphIdOffset);
          if (glyphRaw >= numGlyphs) {
            return {
              ok: false,
              reason: `seg${s} cp U+${cp.toString(16)} → raw gid ${glyphRaw} >= numGlyphs ${numGlyphs}`,
            };
          }
        }
      }
    }
  }
  return { ok: true };
}

/**
 * Decompress MicroType Express (MTX) compressed `<p:embeddedFont>`
 * payloads, register the result on the worker's own `FontFaceSet`
 * (so canvas measurement uses the right metrics), and return the
 * raw TTF bytes for each face the worker thread can transfer back
 * to the main thread.
 *
 * The Rust pipeline drops MTX-compressed faces because the
 * algorithm is a proprietary Agfa Monotype variant of LZ77 +
 * adaptive Huffman that we don't decode in Rust. The
 * `mtx-decompressor` npm package handles the algorithm in JS; the
 * decoded TTF bytes here come straight from that pipeline.
 *
 * We pre-filter via `validateCmap` because Chromium's OTS sanitizer
 * emits `OTS parsing error: …` console warnings at the C++ level
 * for any malformed table — there is no JS-side suppression
 * regardless of whether the load goes through CSS or the FontFace
 * API. Keeping malformed faces out of the loader entirely is the
 * only way to keep the console clean.
 */
async function decompressMtxFonts(
  entries: Array<{
    family: string;
    weight: string;
    style: string;
    payload: Uint8Array;
  }>,
): Promise<{
  /**
   * Decoded TTF bytes per face, ready to ship to the main thread for
   * registration via the FontFace API. The `bytes` field is a fresh
   * `Uint8Array` so its underlying buffer can be transferred without
   * detaching anything still in use by the worker (the worker also
   * keeps a separate FontFace instance referencing the original
   * decoded buffer via `FontFaceSet.add`).
   */
  decoded: Array<{
    family: string;
    weight: string;
    style: string;
    bytes: Uint8Array;
  }>;
  failures: Array<{ family: string; reason: string }>;
}> {
  if (entries.length === 0) {
    return { decoded: [], failures: [] };
  }
  // Lazy-import so non-MTX decks don't pay the parse cost.
  let decompressEotFont: (
    payload: Uint8Array,
    compressed: boolean,
    encrypted: boolean,
  ) => Uint8Array;
  try {
    const mod = await import("mtx-decompressor");
    decompressEotFont = mod.decompressEotFont;
  } catch (err) {
    // Library missing or failed to load — degrade gracefully.
    return {
      decoded: [],
      failures: entries.map((e) => ({
        family: e.family,
        reason: `mtx-decompressor unavailable: ${
          err instanceof Error ? err.message : String(err)
        }`,
      })),
    };
  }

  const decoded: Array<{
    family: string;
    weight: string;
    style: string;
    bytes: Uint8Array;
  }> = [];
  const failures: Array<{ family: string; reason: string }> = [];

  for (const entry of entries) {
    try {
      const ttf = decompressEotFont(entry.payload, true, false);
      // Sanity check: TTF/OTF magic must be present in the first four
      // bytes so we don't ship a corrupt face to the FontFaceSet.
      if (
        ttf.length < 4 ||
        !(
          (ttf[0] === 0x00 &&
            ttf[1] === 0x01 &&
            ttf[2] === 0x00 &&
            ttf[3] === 0x00) ||
          (ttf[0] === 0x4f &&
            ttf[1] === 0x54 &&
            ttf[2] === 0x54 &&
            ttf[3] === 0x4f)
        )
      ) {
        failures.push({
          family: entry.family,
          reason: "decompressed payload missing TTF/OTF magic",
        });
        continue;
      }
      // Pre-filter via the OTS-mirrored cmap walk. Skipping this
      // pre-check would let malformed faces reach Chromium's font
      // loader, which prints "OTS parsing error: …" warnings to the
      // console at the C++ level — uncatchable from JS regardless
      // of whether we use CSS @font-face or the FontFace API. The
      // dominant failure mode is MTX-decompressed Hangul fonts
      // whose glyphIdArray has 0xFFFF sentinels past `numGlyphs`.
      const cmapValid = validateCmap(ttf);
      if (!cmapValid.ok) {
        failures.push({
          family: entry.family,
          reason: `cmap validation failed: ${cmapValid.reason}`,
        });
        continue;
      }
      // Worker-side FontFaceSet registration drives canvas
      // measurement; main-thread document.fonts registration drives
      // the eventual paint and is wired up by PptxPresentation
      // after it receives the bytes via the `decodedFonts` channel
      // below. Both go through the FontFace API rather than CSS
      // `@font-face` data URIs — useful for non-OTS rejection
      // modes (subset / permission / network) which DO surface as
      // catchable promise rejections.
      try {
        const face = new FontFace(entry.family, ttf.buffer as ArrayBuffer, {
          weight: entry.weight,
          style: entry.style,
        });
        const loaded = await face.load();
        (self as unknown as { fonts: FontFaceSet }).fonts.add(loaded);
        // Clone the bytes for transfer so the worker's loaded
        // FontFace keeps an independent buffer (FontFace consumes
        // its constructor input internally and we don't want
        // detachment to surprise it).
        decoded.push({
          family: entry.family,
          weight: entry.weight,
          style: entry.style,
          bytes: new Uint8Array(ttf),
        });
      } catch (err) {
        failures.push({
          family: entry.family,
          reason: `FontFace.load failed: ${
            err instanceof Error ? err.message : String(err)
          }`,
        });
      }
    } catch (err) {
      failures.push({
        family: entry.family,
        reason: `mtx decompress failed: ${
          err instanceof Error ? err.message : String(err)
        }`,
      });
    }
  }

  return { decoded, failures };
}

async function handleOpen(
  id: number,
  bytes: Uint8Array,
  extraFontDefsCss?: string,
): Promise<void> {
  const core = await loadCore();
  doc?.free?.();
  doc = new core.PptxDocument(bytes, [], true);
  const slideCount = doc.slideCount();
  let fontDefs = doc.fontDefs();

  // Await every embedded @font-face so measureText returns accurate
  // ascent/descent when renderSlide calls __slideglanceMeasureLineMetrics.
  // We also collect a per-face load report so the main thread can
  // surface which embedded faces failed (typically MicroType Express
  // payloads our pipeline can't decode, but also subset / permission
  // issues). The report is a strict diagnostic — empty in the steady
  // state (every face loaded), populated only when something failed.
  //
  // Host-supplied `extraFontDefsCss` covers the gap when a deck names a
  // font (e.g. `Anton`) that the embedded payload couldn't supply
  // (MTX-compressed → dropped). The chrome-extension preloads ~40
  // popular Google Fonts there so the canvas measurer's metrics line
  // up with the fonts the browser will actually paint with.
  // MTX-compressed `<p:embeddedFont>` payloads — the Rust pipeline
  // can't decompress them (Agfa Monotype LZ77 + adaptive Huffman) so
  // we fish them out of the new wasm API and decode in JS via
  // `mtx-decompressor`. Decoded TTF bytes are registered on the
  // worker's own FontFaceSet (for canvas measurement) and shipped
  // to the main thread via the `decodedFonts` message field, where
  // PptxPresentation registers them on document.fonts via the
  // FontFace API — bypassing the CSS @font-face data-URI path that
  // makes Chromium's CSS engine eagerly run OTS validation and emit
  // "Failed to decode downloaded font" warnings to the console
  // whenever any of OTS's many table-level checks fail.
  const mtxEntries = doc.mtxCompressedFonts?.() ?? [];
  const mtxResult = await decompressMtxFonts(mtxEntries);

  const deckFaces = parseFontFacesFromCss(fontDefs);
  const extraFaces = extraFontDefsCss
    ? parseFontFacesFromCss(extraFontDefsCss)
    : [];
  const faces = [...deckFaces, ...extraFaces];
  const fontLoadFailures: Array<{ family: string; reason: string }> = [
    ...mtxResult.failures,
  ];
  await Promise.all(
    faces.map(async ({ family, src }) => {
      try {
        const face = new FontFace(family, src);
        const loaded = await face.load();
        (self as unknown as { fonts: FontFaceSet }).fonts.add(loaded);
      } catch (err) {
        const reason =
          err instanceof Error ? `${err.name}: ${err.message}` : String(err);
        fontLoadFailures.push({ family, reason });
      }
    }),
  );
  // Intentionally no console warning here. Embedded-font load
  // failures are not a hard error: the metric-match fallback chain
  // and bundled Google Fonts cover the visible paint. Hosts that
  // want to surface failure detail (e.g. a status-bar indicator)
  // can read the structured `fontLoadFailures` array shipped in the
  // `opened` message. Logging here unconditionally would alarm
  // users on every deck that includes an MTX-compressed face we
  // can't decode — a known and bounded class of failures.

  // Pull the per-typeface usage report so the main-thread UI can show
  // which authored typefaces resolve to which actually-installed fonts
  // (status-bar font-mapping indicator). The report is just a list of
  // `{ requested, fallback_chain, resolved_family }` entries — small
  // even for decks that reference many distinct typefaces.
  let fontUsage: Array<{
    requested: string;
    fallback_chain: string[];
    resolved_family: string | null;
  }>;
  try {
    fontUsage = doc.fontUsage();
  } catch {
    // Older WASM builds without `fontUsage` — fall through with empty list.
    fontUsage = [];
  }

  // Transfer the MTX-decoded TTF buffers to the main thread so it
  // can register them on `document.fonts` via the FontFace API
  // (bypassing CSS @font-face's eager OTS validation). `Uint8Array`
  // doesn't survive a structured clone of the message itself —
  // postMessage clones the typed array but sees the underlying
  // buffer as already detached if we used Transferable. Send each
  // entry's `bytes.buffer` in the transfer list and keep the
  // typed-array view alongside.
  const decodedFonts = mtxResult.decoded.map((d) => ({
    family: d.family,
    weight: d.weight,
    style: d.style,
    bytes: d.bytes,
  }));
  const decodedTransfer = decodedFonts.map(
    (d) => d.bytes.buffer as ArrayBuffer,
  );

  postMessage(
    {
      type: "opened",
      id,
      slideCount,
      fontDefs,
      fontUsage,
      fontLoadFailures,
      decodedFonts,
    },
    decodedTransfer,
  );
}

function handleRender(id: number, slide: number): void {
  if (doc === null) {
    postMessage({ type: "error", id, message: "no document open" });
    return;
  }
  // externalMedia = true, includeFontDefs = false: the main thread
  // mounts the deck-wide `<style>` block once via fontDefs(), and
  // every base64 `data:` URI is replaced with `pptx-media://{hash}`
  // so the SVG payload stays small.
  const result = doc.renderSlide(slide, true, false) as RenderResult | null;
  if (result === null) {
    postMessage({ type: "error", id, message: `slide ${slide} not found` });
    return;
  }
  // serde-wasm-bindgen surfaces the media field as a JS Map<string,{mime,bytes}>.
  // Walk it so we can build a transfer list of every `bytes` ArrayBuffer.
  const transfer: ArrayBuffer[] = [];
  const media = new Map<string, { mime: string; bytes: Uint8Array }>();
  for (const [key, value] of result.media as unknown as Iterable<
    [string, { mime: string; bytes: Uint8Array }]
  >) {
    media.set(key, value);
    transfer.push(value.bytes.buffer as ArrayBuffer);
  }
  postMessage(
    {
      type: "rendered",
      id,
      slide: result.slide_number,
      svg: result.svg,
      media,
      notes: result.notes,
      layoutName: result.layout_name,
      sectionName: result.section_name,
    },
    transfer,
  );
}

function handleClose(id: number): void {
  doc?.free?.();
  doc = null;
  postMessage({ type: "closed", id });
}

self.addEventListener("message", (ev: MessageEvent) => {
  const msg = ev.data as
    | {
        type: "open";
        id: number;
        bytes: Uint8Array;
        extraFontDefsCss?: string;
      }
    | { type: "render"; id: number; slide: number }
    | { type: "close"; id: number };
  switch (msg.type) {
    case "open":
      handleOpen(msg.id, msg.bytes, msg.extraFontDefsCss).catch((err) => {
        postMessage({
          type: "error",
          id: msg.id,
          message: String(err?.message ?? err),
        });
      });
      break;
    case "render":
      try {
        handleRender(msg.id, msg.slide);
      } catch (err) {
        postMessage({
          type: "error",
          id: msg.id,
          message: String((err as Error)?.message ?? err),
        });
      }
      break;
    case "close":
      handleClose(msg.id);
      break;
    default:
      break;
  }
});

// Re-declare postMessage with a type that matches our usage; the DOM
// lib's `postMessage` overloads are fine but TS sometimes resolves the
// wrong one inside `lib.dom.d.ts` when targeting workers without
// `lib: ["webworker"]`. The cast keeps it lib-agnostic.
declare function postMessage(message: unknown, transfer?: Transferable[]): void;
