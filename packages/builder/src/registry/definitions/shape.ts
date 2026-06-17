import type { BuilderNode } from "../../types.ts";
import type { NodeDefinition, Yoga } from "../types.ts";
import type { Node as YogaNode } from "yoga-layout";
import { measureText } from "../../calcYogaLayout/measureText.ts";
import type { BuildContext } from "../../buildContext.ts";
import {
  resolveFontFamily,
  resolveTextStyleValue,
} from "../../defaultTextStyle.ts";
import { renderShapeNode } from "../../renderPptx/nodes/shape.ts";

export const shapeNodeDef: NodeDefinition = {
  type: "shape",
  category: "leaf",
  applyYogaStyle(
    node: BuilderNode,
    yn: YogaNode,
    yoga: Yoga,
    ctx: BuildContext,
  ) {
    const n = node as Extract<BuilderNode, { type: "shape" }>;
    if (n.text) {
      const text = n.text;
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
      const lineHeight = resolveTextStyleValue(
        n.lineHeight,
        ctx.defaultTextStyle.lineHeight,
        1.0,
      );

      yn.setMeasureFunc((width, widthMode) => {
        const maxWidthPx = (() => {
          // `noWrap` forces a single-line measurement regardless of mode —
          // the flex layout may overflow horizontally rather than wrap.
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

        const { widthPx, heightPx } = measureText(
          text,
          maxWidthPx,
          {
            fontFamily,
            fontSizePx,
            lineHeight,
            fontWeight,
            measurer: ctx.measurer,
          },
          ctx.textMeasurementMode,
        );

        return { width: widthPx, height: heightPx };
      });
    }
  },
  render(node, ctx) {
    renderShapeNode(node as Extract<typeof node, { type: "shape" }>, ctx);
  },
};
