import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { buildPptx } from "./buildPptx.ts";
import type { Diagnostic } from "./diagnostics.ts";

const __dirname = dirname(fileURLToPath(import.meta.url));
const XML_REFERENCE_MD_PATH = resolve(__dirname, "../docs/en/xml-reference.md");

type Sample = { index: number; section: string; xml: string };

function extractXmlSamples(md: string): Sample[] {
  const lines = md.split("\n");
  const samples: Sample[] = [];
  let currentSection = "(top)";
  let inBlock = false;
  let buf: string[] = [];
  let index = 0;
  for (const line of lines) {
    const heading = line.match(/^###\s+(?:\d+\.\s+)?(.+)$/);
    if (heading && !inBlock) {
      currentSection = heading[1].trim();
      continue;
    }
    if (!inBlock && line.trim() === "```xml") {
      inBlock = true;
      buf = [];
      continue;
    }
    if (inBlock && line.trim() === "```") {
      inBlock = false;
      samples.push({
        index: index++,
        section: currentSection,
        xml: buf.join("\n"),
      });
      continue;
    }
    if (inBlock) buf.push(line);
  }
  return samples;
}

// Icon: requires @resvg/resvg-wasm which is not resolvable under tsx/vitest
//       (the dist build path covers it; see issue #646 — out of scope).
// Image: samples fetch real URLs via prefetchImageSize, which would make the
//        test depend on network availability. Skip to keep the test hermetic.
const SKIP_SECTIONS = new Set(["`<Icon>`", "`<Image>`"]);

// Samples that use <Import src="..."/> require a caller-supplied resolver that
// the test has no way to mock; skip them.
// Samples rooted at <Fragment> are import-file snippets, not standalone documents.
function usesImport(xml: string): boolean {
  return /<Import\b/.test(xml) || /^\s*<Fragment\b/.test(xml);
}

// Samples that show syntax fragments (e.g. `<Style>`, `<Master>`, `<Slot>`,
// `<Use>`, `<If>`, `<Foreach>`, `<Notes>`) outside a containing slide are
// partial — they illustrate one element only, not a full standalone document.
function isPartialSnippet(xml: string): boolean {
  const trimmed = xml.trim();
  if (!trimmed.startsWith("<")) return true;
  const root = trimmed.slice(1).match(/^[A-Za-z]+/)?.[0];
  if (!root) return true;
  const STANDALONE_ROOTS = new Set([
    "SlideGlance",
    "VStack",
    "HStack",
    "Layer",
    "Slide",
    "Text",
    "Ul",
    "Ol",
    "Image",
    "Table",
    "Shape",
    "Chart",
    "Line",
    "Icon",
    "Svg",
  ]);
  return !STANDALONE_ROOTS.has(root);
}

const md = readFileSync(XML_REFERENCE_MD_PATH, "utf8");
const samples = extractXmlSamples(md);

describe("docs/en/xml-reference.md xml samples", () => {
  it("contains at least one xml sample", () => {
    expect(samples.length).toBeGreaterThan(0);
  });

  for (const sample of samples) {
    const skip =
      SKIP_SECTIONS.has(sample.section) ||
      usesImport(sample.xml) ||
      isPartialSnippet(sample.xml);
    const title = `[${sample.index}] ${sample.section} sample builds without diagnostics`;
    (skip ? it.skip : it)(title, async () => {
      const { diagnostics } = await buildPptx(sample.xml, { w: 1280, h: 720 });
      // Deprecation warnings from legacy attribute forms in samples are allowed.
      // Filter them so that only hard errors (parse failures, schema violations) cause test failure.
      const hardErrors = (diagnostics as Diagnostic[]).filter(
        (d) => d.code !== "DEPRECATED_ATTRIBUTE",
      );
      expect(hardErrors).toEqual([]);
    });
  }
});
