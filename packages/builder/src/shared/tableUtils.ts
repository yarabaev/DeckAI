import type { TableNode } from "../types.ts";

const DEFAULT_TABLE_ROW_HEIGHT = 32;
const DEFAULT_TABLE_COLUMN_WIDTH = 100;

const PERCENT_RE = /^(\d+(?:\.\d+)?)%$/;

/**
 * Resolve a column-width atom against the containing table width.
 *
 * Supported forms:
 *   - number               returned as-is
 *   - "NN%" / "NN.NN%"     resolved against `tableWidth` (or falls through to
 *                          `fallback` if `tableWidth` is unknown, e.g. during
 *                          intrinsic-size calculation)
 *   - "max"                treated as unspecified for sizing — the column
 *                          takes an equal share of the remaining slack
 *                          alongside other unspecified columns
 *   - undefined            uses `fallback`
 */
function numericWidth(
  w: string | number | undefined,
  fallback: number,
  tableWidth?: number,
): number {
  if (typeof w === "number") return w;
  if (typeof w === "string") {
    const m = PERCENT_RE.exec(w);
    if (m && m[1] && tableWidth !== undefined) {
      return (parseFloat(m[1]) / 100) * tableWidth;
    }
  }
  return fallback;
}

/**
 * Whether a column-width atom contributes a "specified" amount to the
 * specifiedTotal sum. Numbers and percentage strings are specified;
 * `undefined` and `"max"` are not (they share the remaining slack).
 */
function isSpecifiedColumnWidth(w: string | number | undefined): boolean {
  if (typeof w === "number") return true;
  if (typeof w === "string" && PERCENT_RE.test(w)) return true;
  return false;
}

export function calcTableIntrinsicSize(node: TableNode) {
  // tableWidth is unknown at intrinsic-measurement time, so percentage
  // columns fall back to DEFAULT_TABLE_COLUMN_WIDTH here. Use absolute
  // numeric widths if an exact intrinsic estimate matters.
  const width = node.columns.reduce(
    (sum, column) =>
      sum + numericWidth(column.w ?? column.width, DEFAULT_TABLE_COLUMN_WIDTH),
    0,
  );
  const height = resolveRowHeights(node).reduce((sum, h) => sum + h, 0);

  return { width, height };
}

export function resolveRowHeights(node: TableNode) {
  const fallbackRowHeight = node.defaultRowHeight ?? DEFAULT_TABLE_ROW_HEIGHT;
  return node.rows.map((row) => {
    const h = row.h ?? row.height;
    return typeof h === "number" ? h : fallbackRowHeight;
  });
}

/**
 * Resolves the width of each table column.
 *
 * - Columns with a numeric width (`<Col w="120"/>`) use that value.
 * - Columns with a percentage width (`<Col w="30%"/>`) resolve against
 *   the `tableWidth` argument.
 * - Columns without a width (or `w="max"`) share the remaining space
 *   equally — matching the flex-grow:1 idiom used elsewhere in the API.
 */
export function resolveColumnWidths(
  node: TableNode,
  tableWidth: number,
): number[] {
  const specifiedTotal = node.columns.reduce(
    (sum, col) => sum + numericWidth(col.w ?? col.width, 0, tableWidth),
    0,
  );
  const unspecifiedCount = node.columns.filter(
    (col) => !isSpecifiedColumnWidth(col.w ?? col.width),
  ).length;

  // Calculate remaining width for columns without explicit widths
  const remainingWidth = Math.max(0, tableWidth - specifiedTotal);
  const widthPerUnspecified =
    unspecifiedCount > 0 ? remainingWidth / unspecifiedCount : 0;

  return node.columns.map((col) =>
    numericWidth(col.w ?? col.width, widthPerUnspecified, tableWidth),
  );
}
