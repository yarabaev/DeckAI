---
title: slideglance-utils — Guides
lang: en
kind: guides
crate: slideglance-utils
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-utils/src/units.rs
---

# slideglance-utils — Guides

## Convert a slide width from EMU to pixels

### Goal

Render a 16:9 slide at 96 DPI. The OOXML default 16:9 slide width is
9,144,000 EMU.

### Code

```rust
use slideglance_utils::Emu;

let slide_width = Emu::new(9_144_000);
let px = slide_width.to_pixels();        // 960.0 at 96 DPI
let px_print = slide_width.to_pixels_at(300); // 3000.0 at 300 DPI
```

### What's happening

`Emu::to_pixels` is hard-wired to 96 DPI for the screen-rendering
default. For print or raster export, use `to_pixels_at` with the
target DPI; the rest of the conversion is `emu / 914_400 * dpi`.

## Convert a font size between EMU, points, and pixels

### Goal

OOXML stores font sizes inconsistently: paragraph spacing uses
`HundredthPt`, run-level `<a:rPr sz="…">` uses `HundredthPt` as a
plain integer attribute, and shape geometry uses `Emu`. Normalize to
`Pt` for measurement.

### Code

```rust
use slideglance_utils::{Emu, HundredthPt, Pt};

// Run-level font size: <a:rPr sz="2800"/> = 28 pt
let raw_attr: i64 = 2800;
let font_pt: Pt = HundredthPt::new(raw_attr).to_points();
assert!((font_pt.raw() - 28.0).abs() < 1e-9);

// Convert to pixels for the renderer
let px = font_pt.to_pixels_at(96); // 37.333… px

// Convert back to EMU for geometry math
let as_emu: Emu = font_pt.to_emu();
assert_eq!(as_emu.raw(), 28 * 12_700);
```

### What's happening

Every length goes through a typed newtype, so the compiler refuses to
add EMU to points. The conversion methods encode the OOXML constants
(`EMU_PER_INCH = 914_400`, `EMU_PER_POINT = 12_700`) once, in
`constants.rs`.

## Convert an OOXML rotation attribute

### Goal

OOXML stores rotation as 60,000-ths of a degree
(`<a:xfrm rot="5400000"/>` = 90°). The renderer expects degrees.

### Code

```rust
use slideglance_utils::rotation_to_degrees;

let rot: i64 = 5_400_000;
assert!((rotation_to_degrees(rot) - 90.0).abs() < 1e-9);
```

### What's happening

`rotation_to_degrees` divides by 60,000 (the OOXML angle unit). It is
a free function rather than a method on a newtype because rotation is
the only place this unit appears in the model — extending it to a
full `Degrees`-vs-`Radians` newtype pair is YAGNI for the current
codebase.
