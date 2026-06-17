import type { BuilderNode, PositionedLayerChild } from "../../types.ts";
import type { NodeDefinition } from "../types.ts";
import { toPositioned } from "../../toPositioned/toPositioned.ts";

export const layerNodeDef: NodeDefinition = {
  type: "layer",
  category: "absolute-child",
  // applyYogaStyle: a layer is a container with absolutely-placed children. Its size is expected to be explicit.
  async toPositioned(pom, absoluteX, absoluteY, layout, ctx, map) {
    const n = pom as Extract<BuilderNode, { type: "layer" }>;
    // A layer's child carries its own relative coordinates (child.x, child.y) inside the layer.
    // Add the layer's absolute coordinate to convert into a slide-absolute coordinate.
    return {
      ...n,
      x: absoluteX,
      y: absoluteY,
      w: layout.width,
      h: layout.height,
      children: await Promise.all(
        n.children.map(async (child): Promise<PositionedLayerChild> => {
          const childX = child.x ?? 0;
          const childY = child.y ?? 0;

          // Line nodes require special handling.
          // Treat x1, y1, x2, y2 as layer-relative; offset by the layer's coordinate.
          if (child.type === "line") {
            const lineAbsoluteX = absoluteX + childX;
            const lineAbsoluteY = absoluteY + childY;
            const adjustedX1 = child.x1 + lineAbsoluteX;
            const adjustedY1 = child.y1 + lineAbsoluteY;
            const adjustedX2 = child.x2 + lineAbsoluteX;
            const adjustedY2 = child.y2 + lineAbsoluteY;

            return {
              ...child,
              x1: adjustedX1,
              y1: adjustedY1,
              x2: adjustedX2,
              y2: adjustedY2,
              x: Math.min(adjustedX1, adjustedX2),
              y: Math.min(adjustedY1, adjustedY2),
              w: Math.abs(adjustedX2 - adjustedX1),
              h: Math.abs(adjustedY2 - adjustedY1),
            };
          }

          // Default handling for other node types.
          const childLayout = map.get(child);
          if (!childLayout) {
            throw new Error("Layout result not found in map for layer child");
          }
          const adjustedParentX = absoluteX + childX - childLayout.left;
          const adjustedParentY = absoluteY + childY - childLayout.top;

          return await toPositioned(
            child,
            ctx,
            map,
            adjustedParentX,
            adjustedParentY,
          );
        }),
      ),
    };
  },
  // render: handle child-element recursion based on category.
};
