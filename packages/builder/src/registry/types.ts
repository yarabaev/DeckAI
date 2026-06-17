import type { BuilderNode, PositionedNode } from "../types.ts";
import type { Node as YogaNode } from "yoga-layout";
import type { RenderContext } from "../renderPptx/types.ts";
import type { loadYoga } from "yoga-layout/load";
import type { BuildContext } from "../buildContext.ts";
import type { LayoutResultMap } from "../calcYogaLayout/types.ts";

export type Yoga = Awaited<ReturnType<typeof loadYoga>>;

/** Node category. Determines how child elements are handled. */
export type NodeCategory =
  | "leaf" // no child elements
  | "multi-child" // multiple children (vstack, hstack)
  | "absolute-child"; // multiple children, absolutely placed (layer)

export interface NodeDefinition {
  /** Node type name. */
  type: BuilderNode["type"];

  /** Node category. */
  category: NodeCategory;

  /** Apply node-specific style / measureFunc onto a YogaNode. */
  applyYogaStyle?: (
    node: BuilderNode,
    yn: YogaNode,
    yoga: Yoga,
    ctx: BuildContext,
  ) => void | Promise<void>;

  /** Custom BuilderNode -> PositionedNode conversion (falls back to the category default). */
  toPositioned?: (
    pom: BuilderNode,
    absoluteX: number,
    absoluteY: number,
    layout: { width: number; height: number },
    ctx: BuildContext,
    map: LayoutResultMap,
  ) => PositionedNode | Promise<PositionedNode>;

  /** Render a PositionedNode onto the slide (leaf-node hook). */
  render?: (node: PositionedNode, ctx: RenderContext) => void;

  /** Collect image sources (used by the prefetcher). */
  collectImageSources?: (node: BuilderNode) => string[];
}
