---
title: slideglance-parser
lang: en
kind: index
crate: slideglance-parser
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-parser/src/lib.rs
  - crates/slideglance-parser/src/archive.rs
  - crates/slideglance-parser/src/xml.rs
---

# slideglance-parser

> Part of the SlideGlance workspace.
> See also: [all crates](../../../../docs/en/crates.md) ·
> [architecture](../../../../docs/en/architecture.md).

## What it is

PPTX archive + OOXML XML parser. Reads a `.pptx` ZIP, walks the
relationships, and produces typed `slideglance-model` values for
themes, presentations, slide masters, slide layouts, slides, shapes,
text, fills, effects, charts, and tables.

## Where it sits

```
slideglance-model
        ↑
slideglance-parser
        ↓
slideglance-renderer
```

The parser is the only layer that touches XML. Every higher crate
consumes the typed model and never sees angle brackets.

## When to use this

- You have raw `.pptx` bytes and need typed model values.
- You are extending fidelity (a new OOXML element, a new chart
  variant, a new effect).
- You are diagnosing a parse mismatch against the spec.

## Quick start

```rust,no_run
use slideglance_parser::{parse_presentation, PptxArchive};

let bytes = std::fs::read("deck.pptx")?;
let archive = PptxArchive::open(&bytes)?;
let info = parse_presentation(&archive)?;
println!("{} slides", info.slide_paths.len());
# Ok::<(), Box<dyn std::error::Error>>(())
```

For a fully resolved `Presentation` use the higher-level
[`slideglance::parse_pptx`](../../../slideglance/docs/en/index.md#quick-start-library);
this crate exposes the per-part parsers for callers that need the
intermediate representations.

## Where to go next

- [Reference](./reference.md)
- [Guides](./guides.md)
- Source: [`crates/slideglance-parser/src/`](../../src/)
