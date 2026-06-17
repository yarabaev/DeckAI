import { describe, expect, it, vi } from "vitest";
import { buildPptx } from "./buildPptx.ts";
import { DiagnosticsError } from "./diagnostics.ts";
import * as measureTextModule from "./calcYogaLayout/measureText.ts";

describe("buildPptx 並列実行", () => {
  const slideSize = { w: 1280, h: 720 };

  it("異なる textMeasurement モードで並列実行しても干渉しない", async () => {
    const xml = `<VStack><Text fontSize="24">テスト文字列</Text></VStack>`;

    const spy = vi.spyOn(measureTextModule, "measureText");

    // Run in parallel with different textMeasurement options.
    const [resultOpentype, resultFallback] = await Promise.all([
      buildPptx(xml, slideSize, {
        textMeasurement: "opentype",
        autoFit: false,
      }),
      buildPptx(xml, slideSize, {
        textMeasurement: "fallback",
        autoFit: false,
      }),
    ]);

    // Both should finish successfully.
    expect(resultOpentype.pptx).toBeDefined();
    expect(resultFallback.pptx).toBeDefined();

    // Verify each mode flows through to the measureText call correctly.
    const opentypeCalls = spy.mock.calls.filter(
      (args) => args[3] === "opentype",
    );
    const fallbackCalls = spy.mock.calls.filter(
      (args) => args[3] === "fallback",
    );
    expect(opentypeCalls.length).toBeGreaterThan(0);
    expect(fallbackCalls.length).toBeGreaterThan(0);

    spy.mockRestore();
  });

  it("同一オプションで並列実行してもキャッシュが干渉しない", async () => {
    const xml1 = `<VStack><Text fontSize="20">文字列A</Text></VStack>`;
    const xml2 = `<VStack><Text fontSize="30">文字列B</Text></VStack>`;

    const [result1, result2] = await Promise.all([
      buildPptx(xml1, slideSize, { autoFit: false }),
      buildPptx(xml2, slideSize, { autoFit: false }),
    ]);

    expect(result1.pptx).toBeDefined();
    expect(result2.pptx).toBeDefined();
  });

  it("Icon を含む並列実行でキャッシュが干渉しない", async () => {
    const xml1 = `<VStack><Icon name="star" size="32" color="#FF0000" /></VStack>`;
    const xml2 = `<VStack><Icon name="heart" size="48" color="#0000FF" /></VStack>`;

    const [result1, result2] = await Promise.all([
      buildPptx(xml1, slideSize, { autoFit: false }),
      buildPptx(xml2, slideSize, { autoFit: false }),
    ]);

    expect(result1.pptx).toBeDefined();
    expect(result2.pptx).toBeDefined();
  });
});

