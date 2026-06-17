---
title: "@slideglance/viewer"
lang: en
kind: index
package: viewer
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/viewer/src/index.ts
  - packages/viewer/src/PptxPresentation.tsx
---

# @slideglance/viewer

> Part of the SlideGlance workspace.
> See also: [all packages](../../../../docs/en/packages.md).

## What it is

A React-based PPTX presentation viewer backed by
[`@slideglance/core`](../../../core/docs/en/index.md). Provides the
top-level `<PptxPresentation>` shell (toolbar, stage, status bar,
zoom, keyboard navigation) plus utilities for hosts that want to
render slides directly without the React shell.

## Install

```sh
npm i @slideglance/viewer @slideglance/core react react-dom
```

## When to use this

- Embedding a PPTX viewer into a React app.
- Building a custom shell on top of the worker-controller pattern.
- Reusing the SVG post-processing utilities (`prepareSvg`,
  `extractAndStripFontStyle`, …) outside the React shell.

## Quick start

```tsx
import { PptxPresentation, createWorkerController } from "@slideglance/viewer";

const controller = createWorkerController();

export function MyViewer({ bytes }: { bytes: Uint8Array }) {
  return <PptxPresentation controller={controller} source={bytes} />;
}
```

## Where to go next

- [Reference](./reference.md)
- [Guides](./guides.md)
- Source: [`packages/viewer/src/`](../../src/)
