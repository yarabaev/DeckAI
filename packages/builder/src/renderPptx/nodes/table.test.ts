import { describe, expect, it, vi } from "vitest";
import { renderTableNode } from "./table.ts";

describe("renderTableNode", () => {
  it("テーブルセルの verticalAlign を反映し、未指定時は middle を使う", () => {
    const addTable = vi.fn();

    renderTableNode(
      {
        type: "table",
        x: 0,
        y: 0,
        w: 200,
        h: 64,
        columns: [{ width: 100 }, { width: 100 }],
        rows: [
          {
            cells: [
              { text: "Default middle" },
              { text: "Bottom", verticalAlign: "bottom" },
            ],
          },
          {
            cells: [
              {
                text: "Top",
                runs: [{ text: "Top" }],
                verticalAlign: "top",
              },
              { text: "Middle", runs: [{ text: "Middle" }] },
            ],
          },
        ],
      },
      {
        slide: { addTable } as never,
        pptx: {} as never,
        buildContext: {} as never,
      },
    );

    expect(addTable).toHaveBeenCalledTimes(1);

    const [tableRows] = addTable.mock.calls[0] as [unknown[]];
    const rows = tableRows as Array<Array<{ options: { valign?: string } }>>;

    expect(rows[0][0].options.valign).toBe("middle");
    expect(rows[0][1].options.valign).toBe("bottom");
    expect(rows[1][0].options.valign).toBe("top");
    expect(rows[1][1].options.valign).toBe("middle");
  });
});
