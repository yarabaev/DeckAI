---
title: slideglance-renderer
lang: en
kind: index
crate: slideglance-renderer
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-renderer/src/lib.rs
  - crates/slideglance-renderer/src/slide/
  - crates/slideglance-renderer/src/shape.rs
---

# slideglance-renderer

> Part of the SlideGlance workspace.
> See also: [all crates](../../../../docs/en/crates.md) ·
> [architecture](../../../../docs/en/architecture.md).

## What it is

Deterministic model → SVG renderer. Consumes
`slideglance-model::Slide` values (post-parse, post-inheritance) and
emits one `<svg>` per slide. Supports both text-mode and path-mode
text rendering (see [`slideglance-font`](../../../slideglance-font/docs/en/index.md)).

## Where it sits

```
slideglance-model      slideglance-font
        ↓                   ↓
       slideglance-renderer
                ↓
          slideglance-png  →  PNG
                ↓
        slideglance / slideglance-wasm
```

## When to use this

- Producing SVG output from a parsed slide.
- Implementing a new shape / fill / effect / chart variant.
- Diagnosing a visual regression against the spec.

## Determinism

The renderer is deterministic: same model + same font resolver +
same options produces bit-identical SVG. It does this by:

- Sorting IDs through `id_gen::IdGen` instead of using random
  numbers.
- Iterating maps in `BTreeMap` order, never `HashMap`.
- Never reading the wall clock.

These properties are required for VRT and for native ↔ WASM
parity.

## Quick start

```rust,no_run
use slideglance_renderer::{render_slide_to_svg, RendererError, SlideViewBox};
# use slideglance_model::Slide;
# fn example(slide: &Slide) -> Result<(), RendererError> {

let svg = render_slide_to_svg(slide, /* context */ &Default::default())?;
println!("{svg}");
# Ok(()) }
```

## Where to go next

- [Reference](./reference.md)
- [Guides](./guides.md)
- Source: [`crates/slideglance-renderer/src/`](../../src/)
