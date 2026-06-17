import type { BuilderNode } from "../../types.ts";
import type { NodeDefinition } from "../types.ts";
import { renderLineNode } from "../../renderPptx/nodes/line.ts";

export const lineNodeDef: NodeDefinition = {
  type: "line",
  category: "leaf",
  applyYogaStyle(_node, yn) {
    // Line nodes use absolute coordinates, so treat their Yoga layout size as 0.
    yn.setWidth(0);
    yn.setHeight(0);
  },
  toPositioned(pom, _absoluteX, _absoluteY, _layout) {
    const n = pom as Extract<BuilderNode, { type: "line" }>;
    // Line nodes carry absolute coordinates (x1, y1, x2, y2), so
    // The yogaNode has no coordinate; compute the bounding box from the node's own coordinates.
    return {
      ...n,
      x: Math.min(n.x1, n.x2),
      y: Math.min(n.y1, n.y2),
      w: Math.abs(n.x2 - n.x1),
      h: Math.abs(n.y2 - n.y1),
    };
  },
  render(node, ctx) {
    renderLineNode(node as Extract<typeof node, { type: "line" }>, ctx);
  },
};
