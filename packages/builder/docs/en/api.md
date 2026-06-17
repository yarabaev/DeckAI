---
title: "@slideglance/builder — API Reference (extended)"
lang: en
kind: reference
package: builder
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/builder/src/index.ts
  - packages/builder/src/buildPptx.ts
---

# API Reference

`@slideglance/builder` exposes a small public surface centered on `buildPptx` plus a parse-only entry point and a handful of error / type re-exports.

## `buildPptx`

Compile an XML string into a PowerPoint deck.

```ts
async function buildPptx(
  xml: string,
  slideSize: { w: number; h: number },
  options?: BuildPptxOptions,
): Promise<BuildPptxResult>;
```

### Parameters

#### `xml` (required)

A `<SlideGlance>`-rooted XML string. For ad-hoc one-shot prototyping, the builder also accepts a serialized form where each top-level element becomes one slide; for any non-trivial deck the `<SlideGlance>` form unlocks slide size, masters, styles, templates, and `<Import>`.

```xml
<SlideGlance>
  <Document size="16:9" />
  <Slide>
    <VStack padding="48"><Text>Hello</Text></VStack>
  </Slide>
</SlideGlance>
```

When processing untrusted XML, validate or restrict the following attribute values before passing them in — they read files, fetch URLs, or end up in the PPTX as hyperlinks: `<Image src>`, `<Master backgroundPath>`, `<A href>`, `<Import src>`. See [Security](./security.md).

#### `slideSize` (required)

Slide dimensions in pixels at 96 DPI. Common values:

| Aspect ratio | `{ w, h }`            |
| ------------ | --------------------- |
| 16:9         | `{ w: 1280, h: 720 }` |
| 4:3          | `{ w: 960, h: 720 }`  |
| 16:10        | `{ w: 1280, h: 800 }` |

#### `options` (optional)

| Option               | Type                                      | Default      | Notes                                                                                                                                                                                                                                                                              |
| -------------------- | ----------------------------------------- | ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `master`             | `SlideMasterOptions`                      | `undefined`  | Single inline master.                                                                                                                                                                                                                                                              |
| `masters`            | `SlideMasterOptions[]`                    | `undefined`  | Multiple named masters. Each must have a unique `title`.                                                                                                                                                                                                                           |
| `defaultMaster`      | `string`                                  | `undefined`  | Master name applied to slides that don't declare one. `<Document defaultMaster="...">` overrides this.                                                                                                                                                                             |
| `masterPptx`         | `ArrayBuffer \| Uint8Array`               | `undefined`  | Existing PPTX whose first slide background is reused.                                                                                                                                                                                                                              |
| `masterPptxLimits`   | `MasterPptxLimits`                        | 50 MB / 5 MB | Size cap for `masterPptx`. Oversized buffers emit `MASTER_PPTX_SIZE_LIMIT`.                                                                                                                                                                                                        |
| `textMeasurement`    | `"opentype" \| "fallback" \| "auto"`      | `"auto"`     | See [Text measurement](./text-measurement.md).                                                                                                                                                                                                                                     |
| `fonts`              | `Uint8Array[]`                            | `undefined`  | Extra TTF/OTF buffers (Regular + Bold) registered with the layout-time text measurer alongside the bundled fonts. Pair with the viewer's `fontStylesheet` so layout-time wrap matches render-time wrap. See [Text measurement](./text-measurement.md#builder--renderer-alignment). |
| `defaultTextStyle`   | `DefaultTextStyle`                        | `undefined`  | Default `fontSize` / `fontFamily` / `color` / `lineHeight` for all text nodes.                                                                                                                                                                                                     |
| `defaultLang`        | `string` (BCP 47)                         | `undefined`  | Fallback `lang` for `<Text>` runs without an explicit `lang`.                                                                                                                                                                                                                      |
| `autoFit`            | `boolean`                                 | `true`       | Auto-shrink content when slides overflow.                                                                                                                                                                                                                                          |
| `strict`             | `boolean`                                 | `false`      | Throw `DiagnosticsError` if any diagnostics are emitted.                                                                                                                                                                                                                           |
| `resolveImport`      | `ImportResolver`                          | `undefined`  | Required when XML uses `<Import>`. Synchronous loader.                                                                                                                                                                                                                             |
| `sourcePath`         | `string`                                  | `undefined`  | Absolute path of the root document — used as `fromPath` for the first `resolveImport` call.                                                                                                                                                                                        |
| `trackSourcePos`     | `boolean`                                 | `false`      | Attach `__nodeId` + `sourceMap` to the result for editor integrations.                                                                                                                                                                                                             |
| `docProps`           | `{ title?, author?, company?, subject? }` | `undefined`  | Written to `docProps/core.xml`.                                                                                                                                                                                                                                                    |
| `allowedHrefSchemes` | `string[]`                                | `undefined`  | Extends the default `<A href>` allowlist (`https:`, `http:`, `mailto:`, `tel:`).                                                                                                                                                                                                   |
| `imageSrcGuard`      | `ImageSrcGuardOptions`                    | `undefined`  | Validates `<Image src>` and `<Master backgroundPath>` against an allowlist.                                                                                                                                                                                                        |
| `maxTemplateNodes`   | `number`                                  | `100000`     | Hard cap on `<Use>` expansion output.                                                                                                                                                                                                                                              |

