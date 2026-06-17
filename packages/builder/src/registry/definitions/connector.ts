import type { BuilderNode } from "../../types.ts";
import type { NodeDefinition } from "../types.ts";
import { renderConnectorNode } from "../../renderPptx/nodes/connector.ts";

export const connectorNodeDef: NodeDefinition = {
  type: "connector",
  category: "leaf",
  applyYogaStyle(_node, yn) {
    // Connector nodes do not participate in flexbox layout. Their real
    // geometry is derived render-side from the from/to shapes' positioned
    // boxes, so the Yoga node is a zero-size leaf that contributes
    // nothing to its parent's main-axis size or cross-axis stretching.
    yn.setWidth(0);
    yn.setHeight(0);
    yn.setFlexShrink(0);
    yn.setFlexGrow(0);
  },
  toPositioned(pom, _absoluteX, _absoluteY, _layout) {
    const n = pom as Extract<BuilderNode, { type: "connector" }>;
    // The bbox here is a placeholder; the renderer computes the real
    // geometry from the slide-level id-index. We still need to satisfy
    // the PositionedBase shape (x, y, w, h).
    return { ...n, x: 0, y: 0, w: 0, h: 0 };
  },
  render(node, ctx) {
    renderConnectorNode(
      node as Extract<typeof node, { type: "connector" }>,
      ctx,
    );
  },
};
