/**
 * Low-level helpers for fast-xml-parser's preserveOrder output.
 *
 * The XML parser produces `XmlElement` objects keyed by tag name with a
 * special `:@` field carrying attributes. These helpers wrap that shape so
 * the rest of the parser does not depend on fast-xml-parser internals.
 */

import { allocateNextPomId, getCurrentSourceMap } from "./parseContext.ts";

export type XmlNode = XmlElement | XmlTextNode;
export type XmlTextNode = { "#text": string };
export interface XmlElement {
  [tagName: string]: XmlNode[] | Record<string, string> | undefined;
  ":@"?: Record<string, string>;
}

/**
 * Raw attribute names reserved for internal source-position tracking. These
 * are injected by `injectSourceAttrs` before XML parsing and must never be
 * surfaced to user-facing validation or POM node construction.
 */
const SOURCE_POS_ATTR_KEYS = new Set(["__sourceLine", "__sourceFile"]);

export function isTextNode(node: XmlNode): node is XmlTextNode {
  return "#text" in node;
}

/**
 * Decode user-friendly text escapes inside element body text.
 *
 * - `\n` → LF
 * - `\t` → TAB
 * - `\\` → literal backslash
 * - `\X` (any other char) → kept verbatim ("\X") so paths like `C:\Users\foo`
 *   stay readable.
 *
 * Applied to body text (Text / Shape / Td / Li / MasterText / Notes) and to
 * inline run text. Attribute values are left untouched because callers may
 * pre-encode JSON (`items='[…]'`, `chartColors='[…]'`, …) where `\n` already
 * has its own meaning inside the JSON string grammar.
 */
export function decodeTextEscapes(s: string): string {
  return s.replace(/\\(.)/g, (_, c: string) => {
    if (c === "n") return "\n";
    if (c === "t") return "\t";
    if (c === "\\") return "\\";
    return `\\${c}`;
  });
}

export function getTagName(node: XmlElement): string {
  for (const key of Object.keys(node)) {
    if (key !== ":@") return key;
  }
  throw new Error("No tag name found in XML element");
}

export function getAttributes(node: XmlElement): Record<string, string> {
  const attrs: Record<string, string> = {};
  const rawAttrs = node[":@"];
  if (rawAttrs) {
    for (const [key, value] of Object.entries(rawAttrs)) {
      const attrName = key.startsWith("@_") ? key.slice(2) : key;
      // Hide internal source-position attrs from all downstream validation.
      if (SOURCE_POS_ATTR_KEYS.has(attrName)) continue;
      attrs[attrName] = value.trim();
    }
  }
  return attrs;
}

/**
 * Read the `__sourceLine` / `__sourceFile` attributes directly from the parsed raw
 * attribute map, bypassing `getAttributes`'s filtering.
 */
function readRawSourcePos(node: XmlElement): {
  line: string | undefined;
  file: string | undefined;
} {
  const rawAttrs = node[":@"];
  if (!rawAttrs) return { line: undefined, file: undefined };
  return {
    line: rawAttrs["@___sourceLine"],
    file: rawAttrs["@___sourceFile"],
  };
}

/**
 * Return `{file, line}` for an element if injectSourceAttrs has run over it.
 * Used internally by `formatErrorAt`.
 */
function getSourcePos(
  node: XmlElement,
): { file: string | undefined; line: number } | undefined {
  const { line, file } = readRawSourcePos(node);
  if (!line) return undefined;
  const n = Number(line);
  if (!Number.isFinite(n)) return undefined;
  return { file: file || undefined, line: n };
}

/**
 * Prefix `message` with `file:line: ` (or `line N: ` when no file) when the
 * element carries source-position attributes. Returns `message` unchanged when
 * no position is available.
 */
export function formatErrorAt(node: XmlElement, message: string): string {
  const pos = getSourcePos(node);
  if (!pos) return message;
  const prefix = pos.file ? `${pos.file}:${pos.line}` : `line ${pos.line}`;
  return `${prefix}: ${message}`;
}

/**
 * Register the element's source position into the current parse's sourceMap
 * and return the allocated POM id (or undefined if no position is present).
 * Used wherever an XmlElement maps to a BuilderNode (or a slide root).
 */
export function registerSourcePosForElement(
  node: XmlElement,
): number | undefined {
  const sourceMap = getCurrentSourceMap();
  const { line, file } = readRawSourcePos(node);
  if (!sourceMap || !line) return undefined;
  const n = Number(line);
  if (!Number.isFinite(n)) return undefined;
  const id = allocateNextPomId();
  sourceMap.set(id, { file: file || undefined, line: n });
  return id;
}

export function getChildElements(node: XmlElement): XmlElement[] {
  const tagName = getTagName(node);
  const children = node[tagName] as XmlNode[] | undefined;
  if (!children) return [];
  return children.filter((child): child is XmlElement => !isTextNode(child));
}

export function getTextContent(node: XmlElement): string | undefined {
  const tagName = getTagName(node);
  const children = node[tagName] as XmlNode[] | undefined;
  if (!children) return undefined;
  const textParts: string[] = [];
  for (const child of children) {
    if (isTextNode(child)) {
      textParts.push(child["#text"]);
    }
  }
  if (textParts.length === 0) return undefined;
  const joined = textParts.join("");
  // Normalize source-code whitespace from multi-line authored content.
  // When the content spans multiple lines (i.e. contains a newline), the
  // surrounding whitespace and every internal newline+indent run are XML
  // indentation, not authored content. Strip the surrounding whitespace
  // AND collapse internal newline+indent runs to a single space — this
  // mirrors HTML's "normal" whitespace rule and matches the inline
  // text-run handler in textRuns.ts (`collapseSourceNewlines`).
  // Single-line content (including " " or "  " used intentionally for
  // spacing, common in monospace runs) is preserved verbatim.
  // Order matters: strip + collapse the raw XML newlines FIRST, then
  // decode `\n` escapes. Doing the inverse would eat author-inserted
  // line breaks that arrived as the `\n` escape sequence.
  const normalized = joined.includes("\n")
    ? joined
        .replace(/^[\s\uFEFF\u00A0]+|[\s\uFEFF\u00A0]+$/g, "")
        .replace(/[ \t]*\n\s*/g, " ")
    : joined;
  if (normalized.length === 0) return undefined;
  return decodeTextEscapes(normalized);
}

export function getRawChildren(node: XmlElement): XmlNode[] {
  const tagName = getTagName(node);
  return (node[tagName] as XmlNode[] | undefined) ?? [];
}

export function parseClassNames(attrs: Record<string, string>): string[] {
  const raw = [attrs.class, attrs.className]
    .filter(
      (value): value is string => typeof value === "string" && value.length > 0,
    )
    .join(" ")
    .trim();
  if (!raw) return [];
  return raw.split(/\s+/).filter(Boolean);
}
