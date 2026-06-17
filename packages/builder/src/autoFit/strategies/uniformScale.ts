import type { BuilderNode } from "../../types.ts";
import { walkPOMTree } from "../../shared/walkTree.ts";

const MIN_SCALE = 0.5;

function scaleNumber(value: number, ratio: number, min: number): number {
  return Math.max(min, Math.round(value * ratio));
}

/**
 * Fallback strategy: scale every size-related property uniformly.
 * @returns `true` when something changed.
 */
export function uniformScale(node: BuilderNode, targetRatio: number): boolean {
  const ratio = Math.max(targetRatio, MIN_SCALE);
  let changed = false;

  walkPOMTree(node, (n) => {
    // fontSize
    if ("fontSize" in n && typeof n.fontSize === "number") {
      const newVal = scaleNumber(n.fontSize, ratio, 8);
      if (newVal !== n.fontSize) {
        (n as { fontSize: number }).fontSize = newVal;
        changed = true;
      }
    }

    // gap (vstack/hstack)
    if ((n.type === "vstack" || n.type === "hstack") && n.gap !== undefined) {
      const newVal = scaleNumber(n.gap, ratio, 1);
      if (newVal !== n.gap) {
        n.gap = newVal;
        changed = true;
      }
    }

    // padding
    if (n.padding !== undefined) {
      if (typeof n.padding === "number") {
        const newVal = scaleNumber(n.padding, ratio, 1);
        if (newVal !== n.padding) {
          n.padding = newVal;
          changed = true;
        }
      } else {
        const dirs = ["top", "right", "bottom", "left"] as const;
        for (const dir of dirs) {
          const val = n.padding[dir];
          if (val !== undefined) {
            const newVal = scaleNumber(val, ratio, 1);
            if (newVal !== val) {
              n.padding[dir] = newVal;
              changed = true;
            }
          }
        }
      }
    }

    // table: defaultRowHeight, row.height
    if (n.type === "table") {
      if (n.defaultRowHeight !== undefined) {
        const newVal = scaleNumber(n.defaultRowHeight, ratio, 16);
        if (newVal !== n.defaultRowHeight) {
          n.defaultRowHeight = newVal;
          changed = true;
        }
      }
      for (const row of n.rows) {
        const h = row.h ?? row.height;
        if (typeof h === "number") {
          const newVal = scaleNumber(h, ratio, 16);
          if (newVal !== h) {
            row.h = newVal;
            delete (row as { height?: number }).height;
            changed = true;
          }
        }
      }
    }

    // ul/ol items fontSize
    if (n.type === "ul" || n.type === "ol") {
      for (const item of n.items) {
        if (item.fontSize !== undefined) {
          const newVal = scaleNumber(item.fontSize, ratio, 8);
          if (newVal !== item.fontSize) {
            item.fontSize = newVal;
            changed = true;
          }
        }
      }
    }

    // icon size
    if (n.type === "icon" && n.size !== undefined) {
      const newVal = scaleNumber(n.size, ratio, 8);
      if (newVal !== n.size) {
        n.size = newVal;
        changed = true;
      }
    }

    // table cells fontSize
    if (n.type === "table") {
      for (const row of n.rows) {
        for (const cell of row.cells) {
          if (cell.fontSize !== undefined) {
            const newVal = scaleNumber(cell.fontSize, ratio, 8);
            if (newVal !== cell.fontSize) {
              cell.fontSize = newVal;
              changed = true;
            }
          }
        }
      }
    }
  });

  return changed;
}
