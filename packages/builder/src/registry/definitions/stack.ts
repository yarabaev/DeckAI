import type { BuilderNode, VStackNode, HStackNode } from "../../types.ts";
import type { NodeDefinition, Yoga } from "../types.ts";
import type { Node as YogaNode } from "yoga-layout";

/**
 * Apply Flex properties common to vstack/hstack.
 */
function applyFlexProperties(
  node: VStackNode | HStackNode,
  yn: YogaNode,
  yoga: Yoga,
): void {
  if (node.gap !== undefined) {
    yn.setGap(yoga.GUTTER_ROW, node.gap);
    yn.setGap(yoga.GUTTER_COLUMN, node.gap);
  }

  if (node.alignItems !== undefined) {
    switch (node.alignItems) {
      case "start":
        yn.setAlignItems(yoga.ALIGN_FLEX_START);
        break;
      case "center":
        yn.setAlignItems(yoga.ALIGN_CENTER);
        break;
      case "end":
        yn.setAlignItems(yoga.ALIGN_FLEX_END);
        break;
      case "stretch":
        yn.setAlignItems(yoga.ALIGN_STRETCH);
        break;
      case "baseline":
        // Yoga supports baseline alignment natively. Without a custom
        // baseline function on the measure node, Yoga falls back to the
        // bottom edge of each child's content box; for slideglance text
        // rows that is close to the visual baseline at the leaf measure
        // node's lineHeight=1.0 default. The mixed-size editorial row
        // idiom (`textVAlign="middle" lineHeight="1.0"`) remains the
        // pixel-perfect path; this enum value is the CSS-natural alias.
        yn.setAlignItems(yoga.ALIGN_BASELINE);
        break;
    }
  }

  if (node.justifyContent !== undefined) {
    switch (node.justifyContent) {
      case "start":
        yn.setJustifyContent(yoga.JUSTIFY_FLEX_START);
        break;
      case "center":
        yn.setJustifyContent(yoga.JUSTIFY_CENTER);
        break;
      case "end":
        yn.setJustifyContent(yoga.JUSTIFY_FLEX_END);
        break;
      case "spaceBetween":
        yn.setJustifyContent(yoga.JUSTIFY_SPACE_BETWEEN);
        break;
      case "spaceAround":
        yn.setJustifyContent(yoga.JUSTIFY_SPACE_AROUND);
        break;
      case "spaceEvenly":
        yn.setJustifyContent(yoga.JUSTIFY_SPACE_EVENLY);
        break;
    }
  }

  if (node.flexWrap !== undefined) {
    switch (node.flexWrap) {
      case "nowrap":
        yn.setFlexWrap(yoga.WRAP_NO_WRAP);
        break;
      case "wrap":
        yn.setFlexWrap(yoga.WRAP_WRAP);
        break;
      case "wrapReverse":
        yn.setFlexWrap(yoga.WRAP_WRAP_REVERSE);
        break;
    }
  }
}

export const vstackNodeDef: NodeDefinition = {
  type: "vstack",
  category: "multi-child",
  applyYogaStyle(node: BuilderNode, yn: YogaNode, yoga: Yoga) {
    yn.setFlexDirection(yoga.FLEX_DIRECTION_COLUMN);
    applyFlexProperties(node as VStackNode, yn, yoga);
  },
  // render: handle child-element recursion based on category.
};

export const hstackNodeDef: NodeDefinition = {
  type: "hstack",
  category: "multi-child",
  applyYogaStyle(node: BuilderNode, yn: YogaNode, yoga: Yoga) {
    yn.setFlexDirection(yoga.FLEX_DIRECTION_ROW);
    applyFlexProperties(node as HStackNode, yn, yoga);
  },
  // render: handle child-element recursion based on category.
};
