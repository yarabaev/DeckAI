/**
 * Document-level parser: turns an XML string into a `ParsedBuilderDocument`.
 *
 * Handles `<SlideGlance>` (with `<Document>`/`<Templates>`/`<Styles>`/
 * `<Master>`/`<Slide>`), implicit slide siblings, and bare-fragment roots.
 * Document-level settings (slide size, defaultMaster, defaultTextStyle) live
 * on the `<Document>` child rather than on the root. Per-element conversion
 * is delegated to `dispatcher.ts`.
 */

import { XMLParser } from "fast-xml-parser";

import type { Diagnostic } from "../diagnostics.ts";
import {
  type DefaultTextStyle,
  type BuilderNode,
  type SlideMasterOptions,
  masterObjectSchema,
  slideNumberOptionsSchema,
} from "../types.ts";
import { coerceByType, type CoerceResult } from "./coerceByType.ts";
import { coerceChildAttrs } from "./coerceAttrs.ts";
import { convertElement } from "./dispatcher.ts";
import { inlineImports, type ImportResolver } from "./imports.ts";
import type { BuilderSourceMap } from "./parseContext.ts";
import { injectSourceAttrs } from "./sourceInjection.ts";
import {
  equalizeAll,
  collectStylesMap,
  type StyleEntry,
} from "../preprocess/equalize.ts";
import { collectStyles, type StyleRegistry } from "./styles.ts";
import {
  DEFAULT_MAX_TEMPLATE_NODES,
  collectTemplates,
  expandTemplatesInNodes,
} from "./templates.ts";
import { appendSchemaErrors } from "./validation.ts";
import { validateConnectorsInSlides } from "./validateConnectors.ts";
import {
  type XmlElement,
  type XmlNode,
  formatErrorAt,
  getAttributes,
  getChildElements,
  getTagName,
  getTextContent,
  isTextNode,
  registerSourcePosForElement,
} from "./xml.ts";

export class ParseXmlError extends Error {
  public readonly errors: string[];
  constructor(errors: string[]) {
    const message = `XML validation failed (${errors.length} error${errors.length > 1 ? "s" : ""}):\n${errors.map((e) => `  - ${e}`).join("\n")}`;
    super(message);
    this.name = "ParseXmlError";
    this.errors = errors;
  }
}

export interface ParseBuilderDocumentOptions {
  /** Sync function that loads imported files. Required if the document uses
   *  `<Import>`; otherwise omit. See {@link ImportResolver}. */
  resolveImport?: ImportResolver;
  /** Absolute path of the root document. Passed to `resolveImport` so that
   *  relative `<Import src="...">` paths can be resolved. */
  sourcePath?: string;
  /**
   * If true, pre-process the XML to attach source-position metadata to each
   * node via BuilderNode.__nodeId and return a `sourceMap` in the result.
   */
  trackSourcePos?: boolean;
  /** Maximum number of nodes that template expansion may produce. Default: 100,000. */
  maxTemplateNodes?: number;
  /**
   * When true, run the equalize-dimensions preprocessor before parsing. The
   * preprocessor recursively inlines all imports at text level and
   * substitutes the sentinel attribute values used by the reference deck:
   *
   *   - `auto` on `<Use>` template params (`titleH="auto"`, `bodyH="auto"` …)
   *     resolves to the worst-case sibling height inside the same `<HStack>`.
   *   - `auto:KEY` on `<Text>` widths and table `<Col>` widths resolves to
   *     the natural width of the widest sibling sharing that KEY.
   *   - `capbar:CLASS` on a `<VStack>` height resolves to the cap-baseline
   *     height of the referenced typography class.
   *
   * When this option is set, the parser skips its tree-level import inline
   * pass since all imports are already resolved at text level. Callers using
   * `equalize: true` should still pass `resolveImport` for the import loader.
   */
  equalize?: boolean;
}

