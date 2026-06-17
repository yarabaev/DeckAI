---
title: slideglance
lang: en
kind: index
crate: slideglance
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance/src/lib.rs
  - crates/slideglance/src/convert/
  - crates/slideglance/src/bin/
---

# slideglance

> Part of the SlideGlance workspace.
> See also: [all crates](../../../../docs/en/crates.md) ·
> [architecture](../../../../docs/en/architecture.md).

## What it is

The top-level entry crate. Composes parser + model + renderer +
rasterizer into a single library API (`parse_pptx`,
`convert_to_svg`, `convert_to_png`) and ships a native CLI binary
(`slideglance convert / render / inspect`).

## Where it sits

```
slideglance-{utils, color, model, parser, font, renderer, png}
                              ↓
                      slideglance (lib + bin)
                              ↓
            slideglance-wasm  ──▶  @slideglance/core
```

## When to use this

- Driving the full PPTX → SVG / PNG pipeline from Rust.
- Building command-line tooling around `.pptx` files.
- Adding orchestrator-level features (font embedding, cache, doc
  serialization) used by both native and WASM frontends.

## Quick start (library)

```rust,no_run
use slideglance::{convert_to_svg, ConvertOptions};

let bytes = std::fs::read("deck.pptx")?;
let svgs = convert_to_svg(bytes, &ConvertOptions::default())?;
for (i, svg) in svgs.iter().enumerate() {
    std::fs::write(format!("slide-{i}.svg"), svg)?;
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Quick start (CLI)

```sh
slideglance convert deck.pptx --out slides/
slideglance render deck.pptx --slide 1 --format png > slide-1.png
slideglance inspect deck.pptx --fonts
```

## Where to go next

- [Reference](./reference.md)
- [Guides](./guides.md)
- Source: [`crates/slideglance/src/`](../../src/)
