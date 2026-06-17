/**
 * Parse-phase scanner: detects bare `<` / `>` characters inside XML
 * attribute values. fast-xml-parser silently accepts these (verified —
 * see `RAW_LT_GT_IN_ATTR` rule docs), so a downstream lint pass is the
 * only place the violation surfaces. Bare angle brackets inside
 * attribute values are forbidden by the XML 1.0 spec (§3.1) and trip
 * stricter consumers (PowerPoint's own parser, Schematron tools, IDE
 * linting); authors should use `&lt;` / `&gt;` instead.
 *
 * Strategy:
 *   1. Mask out non-content regions (`<!-- … -->`, `<![CDATA[ … ]]>`,
 *      `<?xml … ?>`) with spaces so they cannot produce false positives
 *      while preserving offsets for line-number reporting.
 *   2. Walk every tag-open via a regex that respects quoted attribute
 *      values; the regex never consumes a tag-closing `>` from outside
 *      a quoted string.
 *   3. Within each tag body, extract `attr="value"` / `attr='value'`
 *      pairs and report any value containing a bare `<` or `>`.
 *
 * The scanner is deliberately conservative: when a region is ambiguous
 * (e.g. malformed XML the parser would also reject), it emits nothing
 * rather than guess at intent.
 */

import type { Diagnostic } from "../../diagnostics.ts";

const COMMENT_RE = /<!--[\s\S]*?-->/g;
const CDATA_RE = /<!\[CDATA\[[\s\S]*?\]\]>/g;
const PI_RE = /<\?[\s\S]*?\?>/g;

const TAG_OPEN_RE =
  /<([A-Za-z_][\w:.-]*)((?:[^"'/>]|"[^"]*"|'[^']*')*)\s*\/?>/g;
const ATTR_RE = /([A-Za-z_][\w:.-]*)\s*=\s*(?:"([^"]*)"|'([^']*)')/g;

function maskRange(input: string, regex: RegExp): string {
  return input.replace(regex, (m) => " ".repeat(m.length));
}

function buildLineIndex(content: string): number[] {
  const starts: number[] = [0];
  for (let i = 0; i < content.length; i++) {
    if (content.charCodeAt(i) === 0x0a) starts.push(i + 1);
  }
  return starts;
}

function offsetToLine(starts: readonly number[], offset: number): number {
  let lo = 0;
  let hi = starts.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if ((starts[mid] ?? 0) <= offset) lo = mid;
    else hi = mid - 1;
  }
  return lo + 1;
}

function summarize(value: string): string {
  const single = value.replace(/\s+/g, " ").trim();
  return single.length > 60 ? `${single.slice(0, 60)}…` : single;
}

/**
 * Scan one raw XML source for attribute values containing bare `<` / `>`.
 * `path` (when present) is attached to each diagnostic's `sourcePos.file`
 * so multi-file builds (with `<Import>`) can pinpoint the offender.
 */
export function scanRawXmlForBareLtGt(
  content: string,
  path?: string,
): Diagnostic[] {
  if (!content) return [];

  // Mask non-content regions so their inner `<`/`>` cannot leak into
  // tag-open matches. Lengths are preserved so offsets stay aligned
  // with the original source for line-number reporting.
  let masked = content;
  masked = maskRange(masked, COMMENT_RE);
  masked = maskRange(masked, CDATA_RE);
  masked = maskRange(masked, PI_RE);

  const lineStarts = buildLineIndex(content);
  const diagnostics: Diagnostic[] = [];

  for (const tagMatch of masked.matchAll(TAG_OPEN_RE)) {
    const tagName = tagMatch[1] ?? "";
    const tagBody = tagMatch[2] ?? "";
    const tagMatchIndex = tagMatch.index ?? 0;
    // Where the tag body starts inside `content`: after `<` + tag name.
    const tagBodyStart = tagMatchIndex + 1 + tagName.length;

    for (const attrMatch of tagBody.matchAll(ATTR_RE)) {
      const attrName = attrMatch[1] ?? "";
      const rawValue = attrMatch[2] ?? attrMatch[3] ?? "";
      const hasBareLt = rawValue.includes("<");
      const hasBareGt = rawValue.includes(">");
      if (!hasBareLt && !hasBareGt) continue;

      const offending: string[] = [];
      if (hasBareLt) offending.push("<");
      if (hasBareGt) offending.push(">");

      const attrMatchIndex = attrMatch.index ?? 0;
      const attrOffset = tagBodyStart + attrMatchIndex;
      const line = offsetToLine(lineStarts, attrOffset);
      const fixedValue = rawValue.replace(/</g, "&lt;").replace(/>/g, "&gt;");
      const replacement =
        hasBareLt && hasBareGt ? "&lt; / &gt;" : hasBareLt ? "&lt;" : "&gt;";

      diagnostics.push({
        code: "RAW_LT_GT_IN_ATTR",
        severity: "warn",
        message:
          `<${tagName}> attribute ${attrName}="${summarize(rawValue)}" contains bare ` +
          `${offending.join(" / ")} — use ${replacement} instead. ` +
          `fast-xml-parser is lenient here but the XML 1.0 spec forbids unescaped angle brackets ` +
          `in attribute values, and PowerPoint / strict consumers will reject the document.`,
        nodeType: tagName,
        sourcePos: path ? { line, file: path } : { line },
        context: {
          attribute: attrName,
          value: rawValue,
          characters: offending,
        },
        suggestedFix: {
          kind: "attribute-set",
          target: "self",
          set: { [attrName]: fixedValue },
          notes:
            "Replace bare `<` with `&lt;` and bare `>` with `&gt;` so the document remains well-formed XML.",
        },
      });
    }
  }

  return diagnostics;
}
