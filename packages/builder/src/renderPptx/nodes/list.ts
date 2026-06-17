import type { PositionedNode, LiNode } from "../../types.ts";
import type { RenderContext } from "../types.ts";
import {
  resolveFontFamily,
  resolveTextStyleValue,
} from "../../defaultTextStyle.ts";
import { pxToIn, pxToPt } from "../units.ts";
import { convertUnderline, convertStrike } from "../textOptions.ts";
import { getContentArea } from "../utils/contentArea.ts";
import { renderObjectName } from "../utils/objectName.ts";
import { validateHref } from "../utils/href.ts";

type UlPositionedNode = Extract<PositionedNode, { type: "ul" }>;
type OlPositionedNode = Extract<PositionedNode, { type: "ol" }>;

function resolveStyle(
  li: LiNode,
  parent: UlPositionedNode | OlPositionedNode,
  defaultTextStyle: RenderContext["buildContext"]["defaultTextStyle"],
) {
  return {
    fontSize: resolveTextStyleValue(
      li.fontSize,
      parent.fontSize ?? defaultTextStyle?.fontSize,
      24,
    ),
    color: li.color ?? parent.color ?? defaultTextStyle?.color,
    bold: li.bold ?? parent.bold ?? defaultTextStyle?.bold,
    italic: li.italic ?? parent.italic ?? defaultTextStyle?.italic,
    underline: li.underline ?? parent.underline,
    strike: li.strike ?? parent.strike,
    highlight: li.highlight ?? parent.highlight,
    fontFamily: resolveFontFamily(
      li.fontFamily ?? parent.fontFamily,
      defaultTextStyle,
    ),
  };
}

function buildListTextItems(
  items: LiNode[],
  parent: UlPositionedNode | OlPositionedNode,
  defaultTextStyle: RenderContext["buildContext"]["defaultTextStyle"],
  bullet: boolean | Record<string, unknown>,
  ctx: RenderContext,
) {
  const textItems: { text: string; options: Record<string, unknown> }[] = [];
  const allowedSchemes = ctx.buildContext.security.allowedHrefSchemes;
  for (const [i, li] of items.entries()) {
    const style = resolveStyle(li, parent, defaultTextStyle);
    const isLast = i === items.length - 1;
    const baseOptions = {
      fontSize: pxToPt(style.fontSize),
      fontFace: style.fontFamily,
      color: style.color,
      underline: convertUnderline(style.underline),
      strike: convertStrike(style.strike),
      highlight: style.highlight,
    };

    if (li.runs && li.runs.length > 0) {
      const runs = li.runs;
      for (const [j, run] of runs.entries()) {
        const isLastRun = j === runs.length - 1;
        const validatedHref = run.href
          ? validateHref(run.href, allowedSchemes, ctx)
          : undefined;
        textItems.push({
          text: run.text,
          options: {
            ...baseOptions,
            color: run.color ?? style.color,
            bold: run.bold ?? style.bold,
            italic: run.italic ?? style.italic,
            underline: convertUnderline(run.underline ?? style.underline),
            strike: convertStrike(run.strike ?? style.strike),
            highlight: run.highlight ?? style.highlight,
            bullet: j === 0 ? bullet : false,
            // breakLine on the final run of every non-last <Li> opens a new
            // paragraph for the next item, so each <Li> renders with its own
            // bullet/number marker. A literal "\n" here only inserts a soft
            // line break inside the current paragraph and collapses items.
            ...(isLastRun && !isLast ? { breakLine: true } : {}),
            ...(validatedHref ? { hyperlink: { url: validatedHref } } : {}),
          },
        });
      }
    } else {
      textItems.push({
        text: li.text,
        options: {
          ...baseOptions,
          bold: style.bold,
          italic: style.italic,
          bullet,
          ...(isLast ? {} : { breakLine: true }),
        },
      });
    }
  }
  return textItems;
}

function hasItemStyleOverride(items: LiNode[]): boolean {
  return items.some(
    (li) =>
      li.fontSize !== undefined ||
      li.color !== undefined ||
      li.bold !== undefined ||
      li.italic !== undefined ||
      li.underline !== undefined ||
      li.strike !== undefined ||
      li.highlight !== undefined ||
      li.fontFamily !== undefined ||
      li.runs !== undefined,
  );
}