export interface ParsedBuilderDocument {
  nodes: BuilderNode[];
  slideSize?: { w: number; h: number };
  masters?: SlideMasterOptions[];
  masterContents?: Record<string, BuilderNode[]>;
  defaultMaster?: string;
  defaultTextStyle?: DefaultTextStyle;
  /**
   * Maps BuilderNode.__nodeId -> origin {file, line}. Present even when empty so
   * callers can rely on the field existing.
   */
  sourceMap?: BuilderSourceMap;
  /**
   * Names of `<Style name="…"/>` declared in the deck (drives UNUSED_STYLE lint).
   */
  declaredStyles?: Set<string>;
  /** Style names referenced via class="…" anywhere in the deck. */
  referencedStyles?: Set<string>;
  /** Names of `<Template name="…"/>` declared in the deck. */
  declaredTemplates?: Set<string>;
  /** Names of templates actually invoked via `<Use template="…"/>`. */
  referencedTemplates?: Set<string>;
}

const DOCUMENT_SIZE_PRESETS: Record<string, { w: number; h: number }> = {
  "16:9": { w: 1280, h: 720 },
  "4:3": { w: 1024, h: 768 },
  A4: { w: 793, h: 1122 },
  A3: { w: 1122, h: 1587 },
  Letter: { w: 816, h: 1056 },
};

/**
 * Coerce a Document default-text-style field. Most fields are plain
 * scalars; `underline` is a boolean-or-JSON union (matches the legacy
 * `coerceUnionWithRules({type:"union",options:["boolean","json"]})`).
 */
function coerceDefaultTextStyleField(
  key: keyof DefaultTextStyle,
  value: string,
): CoerceResult {
  switch (key) {
    case "fontFamily":
    case "color":
    case "highlight":
      return { value, error: null };
    case "fontSize":
    case "lineHeight":
      return coerceByType(value, "number");
    case "bold":
    case "italic":
    case "strike":
      return coerceByType(value, "boolean");
    case "underline":
      if (value === "true") return { value: true, error: null };
      if (value === "false") return { value: false, error: null };
      if (value.startsWith("{") || value.startsWith("[")) {
        try {
          return { value: JSON.parse(value), error: null };
        } catch {
          /* fall through */
        }
      }
      return {
        value: undefined,
        error: `Cannot convert "${value}" — no union option matched`,
      };
    default: {
      const exhaustive: never = key;
      void exhaustive;
      return { value, error: null };
    }
  }
}

const DOCUMENT_DEFAULT_TEXT_STYLE_KEYS: ReadonlyArray<keyof DefaultTextStyle> =
  [
    "fontFamily",
    "fontSize",
    "color",
    "bold",
    "italic",
    "underline",
    "strike",
    "highlight",
    "lineHeight",
  ];

// Strip an optional XML prolog (`<?xml … ?>`) and any leading whitespace. The
// prolog only carries encoding/version metadata that we already control via
// the input string's encoding; keeping it confuses the wrapped-root reparse
// below since fast-xml-parser then sees the prolog mid-document.
const XML_PROLOG_RE = /^\s*<\?xml[^?]*\?>\s*/;

function parseXmlRootChildren(xmlString: string): XmlNode[] {
  const parser = new XMLParser({
    preserveOrder: true,
    ignoreAttributes: false,
    attributeNamePrefix: "@_",
    parseAttributeValue: false,
    parseTagValue: false,
    trimValues: false,
    processEntities: true,
  });

  const stripped = xmlString.replace(XML_PROLOG_RE, "");
  const wrappedXml = `<__root__>${stripped}</__root__>`;
  const parsed: XmlElement[] = parser.parse(wrappedXml) as XmlElement[];

  const rootElement = parsed?.[0];
  if (!rootElement) return [];
  return (rootElement["__root__"] ?? []) as XmlNode[];
}

function convertRootChildrenToNodes(
  rootChildren: XmlNode[],
  errors: string[],
  styles: StyleRegistry = {},
): BuilderNode[] {
  return rootChildren
    .filter(
      (child): child is XmlElement =>
        !isTextNode(child) && getTagName(child) !== "Styles",
    )
    .map((child) => convertElement(child, errors, styles))
    .filter(
      (child): child is Record<string, unknown> => child !== null,
    ) as BuilderNode[];
}

/**
 * Recursively walk `nodes` (SlideGlance children with root-level <Templates>
 * already removed) and emit a TEMPLATES_NOT_AT_ROOT diagnostic for every
 * <Templates> block found anywhere in the subtree.
 */
