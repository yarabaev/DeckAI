# @slideglance/viewer

Framework-agnostic `<pptx-viewer>` Web Component.
## What it does

A drop-in viewer for `.pptx` files that handles parsing, rendering,
worker offloading, and the surrounding chrome (toolbar, thumbnails,
zoom, search, theme, print, PDF export). The bundle registers a
`<pptx-viewer>` Web Component and also exports a React component
(`<PptxPresentation>`) for React hosts.

All rendering is local — the package never uploads files or makes
network requests beyond the initial wasm bundle fetch.

## Install

```sh
pnpm add @slideglance/viewer @slideglance/core lit
```

## Use (Web Component)

```html
<pptx-viewer src="/decks/example.pptx" current-slide="1"></pptx-viewer>
<script type="module">
  import "@slideglance/viewer";
</script>
```

Or programmatically:

```ts
import "@slideglance/viewer";

const viewer = document.querySelector("pptx-viewer");
viewer.setBuffer(uint8ArrayOfPptxBytes);
viewer.addEventListener("slidechange", (ev) => {
  console.log(ev.detail.current, "/", ev.detail.total);
});
```

## Use (React)

```tsx
import { PptxPresentation, createWorkerController } from "@slideglance/viewer";
import { useEffect, useState } from "react";

function App({ src }: { src: Uint8Array }) {
  const [controller, setController] = useState(null);
  useEffect(() => {
    void createWorkerController().then(setController);
  }, []);
  return <PptxPresentation controller={controller} src={src} />;
}
```

## Properties

| Attribute / Property             | Type                                      | Default             |
| -------------------------------- | ----------------------------------------- | ------------------- |
| `src`                            | `string \| null`                          | `null`              |
| `current-slide` / `currentSlide` | `number`                                  | `1`                 |
| `text-render-mode`               | `"text" \| "path" \| "auto"`              | `"text"`            |
| `font-fallback`                  | `"first-available" \| "system" \| "none"` | `"first-available"` |

## Methods

- `setBuffer(input: Uint8Array | ArrayBuffer): Promise<SlideSvg[]>`
- `goToSlide(n: number): void`
- `nextSlide(): void`
- `prevSlide(): void`
- `resetView(): void`
- `requestFullscreen(): Promise<void>` (inherited)

## Events

- `slidechange` — `{ current, previous, total }`
- `loadprogress` — `{ phase, message? }` where `phase` is one of `fetch / wasm-init / parse / render / done`.
- `error` — `{ phase, message }`.

## Keyboard / mouse / touch

- `←` / `→` / `Space` / `PageUp` / `PageDown` — navigate.
- `Home` / `End` — jump to first / last slide.
- `Esc` — reset zoom & pan (or exit fullscreen if active).
- Mouse wheel — navigate. `Ctrl/⌘`+wheel — zoom.
- Drag — pan. Click left half / right half — prev / next.

## Theming

Override CSS custom properties to restyle the viewer:

```css
pptx-viewer {
  --pptx-viewer-bg: #fafafa;
  --pptx-viewer-fg: #111;
  --pptx-viewer-shadow: rgba(0, 0, 0, 0.18);
  --pptx-viewer-hud-bg: rgba(255, 255, 255, 0.85);
  --pptx-viewer-hud-fg: #111;
  --pptx-viewer-overlay: rgba(255, 255, 255, 0.85);
}
```