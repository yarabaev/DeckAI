import { describe, expect, it, vi, afterEach } from "vitest";
import * as measureImageModule from "../shared/measureImage.ts";
import { buildPptx } from "../buildPptx.ts";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("imageSrcGuard at prefetch phase", () => {
  it("does not call fetch for blocked HTTPS src (SSRF prevention)", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch");

    const xml = `<VStack><Image src="https://evil.com/x.png" w="100" h="100" /></VStack>`;
    const result = await buildPptx(
      xml,
      { w: 1280, h: 720 },
      {
        autoFit: false,
        imageSrcGuard: { allowBaseDir: "/safe-images" },
      },
    );

    // fetch must not have been called for the blocked URL
    const blockedCalls = fetchSpy.mock.calls.filter((args) =>
      String(args[0]).includes("evil.com"),
    );
    expect(blockedCalls).toHaveLength(0);

    // INVALID_IMAGE_SRC diagnostic should be emitted
    const blocked = result.diagnostics.filter(
      (d) => d.code === "INVALID_IMAGE_SRC",
    );
    expect(blocked.length).toBeGreaterThan(0);
  });

  it("does not call prefetchImageSize for blocked file path (filesystem isolation)", async () => {
    // Spy on prefetchImageSize to verify it is not called for blocked paths.
    // We cannot spy on fs.readFileSync directly in ESM, so we verify at the
    // prefetch boundary — if prefetchImageSize is not called, readFileSync cannot run.
    const prefetchSpy = vi.spyOn(measureImageModule, "prefetchImageSize");

    const xml = `<VStack><Image src="/etc/passwd" w="100" h="100" /></VStack>`;
    const result = await buildPptx(
      xml,
      { w: 1280, h: 720 },
      {
        autoFit: false,
        imageSrcGuard: { allowBaseDir: "/safe-images" },
      },
    );

    // prefetchImageSize must not have been called for the blocked path
    const blockedCalls = prefetchSpy.mock.calls.filter(
      (args) => String(args[0]) === "/etc/passwd",
    );
    expect(blockedCalls).toHaveLength(0);

    // INVALID_IMAGE_SRC diagnostic should be emitted
    const blocked = result.diagnostics.filter(
      (d) => d.code === "INVALID_IMAGE_SRC",
    );
    expect(blocked.length).toBeGreaterThan(0);
  });

  it("does not call measureImage via Yoga MeasureFunc for blocked image without explicit w/h (HIGH-3)", async () => {
    // Image with NO explicit w/h triggers Yoga MeasureFunc path.
    // When imageSrcGuard blocks the src, measureImage must not be called (fs.readFileSync bypass).
    const measureSpy = vi.spyOn(measureImageModule, "measureImage");

    const xml = `<VStack><Image src="/etc/passwd" /></VStack>`;
    const result = await buildPptx(
      xml,
      { w: 1280, h: 720 },
      {
        autoFit: false,
        imageSrcGuard: { allowBaseDir: "/safe" },
      },
    );

    const blockedCalls = measureSpy.mock.calls.filter(
      (args) => String(args[0]) === "/etc/passwd",
    );
    expect(blockedCalls).toHaveLength(0);

    const blocked = result.diagnostics.filter(
      (d) => d.code === "INVALID_IMAGE_SRC",
    );
    expect(blocked.length).toBeGreaterThan(0);
  });

  it("does not call fetch for blocked backgroundImage src", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch");

    // backgroundImage requires JSON object syntax
    const xml = `<VStack backgroundImage='{"src":"https://evil.com/bg.png","sizing":"cover"}'><Text>hi</Text></VStack>`;
    const result = await buildPptx(
      xml,
      { w: 1280, h: 720 },
      {
        autoFit: false,
        imageSrcGuard: { allowBaseDir: "/safe-images" },
      },
    );

    const blockedCalls = fetchSpy.mock.calls.filter((args) =>
      String(args[0]).includes("evil.com"),
    );
    expect(blockedCalls).toHaveLength(0);

    // INVALID_IMAGE_SRC diagnostic should be emitted for backgroundImage
    const blocked = result.diagnostics.filter(
      (d) => d.code === "INVALID_IMAGE_SRC",
    );
    expect(blocked.length).toBeGreaterThan(0);
  });
});