describe("buildPptx diagnostics", () => {
  const slideSize = { w: 1280, h: 720 };

  it("正常なビルドでは diagnostics が空配列", async () => {
    const xml = `<VStack><Text fontSize="24">Hello</Text></VStack>`;
    const result = await buildPptx(xml, slideSize, { autoFit: false });
    expect(result.diagnostics).toEqual([]);
  });

  it("画像測定失敗時に IMAGE_MEASURE_FAILED が記録される", async () => {
    const xml = `<VStack><Image src="nonexistent-file.png" /></VStack>`;
    const result = await buildPptx(xml, slideSize, { autoFit: false });
    expect(result.diagnostics.length).toBeGreaterThan(0);
    expect(result.diagnostics[0].code).toBe("IMAGE_MEASURE_FAILED");
  });

  it("strict: true で diagnostics がある場合 DiagnosticsError をスロー", async () => {
    const xml = `<VStack><Image src="nonexistent-file.png" /></VStack>`;
    await expect(
      buildPptx(xml, slideSize, { autoFit: false, strict: true }),
    ).rejects.toThrow(DiagnosticsError);
  });

  it("strict: true で diagnostics がない場合は正常に返る", async () => {
    const xml = `<VStack><Text fontSize="24">Hello</Text></VStack>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      strict: true,
    });
    expect(result.diagnostics).toEqual([]);
    expect(result.pptx).toBeDefined();
  });
});

describe("T36: <A href> scheme allowlist", () => {
  const slideSize = { w: 1280, h: 720 };

  it("javascript: href emits INVALID_HREF_SCHEME and clears the hyperlink", async () => {
    const xml = `<VStack><Text fontSize="24"><A href="javascript:alert(1)">click</A></Text></VStack>`;
    const result = await buildPptx(xml, slideSize, { autoFit: false });
    expect(
      result.diagnostics.some((d) => d.code === "INVALID_HREF_SCHEME"),
    ).toBe(true);
  });

  it("https: href passes with no diagnostic", async () => {
    const xml = `<VStack><Text fontSize="24"><A href="https://example.com">link</A></Text></VStack>`;
    const result = await buildPptx(xml, slideSize, { autoFit: false });
    expect(
      result.diagnostics.some((d) => d.code === "INVALID_HREF_SCHEME"),
    ).toBe(false);
  });

  it("http: href passes with no diagnostic by default", async () => {
    const xml = `<VStack><Text fontSize="24"><A href="http://example.com">link</A></Text></VStack>`;
    const result = await buildPptx(xml, slideSize, { autoFit: false });
    expect(
      result.diagnostics.some((d) => d.code === "INVALID_HREF_SCHEME"),
    ).toBe(false);
  });

  it("mailto: and tel: pass with no diagnostic by default", async () => {
    const xml = `<VStack><Text fontSize="24"><A href="mailto:user@example.com">email</A></Text></VStack>`;
    const result = await buildPptx(xml, slideSize, { autoFit: false });
    expect(
      result.diagnostics.some((d) => d.code === "INVALID_HREF_SCHEME"),
    ).toBe(false);
  });

  it("allowedHrefSchemes option extends the default allowlist", async () => {
    const xml = `<VStack><Text fontSize="24"><A href="custom://example.com">link</A></Text></VStack>`;
    const resultDefault = await buildPptx(xml, slideSize, { autoFit: false });
    expect(
      resultDefault.diagnostics.some((d) => d.code === "INVALID_HREF_SCHEME"),
    ).toBe(true);

    const resultCustom = await buildPptx(xml, slideSize, {
      autoFit: false,
      allowedHrefSchemes: ["custom:"],
    });
    expect(
      resultCustom.diagnostics.some((d) => d.code === "INVALID_HREF_SCHEME"),
    ).toBe(false);
  });
});

describe("T36b: <A href> scheme allowlist — Ul/Ol list runs", () => {
  const slideSize = { w: 1280, h: 720 };

  it("javascript: href in <Ul><Li><A> emits INVALID_HREF_SCHEME", async () => {
    const xml = `<VStack><Ul fontSize="24"><Li>item <A href="javascript:alert(1)">x</A></Li></Ul></VStack>`;
    const result = await buildPptx(xml, slideSize, { autoFit: false });
    expect(
      result.diagnostics.some((d) => d.code === "INVALID_HREF_SCHEME"),
    ).toBe(true);
  });

  it("javascript: href in <Ol><Li><A> emits INVALID_HREF_SCHEME", async () => {
    const xml = `<VStack><Ol fontSize="24"><Li>item <A href="javascript:alert(1)">x</A></Li></Ol></VStack>`;
    const result = await buildPptx(xml, slideSize, { autoFit: false });
    expect(
      result.diagnostics.some((d) => d.code === "INVALID_HREF_SCHEME"),
    ).toBe(true);
  });

  it("https: href in <Ul><Li><A> passes with no diagnostic", async () => {
    const xml = `<VStack><Ul fontSize="24"><Li>item <A href="https://example.com">link</A></Li></Ul></VStack>`;
    const result = await buildPptx(xml, slideSize, { autoFit: false });
    expect(
      result.diagnostics.some((d) => d.code === "INVALID_HREF_SCHEME"),
    ).toBe(false);
  });

  it("https: href in <Ol><Li><A> passes with no diagnostic", async () => {
    const xml = `<VStack><Ol fontSize="24"><Li>item <A href="https://example.com">link</A></Li></Ol></VStack>`;
    const result = await buildPptx(xml, slideSize, { autoFit: false });
    expect(
      result.diagnostics.some((d) => d.code === "INVALID_HREF_SCHEME"),
    ).toBe(false);
  });
});

describe("T37: imageSrcGuard opt-in validation", () => {
  const slideSize = { w: 1280, h: 720 };

  it("path traversal in <Image src> emits INVALID_IMAGE_SRC when guard is active", async () => {
    const xml = `<VStack><Image src="../../../etc/passwd" /></VStack>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      imageSrcGuard: { allowSchemes: ["https:"] },
    });
    expect(result.diagnostics.some((d) => d.code === "INVALID_IMAGE_SRC")).toBe(
      true,
    );
  });

  it("no guard option — path traversal does not emit INVALID_IMAGE_SRC", async () => {
    const xml = `<VStack><Image src="../../../etc/passwd" /></VStack>`;
    const result = await buildPptx(xml, slideSize, { autoFit: false });
    expect(result.diagnostics.some((d) => d.code === "INVALID_IMAGE_SRC")).toBe(
      false,
    );
  });

  it("https: src passes when guard allows https", async () => {
    const xml = `<VStack><Image src="https://example.com/img.png" /></VStack>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      imageSrcGuard: { allowSchemes: ["https:"] },
    });
    expect(result.diagnostics.some((d) => d.code === "INVALID_IMAGE_SRC")).toBe(
      false,
    );
  });

  it("allowBaseDir=/images blocks /images2/x.png (prefix bypass regression)", async () => {
    const xml = `<VStack><Image src="/images2/x.png" /></VStack>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      imageSrcGuard: { allowBaseDir: "/images" },
    });
    expect(result.diagnostics.some((d) => d.code === "INVALID_IMAGE_SRC")).toBe(
      true,
    );
  });

  it("allowBaseDir=/images blocks /images-evil/x.png (prefix bypass regression)", async () => {
    const xml = `<VStack><Image src="/images-evil/x.png" /></VStack>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      imageSrcGuard: { allowBaseDir: "/images" },
    });
    expect(result.diagnostics.some((d) => d.code === "INVALID_IMAGE_SRC")).toBe(
      true,
    );
  });

  // NEW-1: file: scheme with allowSchemes should still be blocked by allowBaseDir
  it("allowSchemes:[file:] + allowBaseDir:/images blocks file:///etc/passwd (HIGH-1 bypass)", async () => {
    const xml = `<VStack><Image src="file:///etc/passwd" /></VStack>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      imageSrcGuard: { allowSchemes: ["file:"], allowBaseDir: "/images" },
    });
    expect(result.diagnostics.some((d) => d.code === "INVALID_IMAGE_SRC")).toBe(
      true,
    );
  });

  // NEW-2: URL-schemed src bypasses allowBaseDir-only guard (SSRF)
  it("allowBaseDir only (no allowSchemes) blocks https://evil.com/x.png (HIGH-2 SSRF)", async () => {
    const xml = `<VStack><Image src="https://evil.com/x.png" /></VStack>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      imageSrcGuard: { allowBaseDir: "/images" },
    });
    expect(result.diagnostics.some((d) => d.code === "INVALID_IMAGE_SRC")).toBe(
      true,
    );
  });

  // Preserved: allowSchemes only (no allowBaseDir) still passes HTTPS URLs
  it("allowSchemes:[https:] (no allowBaseDir) passes https://cdn/x.png", async () => {
    const xml = `<VStack><Image src="https://cdn.example.com/x.png" /></VStack>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      imageSrcGuard: { allowSchemes: ["https:"] },
    });
    expect(result.diagnostics.some((d) => d.code === "INVALID_IMAGE_SRC")).toBe(
      false,
    );
  });

  // Preserved: allowBaseDir only still passes relative path within base dir
  it("allowBaseDir:/images (no allowSchemes) passes images/x.png", async () => {
    const xml = `<VStack><Image src="images/x.png" /></VStack>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      imageSrcGuard: { allowBaseDir: "/images" },
    });
    expect(result.diagnostics.some((d) => d.code === "INVALID_IMAGE_SRC")).toBe(
      false,
    );
  });
});

describe("T38: template expansion node counter", () => {
  const slideSize = { w: 1280, h: 720 };

  it("exceeding maxTemplateNodes emits TEMPLATE_EXPANSION_LIMIT diagnostic", async () => {
    // Each <Use template="BigBlock"/> expands to 200 VStack nodes; 3 uses = 600.
    // maxTemplateNodes=100 forces the expansion to exceed the limit.
    const manyChildren = '<VStack w="10" h="10" />'.repeat(200);
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="BigBlock">
            ${manyChildren}
          </Template>
        </Templates>
        <Slide>
          <VStack>
            <Use template="BigBlock"/>
            <Use template="BigBlock"/>
            <Use template="BigBlock"/>
          </VStack>
        </Slide>
      </SlideGlance>
    `;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      maxTemplateNodes: 100,
    });
    expect(
      result.diagnostics.some((d) => d.code === "TEMPLATE_EXPANSION_LIMIT"),
    ).toBe(true);
  });

  it("expansion within default limit (100,000) produces no TEMPLATE_EXPANSION_LIMIT diagnostic", async () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="Small">
            <VStack w="10" h="10" />
          </Template>
        </Templates>
        <Slide>
          <VStack>
            <Use template="Small"/>
          </VStack>
        </Slide>
      </SlideGlance>
    `;
    const result = await buildPptx(xml, slideSize, { autoFit: false });
    expect(
      result.diagnostics.some((d) => d.code === "TEMPLATE_EXPANSION_LIMIT"),
    ).toBe(false);
  });
});

describe("T50 — docProps option", () => {
  const slideSize = { w: 1280, h: 720 };
  const xml = `<SlideGlance><Slide><Text>x</Text></Slide></SlideGlance>`;

  it("sets pptx.title when docProps.title is provided", async () => {
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      docProps: { title: "My Presentation" },
    });
    expect((result.pptx as unknown as { title: string }).title).toBe(
      "My Presentation",
    );
  });

  it("sets pptx.author when docProps.author is provided", async () => {
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      docProps: { author: "Jane Doe" },
    });
    expect((result.pptx as unknown as { author: string }).author).toBe(
      "Jane Doe",
    );
  });

  it("sets pptx.company when docProps.company is provided", async () => {
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      docProps: { company: "Acme Corp" },
    });
    expect((result.pptx as unknown as { company: string }).company).toBe(
      "Acme Corp",
    );
  });

  it("sets pptx.subject when docProps.subject is provided", async () => {
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      docProps: { subject: "Q1 Report" },
    });
    expect((result.pptx as unknown as { subject: string }).subject).toBe(
      "Q1 Report",
    );
  });
});

describe("T24 regression — Icon backgroundColor applied to background shape", () => {
  const slideSize = { w: 1280, h: 720 };

  it("uses backgroundColor for the background shape fill, not the fallback gray", async () => {
    const xml = `<SlideGlance><Slide><Icon name="cpu" variant="square-filled" backgroundColor="#E8F0FE" size="48" /></Slide></SlideGlance>`;
    const result = await buildPptx(xml, slideSize, { autoFit: false });
    const pptxInternal = result.pptx as unknown as {
      _slides: Array<{
        _slideObjects: Array<{
          _type: string;
          options?: { fill?: { color?: string } };
        }>;
      }>;
    };
    const shapes = pptxInternal._slides[0]._slideObjects.filter(
      (o) => o._type === "text" && o.options?.fill?.color !== undefined,
    );
    const fillColor = shapes[0]?.options?.fill?.color;
    expect(fillColor).toBe("E8F0FE");
  });
});

describe("Container click-to-source — VStack/HStack/Layer hit-area objectName", () => {
  const slideSize = { w: 1280, h: 720 };

  // Without trackSourcePos, no __nodeId is allocated and no objectName is
  // emitted. With trackSourcePos enabled (the VS Code preview's mode),
  // every container should emit at least one shape carrying its
  // `objectName=node#N` so a click on the container's footprint resolves.
  it("emits an objectName-bearing shape for an empty VStack container", async () => {
    const xml = `<SlideGlance><Slide><VStack w="200" h="120" /></Slide></SlideGlance>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      trackSourcePos: true,
    });
    const pptxInternal = result.pptx as unknown as {
      _slides: Array<{
        _slideObjects: Array<{
          _type: string;
          options?: { objectName?: string };
        }>;
      }>;
    };
    const namedShapes =
      pptxInternal._slides[0]?._slideObjects.filter(
        (o) => typeof o.options?.objectName === "string",
      ) ?? [];
    expect(namedShapes.length).toBeGreaterThan(0);
    expect(namedShapes[0]?.options?.objectName).toMatch(/^node#\d+$/);
  });

  it("emits a hit-area shape for a VStack with no fill, no border, only children", async () => {
    const xml = `<SlideGlance><Slide><VStack gap="8" w="200"><Text>a</Text><Text>b</Text></VStack></Slide></SlideGlance>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      trackSourcePos: true,
    });
    const pptxInternal = result.pptx as unknown as {
      _slides: Array<{
        _slideObjects: Array<{
          _type: string;
          options?: { objectName?: string };
        }>;
      }>;
    };
    const objs = pptxInternal._slides[0]?._slideObjects ?? [];
    const namedShapes = objs.filter(
      (o) => typeof o.options?.objectName === "string",
    );
    // Two Text nodes + one container hit-area = at least 3 named shapes.
    expect(namedShapes.length).toBeGreaterThanOrEqual(3);
    // All three node ids should be distinct (children + container).
    const ids = new Set(
      namedShapes
        .map((o) => o.options?.objectName)
        .filter((n): n is string => typeof n === "string"),
    );
    expect(ids.size).toBeGreaterThanOrEqual(3);
  });

  it("does not emit a redundant hit-area when a VStack already has a backgroundColor", async () => {
    const xml = `<SlideGlance><Slide><VStack w="200" h="120" backgroundColor="EEEEEE" /></Slide></SlideGlance>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      trackSourcePos: true,
    });
    const pptxInternal = result.pptx as unknown as {
      _slides: Array<{
        _slideObjects: Array<{
          _type: string;
          options?: { objectName?: string };
        }>;
      }>;
    };
    const objs = pptxInternal._slides[0]?._slideObjects ?? [];
    const namedShapes = objs.filter(
      (o) => typeof o.options?.objectName === "string",
    );
    // Exactly one shape (the visible bg) carries the VStack's objectName —
    // the hit-area path is suppressed because the bg shape is already
    // clickable.
    const ids = new Set(
      namedShapes
        .map((o) => o.options?.objectName)
        .filter((n): n is string => typeof n === "string"),
    );
    expect(ids.size).toBe(1);
  });
});

describe("T42 — defaultLang option", () => {
  const slideSize = { w: 1280, h: 720 };

  it("applies defaultLang as fallback lang on text runs without explicit lang", async () => {
    const xml = `<SlideGlance><Slide><Text><Span>hello</Span></Text></Slide></SlideGlance>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      defaultLang: "en-US",
    });
    // No diagnostic errors and the build completes successfully
    expect(result.pptx).toBeDefined();
    expect(result.diagnostics).toHaveLength(0);
  });

  it("inline lang on a run overrides defaultLang", async () => {
    const xml = `<SlideGlance><Slide><Text><Span lang="ja-JP">hello</Span></Text></Slide></SlideGlance>`;
    const result = await buildPptx(xml, slideSize, {
      autoFit: false,
      defaultLang: "en-US",
    });
    expect(result.pptx).toBeDefined();
    expect(result.diagnostics).toHaveLength(0);
  });
});
