import type { PositionedNode } from "../../types.ts";
import type { RenderContext } from "../types.ts";
import {
  resolveFontFamily,
  resolveTextStyleValue,
} from "../../defaultTextStyle.ts";
import { pxToIn, pxToPt } from "../units.ts";
import { convertUnderline, convertStrike } from "../textOptions.ts";
import { renderObjectName } from "../utils/objectName.ts";

type ShapePositionedNode = Extract<PositionedNode, { type: "shape" }>;

type PaddingInput =
  | number
  | { top?: number; right?: number; bottom?: number; left?: number }
  | undefined;

// pptxgenjs shape/textbox margin array order is [left, right, bottom, top]
// (see pptxgenjs dist: margin[0]→lIns, [1]→rIns, [2]→bIns, [3]→tIns).
function resolveShapeTextInsetPt(
  padding: PaddingInput,
): [number, number, number, number] {
  if (padding === undefined) return [0, 0, 0, 0];
  if (typeof padding === "number") {
    const p = pxToPt(padding);
    return [p, p, p, p];
  }
  return [
    pxToPt(padding.left ?? 0),
    pxToPt(padding.right ?? 0),
    pxToPt(padding.bottom ?? 0),
    pxToPt(padding.top ?? 0),
  ];
}

export function renderShapeNode(
  node: ShapePositionedNode,
  ctx: RenderContext,
): void {
  const defaultTextStyle = ctx.buildContext?.defaultTextStyle;
  const objectName = renderObjectName(node, ctx);
  const shapeOptions = {
    x: pxToIn(node.x),
    y: pxToIn(node.y),
    w: pxToIn(node.w),
    h: pxToIn(node.h),
    ...(objectName ? { objectName } : {}),
    ...(node.rotate !== undefined ? { rotate: node.rotate } : {}),
    fill: node.fill
      ? {
          color: node.fill.color,
          transparency:
            node.fill.transparency !== undefined
              ? node.fill.transparency * 100
              : undefined,
        }
      : undefined,
    line: node.line
      ? {
          color: node.line.color,
          width:
            node.line.width !== undefined ? pxToPt(node.line.width) : undefined,
          dashType: node.line.dashType,
        }
      : undefined,
    shadow: node.shadow
      ? {
          type: node.shadow.type ?? ("outer" as const),
          opacity: node.shadow.opacity,
          blur: node.shadow.blur,
          angle: node.shadow.angle,
          offset: node.shadow.offset,
          color: node.shadow.color,
        }
      : undefined,
  };

  if (node.text) {
    ctx.slide.addText(node.text, {
      ...shapeOptions,
      shape: node.shapeType,
      fontSize: pxToPt(
        resolveTextStyleValue(node.fontSize, defaultTextStyle?.fontSize, 24),
      ),
      fontFace: resolveFontFamily(node.fontFamily, defaultTextStyle),
      color: node.color ?? defaultTextStyle?.color,
      bold: node.bold ?? defaultTextStyle?.bold,
      italic: node.italic ?? defaultTextStyle?.italic,
      underline: convertUnderline(node.underline),
      strike: convertStrike(node.strike),
      highlight: node.highlight,
      align: node.textAlign ?? "center",
      valign: node.textVAlign ?? ("middle" as const),
      lineSpacingMultiple: resolveTextStyleValue(
        node.lineHeight,
        defaultTextStyle?.lineHeight,
        1.0,
      ),
      charSpacing:
        node.letterSpacing !== undefined ? node.letterSpacing * 100 : undefined,
      margin: resolveShapeTextInsetPt(node.padding),
    });
  } else {
    ctx.slide.addShape(node.shapeType, shapeOptions);
  }
}
