---
title: slideglance-emf — Reference
lang: en
kind: reference
crate: slideglance-emf
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-emf/src/lib.rs
  - crates/slideglance-emf/src/emf.rs
  - crates/slideglance-emf/src/wmf.rs
---

# slideglance-emf — Reference

## Crate layout

```
crates/slideglance-emf/
├── src/
│   ├── lib.rs   # detect_metafile_kind, extract_raster*, MetafileKind
│   ├── emf.rs   # extract_raster_from_emf, EmfRasterError
│   └── wmf.rs   # extract_raster_from_wmf, WmfRasterError
└── Cargo.toml
```

## Public items

### Crate-level

#### `pub enum MetafileKind`

Variants: `Emf`, `Wmf`. Returned by `detect_metafile_kind`.

#### `pub fn detect_metafile_kind(bytes: &[u8]) -> Option<MetafileKind>`

Sniffs the leading header bytes. `None` means the buffer is neither
an EMF nor a WMF.

#### `pub fn extract_raster(bytes: &[u8]) -> Option<Vec<u8>>`

Returns the embedded bitmap re-encoded as PNG, or `None` if the
metafile is not a bitmap-wrap variant (it carries vector commands).

#### `pub fn extract_raster_as_bmp(bytes: &[u8]) -> Option<Vec<u8>>`

Same as `extract_raster` but emits a BMP header + DIB body.
Convenient for re-serialising into a destination metafile container.

### Module `emf`

- `pub fn extract_raster_from_emf(bytes: &[u8]) -> Result<Vec<u8>, EmfRasterError>`
- `pub enum EmfRasterError` — distinguishes "not a raster wrap" from
  parse errors.

Source: [`src/emf.rs`](../../src/emf.rs).

### Module `wmf`

- `pub fn extract_raster_from_wmf(bytes: &[u8]) -> Result<Vec<u8>, WmfRasterError>`
- `pub enum WmfRasterError`.

Source: [`src/wmf.rs`](../../src/wmf.rs).

## Out of scope

- Rendering arbitrary EMF / WMF vector records. This crate does not
  emulate GDI; for true vector metafiles the renderer falls back to
  the alt-text or skips the image.
- DIB → vector decompilation.
