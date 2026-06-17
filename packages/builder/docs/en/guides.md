---
title: "@slideglance/builder — Guides"
lang: en
kind: guides
package: builder
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/builder/src/buildPptx.ts
  - packages/builder/src/parseXml/parseXml.ts
  - packages/builder/src/diagnostics.ts
---

# @slideglance/builder — Guides

## Build a deck from XML

### Goal

Generate a `.pptx` from a hand-authored `.sgx` source.

### Code

```ts
import { buildPptx } from "@slideglance/builder";

const xml = `<?xml version="1.0" encoding="UTF-8"?>
<SlideGlance xmlns="urn:slideglance:builder:v1">
  <Document size="16:9" />
  <Slide>
    <VStack padding="48" gap="16">
      <Text fontSize="40" bold="true">Quarterly results</Text>
      <Text fontSize="20" color="#555">Q3 2026</Text>
    </VStack>
  </Slide>
</SlideGlance>`;

const { pptxBytes } = await buildPptx(xml, {});
await Bun.write("out.pptx", pptxBytes);
```

### What's happening

`buildPptx` runs the full pipeline: XML parse → diagnostics →
style equalisation → Yoga (Flexbox) layout → PPTX codegen. The output
opens unchanged in PowerPoint / Keynote / LibreOffice Impress —
nothing is left as a `<picture>` snapshot.

## Handle diagnostics from a bad source

### Goal

A pipeline accepts user-authored XML and must surface every parse /
layout / schema error.

### Code

```ts
import { buildPptx, DiagnosticsError } from "@slideglance/builder";

try {
  const { pptxBytes } = await buildPptx(xml, {});
} catch (err) {
  if (err instanceof DiagnosticsError) {
    for (const d of err.diagnostics) {
      console.error(
        `${d.code}: ${d.message} @ ${d.sourcePos?.line}:${d.sourcePos?.column}`,
      );
    }
    return;
  }
  throw err;
}
```

### What's happening

`DiagnosticsError` carries the full diagnostics array — every issue
the builder found, with `DiagnosticCode` discriminator, message, and
optional source position. Hosts use the source position to underline
the offending range in their editor.

## Supply a custom text measurer

### Goal

Replace the WASM measurer with a host-supplied function (e.g. a
Node-native shaper).

### Code

```ts
import { buildPptx, type TextMeasurementMode } from "@slideglance/builder";

const measurement: TextMeasurementMode = {
  kind: "callback",
  measureWidth(text, family, sizePt, bold) {
    return myShaper.measure(text, { family, sizePt, bold });
  },
};

const { pptxBytes } = await buildPptx(xml, { measurement });
```

### What's happening

By default the builder loads `@slideglance/measure` and uses the
WASM measurer. `TextMeasurementMode = { kind: "callback", ... }`
swaps in any function that returns pixel advances. The chosen
measurer must match what the **rendering** side uses, otherwise the
wrapped width will not match the on-screen width.
