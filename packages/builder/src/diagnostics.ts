export type DiagnosticCode =
  // --- pre-existing parse/render diagnostics ---
  | "IMAGE_MEASURE_FAILED"
  | "IMAGE_NOT_PREFETCHED"
  | "AUTOFIT_OVERFLOW"
  | "SCALE_BELOW_THRESHOLD"
  | "MASTER_PPTX_PARSE_FAILED"
  | "INVALID_HREF_SCHEME"
  | "INVALID_IMAGE_SRC"
  | "TEMPLATE_EXPANSION_LIMIT"
  | "MASTER_PPTX_SIZE_LIMIT"
  | "TEMPLATES_NOT_AT_ROOT"
  | "INVALID_NUMBER_TYPE"
  | "DUPLICATE_NODE_ID"
  | "UNKNOWN_CONNECTOR_ENDPOINT"
  | "INVALID_CONNECTOR_SELF_REF"
  | "CONNECTOR_UNKNOWN_SHAPE_IDX"
  // --- lint codes (parse) ---
  | "RAW_LT_GT_IN_ATTR"
  // --- lint codes (post-layout / post-parse) ---
  | "OUT_OF_PAGE"
  | "OUT_OF_PARENT"
  | "NEGATIVE_DIM"
  | "ZERO_DIM"
  | "TEXT_OVERFLOW_V"
  | "TEXT_OVERFLOW_H"
  | "TEXT_WRAP_TO_1CH"
  | "LINE_OVER_PARENT"
  | "IMAGE_MISSING"
  | "BASELINE_MIX_IN_ROW"
  | "INFLATED_LINE_HEIGHT_IN_ROW"
  | "ANCHOR_INCONSISTENT"
  | "OVERLAP_LAYER"
  | "LOW_CONTRAST"
  | "UNUSED_STYLE"
  | "UNUSED_TEMPLATE"
  | "HARDCODED_COLOR"
  | "INCONSISTENT_FONT"
  | "MASTER_COLLISION"
  | "IMG_NO_ALT"
  | "READING_ORDER_AMBIGUOUS"
  | "ICON_NO_LABEL"
  | "TINY_FONT"
  | "LARGE_IMAGE_INLINED"
  | "EXCESS_NODES"
  | "SLIDE_FONT_COUNT";

export type DiagnosticSeverity = "error" | "warn" | "info";

export interface DiagnosticSourcePos {
  line?: number;
  file?: string;
}

/**
 * LLM-readable suggested action. Three shapes covered:
 *
 *   attribute-set       — set N attributes on one or more matching nodes
 *   wrap-with           — wrap the target with a parent (e.g. add Layer)
 *   text-content-change — replace text content of a target node
 */
export type DiagnosticFix =
  | {
      kind: "attribute-set";
      /**
       * Targets within the offending subtree. Well-known values are
       * `"self"`, `"all-children"`, and `"siblings"`; the field accepts
       * any string so future rules can address narrower scopes via
       * custom selectors.
       */
      target: string;
      /** Attribute → value pairs to set on the targets. */
      set: Record<string, string | number | boolean>;
      /** Free-form notes the autofix runner / LLM should respect. */
      notes?: string;
    }
  | {
      kind: "wrap-with";
      tag: string;
      attrs?: Record<string, string | number>;
      notes?: string;
    }
  | {
      kind: "text-content-change";
      to: string;
      notes?: string;
    };

export interface Diagnostic {
  code: DiagnosticCode;
  message: string;
  /**
   * `error` blocks `prepublishOnly`-style gates; `warn` is the default
   * lint level and is printed but does not fail; `info` is informational
   * (style / design hints).
   */
  severity?: DiagnosticSeverity;
  sourcePos?: DiagnosticSourcePos;
  /** Stable selector / nodeId for the offending node, for LLM input. */
  nodeId?: string;
  nodeType?: string;
  /** Machine-readable values that drove the rule (for LLM context). */
  context?: Record<string, unknown>;
  /** Hint for autofix: how to mutate the source to clear the diagnostic. */
  suggestedFix?: DiagnosticFix;
  /** Anchor in the user-facing docs (slug for xml-reference.md). */
  docsAnchor?: string;
}

export class DiagnosticCollector {
  readonly items: Diagnostic[] = [];

  add(
    code: DiagnosticCode,
    message: string,
    sourcePos?: DiagnosticSourcePos,
  ): void {
    this.items.push(
      sourcePos ? { code, message, sourcePos } : { code, message },
    );
  }

  addAll(diagnostics: readonly Diagnostic[]): void {
    for (const diagnostic of diagnostics) {
      this.items.push(diagnostic);
    }
  }

  /** Push a fully-detailed lint Diagnostic (severity, context, fix). */
  addLint(diag: Diagnostic): void {
    this.items.push(diag);
  }
}

export class DiagnosticsError extends Error {
  constructor(public readonly diagnostics: Diagnostic[]) {
    const summary = diagnostics
      .map((d) => `[${d.code}] ${d.message}`)
      .join("\n");
    super(`Build completed with diagnostics:\n${summary}`);
    this.name = "DiagnosticsError";
  }
}
