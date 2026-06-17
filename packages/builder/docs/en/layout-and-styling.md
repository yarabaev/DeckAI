---
title: "@slideglance/builder — Layout and Styling"
lang: en
kind: guides
package: builder
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/builder/src/calcYogaLayout/
  - packages/builder/src/registry/
---

# Layout & Styling

This document covers four interrelated topics: choosing a layout strategy, sizing and positioning rules, visual styling (colors, fonts, decoration), and master slides.

## Choosing a layout strategy

The builder has four coordinate systems; picking the right one prevents most layout pain.

| System                                                      | When to reach for it                                                                                          | Example                                            |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| `<VStack>` / `<HStack>` (Flexbox flow)                      | Default for any one-axis flow. Most slide content.                                                            | Title + body + footer column                       |
| `position="absolute"` + `top` / `right` / `bottom` / `left` | Single overlay anchored to a flow container's bounds.                                                         | Page number in the corner of a flow-laid `<Slide>` |
| `<Layer>` + child `x` / `y`                                 | Multiple overlapping elements with arbitrary positions. Diagrams, infographics, freely composed scenes.       | Connection lines between named boxes               |
| `<Line>` with `x1` / `y1` / `x2` / `y2`                     | A straight line between two specific points. Coordinates are slide-absolute (or parent-absolute when nested). | Annotation arrow                                   |

Within a single `<Layer>`, prefer `x` / `y` over `position="absolute"` — both place children absolutely, but `x` / `y` is canonical for `<Layer>`. `position="absolute"` is intended for one-off overlays inside flow containers.

## Flex containers

`<VStack>` and `<HStack>` map to `flex-direction: column` and `flex-direction: row`. They share these attributes:

| Attribute        | Type                                                                           | Notes                                 |
| ---------------- | ------------------------------------------------------------------------------ | ------------------------------------- |
| `gap`            | number                                                                         | Gap between children in px.           |
| `padding`        | shorthand / dot                                                                | Inner padding.                        |
| `alignItems`     | `start` / `center` / `end` / `stretch` / `baseline`                            | Cross-axis alignment.                 |
| `justifyContent` | `start` / `center` / `end` / `space-between` / `space-around` / `space-evenly` | Main-axis distribution.               |
| `flexWrap`       | `nowrap` / `wrap`                                                              | Wrap to next line when out of room.   |
| `alignContent`   | same as `alignItems`                                                           | Cross-axis distribution when wrapped. |

Children control their own sizing:

| Attribute                         | Notes                                                                                                                |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `w` / `h`                         | Number (px), `"50%"` (parent), `"max"` (fill remaining).                                                             |
| `minW` / `maxW` / `minH` / `maxH` | Hard size constraints.                                                                                               |
| `flexGrow`                        | Share of remaining space (default 0).                                                                                |
| `flexShrink`                      | Whether the child shrinks under pressure (default 1; pair with `noWrap` for text that must keep its measured width). |
| `flexBasis`                       | Initial main-axis size before grow / shrink.                                                                         |
| `alignSelf`                       | Override parent `alignItems` for this child.                                                                         |

```xml
<HStack padding="48" gap="24" alignItems="start" justifyContent="space-between">
  <VStack flexGrow="1" gap="8">
    <Text fontSize="32" bold="true">Left column</Text>
    <Text>Body text…</Text>
  </VStack>
  <VStack w="320" gap="8">
    <Text fontSize="20" bold="true">Sidebar</Text>
  </VStack>
</HStack>
```

## Absolute layout with `<Layer>`

`<Layer>` is a positioned coordinate space. Children specify `x`, `y`, `w`, `h` directly.

```xml
<Slide>
  <Layer w="1280" h="720">
    <Shape shapeType="rect"  x="0"   y="0"   w="1280" h="80"  fill.color="0F172A" />
    <Text  x="48"  y="24"  w="500" fontSize="28" bold="true" color="FFFFFF">Header</Text>

    <Shape shapeType="ellipse" x="100" y="220" w="160" h="160" fill.color="DBEAFE" />
    <Shape shapeType="ellipse" x="320" y="220" w="160" h="160" fill.color="FEF3C7" />
    <Line  x1="260" y1="300" x2="320" y2="300" lineWidth="2" color="334155" />
  </Layer>
</Slide>
```

