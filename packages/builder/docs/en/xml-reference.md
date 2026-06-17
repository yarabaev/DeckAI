---
title: "@slideglance/builder — XML Reference"
lang: en
kind: reference
package: builder
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/builder/builder.xsd
  - packages/builder/builder.schema.json
  - packages/builder/src/registry/
---

# XML Reference

This document covers the visual nodes and the most common patterns. The complete attribute table for every element is auto-generated from the runtime schema and lives in [`reference.md`](./reference.md).

For composition tooling — `<Templates>`, `<Use>`, `<Slot>`, `<Styles>`, `<Import>`, control-flow tags — see [Composition](./composition.md). For master slides, layout, and styling, see [Layout & styling](./layout-and-styling.md).

## Document shape

```xml
<SlideGlance>
  <Document size="16:9" defaultMaster="CORP" defaultTextStyle.fontFamily="Pretendard" />

  <Master name="CORP" backgroundColor="F8FAFC">
    <!-- master objects -->
  </Master>

  <Styles>
    <Style name="title" fontSize="40" bold="true" />
  </Styles>

  <Slide>
    <VStack padding="48"><Text class="title">Hello</Text></VStack>
  </Slide>
</SlideGlance>
```

- `<SlideGlance>` is the document root.
- `<Document>` carries presentation-level settings (size, masters, default style). One per document.
- `<Slide>` accepts exactly one root child; that child is usually `<VStack>`, `<HStack>`, or `<Layer>`.
- Order of `<Master>`, `<Styles>`, `<Templates>`, `<Slide>` does not matter — `<Master>` and named-style declarations are collected in a single pass before slide rendering.

## Common attributes

Every visual node accepts these layout / decoration attributes.

| Attribute           | Type                                            | Notes                                                                                                                                                               |
| ------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `w`, `h`            | number / `"max"` / `"50%"`                      | Width / height. `"max"` = fill remaining; percentages = of parent.                                                                                                  |
| `minW`, `maxW`      | number                                          | Sizing constraints.                                                                                                                                                 |
| `minH`, `maxH`      | number                                          | Sizing constraints.                                                                                                                                                 |
| `x`, `y`            | number                                          | Absolute coordinates inside `<Layer>`.                                                                                                                              |
| `padding`, `margin` | number / shorthand / dot notation               | `padding="16"`, `padding="8 12"`, `padding.top="8"`.                                                                                                                |
| `border`            | object                                          | `border.color="333" border.width="1" border.dashType="dash"`.                                                                                                       |
| `borderRadius`      | number                                          | Corner radius in px.                                                                                                                                                |
| `backgroundColor`   | hex (no `#`)                                    | e.g. `F8FAFC`.                                                                                                                                                      |
| `backgroundImage`   | object                                          | `backgroundImage.src="url" backgroundImage.sizing="cover"`.                                                                                                         |
| `opacity`           | 0–1                                             | Element opacity.                                                                                                                                                    |
| `position`          | `"relative"` / `"absolute"`                     | Position mode.                                                                                                                                                      |
| `top`, `right`, …   | number                                          | Used with `position="absolute"`.                                                                                                                                    |
| `alignSelf`         | `auto` / `start` / `center` / `end` / `stretch` | Override parent `alignItems`.                                                                                                                                       |
| `zIndex`            | number                                          | Stacking order — higher = on top.                                                                                                                                   |
| `shadow`            | object                                          | `shadow.type="outer" shadow.blur="4" shadow.offset="2" shadow.color="000"`.                                                                                         |
| `class`             | string                                          | Space-separated reusable style names.                                                                                                                               |
| `master`            | string                                          | Slide-master name (only meaningful on `<Slide>` body root).                                                                                                         |
| `isDecorative`      | boolean                                         | Marks element decorative for accessibility (`altText=""`).                                                                                                          |
| `id`                | string (`[A-Za-z_][A-Za-z0-9_-]*`)              | Author-facing id; referenced by `<Connector from/to>`. Unique per slide.                                                                                            |
| `group`             | string                                          | Bundle this node and its descendants into a PowerPoint group (`<p:grpSp>`). Use `"true"` for an auto-named anonymous group or a stable id to merge subtrees / nest. |

Composite attributes accept three forms:

1. **Shorthand**: `padding="16"`, `padding="16 24"`, `padding="8 12 16 12"` (TRBL).
2. **Dot notation**: `padding.top="8" padding.bottom="16"`.
3. **Mixed**: `padding="16" padding.top="32"` — shorthand sets defaults, dot notation overrides per-key.

