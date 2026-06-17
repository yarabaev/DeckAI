import { describe, expect, it } from "vitest";
import {
  attrTable,
  childrenTable,
  escapeHtml,
  highlightXml,
  usedByList,
} from "./htmlTemplates.ts";

describe("escapeHtml", () => {
  it("escapes all five HTML-significant characters", () => {
    expect(escapeHtml("& < > \" '")).toBe("&amp; &lt; &gt; &quot; &#39;");
  });

  it("double-encodes already-escaped entities (input must be raw text)", () => {
    // intentional: re-running escape would double-encode, which is correct.
    expect(escapeHtml("&amp;")).toBe("&amp;amp;");
  });

  it("passes plain text through unchanged", () => {
    expect(escapeHtml("Hello, world.")).toBe("Hello, world.");
  });
});

describe("highlightXml", () => {
  it("wraps tag names in tk-tag spans", () => {
    const out = highlightXml(`<Text>hello</Text>`);
    expect(out).toContain(`<span class="tk-tag">Text</span>`);
  });

  it("wraps attribute names + string values", () => {
    const out = highlightXml(`<Foo bar="baz" />`);
    expect(out).toContain(`<span class="tk-attr">bar</span>`);
    expect(out).toContain(`<span class="tk-str">&quot;baz&quot;</span>`);
  });

  it("wraps {placeholder} interpolations", () => {
    const out = highlightXml(`<Text>{name}</Text>`);
    expect(out).toContain(`<span class="tk-interp">{name}</span>`);
  });

  it("leaves no raw < or > in output text content", () => {
    const out = highlightXml(`<Foo a="b">x</Foo>`);
    expect(out).not.toMatch(/<[A-Za-z][^>]*?>(?![^<]*<\/span)/);
  });

  it("escapes hostile content in attribute values", () => {
    const out = highlightXml(`<Foo a="&lt;evil&gt;" />`);
    expect(out).toContain(`&amp;lt;evil&amp;gt;`);
  });
});

describe("attrTable", () => {
  it("renders one row per attribute, alphabetically sorted", () => {
    const html = attrTable({
      zoo: { coerce: "number" } as never,
      bar: { coerce: "string", required: true } as never,
      apple: { coerce: "boolean" } as never,
    });
    const order = html.match(/<code>(\w+)<\/code>/g) ?? [];
    expect(order.map((t) => t.replace(/<\/?code>/g, ""))).toEqual([
      "apple",
      "boolean",
      "bar",
      "string",
      "zoo",
      "number",
    ]);
  });

  it("marks required attributes with attr-required class", () => {
    const html = attrTable({
      x: { coerce: "string", required: true } as never,
    });
    expect(html).toContain(`class="attr-required"`);
  });

  it("escapes attribute descriptions", () => {
    const html = attrTable({
      x: { coerce: "string", doc: 'Use <foo> & "bar".' } as never,
    });
    expect(html).toContain("&lt;foo&gt; &amp; &quot;bar&quot;");
  });
});

describe("childrenTable", () => {
  it("renders 'no children' when input is empty", () => {
    expect(childrenTable({})).toContain("accepts no child elements");
  });

  it("renders a row per child with cardinality", () => {
    const html = childrenTable({
      Li: { element: "Li", min: 0, doc: "list item" } as never,
    });
    expect(html).toContain(`<code>&lt;Li&gt;</code>`);
    expect(html).toContain("0..∞");
  });
});

describe("usedByList", () => {
  it("renders top-level fallback when entries empty", () => {
    expect(usedByList([])).toContain("Top-level");
  });

  it("links each parent to its reference page", () => {
    const html = usedByList([{ parent: "VStack", cardinality: "0..∞" }]);
    expect(html).toContain(`href="../vstack/"`);
    expect(html).toContain(`<code>&lt;VStack&gt;</code>`);
  });
});
