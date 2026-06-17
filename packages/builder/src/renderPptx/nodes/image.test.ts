import { describe, expect, it, vi } from "vitest";
import { renderImageNode } from "./image.ts";

const mockCtx = {
  slide: {} as never,
  pptx: {} as never,
  buildContext: { security: {}, diagnostics: {} } as never,
};

describe("renderImageNode", () => {
  it("passes altText to addImage when provided", () => {
    const addImage = vi.fn();

    renderImageNode(
      {
        type: "image",
        x: 0,
        y: 0,
        w: 200,
        h: 100,
        src: "test.png",
        altText: "Quarterly chart",
      } as never,
      { ...mockCtx, slide: { addImage } as never },
    );

    expect(addImage).toHaveBeenCalledTimes(1);
    const callArg = addImage.mock.calls[0][0] as Record<string, unknown>;
    expect(callArg.altText).toBe("Quarterly chart");
  });

  it("sets altText to empty string when isDecorative is true", () => {
    const addImage = vi.fn();

    renderImageNode(
      {
        type: "image",
        x: 0,
        y: 0,
        w: 200,
        h: 100,
        src: "test.png",
        isDecorative: true,
      } as never,
      { ...mockCtx, slide: { addImage } as never },
    );

    expect(addImage).toHaveBeenCalledTimes(1);
    const callArg = addImage.mock.calls[0][0] as Record<string, unknown>;
    expect(callArg.altText).toBe("");
  });

  it("passes rotate to addImage when specified", () => {
    const addImage = vi.fn();

    renderImageNode(
      {
        type: "image",
        x: 0,
        y: 0,
        w: 200,
        h: 100,
        src: "test.png",
        rotate: 90,
      } as never,
      { ...mockCtx, slide: { addImage } as never },
    );

    expect(addImage).toHaveBeenCalledTimes(1);
    const callArg = addImage.mock.calls[0][0] as Record<string, unknown>;
    expect(callArg.rotate).toBe(90);
  });

  it("does not include altText when not provided", () => {
    const addImage = vi.fn();

    renderImageNode(
      {
        type: "image",
        x: 0,
        y: 0,
        w: 200,
        h: 100,
        src: "test.png",
      } as never,
      { ...mockCtx, slide: { addImage } as never },
    );

    expect(addImage).toHaveBeenCalledTimes(1);
    const callArg = addImage.mock.calls[0][0] as Record<string, unknown>;
    expect(callArg.altText).toBeUndefined();
  });
});
