---
title: "@slideglance/builder — VS Code Extension Companion"
lang: en
kind: guides
package: builder
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - apps/vscode-extension/package.json
  - apps/vscode-extension/src/
---

# VS Code Extension

**SlideGlance PPTX Viewer** (`slideglance.slide-builder`) is the editor companion for `@slideglance/builder`. It turns any `.sgx` file into a live-preview surface with click-to-source and one-command PPTX export.

The extension lives at [`apps/vscode-extension/`](../../../../apps/vscode-extension/) in the SlideGlance repo and is published to the VS Code Marketplace under the publisher `slideglance`.

## What it does

- **Live preview** — opens a webview that re-renders on save (and on keystroke for unchanged-slide-preserving incremental updates). Built-in `@slideglance/viewer` renders SVG; scroll position, zoom, and thumbnail selection survive edits.
- **Selective invalidation** — edits to `<Import>`'d files invalidate just the slides that referenced them. Changes to `<Master>`, `<Styles>`, `<Templates>`, `defaultTextStyle`, or slide size trigger a full rebuild.
- **Click → reveal source** — clicking any rendered element jumps the editor to the originating XML, including across `<Import>` boundaries. Works because the extension passes `trackSourcePos: true` and reads `objectName="node#N"` back from the rendered SVG.
- **PowerPoint-style zoom** — slider, ± input, "fit to width". Zoom level persists per webview session.
- **Thumbnail rail** — slide thumbnails at the bottom (landscape) or left (portrait). Drag-resizable; remembers its size; scroll-spy highlights the current slide.
- **PPTX export** — one command produces the `.pptx` file using the same `buildPptx` pipeline as the preview.
- **PPTX viewer** — `.pptx` files open in the same viewer through the `slideBuilder.pptxViewer` custom editor (priority: `option`; opt in via "Open With…").
- **Inline diagnostics** — parse / schema errors appear in the editor as you type.

## Install

> Requires VS Code 1.85+.

From the Marketplace UI: search **SlideGlance PPTX Viewer** by **slideglance**.

From the CLI:

```sh
code --install-extension slideglance.slide-builder
```

## Use

1. Open any XML file you author with the builder DSL (`.sgx` is the convention).
2. Run **SlideGlance: Open Preview** from the Command Palette, or click the preview icon in the editor title bar.
3. Edit the file — the preview updates in real time.
4. Run **SlideGlance: Export PPTX** when you're ready to write the `.pptx` file.

`<Import src="..." />` resolves relative to the open file's directory, so multi-file decks work without configuration.

## Commands

| Command                       | Title                        | Effect                              |
| ----------------------------- | ---------------------------- | ----------------------------------- |
| `slideBuilder.openPreview`    | SlideGlance: Open Preview    | Open or focus the live preview pane |
| `slideBuilder.refreshPreview` | SlideGlance: Refresh Preview | Force a full rebuild                |
| `slideBuilder.exportPptx`     | SlideGlance: Export PPTX     | Write the current deck to `.pptx`   |

## Schema-aware editing

The bundled XSD (`packages/builder/builder.xsd`, namespace `urn:slideglance:builder:v1`) drives the [Red Hat XML extension](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-xml) for autocomplete and on-save validation when you wire it up:

```jsonc
// .vscode/settings.json
{
  "xml.fileAssociations": [
    {
      "pattern": "**/*.sgx",
      "systemId": "./node_modules/@slideglance/builder/builder.xsd",
    },
  ],
}
```

The SlideGlance PPTX Viewer extension itself does not require the namespace — declaring it only enables Red Hat XML's tooling.

## How it works

```
.sgx file
  ↓ (edit)
extension host (Node)
  ↓ trackSourcePos: true
@slideglance/builder  →  PPTX bytes  →  webview
                                          ↓
                                       @slideglance/viewer
                                          ↓
                                       SVG paint
                                          ↓ click
                                       data-object-name="node#N"
                                          ↓ postMessage
                                       extension host looks up
                                       BuilderSourceMap[N] → { file, line }
                                          ↓
                                       vscode.window.showTextDocument
```

Two bundle pipelines coexist because the viewer's worker resolves `@slideglance/core`'s WASM via dynamic import + top-level await — Vite's `vite-plugin-wasm` + `vite-plugin-top-level-await` handle that, esbuild does not. The host code (extension entry, preview controller, export command, custom editor, webview HTML) is plain Node and bundles fine through esbuild.

## Develop locally

```sh
git clone https://github.com/SlideGlance/slideglance.git
cd slideglance
pnpm install
pnpm --filter slide-builder build
```

Open `apps/vscode-extension/` in VS Code and press F5 to launch an Extension Development Host with the extension loaded. Edits to host code rebuild via:

```sh
pnpm --filter slide-builder watch:host
```

The webview rebuilds via Vite — `pnpm --filter slide-builder build:webview`.

## Limitations

- The extension bundles its own copy of `@slideglance/builder`. Projects that pin a different builder version at runtime may render slightly differently in the preview than in the final PPTX output. Keep the extension and library versions in sync for the most accurate preview.
- The PPTX custom editor opens with `priority: "option"`, so `.pptx` files do **not** open in SlideGlance PPTX Viewer by default. Use **File → Open With… → SlideGlance PPTX Viewer** to opt in per file, or the **vscode** workspace setting to opt in globally.
