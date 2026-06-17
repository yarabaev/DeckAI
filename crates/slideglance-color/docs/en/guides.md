---
title: slideglance-color — Guides
lang: en
kind: guides
crate: slideglance-color
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-color/src/resolver.rs
  - crates/slideglance-color/src/transforms.rs
---

# slideglance-color — Guides

## Resolve a theme-referenced color

### Goal

A shape carries `<a:solidFill><a:schemeClr val="accent1"/></a:solidFill>`.
Resolve it against the current theme.

### Code

```rust
use slideglance_color::{
    ColorRef, ColorResolver, ColorScheme, Rgb, SchemeColorKey,
};

let scheme = ColorScheme::office_2007();
let resolver = ColorResolver::new(scheme, None);

let resolved = resolver.resolve(
    &ColorRef::Scheme(SchemeColorKey::Accent1),
);
let rgb: Rgb = resolved.rgb;
```

### What's happening

The resolver inlines the scheme color into a concrete `Rgb`. If the
slide carries a `<p:clrMapOvr>`, pass a `ColorMap` as the second
argument and the resolver will route `bg1` / `tx1` / etc. through the
override before looking up the scheme.

## Apply a tint transform

### Goal

Render `accent1` lightened to 40% — equivalent to
`<a:schemeClr val="accent1"><a:lumMod val="60000"/><a:lumOff val="40000"/></a:schemeClr>`.

### Code

```rust
use slideglance_color::{
    apply_color_transforms, ColorRef, ColorResolver, ColorScheme,
    ColorTransform, PerMille, Rgb, SchemeColorKey,
};

let resolver = ColorResolver::new(ColorScheme::office_2007(), None);
let base = resolver.resolve(
    &ColorRef::Scheme(SchemeColorKey::Accent1),
).rgb;

let transforms = vec![
    ColorTransform::LumMod(PerMille(60_000)),
    ColorTransform::LumOff(PerMille(40_000)),
];
let result = apply_color_transforms(base, &transforms);
let final_rgb: Rgb = result.rgb;
```

### What's happening

Transforms are applied in the order received. `LumMod` and `LumOff`
operate in HSL space (Luminosity channel); `apply_color_transforms`
converts in, applies, and converts back to sRGB. Order matters —
swapping `LumMod` and `LumOff` produces a different color.
