import type { BuilderNode } from "../../types.ts";
import type { NodeDefinition, Yoga } from "../types.ts";
import type { Node as YogaNode } from "yoga-layout";
import { measureText } from "../../calcYogaLayout/measureText.ts";
import type { BuildContext } from "../../buildContext.ts";
import {
  resolveFontFamily,
  resolveTextStyleValue,
} from "../../defaultTextStyle.ts";
import { renderTextNode } from "../../renderPptx/nodes/text.ts";

export const textNodeDef: NodeDefinition = {
  type: "text",
  category: "leaf",
  applyYogaStyle(
    node: BuilderNode,
    yn: YogaNode,
    yoga: Yoga,
    ctx: BuildContext,
  ) {
    const n = node as Extract<BuilderNode, { type: "text" }>;
    const text = n.text;
    const fontSizePx = resolveTextStyleValue(
      n.fontSize,
      ctx.defaultTextStyle.fontSize,
      24,
    );
    const fontFamily = resolveFontFamily(n.fontFamily, ctx.defaultTextStyle);
    const nodeBold = resolveTextStyleValue(
      n.bold,
      ctx.defaultTextStyle.bold,
      false,
    );
    // Measure at bold weight when any inline run is bold so that wrap width
    // reflects the actually rendered glyph widths. Otherwise wrap is computed
    // at normal weight while render uses bold for those runs, causing the
    // measured 1-line box to overflow into the parent's padding on 2-line
    // edge cases.
    const hasBoldRun = n.runs?.some((r) => r.bold) ?? false;
    const fontWeight: "bold" | "normal" =
      nodeBold || hasBoldRun ? "bold" : "normal";
    const lineHeight = resolveTextStyleValue(
      n.lineHeight,
      ctx.defaultTextStyle.lineHeight,
      1.0,
    );

    yn.setMeasureFunc((width, widthMode) => {
      const maxWidthPx = (() => {
        // `noWrap` forces a single-line measurement regardless of the
        // mode. The flex layout sees the natural width and may overflow
        // horizontally rather than wrap the glyph run.
        if (n.noWrap) return Number.POSITIVE_INFINITY;
        switch (widthMode) {
          case yoga.MEASURE_MODE_UNDEFINED:
            return Number.POSITIVE_INFINITY;
          case yoga.MEASURE_MODE_EXACTLY:
          case yoga.MEASURE_MODE_AT_MOST:
            // Yoga occasionally probes with width=0 during its
            // intrinsic-sizing pass. Treating that literally forces
            // every whitespace token onto its own line and emits a
            // bogus "tall + narrow" intrinsic size that the
            // subsequent constrained pass never overwrites for
            // height. Fall back to the unconstrained natural width
            // so yoga sees the real intrinsic size.
            if (!width || width <= 0) return Number.POSITIVE_INFINITY;
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
  },
  render(node, ctx) {
    renderTextNode(node as Extract<typeof node, { type: "text" }>, ctx);
  },
};
