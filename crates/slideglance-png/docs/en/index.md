---
title: slideglance-png
lang: en
kind: index
crate: slideglance-png
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-png/src/lib.rs
---

# slideglance-png

> Part of the SlideGlance workspace.
> See also: [all crates](../../../../docs/en/crates.md) ·
> [fonts](../../../../docs/en/fonts.md).

## What it is

SVG → PNG rasterization. Wraps `resvg`, `usvg`, and `tiny_skia` with
a deterministic configuration: no system fonts, geometric-precision
rendering, and explicit DPI control. Produces the same byte stream
across native and WASM targets when the same font buffers are
supplied.

## Where it sits

```
slideglance-renderer  (produces SVG)
        ↓
slideglance-png  (rasterizes to PNG)
        ↓
slideglance / slideglance-wasm  (orchestrators)
```

## When to use this

- After SVG has been produced and a pixel artefact is required.
- When determinism across runtimes matters (CI snapshot tests, server
  rendering, WASM).

The crate refuses to load system fonts. Every font face referenced by
the SVG must be supplied through `PngOptions::fonts`. This is the
project-wide constraint that keeps native and WASM bit-equal — see
[`docs/en/fonts.md`](../../../../docs/en/fonts.md).

## Quick start

```rust
use slideglance_png::{svg_to_png, FontData, PngOptions};

let svg = "<svg xmlns='http://www.w3.org/2000/svg' …>…</svg>";
let fonts = vec![FontData::new(std::fs::read("NotoSans-Regular.ttf")?)];
let out = svg_to_png(svg, &PngOptions {
    width: Some(1920),
    height: None,
    fonts,
})?;
std::fs::write("slide.png", &out.png)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Where to go next

- [Reference](./reference.md)
- [Guides](./guides.md)
- Source: [`crates/slideglance-png/src/lib.rs`](../../src/lib.rs)
