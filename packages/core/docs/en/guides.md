---
title: "@slideglance/core — Guides"
lang: en
kind: guides
package: core
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/core/package.json
  - crates/slideglance-wasm/src/lib.rs
---

# @slideglance/core — Guides

## Convert a PPTX file in a browser tab

### Goal

User drops `deck.pptx` onto the page; the page shows the slide
SVGs.

### Code

```ts
import init, { convert_pptx_to_svg } from "@slideglance/core";

async function handleDrop(file: File) {
  await init();
  const bytes = new Uint8Array(await file.arrayBuffer());
  const svgs: string[] = convert_pptx_to_svg(bytes, {});

  const parser = new DOMParser();
  for (const svg of svgs) {
    // Parse the SVG string into a real SVG element rather than
    // splicing markup via innerHTML — the SVGs come from a parsed
    // .pptx, but treating them as a trusted document boundary keeps
    // the host page safe from any sanitisation regression upstream.
    const doc = parser.parseFromString(svg, "image/svg+xml");
    const node = document.adoptNode(doc.documentElement);
    document.body.append(node);
  }
}
```

### What's happening

`init()` instantiates the WASM module. Subsequent calls to
`convert_pptx_to_svg` reuse the loaded instance. With no fonts
passed in `opts`, text is rendered using the browser's `<text>`
elements (text-mode). Provide a font byte buffer to switch to
path-mode for cross-environment determinism.

## Install a canvas-based text measurer

### Goal

Match `font-kerning` and `letter-spacing` precisely against what the
browser will render.

### Code

```ts
const canvas = new OffscreenCanvas(0, 0);
const ctx = canvas.getContext("2d")!;

(globalThis as any).__slideglanceMeasureText = (
  text: string,
  family: string | undefined,
  _familyEa: string | undefined,
  _chain: string | undefined,
  sizePt: number,
  bold: boolean,
) => {
  ctx.font = `${bold ? "700" : "400"} ${sizePt}pt ${family ?? "sans-serif"}`;
  return ctx.measureText(text).width;
};
```

### What's happening

The WASM module imports `globalThis.__slideglanceMeasureText`. If
present, the Rust resolver uses its measurements instead of the
heuristic / OpenType paths. The host is responsible for matching
the canvas context's font features (e.g. `font-kerning: none`) to
the `<text>` element's attributes — mismatched settings show up as
sub-pixel position drift on long runs.

## Run conversion off the main thread in Node

### Goal

Server-side render a deck without blocking the event loop.

### Code

```ts
// worker.mjs
import { parentPort } from "node:worker_threads";
import init, { convert_pptx_to_png } from "@slideglance/core";

await init();
parentPort?.on("message", async (bytes) => {
  const pngs = convert_pptx_to_png(new Uint8Array(bytes), {});
  parentPort?.postMessage(pngs);
});
```

### What's happening

The `dist/node/` build registers the WASM through Node's binary
loader. The conversion itself is synchronous Rust code; running it in
a `worker_threads` worker is how to avoid blocking the main thread.
For browser equivalents, use a `Worker` or `ServiceWorker`.
