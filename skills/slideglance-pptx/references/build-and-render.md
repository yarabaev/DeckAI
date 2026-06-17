# Build and render — `.sgx` → `.pptx` → PNG

This file collects the production pipeline an author actually runs:
how to invoke `buildPptx`, how to handle the diagnostics it returns,
how to render the resulting `.pptx` to PNG for visual review, and how
to preview live in the VS Code extension. SKILL.md only sketches the
contract; this file has the concrete code.

## 1 — `buildPptx` from a single `.sgx`

```ts
import { readFileSync, writeFileSync } from "node:fs";
import { buildPptx } from "@slideglance/builder";

const xml = readFileSync("./deck.sgx", "utf8");

const { pptx, diagnostics } = await buildPptx(
  xml,
  { w: 1280, h: 720 },
  {
    textMeasurement: "auto",
    sourcePath: "./deck.sgx",
    equalize: true,
    lint: { enabled: true, ruleset: "recommended" },
  },
);

if (diagnostics.length > 0) {
  for (const d of diagnostics) {
    console.warn(
      `${d.severity ?? "info"} ${d.code}: ${d.message} @ ${d.path ?? "?"}`,
    );
  }
}

const bytes = (await pptx.write({ outputType: "uint8array" })) as Uint8Array;
writeFileSync("./deck.pptx", bytes);
```

### Options that matter

| Option            | Default        | What it does                                                                                          |
| ----------------- | -------------- | ----------------------------------------------------------------------------------------------------- |
| `textMeasurement` | `"auto"`       | `"auto"` → opentype for bundled families, heuristic otherwise. `"opentype"` forces opentype.          |
| `sourcePath`      | `undefined`    | Anchors `<Import src="…">` resolution and improves diagnostic paths.                                  |
| `equalize`        | `true`         | Equalize sibling columns in `<HStack>` rows when widths are unspecified.                              |
| `lint`            | disabled       | `{ enabled: true, ruleset: "recommended" \| "strict" }` — see [`lint.md`](lint.md).                   |
| `masterPptx`      | `undefined`    | Inherit background from an existing `.pptx` (see [`composition.md`](composition.md) § Master).        |
| `resolveImport`   | _filesystem_   | Hook for `<Import>` resolution — required in non-Node hosts (browser, Deno, sandbox). See below.      |

## 2 — Multi-file `.sgx` with `<Import>` resolution

For decks split across files (the recommended structure for 2+ slide
decks — see [`composition.md`](composition.md) § `<Import>`), provide
`resolveImport` so `<Import src="…">` can read sibling files:

```ts
import * as path from "node:path";
import { readFileSync } from "node:fs";

const { pptx, diagnostics } = await buildPptx(
  xml,
  { w: 1280, h: 720 },
  {
    sourcePath: "./deck/main.sgx",
    resolveImport: (src, fromPath) => {
      const baseDir = fromPath ? path.dirname(fromPath) : process.cwd();
      const absolute = path.resolve(baseDir, src);
      return { content: readFileSync(absolute, "utf8"), path: absolute };
    },
    lint: { enabled: true, ruleset: "recommended" },
  },
);
```

In browser / sandbox hosts: enforce a base directory and refuse
imports that escape it. Untrusted decks can otherwise read arbitrary
files via `<Import src="../../etc/passwd">`.

## 3 — Diagnostic severity policy

`buildPptx` returns a `diagnostics` array. Treat severity as a hard
contract:

| Severity | Build status                                  | Author action                                                                  |
| -------- | --------------------------------------------- | ------------------------------------------------------------------------------ |
| `error`  | `.pptx` may still be emitted, but it's broken | **Stop.** Fix the source before shipping.                                      |
| `warn`   | `.pptx` emitted, layout likely wrong          | Fix the source. Don't ship warnings — recipients will see the layout bug.      |
| `info`   | `.pptx` emitted, style nitpick                | `"recommended"` ruleset suppresses these; `"strict"` surfaces them. Fix when easy. |

The lint catalog and autofix patterns live in [`lint.md`](lint.md).
Parse-time errors (`ParseXmlError: XML validation failed`) and their
common causes live in [`schema-gotchas.md`](schema-gotchas.md).

## 4 — Render `.pptx` to PNG for visual review

The structural file count is not evidence of correctness. **A deck is
not authored until you have a green PNG in your hand.**

When working inside the slideglance repository, render through the
locally built CLI from the repository root:

```sh
./target/release/slideglance convert deck.pptx --output ./out --format png --width 1280
```

Single slide:

```sh
./target/release/slideglance render deck.pptx --slide 3 --output slide-3.png --width 1920
```

Inspect deck metadata without rendering:

```sh
./target/release/slideglance inspect deck.pptx
```

### Font handling at render time

The CLI walks standard system font directories on startup
(macOS: `/System/Library/Fonts`, `/Library/Fonts`, `~/Library/Fonts`;
Linux: `/usr/share/fonts`, `~/.local/share/fonts`, `~/.fonts`;
Windows: `C:\Windows\Fonts`). Families the deck requests resolve
against those bytes.

When a deck names a family the host doesn't have installed (e.g.
`fontFamily="Inter Display"` on a Mac without Inter), the CLI falls
back to the platform's OS-default Latin sans (`Helvetica Neue` /
`Segoe UI` / `Liberation Sans`) so text still renders. The output
will not be glyph-identical to what a PowerPoint recipient with the
proper font installed will see, but it stays readable.

For pixel-exact rendering — especially for CJK decks — pass the
needed faces explicitly via `--font`:

```sh
./target/release/slideglance convert deck.pptx --output ./out --format png --width 1280 \
    --font /System/Library/Fonts/AppleSDGothicNeo.ttc \
    --font ~/Library/Fonts/Pretendard-Regular.otf \
    --font ~/Library/Fonts/Pretendard-Bold.otf
```

`--font` is repeatable. Pass every weight a deck references — passing
only `Regular` will make bold text fall back, often to a different
face.

## 5 — Live preview in VS Code while authoring

The [`SlideGlance PPTX Viewer`](https://marketplace.visualstudio.com/items?itemName=slideglance.slide-builder)
extension renders `.sgx` next to the editor and re-renders on save
(with keystroke-level incremental updates that preserve unchanged
slides). One command writes the `.pptx` for export.

```sh
code --install-extension slideglance.slide-builder
```

The extension hard-depends on Red Hat XML for schema-aware editing
and on-save validation against the bundled XSD (namespace
`urn:slideglance:builder:v1`). Click any rendered element to jump
back to its source `<Slide>` / `<Use>` / `<Foreach>` site —
`<Import>` boundaries are followed.

## 6 — The author → lint → render → review loop

```
┌─────────────┐   ┌──────────────┐   ┌─────────────┐   ┌──────────┐
│  Edit .sgx  │ → │  buildPptx + │ → │   render    │ → │  visual  │
│             │   │     lint     │   │   to PNG    │   │  review  │
└─────────────┘   └──────────────┘   └─────────────┘   └──────────┘
                       ↓ errors?           ↓ wrong?         ↓ wrong?
                       └──── back to .sgx ─┘                │
                                                            │
                              ↑─────────────────────────────┘
```

A deck passes review when:

1. `buildPptx` returns zero `error` / `warn` diagnostics.
2. Every PNG in the rendered batch matches the intent — no overflow,
   no broken text, no missing glyphs (or only the expected font-fallback
   substitutions).
3. Opening the `.pptx` in PowerPoint matches the PNG (modulo
   recipient-side font installation).
