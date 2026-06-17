import { describe, expect, it, vi } from "vitest";
import { renderShapeNode } from "./shape.ts";

const mockCtx = {
  slide: {} as never,
  pptx: {} as never,
  buildContext: {} as never,
};

describe("renderShapeNode", () => {
  it("uses textVAlign when specified", () => {
    const addText = vi.fn();

    renderShapeNode(
      {
        type: "shape",
        x: 0,
        y: 0,
        w: 200,
        h: 100,
        shapeType: "rect",
        text: "Hello",
        textVAlign: "top",
      } as never,
      { ...mockCtx, slide: { addText } as never },
    );

    expect(addText).toHaveBeenCalledTimes(1);
    const opts = addText.mock.calls[0][1] as Record<string, unknown>;
    expect(opts.valign).toBe("top");
  });

  it("passes rotate to addText when specified", () => {
    const addText = vi.fn();

    renderShapeNode(
      {
        type: "shape",
        x: 0,
        y: 0,
        w: 200,
        h: 100,
        shapeType: "rect",
        text: "Hello",
        rotate: 45,
      } as never,
      { ...mockCtx, slide: { addText } as never },
    );

    expect(addText).toHaveBeenCalledTimes(1);
    const opts = addText.mock.calls[0][1] as Record<string, unknown>;
    expect(opts.rotate).toBe(45);
  });

  it("passes rotate to addShape when no text", () => {
    const addShape = vi.fn();

    renderShapeNode(
      {
        type: "shape",
        x: 0,
        y: 0,
        w: 200,
        h: 100,
        shapeType: "rect",
        rotate: 30,
      } as never,
      { ...mockCtx, slide: { addShape } as never },
    );

    expect(addShape).toHaveBeenCalledTimes(1);
    const opts = addShape.mock.calls[0][1] as Record<string, unknown>;
    expect(opts.rotate).toBe(30);
  });

  it("converts fill.transparency from 0-1 to 0-100 scale", () => {
    const addShape = vi.fn();

    renderShapeNode(
      {
        type: "shape",
        x: 0,
        y: 0,
        w: 200,
        h: 100,
        shapeType: "rect",
        fill: { color: "FF0000", transparency: 0.5 },
      } as never,
      { ...mockCtx, slide: { addShape } as never },
    );

    expect(addShape).toHaveBeenCalledTimes(1);
    const opts = addShape.mock.calls[0][1] as Record<
      string,
      { transparency?: number }
    >;
    expect(opts.fill?.transparency).toBe(50);
  });

  it("defaults textVAlign to middle when not specified", () => {
    const addText = vi.fn();

    renderShapeNode(
      {
        type: "shape",
        x: 0,
        y: 0,
        w: 200,
        h: 100,
        shapeType: "rect",
        text: "Hello",
      } as never,
      { ...mockCtx, slide: { addText } as never },
    );

    expect(addText).toHaveBeenCalledTimes(1);
    const opts = addText.mock.calls[0][1] as Record<string, unknown>;
    expect(opts.valign).toBe("middle");
  });
});
