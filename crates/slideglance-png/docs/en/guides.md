---
title: slideglance-png — Guides
lang: en
kind: guides
crate: slideglance-png
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-png/src/lib.rs
---

# slideglance-png — Guides

## Rasterize a slide SVG to PNG at 1920 px wide

### Goal

Convert one slide's SVG (produced by `slideglance-renderer`) into a
PNG suitable for a thumbnail strip.

### Code

```rust,no_run
use slideglance_png::{svg_to_png, FontData, PngOptions};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svg = fs::read_to_string("slide-1.svg")?;
    let font_bytes = fs::read("NotoSans-Regular.ttf")?;

    let out = svg_to_png(&svg, &PngOptions {
        width: Some(1920),
        height: None,
        fonts: vec![FontData::new(font_bytes)],
    })?;

    fs::write("slide-1.png", &out.png)?;
    println!("{}×{}", out.width, out.height);
    Ok(())
}
```

### What's happening

`svg_to_png` parses the SVG with `usvg`, applies the deterministic
rendering settings, and rasterizes through `tiny_skia` into a pixmap
that is encoded as 8-bit RGBA PNG. Because `width` is set and
`height` is `None`, the output preserves the SVG's intrinsic aspect
ratio.

## Provide multiple font faces

### Goal

A slide uses several typefaces (Latin + CJK + symbols). All must be
present in `fonts` or text will fall back to nothing and the glyphs
will be missing.

### Code

```rust,no_run
use slideglance_png::{svg_to_png, FontData, PngOptions};
use std::fs;

fn faces() -> std::io::Result<Vec<FontData>> {
    Ok(vec![
        FontData::new(fs::read("NotoSans-Regular.ttf")?),
        FontData::new(fs::read("NotoSansCJK-Regular.ttc")?),
        FontData::new(fs::read("NotoSansSymbols2-Regular.ttf")?),
    ])
}
```

### What's happening

`FontData::new` wraps the raw bytes; for TTC files every face inside
is registered. The fontdb is built fresh for each `svg_to_png` call —
keep your font buffers around between calls instead of re-reading
them from disk.