function emitNonRootTemplatesDiagnostics(
  nodes: XmlElement[],
  diagnostics: Diagnostic[],
): void {
  for (const node of nodes) {
    const children = getChildElements(node);
    for (const child of children) {
      if (getTagName(child) === "Templates") {
        diagnostics.push({
          code: "TEMPLATES_NOT_AT_ROOT",
          message:
            "<Templates> must be a direct child of <SlideGlance>; nested <Templates> blocks are ignored",
        });
      } else {
        emitNonRootTemplatesDiagnostics([child], diagnostics);
      }
    }
  }
}

function parseDocumentSize(
  documentEl: XmlElement | undefined,
  attrs: Record<string, string>,
  errors: string[],
): { w: number; h: number } | undefined {
  const at = (msg: string): string =>
    documentEl ? formatErrorAt(documentEl, msg) : msg;
  const sizePreset = attrs.size?.trim();
  // w/h are canonical aliases for width/height (no deprecation — intentional).
  const hasWH = attrs.w !== undefined || attrs.h !== undefined;
  const hasWidthHeight =
    attrs.width !== undefined || attrs.height !== undefined;
  const effectiveWidth = attrs.width ?? attrs.w;
  const effectiveHeight = attrs.height ?? attrs.h;
  const hasCustomSize =
    effectiveWidth !== undefined || effectiveHeight !== undefined;

  if (sizePreset && hasCustomSize) {
    errors.push(
      at('<Document>: Use either "size" or "width"/"height", not both'),
    );
    return undefined;
  }

  if (hasWH && hasWidthHeight) {
    errors.push(
      at('<Document>: Use either "width"/"height" or "w"/"h", not both'),
    );
    return undefined;
  }

  if (sizePreset) {
    const preset = DOCUMENT_SIZE_PRESETS[sizePreset];
    if (!preset) {
      errors.push(
        at(
          '<Document>: Invalid value for attribute "size". Expected: "16:9", "4:3", "A4", "A3", "Letter"',
        ),
      );
      return undefined;
    }
    return preset;
  }

  if (effectiveWidth === undefined && effectiveHeight === undefined) {
    return undefined;
  }
  if (effectiveWidth === undefined || effectiveHeight === undefined) {
    errors.push(
      at(
        '<Document>: Both "width" and "height" are required when using a custom slide size',
      ),
    );
    return undefined;
  }

  const width = Number(effectiveWidth);
  const height = Number(effectiveHeight);

  if (Number.isNaN(width) || width <= 0) {
    errors.push(
      at(
        `<Document>: Invalid value for attribute "width". Cannot convert "${effectiveWidth}" to number`,
      ),
    );
  }
  if (Number.isNaN(height) || height <= 0) {
    errors.push(
      at(
        `<Document>: Invalid value for attribute "height". Cannot convert "${effectiveHeight}" to number`,
      ),
    );
  }
  if (
    Number.isNaN(width) ||
    Number.isNaN(height) ||
    width <= 0 ||
    height <= 0
  ) {
    return undefined;
  }
  return { w: width, h: height };
}

function parseDocumentDefaultTextStyle(
  documentEl: XmlElement | undefined,
  attrs: Record<string, string>,
  errors: string[],
): DefaultTextStyle | undefined {
  const result: Record<string, unknown> = {};
  for (const key of DOCUMENT_DEFAULT_TEXT_STYLE_KEYS) {
    const raw = attrs[key];
    if (raw === undefined) continue;
    const coerced = coerceDefaultTextStyleField(key, raw);
    if (coerced.error !== null) {
      const msg = `<Document>: Invalid value for attribute "${key}". ${coerced.error}`;
      errors.push(documentEl ? formatErrorAt(documentEl, msg) : msg);
      continue;
    }
    result[key] = coerced.value;
  }
  return Object.keys(result).length > 0 ? result : undefined;
}

