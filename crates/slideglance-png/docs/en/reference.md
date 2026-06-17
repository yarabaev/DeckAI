---
title: slideglance-png — Reference
lang: en
kind: reference
crate: slideglance-png
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-png/src/lib.rs
---

# slideglance-png — Reference

## Crate layout

```
crates/slideglance-png/
├── src/lib.rs
└── Cargo.toml
```

A single-module crate. The entire public surface is in `lib.rs`.

## Public items

### Constants

- `pub const DEFAULT_DPI: f32 = 96.0` — the DPI used when the SVG
  omits explicit width / height attributes. Matches the default
  `usvg::Options::dpi`.

### `pub struct FontData`

Caller-supplied font byte buffer (TTF / OTF / TTC). TTC files are
expanded automatically; every face becomes available to the
rasterizer.

| Field | Type | Purpose |
|---|---|---|
| `bytes` | `Vec<u8>` | Raw font file bytes |

Constructor: `pub fn new(bytes: Vec<u8>) -> Self`.

### `pub struct PngOptions`

Conversion options. `width` takes precedence over `height` when both
are provided — matches the spec contract.

| Field | Type | Purpose |
|---|---|---|
| `width` | `Option<u32>` | Output width in pixels |
| `height` | `Option<u32>` | Output height in pixels (honored only when `width` is `None`) |
| `fonts` | `Vec<FontData>` | Font buffers registered with the fontdb |

Default impl uses `None` / `None` / empty vec.

### `pub struct PngOutput`

Successful rasterization result.

| Field | Type | Purpose |
|---|---|---|
| `png` | `Vec<u8>` | PNG-encoded byte buffer (8-bit RGBA) |
| `width` | `u32` | Output width |
| `height` | `u32` | Output height |

### `pub enum PngError`

`thiserror`-derived. Variants include `Parse(usvg::Error)`,
`InvalidDimensions { width, height }`, plus encoder failures. See
[`src/lib.rs`](../../src/lib.rs) for the full list.

### `pub fn svg_to_png(svg: &str, options: &PngOptions) -> Result<PngOutput, PngError>`

The single conversion entry point.

## Determinism contract

The crate is configured for byte-identical output across native and
WASM targets:

- `usvg::TextRendering::GeometricPrecision`
- `usvg::ShapeRendering::GeometricPrecision`
- `usvg::ImageRendering::OptimizeQuality`
- `fontdb` populated only from `options.fonts`; system fonts never
  loaded.

These settings are project policy
(`.plans/00-rust-migration/plan.md` lines 524-532). Changing them
requires a recorded plan entry and a VRT update.
