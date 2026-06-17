---
title: slideglance-measure-wasm
lang: en
kind: index
crate: slideglance-measure-wasm
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-measure-wasm/src/lib.rs
---

# slideglance-measure-wasm

> Part of the SlideGlance workspace.
> See also: [all crates](../../../../docs/en/crates.md) ·
> [packages/measure](../../../../packages/measure/docs/en/index.md).

## What it is

A measurement-only WASM crate. Exposes a single class —
`TextMeasurer` — that wraps `slideglance_font::OpentypeTextMeasurer`
and returns the pixel advance for a given `(text, font, size, bold)`
tuple.

Built into the npm package `@slideglance/measure`.

## Why a separate crate?

`slideglance-wasm` packages the full pipeline (parser + renderer +
resvg + serde) at ~5 MiB compressed. A consumer that only needs
text measurement — typically an upstream layout engine sharing its
pixel-advance numbers with the renderer — does not need any of that.
Splitting the wasm boundary keeps the measurement bundle roughly
10× smaller and lets the two ship on independent release cadences.

## Where it sits

```
slideglance-font
        ↓
slideglance-measure-wasm
        ↓
@slideglance/measure (npm)
        ↓
@slideglance/builder (layout)
```

## When to use this

- Building the `@slideglance/measure` npm artefact.
- Adding a measurement-only function used during PPTX *authoring*.

For runtime rendering of an already-built PPTX, use
`@slideglance/core` (which embeds full measurement).

## Quick start

```sh
wasm-pack build crates/slideglance-measure-wasm --target bundler
```

```ts
import init, { TextMeasurer } from "@slideglance/measure";

await init();
const measurer = new TextMeasurer([noto_sans_bytes]);
const px = measurer.measureWidth("Hello", "Noto Sans", 28, false);
```

## Where to go next

- [Reference](./reference.md)
- [Guides](./guides.md)
- Source: [`src/lib.rs`](../../src/lib.rs)
- Built artefact: [`@slideglance/measure`](../../../../packages/measure/docs/en/index.md)
