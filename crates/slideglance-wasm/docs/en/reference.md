---
title: slideglance-wasm — Reference
lang: en
kind: reference
crate: slideglance-wasm
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-wasm/src/lib.rs
---

# slideglance-wasm — Reference

## Crate layout

Single-file crate at [`src/lib.rs`](../../src/lib.rs). The published
artefact is the wasm bundle produced by `wasm-pack`, not the Rust
crate.

## wasm-bindgen surface

### Standalone functions

| JS name | Rust signature | Purpose |
|---|---|---|
| `init()` | `pub fn init()` | Install the panic hook |
| `version()` | `pub fn version() -> String` | Crate version string |
| `emu_to_pixels(emu)` | `pub fn emu_to_pixels(emu: f64) -> f64` | Same as `Emu::to_pixels` |
| `parse_pptx_data(bytes)` | `pub fn parse_pptx_data(bytes: Vec<u8>) -> Result<JsValue, JsError>` | Returns a serialized `Presentation` |
| `convert_pptx_to_svg(bytes, opts)` | per source | Full-deck PPTX → SVG strings |
| `convert_pptx_to_png(bytes, opts)` | per source | Full-deck PPTX → PNG buffers |
| `svg_to_png_wasm(svg, opts)` | per source | SVG → PNG (wraps `slideglance-png`) |

### Class `PptxDocument`

`#[wasm_bindgen]`-exported. Stateful wrapper around the parsed
presentation; methods on the JS side mirror
`slideglance::PptxDocument` (load, render slide N, list typeface
usage, etc.).

For the precise method signatures, see
[`src/lib.rs`](../../src/lib.rs) — the JS class is what `wasm-pack`
generates from the `#[wasm_bindgen]` annotations.

## JS-side callbacks

The crate imports two callbacks the host **may** install:

| JS name | Purpose |
|---|---|
| `__slideglanceMeasureText` | Returns the pixel advance for a `(text, font, size, bold)` tuple |
| `__slideglanceMeasureLineMetrics` | Returns `{ ascent, descent, lineGap }` for a CSS font declaration |

If absent (or returning zeros), the Rust side falls back to the
heuristic measurer with a `console.warn`. See
[`fonts.md`](../../../../docs/en/fonts.md) for the wider font
pipeline.

## Determinism contract

When the same font buffers are passed in the same order, the output
is bit-identical to the native `slideglance` CLI driven over the
same input. See the
[`slideglance-png` determinism section](../../../slideglance-png/docs/en/reference.md#determinism-contract).

## Cargo manifest notes

`slideglance-wasm/Cargo.toml` does **not** inherit workspace fields.
`wasm-pack`'s manifest parser rejects `field.workspace = true`.
`scripts/sync-versions.mjs` keeps the version aligned with the
workspace.
