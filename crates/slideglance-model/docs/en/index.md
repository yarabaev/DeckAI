---
title: slideglance-model
lang: en
kind: index
crate: slideglance-model
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-model/src/lib.rs
---

# slideglance-model

> Part of the SlideGlance workspace.
> See also: [all crates](../../../../docs/en/crates.md) ·
> [architecture](../../../../docs/en/architecture.md).

## What it is

The OOXML PPTX semantic model — pure data types representing a
parsed `.pptx`. Shapes, fills, text, tables, charts, themes, slides.
No parsing logic; no rendering logic. The parser produces these
values, the renderer consumes them.

## Where it sits

```
slideglance-utils ← slideglance-color ← slideglance-model
                                              ↑
                                       slideglance-parser
                                              ↓
                                       slideglance-renderer
```

## When to use this

- Reading or producing the typed result of a PPTX parse.
- Writing tooling that walks the slide tree (typeface usage, asset
  extraction, content audits).
- Integrating with a custom renderer that targets a non-SVG output.

## Quick start

```rust,no_run
use slideglance_model::{Presentation, ShapeElement, SlideElement};

fn count_shapes(p: &Presentation) -> usize {
    p.slides.iter()
        .flat_map(|slide| slide.elements.iter())
        .filter(|el| matches!(el, SlideElement::Shape(_) | SlideElement::Connector(_)))
        .count()
}
```

## Where to go next

- [Reference](./reference.md) — every module and its top-level types
- [Guides](./guides.md) — walking the slide tree
- Source: [`crates/slideglance-model/src/`](../../src/)
