import type { PositionedNode, LineArrow } from "../../types.ts";
import type { RenderContext } from "../types.ts";
import { pxToIn, pxToPt } from "../units.ts";
import { renderObjectName } from "../utils/objectName.ts";

type LinePositionedNode = Extract<PositionedNode, { type: "line" }>;

/**
 * Resolve a pptxgenjs arrow type from a `boolean | LineArrowOptions` value.
 */
function resolveArrowType(
  arrow: LineArrow | undefined,
): "none" | "arrow" | "diamond" | "oval" | "stealth" | "triangle" | undefined {
  if (arrow === undefined) {
    return undefined;
  }
  if (arrow === false) {
    return "none";
  }
  if (arrow === true) {
    return "triangle"; // default
  }
  return arrow.type ?? "triangle";
}

export function renderLineNode(
  node: LinePositionedNode,
  ctx: RenderContext,
): void {
  const { x1, y1, x2, y2, color, lineWidth, dashType, beginArrow, endArrow } =
    node;

  // Draw the pptxgenjs line shape with x, y, w, h.
  // x, y are the top-left coordinate; w, h carry direction and length.
  const minX = Math.min(x1, x2);
  const minY = Math.min(y1, y2);
  const lineW = Math.abs(x2 - x1);
  const lineH = Math.abs(y2 - y1);

  // Detect line direction to decide the flip.
  // flipH: line points right -> left.
  // flipV: line points bottom -> top.
  const flipH = x2 < x1;
  const flipV = y2 < y1;

  const objectName = renderObjectName(node, ctx);
  ctx.slide.addShape(ctx.pptx.ShapeType.line, {
    x: pxToIn(minX),
    y: pxToIn(minY),
    w: pxToIn(lineW),
    h: pxToIn(lineH),
    flipH,
    flipV,
    ...(objectName ? { objectName } : {}),
    line: {
      color: color ?? "000000",
      width: lineWidth !== undefined ? pxToPt(lineWidth) : 1,
      dashType: dashType,
      beginArrowType: resolveArrowType(beginArrow),
      endArrowType: resolveArrowType(endArrow),
    },
  });
}