function parseMasterElement(
  masterElement: XmlElement,
  errors: string[],
  styles: StyleRegistry,
): { master: SlideMasterOptions; contentNodes: BuilderNode[] } | null {
  const attrs = coerceChildAttrs(
    "SlideGlance",
    "Master",
    getAttributes(masterElement),
    errors,
    styles,
  );
  const name = typeof attrs.name === "string" ? attrs.name : undefined;

  if (!name) {
    errors.push(
      formatErrorAt(
        masterElement,
        '<Master>: Missing required attribute "name"',
      ),
    );
    return null;
  }

  const backgroundKeys = [
    "backgroundColor",
    "backgroundPath",
    "backgroundData",
  ].filter((key) => attrs[key] !== undefined);
  if (backgroundKeys.length > 1) {
    errors.push(
      formatErrorAt(
        masterElement,
        '<Master>: Only one of "backgroundColor", "backgroundPath", or "backgroundData" can be specified',
      ),
    );
  }

  const master: SlideMasterOptions = { title: name };
  const contentNodes: BuilderNode[] = [];

  if (attrs.margin !== undefined) {
    master.margin = attrs.margin as SlideMasterOptions["margin"];
  }
  if (typeof attrs.backgroundColor === "string") {
    master.background = { color: attrs.backgroundColor };
  } else if (typeof attrs.backgroundPath === "string") {
    master.background = { path: attrs.backgroundPath };
  } else if (typeof attrs.backgroundData === "string") {
    master.background = { data: attrs.backgroundData };
  }

  const objects: SlideMasterOptions["objects"] = [];

  for (const child of getChildElements(masterElement)) {
    const tag = getTagName(child);

    if (tag === "SlideNumber") {
      const slideNumber = coerceChildAttrs(
        "Master",
        tag,
        getAttributes(child),
        errors,
        styles,
      );
      appendSchemaErrors(
        slideNumberOptionsSchema.safeParse(slideNumber),
        "SlideNumber",
        errors,
        child,
      );
      if (!master.slideNumber) {
        master.slideNumber = slideNumber as SlideMasterOptions["slideNumber"];
      } else {
        errors.push(
          formatErrorAt(
            child,
            "<Master>: Only one <SlideNumber> child is allowed",
          ),
        );
      }
      continue;
    }

    let obj: Record<string, unknown>;
    if (tag === "MasterText") {
      obj = coerceChildAttrs(
        "Master",
        tag,
        getAttributes(child),
        errors,
        styles,
      );
      const textContent = getTextContent(child);
      if (textContent !== undefined && !("text" in obj)) {
        obj.text = textContent;
      }
      obj.type = "text";
    } else if (tag === "MasterImage") {
      obj = {
        ...coerceChildAttrs(
          "Master",
          tag,
          getAttributes(child),
          errors,
          styles,
        ),
        type: "image",
      };
    } else if (tag === "MasterRect") {
      obj = {
        ...coerceChildAttrs(
          "Master",
          tag,
          getAttributes(child),
          errors,
          styles,
        ),
        type: "rect",
      };
    } else if (tag === "MasterLine") {
      const raw: Record<string, unknown> = coerceChildAttrs(
        "Master",
        tag,
        getAttributes(child),
        errors,
        styles,
      );
      // Endpoint-pair → positioned-rect fold. (x1, y1) → (x, y),
      // (x2 - x1, y2 - y1) → (w, h). pptxgenjs `line` shape accepts
      // signed w/h offsets so diagonal lines work without explicit
      // rotation. Direct (x, y, w, h) takes precedence when both forms
      // are mixed.
      if (
        raw.x === undefined &&
        raw.y === undefined &&
        raw.w === undefined &&
        raw.h === undefined &&
        typeof raw.x1 === "number" &&
        typeof raw.y1 === "number" &&
        typeof raw.x2 === "number" &&
        typeof raw.y2 === "number"
      ) {
        raw.x = raw.x1;
        raw.y = raw.y1;
        raw.w = raw.x2 - raw.x1;
        raw.h = raw.y2 - raw.y1;
      }
      delete raw.x1;
      delete raw.y1;
      delete raw.x2;
      delete raw.y2;
      obj = { ...raw, type: "line" };
    } else {
      const converted = convertElement(child, errors, styles);
      if (converted) {
        contentNodes.push(converted as BuilderNode);
      }
      continue;
    }

    appendSchemaErrors(masterObjectSchema.safeParse(obj), tag, errors, child);
    objects.push(obj as NonNullable<SlideMasterOptions["objects"]>[number]);
  }

  if (objects.length > 0) {
    master.objects = objects;
  }
  return { master, contentNodes };
}

