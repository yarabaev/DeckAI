import type { Diagnostic } from "../diagnostics.ts";
import type { PositionedNode } from "../types.ts";
import type { LintContext, LintOptions, LintReport } from "./types.ts";
import type { TextMeasurer } from "../calcYogaLayout/fontLoader.ts";
import { runLintRules } from "./runner.ts";
import { formatStdout } from "./output/stdout.ts";
import { buildJsonReport } from "./output/json.ts";

export type {
  LintContext,
  LintOptions,
  LintReport,
  LintRule,
  LintOutput,
  LintPhase,
} from "./types.ts";
export { runLintRules };
export { formatStdout };
export { buildJsonReport };

/**
 * Inputs the builder collects across the parse pass that the lint
 * runner needs at post-layout time.
 */
export interface DeckLintInputs {
  declaredStyles?: Set<string>;
  referencedStyles?: Set<string>;
  declaredTemplates?: Set<string>;
  referencedTemplates?: Set<string>;
  /**
   * Per-build text measurer threaded into every rule's LintContext.
   * When omitted, overflow-checking rules use the bundled-fonts
   * singleton (still opentype-accurate for Pretendard / Noto Sans JP
   * and their substitution path for unknown families).
   */
  measurer?: TextMeasurer;
  /**
   * Raw XML source files seen during parsing (root document + every
   * `<Import>` target). Drives parse-phase rules like
   * `RAW_LT_GT_IN_ATTR` that need the original characters before
   * fast-xml-parser quietly normalises them.
   */
  rawXmlSources?: readonly { path?: string; content: string }[];
}

/**
 * Run all enabled lint rules against the rendered slide trees. Returns
 * the merged diagnostics list plus an optional JSON report. The runner
 * never throws — failing rules degrade to info-severity diagnostics.
 */
export function lintDeck(
  slides: readonly {
    root: PositionedNode;
    slideSize: { w: number; h: number };
  }[],
  options: LintOptions,
  inputs: DeckLintInputs = {},
): { diagnostics: Diagnostic[]; report: LintReport } {
  if (options.enabled === false) {
    return {
      diagnostics: [],
      report: buildJsonReport([], slides.length),
    };
  }
  const all: Diagnostic[] = [];

  // Parse-phase rules run once per deck against the raw XML sources.
  // They produce file/line-anchored diagnostics so authors can fix the
  // offending source line directly. When no raw sources are supplied
  // (legacy callers), the parse-phase pass is a no-op.
  if (inputs.rawXmlSources && inputs.rawXmlSources.length > 0) {
    // Parse-phase rules never read tree / slideSize. Borrow the first
    // slide's values as inert placeholders so the shared LintContext
    // type stays honest; for the 0-slide edge case (rare — master-only
    // decks), fall back to a minimal stub so source-level rules still
    // run.
    const anchor = slides[0];
    const parseCtx: LintContext = {
      tree: (anchor?.root ?? {
        type: "slide",
        x: 0,
        y: 0,
        w: 0,
        h: 0,
      }) as PositionedNode,
      slideIndex: 0,
      slideSize: anchor?.slideSize ?? { w: 0, h: 0 },
      rawXmlSources: inputs.rawXmlSources,
      phase: "parse",
    };
    all.push(...runLintRules(parseCtx, options));
  }

  slides.forEach((slide, i) => {
    const ctx: LintContext = {
      tree: slide.root,
      slideIndex: i + 1,
      slideSize: slide.slideSize,
      declaredStyles: inputs.declaredStyles,
      referencedStyles: inputs.referencedStyles,
      declaredTemplates: inputs.declaredTemplates,
      referencedTemplates: inputs.referencedTemplates,
      measurer: inputs.measurer,
      phase: "post-layout",
    };
    all.push(...runLintRules(ctx, options));
  });
  return {
    diagnostics: all,
    report: buildJsonReport(all, slides.length),
  };
}