Properties supporting dot notation: `padding`, `margin`, `border`, `cellBorder`, `line`, `fill`, `shadow`, `underline`, `beginArrow`, `endArrow`, `backgroundImage`, `connectorStyle`, `sizing`.

> **Color format**: 6-digit hex without `#` (e.g. `FF0000`). PPTX theme tokens (`accent1`–`accent6`, `dk1`/`dk2`/`lt1`/`lt2`) are not supported.

## Visual nodes

### `<Text>`

```xml
<Text fontSize="24" bold="true" color="0F172A">Hello</Text>
```

Inline formatting tags: `<B>`, `<I>`, `<U>`, `<S>` (strikethrough), `<Mark>` (highlight), `<Span>` (style group), `<A href="...">` (hyperlink).

```xml
<Text fontSize="18">
  <B>Bold</B> and <I>italic</I> and <Mark color="FFE599">highlighted</Mark>
  with <A href="https://example.com">a link</A>.
</Text>
```

`<Span fontSize="14" color="999999">…</Span>` applies bundled text styling to a run.

`noWrap="true"` keeps text on a single line and pairs with `flexShrink="0"` to prevent the parent flex layout from squeezing it.

### `<Ul>` / `<Ol>` / `<Li>`

```xml
<Ul fontSize="18">
  <Li>First point</Li>
  <Li>Second point</Li>
  <Li bold="true">Third, emphasized</Li>
</Ul>

<Ol numberType="arabicPeriod" fontSize="16">
  <Li>Step one</Li>
  <Li>Step two</Li>
</Ol>
```

`<Ol numberType>` accepts the same enum as PowerPoint: `arabicPeriod`, `arabicParenR`, `romanUcPeriod`, `romanLcPeriod`, `alphaUcPeriod`, `alphaLcPeriod`, etc. See the schema reference for the full list.

Each `<Li>` accepts every text attribute (`bold`, `italic`, `color`, `lang`, …).

### `<Image>`

```xml
<Image src="https://example.com/photo.jpg" w="400" h="300" />
<Image src="./assets/logo.png" sizing.type="contain" w="200" />
<Image src="data:image/png;base64,..." w="120" />
```

`src` accepts URLs, file paths, and `data:` URIs. `sizing.type`:

- `contain` — fit within the box; aspect preserved.
- `cover` — fill the box; aspect preserved; overflow cropped.
- `crop` — explicit crop with `sizing.x` / `sizing.y` / `sizing.w` / `sizing.h` (0–1 ratios).

For untrusted input, gate `<Image src>` via `imageSrcGuard` — see [Security](./security.md).

### `<Icon>`

