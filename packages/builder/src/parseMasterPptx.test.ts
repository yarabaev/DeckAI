import { describe, expect, it } from "vitest";
import { buildPptx } from "./buildPptx.ts";
import { parseMasterPptx } from "./parseMasterPptx.ts";
import type { MasterPptxLimits } from "./buildPptx.ts";

/**
 * Helper that runs buildPptx and returns the resulting PPTX buffer.
 */
async function generatePptxBuffer(
  masterBackground?: { color: string } | { data: string },
): Promise<Uint8Array> {
  const xml = '<VStack><Text fontSize="24">test</Text></VStack>';
  const slideSize = { w: 960, h: 540 };
  const result = await buildPptx(xml, slideSize, {
    master: masterBackground ? { background: masterBackground } : undefined,
  });
  return (await result.pptx.write({ outputType: "uint8array" })) as Uint8Array;
}

describe("parseMasterPptx", () => {
  it("単色背景を抽出できる", async () => {
    const buffer = await generatePptxBuffer({ color: "FF0000" });
    const bg = await parseMasterPptx(buffer);
    expect(bg).toEqual({ color: "FF0000" });
  });

  it("背景なしの PPTX では undefined を返す", async () => {
    const buffer = await generatePptxBuffer();
    const bg = await parseMasterPptx(buffer);
    expect(bg).toBeUndefined();
  });

  it("画像背景を抽出できる（data URI）", async () => {
    // Use a 1x1 red-pixel PNG data URI as the background.
    const redPixelPng =
      "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
    const buffer = await generatePptxBuffer({ data: redPixelPng });
    const bg = await parseMasterPptx(buffer);

    expect(bg).toBeDefined();
    expect(bg).toHaveProperty("data");
    if (bg && "data" in bg) {
      expect(bg.data).toMatch(/^data:image\/[a-z]+;base64,/);
    }
  });
});

describe("buildPptx with masterPptx option", () => {
  it("masterPptx を指定して PPTX を生成できる", async () => {
    // Generate a PPTX with a background.
    const templateBuffer = await generatePptxBuffer({ color: "0000FF" });

    // Pass it as masterPptx and generate a new PPTX.
    const xml = '<VStack><Text fontSize="24">hello</Text></VStack>';
    const result = await buildPptx(
      xml,
      { w: 960, h: 540 },
      {
        masterPptx: templateBuffer,
      },
    );

    expect(result.pptx).toBeDefined();
    expect(result.diagnostics).toEqual([]);
  });

  it("不正な masterPptx を渡した場合は diagnostics に警告が追加される", async () => {
    const xml = '<VStack><Text fontSize="24">hello</Text></VStack>';
    const result = await buildPptx(
      xml,
      { w: 960, h: 540 },
      {
        masterPptx: new Uint8Array([0, 1, 2, 3]), // 壊れたデータ
      },
    );

    expect(result.pptx).toBeDefined();
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe("MASTER_PPTX_PARSE_FAILED");
  });

  it("master.background が明示指定されている場合は masterPptx の背景より優先", async () => {
    const templateBuffer = await generatePptxBuffer({ color: "0000FF" });

    // Specify both masterPptx and master.background.
    const xml = '<VStack><Text fontSize="24">hello</Text></VStack>';
    const result = await buildPptx(
      xml,
      { w: 960, h: 540 },
      {
        masterPptx: templateBuffer,
        master: { background: { color: "FF0000" } },
      },
    );

    // Re-parse the generated PPTX and verify the background.
    const outputBuffer = (await result.pptx.write({
      outputType: "uint8array",
    })) as Uint8Array;
    const bg = await parseMasterPptx(outputBuffer);
    expect(bg).toEqual({ color: "FF0000" });
  });
});

describe("T39: parseMasterPptx size limits", () => {
  it("buffer exceeding maxBytes emits MASTER_PPTX_SIZE_LIMIT diagnostic", async () => {
    const templateBuffer = await generatePptxBuffer({ color: "0000FF" });
    const xml = '<VStack><Text fontSize="24">test</Text></VStack>';
    // Set maxBytes to 1 byte to trigger the limit
    const limits: MasterPptxLimits = { maxBytes: 1 };
    const result = await buildPptx(
      xml,
      { w: 960, h: 540 },
      {
        masterPptx: templateBuffer,
        masterPptxLimits: limits,
      },
    );
    expect(
      result.diagnostics.some((d) => d.code === "MASTER_PPTX_SIZE_LIMIT"),
    ).toBe(true);
  });

  it("buffer within default maxBytes does not emit MASTER_PPTX_SIZE_LIMIT", async () => {
    const templateBuffer = await generatePptxBuffer({ color: "0000FF" });
    const xml = '<VStack><Text fontSize="24">test</Text></VStack>';
    const result = await buildPptx(
      xml,
      { w: 960, h: 540 },
      {
        masterPptx: templateBuffer,
      },
    );
    expect(
      result.diagnostics.some((d) => d.code === "MASTER_PPTX_SIZE_LIMIT"),
    ).toBe(false);
  });

  it("embedded image exceeding maxImageBytes emits MASTER_PPTX_SIZE_LIMIT and skips background", async () => {
    // Generate a PPTX with an embedded image background (1x1 red pixel PNG).
    const redPixelPng =
      "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
    const templateBuffer = await generatePptxBuffer({ data: redPixelPng });

    const capturedDiagnostics: { code: string; message: string }[] = [];
    const collector = {
      add(code: string, message: string) {
        capturedDiagnostics.push({ code, message });
      },
    } as unknown as import("./diagnostics.ts").DiagnosticCollector;

    // maxImageBytes=1 forces the embedded image to exceed the per-image cap.
    const bgResult = await parseMasterPptx(templateBuffer, {
      limits: { maxImageBytes: 1 },
      diagnostics: collector,
    });

    expect(
      capturedDiagnostics.some((d) => d.code === "MASTER_PPTX_SIZE_LIMIT"),
    ).toBe(true);
    // Background is skipped when the image exceeds the per-image cap.
    expect(bgResult).toBeUndefined();
  });
});
