---
title: slideglance-wasm
lang: en
kind: index
crate: slideglance-wasm
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-wasm/src/lib.rs
---

# slideglance-wasm

> Part of the SlideGlance workspace.
> See also: [all crates](../../../../docs/en/crates.md) ·
> [packages/core](../../../../packages/core/docs/en/index.md).

## What it is

The `wasm-bindgen` entry point for the full SlideGlance pipeline.
Re-exports `parse_pptx`, `convert_to_svg`, `convert_to_png`, and the
font-resolution surface from `slideglance` and `slideglance-font` so
JS hosts can drive the same orchestrator the native CLI uses.

Built into the npm package `@slideglance/core` via `wasm-pack`.

## Where it sits

```
slideglance (lib + CLI, native)
        ↓
slideglance-wasm (wasm-bindgen ABI)
        ↓
@slideglance/core (npm)
        ↓
@slideglance/viewer, web-playground, chrome-extension, …
```

## When to use this

- You are building or extending `@slideglance/core` and need to add
  a wasm-bindgen export.
- You are diagnosing a JS-side failure that originates in the WASM
  layer (panic, serde error, font-resolver mismatch).

End-user JS code never imports this crate directly — it consumes the
npm package built from it.

## Quick start

Build the WASM bundle:

```sh
wasm-pack build crates/slideglance-wasm --target bundler
```

Use the produced bundle from JS:

```ts
import init, { convert_pptx_to_svg } from "@slideglance/core";

await init();
const svgs = convert_pptx_to_svg(new Uint8Array(pptxBytes), {});
```

## Where to go next

- [Reference](./reference.md)
- [Guides](./guides.md) — adding a new wasm-bindgen export
- Source: [`crates/slideglance-wasm/src/lib.rs`](../../src/lib.rs)
- Built artefact: [`@slideglance/core`](../../../../packages/core/docs/en/index.md)