[Lucide](https://lucide.dev/) icon by name.

```xml
<Icon name="check-circle" w="32" color="16A34A" />
<Icon name="trending-up" w="48" color="1D4ED8" />
```

The full icon catalogue is bundled into the package; icon strokes render at the requested size and color.

### `<Svg>`

Inline SVG, rasterized to PNG at build time so the result embeds as a real PowerPoint picture (selectable, not lossy text).

```xml
<Svg w="120" h="120">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
    <circle cx="12" cy="12" r="10" fill="#1D4ED8" />
  </svg>
</Svg>
```

### `<Table>` (`<Col>`, `<Tr>`, `<Td>`)

```xml
<Table defaultRowHeight="36" cellBorder.color="CBD5E1" cellBorder.width="1">
  <Col w="80" />
  <Col w="200" />
  <Col w="120" />

  <Tr>
    <Td bold="true" backgroundColor="0F172A" color="FFFFFF">ID</Td>
    <Td bold="true" backgroundColor="0F172A" color="FFFFFF">Name</Td>
    <Td bold="true" backgroundColor="0F172A" color="FFFFFF">Status</Td>
  </Tr>
  <Tr>
    <Td>001</Td>
    <Td>Project Alpha</Td>
    <Td color="16A34A">Active</Td>
  </Tr>
  <Tr>
    <Td colspan="2">Spans two columns</Td>
    <Td rowspan="2">Spans two rows</Td>
  </Tr>
</Table>
```

`<Col w>` sizes columns; `<Tr>` is a row; `<Td>` is a cell. `colspan` and `rowspan` work as in HTML. Per-cell borders are set via `cellBorder.top`, `cellBorder.bottom`, `cellBorder.left`, `cellBorder.right`.

### `<Shape>`

PowerPoint preset shape with fill, line, and optional text. The `shapeType` attribute accepts every OOXML preset shape (178 of them) — `rect`, `roundRect`, `ellipse`, `triangle`, `arrow`, `cloud`, `star5`, etc.

```xml
<Shape shapeType="roundRect" w="240" h="80"
       fill.color="DBEAFE" line.color="1D4ED8" line.width="2"
       fontSize="18" color="0F172A" textAlign="center">
  Click here
</Shape>
```

The shape's text content is supplied as the element body (or `text="..."` attribute).

### `<Line>`

Straight line / arrow between two points.

```xml
<Line x1="100" y1="200" x2="500" y2="200"
      color="334155" lineWidth="2"
      endArrow.type="triangle" />
```

Coordinates are absolute within the parent container (slide-absolute when nested at the slide root).

### `<Connector>`

Smart line that binds to two shapes by their `id` and stays attached when the shapes move in PowerPoint. Compiles to a real PPTX `<p:cxnSp>` with `stCxn`/`endCxn` bindings, so dragging a shape in PowerPoint re-routes the connector automatically.

```xml
<HStack gap="80" padding="32">
  <Shape id="A" shapeType="rect" w="120" h="60" fill.color="DBEAFE"/>
  <Shape id="B" shapeType="rect" w="120" h="60" fill.color="DBEAFE"/>
  <Connector from="A" to="B" kind="elbow" endArrow="true"/>
</HStack>
```

| Attribute                        | Values                                  | Notes                                                                                                            |
| -------------------------------- | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `from`, `to`                     | author `id` of another node             | Required. The referenced ids must exist on the same slide.                                                       |
| `kind`                           | `straight` (default), `elbow`, `curved` | Picks `straightConnector1` / `bentConnectorN` / `curvedConnectorN`; the segment count `N` follows the side pair. |
| `fromSide`, `toSide`             | `top` \| `right` \| `bottom` \| `left`  | When omitted, the renderer auto-picks the dominant axis from the two shapes' bounding boxes.                     |
| `color`, `lineWidth`, `dashType` | same vocabulary as `<Line>`             | Visual style.                                                                                                    |
| `beginArrow`, `endArrow`         | `true` / `false` / `{ type }`           | `triangle` default when `true`.                                                                                  |

Add `id="..."` to any node you want a Connector to attach to. `id` is XML-friendly (`[A-Za-z_][A-Za-z0-9_-]*`) and must be unique within a slide.

The renderer drops connectors whose `from`/`to` cannot be resolved on the same slide and surfaces `UNKNOWN_CONNECTOR_ENDPOINT` (or `INVALID_CONNECTOR_SELF_REF` for `from === to`, `DUPLICATE_NODE_ID` for repeated ids) in the build diagnostics.

### `<Chart>`

Native PowerPoint charts — bar, line, pie, area, doughnut, radar.

```xml
<Chart chartType="bar" w="500" h="300" showTitle="true" title="Quarterly revenue" showLegend="true">
  <ChartSeries name="2025">
    <ChartDataPoint label="Q1" value="120" />
    <ChartDataPoint label="Q2" value="135" />
    <ChartDataPoint label="Q3" value="148" />
    <ChartDataPoint label="Q4" value="162" />
  </ChartSeries>
  <ChartSeries name="2024">
    <ChartDataPoint label="Q1" value="100" />
    <ChartDataPoint label="Q2" value="118" />
    <ChartDataPoint label="Q3" value="132" />
    <ChartDataPoint label="Q4" value="145" />
  </ChartSeries>
</Chart>
```

Charts compile to native PPTX chart parts — recipients can edit the underlying data in PowerPoint.

## Containers

### `<VStack>` — vertical Flex column

```xml
<VStack padding="48" gap="16" alignItems="start">
  <Text fontSize="40" bold="true">Title</Text>
  <Text fontSize="20">Body text.</Text>
</VStack>
```

### `<HStack>` — horizontal Flex row

```xml
<HStack padding="48" gap="24" alignItems="start" justifyContent="space-between">
  <VStack w="50%"><Text>Left</Text></VStack>
  <VStack w="50%"><Text>Right</Text></VStack>
</HStack>
```

Both stacks support every Flexbox attribute: `gap`, `padding`, `alignItems`, `justifyContent`, `flexWrap`, `flexShrink`, `flexGrow`. See [Layout & styling](./layout-and-styling.md) for the full mapping.

### `<Layer>` — absolute positioning

```xml
<Layer w="1280" h="720">
  <Shape shapeType="rect" x="0" y="0" w="1280" h="80" fill.color="0F172A" />
  <Text x="48" y="24" w="600" fontSize="32" bold="true" color="FFFFFF">Header</Text>
  <Shape shapeType="ellipse" x="100" y="220" w="160" h="160" fill.color="DBEAFE" />
  <Shape shapeType="ellipse" x="320" y="220" w="160" h="160" fill.color="FEF3C7" />
  <Line x1="260" y1="300" x2="320" y2="300" lineWidth="2" color="334155" />
</Layer>
```

`<Layer>` accepts children with absolute `x` / `y`. Use it for diagrams, infographics, and freely-composed scenes. Children may also omit `x` / `y` — they then anchor at `(0, 0)`.

> Reading order in PowerPoint follows **document source order**, not visual position. Place decorative shapes first and informational content last when using `<Layer>` so screen readers iterate sensibly.

## Speaker notes

```xml
<SlideGlance>
  <Document size="16:9" />
  <Slide>
    <VStack><Text>Slide content</Text></VStack>
    <Notes>Speaker notes for this slide.</Notes>
  </Slide>
</SlideGlance>
```

`<Notes>` accepts plain text or `<Text>` runs.

## Editorial idioms

### Mixed-size text rows

A `<HStack>` row of two or more `<Text>` elements with different
`fontSize`s — say a big label next to a small caption — needs both of
the following to look optically aligned:

1. **`textVAlign="middle"` on every sibling** — PPTX text frames anchor
   their glyphs to the top of the frame by default. After yoga stretches
   the smaller sibling's box to match the larger one's height, the
   smaller text floats at the top of the equalized row unless its anchor
   is centered.
2. **`lineHeight="1.0"` on the larger sibling** — paragraph styles like
   `body-lead` often carry a generous `lineHeight` (1.4–1.5) for
   multi-line readability. In a single-line row that extra leading
   inflates the measured box height beyond the glyph extent: the larger
   sibling's "specified" box no longer matches what the visual line
   actually needs, the row stretches taller than the glyphs, and the
   surrounding baseline reads as offset. Override `lineHeight` to a
   tight value (`1.0` is right for single-line labels) so the measured
   height equals the glyph height.

```xml
<HStack gap="10" alignItems="baseline">
  <Text fontFamily="Georgia" fontSize="13" bold="true" lineHeight="1.0" textVAlign="middle">Mara Olsen</Text>
  <Text fontFamily="Inter"   fontSize="9"  bold="true" lineHeight="1.0" textVAlign="middle">Lead writer</Text>
</HStack>
```

`textVAlign` accepts `top` (default), `middle`, `bottom`. It applies to
`<Text>`, `<Ul>`, and `<Ol>`.

### Lead-in (canonical drop-cap substitute)

True magazine drop-cap — a large initial letter the first 2-3 lines of body
text wrap around — requires text-flow-around-shape, which is supported by
neither `pptxgenjs` nor the builder's yoga flex layout. The closest editorial
idiom available is a **lead-in**: an oversized, accent-colored first
sentence that introduces the paragraph, with the body running below at
normal weight.

```xml
<VStack gap="10">
  <Text fontFamily="Georgia" fontSize="22" bold="true" color="9A2A1F" lineHeight="1.2">
    The most photographed moment of a release is the launch day.
  </Text>
  <Text fontFamily="Georgia" fontSize="13" color="3A332B" lineHeight="1.5">
    The work that actually defines whether the release was good — the year
    that follows — gets almost no coverage.
  </Text>
</VStack>
```

This composition is featured in the
[editorial sample deck](../../../../examples/playground-samples/editorial/)
as the recommended opening for long-form articles.

## What's not in the builder

- **True drop-caps** — see the "Lead-in" idiom above.
- **Slide transitions** and **slide-level animations** — the builder targets static visual layout. Add transitions after export if needed.
- **PPTX theme tokens** (`accent1`, `dk1`, …) — colors are 6-digit hex only.
- **Right-to-left script-specific shaping** — text rendering is what pptxgenjs + PowerPoint provide; the builder does not customize bidi behavior.

For diagrams (timelines, trees, flowcharts, matrices, pyramids, process arrows), compose them from `<Layer>` + `<Shape>` + `<Line>` + `<Text>`. The [reference deck](../../../../examples/builder-reference/) shows worked recipes for each.