### Return value

```ts
interface BuildPptxResult {
  pptx: PptxGenJS;
  diagnostics: Diagnostic[];
  sourceMap?: BuilderSourceMap; // present iff trackSourcePos: true
}
```

The `pptx` instance supports the full pptxgenjs save / write API:

```ts
await pptx.writeFile({ fileName: "deck.pptx" }); // Node fs
const buffer = await pptx.write({ outputType: "nodebuffer" });
const stream = await pptx.stream(); // for HTTP responses
```

### Errors

| Error              | When                                                                       |
| ------------------ | -------------------------------------------------------------------------- |
| `ParseXmlError`    | XML is malformed, references unknown tags / attributes, or fails coercion. |
| `DiagnosticsError` | `strict: true` and at least one diagnostic was emitted.                    |

```ts
import {
  buildPptx,
  ParseXmlError,
  DiagnosticsError,
} from "@slideglance/builder";

try {
  const { pptx } = await buildPptx(xml, { w: 1280, h: 720 }, { strict: true });
} catch (e) {
  if (e instanceof ParseXmlError) console.error("Invalid XML:", e.message);
  if (e instanceof DiagnosticsError)
    console.error("Diagnostics:", e.diagnostics);
}
```

> Diagnostic messages may include user-supplied attribute values verbatim (e.g. `Cannot convert "secret-token" to number`). Mask sensitive substrings before persisting or forwarding diagnostics from a multi-tenant server.

## Diagnostic codes

Each `Diagnostic` carries a stable `code` for branching without parsing `message`. Optional `sourcePos` carries `{ file, line }` when the parser can attribute the offending element.

| Code                       | Severity | Triggered by                                                                                  |
| -------------------------- | -------- | --------------------------------------------------------------------------------------------- |
| `IMAGE_MEASURE_FAILED`     | warning  | An `<Image>` could not be measured during prefetch; layout falls back to declared `w` / `h`.  |
| `IMAGE_NOT_PREFETCHED`     | warning  | An `<Image>` reached layout without measurement.                                              |
| `AUTOFIT_OVERFLOW`         | warning  | Auto-fit ran every shrink strategy and content still overflowed.                              |
| `SCALE_BELOW_THRESHOLD`    | warning  | Auto-fit's uniform-scale step would scale below 0.5×; content left at its overflowing size.   |
| `MASTER_PPTX_PARSE_FAILED` | warning  | The `masterPptx` buffer could not be parsed. Build proceeds without the extracted background. |
| `MASTER_PPTX_SIZE_LIMIT`   | warning  | `masterPptx` (or one of its embedded images) exceeds `masterPptxLimits`.                      |
| `INVALID_HREF_SCHEME`      | warning  | `<A href>` scheme outside the allowlist. The hyperlink is dropped; text is preserved.         |
| `INVALID_IMAGE_SRC`        | warning  | `imageSrcGuard` rejected `<Image src>` or `<Master backgroundPath>`. The image is dropped.    |
| `TEMPLATE_EXPANSION_LIMIT` | warning  | `<Use>` expansion produced more than `maxTemplateNodes` nodes. Surplus expansion aborted.     |
| `TEMPLATES_NOT_AT_ROOT`    | warning  | `<Templates>` block nested inside `<Slide>` / a container. Block ignored.                     |
| `INVALID_NUMBER_TYPE`      | warning  | `<Ol numberType="...">` value outside the supported enum. Attribute stripped.                 |

## `parseBuilderDocument`

Parse a `.sgx` string into a fully-materialized BuilderNode tree with presentation-level metadata, **without** building a PPTX. Intended for tools that analyze or transform documents (e.g. the VS Code preview extension).

