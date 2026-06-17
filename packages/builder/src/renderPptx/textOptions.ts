import type { DefaultTextStyle } from "../types.ts";
import {
  resolveFontFamily,
  resolveTextStyleValue,
} from "../defaultTextStyle.ts";
import type { PositionedNode, Underline, UnderlineStyle } from "../types.ts";
import { pxToIn, pxToPt } from "./units.ts";
import { getContentArea } from "./utils/contentArea.ts";

type TextNode = Extract<PositionedNode, { type: "text" }>;

/**
 * Converts the underline property to pptxgenjs format.
 */
export function convertUnderline(
  underline: Underline | undefined,
): { style?: UnderlineStyle; color?: string } | undefined {
  if (underline === undefined) return undefined;
  if (underline === false) return undefined;
  if (underline === true) return { style: "sng" };
  return {
    style: underline.style,
    color: underline.color,
  };
}

/**
 * Converts the strike property to pptxgenjs format.
 */
export function convertStrike(
  strike: boolean | undefined,
): "sngStrike" | undefined {
  if (strike) return "sngStrike";
  return undefined;
}

export function createTextOptions(
  node: TextNode,
  defaultTextStyle?: DefaultTextStyle,
) {
  const fontSizePx = resolveTextStyleValue(
    node.fontSize,
    defaultTextStyle?.fontSize,
    24,
  );
  const fontFamily = resolveFontFamily(node.fontFamily, defaultTextStyle);
  const lineHeight = resolveTextStyleValue(
    node.lineHeight,
    defaultTextStyle?.lineHeight,
    1.0,
  );
  const content = getContentArea(node);

  return {
    x: pxToIn(content.x),
    y: pxToIn(content.y),
    w: pxToIn(content.w),
    h: pxToIn(content.h),
    fontSize: pxToPt(fontSizePx),
    fontFace: fontFamily,
    align: node.textAlign ?? "left",
    // `textVAlign` controls glyph anchor inside the rendered text frame.
    // Defaults to "top" — matches the previous hard-coded behavior. When
    // an HStack stretches a smaller-fontSize sibling to the row's max
    // height, setting `textVAlign="middle"` is what visually centers the
    // glyphs (without it the glyphs float to the top of the equalized
    // box, creating the optical misalignment that mixed-size text rows
    // are known for).
    valign: node.textVAlign ?? ("top" as const),
    margin: 0,
    lineSpacingMultiple: lineHeight,
    color: node.color ?? defaultTextStyle?.color,
    bold: node.bold ?? defaultTextStyle?.bold,
    italic: node.italic ?? defaultTextStyle?.italic,
    underline: convertUnderline(node.underline),
    strike: convertStrike(node.strike),
    highlight: node.highlight,
    charSpacing:
      node.letterSpacing !== undefined ? node.letterSpacing * 100 : undefined,
  };
}
