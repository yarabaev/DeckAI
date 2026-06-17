import { type CSSProperties } from "react";

/**
 * One selected shape's stage-relative bounding box (CSS pixels).
 *
 * Computed by projecting the SVG shape's user-space bbox through the
 * SVG element's `getScreenCTM()`, then translating into the stage
 * element's local frame so the overlay (which is `position: absolute;
 * inset: 0` inside the stage) can render at viewport coords without
 * any further transform.
 */
export interface SelectionBox {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface RubberBandRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface SelectionOverlayProps {
  boxes: SelectionBox[];
  rubberBand: RubberBandRect | null;
}

/**
 * Purely-visual sibling of the slide host inside the stage. Draws a
 * dashed bbox + 8 corner / midpoint handles for every `boxes[]`
 * entry, plus an optional translucent dashed `rubberBand` rect during
 * empty-canvas drag selection.
 *
 * The overlay is `pointer-events: none`, so the underlying selection
 * state machine on `<PptxPresentation>` keeps owning all pointer
 * events. No event handlers — display only.
 */
export function SelectionOverlay(props: SelectionOverlayProps): JSX.Element {
  const { boxes, rubberBand } = props;
  return (
    <svg style={overlaySvgStyle} aria-hidden="true">
      {boxes.map((b) => (
        <g key={b.id}>
          <rect
            data-sp-id={b.id}
            x={b.x}
            y={b.y}
            width={b.w}
            height={b.h}
            fill="rgba(106, 163, 255, 0.08)"
            stroke="var(--pptx-shell-accent, #6aa3ff)"
            strokeWidth={1}
            strokeDasharray="4 4"
          />
          {handles(b).map((h, i) => (
            <circle
              key={i}
              cx={h.cx}
              cy={h.cy}
              r={3}
              fill="#fff"
              stroke="var(--pptx-shell-accent, #6aa3ff)"
              strokeWidth={1}
            />
          ))}
        </g>
      ))}
      {rubberBand ? (
        <rect
          x={rubberBand.x}
          y={rubberBand.y}
          width={rubberBand.w}
          height={rubberBand.h}
          fill="rgba(106, 163, 255, 0.15)"
          stroke="var(--pptx-shell-accent, #6aa3ff)"
          strokeWidth={1}
          strokeDasharray="2 2"
        />
      ) : null}
    </svg>
  );
}

function handles(b: SelectionBox): { cx: number; cy: number }[] {
  return [
    { cx: b.x, cy: b.y },
    { cx: b.x + b.w / 2, cy: b.y },
    { cx: b.x + b.w, cy: b.y },
    { cx: b.x, cy: b.y + b.h / 2 },
    { cx: b.x + b.w, cy: b.y + b.h / 2 },
    { cx: b.x, cy: b.y + b.h },
    { cx: b.x + b.w / 2, cy: b.y + b.h },
    { cx: b.x + b.w, cy: b.y + b.h },
  ];
}

const overlaySvgStyle: CSSProperties = {
  position: "absolute",
  inset: 0,
  width: "100%",
  height: "100%",
  pointerEvents: "none",
  zIndex: 4,
};
