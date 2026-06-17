import type { NodeDefinition } from "../types.ts";
import { renderChartNode } from "../../renderPptx/nodes/chart.ts";

export const chartNodeDef: NodeDefinition = {
  type: "chart",
  category: "leaf",
  render(node, ctx) {
    renderChartNode(node as Extract<typeof node, { type: "chart" }>, ctx);
  },
};
