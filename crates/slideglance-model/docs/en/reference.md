---
title: slideglance-model — Reference
lang: en
kind: reference
crate: slideglance-model
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-model/src/lib.rs
  - crates/slideglance-model/src/slide.rs
  - crates/slideglance-model/src/shape.rs
  - crates/slideglance-model/src/text.rs
  - crates/slideglance-model/src/fill.rs
  - crates/slideglance-model/src/line.rs
  - crates/slideglance-model/src/effect.rs
  - crates/slideglance-model/src/image.rs
  - crates/slideglance-model/src/table.rs
  - crates/slideglance-model/src/chart.rs
  - crates/slideglance-model/src/theme.rs
  - crates/slideglance-model/src/presentation.rs
---

# slideglance-model — Reference

## Crate layout

```
crates/slideglance-model/
├── src/
│   ├── lib.rs           # re-exports
│   ├── presentation.rs  # PresentationInfo, EmbeddedFont, SlideSize, …
│   ├── slide.rs         # Presentation, Slide, SlideLayout, SlideMaster, Background, RenderedSlide
│   ├── shape.rs         # ShapeElement, ConnectorElement, Geometry, Transform, …
│   ├── text.rs          # TextBody, Paragraph, TextRun, BodyProperties, …
│   ├── fill.rs          # Fill, GradientFill, ImageFill, PatternFill, …
│   ├── line.rs          # Outline, DashStyle, LineCap, LineJoin, Arrow*
│   ├── effect.rs        # EffectList, Glow, InnerShadow, OuterShadow, SoftEdge, BlipEffects, …
│   ├── image.rs         # ImageElement, SrcRect, StretchFillRect, TileInfo
│   ├── table.rs         # TableElement, TableData, TableCell, CellBorders, …
│   ├── chart.rs         # ChartElement, ChartType, ChartSeries, ChartAxis, …
│   └── theme.rs         # Theme, FontScheme, FormatScheme
└── Cargo.toml
```

## Top-level entry types

| Type | Module | Purpose |
|---|---|---|
| `Presentation` | `slide` | Root — every slide + the resolved theme |
| `Slide` | `slide` | One slide with its elements, background, header/footer |
| `SlideLayout` | `slide` | Layout-level placeholders + theme overrides |
| `SlideMaster` | `slide` | Master-level defaults |
| `Theme` | `theme` | Color scheme + font scheme + format scheme |
| `PresentationInfo` | `presentation` | Presentation-level metadata |

## Element discriminator

`SlideElement` is the tagged enum every slide / group iterates over:

```rust,no_run
use slideglance_model::SlideElement;

# fn for_each(elements: &[SlideElement]) {
for element in elements {
    match element {
        SlideElement::Shape(s) => {},
        SlideElement::Connector(c) => {},
        SlideElement::Group(g) => {},
        SlideElement::Image(i) => {},
        SlideElement::Table(t) => {},
        SlideElement::Chart(c) => {},
    }
}
# }
```

## Color types

Re-exported from `slideglance-color` so consumers do not need a
second dependency:

- `ColorMap`, `ColorRef`, `ColorResolver`, `ColorScheme`,
  `ResolvedColor`, `Rgb`.

## Idiomatic conversions from OOXML

| OOXML construct | Rust type |
|---|---|
| Tagged union `<x:foo type="…">…</x:foo>` | `enum` with `#[serde(tag = "type")]` |
| `string | null` attribute | `Option<String>` |
| EMU length | `slideglance_utils::Emu` |
| Hundredths-of-a-point | `slideglance_utils::HundredthPt` |
| Resolved color | `slideglance_color::ResolvedColor` |

For the full list of re-exported items see
[`src/lib.rs`](../../src/lib.rs).
