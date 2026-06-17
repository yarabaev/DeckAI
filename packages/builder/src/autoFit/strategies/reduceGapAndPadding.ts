import type { BuilderNode } from "../../types.ts";
import { walkPOMTree } from "../../shared/walkTree.ts";

const MIN_GAP = 2;
const MIN_PADDING = 2;
const MIN_SCALE = 0.25;

/**
 * Shrink gap and padding.
 * @returns `true` when something changed.
 */
export function reduceGapAndPadding(
  node: BuilderNode,
  targetRatio: number,
): boolean {
  const ratio = Math.max(targetRatio, MIN_SCALE);
  let changed = false;

  walkPOMTree(node, (n) => {
    // Shrink gap (vstack, hstack).
    if ((n.type === "vstack" || n.type === "hstack") && n.gap !== undefined) {
      const newGap = Math.max(MIN_GAP, Math.round(n.gap * ratio));
      if (newGap !== n.gap) {
        n.gap = newGap;
        changed = true;
      }
    }

    // Shrink padding.
    if (n.padding !== undefined) {
      if (typeof n.padding === "number") {
        const newPadding = Math.max(MIN_PADDING, Math.round(n.padding * ratio));
        if (newPadding !== n.padding) {
          n.padding = newPadding;
          changed = true;
        }
      } else {
        const dirs = ["top", "right", "bottom", "left"] as const;
        for (const dir of dirs) {
          const val = n.padding[dir];
          if (val !== undefined) {
            const newVal = Math.max(MIN_PADDING, Math.round(val * ratio));
            if (newVal !== val) {
              n.padding[dir] = newVal;
              changed = true;
            }
          }
        }
      }
    }
  });

  return changed;
}
