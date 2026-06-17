import type { BuilderNode } from "../../types.ts";
import { walkPOMTree } from "../../shared/walkTree.ts";

const MIN_ROW_HEIGHT = 20;
const MIN_SCALE = 0.5;

/**
 * Shrink `table.defaultRowHeight` and per-row heights.
 * @returns `true` when something changed.
 */
export function reduceTableRowHeight(
  node: BuilderNode,
  targetRatio: number,
): boolean {
  const ratio = Math.max(targetRatio, MIN_SCALE);
  let changed = false;

  walkPOMTree(node, (n) => {
    if (n.type !== "table") return;

    if (n.defaultRowHeight !== undefined) {
      const newHeight = Math.max(
        MIN_ROW_HEIGHT,
        Math.round(n.defaultRowHeight * ratio),
      );
      if (newHeight !== n.defaultRowHeight) {
        n.defaultRowHeight = newHeight;
        changed = true;
      }
    }

    for (const row of n.rows) {
      const h = row.h ?? row.height;
      if (typeof h === "number") {
        const newHeight = Math.max(MIN_ROW_HEIGHT, Math.round(h * ratio));
        if (newHeight !== h) {
          row.h = newHeight;
          delete (row as { height?: number }).height;
          changed = true;
        }
      }
    }
  });

  return changed;
}