export function renderUlNode(node: UlPositionedNode, ctx: RenderContext): void {
  const defaultTextStyle = ctx.buildContext?.defaultTextStyle;
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
  const objectName = renderObjectName(node, ctx);

  // bullet indent: px -> pt for pptxgenjs
  const ulBullet: Record<string, unknown> = {
    indent: pxToPt(node.bulletIndent ?? 19),
  };

  if (hasItemStyleOverride(node.items)) {
    // Use array form when Li has per-item styling
    const textItems = buildListTextItems(
      node.items,
      node,
      defaultTextStyle,
      ulBullet,
      ctx,
    );

    ctx.slide.addText(textItems, {
      x: pxToIn(content.x),
      y: pxToIn(content.y),
      w: pxToIn(content.w),
      h: pxToIn(content.h),
      align: node.textAlign ?? "left",
      valign: node.textVAlign ?? ("top" as const),
      margin: 0,
      lineSpacingMultiple: lineHeight,
      charSpacing:
        node.letterSpacing !== undefined ? node.letterSpacing * 100 : undefined,
      ...(objectName ? { objectName } : {}),
    });
  } else {
    const text = node.items.map((li) => li.text).join("\n");

    ctx.slide.addText(text, {
      x: pxToIn(content.x),
      y: pxToIn(content.y),
      w: pxToIn(content.w),
      h: pxToIn(content.h),
      fontSize: pxToPt(fontSizePx),
      fontFace: fontFamily,
      align: node.textAlign ?? "left",
      valign: node.textVAlign ?? ("top" as const),
      margin: 0,
      lineSpacingMultiple: lineHeight,
      charSpacing:
        node.letterSpacing !== undefined ? node.letterSpacing * 100 : undefined,
      color: node.color ?? defaultTextStyle?.color,
      bold: node.bold ?? defaultTextStyle?.bold,
      italic: node.italic ?? defaultTextStyle?.italic,
      underline: convertUnderline(node.underline),
      strike: convertStrike(node.strike),
      highlight: node.highlight,
      bullet: ulBullet,
      ...(objectName ? { objectName } : {}),
    });
  }
}

export function renderOlNode(node: OlPositionedNode, ctx: RenderContext): void {
  const defaultTextStyle = ctx.buildContext?.defaultTextStyle;
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
  const objectName = renderObjectName(node, ctx);

  const bulletOptions: Record<string, unknown> = {
    type: "number",
    indent: pxToPt(node.bulletIndent ?? 19),
  };
  if (node.numberType !== undefined) {
    bulletOptions.numberType = node.numberType;
  }
  if (node.numberStartAt !== undefined) {
    bulletOptions.numberStartAt = node.numberStartAt;
  }

  if (hasItemStyleOverride(node.items)) {
    const textItems = buildListTextItems(
      node.items,
      node,
      defaultTextStyle,
      bulletOptions,
      ctx,
    );

    ctx.slide.addText(textItems, {
      x: pxToIn(content.x),
      y: pxToIn(content.y),
      w: pxToIn(content.w),
      h: pxToIn(content.h),
      align: node.textAlign ?? "left",
      valign: node.textVAlign ?? ("top" as const),
      margin: 0,
      lineSpacingMultiple: lineHeight,
      charSpacing:
        node.letterSpacing !== undefined ? node.letterSpacing * 100 : undefined,
      ...(objectName ? { objectName } : {}),
    });
  } else {
    const text = node.items.map((li) => li.text).join("\n");

    ctx.slide.addText(text, {
      x: pxToIn(content.x),
      y: pxToIn(content.y),
      w: pxToIn(content.w),
      h: pxToIn(content.h),
      fontSize: pxToPt(fontSizePx),
      fontFace: fontFamily,
      align: node.textAlign ?? "left",
      valign: node.textVAlign ?? ("top" as const),
      margin: 0,
      lineSpacingMultiple: lineHeight,
      charSpacing:
        node.letterSpacing !== undefined ? node.letterSpacing * 100 : undefined,
      color: node.color ?? defaultTextStyle?.color,
      bold: node.bold ?? defaultTextStyle?.bold,
      italic: node.italic ?? defaultTextStyle?.italic,
      underline: convertUnderline(node.underline),
      strike: convertStrike(node.strike),
      highlight: node.highlight,
      bullet: bulletOptions,
      ...(objectName ? { objectName } : {}),
    });
  }
}