Layers nest — a `<Layer>` inside another `<Layer>` defines its own coordinate space starting from `(0, 0)`.

> **Accessibility** — PowerPoint screen readers iterate shapes in **document source order**, not visual order. When using `<Layer>` for complex diagrams, place decorative background shapes first and informational content last so the reading order matches the visual flow. Mark purely decorative elements with `isDecorative="true"` to skip them entirely.

## Sizing rules

| Value     | Meaning                                                  |
| --------- | -------------------------------------------------------- |
| number    | Size in pixels (96 DPI; converted to inches internally). |
| `"50%"`   | Percentage of the parent's content box.                  |
| `"max"`   | Fill the remaining space along the main axis.            |
| (omitted) | The Flex layout sizes the child based on its content.    |

`"max"` is the equivalent of CSS `flex: 1 1 auto`. It only makes sense inside flex containers.

Size constraints apply after layout: a child that requested `w="100%"` but is bounded by `maxW="600"` clamps at 600.

## Auto-fit

When a slide's measured content exceeds the slide height, the builder applies shrink strategies in this order:

1. Reduce `<Tr>` / `<Td>` row heights.
2. Reduce text font sizes.
3. Reduce `gap` and `padding`.
4. Apply uniform scale (down to 0.5×; below that, content stays at its natural size and `AUTOFIT_OVERFLOW` is emitted).

```ts
await buildPptx(xml, { w: 1280, h: 720 }, { autoFit: false });
```

Disable when you want pixel-perfect control and prefer overflow over shrinking.

## Colors

All colors are **6-digit hex without `#`** (e.g. `FF0000`, `0F172A`).

```xml
<Text color="0F172A">Primary text</Text>
<Text color="64748B">Secondary text</Text>
<VStack backgroundColor="F8FAFC"><Text>Muted background</Text></VStack>
```

PPTX theme tokens (`accent1`, `dk1`, `lt1`, `hlink`, …) are not supported. The 6-digit form is portable across editors and locks output to the deterministic value the deck specifies.

A practical palette for business decks:

| Purpose  | Hex      |
| -------- | -------- |
| Title    | `0F172A` |
| Body     | `1F2937` |
| Muted    | `64748B` |
| Primary  | `1D4ED8` |
| Success  | `16A34A` |
| Warning  | `D97706` |
| Danger   | `DC2626` |
| Info     | `0EA5E9` |
| Light bg | `F8FAFC` |
| Border   | `CBD5E1` |

## Fonts

The package bundles two font families:

- **Pretendard** — Korean + Latin sans-serif (recommended default).
- **Noto Sans JP** — Japanese sans-serif.

Bundled fonts are used for **measurement** so layout matches what PowerPoint renders. To author with a non-bundled face:

```xml
<Document defaultTextStyle.fontFamily="Inter" />

<Slide>
  <Text fontFamily="Inter" fontSize="24">Body text in Inter</Text>
  <Text fontFamily="Pretendard">제목</Text>
</Slide>
```

When `fontFamily` is a non-bundled name, the builder switches to a heuristic measurer (CJK = 1em, alphanumeric = 0.5em) to avoid metric mismatch. PowerPoint resolves the actual font at render time on the recipient's machine, so install or embed your chosen family if it's not common.

See [Text measurement](./text-measurement.md) for the full mode breakdown and limitations.

## Decoration

### Borders

```xml
<VStack border.color="CBD5E1" border.width="1" border.dashType="dash">…</VStack>
```

Per-side borders:

```xml
<VStack
  border.top.color="CBD5E1" border.top.width="1"
  border.bottom.color="0F172A" border.bottom.width="2">…</VStack>
```

`dashType` accepts: `solid`, `dash`, `dashDot`, `lgDash`, `lgDashDot`, `lgDashDotDot`, `sysDash`, `sysDot`.

### Shadows

```xml
<Shape shapeType="roundRect" w="240" h="80" fill.color="FFFFFF"
       shadow.type="outer" shadow.blur="12" shadow.offset="4"
       shadow.color="0F172A" shadow.transparency="50">
  Card
</Shape>
```

`shadow.type`: `outer` (drop) or `inner`. `transparency` is 0–100.

