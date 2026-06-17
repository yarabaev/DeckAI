---
title: "@slideglance/builder"
lang: en
kind: index
package: builder
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/builder/src/index.ts
  - packages/builder/src/buildPptx.ts
---

# @slideglance/builder

> Part of the SlideGlance workspace.
> See also: [all packages](../../../../docs/en/packages.md).

## What it is

A declarative slide builder: an XML DSL (`.sgx`) plus a Flexbox-style
layout engine, compiled to a real, editable `.pptx`. Layout decisions
are driven by the same OpenType measurements
[`@slideglance/measure`](../../../measure/docs/en/index.md) exposes,
so the rendered PPTX width-wraps the same way the viewer does.

## Install

```sh
npm i @slideglance/builder
```

## When to use this

- Programmatically generating PPTX from data (reports, dashboards,
  AI-authored slides).
- Authoring decks in source-controlled XML rather than binary PPTX.
- Writing a higher-level visual editor that emits `.sgx`.

## Quick start

```ts
import { buildPptx } from "@slideglance/builder";

const xml = `<?xml version="1.0" encoding="UTF-8"?>
<SlideGlance xmlns="urn:slideglance:builder:v1">
  <Document size="16:9" />
  <Slide>
    <VStack padding="32">
      <Text fontSize="44" bold="true">Hello, slides.</Text>
      <Text fontSize="20">Authored from XML.</Text>
    </VStack>
  </Slide>
</SlideGlance>`;

const { pptxBytes } = await buildPptx(xml, {});
await Bun.write("out.pptx", pptxBytes);
```

## Where to go next

- [Reference](./reference.md)
- [Guides](./guides.md)
- Schema: [`packages/builder/builder.xsd`](../../builder.xsd)
- JSON Schema: [`packages/builder/builder.schema.json`](../../builder.schema.json)
- Source: [`packages/builder/src/`](../../src/)
