/**
 * Layout regression tests for the three "silent surprises" patterns:
 *
 *   1. HStack with a w="max" sibling — non-max siblings must keep their
 *      intrinsic content width and not collapse to ~1ch.
 *   2. Table <Col w="NN%"/> — percentage column widths resolve against
 *      the table's resolved width (rather than silently becoming 0).
 *   3. Multi-line <Text> body — the source XML's formatting newlines
 *      and leading indent must NOT bleed into the rendered text. Author
 *      `\n` escapes still survive as line breaks.
 */

import { describe, expect, it } from "vitest";
import { parseXml } from "../parseXml/parseXml.ts";
import { buildPptx } from "../buildPptx.ts";
import {
  resolveColumnWidths,
  calcTableIntrinsicSize,
} from "../shared/tableUtils.ts";
import type { TableNode, BuilderNode } from "../types.ts";

describe("Multi-line <Text> source collapses formatting whitespace", () => {
  it("collapses newlines + indent inside a plain Text body", () => {
    const nodes = parseXml(`
      <Text>
        first line of paragraph
        second line of paragraph
      </Text>
    `);
    const text = nodes[0] as Extract<BuilderNode, { type: "text" }>;
    expect(text.text).toBe("first line of paragraph second line of paragraph");
  });

  it("collapses formatting whitespace around inline <Span> runs", () => {
    const nodes = parseXml(`
      <Text>
        foo
        <Span color="FF0000">bar</Span>
        baz
      </Text>
    `);
    const text = nodes[0] as Extract<BuilderNode, { type: "text" }>;
    expect(text.text).toBe("foo bar baz");
    // The middle Span is rendered as a run, not part of the surrounding
    // text. Spot-check the colored run survives.
    const colored = text.runs?.find((r) => r.color === "FF0000");
    expect(colored?.text).toBe("bar");
  });

  it("preserves author-inserted \\n line breaks", () => {
    const nodes = parseXml(`<Text>line one\\nline two</Text>`);
    const text = nodes[0] as Extract<BuilderNode, { type: "text" }>;
    expect(text.text).toBe("line one\nline two");
  });

  it("leaves same-line internal spacing untouched (monospace code)", () => {
    const nodes = parseXml(
      `<Text fontFamily="JetBrains Mono">  indented code line</Text>`,
    );
    const text = nodes[0] as Extract<BuilderNode, { type: "text" }>;
    expect(text.text).toBe("  indented code line");
  });

  it("trims leading/trailing source whitespace on inline-runs Text", () => {
    const nodes = parseXml(`
      <Text>
        <Span color="FF0000">only run</Span>
      </Text>
    `);
    const text = nodes[0] as Extract<BuilderNode, { type: "text" }>;
    // Joined runs text should contain exactly the authored content,
    // with no leading/trailing source whitespace from the surrounding
    // XML indentation.
    expect(text.text).toBe("only run");
    const nonEmptyRuns = text.runs?.filter((r) => r.text.length > 0) ?? [];
    expect(nonEmptyRuns.map((r) => r.text)).toEqual(["only run"]);
  });
});

