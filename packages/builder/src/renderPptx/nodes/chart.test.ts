import { describe, expect, it, vi } from "vitest";
import { renderChartNode } from "./chart.ts";

describe("renderChartNode", () => {
  it("passes altText to addChart when provided", () => {
    const addChart = vi.fn();

    renderChartNode(
      {
        type: "chart",
        x: 0,
        y: 0,
        w: 400,
        h: 300,
        chartType: "bar",
        data: [{ name: "Series 1", labels: ["A", "B"], values: [1, 2] }],
        altText: "Bar chart showing results",
      } as never,
      {
        slide: { addChart } as never,
        pptx: {} as never,
        buildContext: {} as never,
      },
    );

    expect(addChart).toHaveBeenCalledTimes(1);
    const callArgs = addChart.mock.calls[0] as [
      string,
      unknown[],
      Record<string, unknown>,
    ];
    expect(callArgs[2].altText).toBe("Bar chart showing results");
  });

  it("passes showValue, barGrouping, valAxisMinVal, valAxisMaxVal to addChart", () => {
    const addChart = vi.fn();

    renderChartNode(
      {
        type: "chart",
        x: 0,
        y: 0,
        w: 400,
        h: 300,
        chartType: "bar",
        data: [{ name: "S1", labels: ["A"], values: [1] }],
        showValue: true,
        barGrouping: "stacked",
        valAxisMinVal: 0,
        valAxisMaxVal: 100,
      } as never,
      {
        slide: { addChart } as never,
        pptx: {} as never,
        buildContext: {} as never,
      },
    );

    expect(addChart).toHaveBeenCalledTimes(1);
    const opts = addChart.mock.calls[0][2] as Record<string, unknown>;
    expect(opts.showValue).toBe(true);
    expect(opts.barGrouping).toBe("stacked");
    expect(opts.valAxisMinVal).toBe(0);
    expect(opts.valAxisMaxVal).toBe(100);
  });

  it("does not include altText when not provided", () => {
    const addChart = vi.fn();

    renderChartNode(
      {
        type: "chart",
        x: 0,
        y: 0,
        w: 400,
        h: 300,
        chartType: "bar",
        data: [{ name: "Series 1", labels: ["A", "B"], values: [1, 2] }],
      } as never,
      {
        slide: { addChart } as never,
        pptx: {} as never,
        buildContext: {} as never,
      },
    );

    expect(addChart).toHaveBeenCalledTimes(1);
    const callArgs = addChart.mock.calls[0] as [
      string,
      unknown[],
      Record<string, unknown>,
    ];
    expect(callArgs[2].altText).toBeUndefined();
  });
});
