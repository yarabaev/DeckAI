/**
 * Unit tests for the parse-phase `RAW_LT_GT_IN_ATTR` scanner.
 *
 * The scanner is exercised here in isolation (not through buildPptx)
 * because the violation is a pre-parse source-level concern: by the
 * time fast-xml-parser has decoded the attribute value, the original
 * characters are gone.
 */

import { describe, expect, it } from "vitest";
import { scanRawXmlForBareLtGt } from "./rawXml.ts";

describe("scanRawXmlForBareLtGt", () => {
  it("flags a bare `<` inside an attribute value", () => {
    const xml = `<Td text="latency < 200ms" fontSize="10"/>`;
    const diags = scanRawXmlForBareLtGt(xml);
    expect(diags).toHaveLength(1);
    const d = diags[0]!;
    expect(d.code).toBe("RAW_LT_GT_IN_ATTR");
    expect(d.severity).toBe("warn");
    expect(d.context?.attribute).toBe("text");
    expect(d.context?.characters).toEqual(["<"]);
    expect(d.suggestedFix).toMatchObject({
      kind: "attribute-set",
      set: { text: "latency &lt; 200ms" },
    });
  });

  it("flags a bare `>` inside an attribute value", () => {
    const xml = `<Td text="latency > 200ms" fontSize="10"/>`;
    const diags = scanRawXmlForBareLtGt(xml);
    expect(diags).toHaveLength(1);
    expect(diags[0]?.context?.characters).toEqual([">"]);
    expect(diags[0]?.suggestedFix).toMatchObject({
      set: { text: "latency &gt; 200ms" },
    });
  });

  it("flags both `<` and `>` in the same attribute value", () => {
    const xml = `<Td text="5 < x > 1"/>`;
    const diags = scanRawXmlForBareLtGt(xml);
    expect(diags).toHaveLength(1);
    expect(diags[0]?.context?.characters).toEqual(["<", ">"]);
    expect(diags[0]?.suggestedFix).toMatchObject({
      set: { text: "5 &lt; x &gt; 1" },
    });
  });

  it("does NOT flag entity-escaped `&lt;` / `&gt;` (correctly authored)", () => {
    const xml = `<Td text="latency &lt; 200ms and &gt; 5ms"/>`;
    const diags = scanRawXmlForBareLtGt(xml);
    expect(diags).toHaveLength(0);
  });

  it("does NOT flag arrows or other non-bracket characters", () => {
    const xml = `<Td text="control → director"/>`;
    expect(scanRawXmlForBareLtGt(xml)).toHaveLength(0);
  });

  it("does NOT flag bare brackets inside an XML comment", () => {
    const xml = `<!-- 5 < x and a > b are fine inside comments --><Td text="ok"/>`;
    expect(scanRawXmlForBareLtGt(xml)).toHaveLength(0);
  });

  it("does NOT flag bare brackets inside a CDATA section", () => {
    const xml = `<Doc><![CDATA[ x < 1 and y > 2 ]]></Doc><Td text="ok"/>`;
    expect(scanRawXmlForBareLtGt(xml)).toHaveLength(0);
  });

  it("does NOT flag the XML processing instruction", () => {
    const xml = `<?xml version="1.0" encoding="utf-8" ?><Td text="ok"/>`;
    expect(scanRawXmlForBareLtGt(xml)).toHaveLength(0);
  });

  it("reports accurate line numbers", () => {
    const xml = [
      `<Root>`,
      `  <Td text="ok"/>`,
      `  <Td text="bad < value"/>`,
      `  <Td text="ok"/>`,
      `</Root>`,
    ].join("\n");
    const diags = scanRawXmlForBareLtGt(xml);
    expect(diags).toHaveLength(1);
    expect(diags[0]?.sourcePos?.line).toBe(3);
  });

  it("attaches the supplied source path to each diagnostic", () => {
    const xml = `<Td text="bad < val"/>`;
    const diags = scanRawXmlForBareLtGt(xml, "/abs/path/chapter.xml");
    expect(diags[0]?.sourcePos?.file).toBe("/abs/path/chapter.xml");
  });

  it("flags every offender across multiple attributes and tags", () => {
    const xml = [
      `<Td text="a < b" subtext="c > d"/>`,
      `<Td text="ok"/>`,
      `<Td text="e <"/>`,
    ].join("\n");
    const diags = scanRawXmlForBareLtGt(xml);
    expect(diags).toHaveLength(3);
    // Order matches source order (tag, then attribute order within tag).
    expect(diags.map((d) => d.context?.attribute)).toEqual([
      "text",
      "subtext",
      "text",
    ]);
  });

  it("handles single-quoted attribute values", () => {
    const xml = `<Td text='bad < val'/>`;
    const diags = scanRawXmlForBareLtGt(xml);
    expect(diags).toHaveLength(1);
    expect(diags[0]?.context?.attribute).toBe("text");
  });

  it("is a no-op for empty input", () => {
    expect(scanRawXmlForBareLtGt("")).toEqual([]);
  });
});

describe("RAW_LT_GT_IN_ATTR — integration via buildPptx", async () => {
  const { buildPptx } = await import("../../buildPptx.ts");

  it("surfaces the diagnostic in the lint report", async () => {
    const xml = `<Text fontSize="13" text="latency < 200ms"/>`;
    const result = await buildPptx(
      xml,
      { w: 1280, h: 720 },
      {
        autoFit: false,
        textMeasurement: "fallback",
        lint: { enabled: true, ruleset: "recommended" },
      },
    );
    const codes = result.diagnostics.map((d) => d.code);
    expect(codes).toContain("RAW_LT_GT_IN_ATTR");
    expect(result.lintReport?.summary.warn).toBeGreaterThan(0);
  });

  it("is silent when lint is disabled", async () => {
    const xml = `<Text fontSize="13" text="latency < 200ms"/>`;
    const result = await buildPptx(
      xml,
      { w: 1280, h: 720 },
      {
        autoFit: false,
        textMeasurement: "fallback",
        lint: { enabled: false },
      },
    );
    const codes = result.diagnostics.map((d) => d.code);
    expect(codes).not.toContain("RAW_LT_GT_IN_ATTR");
  });
});
