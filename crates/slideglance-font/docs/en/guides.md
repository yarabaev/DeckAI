---
title: slideglance-font — Guides
lang: en
kind: guides
crate: slideglance-font
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-font/src/font_resolver.rs
  - crates/slideglance-font/src/text_engine.rs
  - crates/slideglance-font/src/text_measurer.rs
---

# slideglance-font — Guides

## Build a resolver chain for browser rendering

### Goal

Combine caller-supplied buffer fonts with CJK fallbacks and the
authored-name mapping table.

### Code

```rust,no_run
use slideglance_font::{
    standard_resolver_chain, BufferFontResolver, CjkPlatform, FontResolver,
};
use std::sync::Arc;

let buffers = vec![std::fs::read("NotoSans-Regular.ttf")?];
let buffer_resolver: Arc<dyn FontResolver> = Arc::new(
    BufferFontResolver::from_buffers(buffers)?,
);

let chain: Arc<dyn FontResolver> =
    standard_resolver_chain(buffer_resolver, CjkPlatform::Generic);
# Ok::<(), Box<dyn std::error::Error>>(())
```

### What's happening

`standard_resolver_chain` wraps the buffer resolver in
`CjkFallback` (handles `Jpan`, `Hang`, `Hans`, `Hant` equally) and
then in `Mapping` (resolves authored aliases like `+mj-lt` →
`Calibri` via the workspace mapping table). When the renderer asks
for a face, it falls through the chain until one resolver returns
`Some`.

## Measure a text run with the OpenType measurer

### Goal

Compute the pixel advance of a string at 28 pt bold, given the same
fonts the renderer will use.

### Code

```rust,no_run
use slideglance_font::{
    standard_resolver_chain, BufferFontResolver, CjkPlatform, FontResolver,
    FontStyle, OpentypeTextMeasurer, TextMeasurer,
};
use std::sync::Arc;

let buffers = vec![std::fs::read("NotoSans-Regular.ttf")?,
                    std::fs::read("NotoSans-Bold.ttf")?];
let buffer_resolver: Arc<dyn FontResolver> = Arc::new(
    BufferFontResolver::from_buffers(buffers)?,
);
let resolver = standard_resolver_chain(buffer_resolver, CjkPlatform::Generic);

let measurer = OpentypeTextMeasurer::new(resolver);
let style = FontStyle::new("Noto Sans", 28.0, /* bold */ true);
let advance_px = measurer.measure_width("Hello, world.", &style);
# Ok::<(), Box<dyn std::error::Error>>(())
```

### What's happening

`OpentypeTextMeasurer` parses `name` and `hmtx` tables once per face
and caches the result. The resolver picks the bold variant
automatically when `FontStyle::bold` is `true` and a face with
`OS/2.usWeightClass >= 600` is registered. If no face matches at
all, falls back to `HeuristicTextMeasurer` with a logged warning.

## Switch between text-mode and path-mode rendering

### Goal

Output selectable `<text>` elements when fonts are available in the
viewer, or always-correct `<path>` elements when they are not.

### Code

```rust,no_run
use slideglance_font::{RenderMode, TextEngineBuilder};

let text_engine = TextEngineBuilder::new()
    .render_mode(RenderMode::Path)   // or RenderMode::Text
    .build(/* resolver, measurer, … */);
```

### What's happening

`RenderMode::Text` emits `<text>` elements — small, selectable, but
the viewer must have the font installed. `RenderMode::Path` runs
each glyph through the OpenType outline-to-SVG converter, producing
larger but byte-deterministic SVG that the rasterizer can reproduce
pixel-for-pixel regardless of which fonts the viewer carries. The
workspace defaults to `Text` for the in-browser viewer and `Path`
for PNG export.