function parseSlideElement(
  slideElement: XmlElement,
  errors: string[],
  styles: StyleRegistry,
): BuilderNode | null {
  const attrs = getAttributes(slideElement);
  // Register the <Slide> tag's source position so the slide node reports the
  // <Slide> line (more intuitive for thumbnail clicks) rather than the child
  // root node's line.
  const slideNodeId = registerSourcePosForElement(slideElement);
  const rawChildElements = getChildElements(slideElement);
  const notesElements = rawChildElements.filter(
    (child) => getTagName(child) === "Notes",
  );
  const childElements = rawChildElements.filter(
    (child) => getTagName(child) !== "Notes",
  );
  const allowedAttrs = new Set(["master"]);

  for (const key of Object.keys(attrs)) {
    if (!allowedAttrs.has(key)) {
      errors.push(
        formatErrorAt(
          slideElement,
          `<Slide>: Unknown attribute "${key}"${key === "class" || key === "className" ? ". <Slide> is a container and does not support style classes" : ""}`,
        ),
      );
    }
  }

  if (notesElements.length > 1) {
    errors.push(
      formatErrorAt(slideElement, "<Slide>: Only one <Notes> child is allowed"),
    );
  }

  const firstNotesEl = notesElements[0];
  const notesFromChild = firstNotesEl
    ? (getTextContent(firstNotesEl)?.trim() ?? "")
    : undefined;
  for (const notesEl of notesElements) {
    for (const key of Object.keys(getAttributes(notesEl))) {
      errors.push(
        formatErrorAt(notesEl, `<Notes>: Unknown attribute "${key}"`),
      );
    }
    const nested = getChildElements(notesEl);
    if (nested.length > 0) {
      errors.push(
        formatErrorAt(
          notesEl,
          "<Notes>: Unexpected child elements. <Notes> only accepts text content",
        ),
      );
    }
  }

  const child = childElements[0];
  if (childElements.length !== 1 || !child) {
    errors.push(
      formatErrorAt(
        slideElement,
        `<Slide>: Expected exactly 1 child slide root node, but found ${childElements.length}`,
      ),
    );
    return null;
  }

  const tag = getTagName(child);
  if (tag === "Master" || tag === "Styles" || tag === "Slide") {
    errors.push(
      formatErrorAt(
        child,
        `<Slide>: Invalid child element <${tag}>. Expected a slide root node such as <VStack>, <HStack>, or <Layer>`,
      ),
    );
    return null;
  }

  // T10: master/notes were removed from BASE_RULES. The Slide root child may
  // still legally carry them (Slide-level slot), so strip from the child's raw
  // attribute map before generic coercion and re-attach below.
  const childRawAttrs = child[":@"];
  let childMaster: string | undefined;
  let childNotes: string | undefined;
  if (childRawAttrs) {
    if (typeof childRawAttrs["@_master"] === "string") {
      childMaster = childRawAttrs["@_master"].trim();
      delete childRawAttrs["@_master"];
    }
    if (typeof childRawAttrs["@_notes"] === "string") {
      childNotes = childRawAttrs["@_notes"].trim();
      delete childRawAttrs["@_notes"];
    }
  }
  const converted = convertElement(child, errors, styles);
  if (!converted) return null;

  const slideNode = converted as BuilderNode;
  if (childMaster !== undefined) {
    (slideNode as Record<string, unknown>).master = childMaster;
  }
  if (childNotes !== undefined) {
    (slideNode as Record<string, unknown>).notes = childNotes;
  }
  if (slideNodeId !== undefined) {
    // Prefer the <Slide> tag's source position over the child root's.
    slideNode.__nodeId = slideNodeId;
  }
  const slideMaster =
    typeof attrs.master === "string" && attrs.master.trim().length > 0
      ? attrs.master.trim()
      : undefined;
  const slideNotes = notesFromChild;

  if (!slideMaster && slideNotes === undefined) {
    return slideNode;
  }
  if (
    "master" in slideNode &&
    typeof slideNode.master === "string" &&
    slideNode.master !== slideMaster
  ) {
    errors.push(
      formatErrorAt(
        slideElement,
        `<Slide>: Conflicting master assignment. <Slide master="${slideMaster}"> conflicts with child node master="${slideNode.master}"`,
      ),
    );
    return slideNode;
  }
  return {
    ...slideNode,
    ...(slideMaster ? { master: slideMaster } : {}),
    ...(slideNotes !== undefined ? { notes: slideNotes } : {}),
  };
}

