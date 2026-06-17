---
title: "@slideglance/core"
lang: en
kind: index
package: core
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/core/package.json
  - crates/slideglance-wasm/src/lib.rs
---

# @slideglance/core

> Part of the SlideGlance workspace.
> See also: [all packages](../../../../docs/en/packages.md) ·
> [distribution](../../../../docs/en/distribution.md).

## What it is

The deterministic PPTX → SVG / PNG conversion runtime, as a
WebAssembly module. Built from the Rust crate
[`slideglance-wasm`](../../../../crates/slideglance-wasm/docs/en/index.md)
via `wasm-pack`. Every other JS surface in the workspace consumes
this package.

## Install

```sh
npm i @slideglance/core
```

## When to use this

- Building a web viewer for `.pptx` files.
- Adding PPTX import to an existing JS / TS app.
- Driving the same pipeline the native CLI uses, but in the browser
  or in Node.

For React UI use the higher-level
[`@slideglance/viewer`](../../../viewer/docs/en/index.md) instead of
wiring `core` directly into JSX.

## Distribution shapes

The package ships three artefacts in one tarball:

| Subpath | Target | Use case |
|---|---|---|
| `dist/web/` | `wasm-pack --target web` | Modern browsers, `<script type="module">` |
| `dist/bundler/` | `wasm-pack --target bundler` | Webpack / Vite / Rollup |
| `dist/node/` | `wasm-pack --target nodejs` | Node 22+, server-side |

Resolution is via the `"exports"` field in `package.json`.

## Quick start

```ts
import init, {
  convert_pptx_to_svg,
  convert_pptx_to_png,
  PptxDocument,
} from "@slideglance/core";

await init();

const bytes = new Uint8Array(await (await fetch("/deck.pptx")).arrayBuffer());
const svgs: string[] = convert_pptx_to_svg(bytes, {});
```

## Where to go next

- [Reference](./reference.md)
- [Guides](./guides.md)
- Underlying Rust crate: [`slideglance-wasm`](../../../../crates/slideglance-wasm/docs/en/index.md)
