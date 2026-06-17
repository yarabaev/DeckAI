import type { BuilderNode } from "../../types.ts";
import type { NodeDefinition } from "../types.ts";
import { calcTableIntrinsicSize } from "../../shared/tableUtils.ts";
import { renderTableNode } from "../../renderPptx/nodes/table.ts";

export const tableNodeDef: NodeDefinition = {
  type: "table",
  category: "leaf",
  applyYogaStyle(node, yn) {
    const n = node as Extract<BuilderNode, { type: "table" }>;
    yn.setMeasureFunc(() => {
      const { width, height } = calcTableIntrinsicSize(n);
      return { width, height };
    });
  },
  render(node, ctx) {
    renderTableNode(node as Extract<typeof node, { type: "table" }>, ctx);
  },
};
