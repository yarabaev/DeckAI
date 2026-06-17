---
title: "@slideglance/measure — Guides"
lang: en
kind: guides
package: measure
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/measure/package.json
  - crates/slideglance-measure-wasm/src/lib.rs
---

# @slideglance/measure — Guides

## Wrap a paragraph at a fixed pixel width

### Goal

Greedy word-wrap for a text run at 320 px wide before generating the
target PPTX.

### Code

```ts
import init, { TextMeasurer } from "@slideglance/measure";

await init();
const measurer = new TextMeasurer([
  new Uint8Array(await (await fetch("/fonts/NotoSans-Regular.ttf")).arrayBuffer()),
]);

function wrap(text: string, family: string, sizePt: number, widthPx: number) {
  const words = text.split(/\s+/);
  const lines: string[] = [];
  let current = "";
  for (const word of words) {
    const candidate = current ? `${current} ${word}` : word;
    const width = measurer.measureWidth(candidate, family, sizePt, false);
    if (width <= widthPx) {
      current = candidate;
    } else {
      if (current) lines.push(current);
      current = word;
    }
  }
  if (current) lines.push(current);
  return lines;
}
```

### What's happening

`measureWidth` reuses parsed font tables across calls — fast enough
to call in a tight loop. The result is in CSS pixels at the font's
intrinsic resolution; downstream `EMU` conversion uses the standard
factor (1 px @ 96 DPI = 9525 EMU).

## Use the same fonts for measurement and rendering

### Goal

A pipeline writes a PPTX (using `@slideglance/builder`) and then
renders it back (using `@slideglance/core`). Both must see the same
glyph widths.

### Code

```ts
import initMeasure, { TextMeasurer } from "@slideglance/measure";
import initCore, { convert_pptx_to_svg } from "@slideglance/core";

await Promise.all([initMeasure(), initCore()]);

const fontBuffers = [
  new Uint8Array(await fetch("/fonts/NotoSans-Regular.ttf").then(r => r.arrayBuffer())),
];

const measurer = new TextMeasurer(fontBuffers);
// … use measurer during PPTX authoring …

const svgs = convert_pptx_to_svg(pptxBytes, { fonts: fontBuffers });
// the SVGs render at the same widths the measurer reported
```

### What's happening

Both packages route font buffers through the same Rust crate
(`slideglance-font`). When given identical buffers, the rendered
glyph positions match the measured advances to the bit. This is the
constraint that makes `@slideglance/builder`'s flexbox layout
reliable.
