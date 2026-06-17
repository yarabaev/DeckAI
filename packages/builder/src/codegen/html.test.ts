import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  generateReferenceHtml,
  renderElementPage,
  renderIndexPage,
} from "./html.ts";
import { buildReferenceModel } from "./walkRegistry.ts";

describe("renderElementPage(Text)", () => {
  const model = buildReferenceModel();
  const text = model.nodes.find((n) => n.tag === "Text")!;
  const html = renderElementPage(text, model);

  it("has the correct <title>", () => {
    expect(html).toContain(
      "<title>&lt;Text&gt; · Builder XML Reference</title>",
    );
  });

  it("renders FOUC theme bootstrap script", () => {
    expect(html).toContain(`localStorage.getItem("sg-theme")`);
  });

  it("includes a skip-link as first focusable element", () => {
    const skipIdx = html.indexOf('class="skip-link"');
    const navIdx = html.indexOf("<nav");
    expect(skipIdx).toBeGreaterThan(-1);
    expect(skipIdx).toBeLessThan(navIdx);
  });

  it("wraps the sidebar in nav[aria-label]", () => {
    expect(html).toContain(
      `<nav class="ref-sidebar" aria-label="Builder elements">`,
    );
  });

  it("marks the current page with aria-current=page", () => {
    expect(html).toMatch(/href="\.\.\/text\/"\s+aria-current="page"/);
  });

  it("includes Visual nodes and Meta & composition group headers", () => {
    expect(html).toContain(">Visual nodes</h2>");
    expect(html).toContain(">Meta &amp; composition</h2>");
  });

  it("renders an attribute table with bold and color rows", () => {
    expect(html).toContain("<code>bold</code>");
    expect(html).toContain("<code>color</code>");
  });

  it("renders Used by section listing VStack", () => {
    expect(html).toContain(`href="../vstack/"`);
  });

  it("renders the Example heading exactly once (no synopsis duplication)", () => {
    const matches = html.match(/<h2[^>]*>Example<\/h2>/g) ?? [];
    expect(matches).toHaveLength(1);
  });

  it("renders the xml-snippet exactly once (matches single Example block)", () => {
    const snippets = html.match(/<pre class="xml-snippet">/g) ?? [];
    expect(snippets).toHaveLength(1);
  });

  it("renders See also section with at least one link", () => {
    expect(html).toMatch(/<h2[^>]*>See also/);
  });

  it("renders Source section linking to compiled/index.ts", () => {
    expect(html).toMatch(
      /blob\/main\/packages\/builder\/src\/registry\/compiled\/index\.ts#L\d+/,
    );
  });
});

describe("renderElementPage(SlideGlance) — empty usedBy branch", () => {
  const model = buildReferenceModel();
  const node = model.nodes.find((n) => n.tag === "SlideGlance")!;
  const html = renderElementPage(node, model);

  it("renders top-level fallback for usedBy", () => {
    expect(html).toContain("Top-level");
  });

  it("omits See also section when no entry exists", () => {
    expect(html).not.toMatch(/<h2[^>]*>See also/);
  });
});

describe("renderIndexPage", () => {
  const model = buildReferenceModel();
  const html = renderIndexPage(model);

  it("renders 30 cards total (17 nodes + 13 meta)", () => {
    const cards = html.match(/class="ref-card"/g) ?? [];
    expect(cards).toHaveLength(30);
  });

  it("includes data-haystack on every card with lowercased content", () => {
    const matches = [...html.matchAll(/data-haystack="([^"]+)"/g)];
    expect(matches).toHaveLength(30);
    for (const m of matches) {
      const haystack = m[1];
      expect(haystack).toBe(haystack.toLowerCase());
    }
  });

  it("includes the filter input with id=ref-q", () => {
    expect(html).toContain('id="ref-q"');
  });

  it("renders empty-state region with aria-live=polite", () => {
    expect(html).toContain('aria-live="polite"');
  });

  it("includes both group headings", () => {
    expect(html).toContain(">Visual nodes</h2>");
    expect(html).toContain(">Meta &amp; composition</h2>");
  });
});

describe("generateReferenceHtml", () => {
  const files = generateReferenceHtml();

  it("produces 30 HTML files + styles.css + scripts/site.js", () => {
    const html = [...files.keys()].filter((k) => k.endsWith(".html"));
    expect(html.sort()).toContain("index.html");
    expect(html).toContain("text/index.html");
    expect(html).toContain("slideglance/index.html");
    expect(html).toContain("document/index.html");
    expect(html).toContain("connector/index.html");
    expect(html).toHaveLength(31);
  });

  it("produces styles.css and scripts/site.js", () => {
    expect(files.has("styles.css")).toBe(true);
    expect(files.has("scripts/site.js")).toBe(true);
  });

  it("styles.css mirrors landing palette tokens", () => {
    const css = files.get("styles.css")!;
    expect(css).toContain("--bg:");
    expect(css).toContain("--accent:");
    expect(css).toContain('[data-theme="dark"]');
  });

  it("scripts/site.js gates / shortcut against editable surfaces", () => {
    const js = files.get("scripts/site.js")!;
    expect(js).toContain("contenteditable");
    expect(js).toContain('role="textbox"');
  });
});

describe("golden snapshots", () => {
  const HERE = fileURLToPath(import.meta.url);
  const snapDir = resolve(HERE, "../__snapshots__");
  const cases: Array<{ tag: string; file: string }> = [
    { tag: "Text", file: "text-index.html" },
    { tag: "SlideGlance", file: "slideglance-index.html" },
    { tag: "VStack", file: "vstack-index.html" },
    { tag: "Chart", file: "chart-index.html" },
  ];
  const files = generateReferenceHtml();
  for (const { tag, file } of cases) {
    it(`${tag} page matches snapshot`, () => {
      const slug = tag.toLowerCase();
      const generated = files.get(`${slug}/index.html`)!;
      const snapPath = resolve(snapDir, file);
      if (!existsSync(snapPath) || process.env.UPDATE_SNAPSHOTS === "1") {
        writeFileSync(snapPath, generated);
      }
      const expected = readFileSync(snapPath, "utf8");
      expect(generated).toBe(expected);
    });
  }
});