```ts
function parseBuilderDocument(
  xml: string,
  options?: ParseBuilderDocumentOptions,
): ParseResult;

interface ParseResult {
  document: ParsedBuilderDocument;
  diagnostics: Diagnostic[];
}

interface ParsedBuilderDocument {
  nodes: BuilderNode[];
  slideSize?: { w: number; h: number };
  masters?: SlideMasterOptions[];
  masterContents?: Record<string, BuilderNode[]>;
  defaultMaster?: string;
  defaultTextStyle?: DefaultTextStyle;
  sourceMap?: BuilderSourceMap;
}
```

`<Import>` resolution and `<Templates>` expansion both run during `parseBuilderDocument`, so `nodes[i]` is the fully-materialized tree for slide `i`.

## Source position tracking

With `trackSourcePos: true`, every BuilderNode is tagged with an internal `__nodeId` and the result carries a `sourceMap: Map<number, { file?, line }>`. Every rendered pptxgenjs object also gets `objectName="node#N"` so downstream tools can recover the origin from a saved `.pptx`.

```ts
const { pptx, sourceMap } = await buildPptx(
  xml,
  { w: 1280, h: 720 },
  { trackSourcePos: true, resolveImport, sourcePath },
);

for (const [id, pos] of sourceMap!) {
  console.log(`node #${id} originated at ${pos.file ?? "(root)"}:${pos.line}`);
}
```

This is what powers the [VS Code extension](./vscode-extension.md)'s click-to-source feature.

## Auto-fit

When content exceeds slide height, the builder applies shrink strategies in order:

1. Reduce table row heights.
2. Reduce text font sizes.
3. Reduce gap / padding.
4. Uniform scale (fallback, never below 0.5×).

If every strategy fails, `AUTOFIT_OVERFLOW` is emitted and the slide renders with overflow.

```ts
await buildPptx(xml, { w: 1280, h: 720 }, { autoFit: false });
```

## Image source guard

Opt-in validation for `<Image src>` and `<Master backgroundPath>`.

```ts
type ImageSrcGuardOptions = {
  /** URL schemes allowed for <Image src> and <Master backgroundPath>. */
  allowSchemes?: string[];
  /**
   * file:// and relative paths must resolve under this directory.
   * Paths outside the base dir emit INVALID_IMAGE_SRC and are dropped.
   */
  allowBaseDir?: string;
};

await buildPptx(
  xml,
  { w: 1280, h: 720 },
  {
    imageSrcGuard: {
      allowSchemes: ["https:", "data:"],
      allowBaseDir: path.resolve("./assets"),
    },
  },
);
```

When omitted, no validation runs. See [Security](./security.md) for guidance.

## Master PPTX limits

Caps for the `masterPptx` buffer and its embedded images.

```ts
type MasterPptxLimits = {
  maxBytes?: number; // default: 50 MB
  maxImageBytes?: number; // default: 5 MB
};
```

Buffers exceeding the cap emit `MASTER_PPTX_SIZE_LIMIT` and are rejected; the build proceeds without the extracted background.

## Exported types

```ts
import type {
  BuildPptxResult,
  BuildPptxOptions,
  TextMeasurementMode,
  ImageSrcGuardOptions,
  MasterPptxLimits,
  Diagnostic,
  DiagnosticCode,
  ParseResult,
  ParsedBuilderDocument,
  ParseBuilderDocumentOptions,
  ImportResolver,
  BuilderSourceMap,
  BuilderSourcePos,
  DefaultTextStyle,
  SlideMasterOptions,
  SlideMasterBackground,
  SlideMasterMargin,
  MasterObject,
  MasterTextObject,
  MasterImageObject,
  MasterRectObject,
  MasterLineObject,
  SlideNumberOptions,
  // BuilderNode unions
  BuilderNode,
  PositionedNode,
  PositionedLayerChild,
  // Per-node types
  TextNode,
  UlNode,
  OlNode,
  LiNode,
  ImageNode,
  IconNode,
  SvgNode,
  TableNode,
  ShapeNode,
  ChartNode,
  LineNode,
  LineArrow,
  VStackNode,
  HStackNode,
  LayerNode,
  // Style atoms
  Length,
  Padding,
  BorderDash,
  BorderStyle,
  FillStyle,
  ShadowStyle,
  Underline,
  UnderlineStyle,
  AlignItems,
  AlignSelf,
  PositionType,
  FlexWrap,
  JustifyContent,
  BulletNumberType,
  ShapeType,
  BackgroundImage,
  BackgroundImageSizing,
} from "@slideglance/builder";
```

For the full per-attribute table per element, see [Schema reference](../../reference.md) — auto-generated from the runtime registry.
