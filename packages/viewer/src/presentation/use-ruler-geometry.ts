/**
 * `useRulerGeometry` — derive the slide's live position + size in
 * stage-relative pixel coordinates so the [`Ruler`](../ui/Ruler.tsx)
 * component can paint ticks that always align with the slide
 * centre, regardless of zoom, pan, sidebar resize, or stage scroll.
 *
 * Mirrors the historic Lit shell, which tracked `slideRect()` via
 * a ResizeObserver on the viewer-wrap. The hook owns:
 * - the `intrinsicViewBox` derivation from the deck SVG's `viewBox`,
 * - the live `rulerRect` state with a 0.5-pixel diff threshold so
 *   sub-pixel jitter from CSS layout doesn't trigger a re-render,
 * - the ResizeObserver + scroll listener wiring against the stage
 *   and slide DOM nodes.
 */

import { useEffect, useMemo, useState } from "react";
import type { MutableRefObject } from "react";

export interface RulerRect {
  originX: number;
  originY: number;
  extentX: number;
  extentY: number;
}

export interface UseRulerGeometryArgs {
  /** Whether the ruler is currently mounted. The hook short-circuits
   *  ResizeObserver / scroll listeners when off. */
  rulerOn: boolean;
  /** Deck SVG string — used to extract the intrinsic viewBox. */
  slideSvg: string;
  /** Stage scrollable container. */
  stageRef: MutableRefObject<HTMLElement | null>;
  /** Slide host element (the `<svg>` parent). */
  slideRef: MutableRefObject<HTMLElement | null>;
  /** Tracked layout dependencies — we don't read them, but the
   *  effect must re-measure when they change. */
  slideW: number;
  slideH: number;
  panX: number;
  panY: number;
  stageW: number;
  stageH: number;
}

export interface UseRulerGeometryResult {
  /** Intrinsic deck size in pixels and centimetres (X axis). */
  intrinsic: { px: number; cm: number };
  /** Intrinsic deck size in pixels and centimetres (Y axis). */
  intrinsicY: { px: number; cm: number };
  /** Live slide rect, in stage-relative pixel coords. */
  rulerRect: RulerRect;
}

export function useRulerGeometry(
  args: UseRulerGeometryArgs,
): UseRulerGeometryResult {
  const { rulerOn, slideSvg, stageRef, slideRef } = args;

  const intrinsicViewBox = useMemo(() => {
    const m = slideSvg.match(/viewBox=["']([^"']+)["']/);
    if (!m) return { w: 0, h: 0 };
    const parts = m[1].split(/\s+/).map(Number);
    if (parts.length < 4) return { w: 0, h: 0 };
    return { w: parts[2], h: parts[3] };
  }, [slideSvg]);
  const intrinsic = {
    px: intrinsicViewBox.w,
    cm: (intrinsicViewBox.w * 2.54) / 96,
  };
  const intrinsicY = {
    px: intrinsicViewBox.h,
    cm: (intrinsicViewBox.h * 2.54) / 96,
  };

  // Live slide rect, recomputed whenever zoom / pan / stageSize / aspect
  // changes — useState so a render bump propagates to the Ruler.
  const [rulerRect, setRulerRect] = useState<RulerRect>({
    originX: 0,
    originY: 0,
    extentX: 0,
    extentY: 0,
  });
  useEffect(() => {
    if (!rulerOn) return;
    const stage = stageRef.current;
    const slide = slideRef.current;
    if (!stage || !slide) return;
    const measure = (): void => {
      const stageRect = stage.getBoundingClientRect();
      const slideR = slide.getBoundingClientRect();
      if (slideR.width <= 0 || slideR.height <= 0) return;
      setRulerRect((prev) => {
        const next: RulerRect = {
          originX: slideR.left - stageRect.left,
          originY: slideR.top - stageRect.top,
          extentX: slideR.width,
          extentY: slideR.height,
        };
        if (
          Math.abs(prev.originX - next.originX) < 0.5 &&
          Math.abs(prev.originY - next.originY) < 0.5 &&
          Math.abs(prev.extentX - next.extentX) < 0.5 &&
          Math.abs(prev.extentY - next.extentY) < 0.5
        ) {
          return prev;
        }
        return next;
      });
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(stage);
    ro.observe(slide);
    const onScroll = (): void => measure();
    stage.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      ro.disconnect();
      stage.removeEventListener("scroll", onScroll);
    };
    // The deps array is intentionally a superset of what `measure`
    // reads: `slideW` / `slideH` / `panX` / `panY` / `stageW` / `stageH`
    // are not touched directly inside the effect, but they're the
    // upstream signals that move the slide on screen, so a change in
    // any one means a re-measure is needed.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    rulerOn,
    args.slideW,
    args.slideH,
    args.panX,
    args.panY,
    args.stageW,
    args.stageH,
  ]);

  return { intrinsic, intrinsicY, rulerRect };
}