// Recursively inline `<Import src="..."/>` elements at text level. Used by
// the equalize pre-pass: equalize needs every chapter visible in a single
// buffer to scope HStack groups and resolve `<Style>` references.
// Match self-closing `<Import ...attrs.../>`. The attrs body may contain `/`
// (e.g. `src="styles/colors.xml"`); we only stop at the closing `/>`.
const IMPORT_TAG_RE = /<Import\s+([^>]*?)\/>/g;
const IMPORT_ATTR_RE = /([a-zA-Z_][a-zA-Z0-9_-]*)\s*=\s*("([^"]*)"|'([^']*)')/g;

function inlineImportsAsText(
  rootText: string,
  resolver: ImportResolver | undefined,
  fromPath: string | undefined,
  visited: Set<string>,
  errors: string[],
  trackSource: boolean,
): string {
  if (!resolver) return rootText;
  return rootText.replace(
    IMPORT_TAG_RE,
    (_match: string, attrsRaw: string): string => {
      const attrs: Record<string, string> = {};
      for (const m of attrsRaw.matchAll(IMPORT_ATTR_RE)) {
        const key = m[1];
        if (!key) continue;
        attrs[key] = m[3] ?? m[4] ?? "";
      }
      const src = attrs.src?.trim();
      if (!src) {
        errors.push('<Import>: missing required attribute "src"');
        return "";
      }
      let resolved: { content: string; path: string };
      try {
        resolved = resolver(src, fromPath);
      } catch (e) {
        const message = e instanceof Error ? e.message : String(e);
        errors.push(`<Import src="${src}">: failed to load — ${message}`);
        return "";
      }
      if (visited.has(resolved.path)) {
        errors.push(
          `<Import src="${src}">: circular import detected at ${resolved.path}`,
        );
        return "";
      }
      visited.add(resolved.path);
      let importedText = resolved.content;
      if (trackSource) {
        importedText = injectSourceAttrs(importedText, resolved.path);
      }
      importedText = importedText.replace(XML_PROLOG_RE, "");
      importedText = inlineImportsAsText(
        importedText,
        resolver,
        resolved.path,
        visited,
        errors,
        trackSource,
      );
      visited.delete(resolved.path);
      // Strip the outer <Fragment> / <SlideGlance> wrapper so the inner children
      // splice into the parent in place of the <Import> element.
      const wrapMatch = importedText.match(
        /^\s*<(Fragment|SlideGlance)\b[^>]*>([\s\S]*)<\/\1>\s*$/,
      );
      return wrapMatch && wrapMatch[2] !== undefined
        ? wrapMatch[2]
        : importedText;
    },
  );
}

/**
 * Inner document parser. Caller is responsible for managing parse-context
 * lifetime (see `parseBuilderDocument` in parseXml.ts).
 */
