/**
 * `useSlideDomMount` — imperatively mount the active slide's SVG
 * into the slide-host `<div>` and rebuild the bbox cache used by
 * the selection state machine.
 *
 * Why imperative: React's reconciler can't safely round-trip an SVG
 * through string → DOM tree → React vnodes without breaking the
 * `getBoundingClientRect()` invariants the selection layer relies
 * on. We use `DOMParser` + `importNode` so the mounted SVG carries
 * the real layout state the browser computed, and we re-namespace
 * every `id="…"` so the main stage's SVG can't collide with the
 * sibling thumbnails the sidebar mounts at the same time.
 *
 * The bbox cache is built from `getBoundingClientRect()` →
 * `inverse(getScreenCTM())` rather than `getBBox()`. Group bboxes
 * are unreliable for `<text>` runs whose width depends on font
 * substitution; the rendered-geometry path always matches what the
 * user sees on screen.
 */

import { useEffect, useRef } from "react";
import type { MutableRefObject } from "react";

import { uniquifyIds } from "../svg-utils.js";

export interface UseSlideDomMountArgs {
  slideSvg: string;
  currentSlide: number;
  slideshow: boolean;
  /** Stage host for the normal (non-slideshow) view. */
  slideRef: MutableRefObject<HTMLElement | null>;
  /** Stage host for the slideshow overlay. */
  slideshowSlideRef: MutableRefObject<HTMLElement | null>;
  /** Cache of per-shape user-space bboxes; rebuilt every mount. */
  bboxMapRef: MutableRefObject<
    Map<string, { x: number; y: number; w: number; h: number }>
  >;
  /** Latched `onReady` flag — flipped on the first successful mount
   *  of a deck so the host can dismiss its loading overlay. */
  onReadyFiredRef: MutableRefObject<boolean>;
  onReady?: () => void;
  setErrorMsg: (msg: string | null) => void;

  /** Layout signals that don't read inside the effect but require a
   *  re-mount when the host's frame around the slide changes
   *  (sidebar resize, notes toggle, view-mode swap, stage resize). */
  sidebarWidth: number;
  notesOpen: boolean;
  viewMode: "normal" | "grid";
  stageW: number;
  stageH: number;
}

