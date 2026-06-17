/**
 * `useSlideCache` — slide rendering + caching pipeline.
 *
 * Owns:
 * - The `slideCache: Map<slide#, CachedSlide>` plus its setter (the
 *   keyboard / print / pdf / sectionNav callers all need read
 *   access; the setter is exposed so the host's deck-loader can
 *   flush it on a deck swap).
 * - `pendingRef` — in-flight `requestSlide` tasks deduplicated by
 *   slide number so a rapid keyboard repeat or sidebar hover doesn't
 *   fire N IPC roundtrips for the same slide.
 * - `requestSlide(slide)` — cache-or-fetch path. Inlines media as
 *   `data:` URLs (not `blob:`) so the SVG is portable and never
 *   revokes underneath print / PDF.
 * - `ensureAllSlidesRendered(silent, onProgress)` — force-render
 *   every slide. Used by Print / PDF / Search and by the background
 *   prefetch.
 *
 * Plus two effects:
 * - **Background prefetch** — once a deck is open, walk every slide
 *   in the background so deck-wide actions stop showing partial
 *   output. Skipped when the host sets `noPrefetch`.
 * - **Active slide fetch** — request the visible slide when
 *   `currentSlide` changes.
 *
 * Stale-resolution handling: every task captures `deckEpochRef`'s
 * current value when it starts; if the epoch advances mid-flight
 * (deck swap), the resolution is dropped before it stamps the new
 * deck's cache. Without this, a deck swap during a slow render
 * leaves the *previous* deck's SVG in the new cache slot.
 */

import { useCallback, useEffect, useState } from "react";
import type { MutableRefObject } from "react";

import { inlineMediaAsDataUrls } from "../ui/media-inline.js";
import { prepareSvg } from "../svg-utils.js";
import { t } from "../ui/i18n.js";
import type {
  RenderedSlide,
  SlideController,
  SlideMeta,
  SlideSvg,
} from "../types.js";
import type { CachedSlide } from "./types.js";

export interface UseSlideCacheArgs {
  controller: SlideController | null;
  slideCount: number;
  currentSlide: number;
  /** Host opt-out for the background prefetch. */
  noPrefetch?: boolean;
  /** Optional async (or sync) hook the host installs to merge in
   *  extra per-slide metadata (Tauri shell uses this to override
   *  layout / section names from a sidecar JSON). */
  resolveMeta?: (
    slide: number,
  ) =>
    | Promise<Partial<SlideMeta> | null | undefined>
    | Partial<SlideMeta>
    | null;
  /** All-slides-rendered gate — flipped on once `ensureAllSlidesRendered`
   *  delivers `slideCount` entries. */
  allSlidesReady: boolean;
  setAllSlidesReady: (next: boolean) => void;

  setErrorMsg: (msg: string | null) => void;
  setPhase: (phase: string) => void;

  /** Monotonic deck-epoch counter. The host owns this and bumps it
   *  on every deck swap so in-flight render tasks know to drop
   *  their result instead of stamping it into the new deck's
   *  cache. */
  deckEpochRef: MutableRefObject<number>;
  /** In-flight task map. The host owns the ref so the deck-loader
   *  effect can clear it during teardown. */
  pendingRef: MutableRefObject<Map<number, Promise<CachedSlide | null>>>;
}

export interface UseSlideCacheResult {
  slideCache: Map<number, CachedSlide>;
  setSlideCache: (
    next:
      | Map<number, CachedSlide>
      | ((prev: Map<number, CachedSlide>) => Map<number, CachedSlide>),
  ) => void;
  requestSlide: (slide: number) => Promise<CachedSlide | null>;
  ensureAllSlidesRendered: (
    silent?: boolean,
    onProgress?: (current: number, total: number) => void,
  ) => Promise<SlideSvg[]>;
}

