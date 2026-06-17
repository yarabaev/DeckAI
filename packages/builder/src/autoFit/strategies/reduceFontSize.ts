import type { BuilderNode } from "../../types.ts";
import { walkPOMTree } from "../../shared/walkTree.ts";

const MIN_FONT_SIZE = 10;
const MIN_SCALE = 0.6;

/**
 * Shrink `fontSize` on text-bearing nodes.
 * Affects: text, ul, ol, shape.
 * @returns `true` when something changed.
 */
export function reduceFontSize(
  node: BuilderNode,
  targetRatio: number,
): boolean {
  const ratio = Math.max(targetRatio, MIN_SCALE);
  let changed = false;

  walkPOMTree(node, (n) => {
    switch (n.type) {
      case "text":
      case "shape":
      case "ul":
      case "ol": {
        if (n.fontSize !== undefined) {
          const newSize = Math.max(
            MIN_FONT_SIZE,
            Math.round(n.fontSize * ratio),
          );
          if (newSize !== n.fontSize) {
            n.fontSize = newSize;
            changed = true;
          }
        }
        break;
      }
    }

    // Also shrink fontSize on ul/ol <li> elements.
    if (n.type === "ul" || n.type === "ol") {
      for (const item of n.items) {
        if (item.fontSize !== undefined) {
          const newSize = Math.max(
            MIN_FONT_SIZE,
            Math.round(item.fontSize * ratio),
          );
          if (newSize !== item.fontSize) {
            item.fontSize = newSize;
            changed = true;
          }
        }
      }
    }

    // Also shrink fontSize on table cells.
    if (n.type === "table") {
      for (const row of n.rows) {
        for (const cell of row.cells) {
          if (cell.fontSize !== undefined) {
            const newSize = Math.max(
              MIN_FONT_SIZE,
              Math.round(cell.fontSize * ratio),
            );
            if (newSize !== cell.fontSize) {
              cell.fontSize = newSize;
              changed = true;
            }
          }
        }
      }
    }
  });

  return changed;
}
