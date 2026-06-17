# Lint reference

The builder ships a lint pass that catches overflow, baseline
misalignment, accessibility gaps, and design-system regressions
before the deck reaches the recipient. Treat warnings as bugs.

## Enable

```ts
import { buildPptx } from "@slideglance/builder";

const { pptx, diagnostics } = await buildPptx(xml, { w: 1280, h: 720 }, {
  lint: { enabled: true, ruleset: "recommended" },
});

if (diagnostics.length) {
  for (const d of diagnostics) {
    console.warn(`${d.severity ?? "info"} ${d.code}: ${d.message} @ ${d.path ?? "?"}`);
  }
}
```

Default ruleset: `recommended` (`error` + `warn`). Use `strict` to
also surface `info`-level findings.

## Rule catalog

### A — Overflow / dimension

| Code | Severity | Description |
| --- | --- | --- |
| `OUT_OF_PAGE` | error | Node spills past the slide canvas. |
| `OUT_OF_PARENT` | error | Node overflows its non-`<Layer>` parent. |
| `NEGATIVE_DIM` | error | Resolved dimensions are negative (padding > size). |
| `ZERO_DIM` | warn | Node renders at `0×N` or `N×0` (invisible). |
| `TEXT_OVERFLOW_V` | warn | Re-measured wrap height (opentype) exceeds the box. |
| `TEXT_OVERFLOW_H` | warn | Single-line box's natural opentype-measured width exceeds box width. |
| `TEXT_WRAP_TO_1CH` | error | Text wraps to ≤ 1 character per line. |
| `LINE_OVER_PARENT` | warn | `<Line>` endpoint sits outside its non-`<Layer>` parent. |
| `IMAGE_MISSING` | error | `<Image src>` did not resolve to bytes. |

### B — Visual coherence

| Code | Severity | Description |
| --- | --- | --- |
| `BASELINE_MIX_IN_ROW` | warn | Row mixes fontSizes but doesn't apply the `textVAlign="middle"` + `lineHeight="1.0"` idiom on every sibling. |
| `INFLATED_LINE_HEIGHT_IN_ROW` | warn | One row sibling has `lineHeight ≥ 1.3`; visually misaligned with tight neighbors. |
| `ANCHOR_INCONSISTENT` | warn | Row siblings disagree on `textVAlign`. |
| `OVERLAP_LAYER` | info | Two non-decorative `<Layer>` children overlap by > 50% of the smaller bbox. |
| `LOW_CONTRAST` | info | Text vs background contrast ratio < WCAG AA 4.5. |

### C — Design system

| Code | Severity | Description |
| --- | --- | --- |
| `UNUSED_STYLE` | info | Declared `<Style>` is not referenced by any `class=`. |
| `UNUSED_TEMPLATE` | info | Declared `<Template>` is not invoked by any `<Use>`. |
| `HARDCODED_COLOR` | info | Same hex literal appears in 4+ places — extract to a `<Style>`. |
| `INCONSISTENT_FONT` | info | Deck uses 3+ font families (editorial baseline pairs two). |

### D — Accessibility

| Code | Severity | Description |
| --- | --- | --- |
| `IMG_NO_ALT` | warn | `<Image>` lacks `altText` and is not marked `isDecorative="true"`. |
| `READING_ORDER_AMBIGUOUS` | info | `<Layer>` source order diverges from visual top-to-bottom order at 2+ positions. |
| `ICON_NO_LABEL` | info | `<Icon>` appears alone (no adjacent text), not marked `isDecorative`. |
| `TINY_FONT` | info | `fontSize < 8pt` — below projector readability floor. |

### E — Performance

| Code | Severity | Description |
| --- | --- | --- |
| `LARGE_IMAGE_INLINED` | info | Image > 1 MB but display area < 200k px². |
| `EXCESS_NODES` | info | Slide has > 200 nodes — PowerPoint UX degrades. |
| `SLIDE_FONT_COUNT` | info | Slide uses > 5 font families. |

## Rulesets

| Ruleset | Includes |
| --- | --- |
| `errors-only` | `error` only |
| `recommended` (default) | `error` + `warn` |
| `strict` | `error` + `warn` + `info` |

Combine with overrides:

```ts
lint: {
  enabled: true,
  ruleset: "recommended",
  overrides: {
    TINY_FONT: "warn",        // promote info → warn
    OVERLAP_LAYER: "off",     // suppress entirely
  }
}
```

## Autofix patterns

When the linter reports a finding, fix the source rather than
suppressing.

### `OUT_OF_PAGE` / `OUT_OF_PARENT`

Reduce content, reduce `fontSize`, add `flexWrap="wrap"` on the
container, or split the slide.

### `BASELINE_MIX_IN_ROW`

Add `textVAlign="middle" lineHeight="1.0"` to **every** sibling in
the row.

### `HARDCODED_COLOR`

Promote the literal to `<Styles>`:

```xml
<!-- Before -->
<Text color="0F172A">…</Text>
<VStack border.color="0F172A">…</VStack>

<!-- After -->
<Styles>
  <Style name="ink-line" color="0F172A" border.color="0F172A" />
</Styles>
```

### `IMG_NO_ALT`

Either supply alt text:

```xml
<Image src="./chart.png" altText="Quarterly revenue chart showing 22% YoY growth." />
```

Or mark decorative:

```xml
<Image src="./bg-pattern.svg" isDecorative="true" />
```

### `INCONSISTENT_FONT`

Pick two families. The lint flags 3+ specifically because editorial
decks pair display + body. If a deliberate three-family decision is
needed (display + body + mono), suppress with
`overrides: { INCONSISTENT_FONT: "off" }`.

## LLM authoring loop

1. Author / edit `.sgx`.
2. Build with `lint.enabled = true`.
3. Read diagnostics.
4. Fix the source files (not the lint config).
5. Re-build.

Don't ship diagnostics. A clean build is a deck that opens correctly
on the recipient's machine.
