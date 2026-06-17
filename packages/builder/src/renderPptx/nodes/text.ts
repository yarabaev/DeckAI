import type { PositionedNode } from "../../types.ts";
import type { RenderContext } from "../types.ts";
import {
  createTextOptions,
  convertUnderline,
  convertStrike,
} from "../textOptions.ts";
import {
  resolveFontFamily,
  resolveTextStyleValue,
} from "../../defaultTextStyle.ts";
import { pxToIn, pxToPt } from "../units.ts";
import { shouldEmbedBackgroundInText } from "../utils/backgroundBorder.ts";
import { renderObjectName } from "../utils/objectName.ts";
import { validateHref } from "../utils/href.ts";

type TextPositionedNode = Extract<PositionedNode, { type: "text" }>;

function buildRuns(
  node: TextPositionedNode,
  defaultTextStyle:
    | RenderContext["buildContext"]["defaultTextStyle"]
    | undefined,
  ctx: RenderContext,
) {
  const fontSizePx = resolveTextStyleValue(
    node.fontSize,
    defaultTextStyle?.fontSize,
    24,
  );
  const fontFamily = resolveFontFamily(node.fontFamily, defaultTextStyle);
  const allowedSchemes = ctx.buildContext.security.allowedHrefSchemes;
  // Caller guarantees `runs` is non-empty; treat absence as an empty array.
  const runs = node.runs ?? [];
  return runs.map((run) => {
    const validatedHref = run.href
      ? validateHref(run.href, allowedSchemes, ctx)
      : undefined;
    return {
      text: run.text,
      options: {
        fontSize: pxToPt(fontSizePx),
        fontFace: fontFamily,
        color: run.color ?? node.color ?? defaultTextStyle?.color,
        bold: run.bold ?? node.bold ?? defaultTextStyle?.bold,
        italic: run.italic ?? node.italic ?? defaultTextStyle?.italic,
        underline: convertUnderline(run.underline ?? node.underline),
        strike: convertStrike(run.strike ?? node.strike),
        highlight: run.highlight ?? node.highlight,
        ...((run.lang ?? ctx.buildContext.defaultLang)
          ? { lang: run.lang ?? ctx.buildContext.defaultLang }
          : {}),
        ...(validatedHref ? { hyperlink: { url: validatedHref } } : {}),
      },
    };
  });
}

function renderEmbeddedTextNode(
  node: TextPositionedNode,
  ctx: RenderContext,
): void {
  const defaultTextStyle = ctx.buildContext?.defaultTextStyle;
  const base = createTextOptions(node, defaultTextStyle);

  const hasBg = Boolean(node.backgroundColor);
  const hasBorder = Boolean(
    node.border &&
    (node.border.color !== undefined ||
      node.border.width !== undefined ||
      node.border.dashType !== undefined),
  );

  const fill = hasBg
    ? {
        color: node.backgroundColor,
        transparency:
          node.opacity !== undefined ? (1 - node.opacity) * 100 : undefined,
      }
    : { type: "none" as const };

  const line = hasBorder
    ? {
        color: node.border?.color ?? "000000",
        width:
          node.border?.width !== undefined
            ? pxToPt(node.border.width)
            : undefined,
        dashType: node.border?.dashType,
      }
    : { type: "none" as const };

  const shapeType =
    node.borderRadius !== undefined
      ? ctx.pptx.ShapeType.roundRect
      : ctx.pptx.ShapeType.rect;
  const rectRadius =
    node.borderRadius !== undefined
      ? Math.min((node.borderRadius / Math.min(node.w, node.h)) * 2, 1)
      : undefined;

  const shadow = node.shadow
    ? {
        type: node.shadow.type ?? ("outer" as const),
        opacity: node.shadow.opacity,
        blur: node.shadow.blur,
        angle: node.shadow.angle,
        offset: node.shadow.offset,
        color: node.shadow.color,
      }
    : undefined;

  // Map node padding to pptxgenjs bodyPr inset margin (points).
  // pptxgenjs interprets the array as [lIns, rIns, bIns, tIns]
  // (margin[0]→left, [1]→right, [2]→bottom, [3]→top — see pptxgenjs
  // dist around line 5387). Earlier code passed [L, T, R, B] which
  // swapped padding.top/right and padding.bottom/top.
  let padTop = 0;
  let padRight = 0;
  let padBottom = 0;
  let padLeft = 0;
  if (typeof node.padding === "number") {
    padTop = padRight = padBottom = padLeft = node.padding;
  } else if (node.padding) {
    padTop = node.padding.top ?? 0;
    padRight = node.padding.right ?? 0;
    padBottom = node.padding.bottom ?? 0;
    padLeft = node.padding.left ?? 0;
  }
  const margin: [number, number, number, number] = [
    pxToPt(padLeft),
    pxToPt(padRight),
    pxToPt(padBottom),
    pxToPt(padTop),
  ];

  const objectName = renderObjectName(node, ctx);
  const frameOptions = {
    shape: shapeType,
    x: pxToIn(node.x),
    y: pxToIn(node.y),
    w: pxToIn(node.w),
    h: pxToIn(node.h),
    fill,
    line,
    rectRadius,
    shadow,
    align: base.align,
    valign: base.valign,
    margin,
    lineSpacingMultiple: base.lineSpacingMultiple,
    ...(objectName ? { objectName } : {}),
  };

  if (node.runs && node.runs.length > 0) {
    ctx.slide.addText(buildRuns(node, defaultTextStyle, ctx), frameOptions);
  } else {
    ctx.slide.addText(node.text ?? "", {
      ...frameOptions,
      fontSize: base.fontSize,
      fontFace: base.fontFace,
      color: base.color,
      bold: base.bold,
      italic: base.italic,
      underline: base.underline,
      strike: base.strike,
      highlight: base.highlight,
    });
  }
}

export function renderTextNode(
  node: TextPositionedNode,
  ctx: RenderContext,
): void {
  if (shouldEmbedBackgroundInText(node)) {
    renderEmbeddedTextNode(node, ctx);
    return;
  }

  const defaultTextStyle = ctx.buildContext?.defaultTextStyle;
  const textOptions = createTextOptions(node, defaultTextStyle);
  const objectName = renderObjectName(node, ctx);

  if (node.runs && node.runs.length > 0) {
    ctx.slide.addText(buildRuns(node, defaultTextStyle, ctx), {
      x: textOptions.x,
      y: textOptions.y,
      w: textOptions.w,
      h: textOptions.h,
      align: textOptions.align,
      valign: textOptions.valign,
      margin: textOptions.margin,
      lineSpacingMultiple: textOptions.lineSpacingMultiple,
      ...(objectName ? { objectName } : {}),
    });
  } else {
    ctx.slide.addText(node.text ?? "", {
      ...textOptions,
      ...(objectName ? { objectName } : {}),
    });
  }
}
