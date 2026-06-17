/**
 * Public entry points for the builder XML parser.
 *
 * The actual parsing is split across:
 * - `dispatcher.ts`: per-element conversion (attributes, children, post-process)
 * - `document.ts`: document-level parsing (SlideGlance/Master/Slide and bare-fragment roots)
 * - smaller helpers in `xml.ts`, `styles.ts`, `coerceAttrs.ts`, `validation.ts`,
 *   `textRuns.ts`, `childConverters.ts`, `parseContext.ts`.
 *
 * `parseXml.ts` itself only owns the public surface: types, the
 * parse-context lifecycle, and the thin entry functions.
 */

import type { Diagnostic } from "../diagnostics.ts";
import type { BuilderNode } from "../types.ts";

import {
  type ParseBuilderDocumentOptions,
  type ParsedBuilderDocument,
  parseBuilderDocumentInner,
} from "./document.ts";
import {
  beginParseContext,
  endParseContext,
  type BuilderSourceMap,
} from "./parseContext.ts";

export type { ImportResolver } from "./imports.ts";
export type { BuilderSourceMap, BuilderSourcePos } from "./parseContext.ts";
export type {
  ParseBuilderDocumentOptions,
  ParsedBuilderDocument,
} from "./document.ts";
export { ParseXmlError } from "./document.ts";

/**
 * Result of {@link parseBuilderDocument}: the parsed document plus any non-fatal
 * deprecation/validation diagnostics emitted during parsing. Fatal errors are
 * still raised via {@link ParseXmlError}; this channel carries warnings only.
 */
export interface ParseResult {
  document: ParsedBuilderDocument;
  diagnostics: Diagnostic[];
}

/**
 * Parse a builder XML document and return the structured BuilderNode tree along with
 * any non-fatal diagnostics. Throws {@link ParseXmlError} on validation
 * failure.
 */
export function parseBuilderDocument(
  xmlString: string,
  options?: ParseBuilderDocumentOptions,
): ParseResult {
  if (!xmlString.trim()) {
    const empty = options?.trackSourcePos
      ? { nodes: [], sourceMap: new Map() }
      : { nodes: [] };
    return { document: empty, diagnostics: [] };
  }

  const diagnostics: Diagnostic[] = [];
  const sourceMap: BuilderSourceMap | null = options?.trackSourcePos
    ? new Map()
    : null;

  beginParseContext(sourceMap, diagnostics);
  try {
    const document = parseBuilderDocumentInner(
      xmlString,
      options,
      sourceMap,
      diagnostics,
    );
    return { document, diagnostics };
  } finally {
    endParseContext();
  }
}

/**
 * Convenience wrapper around {@link parseBuilderDocument} that returns just the
 * parsed builder nodes.
 *
 * @example
 * ```typescript
 * import { parseXml, buildPptx } from "@slideglance/builder";
 *
 * const xml = `
 *   <VStack gap="16" padding="32">
 *     <Text fontSize="32" bold="true">Sales Report</Text>
 *   </VStack>
 * `;
 *
 * const nodes = parseXml(xml);
 * const pptx = await buildPptx(nodes, { w: 1280, h: 720 });
 * ```
 */
export function parseXml(xmlString: string): BuilderNode[] {
  return parseBuilderDocument(xmlString).document.nodes;
}
