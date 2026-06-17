---
title: "@slideglance/viewer — Reference"
lang: en
kind: reference
package: viewer
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/viewer/src/index.ts
  - packages/viewer/src/PptxPresentation.tsx
  - packages/viewer/src/svg-utils.ts
  - packages/viewer/src/worker-controller.ts
  - packages/viewer/src/types.ts
  - packages/viewer/src/ui/SettingsDialog.tsx
---

# @slideglance/viewer — Reference

## Package shape

```
packages/viewer/
├── src/
│   ├── index.ts                 # public exports
│   ├── PptxPresentation.tsx     # top-level shell
│   ├── pptx-worker.ts           # web-worker source
│   ├── worker-controller.ts     # createWorkerController
│   ├── svg-utils.ts             # SVG post-processing helpers
│   ├── types.ts                 # public type aliases
│   ├── presentation/            # stage, toolbar, status bar
│   ├── ui/                      # dialogs, popovers
│   └── jsx-shim.d.ts
├── package.json
└── dist/                        # built bundle (Vite)
```

## Exports

### Components

| Name               | Purpose                                         |
| ------------------ | ----------------------------------------------- |
| `PptxPresentation` | Top-level shell with toolbar, stage, status bar |
| `SettingsDialog`   | The font-mapping / theme settings popover       |

Each component is paired with a `*Props` type alias.

### Functions

| Name                       | Purpose                                                          |
| -------------------------- | ---------------------------------------------------------------- |
| `createWorkerController`   | Web-Worker-backed `SlideController` for browser hosts            |
| `parseAspect`              | Read `viewBox` and `preserveAspectRatio` from an SVG string      |
| `prepareSvg`               | Sanitise + decorate a raw slide SVG before mounting              |
| `rewriteMediaRefs`         | Replace `<image href>` references with blob URLs                 |
| `extractAndStripFontStyle` | Pull `@font-face` rules out of an SVG into a separate stylesheet |
| `extractFontStyleCss`      | Read `@font-face` rules without removing them                    |

### Types

| Name              | Purpose                                                             |
| ----------------- | ------------------------------------------------------------------- |
| `SlideController` | The interface between the React shell and a slide-rendering backend |
| `RenderedSlide`   | `{ svg: string, mediaBlobs: MediaBlob[] }`                          |
| `MediaBlob`       | `{ id, mime, bytes }` for embedded images                           |
| `SlideSvg`        | Parsed-aspect SVG record                                            |
| `SlideMeta`       | Per-slide title / notes / size                                      |
| `TextRenderMode`  | `"text" \| "path"`                                                  |
| `FontFallback`    | Caller-provided font mapping                                        |

## Slide-controller contract

`SlideController` decouples the React shell from any specific backend:

```ts
interface SlideController {
  init(source: Uint8Array): Promise<void>;
  slideCount(): number;
  renderSlide(index: number, opts: SlideRenderOptions): Promise<RenderedSlide>;
  dispose(): void;
}
```

`createWorkerController` spawns a Web Worker that runs
`@slideglance/core` and implements the contract. Native hosts (Tauri,
Electron) supply their own controller talking to a Rust backend over
IPC.

## SVG post-processing pipeline

```
core.convert_pptx_to_svg
   ↓
prepareSvg          (parse, set xmlns)
   ↓
extractAndStripFontStyle  (split @font-face into <style>)
   ↓
rewriteMediaRefs    (blip refs → blob URLs)
   ↓
mount into <svg> in React tree
```

Hosts that bypass the React shell run this pipeline themselves; the
helpers are exported for that purpose.
