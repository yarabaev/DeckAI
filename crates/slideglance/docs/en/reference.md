---
title: slideglance — Reference
lang: en
kind: reference
crate: slideglance
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance/src/lib.rs
  - crates/slideglance/src/convert/
  - crates/slideglance/src/doc.rs
  - crates/slideglance/src/cache.rs
  - crates/slideglance/src/embedded_fonts.rs
  - crates/slideglance/src/font_usage.rs
  - crates/slideglance/src/clr_map_override.rs
  - crates/slideglance/src/bin/
---

# slideglance — Reference

## Crate layout

```
crates/slideglance/
├── src/
│   ├── lib.rs               # re-exports
│   ├── convert/             # convert_to_svg, convert_to_png, ConvertOptions
│   ├── doc.rs               # PptxDocument, RenderedSlide, MediaBlob, SlideRenderOptions
│   ├── cache.rs             # per-deck rendering cache
│   ├── embedded_fonts.rs    # extract / register embedded fonts
│   ├── font_usage.rs        # TypefaceUsage, build_typeface_usage
│   ├── clr_map_override.rs  # <p:clrMapOvr> resolution
│   └── bin/                 # CLI binary entry
└── Cargo.toml
```

## Public items

### `pub fn parse_pptx(bytes: impl Into<Vec<u8>>) -> Result<Presentation, PptxError>`

Parses a `.pptx` byte buffer and returns a fully resolved
`Presentation` — every slide is merged with its layout / master
inheritance and the text-style chain is resolved.

### `pub enum PptxError`

The crate's error type. Wraps every lower-layer error
(`ArchiveError`, `XmlError`, parser-module errors, renderer errors).

### `pub fn convert_to_svg(...)` and `convert_to_png(...)`

Re-exported from the `convert` module:

| Function | Returns |
|---|---|
| `convert_to_svg(bytes, opts)` | `Vec<String>` — one SVG per slide |
| `convert_to_png(bytes, opts)` | `Vec<Vec<u8>>` — one PNG per slide |

Both accept a `ConvertOptions` carrying font buffers, render mode,
target DPI, and other knobs. See
[`src/convert/mod.rs`](../../src/convert/mod.rs) for the full
signature.

### `pub struct PptxDocument`

Stateful wrapper for incremental rendering. Methods on the JS side
mirror this API via `slideglance-wasm`. See
[`src/doc.rs`](../../src/doc.rs):

- `PptxDocument::new(bytes, options)`
- `PptxDocument::render_slide(index, &SlideRenderOptions) -> RenderedSlide`
- `PptxDocument::media_blobs() -> Vec<MediaBlob>`

### `pub fn extract_referenced_font_families(bytes: &[u8]) -> Vec<String>`

Walks the deck and returns every authored font family referenced by
text runs, paragraph defaults, or theme schemes. Used by the
embed-font tooling and by hosts that pre-fetch fonts before
rendering.

### Font-pipeline re-exports

- `slideglance_font::RenderMode`
- `BufferFontResolver`, `CjkPlatform`, `FontMapping`, `FontResolver`,
  `FontStyle`, `TextMeasurer`, `standard_resolver_chain`, …

These are exposed so a consumer can build a custom resolver chain
without depending on `slideglance-font` directly.

### `TypefaceUsage` and `build_typeface_usage`

Reports which typefaces appear in the deck, with run counts.

## CLI binary

`bin/slideglance` (`cargo install slideglance`):

```text
slideglance convert <PPTX> --out <DIR>   # write SVGs (one per slide)
slideglance render  <PPTX> --slide <N> --format svg|png
slideglance inspect <PPTX> --fonts|--media|--slides
```

See [`src/bin/`](../../src/bin/) for the full subcommand list.

## Determinism

Same `bytes` + same `ConvertOptions` (including font buffers in the
same order) produces bit-identical output. Inherited from
`slideglance-renderer` and `slideglance-png`.