export function parseBuilderDocumentInner(
  xmlString: string,
  options: ParseBuilderDocumentOptions | undefined,
  sourceMap: BuilderSourceMap | null,
  diagnostics: Diagnostic[],
): ParsedBuilderDocument {
  // Source-position attributes are always injected so error messages can
  // include `file:line:` even when the caller has not opted into the full
  // sourceMap via trackSourcePos. Cost is one regex pass over the XML text.
  const track = sourceMap !== null;
  const injectedRoot = injectSourceAttrs(xmlString, options?.sourcePath);

  let rootChildren: XmlNode[];
  if (options?.equalize) {
    // Text-level inline of every <Import> + equalize pre-pass. The runtime
    // parser then sees a single self-contained buffer with no <Import>
    // markers remaining.
    const importErrors: string[] = [];
    const merged = inlineImportsAsText(
      injectedRoot,
      options?.resolveImport,
      options?.sourcePath,
      new Set<string>(),
      importErrors,
      track,
    );
    if (importErrors.length > 0) {
      throw new ParseXmlError(importErrors);
    }
    // Strip XML comments before equalize — comments may contain pseudo-tag
    // text like `<HStack>` inside docstrings which would otherwise confuse
    // the regex-based block scanner.
    const stripped = merged.replace(/<!--[\s\S]*?-->/g, "");
    const styles: Map<string, StyleEntry> = collectStylesMap(stripped);
    const equalized = equalizeAll(stripped, undefined, styles);
    rootChildren = parseXmlRootChildren(equalized);
  } else {
    const importErrors: string[] = [];
    const inlinedRoot = inlineImports(
      parseXmlRootChildren(injectedRoot),
      options?.resolveImport,
      options?.sourcePath,
      importErrors,
    );
    if (importErrors.length > 0) {
      throw new ParseXmlError(importErrors);
    }
    rootChildren = inlinedRoot;
  }
  const elementChildren = rootChildren.filter(
    (child): child is XmlElement => !isTextNode(child),
  );

  const firstElement = elementChildren[0];
  if (
    elementChildren.length === 1 &&
    firstElement &&
    getTagName(firstElement) === "SlideGlance"
  ) {
    return parseSlideGlance(firstElement, options, sourceMap, diagnostics);
  }

  const errors: string[] = [];
  const styles = collectStyles(elementChildren, errors);
  const nodes = convertRootChildrenToNodes(rootChildren, errors, styles);

  if (errors.length > 0) {
    throw new ParseXmlError(errors);
  }
  // Slide-scope id / connector validation for the bare-fragment path
  // (no <SlideGlance> wrapper). Mirrors the SlideGlance branch so
  // every entry point through parseBuilderDocumentInner emits the same
  // diagnostics and the renderer never sees a dangling Connector.
  validateConnectorsInSlides(nodes, diagnostics);

  const declaredStyles = new Set(Object.keys(styles));
  const referencedStyles = collectClassRefs(elementChildren);
  return {
    nodes,
    ...(sourceMap ? { sourceMap } : {}),
    declaredStyles,
    referencedStyles,
  };
}

/**
 * Recursively walk a list of XmlElements collecting every value
 * appearing in `class=` / `className=` attributes. Splits on whitespace
 * to match the runtime parser's behavior.
 */
function collectClassRefs(elements: XmlElement[]): Set<string> {
  const out = new Set<string>();
  function visit(el: XmlElement): void {
    const attrs = getAttributes(el);
    const raw = attrs.class ?? attrs.className;
    if (raw) {
      for (const name of raw.split(/\s+/)) if (name) out.add(name);
    }
    for (const child of getChildElements(el)) visit(child);
  }
  for (const el of elements) visit(el);
  return out;
}

/**
 * Recursively walk a list of XmlElements collecting every value
 * appearing on `<Use template="…"/>` markers.
 */
function collectTemplateRefs(elements: XmlElement[]): Set<string> {
  const out = new Set<string>();
  function visit(el: XmlElement): void {
    if (getTagName(el) === "Use") {
      const t = getAttributes(el).template?.trim();
      if (t) out.add(t);
    }
    for (const child of getChildElements(el)) visit(child);
  }
  for (const el of elements) visit(el);
  return out;
}

