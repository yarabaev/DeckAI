/**
 * Inline text-run extraction for <Text>, <Li>, and <Td>.
 *
 * Children that are entirely <B>/<I>/<U>/<S>/<A>/<Mark>/<Span> compose a
 * styled run sequence; otherwise they are rejected by the caller.
 */

import {
  type XmlElement,
  type XmlNode,
  decodeTextEscapes,
  getAttributes,
  getRawChildren,
  getTagName,
  isTextNode,
} from "./xml.ts";

export const INLINE_FORMAT_TAGS = new Set([
  "B",
  "I",
  "A",
  "U",
  "S",
  "Mark",
  "Span",
]);

type TextRunResult = {
  text: string;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strike?: boolean;
  highlight?: string;
  color?: string;
  href?: string;
  lang?: string;
};

/**
 * Collapse the source-code whitespace that lives inside multi-line Text
 * elements. Authors typically write
 *
 *   <Text>
 *     a long paragraph that spans
 *     two source lines.
 *   </Text>
 *
 * and (because the XML parser is configured with `trimValues:false` to
 * preserve intentional in-line whitespace inside e.g. monospace runs)
 * would otherwise see those newlines and the leading indent show up as
 * literal whitespace in the rendered slide.
 *
 * We collapse every run of (whitespace + newline + whitespace) to a
 * single space, which gives HTML-like "normal" whitespace semantics
 * for line-break source whitespace while leaving same-line indentation
 * (used by `<Text class="code-text">  some code</Text>` etc.) intact.
 */
const SOURCE_NEWLINE_RUN_RE = /[ \t]*\n\s*/g;
function collapseSourceNewlines(text: string): string {
  return text.replace(SOURCE_NEWLINE_RUN_RE, " ");
}

function hasInlineFormatChildren(childElements: XmlElement[]): boolean {
  return (
    childElements.length > 0 &&
    childElements.every((el) => INLINE_FORMAT_TAGS.has(getTagName(el)))
  );
}

function extractTextRuns(
  children: XmlNode[],
  inheritBold?: boolean,
  inheritItalic?: boolean,
  inheritHref?: string,
  inheritUnderline?: boolean,
  inheritStrike?: boolean,
  inheritHighlight?: string,
  inheritColor?: string,
  inheritLang?: string,
): TextRunResult[] {
  const runs: TextRunResult[] = [];
  for (const child of children) {
    if (isTextNode(child)) {
      // Order matters: collapse formatting newlines from the raw XML
      // first, THEN decode user escapes. If we decoded first, the
      // collapse would also eat author-inserted `\n` line breaks.
      const run: TextRunResult = {
        text: decodeTextEscapes(collapseSourceNewlines(child["#text"])),
      };
      if (inheritBold) run.bold = true;
      if (inheritItalic) run.italic = true;
      if (inheritUnderline) run.underline = true;
      if (inheritStrike) run.strike = true;
      if (inheritHighlight) run.highlight = inheritHighlight;
      if (inheritColor) run.color = inheritColor;
      if (inheritHref) run.href = inheritHref;
      if (inheritLang) run.lang = inheritLang;
      runs.push(run);
    } else {
      const tag = getTagName(child);
      const innerChildren = getRawChildren(child);
      if (tag === "B") {
        runs.push(
          ...extractTextRuns(
            innerChildren,
            true,
            inheritItalic,
            inheritHref,
            inheritUnderline,
            inheritStrike,
            inheritHighlight,
            inheritColor,
            inheritLang,
          ),
        );
      } else if (tag === "I") {
        runs.push(
          ...extractTextRuns(
            innerChildren,
            inheritBold,
            true,
            inheritHref,
            inheritUnderline,
            inheritStrike,
            inheritHighlight,
            inheritColor,
            inheritLang,
          ),
        );
      } else if (tag === "A") {
        const href = getAttributes(child).href ?? "";
        runs.push(
          ...extractTextRuns(
            innerChildren,
            inheritBold,
            inheritItalic,
            href,
            inheritUnderline,
            inheritStrike,
            inheritHighlight,
            inheritColor,
            inheritLang,
          ),
        );
      } else if (tag === "U") {
        runs.push(
          ...extractTextRuns(
            innerChildren,
            inheritBold,
            inheritItalic,
            inheritHref,
            true,
            inheritStrike,
            inheritHighlight,
            inheritColor,
            inheritLang,
          ),
        );
      } else if (tag === "S") {
        runs.push(
          ...extractTextRuns(
            innerChildren,
            inheritBold,
            inheritItalic,
            inheritHref,
            inheritUnderline,
            true,
            inheritHighlight,
            inheritColor,
            inheritLang,
          ),
        );
      } else if (tag === "Mark") {
        const rawColor = getAttributes(child).color;
        const color = rawColor && rawColor.trim() ? rawColor : "FFFF00";
        runs.push(
          ...extractTextRuns(
            innerChildren,
            inheritBold,
            inheritItalic,
            inheritHref,
            inheritUnderline,
            inheritStrike,
            color,
            inheritColor,
            inheritLang,
          ),
        );
      } else if (tag === "Span") {
        const rawSpanColor = getAttributes(child).color;
        const spanColor =
          rawSpanColor && rawSpanColor.trim() ? rawSpanColor : inheritColor;
        const rawSpanLang = getAttributes(child).lang;
        const spanLang =
          rawSpanLang && rawSpanLang.trim() ? rawSpanLang : inheritLang;
        runs.push(
          ...extractTextRuns(
            innerChildren,
            inheritBold,
            inheritItalic,
            inheritHref,
            inheritUnderline,
            inheritStrike,
            inheritHighlight,
            spanColor,
            spanLang,
          ),
        );
      }
    }
  }
  return runs;
}

export function buildRunsAndText(
  node: XmlElement,
): { runs: TextRunResult[]; text: string } | null {
  const rawChildren = getRawChildren(node);
  const childElements = rawChildren.filter(
    (c): c is XmlElement => !isTextNode(c),
  );
  if (!hasInlineFormatChildren(childElements)) return null;
  const runs = extractTextRuns(rawChildren);
  if (runs.length > 0) {
    const first = runs[0]!;
    runs[0] = {
      ...first,
      text: first.text.replace(/^[\s\uFEFF\u00A0]+/, ""),
    };
    const last = runs.length - 1;
    const lastRun = runs[last]!;
    runs[last] = {
      ...lastRun,
      text: lastRun.text.replace(/[\s\uFEFF\u00A0]+$/, ""),
    };
  }
  const text = runs.map((r) => r.text).join("");
  return { runs, text };
}