describe('Table <Col w="NN%"/> resolves against table width', () => {
  function tableFromNodes(nodes: BuilderNode[]): TableNode {
    // parseXml(<Table>...) returns [TableNode].
    return nodes[0] as TableNode;
  }

  it("distributes percentage columns proportionally", () => {
    const nodes = parseXml(`
      <Table w="100%" defaultRowHeight="24">
        <Col w="25%"/>
        <Col w="25%"/>
        <Col w="50%"/>
        <Tr>
          <Td text="a"/><Td text="b"/><Td text="c"/>
        </Tr>
      </Table>
    `);
    const tbl = tableFromNodes(nodes);
    const widths = resolveColumnWidths(tbl, 800);
    expect(widths).toEqual([200, 200, 400]);
  });

  it("mixes percentage and absolute columns", () => {
    const nodes = parseXml(`
      <Table w="100%" defaultRowHeight="24">
        <Col w="100"/>
        <Col w="25%"/>
        <Col w="25%"/>
        <Tr>
          <Td text="a"/><Td text="b"/><Td text="c"/>
        </Tr>
      </Table>
    `);
    const tbl = tableFromNodes(nodes);
    const widths = resolveColumnWidths(tbl, 800);
    // 100 fixed + 25% of 800 + 25% of 800 = 100 + 200 + 200 = 500.
    expect(widths).toEqual([100, 200, 200]);
  });

  it('treats Col w="max" as unspecified (shares slack equally)', () => {
    const nodes = parseXml(`
      <Table w="100%" defaultRowHeight="24">
        <Col w="200"/>
        <Col w="max"/>
        <Col w="max"/>
        <Tr>
          <Td text="a"/><Td text="b"/><Td text="c"/>
        </Tr>
      </Table>
    `);
    const tbl = tableFromNodes(nodes);
    const widths = resolveColumnWidths(tbl, 800);
    // 200 fixed; remaining 600 split equally across the two "max"
    // columns = 300 each.
    expect(widths).toEqual([200, 300, 300]);
  });

  it("calcTableIntrinsicSize is robust to percent widths (falls back)", () => {
    const nodes = parseXml(`
      <Table w="100%" defaultRowHeight="24">
        <Col w="50%"/>
        <Col w="50%"/>
        <Tr>
          <Td text="a"/><Td text="b"/>
        </Tr>
      </Table>
    `);
    const tbl = tableFromNodes(nodes);
    // Intrinsic size is computed without knowing tableWidth, so percent
    // columns fall back to DEFAULT_TABLE_COLUMN_WIDTH (100).
    const { width } = calcTableIntrinsicSize(tbl);
    expect(width).toBe(200);
  });
});

describe("Text textVAlign flows through to renderer", () => {
  it("parses textVAlign=middle on a Text node", () => {
    const nodes = parseXml(
      `<Text fontSize="13" textVAlign="middle">centered glyphs</Text>`,
    );
    const text = nodes[0] as Extract<BuilderNode, { type: "text" }>;
    expect(text.textVAlign).toBe("middle");
  });

  it("parses textVAlign=bottom on a Ul node", () => {
    const nodes = parseXml(
      `<Ul fontSize="13" textVAlign="bottom"><Li text="a"/></Ul>`,
    );
    const ul = nodes[0] as Extract<BuilderNode, { type: "ul" }>;
    expect(ul.textVAlign).toBe("bottom");
  });

  it("buildPptx round-trip honors textVAlign in mixed-size HStack rows", async () => {
    // Smoke test: an HStack with one big bold name and one small caption,
    // where the smaller caption opts into vertical centering inside the
    // flex-stretched box. The pre-feature output would have the caption
    // glyphs visually floating at the top of the equalized 20px box.
    const xml = `
      <VStack w="400">
        <HStack gap="10" alignItems="baseline">
          <Text fontSize="16" bold="true">Mara Olsen</Text>
          <Text fontSize="9" bold="true" textVAlign="middle">Lead writer</Text>
        </HStack>
      </VStack>
    `;
    const result = await buildPptx(
      xml,
      { w: 1280, h: 720 },
      {
        textMeasurement: "fallback",
        autoFit: false,
      },
    );
    expect(result.pptx).toBeDefined();
    expect(result.diagnostics).toEqual([]);
  });
});

describe('HStack w="max" sibling does not collapse intrinsic siblings', () => {
  it("buildPptx renders a number/title/page row without throwing", async () => {
    // Smoke test that the equal-distribution heuristic doesn't kick in
    // when a sibling has w="max". The pre-fix bug collapsed unsized
    // siblings (e.g. <Text>01</Text>) to ~1ch which surfaced as one
    // character per line at render time. This test guards the parse +
    // layout path; the rendered widths are visually verified in the
    // playground-samples editorial TOC slide.
    const xml = `
      <VStack w="400">
        <HStack gap="12" alignItems="baseline" w="100%">
          <Text fontFamily="Inter" fontSize="13" bold="true">01</Text>
          <Text w="max" fontFamily="Inter" fontSize="13">A very long title that would otherwise eat its siblings</Text>
          <Text fontFamily="Inter" fontSize="13">19</Text>
        </HStack>
      </VStack>
    `;
    const result = await buildPptx(
      xml,
      { w: 1280, h: 720 },
      {
        textMeasurement: "fallback",
        autoFit: false,
      },
    );
    expect(result.pptx).toBeDefined();
    expect(result.diagnostics).toEqual([]);
  });
});
