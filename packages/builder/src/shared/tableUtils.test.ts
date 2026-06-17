import { describe, expect, it } from "vitest";
import {
  calcTableIntrinsicSize,
  resolveColumnWidths,
  resolveRowHeights,
} from "./tableUtils.ts";
import type { TableNode } from "../types.ts";

function makeTable(overrides: Partial<TableNode> = {}): TableNode {
  return {
    type: "table",
    columns: [],
    rows: [],
    ...overrides,
  };
}

describe("calcTableIntrinsicSize", () => {
  it("reads column.w (canonical) for width calculation", () => {
    const table = makeTable({ columns: [{ w: 150 }, { w: 200 }], rows: [] });
    const { width } = calcTableIntrinsicSize(table);
    expect(width).toBe(350);
  });

  it("falls back to column.width (legacy) when column.w is absent", () => {
    const table = makeTable({ columns: [{ width: 120 }], rows: [] });
    const { width } = calcTableIntrinsicSize(table);
    expect(width).toBe(120);
  });

  it("prefers column.w over column.width when both present", () => {
    const table = makeTable({ columns: [{ w: 200, width: 120 }], rows: [] });
    const { width } = calcTableIntrinsicSize(table);
    expect(width).toBe(200);
  });
});

describe("resolveRowHeights", () => {
  it("reads row.h (canonical) for height calculation", () => {
    const table = makeTable({
      columns: [],
      rows: [
        { cells: [], h: 80 },
        { cells: [], h: 60 },
      ],
    });
    const heights = resolveRowHeights(table);
    expect(heights).toEqual([80, 60]);
  });

  it("falls back to row.height (legacy) when row.h is absent", () => {
    const table = makeTable({
      columns: [],
      rows: [{ cells: [], height: 50 }],
    });
    const heights = resolveRowHeights(table);
    expect(heights).toEqual([50]);
  });

  it("prefers row.h over row.height when both present", () => {
    const table = makeTable({
      columns: [],
      rows: [{ cells: [], h: 80, height: 50 }],
    });
    const heights = resolveRowHeights(table);
    expect(heights).toEqual([80]);
  });

  it("uses defaultRowHeight fallback when neither h nor height is set", () => {
    const table = makeTable({
      columns: [],
      rows: [{ cells: [] }],
      defaultRowHeight: 40,
    });
    const heights = resolveRowHeights(table);
    expect(heights).toEqual([40]);
  });
});

describe("resolveColumnWidths", () => {
  it("reads col.w (canonical) for specified column widths", () => {
    const table = makeTable({
      columns: [{ w: 150 }, { w: 200 }],
      rows: [],
    });
    const widths = resolveColumnWidths(table, 350);
    expect(widths).toEqual([150, 200]);
  });

  it("falls back to col.width (legacy) for specified column widths", () => {
    const table = makeTable({
      columns: [{ width: 120 }, { width: 80 }],
      rows: [],
    });
    const widths = resolveColumnWidths(table, 200);
    expect(widths).toEqual([120, 80]);
  });

  it("distributes remaining width to columns with no w or width", () => {
    const table = makeTable({
      columns: [{ w: 100 }, {}],
      rows: [],
    });
    const widths = resolveColumnWidths(table, 300);
    expect(widths).toEqual([100, 200]);
  });
});
