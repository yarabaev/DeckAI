---
title: slideglance-measure-wasm — Reference
lang: en
kind: reference
crate: slideglance-measure-wasm
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-measure-wasm/src/lib.rs
---

# slideglance-measure-wasm — Reference

## Crate layout

Single-file crate at [`src/lib.rs`](../../src/lib.rs).

## wasm-bindgen surface

### Class `TextMeasurer` (JS) / `WasmTextMeasurer` (Rust)

Exported via `#[wasm_bindgen(js_name = TextMeasurer)]`.

Construct once with a set of font byte buffers — fonts are parsed
exactly once at construction time, which matters for callers that
drive measurement from a hot path (e.g. a layout engine's wrap
callback firing per word).

| Method (JS) | Purpose |
|---|---|
| `new TextMeasurer(buffers: Uint8Array[])` | Build from font byte buffers |
| `measureWidth(text, family, sizePt, bold, …)` | Pixel advance for the run |

For the full method list including ascent / descent helpers, see
[`src/lib.rs`](../../src/lib.rs). The exact signatures are what
`wasm-pack` emits from the `#[wasm_bindgen]` annotations.

### Standalone function

- `pub fn version() -> String` — crate version string.

## Bold detection

Any face whose `OS/2.usWeightClass >= 600` is registered as the bold
variant of the corresponding family. `measureWidth(..., bold = true)`
then resolves to the Bold face directly with no caller-side family
rename. The 600 cutoff matches CSS's `bolder` keyword and catches
everything from Semibold (600) up through Black (900).

## Parity with the renderer

The measurer's resolver chain is
`Mapped(CjkFallback(Buffer))` — identical to the chain
`@slideglance/core` uses on the render side. A host that drives both
measurement (`@slideglance/measure`) and rendering
(`@slideglance/core`) over the same font buffers gets identical
pixel advances on either side.

## Cargo manifest notes

Like `slideglance-wasm`, this crate does **not** inherit workspace
fields. `wasm-pack`'s manifest parser rejects `field.workspace =
true`. `scripts/sync-versions.mjs` syncs the version.
