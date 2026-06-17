# @slideglance/measure

Deterministic OpenType text-width / line-metrics measurement as a WebAssembly module.


## What it does

Exposes the text-measurement entry point of `slideglance-font` as a
small WebAssembly bundle (~10× smaller than `@slideglance/core`). For
consumers that only need to measure text widths — typically an upstream
layout engine that wants to share metrics with the SlideGlance renderer
for pixel parity.

Three target builds ship under `dist/{bundler,web,node}/`. Pick the
`exports` subpath that matches your bundler.

## Why a separate package from `@slideglance/core`

`@slideglance/core` ships the full PPTX → SVG / PNG pipeline (~5 MiB
compressed wasm). A measurement-only consumer doesn't need any of
that. The two packages release on independent cadences.

## Install

```sh
pnpm add @slideglance/measure
```

## Quick start

```ts
import { measureTextWidth } from "@slideglance/measure";

const widthPx = measureTextWidth({
  text: "Hello",
  fontFamily: "Inter",
  fontSize: 16,
  fontBytes: await fetch("/fonts/Inter-Regular.otf").then((r) => r.arrayBuffer()),
});
```

Measurement never reads system fonts — supply the font bytes
explicitly via `fontBytes`. This is what makes the result deterministic
across machines and environments.
