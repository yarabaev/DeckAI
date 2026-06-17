---
title: slideglance-color
lang: en
kind: index
crate: slideglance-color
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-color/src/lib.rs
  - crates/slideglance-color/src/resolver.rs
  - crates/slideglance-color/src/scheme.rs
---

# slideglance-color

> Part of the SlideGlance workspace.
> See also: [all crates](../../../../docs/en/crates.md) ·
> [architecture](../../../../docs/en/architecture.md).

## What it is

OOXML theme color resolution and color transforms (ECMA-376
§20.1.2.3 and §20.1.6). Implements `lumMod`, `lumOff`, `tint`,
`shade`, `alpha` transforms, the eight-name preset table, and the
sRGB ↔ HSL conversion needed by the transforms.

## Where it sits

```
slideglance-utils ← slideglance-color ← slideglance-model
```

Purely model-level — does not touch XML. The parser constructs
`ColorRef` values and feeds them through `ColorResolver` to obtain a
`ResolvedColor` (RGB + alpha).

## When to use this

- Resolving any `<a:srgbClr>`, `<a:schemeClr>`, `<a:prstClr>`, or
  `<a:sysClr>` value to a final RGBA.
- Applying tint / shade / luminosity transforms to a color.
- Looking up a theme color by `ColorScheme` name.

## Quick start

```rust
use slideglance_color::{ColorRef, ColorResolver, ColorScheme, Rgb};

let resolver = ColorResolver::new(ColorScheme::office_2007(), None);
let resolved = resolver.resolve(&ColorRef::Srgb(Rgb::new(0x44, 0x55, 0x66)));
```

## Where to go next

- [Reference](./reference.md)
- [Guides](./guides.md)
- Source: [`crates/slideglance-color/src/`](../../src/)
