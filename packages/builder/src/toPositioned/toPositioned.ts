import type { BuilderNode, PositionedNode } from "../types.ts";
import type { BuildContext } from "../buildContext.ts";
import type { LayoutResultMap } from "../calcYogaLayout/types.ts";
import { getNodeDef } from "../registry/nodeRegistry.ts";

/**
 * Convert a BuilderNode tree into a PositionedNode tree with absolute coordinates.
 * @param pom input BuilderNode
 * @param ctx BuildContext
 * @param map LayoutResultMap — BuilderNode -> computed layout result.
 * @param parentX Parent node's absolute X coordinate.
 * @param parentY Parent node's absolute Y coordinate.
 * @returns The positioned-node tree.
 */
export async function toPositioned(
  pom: BuilderNode,
  ctx: BuildContext,
  map: LayoutResultMap,
  parentX = 0,
  parentY = 0,
): Promise<PositionedNode> {
  const layout = map.get(pom);
  if (!layout) {
    throw new Error("Layout result not found in map for BuilderNode");
  }
  const absoluteX = parentX + layout.left;
  const absoluteY = parentY + layout.top;

  const def = getNodeDef(pom.type);

  // Use the node-specific custom converter when one is defined.
  if (def.toPositioned) {
    return def.toPositioned(pom, absoluteX, absoluteY, layout, ctx, map);
  }

  // Category-based default handling.
  switch (def.category) {
    case "leaf":
      return {
        ...pom,
        x: absoluteX,
        y: absoluteY,
        w: layout.width,
        h: layout.height,
      } as PositionedNode;

    case "multi-child": {
      const containerNode = pom as Extract<
        BuilderNode,
        { type: "vstack" | "hstack" }
      >;
      return {
        ...containerNode,
        x: absoluteX,
        y: absoluteY,
        w: layout.width,
        h: layout.height,
        children: await Promise.all(
          containerNode.children.map((child) =>
            toPositioned(child, ctx, map, absoluteX, absoluteY),
          ),
        ),
      };
    }

    case "absolute-child":
      // absolute-child (layer) must define a custom toPositioned.
      throw new Error(
        `Node type "${pom.type}" with category "absolute-child" must have a custom toPositioned`,
      );
  }
}
