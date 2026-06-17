---
title: "@slideglance/viewer — Guides"
lang: en
kind: guides
package: viewer
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/viewer/src/PptxPresentation.tsx
  - packages/viewer/src/worker-controller.ts
  - packages/viewer/src/svg-utils.ts
---

# @slideglance/viewer — Guides

## Embed the viewer in a React route

### Goal

Show a PPTX file at `/decks/:id` in a React Router app.

### Code

```tsx
import { useEffect, useState } from "react";
import { PptxPresentation, createWorkerController } from "@slideglance/viewer";

export function DeckRoute({ id }: { id: string }) {
  const [bytes, setBytes] = useState<Uint8Array | null>(null);
  const [controller] = useState(() => createWorkerController());

  useEffect(() => {
    fetch(`/api/decks/${id}.pptx`)
      .then((r) => r.arrayBuffer())
      .then((buf) => setBytes(new Uint8Array(buf)));
    return () => controller.dispose();
  }, [id, controller]);

  if (!bytes) return <p>Loading…</p>;
  return <PptxPresentation controller={controller} source={bytes} />;
}
```

### What's happening

`createWorkerController` spawns a Web Worker that loads
`@slideglance/core`. The React shell talks to it via the
`SlideController` interface, so the heavy parsing and rendering work
never blocks the main thread. Disposing the controller cleanly
terminates the worker — important when the route unmounts.

## Render a slide without the React shell

### Goal

A bare-bones HTML page wants to show one slide without pulling in
React.

### Code

```ts
import init, { convert_pptx_to_svg } from "@slideglance/core";
import {
  prepareSvg,
  extractAndStripFontStyle,
  rewriteMediaRefs,
} from "@slideglance/viewer";

await init();
const svgs: string[] = convert_pptx_to_svg(bytes, {});
const raw = svgs[0];

// Run the same post-processing the React shell uses.
const { svg, fontStyleCss } = extractAndStripFontStyle(raw);
const prepared = prepareSvg(svg);
const withMedia = rewriteMediaRefs(prepared, mediaBlobs);

document.querySelector("style")!.append(document.createTextNode(fontStyleCss));
const doc = new DOMParser().parseFromString(withMedia, "image/svg+xml");
document.body.append(document.adoptNode(doc.documentElement));
```

### What's happening

The helpers in `svg-utils` are the exact pipeline the React shell
uses, exposed for hosts that mount the SVG themselves. Running the
pipeline gets you matching `@font-face` extraction, media-blob
rewriting, and aspect-ratio metadata — without React.

## Supply a custom SlideController for a Tauri host

### Goal

A native desktop shell runs the Rust pipeline directly via Tauri IPC,
not through `@slideglance/core` in the browser.

### Code

```ts
import type { SlideController, RenderedSlide } from "@slideglance/viewer";
import { invoke } from "@tauri-apps/api/core";

export function createTauriController(): SlideController {
  let slideCount = 0;
  return {
    async init(source) {
      slideCount = await invoke<number>("load_deck", { bytes: source });
    },
    slideCount: () => slideCount,
    renderSlide: (index, opts) =>
      invoke<RenderedSlide>("render_slide", { index, opts }),
    dispose: () => {},
  };
}
```

### What's happening

The React shell does not care whether the controller talks to a
Web Worker, a native binary, or a remote API. Implement the
`SlideController` interface and pass it to `<PptxPresentation>`.
`apps/desktop-viewer` follows this exact pattern.
