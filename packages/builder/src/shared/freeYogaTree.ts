import type { YogaNodeMap } from "../calcYogaLayout/types.ts";

/**
 * Release every YogaNode held by the YogaNodeMap.
 * Must be called before `calcYogaLayout` runs again.
 */
export function freeYogaTree(map: YogaNodeMap): void {
  // Map insertion order is parent -> child; release in reverse (leaf-first).
  const yogaNodes = Array.from(map.values());

  for (let i = yogaNodes.length - 1; i >= 0; i--) {
    const yn = yogaNodes[i];
    if (!yn) continue;
    // Detach from the parent before freeing.
    const owner = yn.getParent();
    if (owner) {
      const childCount = owner.getChildCount();
      for (let j = 0; j < childCount; j++) {
        if (owner.getChild(j) === yn) {
          owner.removeChild(yn);
          break;
        }
      }
    }
    yn.free();
  }

  map.clear();
}
