/**
 * `useDeckLoader` — auto-open a `.pptx` source on mount and feed
 * everything the controller hands back into the host's slide-state
 * slots.
 *
 * Steps:
 * 1. Resolve `src` to bytes — `string` is fetched, `ArrayBuffer` is
 *    wrapped, raw `Uint8Array` is used as-is.
 * 2. Hand the bytes to `controller.open()` with the host-supplied
 *    `bundledFontDefsCss` (chrome-extension uses this for its
 *    bundled Google Fonts so the worker's canvas measurer sees the
 *    same metrics the eventual browser paint will).
 * 3. Mirror the controller's metadata into host state (slide count,
 *    typeface usage, font-defs CSS).
 * 4. Register every MTX-decoded TTF buffer on `document.fonts` via
 *    the FontFace API. The worker pre-filtered these through
 *    `validateCmap` so OTS rejection is the rare exception, but
 *    silent-catch the promise rejection regardless — fallback fonts
 *    cover the visible paint.
 * 5. Reset slide / zoom / pan state to the new deck's defaults and
 *    revoke any blob URLs lingering in the previous deck's cache.
 *
 * Skipped entirely when the host opted into manual control by
 * passing `slideCount` (`externalSlideCount != null`) — that mode
 * means the embedder owns deck loading and the viewer just renders.
 */

import { useEffect } from "react";

import { extractFontStyleCss } from "../svg-utils.js";
import type { SlideController, TypefaceUsage } from "../types.js";
import type { CachedSlide } from "./types.js";

export interface UseDeckLoaderArgs {
  controller: SlideController | null;
  src?: Uint8Array | ArrayBuffer | string | null;
  externalSlideCount?: number;
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
   * The slide cache treatment depends on whether
   * `invalidatedSlides` is also supplied — see that prop.
   */
  incrementalUpdate?: boolean;
  /**
   * 1-based indices of slides whose cache entries should be flushed
   * on the next `src` change. Only consulted when
   * `incrementalUpdate` is `true`; the default-off path
   * (full-deck-open) always flushes the entire cache.
   *
   *  - `undefined`: no per-slide hint, fall back to flushing the
   *    entire cache. Safest default — guarantees the visible slide
   *    matches the new bytes even if the host has not computed a
   *    diff.
   *  - `[]`: empty list — host is asserting nothing changed
   *    visually, keep the cache intact. The worker still re-parses
   *    so subsequent navigation reflects the new bytes; existing
   *    cached SVGs (which the host knows are bit-identical to what
   *    the new parse would produce) survive.
   *  - `[3, 5, …]`: only invalidate cache entries for the listed
   *    1-based slide indices. Hosts compute this list by hashing
   *    each slide's source between edits.
   */
  invalidatedSlides?: number[];

  setPhase: (phase: string) => void;
  setSlideCount: (next: number) => void;
  setFontUsage: (next: TypefaceUsage[]) => void;
  setFontDefsCss: (next: string) => void;
  setCurrentSlide: (next: number) => void;
  setZoom: (next: number) => void;
  setPanX: (next: number) => void;
  setPanY: (next: number) => void;
  setErrorMsg: (next: string | null) => void;
  setSlideCache: (
    next:
      | Map<number, CachedSlide>
      | ((prev: Map<number, CachedSlide>) => Map<number, CachedSlide>),
  ) => void;
}

