---
title: slideglance-emf
lang: en
kind: index
crate: slideglance-emf
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-emf/src/lib.rs
  - crates/slideglance-emf/src/emf.rs
  - crates/slideglance-emf/src/wmf.rs
---

# slideglance-emf

> Part of the SlideGlance workspace.
> See also: [all crates](../../../../docs/en/crates.md).

## What it is

EMF / WMF metafile **raster extraction** — pulls the embedded DIB
bitmap out of bitmap-wrapping metafiles and converts it to a BMP / PNG
byte stream. Does *not* render arbitrary vector EMF / WMF; the goal is
the much narrower case of "the metafile is a wrapped bitmap, get the
bitmap out".

## Where it sits

```
slideglance-parser  (image branch)
        ↓
slideglance-emf  (raster extraction)
        ↓
renderer image pipeline
```

PowerPoint frequently embeds JPEG / PNG photos wrapped in an EMF
header for legacy compatibility. Unwrapping is required to render
them faithfully without dragging in a full GDI emulation.

## When to use this

- The parser encountered `<a:blip r:embed="…">` referencing an
  `.emf` or `.wmf` file.
- You need to detect whether a metafile is bitmap-wrap or true vector
  before deciding to skip / convert / rasterize.

## Quick start

```rust
use slideglance_emf::{detect_metafile_kind, extract_raster, MetafileKind};

let bytes = std::fs::read("image.emf")?;
match detect_metafile_kind(&bytes) {
    Some(MetafileKind::Emf) | Some(MetafileKind::Wmf) => {
        if let Some(raster_png) = extract_raster(&bytes) {
            std::fs::write("image.png", &raster_png)?;
        }
    }
    None => eprintln!("not a recognised metafile"),
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Where to go next

- [Reference](./reference.md)
- [Guides](./guides.md)
- Source: [`crates/slideglance-emf/src/`](../../src/)
