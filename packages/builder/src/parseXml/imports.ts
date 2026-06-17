// ===== <Import src="..."/> macro expansion =====
//
// Allows splitting a builder XML document across multiple files. <Import> is
// valid anywhere in the tree (top-level SlideGlance child, inside <Slide>,
// inside a container like <VStack>, etc). The imported file is loaded
// synchronously via a caller-supplied resolver and its content is inlined in
// place of the <Import> element.
//
// Each imported file must be a well-formed XML document with a single root
// element:
//   - <Fragment>...</Fragment>  — generic fragment wrapper. Its children are
//                                  inlined wherever the <Import> element sits.
//   - <SlideGlance>...</SlideGlance>  — accepted when the file is also
//                                  runnable as a standalone document.
//                                  Children are inlined; root attributes are
//                                  ignored (only the importing root document
//                                  controls slide size, masters, etc.).
//
// Any other root element (or multiple top-level elements) is rejected.
//
// Cycle detection uses the absolute path returned by the resolver. Recursive
// expansion is bounded by MAX_IMPORT_DEPTH.

import { XMLParser } from "fast-xml-parser";
import { injectSourceAttrs } from "./sourceInjection.ts";

type XmlTextNode = { "#text": string };
type XmlElement = {
  [tagName: string]: XmlNode[] | Record<string, string> | undefined;
  ":@"?: Record<string, string>;
};
type XmlNode = XmlElement | XmlTextNode;

const MAX_IMPORT_DEPTH = 16;

/**
 * Caller-supplied function that loads an imported file synchronously.
 *
 * @param src       The `src` attribute value as written on `<Import>`.
 * @param fromPath  The absolute path of the file containing this `<Import>`,
 *                  or `undefined` for the root document. The resolver should
 *                  use this to resolve relative paths.
 * @returns         An object with the file's text `content` and the absolute
 *                  `path` it resolved to (used for cycle detection).
 */
export type ImportResolver = (
  src: string,
  fromPath: string | undefined,
) => { content: string; path: string };

function isTextNode(node: XmlNode): node is XmlTextNode {
  return "#text" in node;
}

function getTagName(node: XmlElement): string {
  for (const key of Object.keys(node)) {
    if (key !== ":@") return key;
  }
  throw new Error("No tag name found in XML element");
}

function getRawChildren(node: XmlElement): XmlNode[] {
  const tagName = getTagName(node);
  return (node[tagName] as XmlNode[] | undefined) ?? [];
}

function getAttributes(node: XmlElement): Record<string, string> {
  const attrs: Record<string, string> = {};
  const rawAttrs = node[":@"];
  if (rawAttrs) {
    for (const [key, value] of Object.entries(rawAttrs)) {
      const attrName = key.startsWith("@_") ? key.slice(2) : key;
      attrs[attrName] = value;
    }
  }
  return attrs;
}

function buildElement(
  tag: string,
  attrs: Record<string, string>,
  children: XmlNode[],
): XmlElement {
  const el: XmlElement = { [tag]: children };
  if (Object.keys(attrs).length > 0) {
    const rawAttrs: Record<string, string> = {};
    for (const [k, v] of Object.entries(attrs)) {
      rawAttrs[`@_${k}`] = v;
    }
    el[":@"] = rawAttrs;
  }
  return el;
}

function parseImportedXml(
  xml: string,
  src: string,
  errors: string[],
  resolvedPath: string,
): XmlNode[] | null {
  const parser = new XMLParser({
    preserveOrder: true,
    ignoreAttributes: false,
    attributeNamePrefix: "@_",
    parseAttributeValue: false,
    parseTagValue: false,
    trimValues: false,
    processEntities: true,
  });
  // Inject source-position attributes keyed to the imported file path so that
  // descendants of this <Import> carry the correct origin in the BuilderSourceMap.
  const injected = injectSourceAttrs(xml, resolvedPath);
  // Strip the optional XML prolog so the wrap below stays well-formed.
  const stripped = injected.replace(/^\s*<\?xml[^?]*\?>\s*/, "");
  const wrapped = `<__root__>${stripped}</__root__>`;
  const parsed = parser.parse(wrapped) as XmlElement[];
  const rootElement = parsed?.[0];
  if (!rootElement) {
    errors.push(
      `<Import src="${src}">: imported file is empty; expected a single <Fragment> or <SlideGlance> root element`,
    );
    return null;
  }
  const children = (rootElement["__root__"] ?? []) as XmlNode[];
  const elementChildren = children.filter(
    (c): c is XmlElement => !isTextNode(c),
  );
  const rootEl = elementChildren[0];
  if (elementChildren.length !== 1 || !rootEl) {
    errors.push(
      `<Import src="${src}">: imported file must have exactly one root element (<Fragment> or <SlideGlance>); found ${elementChildren.length}`,
    );
    return null;
  }
  const rootTag = getTagName(rootEl);
  if (rootTag !== "Fragment" && rootTag !== "SlideGlance") {
    errors.push(
      `<Import src="${src}">: imported file root must be <Fragment> or <SlideGlance>, got <${rootTag}>`,
    );
    return null;
  }
  return getRawChildren(rootEl);
}

