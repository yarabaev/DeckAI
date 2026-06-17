---
title: "@slideglance/measure"
lang: en
kind: index
package: measure
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/measure/package.json
  - crates/slideglance-measure-wasm/src/lib.rs
---

# @slideglance/measure

> Part of the SlideGlance workspace.
> See also: [all packages](../../../../docs/en/packages.md).

## What it is

A measurement-only WebAssembly package. Exposes one class —
`TextMeasurer` — that returns the pixel advance for a `(text, font,
size, bold)` tuple. Built from
[`slideglance-measure-wasm`](../../../../crates/slideglance-measure-wasm/docs/en/index.md).

## Why a separate package from `@slideglance/core`?

`@slideglance/core` ships the full pipeline at ~5 MiB compressed.
`@slideglance/measure` ships only the measurement primitives at
roughly 10× smaller. The split lets layout engines (such as
[`@slideglance/builder`](../../../builder/docs/en/index.md)) share
the same measurement code with the renderer without paying for code
they will never run.

## Install

```sh
npm i @slideglance/measure
```

## When to use this

- A layout engine that decides slide content **before** PPTX is
  written.
- Any tool that needs to know "how wide will this run be?" without
  rendering.

## Quick start

```ts
import init, { TextMeasurer } from "@slideglance/measure";

await init();

const fonts = await Promise.all([
  fetch("/fonts/NotoSans-Regular.ttf").then((r) => r.arrayBuffer()),
  fetch("/fonts/NotoSans-Bold.ttf").then((r) => r.arrayBuffer()),
]);

const measurer = new TextMeasurer(fonts.map((b) => new Uint8Array(b)));
const widthPx = measurer.measureWidth("Hello", "Noto Sans", 24, false);
```

## Where to go next

- [Reference](./reference.md)
- [Guides](./guides.md)
- Underlying Rust crate: [`slideglance-measure-wasm`](../../../../crates/slideglance-measure-wasm/docs/en/index.md)
