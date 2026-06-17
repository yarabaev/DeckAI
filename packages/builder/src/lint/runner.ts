import type { Diagnostic } from "../diagnostics.ts";
import type { LintContext, LintOptions, LintRule } from "./types.ts";
import { ALL_RULES } from "./rules/index.ts";

/**
 * Filter & severity-override the rule set per LintOptions, then run.
 */
export function runLintRules(
  ctx: LintContext,
  options: LintOptions,
): Diagnostic[] {
  if (options.enabled === false) return [];
  const ruleset = options.ruleset ?? "recommended";
  const overrides = options.overrides ?? {};

  const out: Diagnostic[] = [];
  for (const rule of ALL_RULES) {
    if (rule.phase !== ctx.phase) continue;

    const override = overrides[rule.code];
    if (override === "off") continue;
    const effectiveSeverity = override ?? rule.severity;

    // ruleset filters by SEVERITY of the rule itself.
    if (ruleset === "errors-only" && effectiveSeverity !== "error") continue;
    if (
      ruleset === "recommended" &&
      effectiveSeverity !== "error" &&
      effectiveSeverity !== "warn"
    )
      continue;

    let diags: Diagnostic[];
    try {
      diags = rule.check(ctx);
    } catch (err) {
      // A lint rule must never break the build. Swallow + surface as info.
      diags = [
        {
          code: rule.code,
          severity: "info",
          message: `Lint rule "${rule.code}" threw: ${
            err instanceof Error ? err.message : String(err)
          }`,
        },
      ];
    }

    for (const d of diags) {
      // Tag the severity that the caller's overrides resolved to.
      out.push({ ...d, severity: d.severity ?? effectiveSeverity });
    }
  }
  return out;
}

/** Used by tests to enumerate rules. */
export function getAllRules(): readonly LintRule[] {
  return ALL_RULES;
}
