import type { Diagnostic } from "../diagnostics.ts";
import type { PositionedNode, BuilderNode } from "../types.ts";
import type { TextMeasurer } from "../calcYogaLayout/fontLoader.ts";

export type LintPhase = "parse" | "post-layout";

/**
 * Bag of state the runner gives every rule. Rules are pure: read context,
 * return Diagnostics. Mutation of `tree`/`raw` is not permitted.
 */
export interface LintContext {
  /** Original BuilderNode tree (pre-layout), if available. */
  rawTree?: BuilderNode;
  /** Slide root after toPositioned. Present for post-layout rules. */
  tree: PositionedNode;
  /** Slide index (1-based) within the deck, for diagnostics. */
  slideIndex: number;
  /** Slide canvas size, in builder px. */
  slideSize: { w: number; h: number };
  /** Set of Style class names declared in the deck (drives unused-style). */
  declaredStyles?: Set<string>;
  /** Set of Style class names actually referenced anywhere. */
  referencedStyles?: Set<string>;
  /** Set of Template names declared. */
  declaredTemplates?: Set<string>;
  /** Set of Template names referenced via <Use>. */
  referencedTemplates?: Set<string>;
  /** Phase. Rules opt into the phase they need by setting `LintRule.phase`. */
  phase: LintPhase;
  /**
   * Per-build text measurer. Shared with the layout pass so rules that
   * re-verify overflow (TEXT_OVERFLOW_V / TEXT_OVERFLOW_H) measure with
   * the same glyph metrics that produced the positioned bboxes. When
   * omitted, rules fall back to the bundled-fonts singleton.
   */
  measurer?: TextMeasurer;
  /**
   * Raw XML source strings for parse-phase rules (e.g.
   * `RAW_LT_GT_IN_ATTR`). The root document plus every file resolved
   * through `<Import>` should appear here so source-level violations
   * surface with accurate file + line attribution. Empty / absent for
   * post-layout contexts.
   */
  rawXmlSources?: readonly { path?: string; content: string }[];
}

export interface LintRule {
  /** Stable diagnostic code emitted by this rule. */
  code: Diagnostic["code"];
  /** Default severity (caller may override). */
  severity: "error" | "warn" | "info";
  /** Phase the rule runs in. */
  phase: LintPhase;
  /** Returns 0+ diagnostics. Pure function; no side-effects. */
  check: (ctx: LintContext) => Diagnostic[];
}

export type LintOutput = "stdout" | "json" | "silent";

export interface LintOptions {
  enabled?: boolean;
  /**
   * Selects which rule severities to include.
   *   - "recommended": error + warn  (default)
   *   - "strict":      error + warn + info
   *   - "errors-only": error only
   */
  ruleset?: "recommended" | "strict" | "errors-only";
  /** Where to send the lint report (or both). */
  output?: LintOutput | LintOutput[];
  /** Path to write JSON report (used when output includes "json"). */
  outputPath?: string;
  /** Override individual rule severities or disable specific codes. */
  overrides?: Partial<
    Record<Diagnostic["code"], "error" | "warn" | "info" | "off">
  >;
}

export interface LintReport {
  /** Schema version of the report — bump on breaking changes. */
  version: 1;
  generatedAt: string;
  slideCount: number;
  summary: {
    error: number;
    warn: number;
    info: number;
  };
  diagnostics: Diagnostic[];
}
