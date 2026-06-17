import type { PositionedNode } from "../../types.ts";
import type { RenderContext } from "../types.ts";
import { pxToIn } from "../units.ts";
import { renderObjectName } from "../utils/objectName.ts";

type IconPositionedNode = Extract<PositionedNode, { type: "icon" }>;

export function renderIconNode(
  node: IconPositionedNode,
  ctx: RenderContext,
): void {
  const objectName = renderObjectName(node, ctx);
  // pptxgenjs uses `transparency` as percent-opaque-removed (0 = fully
  // visible, 100 = invisible). Translate the builder's 0-1 opacity to
  // that scale; `undefined` skips the attribute entirely so the default
  // (opaque) path is unchanged for callers that don't specify opacity.
  const transparency =
    node.opacity !== undefined ? (1 - node.opacity) * 100 : undefined;

  // Draw background shape when variant is specified
  if (node.variant) {
    const isCircle = node.variant.startsWith("circle");
    const isFilled = node.variant.endsWith("-filled");
    const bgColor = node.backgroundColor ?? "#E0E0E0";
    const colorValue = bgColor.replace(/^#/, "");

    const shapeType = isCircle ? "ellipse" : "roundRect";
    const shapeOptions: Record<string, unknown> = {
      x: pxToIn(node.bgX ?? node.x),
      y: pxToIn(node.bgY ?? node.y),
      w: pxToIn(node.bgW ?? node.w),
      h: pxToIn(node.bgH ?? node.h),
      fill: isFilled
        ? {
            color: colorValue,
            ...(transparency !== undefined ? { transparency } : {}),
          }
        : { type: "none" as const },
      line: isFilled ? undefined : { color: colorValue, width: 1.5 },
      rectRadius: isCircle ? undefined : 0.1,
      ...(objectName ? { objectName } : {}),
    };

    ctx.slide.addShape(shapeType, shapeOptions);
  }

  ctx.slide.addImage({
    data: node.iconImageData,
    x: pxToIn(node.iconX ?? node.x),
    y: pxToIn(node.iconY ?? node.y),
    w: pxToIn(node.iconW ?? node.w),
    h: pxToIn(node.iconH ?? node.h),
    ...(transparency !== undefined ? { transparency } : {}),
    ...(objectName ? { objectName } : {}),
    ...(node.isDecorative
      ? { altText: "" }
      : node.altText !== undefined
        ? { altText: node.altText }
        : {}),
  });
}
