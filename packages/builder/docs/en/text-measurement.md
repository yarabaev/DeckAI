---
title: "@slideglance/builder — Text Measurement"
lang: en
kind: guides
package: builder
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/builder/src/buildPptx.ts
  - crates/slideglance-measure-wasm/src/lib.rs
---

# Text Measurement

The builder lays out text **before** PowerPoint sees it, so it must measure run widths the same way PowerPoint will. It does that with a WebAssembly OpenType measurer (`@slideglance/measure`) seeded with the bundled font set — and, optionally, every font buffer the caller passes through the `fonts` build option.

## Bundled fonts

Two families ship inside the package and are used for measurement:

- **Pretendard** (Korean + Latin sans-serif) — the default for non-Japanese decks.
- **Noto Sans JP** (Japanese sans-serif) — the default fallback when no `fontFamily` is set.

The actual rendered font on the recipient's machine is whatever PowerPoint resolves at open time. Bundled measurement gives layout consistency; rendering depends on installed fonts.

## Measurement modes

```ts
await buildPptx(
  xml,
  { w: 1280, h: 720 },
  {
    textMeasurement: "auto", // "opentype" | "fallback" | "auto"
  },
);
```

| Value        | Behavior                                                                                                                                                                                                                                                                                  |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `"opentype"` | Always use the OpenType measurer. Bundled fonts (and any caller-supplied fonts via the `fonts` option) measure with their native metrics. Unknown families substitute through Pretendard / Noto Sans JP via the script-aware picker.                                                      |
| `"fallback"` | Always use the heuristic estimator (CJK = 1em; Latin per-character lookup averaging ~0.45em). Font-independent; fastest; least accurate. Use only when you need a layout that does not depend on any installed-font metrics — e.g. golden-file tests or deterministic measurement bench.  |
| `"auto"`     | (Default.) Always uses the OpenType path. Caller-supplied families resolve natively; unknown families substitute through the script-aware bundled picker. This is the policy that keeps the builder aligned with the renderer's font-fallback chain — see "Builder ↔ renderer alignment". |

> **Behavior change (T-?)**: `"auto"` mode previously routed non-bundled fonts to the heuristic estimator. That created a gap where layout-time wrap differed from render-time wrap (workshop deck exhibited horizontal overflow; editorial deck exhibited vertical overlap). The mode now routes every family to the OpenType measurer with substitution, mirroring how the viewer's `pptx-worker.ts` measurement and the Rust renderer both follow the actual font-fallback chain. Decks that need the legacy behavior should set `textMeasurement: "fallback"` explicitly.

## Builder ↔ renderer alignment

The builder, the viewer, and the Rust renderer each have their own font-fallback chain. The current state of each layer:

| Layer                                | Font fallback policy                                                                                                                                                               |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Builder layout (`"auto"`)            | OpenType measurer with bundled fonts + caller-supplied `fonts`. Unknown families substitute via `pickBundledFontForText` (Pretendard for Hangul-dominant, Noto Sans JP otherwise). |
| Viewer (`pptx-worker.ts`)            | OffscreenCanvas with the deck's `font-family` plus the CSS `@font-face` chain the host injects via `fontStylesheet`.                                                               |
| Rust renderer (`slideglance render`) | Auto-loads `~/Library/Fonts`, `/System/Library/Fonts`, `~/.fonts`. Falls back to its own bundled set when nothing matches.                                                         |

To guarantee that layout-time wrap = render-time wrap, supply the same TTF buffers to **both** the builder (`fonts` build option) and the viewer (`fontStylesheet` prop, as `@font-face` data URIs pointing at the same bytes):

```ts
const interRegular = await fs.readFile("Inter-Regular.ttf");
const interBold = await fs.readFile("Inter-Bold.ttf");

const { pptx } = await buildPptx(xml, slideSize, {
  fonts: [interRegular, interBold],
  textMeasurement: "auto",
});

// Same buffers, hashed into a data-URI @font-face stylesheet for the viewer.
const fontStylesheet = makeFontFaceCss({
  Inter: [interRegular, interBold],
});
```

Slideglance routes faces with `OS/2.usWeightClass >= 600` to the bold-variant slot automatically, so a single `fontFamily="Inter"` + `bold="true"` lookup reaches the Bold face.

## Performance

OpenType measurement runs in a single WebAssembly module loaded once per process. It adds milliseconds — not seconds — to a typical deck build. The heuristic measurer is allocation-free and runs in microseconds.
