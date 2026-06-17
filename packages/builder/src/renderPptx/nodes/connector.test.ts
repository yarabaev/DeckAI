import { describe, expect, it } from "vitest";
import { autoSidePair, pickConnectorPreset, sideAnchor } from "./connector.ts";
import type { SlideBBox } from "../types.ts";

const A: SlideBBox = { x: 0, y: 0, w: 100, h: 60, prst: "rect" };
const B_right: SlideBBox = { x: 300, y: 20, w: 100, h: 60, prst: "rect" };
const B_left: SlideBBox = { x: -300, y: 20, w: 100, h: 60, prst: "rect" };
const B_below: SlideBBox = { x: 20, y: 200, w: 100, h: 60, prst: "rect" };
const B_above: SlideBBox = { x: 20, y: -200, w: 100, h: 60, prst: "rect" };

describe("autoSidePair", () => {
  it("right neighbour -> right / left", () => {
    expect(autoSidePair(A, B_right)).toEqual({
      fromSide: "right",
      toSide: "left",
    });
  });
  it("left neighbour -> left / right", () => {
    expect(autoSidePair(A, B_left)).toEqual({
      fromSide: "left",
      toSide: "right",
    });
  });
  it("below neighbour -> bottom / top", () => {
    expect(autoSidePair(A, B_below)).toEqual({
      fromSide: "bottom",
      toSide: "top",
    });
  });
  it("above neighbour -> top / bottom", () => {
    expect(autoSidePair(A, B_above)).toEqual({
      fromSide: "top",
      toSide: "bottom",
    });
  });
  it("diagonals: dominant axis decides (ties prefer horizontal)", () => {
    const tied: SlideBBox = { x: 200, y: 200, w: 100, h: 60, prst: "rect" };
    // dx > 0, dy > 0, |dx|==|dy| after center math -> horizontal wins.
    expect(autoSidePair(A, tied).fromSide).toBe("right");
  });
});

describe("pickConnectorPreset", () => {
  it("straight is geometry-agnostic", () => {
    expect(pickConnectorPreset("straight", "right", "left")).toBe(
      "straightConnector1",
    );
    expect(pickConnectorPreset("straight", "top", "top")).toBe(
      "straightConnector1",
    );
  });

  it("elbow: facing sides -> bentConnector3 (2 bends, Z shape)", () => {
    expect(pickConnectorPreset("elbow", "right", "left")).toBe(
      "bentConnector3",
    );
    expect(pickConnectorPreset("elbow", "top", "bottom")).toBe(
      "bentConnector3",
    );
  });

  it("elbow: perpendicular sides -> bentConnector2 (1 bend, L shape)", () => {
    expect(pickConnectorPreset("elbow", "right", "top")).toBe("bentConnector2");
    expect(pickConnectorPreset("elbow", "left", "bottom")).toBe(
      "bentConnector2",
    );
    expect(pickConnectorPreset("elbow", "top", "right")).toBe("bentConnector2");
  });

  it("elbow: same side -> bentConnector4 (3 bends, U shape)", () => {
    expect(pickConnectorPreset("elbow", "right", "right")).toBe(
      "bentConnector4",
    );
    expect(pickConnectorPreset("elbow", "top", "top")).toBe("bentConnector4");
  });

  it("curved family mirrors the bent N picker", () => {
    expect(pickConnectorPreset("curved", "right", "left")).toBe(
      "curvedConnector3",
    );
    expect(pickConnectorPreset("curved", "right", "top")).toBe(
      "curvedConnector2",
    );
    expect(pickConnectorPreset("curved", "top", "top")).toBe(
      "curvedConnector4",
    );
  });
});

describe("sideAnchor", () => {
  const bbox: SlideBBox = { x: 100, y: 200, w: 50, h: 80, prst: "rect" };
  it("returns midpoint of each side in slide coordinates", () => {
    expect(sideAnchor(bbox, "top")).toEqual({ x: 125, y: 200 });
    expect(sideAnchor(bbox, "right")).toEqual({ x: 150, y: 240 });
    expect(sideAnchor(bbox, "bottom")).toEqual({ x: 125, y: 280 });
    expect(sideAnchor(bbox, "left")).toEqual({ x: 100, y: 240 });
  });
});
