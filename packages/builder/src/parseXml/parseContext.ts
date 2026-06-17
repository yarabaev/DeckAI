/**
 * Module-scoped parse context.
 *
 * The whole parse pipeline is synchronous, so a simple mutable variable
 * suffices — it avoids threading a context object through every converter.
 * Set at the start of parseBuilderDocument and cleared in its finally block.
 */

import type { Diagnostic } from "../diagnostics.ts";

/**
 * Source location of a BuilderNode in its originating XML file.
 * `file` is the absolute file path when known (e.g. for imports) or undefined
 * when the root document was passed as a plain string without `sourcePath`.
 * `line` is 1-based.
 */
export interface BuilderSourcePos {
  file: string | undefined;
  line: number;
}

export type BuilderSourceMap = Map<number, BuilderSourcePos>;

let CURRENT_SOURCE_MAP: BuilderSourceMap | null = null;
let NEXT_NODE_ID = 0;
let CURRENT_DIAGNOSTICS: Diagnostic[] | null = null;

export function beginParseContext(
  sourceMap: BuilderSourceMap | null,
  diagnostics: Diagnostic[] | null,
): void {
  CURRENT_SOURCE_MAP = sourceMap;
  CURRENT_DIAGNOSTICS = diagnostics;
  NEXT_NODE_ID = 0;
}

export function endParseContext(): void {
  CURRENT_SOURCE_MAP = null;
  CURRENT_DIAGNOSTICS = null;
}

export function getCurrentSourceMap(): BuilderSourceMap | null {
  return CURRENT_SOURCE_MAP;
}

export function getCurrentDiagnostics(): Diagnostic[] | null {
  return CURRENT_DIAGNOSTICS;
}

export function allocateNextPomId(): number {
  return ++NEXT_NODE_ID;
}
