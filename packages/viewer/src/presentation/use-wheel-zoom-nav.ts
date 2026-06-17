/**
 * `useWheelZoomNav` — wheel + pinch handler installed on the slide
 * stage. Two flavours of intent are dispatched off the same event:
 *
 * - **Zoom** (Ctrl/Cmd-wheel, pinch). The browser delivers pinch as
 *   a synthetic `wheel` with `ctrlKey=true` so the same path covers
 *   trackpad pinch and key-modified scroll wheels. Deltas accumulate
 *   inside a single rAF tick so a fast pinch-zoom doesn't fire a
 *   `setZoom` per event.
 * - **Slide nav** (plain wheel). Native scroll handles in-slide
 *   panning until the user hits the top / bottom edge, after which
 *   wheel travel above `BOUNDARY_THRESHOLD` commits to the next /
 *   previous slide. A `COOLDOWN_MS` window swallows subsequent wheel
 *   events to absorb macOS trackpad inertia tails.
 *
 * Constants tuned conservatively: the historic Lit shell used
 * 1.1/0.9 step factors that felt jumpy on trackpads. The
 * exponential `factor = exp(-delta * SENSITIVITY)` form gives the
 * same perceived speed as a Cmd-wheel "click" while making pinch
 * proportional and decoupling line / page / pixel deltaModes from
 * the threshold constant.
 */

import { useEffect } from "react";
import type { MutableRefObject } from "react";

const ZOOM_MIN = 0.25;
const ZOOM_MAX = 8;
const SENSITIVITY = 0.0035;
const BOUNDARY_THRESHOLD = 240;
const COOLDOWN_MS = 350;
const RESET_AFTER_MS = 250;

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export interface UseWheelZoomNavArgs {
  stageRef: MutableRefObject<HTMLElement | null>;
  slideshow: boolean;
  viewMode: "normal" | "grid";
  slideCount: number;
  setZoom: (next: number | ((prev: number) => number)) => void;
  setCurrentSlide: (next: number | ((prev: number) => number)) => void;
}

export function useWheelZoomNav(args: UseWheelZoomNavArgs): void {
  const {
    stageRef,
    slideshow,
    viewMode,
    slideCount,
    setZoom,
    setCurrentSlide,
  } = args;

  useEffect(() => {
    const stage = stageRef.current;
    if (!stage) return;

    let pendingZoomDelta = 0;
    let raf = 0;
    let boundaryDelta = 0;
    let lastWheelAt = 0;
    let cooldownUntil = 0;

    const flushZoom = (): void => {
      raf = 0;
      if (pendingZoomDelta === 0) return;
      // exp(-d * k): negative delta (zoom-in) → factor > 1.
      const factor = Math.exp(-pendingZoomDelta * SENSITIVITY);
      pendingZoomDelta = 0;
      setZoom((z) => clamp(z * factor, ZOOM_MIN, ZOOM_MAX));
    };

    const onWheel = (ev: WheelEvent): void => {
      // DOM_DELTA_LINE / PAGE → multiply to a px-equivalent so the
      // sensitivity / threshold constants work the same regardless
      // of input device. Most trackpads ship pixel deltas already.
      let dy = ev.deltaY;
      if (ev.deltaMode === 1)
        dy *= 16; // line → px
      else if (ev.deltaMode === 2) dy *= stage.clientHeight; // page

      // ctrlKey covers both real Ctrl-wheel AND the synthetic pinch
      // event the browser fires as ctrlKey=true.
      const isZoomIntent = ev.ctrlKey || ev.metaKey;
      if (isZoomIntent) {
        ev.preventDefault();
        pendingZoomDelta += dy;
        if (raf === 0) raf = requestAnimationFrame(flushZoom);
        return;
      }

      // Slideshow / grid view manage their own navigation; only the
      // normal slide stage opts into wheel-driven page flipping.
      if (slideshow) return;
      if (viewMode !== "normal") return;
      if (slideCount <= 0) return;

      const now = performance.now();
      if (now < cooldownUntil) {
        // Inertia tail after a recent slide change — block native
        // scroll too so the new slide stays put while the gesture
        // dies down.
        ev.preventDefault();
        return;
      }
      if (now - lastWheelAt > RESET_AFTER_MS) {
        boundaryDelta = 0;
      }
      lastWheelAt = now;

      // Treat sub-pixel rounding as still-at-edge.
      const atBottom =
        stage.scrollTop + stage.clientHeight >= stage.scrollHeight - 1;
      const atTop = stage.scrollTop <= 0;

      if (dy > 0) {
        if (!atBottom) {
          boundaryDelta = 0;
          return; // native scroll handles it
        }
        ev.preventDefault();
        boundaryDelta = Math.max(0, boundaryDelta) + dy;
        if (boundaryDelta >= BOUNDARY_THRESHOLD) {
          boundaryDelta = 0;
          cooldownUntil = now + COOLDOWN_MS;
          setCurrentSlide((s) => Math.min(slideCount, s + 1));
        }
      } else if (dy < 0) {
        if (!atTop) {
          boundaryDelta = 0;
          return;
        }
        ev.preventDefault();
        boundaryDelta = Math.min(0, boundaryDelta) + dy;
        if (boundaryDelta <= -BOUNDARY_THRESHOLD) {
          boundaryDelta = 0;
          cooldownUntil = now + COOLDOWN_MS;
          setCurrentSlide((s) => Math.max(1, s - 1));
        }
      }
    };

    stage.addEventListener("wheel", onWheel, { passive: false });
    return () => {
      stage.removeEventListener("wheel", onWheel);
      if (raf !== 0) cancelAnimationFrame(raf);
    };
    // Re-bind on slideshow / viewMode / slideCount changes so the
    // handler closes over the current values instead of stale state.
  }, [stageRef, slideshow, viewMode, slideCount, setZoom, setCurrentSlide]);
}