export function useDeckLoader(args: UseDeckLoaderArgs): void {
  const {
    controller,
    src,
    externalSlideCount,
    bundledFontDefsCss,
    incrementalUpdate,
    invalidatedSlides,
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
  } = args;

  useEffect(() => {
    if (!controller || src == null || externalSlideCount != null) return;
    let cancelled = false;
    void (async () => {
      try {
        setPhase("loading");
        let bytes: Uint8Array;
        if (typeof src === "string") {
          const res = await fetch(src);
          if (!res.ok) throw new Error(`fetch ${src} → ${res.status}`);
          bytes = new Uint8Array(await res.arrayBuffer());
        } else if (src instanceof Uint8Array) {
          bytes = src;
        } else {
          bytes = new Uint8Array(src);
        }
        const meta = await controller.open(bytes, {
          extraFontDefsCss: bundledFontDefsCss,
        });
        if (cancelled) return;
        setSlideCount(meta.slideCount);
        setFontUsage(meta.fontUsage ?? []);
        setFontDefsCss(extractFontStyleCss(meta.fontDefs ?? ""));
        // Register MTX-decoded TTF buffers on `document.fonts` via
        // the FontFace API. Worker already decoded the bytes and
        // pre-filtered them through `validateCmap` (the OTS cmap
        // checks Chromium prints uncatchable C++-level warnings
        // for); the bytes that reach us here are expected to load
        // cleanly. Non-OTS failure modes (e.g. CORS / detachment
        // races) DO surface as catchable promise rejections, which
        // we silence below since the metric-match fallback chain
        // and bundled Google Fonts carry the visible paint.
        if (typeof document !== "undefined" && document.fonts) {
          for (const d of meta.decodedFonts ?? []) {
            try {
              const face = new FontFace(
                d.family,
                d.bytes.buffer as ArrayBuffer,
                {
                  weight: d.weight,
                  style: d.style,
                },
              );
              face
                .load()
                .then((loaded) => {
                  document.fonts.add(loaded);
                })
                .catch(() => {
                  // OTS rejection or other browser-side font decode
                  // failure — silently skip. The deck still renders
                  // via the Phase 2 bundled Google Fonts fallback or
                  // the metric-match catalog substitute.
                });
            } catch {
              // FontFace constructor itself only throws for invalid
              // descriptor strings — also silenced.
            }
          }
        }
        // Resetting slide index / zoom / pan is the right default for
        // a fresh deck open, but a live editing surface (pom VS Code
        // preview etc.) re-feeds the same deck on every keystroke and
        // wants the user's scroll position preserved across edits.
        // `incrementalUpdate` opts those hosts out of the reset.
        if (!incrementalUpdate) {
          setCurrentSlide(1);
          setZoom(1);
          setPanX(0);
          setPanY(0);
        }
        setErrorMsg(null);
        setPhase("");

        // Cache invalidation policy:
        //   - default-off (`!incrementalUpdate`): flush every entry,
        //     revoking media blob URLs. The host changed decks
        //     wholesale.
        //   - on + `invalidatedSlides=[]`: nothing changed visually,
        //     keep the entire cache.
        //   - on + `invalidatedSlides=[i, …]`: drop only those
        //     entries. The active-slide effect in `useSlideCache`
        //     re-renders the visible slide if it appears in the
        //     list; navigating to other invalidated slides triggers
        //     their fetch on demand.
        //   - on + `invalidatedSlides=undefined`: host did not
        //     supply a diff. Fall back to flushing all so the
        //     visible slide is guaranteed fresh.
        if (incrementalUpdate && invalidatedSlides !== undefined) {
          if (invalidatedSlides.length > 0) {
            const dropSet = new Set(invalidatedSlides);
            setSlideCache((prev) => {
              const next = new Map(prev);
              for (const idx of dropSet) {
                const entry = next.get(idx);
                if (!entry) continue;
                for (const u of entry.blobUrls) URL.revokeObjectURL(u);
                next.delete(idx);
              }
              return next;
            });
          }
          // empty list: leave cache intact.
        } else {
          setSlideCache((prev) => {
            for (const c of prev.values()) {
              for (const u of c.blobUrls) URL.revokeObjectURL(u);
            }
            return new Map();
          });
        }
      } catch (err) {
        if (!cancelled) {
          setErrorMsg(`${(err as Error).message ?? err}`);
          setPhase("");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [controller, src, externalSlideCount]);
}
