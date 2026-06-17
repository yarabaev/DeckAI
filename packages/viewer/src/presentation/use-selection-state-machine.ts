/**
 * `useSelectionStateMachine` — pointer-based selection on the slide
 * stage, plus everything that derives from the selection set.
 *
 * Responsibilities:
 * - Pointer-down / move / up state machine that distinguishes click
 *   vs. drag (≥3px movement = drag) and exposes the four
 *   `onStage*` event handlers the host wires to its stage `<main>`.
 * - Pan support when the host's `spaceHeld` flag is on (Space-held
 *   drag is the standard PowerPoint convention).
 * - Rubber-band hit-test that projects every cached bbox into
 *   screen coords via the live `getScreenCTM()` and AABB-intersects
 *   with the rubber-band rect.
 * - Double-click → enter text-edit mode for shapes that contain
 *   text.
 * - Selection-bbox projection (`selectionBoxes`) into stage-relative
 *   pixel coords, recomputed on every CTM change.
 * - Authored-font derivation (`selectionFonts`) read straight off
 *   the rendered SVG's `<tspan font-family="...">` chain.
 * - Outside-click + selection-empty auto-close for the status-bar
 *   font popover.
 * - Rubber-band rect projection for the visual overlay.
 *
 * The host owns `selectedIds` / `rubberBand` state because the
 * keyboard handler (Esc, Cmd+A, Cmd+C) also needs to mutate them;
 * the hook accepts state setters and never closes over its own
 * reducer state.
 */

import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useState,
} from "react";
import type { MutableRefObject } from "react";

import { parseFirstFontFamily } from "../svg-utils.js";
import type { RubberBandRect, SelectionBox } from "../ui/SelectionOverlay.js";

export interface RubberBandState {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

export interface PanStart {
  x: number;
  y: number;
  panX: number;
  panY: number;
}

export interface PointerDownAt {
  x: number;
  y: number;
  target: HTMLElement | null;
}

export interface UseSelectionStateMachineArgs {
  selectedIds: Set<string>;
  setSelectedIds: (
    next: Set<string> | ((prev: Set<string>) => Set<string>),
  ) => void;
  rubberBand: RubberBandState | null;
  setRubberBand: (next: RubberBandState | null) => void;
  textEditId: string | null;
  setTextEditId: (next: string | null) => void;
  spaceHeld: boolean;

  panX: number;
  panY: number;
  setPanX: (next: number) => void;
  setPanY: (next: number) => void;

  zoom: number;
  stageW: number;
  stageH: number;
  slideSvg: string;

  panStartRef: MutableRefObject<PanStart | null>;
  pointerDownAtRef: MutableRefObject<PointerDownAt | null>;
  bboxMapRef: MutableRefObject<
    Map<string, { x: number; y: number; w: number; h: number }>
  >;
  stageRef: MutableRefObject<HTMLElement | null>;
  slideRef: MutableRefObject<HTMLElement | null>;