export function useSlideCache(args: UseSlideCacheArgs): UseSlideCacheResult {
  const {
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
  } = args;

  const [slideCache, setSlideCache] = useState<Map<number, CachedSlide>>(
    () => new Map(),
  );

  const requestSlide = useCallback(
    async (slide: number): Promise<CachedSlide | null> => {
      if (!controller || slide < 1) return null;
      // Snapshot the current deck epoch so we can detect a deck
      // swap that lands while this task is in flight. Any IPC
      // roundtrip older than the current epoch is dropped before
      // it stamps the new deck's cache.
      const myEpoch = deckEpochRef.current;
      let cached: CachedSlide | undefined;
      setSlideCache((prev) => {
        cached = prev.get(slide);
        return prev;
      });
      if (cached) return cached;
      const inflight = pendingRef.current.get(slide);
      if (inflight) return inflight;
      const task = (async () => {
        try {
          const rendered: RenderedSlide = await controller.renderSlide(slide);
          if (myEpoch !== deckEpochRef.current) return null; // stale
          // Inline media as `data:` URLs rather than `blob:` URLs.
          //
          // `blob:` URLs are document-scoped and revoked on page
          // teardown — they also race catastrophically with React
          // 18 StrictMode and Vite HMR (the component double-
          // mounts; first-mount cleanup revokes URLs the second
          // mount has already stamped into cache, surfacing as
          // `net::ERR_FILE_NOT_FOUND` on every grid-view toggle).
          // `data:` URLs are self-contained, never revoke, and
          // copy intact when the SVG is exported to print / PDF.
          // The base64 overhead is acceptable: identical media
          // hashed to the same key only encodes once per slide and
          // the worker already deduplicates across the deck.
          const result =
            rendered.media && rendered.media.size > 0
              ? {
                  svg: inlineMediaAsDataUrls(rendered.svg, rendered.media),
                  blobUrls: [] as string[],
                }
              : { svg: rendered.svg, blobUrls: [] as string[] };
          const inlineMeta: SlideMeta = {
            notes: rendered.notes,
            layout_name: rendered.layoutName,
            section_name: rendered.sectionName,
          };
          let mergedMeta = inlineMeta;
          if (resolveMeta) {
            try {
              const extra = await resolveMeta(slide);
              if (extra) mergedMeta = { ...inlineMeta, ...extra };
            } catch {
              /* fall through */
            }
          }
          if (myEpoch !== deckEpochRef.current) return null; // stale post-meta
          const entry: CachedSlide = {
            svg: result.svg,
            preparedSvg: prepareSvg(result.svg),
            blobUrls: result.blobUrls,
            meta: mergedMeta,
          };
          setSlideCache((prev) => {
            // Final defensive check inside the setState — if a
            // deck swap happened between the prior epoch read and
            // this commit, leave the (already-flushed) cache
            // alone.
            if (myEpoch !== deckEpochRef.current) return prev;
            const existing = prev.get(slide);
            if (existing) {
              for (const u of entry.blobUrls) URL.revokeObjectURL(u);
              return prev;
            }
            const next = new Map(prev);
            next.set(slide, entry);
            return next;
          });
          return entry;
        } catch (err) {
          setErrorMsg(`${(err as Error).message ?? err}`);
          return null;
        } finally {
          pendingRef.current.delete(slide);
        }
      })();
      pendingRef.current.set(slide, task);
      return task;
    },
    [controller, resolveMeta, deckEpochRef, pendingRef, setErrorMsg],
  );

  const ensureAllSlidesRendered = useCallback(
    async (
      silent = false,
      onProgress?: (current: number, total: number) => void,
    ): Promise<SlideSvg[]> => {
      if (!controller || slideCount === 0) return [];
      const out: SlideSvg[] = [];
      for (let n = 1; n <= slideCount; n += 1) {
        if (!silent) {
          setPhase(
            t("phase.preparingSlideOf", { current: n, total: slideCount }),
          );
        }
        onProgress?.(n, slideCount);
        const cached = await requestSlide(n);
        if (!cached) continue;
        out.push({
          slide_number: n,
          svg: cached.svg,
          notes: cached.meta.notes ?? undefined,
          layout_name: cached.meta.layout_name ?? undefined,
          section_name: cached.meta.section_name ?? undefined,
        });
      }
      if (!silent) setPhase("");
      if (out.length === slideCount) setAllSlidesReady(true);
      return out;
    },
    [controller, slideCount, requestSlide, setPhase, setAllSlidesReady],
  );

  // Background prefetch — once a deck is loaded, walk every slide
  // in the background so deck-wide actions (Print / PDF / Slideshow
  // / Search) become available without an interactive stall.
  // Failures here are swallowed: prefetch is a UX accelerator, not
  // a correctness requirement. Cancellation is handled implicitly
  // by letting `slideCount` change reset the cache and start a new
  // pass.
  useEffect(() => {
    if (noPrefetch) return;
    if (!controller || slideCount === 0) return;
    if (allSlidesReady) return;
    let cancelled = false;
    void (async () => {
      try {
        await ensureAllSlidesRendered(true);
      } catch {
        /* ignore background prefetch errors */
      }
      if (cancelled) return;
    })();
    return () => {
      cancelled = true;
    };
  }, [
    noPrefetch,
    controller,
    slideCount,
    allSlidesReady,
    ensureAllSlidesRendered,
  ]);

  // Active slide fetch.
  //
  // Including `slideCache` in the dependency list lets external
  // cache invalidation (e.g. the deck-loader's surgical
  // `invalidatedSlides` path on an in-place edit-cycle update) drive
  // a refetch of the visible slide without the user having to
  // navigate. `requestSlide` is a no-op cache hit when the entry is
  // still present, so additions to the cache (caused by prior calls
  // to this very effect) don't loop — they short-circuit on the
  // hot-cache check inside `requestSlide`.
  useEffect(() => {
    if (!controller || slideCount === 0) return;
    void requestSlide(currentSlide);
  }, [controller, slideCount, currentSlide, requestSlide, slideCache]);

  return { slideCache, setSlideCache, requestSlide, ensureAllSlidesRendered };
}
