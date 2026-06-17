# slideglance Builder — development reference

This file is for **contributors** working on the slideglance codebase
itself (especially `packages/builder/`), not for authors writing
`.sgx` decks. For the authoring grammar see [`grammar.md`](./grammar.md)
and the [`SKILL.md`](../SKILL.md) top-level entry.

The five topics below were previously split across `.claude/rules/*.md`;
they are merged here so a single skill invocation surfaces them all.

## @slideglance/builder (`packages/builder/`)

Declarative slide builder. TypeScript library that compiles an XML DSL
into PPTX, with yoga-layout-driven Flexbox positioning and pptxgenjs
rendering.

### Tech stack

TypeScript 5.6, yoga-layout 3.2.1, pptxgenjs 4.0.1, opentype-equivalent
measurement via `@slideglance/measure` (WASM), fast-xml-parser 5.x,
zod 4.x, Vitest 2.1, ESLint, Prettier, pnpm workspace.

### Behavioral principles

- Read existing code before making changes — especially check the 3-stage
  pipeline impact scope.
- When adding features, follow the **Feature Addition Checklist** below.
- VRT baseline updates must use the Docker environment
  (`pnpm run vrt:docker:update`).
- When changes span multiple packages, explicitly state the impact scope.

### Commands (from `packages/builder/`)

```bash
pnpm run build              # TypeScript compilation
pnpm run lint               # ESLint
pnpm run fmt                # Prettier formatting
pnpm run typecheck          # Type checking
pnpm run knip               # Detect unused code
pnpm run test:run           # Run tests
pnpm run codegen            # Regenerate dist-schema/{builder.xsd, builder.schema.json, reference.md}
pnpm run codegen:check      # CI: fail on codegen drift
pnpm run vrt:docker:update  # Update VRT baseline (Docker)
```

Root: `pnpm --filter @slideglance/builder run <script>`.

### Architecture

PPTX generation pipeline:

```
parseXml (with template macro expansion)
  → calcYogaLayout
  → toPositioned
  → renderPptx
```

Additionally, **autoFit** adjusts slides when content overflows.

#### Public API

- `buildPptx(xml, slideSize, options?)` — XML string → PPTX.
- `BuildPptxResult`, `ParseXmlError`, `DiagnosticsError`, `Diagnostic`,
  `DiagnosticCode`.
- `TextMeasurementMode` (`"opentype"` | `"fallback"` | `"auto"`),
  `SlideMasterOptions`.

#### Key internal types

- **`BuilderNode`** — Input node union: `Text`, `Ul`, `Ol`, `Image`,
  `Table`, `Shape`, `Chart`, `Line`, `Connector`, `Layer`, `VStack`,
  `HStack`, `Icon`, `Svg`.

- **`ConnectorNode`** — Smart line bound to two shapes by their
  author-facing `id`. Emits a PPTX `<p:cxnSp>` with `stCxn`/`endCxn`
  so PowerPoint auto-reroutes when shapes move. Skipped by Yoga
  layout; geometry derives from the from/to shapes' positioned boxes
  at render time, then a post-process pass over the zipped PPTX
  rewrites the placeholder `<p:sp>` (see
  `src/renderPptx/postProcess/`) because pptxgenjs has no cxnSp
  emit. Author-id markers and connector sigils flow through
  `<p:cNvPr name="sg-id:...">` / `name="sg-cxn:..."`.

