---
title: slideglance-emf — Guides
lang: en
kind: guides
crate: slideglance-emf
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-emf/src/lib.rs
---

# slideglance-emf — Guides

## Detect and unwrap a bitmap-wrapped EMF

### Goal

A `.pptx` referenced `media/image3.emf`. Determine whether it is a
JPEG / PNG wrapper or a true vector metafile, and convert the wrapper
case to PNG.

### Code

```rust
use slideglance_emf::{detect_metafile_kind, extract_raster, MetafileKind};

fn unwrap_to_png(bytes: &[u8]) -> Option<Vec<u8>> {
    match detect_metafile_kind(bytes)? {
        MetafileKind::Emf | MetafileKind::Wmf => extract_raster(bytes),
    }
}
```

### What's happening

`detect_metafile_kind` looks at the leading 4–8 bytes (`\x01\x00\x00\x00`
EMF signature; `\xd7\xcd\xc6\x9a` placeable WMF signature). When the
metafile contains a single bitmap-wrap record, `extract_raster` re-
serialises the DIB body as PNG. `None` means either the input was not
a metafile or the metafile carries vector content — fall back to alt-
text or skip rendering.

## Re-emit as BMP for a downstream tool

### Goal

A pipeline expects BMP rather than PNG (e.g. it consumes Windows
DIBs directly).

### Code

```rust
use slideglance_emf::extract_raster_as_bmp;

if let Some(bmp_bytes) = extract_raster_as_bmp(&emf_bytes) {
    std::fs::write("unwrapped.bmp", &bmp_bytes)?;
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

### What's happening

`extract_raster_as_bmp` returns BMP file bytes (BMP header + DIB
header + pixel data). The DIB body is bit-identical to what
PowerPoint embedded — no recompression, no colour-space conversion.