function parseSlideGlance(
  slideGlance: XmlElement,
  options: ParseBuilderDocumentOptions | undefined,
  sourceMap: BuilderSourceMap | null,
  diagnostics: Diagnostic[],
): ParsedBuilderDocument {
  const errors: string[] = [];
  const masters: SlideMasterOptions[] = [];
  const masterContents: Record<string, BuilderNode[]> = {};
  const nodes: BuilderNode[] = [];
  const rootChildrenAll = getChildElements(slideGlance);
  const { registry: templateRegistry, remaining: rootChildrenNoTpl } =
    collectTemplates(rootChildrenAll, errors);

  // Detect <Templates> blocks not at the SlideGlance root: collectTemplates
  // silently ignores them; emit a non-fatal diagnostic instead.
  emitNonRootTemplatesDiagnostics(rootChildrenNoTpl, diagnostics);

  // Apply template expansion to the remaining top-level elements (Master,
  // Slide, Styles, etc). Each retains its tag identity; only <Use> markers
  // anywhere in the descendants get rewritten to the template body.
  const rootChildren = expandTemplatesInNodes(
    rootChildrenNoTpl,
    templateRegistry,
    errors,
    0,
    { count: 0 },
    options?.maxTemplateNodes ?? DEFAULT_MAX_TEMPLATE_NODES,
    diagnostics,
  ).filter((node): node is XmlElement => !("#text" in node));
  const styles = collectStyles(rootChildren, errors);

  // Pluck the (optional) <Document> child for document-level settings.
  // Multiple <Document> children → first wins, others diagnostic'd.
  const documentChildren = rootChildren.filter(
    (child) => getTagName(child) === "Document",
  );
  if (documentChildren.length > 1) {
    const extra = documentChildren[1];
    const msg =
      "<SlideGlance>: Only one <Document> child is allowed; extras ignored";
    errors.push(extra ? formatErrorAt(extra, msg) : msg);
  }
  const documentEl = documentChildren[0];
  const documentAttrs = documentEl ? getAttributes(documentEl) : {};
  const slideSize = parseDocumentSize(documentEl, documentAttrs, errors);
  const defaultMaster = documentAttrs.defaultMaster?.trim() || undefined;
  const defaultTextStyle = parseDocumentDefaultTextStyle(
    documentEl,
    documentAttrs,
    errors,
  );

  for (const child of rootChildren) {
    const tag = getTagName(child);

    if (tag === "Styles" || tag === "Document") continue;

    if (tag === "Master") {
      const parsedMaster = parseMasterElement(child, errors, styles);
      if (parsedMaster) {
        masters.push(parsedMaster.master);
        if (parsedMaster.contentNodes.length > 0 && parsedMaster.master.title) {
          masterContents[parsedMaster.master.title] = parsedMaster.contentNodes;
        }
      }
      continue;
    }

    if (tag === "Slide") {
      const slideNode = parseSlideElement(child, errors, styles);
      if (slideNode) nodes.push(slideNode);
      continue;
    }

    // Implicit slide (SlideGlance child that is not <Slide>): preserve
    // slide-level `master`/`notes` attrs which were removed from BASE_RULES
    // in T10. Strip them from the raw XML attribute map so the downstream
    // unknown-attribute check does not flag them, then re-attach to the
    // converted node afterwards.
    const rawAttrMap = child[":@"];
    let slideMaster: string | undefined;
    let slideNotes: string | undefined;
    if (rawAttrMap) {
      if (typeof rawAttrMap["@_master"] === "string") {
        slideMaster = rawAttrMap["@_master"].trim();
        delete rawAttrMap["@_master"];
      }
      if (typeof rawAttrMap["@_notes"] === "string") {
        slideNotes = rawAttrMap["@_notes"].trim();
        delete rawAttrMap["@_notes"];
      }
    }
    const converted = convertElement(child, errors, styles);
    if (converted) {
      if (slideMaster !== undefined) converted.master = slideMaster;
      if (slideNotes !== undefined) converted.notes = slideNotes;
      nodes.push(converted as BuilderNode);
    }
  }

  if (errors.length > 0) {
    throw new ParseXmlError(errors);
  }

  // Slide-scope id / connector validation. Runs after the slide trees
  // are assembled so every author-id can be discovered, and emits
  // non-fatal diagnostics for duplicates / dangling / self-referencing
  // Connectors. Invalid Connectors are stripped from the trees so the
  // renderer never has to defend against missing endpoints.
  validateConnectorsInSlides(nodes, diagnostics);

  const declaredStyles = new Set(Object.keys(styles));
  const referencedStyles = collectClassRefs(rootChildren);
  const declaredTemplates = new Set(templateRegistry.keys());
  // `referencedTemplates` is scanned on rootChildrenAll (pre-expansion)
  // because `expandTemplatesInNodes` replaces `<Use>` markers with the
  // template body — by the time we hold `rootChildren` (post-expansion)
  // every <Use> is gone.
  const referencedTemplates = collectTemplateRefs(rootChildrenAll);
  return {
    nodes,
    ...(sourceMap ? { sourceMap } : {}),
    ...(slideSize ? { slideSize } : {}),
    ...(masters.length > 0 ? { masters } : {}),
    ...(Object.keys(masterContents).length > 0 ? { masterContents } : {}),
    ...(defaultMaster ? { defaultMaster } : {}),
    ...(defaultTextStyle ? { defaultTextStyle } : {}),
    declaredStyles,
    referencedStyles,
    declaredTemplates,
    referencedTemplates,
  };
}
