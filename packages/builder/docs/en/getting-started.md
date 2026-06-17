---
title: "@slideglance/builder — Getting Started"
lang: en
kind: guides
package: builder
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/builder/src/index.ts
  - packages/builder/src/buildPptx.ts
---

# Getting Started

This guide walks from "I just installed `@slideglance/builder`" to a multi-slide deck composed across several files.

## Install

```sh
pnpm add @slideglance/builder
```

Requires Node.js ≥ 22.

## A first slide

`buildPptx` takes XML, slide size, and an options object; returns a [pptxgenjs](https://gitbrent.github.io/PptxGenJS/) instance and a list of diagnostics.

```ts
import { buildPptx } from "@slideglance/builder";

const xml = `
<SlideGlance>
  <Document size="16:9" />
  <Slide>
    <VStack padding="48" gap="16">
      <Text fontSize="40" bold="true">Hello, builder</Text>
      <Text fontSize="20" color="666666">A declarative way to author PowerPoint slides.</Text>
    </VStack>
  </Slide>
</SlideGlance>
`;

const { pptx } = await buildPptx(xml, { w: 1280, h: 720 });
await pptx.writeFile({ fileName: "hello.pptx" });
```

`{ w: 1280, h: 720 }` is the slide size in pixels (96 DPI). The builder normalizes everything to inches internally.

## Anatomy of a deck

A document has four kinds of children at its root:

| Element                 | Purpose                                                               |
| ----------------------- | --------------------------------------------------------------------- |
| `<Document>`            | Slide size, default master, default text style. Optional but typical. |
| `<Slide>`               | One slide. The body is a single layout subtree.                       |
| `<Master name="...">`   | Reusable header / footer / background.                                |
| `<Styles>` / `<Style>`  | Named attribute presets referenced by `class="..."`.                  |
| `<Templates>` / `<Use>` | Reusable XML fragments parameterized by `{placeholder}`.              |
| `<Import src="...">`    | Splices another file in place.                                        |
| `<Notes>`               | Speaker notes for the parent `<Slide>`.                               |

`<Slide>` accepts exactly **one root child**, which is typically a `<VStack>`, `<HStack>`, or `<Layer>` filled with content.

## Add a second slide

```xml
<SlideGlance>
  <Document size="16:9" />

  <Slide>
    <VStack padding="48" gap="16">
      <Text fontSize="40" bold="true">Q4 Highlights</Text>
      <Text fontSize="20" color="666666">Three things that mattered.</Text>
    </VStack>
  </Slide>

  <Slide>
    <VStack padding="48" gap="12">
      <Text fontSize="32" bold="true">Revenue</Text>
      <Text fontSize="18">Up 12% YoY, driven by enterprise renewals.</Text>
    </VStack>
  </Slide>
</SlideGlance>
```

Run it again and you have a two-slide deck.

## Layout containers

`<VStack>` stacks children vertically; `<HStack>` arranges them horizontally; `<Layer>` lets children specify absolute `x` / `y`. They share Flexbox semantics — `gap`, `padding`, `alignItems`, `justifyContent`, `flexShrink`, etc. — so existing flexbox intuition transfers.

```xml
<HStack padding="48" gap="24" alignItems="start">
  <VStack w="50%" gap="8">
    <Text fontSize="32" bold="true">Left column</Text>
    <Text fontSize="16">Body text here.</Text>
  </VStack>
  <VStack w="50%" gap="8">
    <Text fontSize="32" bold="true">Right column</Text>
    <Text fontSize="16">More body text.</Text>
  </VStack>
</HStack>
```

`w="50%"` is a percentage of the parent. `w="max"` consumes remaining space. Plain numbers are pixels. See [Layout & styling](./layout-and-styling.md).

## Add a master slide

A `<Master>` is a reusable backdrop. Attach it to slides via `master="..."`.

```xml
<SlideGlance>
  <Document size="16:9" defaultMaster="CORP" />

  <Master name="CORP" backgroundColor="F8FAFC">
    <MasterRect x="0" y="0" w="1280" h="40" fill="0F172A" />
    <MasterText x="48" y="12" w="200" h="28" text="ACME Corp" color="FFFFFF" fontSize="14" />
    <SlideNumber x="1180" y="690" fontSize="10" color="666666" format="{n} / {N}" />
  </Master>

  <Slide>
    <VStack padding="80" gap="24">
      <Text fontSize="40" bold="true">Q4 Highlights</Text>
    </VStack>
  </Slide>
</SlideGlance>
```

Every slide picks up the header bar and page number automatically. See [Layout & styling → Master slides](./layout-and-styling.md#master-slides).

## Reuse with styles

Tag any node with `class="name"` to apply a named attribute preset.

```xml
<Styles>
  <Style name="page" padding="48" backgroundColor="F8FAFC" />
  <Style name="title" fontSize="40" bold="true" color="0F172A" />
  <Style name="muted" fontSize="18" color="64748B" />
</Styles>

<Slide>
  <VStack class="page" gap="12">
    <Text class="title">Q4 Highlights</Text>
    <Text class="muted">Three things that mattered.</Text>
  </VStack>
</Slide>
```

Multiple classes are allowed (`class="title primary"`); per-node attributes override class values.

## Split the deck across files

Once a deck grows past one file, lift shared pieces into separate XML and `<Import>` them.

`./styles.xml`:

```xml
<Fragment>
  <Styles>
    <Style name="page" padding="48" backgroundColor="F8FAFC" />
    <Style name="title" fontSize="40" bold="true" color="0F172A" />
  </Styles>
</Fragment>
```

`./main.sgx`:

```xml
<SlideGlance>
  <Document size="16:9" />
  <Import src="./styles.xml" />

  <Slide>
    <VStack class="page">
      <Text class="title">Q4 Highlights</Text>
    </VStack>
  </Slide>
</SlideGlance>
```

The caller supplies a synchronous resolver:

```ts
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { buildPptx } from "@slideglance/builder";

const resolveImport = (src: string, fromPath: string | undefined) => {
  const baseDir = fromPath ? dirname(fromPath) : process.cwd();
  const path = resolve(baseDir, src);
  return { content: readFileSync(path, "utf8"), path };
};

const { pptx } = await buildPptx(
  readFileSync("./main.sgx", "utf8"),
  { w: 1280, h: 720 },
  { resolveImport, sourcePath: resolve("./main.sgx") },
);
```

See [Composition](./composition.md) for templates, control flow, and security notes for untrusted input.

## Diagnostics

`buildPptx` collects non-fatal warnings (image not measurable, autofit overflow, master pptx parse failure, …) in `result.diagnostics`. Pass `strict: true` to upgrade them to a `DiagnosticsError`.

```ts
const { pptx, diagnostics } = await buildPptx(xml, { w: 1280, h: 720 });
for (const d of diagnostics) {
  console.warn(`[${d.code}] ${d.message}`);
}
```

The full code list is in [API reference → Diagnostic codes](./api.md#diagnostic-codes).

## Where to go next

- [API reference](./api.md) — every option and exported type.
- [XML reference](./xml-reference.md) — hand-picked examples for each node.
- [Layout & styling](./layout-and-styling.md) — Flex behavior, positioning, colors, fonts, decoration, master slides.
- [Composition](./composition.md) — templates, imports, control flow.
- [Schema reference](../../reference.md) — full attribute table for every element (auto-generated).
- [Reference deck](../../../../examples/builder-reference/) — runnable end-to-end example.
