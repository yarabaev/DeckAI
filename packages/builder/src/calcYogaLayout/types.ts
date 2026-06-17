import type { Node as YogaNode } from "yoga-layout";
import type { BuilderNode } from "../types.ts";

/** Mapping from each BuilderNode to its supporting YogaNode. Exists only during the layout-compute phase. */
export type YogaNodeMap = Map<BuilderNode, YogaNode>;

/** Lightweight representation of a Yoga computation result. No runtime dependency on Yoga itself. */
export interface LayoutResult {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Mapping from each BuilderNode to its computed layout result. */
export type LayoutResultMap = Map<BuilderNode, LayoutResult>;

/** Extract the computed layout result from a YogaNodeMap. */
export function extractLayoutResults(yogaMap: YogaNodeMap): LayoutResultMap {
  const layoutMap: LayoutResultMap = new Map();
  for (const [builderNode, yogaNode] of yogaMap) {
    const computed = yogaNode.getComputedLayout();
    layoutMap.set(builderNode, {
      left: computed.left,
      top: computed.top,
      width: computed.width,
      height: computed.height,
    });
  }
  return layoutMap;
}
