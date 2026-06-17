---
title: slideglance-utils
lang: en
kind: index
crate: slideglance-utils
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-utils/src/lib.rs
  - crates/slideglance-utils/src/units.rs
  - crates/slideglance-utils/src/constants.rs
---

# slideglance-utils

> Part of the SlideGlance workspace.
> See also: [all crates](../../../../docs/en/crates.md) ·
> [architecture](../../../../docs/en/architecture.md).

## What it is

The lowest layer of the workspace dependency graph. Provides
unit-aware primitives — `Emu`, `Pt`, `HundredthPt` — and conversion
helpers, plus the shared constants for OOXML coordinate math.

## Where it sits

```
slideglance-utils  ← color, model, parser, font, renderer, png, …
```

Every other `slideglance-*` crate depends on `slideglance-utils`.
Raw `i64` / `f64` lengths must never cross a module boundary without
going through these newtypes; a workspace-wide convention treats raw
numbers in public signatures as a code-review block.

## When to use this

- Authoring or reviewing code that handles PPTX lengths (EMU, points,
  hundredths of a point).
- Computing slide layout, shape transforms, or font sizes.
- Reading or writing the geometry attributes in OOXML.

## Quick start

```rust
use slideglance_utils::{Emu, Pt};

// 1 inch = 914,400 EMU
let one_inch = Emu::new(914_400);
assert_eq!(one_inch.to_pixels(), 96.0);

// 1 pt = 12,700 EMU
let twelve_pt = Pt::new(12.0);
assert_eq!(twelve_pt.to_emu().raw(), 152_400);
```

## Where to go next

- [Reference](./reference.md) — full public API surface
- [Guides](./guides.md) — common conversion recipes
- Source: [`crates/slideglance-utils/src/`](../../src/)