  selectionFontsOpen: boolean;
  setSelectionFontsOpen: (next: boolean) => void;
  selectionFontsRef: MutableRefObject<HTMLDivElement | null>;
}

export interface UseSelectionStateMachineResult {
  onStagePointerDown: (ev: React.PointerEvent<HTMLElement>) => void;
  onStagePointerMove: (ev: React.PointerEvent<HTMLElement>) => void;
  onStagePointerUp: (ev: React.PointerEvent<HTMLElement>) => void;
  onStageDoubleClick: (ev: React.MouseEvent<HTMLElement>) => void;
  selectionBoxes: SelectionBox[];
  selectionFonts: string[];
  rubberBandRect: RubberBandRect | null;
}

export function useSelectionStateMachine(
  args: UseSelectionStateMachineArgs,
): UseSelectionStateMachineResult {
  const {
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
    stageW,
    stageH,
    slideSvg,
    panStartRef,
    pointerDownAtRef,
    bboxMapRef,
    stageRef,
    slideRef,
    selectionFontsOpen,
    setSelectionFontsOpen,
    selectionFontsRef,
  } = args;

  // Pointer-down: capture target's `[data-sp-id]` ancestor (or null).
  // - Space-held → start panning, capture pointer for off-stage drag.
  // - Otherwise → record snapshot for the click vs drag distinction in
  //   pointerup (≥3px movement = drag).
  const onStagePointerDown = useCallback(
    (ev: React.PointerEvent<HTMLElement>): void => {
      if (ev.button !== 0) return;
      if (textEditId) {
        // Pointer down inside the same shape that owns text-edit → let
        // text-edit keep the pointer (caret placement / selection).
        // Anywhere else → exit text-edit and treat the click as a fresh
        // selection so the user is not stuck after double-clicking once.
        const inEditTarget = (ev.target as Element | null)?.closest(
          `[data-sp-id="${textEditId}"]`,
        );
        if (inEditTarget) return;
        setTextEditId(null);
        // Fall through into the normal selection path below.
      }
      if (spaceHeld) {
        try {
          (ev.target as Element).setPointerCapture(ev.pointerId);
        } catch {
          /* not capturable */
        }
        panStartRef.current = {
          x: ev.clientX,
          y: ev.clientY,
          panX,
          panY,
        };
        return;
      }
      const target = (ev.target as Element | null)?.closest(
        "[data-sp-id]",
      ) as HTMLElement | null;
      pointerDownAtRef.current = { x: ev.clientX, y: ev.clientY, target };
      if (!target) setRubberBand(null);
    },
    [
      spaceHeld,
      textEditId,
      setTextEditId,
      panX,
      panY,
      panStartRef,
      pointerDownAtRef,
      setRubberBand,
    ],
  );

  const onStagePointerMove = useCallback(
    (ev: React.PointerEvent<HTMLElement>): void => {
      if (panStartRef.current) {
        setPanX(
          panStartRef.current.panX + (ev.clientX - panStartRef.current.x),
        );
        setPanY(
          panStartRef.current.panY + (ev.clientY - panStartRef.current.y),
        );
        return;
      }
      const downAt = pointerDownAtRef.current;
      if (!downAt) return;
      const dx = ev.clientX - downAt.x;
      const dy = ev.clientY - downAt.y;
      if (Math.hypot(dx, dy) < 3) return;
      if (!downAt.target) {
        setRubberBand({
          x0: downAt.x,
          y0: downAt.y,
          x1: ev.clientX,
          y1: ev.clientY,
        });
      }
    },
    [panStartRef, pointerDownAtRef, setPanX, setPanY, setRubberBand],
  );

  const onStagePointerUp = useCallback(
    (ev: React.PointerEvent<HTMLElement>): void => {
      if (panStartRef.current) {
        try {
          (ev.target as Element).releasePointerCapture(ev.pointerId);
        } catch {
          /* nothing captured */
        }
        panStartRef.current = null;
        return;
      }
      const downAt = pointerDownAtRef.current;
      if (!downAt) return;
      const moved =
        Math.hypot(ev.clientX - downAt.x, ev.clientY - downAt.y) >= 3;
      if (downAt.target && !moved) {
        const id = downAt.target.dataset.spId;
        if (id) {
          if (ev.shiftKey || ev.metaKey || ev.ctrlKey) {
            setSelectedIds((prev) => {
              const next = new Set(prev);
              if (next.has(id)) next.delete(id);
              else next.add(id);
              return next;
            });
          } else {
            setSelectedIds(new Set([id]));
          }
        }
      } else if (!downAt.target && !moved) {
        setSelectedIds(new Set());
      } else if (!downAt.target && moved && rubberBand) {
        // Rubber-band hit-test: project every cached bbox into screen
        // coords via the live `getScreenCTM()` and AABB-intersect with
        // the band. Coords throughout are viewport (pointer client).
        const svgEl = slideRef.current
          ?.firstElementChild as SVGSVGElement | null;
        const ctm = svgEl?.getScreenCTM?.() ?? null;
        if (svgEl && ctm) {
          const hits = new Set<string>();
          const left = Math.min(rubberBand.x0, rubberBand.x1);
          const right = Math.max(rubberBand.x0, rubberBand.x1);
          const top = Math.min(rubberBand.y0, rubberBand.y1);
          const bottom = Math.max(rubberBand.y0, rubberBand.y1);
          const p = svgEl.createSVGPoint();
          for (const [id, b] of bboxMapRef.current) {
            const corners = [
              [b.x, b.y],
              [b.x + b.w, b.y],
              [b.x, b.y + b.h],
              [b.x + b.w, b.y + b.h],
            ].map(([x, y]) => {
              p.x = x;
              p.y = y;
              return p.matrixTransform(ctm);
            });
            const xs = corners.map((c) => c.x);
            const ys = corners.map((c) => c.y);
            const rl = Math.min(...xs);
            const rr = Math.max(...xs);
            const rt = Math.min(...ys);
            const rb = Math.max(...ys);
            if (rr >= left && rl <= right && rb >= top && rt <= bottom) {
              hits.add(id);
            }
          }
          setSelectedIds(hits);
        }
      }
      pointerDownAtRef.current = null;
      setRubberBand(null);
    },
    [
      rubberBand,
      panStartRef,
      pointerDownAtRef,
      bboxMapRef,
      slideRef,
      setSelectedIds,
      setRubberBand,
    ],
  );

  const onStageDoubleClick = useCallback(
    (ev: React.MouseEvent<HTMLElement>): void => {
      const target = (ev.target as Element | null)?.closest(
        "[data-sp-id]",
      ) as HTMLElement | null;
      if (!target) return;
      // Only enter text-edit mode for shapes that actually contain text.
      if (!target.querySelector("text")) return;
      const id = target.dataset.spId;
      if (id) setTextEditId(id);
    },
    [setTextEditId],
  );

  // Project each selected shape's stored user-space bbox into the
  // overlay's stage-relative pixel space. Stored in state and refreshed
  // via useLayoutEffect (NOT useMemo) because the projection reads
  // `getScreenCTM()` from the live SVG — and during a render-phase
  // useMemo, the DOM still reflects the *previous* zoom/pan, so the
  // overlay would draw at the old screen position for one frame after
  // every zoom step. useLayoutEffect runs synchronously after commit but
  // before paint, so the CTM is current.
  const [selectionBoxes, setSelectionBoxes] = useState<SelectionBox[]>([]);
  useLayoutEffect(() => {
    if (selectedIds.size === 0) {
      setSelectionBoxes([]);
      return;
    }
    const stage = stageRef.current;
    const svgEl = slideRef.current?.firstElementChild as SVGSVGElement | null;
    if (!stage || !svgEl) {
      setSelectionBoxes([]);
      return;
    }
    const ctm = svgEl.getScreenCTM?.();
    if (!ctm) {
      setSelectionBoxes([]);
      return;
    }
    const stageRect = stage.getBoundingClientRect();
    const scrollLeft = stage.scrollLeft;
    const scrollTop = stage.scrollTop;
    const out: SelectionBox[] = [];
    const p = svgEl.createSVGPoint();
    for (const id of selectedIds) {
      const b = bboxMapRef.current.get(id);
      if (!b) continue;
      const corners = [
        [b.x, b.y],
        [b.x + b.w, b.y],
        [b.x, b.y + b.h],
        [b.x + b.w, b.y + b.h],
      ].map(([x, y]) => {
        p.x = x;
        p.y = y;
        return p.matrixTransform(ctm);
      });
      const xs = corners.map((c) => c.x);
      const ys = corners.map((c) => c.y);
      out.push({
        id,
        x: Math.min(...xs) - stageRect.left + scrollLeft,
        y: Math.min(...ys) - stageRect.top + scrollTop,
        w: Math.max(...xs) - Math.min(...xs),
        h: Math.max(...ys) - Math.min(...ys),
      });
    }
    setSelectionBoxes(out);
    // panX / panY / zoom changes move the slide → CTM changes →
    // re-project. The deps array is intentionally a superset of what
    // the body reads.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedIds, panX, panY, zoom, stageW, stageH, slideSvg]);

  // Authored font(s) of the currently selected shape(s) — read
  // straight off the rendered SVG's `<tspan font-family="...">`
  // chain. We surface the FIRST family in the chain (the authored
  // typeface) so users can identify what font a shape uses without
  // opening the font-usage popover. Multiple fonts within one shape
  // (run-level overrides) collapse to a deduplicated list;
  // multi-shape selection unions them.
  const selectionFonts: string[] = useMemo(() => {
    if (selectedIds.size === 0) return [];
    const svgEl = slideRef.current?.firstElementChild as SVGSVGElement | null;
    if (!svgEl) return [];
    const fonts = new Set<string>();
    for (const id of selectedIds) {
      // Selector is safe: sp-ids are renderer-emitted decimal integers,
      // so no CSS escaping is required and no untrusted strings are
      // interpolated into the selector.
      const host = svgEl.querySelector<SVGGElement>(`g[data-sp-id="${id}"]`);
      if (!host) continue;
      // `<tspan>` carries the most specific styling (run-level), but
      // the outer `<text>` may declare the font for an unstyled run,
      // so probe both.
      const nodes = host.querySelectorAll<SVGElement>("tspan, text");
      for (const node of nodes) {
        const family = node.getAttribute("font-family");
        if (!family) continue;
        const first = parseFirstFontFamily(family);
        if (first) fonts.add(first);
      }
    }
    return Array.from(fonts);
    // selectedIds drives the dependency; slideSvg re-runs the effect
    // after a slide swap so refs from the prior slide aren't reused.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedIds, slideSvg]);

  // Auto-close the selection-font popover whenever the underlying
  // font list goes empty (selection cleared) or the user clicks
  // outside the trigger / popover. Mirrors `FontUsageIndicator`'s
  // outside-click pattern so the status-bar UX stays consistent.
  useEffect(() => {
    if (selectionFonts.length === 0 && selectionFontsOpen) {
      setSelectionFontsOpen(false);
    }
  }, [selectionFonts, selectionFontsOpen, setSelectionFontsOpen]);
  useEffect(() => {
    if (!selectionFontsOpen) return;
    function onDocClick(ev: MouseEvent) {
      const root = selectionFontsRef.current;
      if (root && !root.contains(ev.target as Node)) {
        setSelectionFontsOpen(false);
      }
    }
    document.addEventListener("mousedown", onDocClick);
    return () => document.removeEventListener("mousedown", onDocClick);
  }, [selectionFontsOpen, selectionFontsRef, setSelectionFontsOpen]);

  const rubberBandRect: RubberBandRect | null = useMemo(() => {
    if (!rubberBand) return null;
    const stage = stageRef.current;
    if (!stage) return null;
    const r = stage.getBoundingClientRect();
    const left =
      Math.min(rubberBand.x0, rubberBand.x1) - r.left + stage.scrollLeft;
    const top =
      Math.min(rubberBand.y0, rubberBand.y1) - r.top + stage.scrollTop;
    const right =
      Math.max(rubberBand.x0, rubberBand.x1) - r.left + stage.scrollLeft;
    const bottom =
      Math.max(rubberBand.y0, rubberBand.y1) - r.top + stage.scrollTop;
    return { x: left, y: top, w: right - left, h: bottom - top };
  }, [rubberBand, stageRef]);

  return {
    onStagePointerDown,
    onStagePointerMove,
    onStagePointerUp,
    onStageDoubleClick,
    selectionBoxes,
    selectionFonts,
    rubberBandRect,
  };
}
