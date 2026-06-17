---
title: slideglance-font
lang: en
kind: index
crate: slideglance-font
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-font/src/lib.rs
  - crates/slideglance-font/src/text_engine.rs
  - crates/slideglance-font/src/font_resolver.rs
---

# slideglance-font

> Part of the SlideGlance workspace.
> See also: [all crates](../../../../docs/en/crates.md) ·
> [fonts overview](../../../../docs/en/fonts.md).

## What it is

The font pipeline: mapping authored typefaces to physical fonts,
measuring text advances, shaping glyphs, and selecting fallbacks for
unsupported scripts (especially CJK). Used by both the renderer
(`slideglance-renderer`) and the standalone measurement WASM
(`slideglance-measure-wasm`).

## Where it sits

```
slideglance-utils
        ↓
slideglance-font  ──▶  slideglance-renderer
        ↓
slideglance-measure-wasm
```

## When to use this

- The renderer needs to convert authored fonts (`<a:latin typeface="…">`)
  to actual `FontFace` instances.
- A consumer needs to measure text advances at runtime.
- You are extending fallback policy for a script the workspace does
  not yet handle.

## Key concepts

- **Resolver chain**: `Mapped(CjkFallback(Buffer))` is the
  default — caller-provided buffers first, then platform CJK
  fallbacks, then the global typeface-mapping table.
- **Render mode**: `text` (preserve `<text>` for selectable output)
  vs `path` (convert to `<path>` for guaranteed rasterizer
  reproducibility).
- **Measurer**: `OpentypeTextMeasurer` (precise, uses parsed
  `name`/`OS/2` tables) vs `HeuristicTextMeasurer` (no font data
  required, used as last-resort fallback).

## Quick start

```rust,no_run
use slideglance_font::{
    standard_resolver_chain, BufferFontResolver, CjkPlatform, FontResolver,
};
use std::sync::Arc;

let buffer_resolver: Arc<dyn FontResolver> = Arc::new(
    BufferFontResolver::from_buffers(vec![/* TTF bytes */]),
);
let chain = standard_resolver_chain(buffer_resolver, CjkPlatform::Generic);

// `chain` resolves an authored typeface name to a concrete FontFace.
```

## Where to go next

- [Reference](./reference.md)
- [Guides](./guides.md)
- [Workspace fonts overview](../../../../docs/en/fonts.md)
- Source: [`crates/slideglance-font/src/`](../../src/)
