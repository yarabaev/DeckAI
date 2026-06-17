import type { PositionedNode } from "../../types.ts";
import type { RenderContext } from "../types.ts";
import { pxToIn } from "../units.ts";
import { getContentArea } from "../utils/contentArea.ts";
import { renderObjectName } from "../utils/objectName.ts";

type ChartPositionedNode = Extract<PositionedNode, { type: "chart" }>;

export function renderChartNode(
  node: ChartPositionedNode,
  ctx: RenderContext,
): void {
  const chartData = node.data.map((d) => ({
    name: d.name,
    labels: d.labels,
    values: d.values,
  }));

  const content = getContentArea(node);
  const objectName = renderObjectName(node, ctx);
  const chartOptions: Record<string, unknown> = {
    x: pxToIn(content.x),
    y: pxToIn(content.y),
    w: pxToIn(content.w),
    h: pxToIn(content.h),
    ...(objectName ? { objectName } : {}),
    ...(node.isDecorative
      ? { altText: "" }
      : node.altText !== undefined
        ? { altText: node.altText }
        : {}),
    showLegend: node.showLegend ?? false,
    showTitle: node.showTitle ?? false,
    title: node.title,
    chartColors: node.chartColors,
    legendPos: node.legendPos ?? "r",
    legendFontSize: node.legendFontSize ?? 12,
    catAxisLabelFontSize: node.catAxisLabelFontSize ?? 12,
    valAxisLabelFontSize: node.valAxisLabelFontSize ?? 12,
    barGapWidthPct: node.barGapWidthPct ?? 100,
    lineDataSymbolSize: node.lineDataSymbolSize ?? 8,
    ...(node.showValue !== undefined ? { showValue: node.showValue } : {}),
    ...(node.barGrouping !== undefined
      ? { barGrouping: node.barGrouping }
      : {}),
    ...(node.valAxisMinVal !== undefined
      ? { valAxisMinVal: node.valAxisMinVal }
      : {}),
    ...(node.valAxisMaxVal !== undefined
      ? { valAxisMaxVal: node.valAxisMaxVal }
      : {}),
  };

  // Radar-only option
  if (node.chartType === "radar" && node.radarStyle) {
    chartOptions.radarStyle = node.radarStyle;
  }

  ctx.slide.addChart(node.chartType, chartData, chartOptions);
}
