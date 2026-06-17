---
title: slideglance-measure-wasm — Guides
lang: en
kind: guides
crate: slideglance-measure-wasm
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-measure-wasm/src/lib.rs
---

# slideglance-measure-wasm — Guides

## Build the measurement WASM bundle

### Goal

Produce the WASM bundle consumed by `@slideglance/measure`.

### Steps

```sh
wasm-pack build crates/slideglance-measure-wasm --target bundler
```

### Expected result

A `pkg/` directory containing the WASM module plus generated TS
declarations.

## Drive measurement from a layout engine

### Goal

Wrap text at a target box width during PPTX authoring without
shipping the full renderer.

### Code

```ts
import init, { TextMeasurer } from "@slideglance/measure";

await init();

const measurer = new TextMeasurer([
  new Uint8Array(await fetch("/fonts/NotoSans-Regular.ttf").then(r => r.arrayBuffer())),
  new Uint8Array(await fetch("/fonts/NotoSans-Bold.ttf").then(r => r.arrayBuffer())),
]);

function fitsInBox(text: string, widthPx: number) {
  const advance = measurer.measureWidth(text, "Noto Sans", 24, false);
  return advance <= widthPx;
}
```

### What's happening

`TextMeasurer` is built once and re-used. The internal
`OpentypeTextMeasurer` parses each font file the first time it sees
it; subsequent calls hit the cached `FontFace`. Bold variants are
auto-detected (any face whose `OS/2.usWeightClass >= 600`), so a
caller does not need to pass a separate family name for the Bold
weight.