export function useSlideDomMount(args: UseSlideDomMountArgs): void {
  const {
    slideSvg,
    currentSlide,
    slideshow,
    slideRef,
    slideshowSlideRef,
    bboxMapRef,
    onReadyFiredRef,
    onReady,
    setErrorMsg,
  } = args;

  // Track the previously rendered slide index so we can distinguish
  // "user navigated to a not-yet-fetched slide" (where a brief
  // loading-blank is the correct behaviour) from "the cache for the
  // currently-visible slide was invalidated by an in-place edit-
  // cycle update" (where keeping the stale SVG in the DOM until the
  // refreshed one arrives prevents a perceptible white flash on
  // every keystroke).
  const prevSlideRef = useRef<number>(0);

  useEffect(() => {
    // Mount the SVG into whichever slide host is currently visible.
    // Slideshow mode uses its own overlay-mounted div with a
    // separate ref so the slide doesn't get torn out when
    // fullscreen flips.
    const host = slideshow ? slideshowSlideRef.current : slideRef.current;
    if (!host) return;
    if (!slideSvg) {
      const sameSlide = prevSlideRef.current === currentSlide;
      const hasContent = host.firstChild !== null;
      // Only clear when we're transitioning to a different slide (or
      // the host is already empty). Clearing on a same-slide cache
      // invalidation strips the SVG out of the DOM until the worker
      // re-renders it — every edit produces a one-frame blank
      // through the slide stage, which the user reads as a flicker.
      if (sameSlide && hasContent) return;
      while (host.firstChild) host.removeChild(host.firstChild);
      return;
    }
    const existing = host.firstElementChild;
    if (
      existing &&
      existing.tagName.toLowerCase() === "svg" &&
      host.dataset.slideSvgKey === slideSvg
    ) {
      return; // already mounted, no work
    }
    try {
      // Rewrite every `id="…"` / `url(#…)` reference to a
      // mount-unique namespace so the main-stage SVG can never
      // collide with the sibling SVGs the thumbnail strip mounts
      // simultaneously.
      const namespaced = uniquifyIds(slideSvg, `stage-s${currentSlide}`);
      const doc = new DOMParser().parseFromString(namespaced, "image/svg+xml");
      const root = doc.documentElement;
      if (!root) return;
      const errNode = root.querySelector("parsererror");
      if (errNode) {
        setErrorMsg(errNode.textContent ?? "svg parse error");
        return;
      }
      // Atomic swap: build the new node off-DOM first, then replace
      // the previous content in a single mutation. The browser never
      // observes an empty host between frames, which would otherwise
      // surface as a sub-frame flash even though the JS work
      // completes within one event-loop turn.
      const incoming = document.importNode(root, true);
      host.replaceChildren(incoming);
      host.dataset.slideSvgKey = slideSvg;
      prevSlideRef.current = currentSlide;
      // First successful mount of this deck — fire `onReady` once
      // so host-level loading overlays can dismiss right when the
      // user can actually see content. Subsequent slide changes
      // re-enter this branch but the ref guard short-circuits.
      if (!onReadyFiredRef.current) {
        onReadyFiredRef.current = true;
        try {
          onReady?.();
        } catch {
          // Host-supplied callback errors must never derail the SVG
          // mount path — the slide is already in the DOM at this
          // point and a thrown onReady would leak through to the
          // try/catch below and surface as a parser error to the
          // user.
        }
      }
      // Rebuild bbox map for hit-testing. We use
      // `getBoundingClientRect()` (the browser-truth painted
      // bounds) and project back into SVG user space via the
      // inverse of `getScreenCTM()` so the stored rect always
      // matches what the user actually sees.
      //
      // Why not `getBBox()` + `getCTM()`?
      //   `<g>.getBBox()` is unreliable for groups whose children
      //   are `<text>` runs whose width depends on font
      //   substitution: with web-font fallback in flight the
      //   geometric bbox can clip the visible glyph row, leaving
      //   the selection rectangle drawn too small / shifted
      //   relative to the glyph the user can see (a common
      //   artefact on table cells whose `<g>` wraps text laid out
      //   at the original metric-match width).
      //   `getBoundingClientRect()` reflects the rendered geometry
      //   after font load and after every transform on the chain,
      //   so it's the authoritative source for "where the shape is
      //   on screen" — converting through `inverseScreenCTM` then
      //   gives a user-space rect that the existing zoom-aware
      //   projection step in `selectionBoxes` can transform back
      //   to screen space.
      const map = new Map<
        string,
        { x: number; y: number; w: number; h: number }
      >();
      const svgEl = host.firstElementChild as SVGSVGElement | null;
      const screenCTM = svgEl?.getScreenCTM?.() ?? null;
      if (
        svgEl &&
        screenCTM &&
        typeof (svgEl as unknown as { createSVGPoint?: () => unknown })
          .createSVGPoint === "function"
      ) {
        let inverse: DOMMatrix | null = null;
        try {
          inverse = screenCTM.inverse();
        } catch {
          inverse = null;
        }
        if (inverse) {
          const els =
            svgEl.querySelectorAll<SVGGraphicsElement>("[data-sp-id]");
          for (const el of Array.from(els)) {
            const id = (el as unknown as HTMLElement).dataset.spId;
            if (!id) continue;
            const rect = el.getBoundingClientRect();
            // Skip elements that haven't laid out yet (e.g. inside
            // `display:none` ancestors or detached subtrees) — their
            // 0×0 rect would otherwise project to a 0-area selection
            // box anchored at the SVG origin.
            if (rect.width === 0 && rect.height === 0) continue;
            const p = svgEl.createSVGPoint();
            const corners = [
              [rect.left, rect.top],
              [rect.right, rect.top],
              [rect.left, rect.bottom],
              [rect.right, rect.bottom],
            ].map(([x, y]) => {
              p.x = x;
              p.y = y;
              return p.matrixTransform(inverse!);
            });
            const xs = corners.map((c) => c.x);
            const ys = corners.map((c) => c.y);
            const minX = Math.min(...xs);
            const maxX = Math.max(...xs);
            const minY = Math.min(...ys);
            const maxY = Math.max(...ys);
            map.set(id, { x: minX, y: minY, w: maxX - minX, h: maxY - minY });
          }
        }
      }
      bboxMapRef.current = map;
    } catch (err) {
      setErrorMsg(`${(err as Error).message ?? err}`);
    }
    // Layout-signal deps don't appear inside the effect body but a
    // change in any of them moves the slide on screen, which
    // invalidates the cached client rects below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    slideSvg,
    currentSlide,
    args.sidebarWidth,
    args.notesOpen,
    args.viewMode,
    slideshow,
    args.stageW,
    args.stageH,
  ]);
}
