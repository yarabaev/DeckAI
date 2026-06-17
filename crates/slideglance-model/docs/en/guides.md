---
title: slideglance-model — Guides
lang: en
kind: guides
crate: slideglance-model
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-model/src/slide.rs
  - crates/slideglance-model/src/shape.rs
  - crates/slideglance-model/src/text.rs
---

# slideglance-model — Guides

## Walk every text run in a deck

### Goal

Extract every `TextRun` in a `Presentation` for content audit (e.g.
spell-check, link audit, typeface usage).

### Code

```rust,no_run
use slideglance_model::{Presentation, ShapeElement, SlideElement, TextRun};

fn for_each_run<F: FnMut(&TextRun)>(p: &Presentation, mut visit: F) {
    fn walk_shape<F: FnMut(&TextRun)>(shape: &ShapeElement, visit: &mut F) {
        if let Some(body) = shape.text_body.as_ref() {
            for paragraph in &body.paragraphs {
                for run in &paragraph.runs {
                    visit(run);
                }
            }
        }
    }
    fn walk_elements<F: FnMut(&TextRun)>(elements: &[SlideElement], visit: &mut F) {
        for el in elements {
            match el {
                SlideElement::Shape(s) => walk_shape(s, visit),
                SlideElement::Group(g) => walk_elements(&g.children, visit),
                _ => {}
            }
        }
    }
    for slide in &p.slides {
        walk_elements(&slide.elements, &mut visit);
    }
}
```

### What's happening

`SlideElement` is the tagged enum carrying every element variant.
Groups recurse; tables and charts carry their own text under
different keys (see `TableCell.text_body`, `ChartDataLabels`) so
extend the helper as you need.

## Resolve a shape's effective fill against the theme

### Goal

A shape may carry `<a:solidFill><a:schemeClr val="accent1"/></a:solidFill>`.
Convert to a concrete `Rgb`.

### Code

```rust,no_run
use slideglance_model::{ColorResolver, Fill, Rgb, ShapeElement, SolidFill};

fn solid_rgb(shape: &ShapeElement, resolver: &ColorResolver) -> Option<Rgb> {
    match shape.fill.as_ref()? {
        Fill::Solid(SolidFill { color }) => Some(resolver.resolve(color).rgb),
        _ => None,
    }
}
```

### What's happening

`ColorResolver` is built once from the deck's `Theme` + the slide's
`ColorMap` and reused across the deck — instantiation is cheap but
not free. For non-solid fills (gradient, pattern, image) walk the
matching enum arm; the renderer in `slideglance-renderer` does the
same thing through `fill::render_fill_attrs`.