- **`group` attribute** — any node can opt into a PowerPoint group
  (`<p:grpSp>`) wrapping it and its descendants. The renderer threads
  an ancestor group stack through `RenderContext.groupStack`; every
  leaf emits one `sg-grp:G` token per active ancestor on its
  `cNvPr@name`. A post-process pass
  (`renderPptx/postProcess/groupSp.ts`) walks the spTree top-down,
  finds contiguous runs sharing each group id at each depth, and
  wraps them into nested grpSp elements. `group="true"` produces an
  auto-named group; any other string is the stable group id (used as
  the group's `cNvPr@name`). Sigils are stripped on the way out so
  the final PPTX shows nothing unusual.

- **`PositionedNode`** — Node with absolute position (`x`, `y`, `w`, `h`).

- **`parseXml`** — XML strings → BuilderNode arrays (PascalCase tags,
  Zod-validated attributes).

- **`templates`** (`packages/builder/src/parseXml/templates.ts`) —
  Parse-time macro expansion for
  `<Templates>` / `<Template>` / `<Use>` / `<Slot>`. Runs over the
  raw XML tree before BuilderNode conversion, so the rest of the
  pipeline is unaffected. `{name}` placeholders substitute in
  attribute values and text content; `<Slot>` carries multi-element
  content. Recursive expansion has a depth limit of 32.

- **`imports`** (`packages/builder/src/parseXml/imports.ts`) —
  Parse-time file inlining for `<Import src="..."/>`. Runs before
  template expansion. The caller supplies a sync
  `resolveImport(src, fromPath) → { content, path }`; `<Import>` can
  sit anywhere in the tree. Imported files require a `<Fragment>`
  (or `<Presentation>`) root; other roots are rejected. Cycle
  detection uses the absolute `path`; depth limit is 16.

- **`registry/compiled/`**
  (`packages/builder/src/registry/compiled/index.ts`) — Compiled XML
  SSoT. Each node and meta element is declared via `defineNode` /
  `defineMeta`, importing the runtime Zod schema from `types.ts` and
  adding XML metadata (attribute `coerce` type, optional
  `objectShape` for `json` + dot-notation attrs, doc strings, child
  element cardinality). Read by `src/codegen/` to emit
  `dist-schema/builder.xsd` (target namespace
  `urn:slideglance:builder:v1`), `dist-schema/builder.schema.json`,
  and `dist-schema/reference.md`. **Also drives the runtime parser**:
  `parseXml/dispatcher.ts` reads `ALL_COMPILED_NODES` for tag /
  attribute validation and delegates string-to-typed-value coercion
  to `parseXml/coerceByType.ts` (CoerceType-driven engine).
  Per-child-element specs (Master objects, table Cell / Tr / Td,
  list Li, inline format A / B / I / U / S / Mark / Span) live in
  `parseXml/childAttributeSpecs.ts`.

### Examples

`examples/builder-reference/` is a runnable reference deck. Used as a
smoke test for the public API and as an end-user-facing showcase.
Build via `pnpm --filter @slideglance/builder-reference-example run build`.

## Feature Addition Checklist

When adding new properties or features to the builder, update the
following files in order:

1. **Type definitions**: `packages/builder/src/types.ts` — Add or
   extend the Zod schema for the affected node. The compiled
   registry imports these schemas directly.
2. **Node registry (XML SSoT)**:
   `packages/builder/src/registry/compiled/index.ts` — Add or update
   the `defineNode` / `defineMeta` entry with the new attribute spec
   (`coerce`, optional `dotNotation` / `objectShape`, doc string,
   `deprecated` hints) and any child element bindings. **This drives
   both codegen (XSD / JSON Schema / reference.md) and the runtime
   parser.**
   - For child elements that are *not* full POMNodes (Master objects,
     table Cell / Tr / Td, list Li, inline format tags, timeline /
     matrix / flow / tree items), add or update the entry in
     `packages/builder/src/parseXml/childAttributeSpecs.ts`.
3. **Node registry (runtime hooks)**:
   `packages/builder/src/registry/definitions/` — Add
   `applyYogaStyle` / `toPositioned` / `render` /
   `collectImageSources` for the new node, if any.
4. **Custom child conversion / post-processing** (rare): if the new
   node needs a child element converter beyond what
   `coerceChildAttrs` handles, add it to
   `packages/builder/src/parseXml/childConverters.ts`. Per-node
   post-processing (deprecation migrations, normalization) lives in
   `packages/builder/src/parseXml/dispatcher.ts`
   `applyPostProcessing`.
5. **Rendering**: Under `packages/builder/src/renderPptx/` —
   Implement pptxgenjs conversion.
6. **Regenerate schema artefacts**: Run `pnpm run codegen` (from
   `packages/builder/`) and commit the updated
   `dist-schema/builder.xsd`, `dist-schema/builder.schema.json`,
   `dist-schema/reference.md`, and `dist-schema/.codegen-hash.json`.
7. **VRT test data**: `packages/builder/vrt/decks/generatePptx.ts` —
   Add test cases for the new feature.
8. **Update VRT baseline**: Run `pnpm run vrt:docker:update` (from
   `packages/builder/`).
9. **Documentation updates**:
   - `packages/builder/README.md` — User-facing documentation.
   - `packages/builder/docs/xml-reference.md` — Hand-curated XML
     samples.
   - `packages/builder/docs/api.md` — Update if the public API
     surface changes.
   - The "Key internal types" section above — Add to it when a new
     node type is introduced.

> **Single-edit-per-attribute**: as of the registry-as-grammar
> refactor, adding an attribute is **one edit in `compiled/index.ts`**
> (driven by the Zod schema from `types.ts`); the runtime parser,
> codegen artefacts, and docs all derive from there. Hand-editing
> `parseXml.ts` for new attributes is not needed.

## Text measurement & unit conversion

Text width measurement uses `@slideglance/measure`'s WASM
`TextMeasurer` (a small ~300 KB measurement-only WASM split out of
`@slideglance/core`'s ~5 MB full pipeline). The bundled fonts (Noto
Sans JP, Pretendard — Regular + Bold) are registered with the
measurer once on first use and work in both Node.js and browser
environments. Bold variants are auto-detected by slideglance via
`OS/2.usWeightClass >= 600` and routed to `BufferFontResolver`'s
bold-variant slot, so the same family name (e.g. `"Pretendard"`)
resolves to the Regular or Bold face purely based on the `bold` flag.

Key files:

- `packages/builder/src/calcYogaLayout/measureText.ts` — Text
  measurement logic (wrap, line height, calculation).
- `packages/builder/src/calcYogaLayout/fontLoader.ts` — slideglance
  `TextMeasurer` facade + bundled-font registry.
- `packages/builder/src/calcYogaLayout/fonts/` — Bundled fonts
  (Base64-encoded TTF buffers).

The `textMeasurement` option in `buildPptx` selects the measurement
method:

- `"opentype"` — Always measure via slideglance (default).
- `"fallback"` — Always use fallback calculation (CJK characters
  = 1em, alphanumeric = 0.5em).
- `"auto"` — Measure via slideglance for bundled families, fallback
  otherwise.

### Unit conversion

- User input: pixels (`px`).
- Internal layout: pixels (yoga-layout).
- slideglance API: points (`pt`) — pom converts via
  `fontSizePt = fontSizePx * 72/96`.
- PPTX output: inches (converted via `pxToIn`, 96 DPI basis).

## Preview workflow (PPTX → PNG)

When modifying the sample deck to verify PPTX output, follow these
steps:

1. Edit `packages/builder/vrt/preview/sample.ts` (and modify logic
   under `packages/builder/src/` as needed).
2. Run `pnpm run preview:docker` from `packages/builder/`.
3. Visually verify
   `packages/builder/vrt/preview/output/sample.png` using the Read
   tool.
4. If there are layout issues, fix and return to step 2.
5. If everything looks good, commit.

### Output files

- `packages/builder/vrt/preview/output/sample.pptx` — Generated PPTX.
- `packages/builder/vrt/preview/output/sample.png` — PNG image (for
  layout verification).

## VS Code extension (`apps/vscode-extension/`)

VS Code extension for live preview of `.sgx` files. Mounts the
`@slideglance/viewer` React shell (`<PptxPresentation>`) inside a VS
Code webview; the extension itself ships no slide-rendering UI.

```bash
pnpm --filter @slideglance/vscode-extension run build           # full build (webview + host)
pnpm --filter @slideglance/vscode-extension run build:webview   # vite → dist/webview/
pnpm --filter @slideglance/vscode-extension run build:host      # esbuild → dist/extension.js
pnpm --filter @slideglance/vscode-extension run watch:host      # esbuild watch (host only)
pnpm --filter @slideglance/vscode-extension run lint            # ESLint
pnpm --filter @slideglance/vscode-extension run fmt             # Prettier formatting
pnpm --filter @slideglance/vscode-extension run fmt:check       # Format check
pnpm --filter @slideglance/vscode-extension run typecheck       # Type checking
```

### Pipeline

- `.sgx → @slideglance/builder buildPptx() → PPTX bytes →
  webview postMessage → @slideglance/viewer PptxPresentation →
  SVG paint`.
- Source-reveal: SVG element click → webview reads
  `data-object-name="node#N"` → host looks up the document's
  BuilderSourceMap → `vscode.window.showTextDocument(file, line)`.
- PPTX viewer (custom editor):
  `.pptx → vscode.workspace.fs.readFile → webview postMessage →
  PptxPresentation`. Registered with `priority: "option"` so users
  opt in via "Open With…".

### Bundle pipelines

Two bundle pipelines coexist because the slideglance worker resolves
`@slideglance/core`'s WASM via dynamic import + top-level await —
Vite's `vite-plugin-wasm` + `vite-plugin-top-level-await` handle it,
esbuild does not. Extension host (`src/extension.ts`,
`src/preview.ts`, `src/exportPptx.ts`, `src/pptxViewer.ts`,
`src/webviewHtml.ts`, `src/importResolver.ts`) is plain Node and
bundles fine through esbuild.

### Local testing

Open `apps/vscode-extension` in VS Code and press **F5** to launch
Extension Development Host.
