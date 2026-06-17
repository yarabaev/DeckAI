---
title: slideglance-utils — Reference
lang: en
kind: reference
crate: slideglance-utils
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-utils/src/lib.rs
  - crates/slideglance-utils/src/units.rs
  - crates/slideglance-utils/src/constants.rs
---

# slideglance-utils — Reference

> A map into the public surface. Run
> `cargo doc --no-deps --package slideglance-utils --open` for full
> rendered docs.

## Crate layout

```
crates/slideglance-utils/
├── src/
│   ├── lib.rs           # re-exports
│   ├── units.rs         # Emu, Pt, HundredthPt newtypes
│   └── constants.rs     # EMU_PER_INCH, EMU_PER_POINT, DEFAULT_DPI, …
└── Cargo.toml
```

## Public items

### Module `units`

#### `pub struct Emu(pub i64)`

English Metric Units. The native coordinate unit of OOXML. `1 inch =
914,400 EMU`.
`#[repr(transparent)]`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Ord`.

Source: [`src/units.rs`](../../src/units.rs).

Methods:

| Signature | Purpose |
|---|---|
| `const fn new(i64) -> Self` | Wrap a raw EMU count |
| `fn from_f64(f64) -> Self` | Construct from `f64`, truncating |
| `const fn raw(self) -> i64` | Underlying integer |
| `fn to_pixels(self) -> f64` | Convert at 96 DPI |
| `fn to_pixels_at(self, dpi: u32) -> f64` | Convert at given DPI |
| `fn to_points(self) -> Pt` | Convert to `Pt` (1 pt = 12,700 EMU) |

#### `pub struct Pt(pub f64)`

Point — 1/72 of an inch. Used for font sizes and post-conversion
spacing.
`#[repr(transparent)]`, `Copy`, `PartialEq`, `PartialOrd`.

Methods:

| Signature | Purpose |
|---|---|
| `const fn new(f64) -> Self` | Wrap a raw `f64` |
| `const fn raw(self) -> f64` | Underlying float |
| `fn to_emu(self) -> Emu` | Convert to `Emu` |
| `fn to_pixels_at(self, dpi: u32) -> f64` | Convert at given DPI |

#### `pub struct HundredthPt(pub i64)`

1/100 of a point. Used by ECMA-376 paragraph spacing
(`a:spcPts`, `a:lnSpc`, etc.).
`#[repr(transparent)]`, `Copy`, `Eq`, `Hash`, `Ord`.

Methods:

| Signature | Purpose |
|---|---|
| `const fn new(i64) -> Self` | Wrap a raw `1/100 pt` count |
| `const fn raw(self) -> i64` | Underlying integer |
| `fn to_points(self) -> Pt` | Divide by 100 |

### Crate-level functions

#### `pub fn rotation_to_degrees(rotation: i64) -> f64`

Converts an OOXML rotation value (`a:xfrm rot`, in 60,000-ths of a
degree) to degrees.

Source: [`src/lib.rs`](../../src/lib.rs).

### Re-exports from `constants`

| Item | Value |
|---|---|
| `pub const EMU_PER_INCH: i64` | 914,400 |
| `pub const EMU_PER_POINT: i64` | 12,700 |
| `pub const DEFAULT_DPI: u32` | 96 |

For any other constant the crate exports, see
[`src/constants.rs`](../../src/constants.rs).

## Cargo features

| Feature | Effect |
|---|---|
| `serde` | Derives `Serialize` / `Deserialize` on `Emu`, `Pt`, `HundredthPt` |