interface InlineContext {
  resolver: ImportResolver | undefined;
  fromPath: string | undefined;
  /** Absolute paths currently being expanded — guards against cycles. */
  visited: Set<string>;
  depth: number;
  errors: string[];
}

/**
 * Build a source-origin suffix from parser-injected __sourceLine / __sourceFile
 * attrs. Returns " (at line N of <file>)" when either value is present,
 * or an empty string when source tracking is disabled.
 */
function formatImportOrigin(attrs: Record<string, string>): string {
  const line = attrs.__sourceLine;
  const file = attrs.__sourceFile;
  if (!line && !file) return "";
  const filePart = file ? ` of ${file}` : "";
  return ` (at line ${line ?? "?"}${filePart})`;
}

function inlineImportElement(
  importEl: XmlElement,
  ctx: InlineContext,
): XmlNode[] {
  const attrs = getAttributes(importEl);
  const src = attrs.src?.trim();
  const origin = formatImportOrigin(attrs);
  if (!src) {
    ctx.errors.push(`<Import>: missing required attribute "src"${origin}`);
    return [];
  }
  if (!ctx.resolver) {
    ctx.errors.push(
      `<Import src="${src}">: no resolveImport function was provided when calling buildPptx/parseBuilderDocument${origin}`,
    );
    return [];
  }
  if (ctx.depth >= MAX_IMPORT_DEPTH) {
    ctx.errors.push(
      `<Import src="${src}">: import depth exceeded ${MAX_IMPORT_DEPTH}; possible circular import${origin}`,
    );
    return [];
  }
  let resolved: { content: string; path: string };
  try {
    resolved = ctx.resolver(src, ctx.fromPath);
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    ctx.errors.push(
      `<Import src="${src}">: failed to load — ${message}${origin}`,
    );
    return [];
  }
  if (ctx.visited.has(resolved.path)) {
    ctx.errors.push(
      `<Import src="${src}">: circular import detected at ${resolved.path}${origin}`,
    );
    return [];
  }

  const importedNodes = parseImportedXml(
    resolved.content,
    src,
    ctx.errors,
    resolved.path,
  );
  if (importedNodes === null) return [];
  ctx.visited.add(resolved.path);
  const expanded = inlineImportsInNodes(importedNodes, {
    ...ctx,
    fromPath: resolved.path,
    depth: ctx.depth + 1,
  });
  ctx.visited.delete(resolved.path);
  return expanded;
}

function inlineImportsInNodes(nodes: XmlNode[], ctx: InlineContext): XmlNode[] {
  const result: XmlNode[] = [];
  for (const node of nodes) {
    if (isTextNode(node)) {
      result.push(node);
      continue;
    }
    const tag = getTagName(node);
    if (tag === "Import") {
      for (const inlined of inlineImportElement(node, ctx)) {
        result.push(inlined);
      }
      continue;
    }
    const attrs = getAttributes(node);
    const children = inlineImportsInNodes(getRawChildren(node), ctx);
    result.push(buildElement(tag, attrs, children));
  }
  return result;
}

/**
 * Top-level entry. Inlines all `<Import>` elements anywhere in `nodes` using
 * the supplied resolver. Errors during expansion are pushed to `errors`.
 */
export function inlineImports(
  nodes: XmlNode[],
  resolver: ImportResolver | undefined,
  sourcePath: string | undefined,
  errors: string[],
): XmlNode[] {
  return inlineImportsInNodes(nodes, {
    resolver,
    fromPath: sourcePath,
    visited: new Set<string>(sourcePath ? [sourcePath] : []),
    depth: 0,
    errors,
  });
}