### Background images

```xml
<VStack w="100%" h="max" backgroundImage.src="./assets/hero.jpg" backgroundImage.sizing="cover">
  <Text fontSize="56" color="FFFFFF">Hero title</Text>
</VStack>
```

`sizing`: `cover` (default) fills the area; `contain` fits within it.

### Opacity

```xml
<Shape shapeType="rect" w="1280" h="720" fill.color="000000" opacity="0.5" />
```

## Master slides

A `<Master>` is reusable backdrop chrome — header bars, footer text, page numbers, watermarks. It applies to every slide that references it.

### Inline master

```xml
<SlideGlance>
  <Document size="16:9" defaultMaster="CORP" />

  <Master name="CORP" backgroundColor="F8FAFC">
    <MasterRect x="0" y="0" w="1280" h="48" fill="0F172A" />
    <MasterText x="48"   y="14" w="400" h="24" text="ACME Corp"     color="FFFFFF" fontSize="14" />
    <MasterText x="1080" y="14" w="160" h="24" text="2026 / 03"     color="CBD5E1" fontSize="12" textAlign="right" />
    <SlideNumber x="1180" y="690" w="60" h="20" fontSize="10" color="64748B" format="{n} / {N}" />
  </Master>

  <Slide>
    <VStack padding="80" paddingTop="120">
      <Text fontSize="40" bold="true">Q4 Highlights</Text>
    </VStack>
  </Slide>
</SlideGlance>
```

`<Master>` children:

| Element         | Purpose                                              |
| --------------- | ---------------------------------------------------- |
| `<MasterRect>`  | Filled rectangle (header bars, sidebars, accents).   |
| `<MasterText>`  | Static text box.                                     |
| `<MasterImage>` | Logo or watermark.                                   |
| `<MasterLine>`  | Decorative line.                                     |
| `<SlideNumber>` | Auto page number. `format` supports `{n}` and `{N}`. |

`<Master backgroundColor>` and `<Master backgroundPath>` set the fill / image background. `<Master backgroundData>` accepts a `data:` URI.

### Multiple masters

Define several and pick per slide:

```xml
<Master name="LIGHT" backgroundColor="FFFFFF">…</Master>
<Master name="DARK"  backgroundColor="0F172A">…</Master>

<Slide master="LIGHT"><VStack><Text color="0F172A">Light slide</Text></VStack></Slide>
<Slide master="DARK"> <VStack><Text color="FFFFFF">Dark slide</Text></VStack></Slide>
```

### Master from existing PPTX

To reuse the background of a corporate template, point `masterPptx` at its bytes:

```ts
import { readFileSync } from "node:fs";

await buildPptx(
  xml,
  { w: 1280, h: 720 },
  {
    masterPptx: readFileSync("./template.pptx"),
  },
);
```

The builder extracts the first slide's background (image and/or color) and applies it as a default master. Failures emit `MASTER_PPTX_PARSE_FAILED` and the build proceeds without the extracted background.

> Cap the buffer size via `masterPptxLimits` (default 50 MB total, 5 MB per embedded image) when accepting templates from untrusted sources.

### Margins (content area)

```xml
<Master name="CORP" margin="48" margin.top="120">…</Master>
```

`margin` defines the slide's content area. Slide bodies render inside this inset, so `<VStack>` doesn't need to repeat the padding on every slide.

## Reusable styles

Define attribute presets once and apply them with `class="..."`.

```xml
<Styles>
  <Style name="page"   padding="48" backgroundColor="F8FAFC" />
  <Style name="title"  fontSize="40" bold="true" color="0F172A" />
  <Style name="muted"  fontSize="18" color="64748B" />
  <Style name="th"     fontSize="11" color="FFFFFF" bold="true" backgroundColor="0F172A" textAlign="center" />
</Styles>

<Slide>
  <VStack class="page" gap="12">
    <Text class="title">Q4 Highlights</Text>
    <Text class="muted">Three things that mattered.</Text>
  </VStack>
</Slide>
```

Multiple classes: `class="title primary"`. Later classes override earlier ones; per-node attributes override class values.

See [Composition](./composition.md) for `<Templates>`, `<Use>`, `<Slot>`, control-flow tags, and `<Import>`.
