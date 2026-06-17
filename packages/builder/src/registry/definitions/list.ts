import type { BuilderNode } from "../../types.ts";
import type { NodeDefinition, Yoga } from "../types.ts";
import type { Node as YogaNode } from "yoga-layout";
import { measureText } from "../../calcYogaLayout/measureText.ts";
import { measureFontLineHeightRatio } from "../../calcYogaLayout/fontLoader.ts";
import type { BuildContext } from "../../buildContext.ts";
import {
  resolveFontFamily,
  resolveTextStyleValue,
} from "../../defaultTextStyle.ts";
import { renderUlNode, renderOlNode } from "../../renderPptx/nodes/list.ts";

function applyListYogaStyle(
  node: BuilderNode,
  yn: YogaNode,
  yoga: Yoga,
  ctx: BuildContext,
) {
  const n = node as Extract<BuilderNode, { type: "ul" | "ol" }>;
  const combinedText = n.items.map((item) => item.text).join("\n");
  const fontSizePx = resolveTextStyleValue(
    n.fontSize,
    ctx.defaultTextStyle.fontSize,
    24,
  );
  const fontFamily = resolveFontFamily(n.fontFamily, ctx.defaultTextStyle);
  const fontWeight = resolveTextStyleValue(
    n.bold,
    ctx.defaultTextStyle.bold,
    false,
  )
    ? "bold"
    : "normal";
  const spacingMultiple = resolveTextStyleValue(
    n.lineHeight,
    ctx.defaultTextStyle.lineHeight,
    1.0,
  );

  const fontMetricsRatio = measureFontLineHeightRatio(
    fontFamily,
    fontWeight,
    ctx.measurer,
  );
  const lineHeight = fontMetricsRatio * spacingMultiple;

  // Bullet/number indent width in px (consistent with fontSize/w/h). Default 19 px (~0.5 cm).
  const bulletIndentPx = n.bulletIndent ?? 19;

  yn.setMeasureFunc((width, widthMode) => {
    const maxWidthPx = (() => {
      // `noWrap` forces a single-line measurement (per item) regardless
      // of mode — the flex layout may overflow horizontally rather than
      // wrap a list item.
      if (n.noWrap) return Number.POSITIVE_INFINITY;
      switch (widthMode) {
        case yoga.MEASURE_MODE_UNDEFINED:
          return Number.POSITIVE_INFINITY;
        case yoga.MEASURE_MODE_EXACTLY:
        case yoga.MEASURE_MODE_AT_MOST:
          return width;
        default:
          return Number.POSITIVE_INFINITY;
      }
    })();

    const textMaxWidthPx = Math.max(0, maxWidthPx - bulletIndentPx);

    const { widthPx, heightPx } = measureText(
      combinedText,
      textMaxWidthPx,
      {
        fontFamily,
        fontSizePx,
        lineHeight,
        fontWeight,
        measurer: ctx.measurer,
      },
      ctx.textMeasurementMode,
    );

    return {
      width: widthPx + bulletIndentPx,
      height: heightPx,
    };
  });
}

export const ulNodeDef: NodeDefinition = {
  type: "ul",
  category: "leaf",
  applyYogaStyle: applyListYogaStyle,
  render(node, ctx) {
    renderUlNode(node as Extract<typeof node, { type: "ul" }>, ctx);
  },
};

export const olNodeDef: NodeDefinition = {
  type: "ol",
  category: "leaf",
  applyYogaStyle: applyListYogaStyle,
  render(node, ctx) {
    renderOlNode(node as Extract<typeof node, { type: "ol" }>, ctx);
  },
};
