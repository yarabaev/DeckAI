---
title: "@slideglance/builder — Lint"
lang: en
kind: guides
package: builder
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/builder/src/
---

# Lint

`@slideglance/builder` ships a post-layout linter that walks the rendered
positioned-node tree and emits structured Diagnostics for the layout
failure modes that recur in real decks: overflow, baseline mismatch,
unused tokens, accessibility gaps, and performance traps. The runner is
opt-in via `BuildPptxOptions.lint`.

## Enabling the linter

```ts
import { buildPptx } from "@slideglance/builder";

const { pptx, diagnostics, lintReport } = await buildPptx(xml, slideSize, {
  lint: {
    enabled: true,
    ruleset: "recommended", // recommended | strict | errors-only
    output: ["stdout", "json"], // optional; consumers decide
    outputPath: "lint.json", // when output includes "json"
    overrides: { TINY_FONT: "off" },
  },
});
```

`diagnostics` is the unified list (parse + render + lint). `lintReport`
is a stable JSON structure consumers can feed to tooling and LLMs:

```json
{
  "version": 1,
  "generatedAt": "2026-05-11T00:00:00Z",
  "slideCount": 4,
  "summary": { "error": 0, "warn": 2, "info": 5 },
  "diagnostics": [
    {
      "code": "BASELINE_MIX_IN_ROW",
      "severity": "warn",
      "message": "Slide 5: Row mixes fontSize 13 / 9 but does not apply the…",
      "nodeId": "editorial/04-closing.xml:21",
      "nodeType": "hstack",
      "sourcePos": { "file": "editorial/04-closing.xml", "line": 21 },
      "context": {
        "fontSizes": [13, 9],
        "anchors": ["top", "top"],
        "lineHeights": [1.5, 1.0]
      },
      "suggestedFix": {
        "kind": "attribute-set",
        "target": "all-children",
        "set": { "textVAlign": "middle", "lineHeight": 1.0 }
      },
      "docsAnchor": "mixed-size-text-rows"
    }
  ]
}
```

`suggestedFix` is machine-applicable: an autofix runner or LLM can use
the `kind`, `target`, and `set` fields to mutate the source.

## Rule catalog

### A — Overflow / dimension (`error` / `warn`)

| Code               | Severity | Description                                                                                                                |
| ------------------ | -------- | -------------------------------------------------------------------------------------------------------------------------- |
| `OUT_OF_PAGE`      | error    | Node spills past the slide canvas.                                                                                         |
| `OUT_OF_PARENT`    | error    | Node overflows its non-`<Layer>` parent.                                                                                   |
| `NEGATIVE_DIM`     | error    | Resolved dimensions are negative (padding > size).                                                                         |
| `ZERO_DIM`         | warn     | Node renders at `0×N` or `N×0` (invisible).                                                                                |
| `TEXT_OVERFLOW_V`  | warn     | Re-measured wrap height (opentype) exceeds the box height. Disagreement between layout-time and renderer-time measurement. |
| `TEXT_OVERFLOW_H`  | warn     | Single-line box, but the natural opentype-measured width exceeds the box width — text will spill past the right edge.      |
| `TEXT_WRAP_TO_1CH` | error    | Text wraps to ≤ 1 character per line (sibling claimed slack).                                                              |
| `LINE_OVER_PARENT` | warn     | `<Line>` endpoint sits outside its non-`<Layer>` parent.                                                                   |
| `IMAGE_MISSING`    | error    | `<Image src>` did not resolve to bytes.                                                                                    |

### B — Visual coherence (`warn` / `info`)

| Code                          | Severity | Description                                                                                                   |
| ----------------------------- | -------- | ------------------------------------------------------------------------------------------------------------- |
| `BASELINE_MIX_IN_ROW`         | warn     | Row mixes fontSizes but does not apply the `textVAlign="middle"` + `lineHeight="1.0"` idiom on every sibling. |
| `INFLATED_LINE_HEIGHT_IN_ROW` | warn     | One row sibling has `lineHeight ≥ 1.3`; visually misaligned with tight neighbors.                             |
| `ANCHOR_INCONSISTENT`         | warn     | Row siblings disagree on `textVAlign`.                                                                        |
| `OVERLAP_LAYER`               | info     | Two non-decorative `<Layer>` children overlap by > 50% of the smaller bbox.                                   |
| `LOW_CONTRAST`                | info     | Text vs background contrast ratio < WCAG AA 4.5.                                                              |

### C — Design system (`info`)

| Code                | Severity | Description                                                                            |
| ------------------- | -------- | -------------------------------------------------------------------------------------- |
| `UNUSED_STYLE`      | info     | Declared `<Style>` is not referenced by any `class=`.                                  |
| `UNUSED_TEMPLATE`   | info     | Declared `<Template>` is not invoked by any `<Use>`.                                   |
| `HARDCODED_COLOR`   | info     | Same hex literal appears in 4+ places — extract to a `<Style>`.                        |
| `INCONSISTENT_FONT` | info     | Deck uses 3+ font families (editorial baseline pairs two).                             |
| `MASTER_COLLISION`  | warn     | (Placeholder — requires master-geometry exposure on the build context. No-op for now.) |

### D — Accessibility (`warn` / `info`)

| Code                      | Severity | Description                                                                      |
| ------------------------- | -------- | -------------------------------------------------------------------------------- |
| `IMG_NO_ALT`              | warn     | `<Image>` lacks `altText` and is not marked `decorative="true"`.                 |
| `READING_ORDER_AMBIGUOUS` | info     | `<Layer>` source order diverges from visual top-to-bottom order at 2+ positions. |
| `ICON_NO_LABEL`           | info     | `<Icon>` appears alone (no adjacent text), not marked `decorative`.              |
| `TINY_FONT`               | info     | `fontSize < 8pt` — below the projected-slide readability floor.                  |

### E — Performance (`info`)

| Code                  | Severity | Description                                     |
| --------------------- | -------- | ----------------------------------------------- |
| `LARGE_IMAGE_INLINED` | info     | Image > 1 MB but display area < 200k px².       |
| `EXCESS_NODES`        | info     | Slide has > 200 nodes — PowerPoint UX degrades. |
| `SLIDE_FONT_COUNT`    | info     | Slide uses > 5 font families.                   |

## Rulesets

| Ruleset                 | Includes                  |
| ----------------------- | ------------------------- |
| `errors-only`           | `error` only              |
| `recommended` (default) | `error` + `warn`          |
| `strict`                | `error` + `warn` + `info` |

Combine with `overrides` to demote / promote / disable individual rules:

```ts
lint: {
  enabled: true,
  ruleset: "recommended",
  overrides: {
    TINY_FONT: "warn",         // promote info → warn
    OVERLAP_LAYER: "off",      // suppress entirely
  }
}
```

## LLM autofix pattern

Each diagnostic carries enough machine-readable context for an LLM to
produce a patch:

1. `nodeId` + `sourcePos` localizes the node in source XML.
2. `context` provides the rule's reasoning input.
3. `suggestedFix.kind` selects the mutation type (`attribute-set`,
   `wrap-with`, `text-content-change`).
4. `suggestedFix.target` constrains the scope (`self`, `all-children`,
   `siblings`).
5. `suggestedFix.set` enumerates the attribute changes to apply.

The recommended workflow:

```
build w/ lint → lintReport.json
         ↓
LLM (input: report.diagnostics) → suggested patches
         ↓
human review / autofix runner → modified XML
         ↓
re-build to verify
```
