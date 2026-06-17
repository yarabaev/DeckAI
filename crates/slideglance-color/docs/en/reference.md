---
title: slideglance-color — Reference
lang: en
kind: reference
crate: slideglance-color
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-color/src/lib.rs
  - crates/slideglance-color/src/hsl.rs
  - crates/slideglance-color/src/rgb.rs
  - crates/slideglance-color/src/resolver.rs
  - crates/slideglance-color/src/scheme.rs
  - crates/slideglance-color/src/transforms.rs
  - crates/slideglance-color/src/presets.rs
---

# slideglance-color — Reference

## Crate layout

```
crates/slideglance-color/
├── src/
│   ├── lib.rs        # re-exports
│   ├── rgb.rs        # Rgb, ResolvedColor, ColorParseError
│   ├── hsl.rs        # Hsl + sRGB conversions
│   ├── scheme.rs     # ColorScheme, ColorMap, ColorRef, SchemeColorKey
│   ├── resolver.rs   # ColorResolver
│   ├── transforms.rs # ColorTransform, apply_color_transforms, PerMille
│   └── presets.rs    # resolve_preset (eight-name table)
└── Cargo.toml
```

## Public items

### Module `rgb`

- `pub struct Rgb { r: u8, g: u8, b: u8 }` — sRGB triple.
- `pub struct ResolvedColor { rgb: Rgb, alpha: PerMille }` — output
  of the resolver.
- `pub enum ColorParseError` — parse failure from `Rgb::from_hex`.

Source: [`src/rgb.rs`](../../src/rgb.rs).

### Module `hsl`

- `pub struct Hsl { h: f64, s: f64, l: f64 }` — HSL triple in
  `[0, 1]`. Conversion to/from `Rgb` matches W3C CSS Color Module
  Level 3 §4.2.4.

Source: [`src/hsl.rs`](../../src/hsl.rs).

### Module `scheme`

- `pub struct ColorScheme` — the twelve-slot theme color scheme.
- `pub struct ColorMap` — `clrMapOvr` mapping from logical names
  (`bg1`, `tx1`, etc.) to scheme slots.
- `pub enum ColorRef { Srgb(Rgb), Scheme(SchemeColorKey),
  Preset(&'static str), … }`.
- `pub enum SchemeColorKey` — the twelve scheme keys.
- `pub struct UnknownSchemeName` — error returned when a scheme name
  is invalid.

Source: [`src/scheme.rs`](../../src/scheme.rs).

### Module `resolver`

- `pub struct ColorResolver` — pairs a `ColorScheme` with an optional
  `ColorMap`. The single entry point for the parser → renderer flow.

Source: [`src/resolver.rs`](../../src/resolver.rs).

### Module `transforms`

- `pub enum ColorTransform { LumMod, LumOff, Tint, Shade, Alpha, … }`.
- `pub struct PerMille(pub i32)` — 1/1000 unit used by OOXML
  transform amounts.
- `pub fn apply_color_transforms(base: Rgb, transforms: &[ColorTransform]) -> ResolvedColor`.

Source: [`src/transforms.rs`](../../src/transforms.rs).

### Module `presets`

- `pub fn resolve_preset(name: &str) -> Option<Rgb>` — the eight-name
  preset color table. Returns `None` for any other name.

Source: [`src/presets.rs`](../../src/presets.rs).

## Notable design constraints

- `satMod` is intentionally omitted (matches the spec, the
  source of truth at this phase).
- The preset table is fixed at eight names — extending it requires a
  matching change in the spec and a recorded plan entry.
