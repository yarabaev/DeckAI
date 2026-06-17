# Grammar reference

The slideglance grammar (`.sgx`) is a small XML dialect compiled to
editable `.pptx` by `@slideglance/builder`. This page lists the visual
nodes and the most common attributes in one place. The authoritative
attribute table is auto-generated at
`packages/builder/reference.md` (or
`https://unpkg.com/@slideglance/builder@^1/reference.md` once
published). When this page and that file disagree, that file wins —
and please log the drift in `schema-gotchas.md`.

## Document shape

```xml
<?xml version="1.0" encoding="UTF-8"?>
<SlideGlance xmlns="urn:slideglance:builder:v1">
  <Document size="16:9" defaultMaster="CORP" fontFamily="Pretendard" />
  <Master name="CORP" backgroundColor="F8FAFC">…</Master>
  <Styles>…</Styles>
  <Templates>…</Templates>
  <Slide>…</Slide>
</SlideGlance>
```

- `<SlideGlance>` — document root.
- `<Document>` — presentation-level settings (size, default master, default font). One per document.
- `<Slide>` — accepts **exactly one** root child (usually `<VStack>`, `<HStack>`, or `<Layer>`).
- Order of `<Master>`, `<Styles>`, `<Templates>`, `<Slide>` does not matter — declarations are collected in a single pass before slide rendering.

### Slide size on `<Document>`

`size="16:9"` is the default. Other named presets and the explicit
`w` / `h` form:

| Aspect                           | Named preset       | `w`  | `h`  |
| -------------------------------- | ------------------ | ---- | ---- |
| 16:9 (modern presentations)      | `size="16:9"`      | 1280 | 720  |
| 4:3 (legacy / projector)         | `size="4:3"`       | 1024 | 768  |
| A4 portrait (printable handout)  | `size="A4"`        | 794  | 1123 |
| Letter portrait                  | `size="Letter"`    | 816  | 1056 |
| 9:16 (vertical / social)         | `size="9:16"`      | 720  | 1280 |
| 3:4 (xhs / 小红书 vertical card) | (explicit w/h)     | 960  | 1280 |

For a custom aspect, use the explicit form:

```xml
<Document w="960" h="1280" fontFamily="Pretendard" />
```

Don't mix `size="custom"` with `w` / `h` — pick one form, not both.

## Common attributes (every visual node)

| Attribute                        | Type                                            | Notes                                                                                                                                                                                                          |
| -------------------------------- | ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `w`, `h`                         | number / `"max"` / `"50%"`                      | Width / height. `"max"` = fill remaining main axis; percentages are of parent.                                                                                                                                 |
| `minW`, `maxW`, `minH`, `maxH`   | number                                          | Hard size constraints.                                                                                                                                                                                         |
| `x`, `y`                         | number                                          | Absolute coordinates inside `<Layer>`.                                                                                                                                                                         |
| `padding`, `margin`              | shorthand / dot                                 | `padding="16"` (uniform), `padding="10 0"` (TB/LR), `padding="8 12 16 12"` (TRBL), or dot notation `padding.top="8"`. Mixing shorthand with dot also works: `padding="80" padding.top="120"`.                  |
| `border`                         | object                                          | `border.color="333" border.width="1" border.dashType="dash"`. For per-side variations use `borderTop` / `borderRight` / `borderBottom` / `borderLeft` (same shape; composes additively with uniform `border`). |
| `borderRadius`                   | number                                          | Corner radius in px.                                                                                                                                                                                           |
| `backgroundColor`                | hex (no `#`)                                    | e.g. `F8FAFC`.                                                                                                                                                                                                 |
| `backgroundImage`                | object                                          | `backgroundImage.src="url" backgroundImage.sizing="cover"`.                                                                                                                                                    |
| `opacity`                        | 0–1                                             | Element opacity.                                                                                                                                                                                               |
| `position`                       | `relative` / `absolute`                         | Position mode.                                                                                                                                                                                                 |
| `top`, `right`, `bottom`, `left` | number                                          | With `position="absolute"`.                                                                                                                                                                                    |
| `alignSelf`                      | `auto` / `start` / `center` / `end` / `stretch` | Override parent `alignItems`.                                                                                                                                                                                  |
| `zIndex`                         | number                                          | Stacking order. Higher = on top.                                                                                                                                                                               |
| `shadow`                         | object                                          | `shadow.type="outer" shadow.blur="4" shadow.offset="2" shadow.color="000"`.                                                                                                                                    |
| `class`                          | string                                          | Space-separated reusable style names.                                                                                                                                                                          |
| `master`                         | string                                          | Slide-master name (only meaningful on `<Slide>`).                                                                                                                                                              |
| `isDecorative`                   | boolean                                         | Marks element decorative for accessibility (`altText=""`).                                                                                                                                                     |

Composite attributes accept two forms — pick one per attribute:

1. **Shorthand single value**: `padding="16"`.
2. **Dot notation**: `padding.top="8" padding.bottom="16"`.

Properties supporting dot notation: `padding`, `margin`, `border`,
`cellBorder`, `line`, `fill`, `shadow`, `underline`, `beginArrow`,
`endArrow`, `backgroundImage`, `connectorStyle`, `sizing`.

> **Color format**: 6-digit hex without `#` (e.g. `FF0000`). PPTX theme tokens (`accent1`–`accent6`, `dk1`/`dk2`/`lt1`/`lt2`) are intentionally not supported.

## Visual nodes

### `<Text>`

```xml
<Text fontSize="24" bold="true" color="0F172A">Hello</Text>
```

Inline tags: `<B>`, `<I>`, `<U>`, `<S>` (strike), `<Mark>` (highlight),
`<Span>` (style group), `<A href="…">` (hyperlink).

Text-specific attributes:

| Attribute        | Notes                                                                                                                                                                                                                                          |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `fontFamily`     | Family name. Bundled: `Pretendard`, `Noto Sans JP`. Others use heuristic measurement.                                                                                                                                                          |
| `fontSize`       | pt — `24` not `24pt`.                                                                                                                                                                                                                          |
| `bold`, `italic` | boolean.                                                                                                                                                                                                                                       |
| `color`          | 6-digit hex.                                                                                                                                                                                                                                   |
| `textAlign`      | `left` / `center` / `right`.                                                                                                                                                                                                                   |
| `textVAlign`     | `top` / `middle` / `bottom`.                                                                                                                                                                                                                   |
| `lineHeight`     | unitless multiplier — `1.0` for tight single-line rows, `1.4–1.5` for paragraph body.                                                                                                                                                          |
| `letterSpacing`  | em-unit tracking — `-0.02` for tight display, `0.18` for small-caps eyebrows. Maps to pptxgenjs `charSpacing` (1/100 em). The WASM measurer ignores tracking, so autofit may underestimate wrap width slightly when absolute values are large. |
| `noWrap`         | boolean — keep text on a single line.                                                                                                                                                                                                          |
| `lang`           | BCP-47 tag for language tagging (a11y / font fallback).                                                                                                                                                                                        |

> `letterSpacing` is **not** in the schema. See [`schema-gotchas.md`](./schema-gotchas.md).

### `<Ul>`, `<Ol>`, `<Li>`

```xml
<Ul fontSize="18">
  <Li>First point</Li>
  <Li bold="true">Second, emphasized</Li>
</Ul>

<Ol numberType="arabicPeriod">
  <Li>Step one</Li>
  <Li>Step two</Li>
</Ol>
```

`<Ol numberType>` accepts the same enum as PowerPoint: `arabicPeriod`,
`arabicParenR`, `romanUcPeriod`, `romanLcPeriod`, `alphaUcPeriod`,
`alphaLcPeriod`, …

### `<Image>`

```xml
<Image src="https://example.com/photo.jpg" w="400" h="300" />
<Image src="./assets/logo.png" sizing.type="contain" w="200" />
<Image src="data:image/png;base64,…" w="120" />
```

Image-specific attributes:

| Attribute      | Notes                                                                                         |
| -------------- | --------------------------------------------------------------------------------------------- |
| `src`          | URL · `data:` URI · relative path. Required.                                                  |
| `sizing.type`  | `contain` / `cover` / `crop` — see modes below.                                               |
| `altText`      | Accessibility text. Set on every informational image.                                         |
| `isDecorative` | `true` to mark as decorative (empty alt text). Use for ambient backgrounds.                   |
| `rotate`       | Rotation in degrees.                                                                          |
| `opacity`      | 0..1 — inherited from `BASE_ATTRS` so the same key applies to every node, not just `<Image>`. |
| `borderRadius` | Px corner radius — clips the bitmap into a rounded rectangle.                                 |

`sizing.type`:

- `contain` — fit within the box; aspect preserved; transparent letterbox if needed.
- `cover` — fill the box; aspect preserved; overflow cropped.
- `crop` — explicit crop with `sizing.x` / `sizing.y` / `sizing.w` / `sizing.h` (0–1 ratios into the source bitmap).

For untrusted input, gate `<Image src>` via the `imageSrcGuard` build
option.

### `<Icon>`

[Lucide](https://lucide.dev/) icon by name.

```xml
<Icon name="check-circle" w="32" color="16A34A" />
<Icon name="trending-up" w="48" color="1D4ED8" />
<Icon name="zap" size="40" variant="circle-filled"
      color="FFFFFF" backgroundColor="1D4ED8" />
```

Icon-specific attributes:

| Attribute         | Notes                                                                                                 |
| ----------------- | ----------------------------------------------------------------------------------------------------- |
| `name`            | Lucide icon name. Required.                                                                           |
| `size`            | Square pixel size — drives both `w` and `h` for the glyph.                                            |
| `color`           | Foreground (glyph) colour.                                                                            |
| `backgroundColor` | Surface colour (only meaningful when `variant` is one of the chip variants).                          |
| `variant`         | `circle-filled` / `circle-outlined` / `square-filled` / `square-outlined`. Wraps the glyph in a chip. |
| `altText`         | Accessibility text.                                                                                   |

> **Chip inflation.** `variant="circle-filled"` (and the other chip
> variants) inflates the outer box to about `size × 1.75` to make
> room for the chip padding. Pick `size` accordingly when a card
> needs a stable height — see
> [`recipes.md`](./recipes.md) §7 gotcha.

### `<Svg>`

Inline SVG, rasterized to PNG at build time.

```xml
<Svg w="120" h="120">
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
    <circle cx="12" cy="12" r="10" fill="#1D4ED8" />
  </svg>
</Svg>
```

`<Svg>` width is capped at 1024 by the schema.

### `<Table>` / `<Col>` / `<Tr>` / `<Td>`

```xml
<Table defaultRowHeight="36" cellBorder.color="CBD5E1" cellBorder.width="1">
  <Col w="80" />
  <Col w="200" />
  <Tr>
    <Td bold="true" backgroundColor="0F172A" color="FFFFFF">ID</Td>
    <Td bold="true" backgroundColor="0F172A" color="FFFFFF">Name</Td>
  </Tr>
  <Tr>
    <Td>001</Td>
    <Td>Project Alpha</Td>
  </Tr>
</Table>
```

`colspan` and `rowspan` work as in HTML. Per-side cell borders via
`cellBorder.top` / `cellBorder.bottom` / `cellBorder.left` /
`cellBorder.right`. `<Td>` accepts both `padding` and `margin` as
aliases for the cell's inner spacing (PPTX table cells have no
outer-spacing concept). See
`packages/builder/src/parseXml/childAttributeSpecs.ts` for the
authoritative `<Td>` attribute list.

**JSON form** — `<Col>` / `<Tr>` / `<Td>` element notation can be
replaced wholesale by `columns="…"` / `rows="…"` JSON arrays on
`<Table>` when the table is data-driven:

```xml
<Table defaultRowHeight="36" cellBorder.color="CBD5E1"
       columns='[{"w":160},{"w":120},{"w":120}]'
       rows='[
         {"cells":[{"text":"Cohort","bold":true},{"text":"Orgs"},{"text":"P95"}]},
         {"cells":[{"text":"Internal"},{"text":"1"},{"text":"410ms"}]}
       ]' />
```

Use the element form for hand-authored tables with rich styling and
the JSON form when the data lives in a `<Foreach>` loop or comes
from an upstream source.

### `<Shape>`

PowerPoint preset shape with fill, line, and optional text. The
`shapeType` accepts every OOXML preset (178 shapes): `rect`,
`roundRect`, `ellipse`, `triangle`, `arrow`, `cloud`, `star5`, … See
`packages/builder/reference.md` for the exhaustive list.

```xml
<Shape shapeType="roundRect" w="240" h="80"
       fill.color="DBEAFE" line.color="1D4ED8" line.width="2"
       fontSize="18" color="0F172A" textAlign="center">
  Click here
</Shape>
```

Shape-specific attributes:

| Attribute                  | Notes                                                                                                  |
| -------------------------- | ------------------------------------------------------------------------------------------------------ |
| `shapeType`                | OOXML preset name (`rect`, `roundRect`, `ellipse`, `diamond`, `arrow`, `cloud`, `star5`, …). Required. |
| `fill`                     | `fill.color` + optional `fill.transparency` (0..1).                                                    |
| `line`                     | `line.color` + `line.width` + `line.dashType` — solid border styling.                                  |
| `text`                     | Optional body text (alternative to element body content).                                              |
| `textAlign` / `textVAlign` | In-shape text alignment.                                                                               |
| `rotate`                   | Rotation in degrees.                                                                                   |

**`dashType` values** (used by `line.dashType`, `cellBorder.dashType`,
and `<Line>` `dashType`): `solid`, `dash`, `dashDot`, `lgDash`,
`lgDashDot`, `lgDashDotDot`, `sysDash`, `sysDot`. PowerPoint preset
dash patterns — matches OOXML's `<a:prstDash>` enumeration.

> `<Shape>` does **not accept child elements**. Body text only.

### `<Line>`

Straight line / arrow between two points. Coordinates are absolute
within the parent (slide-absolute when nested at slide root).

```xml
<Line x1="100" y1="200" x2="500" y2="200"
      color="334155" lineWidth="2"
      endArrow.type="triangle" />
<Line x1="0" y1="0" x2="200" y2="200"
      color="6B7280" dashType="dash"
      beginArrow.type="oval" endArrow.type="stealth" />
```

`beginArrow` / `endArrow` accept either a boolean (`true` for a
default arrow) or an object `{type: …}` where `type` is one of:
`none`, `arrow`, `triangle`, `diamond`, `oval`, `stealth`.

`dashType` accepts the same enumeration as `<Shape>` `line.dashType`
(see above).

> `<MasterLine>` does **not** use endpoint pairs — see
> [`schema-gotchas.md`](./schema-gotchas.md).

### `<Chart>`

Native PowerPoint charts — bar, line, pie, area, doughnut, radar.

```xml
<Chart chartType="bar" w="500" h="300" showTitle="true" title="Quarterly revenue"
       showLegend="true" legendPos="b"
       chartColors='["1D4ED8","16A34A","D97706"]'
       barGrouping="stacked">
  <ChartSeries name="2025">
    <ChartDataPoint label="Q1" value="120" />
    <ChartDataPoint label="Q2" value="135" />
  </ChartSeries>
</Chart>
```

Chart-specific attributes:

| Attribute     | Notes                                                                                                              |
| ------------- | ------------------------------------------------------------------------------------------------------------------ |
| `chartType`   | `bar` / `line` / `pie` / `area` / `doughnut` / `radar`. Required.                                                  |
| `title`       | Chart title (shown when `showTitle="true"`).                                                                       |
| `showTitle`   | Boolean — default `false`.                                                                                         |
| `showLegend`  | Boolean — default `true`.                                                                                          |
| `legendPos`   | `t` / `b` / `l` / `r` / `tr` (top / bottom / left / right / top-right).                                            |
| `showValue`   | Boolean — print numeric labels on each datum.                                                                      |
| `chartColors` | JSON array of hex strings — explicit per-series palette. Overrides the deck's theme colours.                       |
| `barGrouping` | `clustered` / `stacked` / `percentStacked` (bar charts only).                                                      |
| `data`        | JSON array `[{name?, labels[], values[]}]` — alternative to `<ChartSeries>` / `<ChartDataPoint>` element notation. |

Use the element form for hand-authored charts and the `data="…"`
JSON form when the series come from a `<Foreach>` loop or upstream
data. Charts behave as native PowerPoint shapes — every axis,
label, and grid line stays editable downstream.

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
<HStack padding="48" gap="24" alignItems="start" justifyContent="spaceBetween">
  <VStack w="50%"><Text>Left</Text></VStack>
  <VStack w="50%"><Text>Right</Text></VStack>
</HStack>
```

Flex attributes both stacks accept:

| Attribute        | Values                                                                      |
| ---------------- | --------------------------------------------------------------------------- |
| `gap`            | number                                                                      |
| `alignItems`     | `start` / `center` / `end` / `stretch` (no `baseline`)                      |
| `justifyContent` | `start` / `center` / `end` / `spaceBetween` / `spaceAround` / `spaceEvenly` |
| `flexWrap`       | `nowrap` / `wrap` / `wrapReverse`                                           |

Child sizing:

| Attribute                         | Notes                                     |
| --------------------------------- | ----------------------------------------- |
| `w` / `h`                         | percentage / `"max"` / number             |
| `minW` / `maxW` / `minH` / `maxH` | hard constraints                          |
| `alignSelf`                       | per-child override of parent `alignItems` |

`flexGrow`, `flexShrink`, and `flexBasis` are exposed on every node.
They override the context-aware defaults (HStack equal-distribution,
growing-sibling pin, noWrap shrink-0), so an author-specified value
always wins:

```xml
<HStack>
  <VStack flexGrow="2" />     <!-- claims twice the slack -->
  <VStack flexShrink="0" />   <!-- pinned; won't shrink under pressure -->
  <VStack flexBasis="200" />  <!-- 200px starting basis before grow/shrink -->
</HStack>
```

`flexBasis="max"` maps to flex-basis auto (size to content first,
then participate in grow). The simpler idiom `w="max"` / `h="max"`
remains the recommended shorthand for "fill the remaining axis".

### `<Layer>` — absolute positioning

```xml
<Layer w="1280" h="720">
  <Shape shapeType="rect" x="0" y="0" w="1280" h="80" fill.color="0F172A" />
  <Text x="48" y="24" w="600" fontSize="32" bold="true" color="FFFFFF">Header</Text>
</Layer>
```

> **Accessibility**: PowerPoint screen readers iterate shapes in document source order, not visual position. Place decorative backgrounds first, informational shapes last when using `<Layer>`. Mark backgrounds `isDecorative="true"`.

## Speaker notes

```xml
<Slide>
  <VStack><Text>Slide content</Text></VStack>
  <Notes>
    Speaker notes for this slide. The audience never sees these.
  </Notes>
</Slide>
```

`<Notes>` accepts plain text or `<Text>` runs.

## Editorial idioms

### Mixed-size text rows

A `<HStack>` row of two `<Text>` elements with different `fontSize`s
needs two compensations to look optically aligned:

1. **`textVAlign="middle"` on every sibling** — PPTX text frames anchor glyphs to the top by default.
2. **`lineHeight="1.0"` on the larger sibling** — tighten line height for single-line rows.

```xml
<HStack gap="10" alignItems="center">
  <Text fontFamily="Georgia" fontSize="13" bold="true" lineHeight="1.0" textVAlign="middle">Mara Olsen</Text>
  <Text fontFamily="Inter"   fontSize="9"  bold="true" lineHeight="1.0" textVAlign="middle">Lead writer</Text>
</HStack>
```

### Multi-line headlines and line breaks

`<Text>` body content is whitespace-normalized before rendering:

- **Source line breaks collapse to a single space.** A run of whitespace
  spanning a newline in the XML source (typical when one prose line is
  hand-wrapped across two source lines) becomes one space — HTML-like
  "normal" whitespace.
- **Same-line whitespace is preserved.** Leading and interior spaces
  on a single source line stay intact, so
  `<Text class="code-text">  some code</Text>` keeps its indent.
- **Leading / trailing whitespace is trimmed** from the body run.

To force a deliberate line break, pick one of two forms:

1. **`\n` literal escape** inside the body. The parser decodes escapes
   _after_ collapsing source newlines, so author-written `\n` survives
   and renders as a real line break. `\t` (tab) and `\\` (literal
   backslash) decode the same way; any other `\X` passes through
   verbatim (paths like `C:\Users\foo` stay readable). Escapes are
   only honoured in element body text — attribute values are left
   untouched.

   ```xml
   <Text class="display-xl">Quiet doesn't\nmean empty.</Text>
   ```

2. **Stack of single-line `<Text>`** — preferred when each line needs
   its own style or `class`.

   ```xml
   <VStack gap="0">
     <Text class="display-xl">Quiet doesn't</Text>
     <Text class="display-xl">mean empty.</Text>
   </VStack>
   ```

> **`<Br>` is not in the grammar.** There is no line-break element;
> authoring `<Br/>` is silently ignored. Use `\n` or a stack of
> single-line `<Text>` instead. See
> [`schema-gotchas.md`](./schema-gotchas.md).

For continuous prose with natural wrap, write a single `<Text>` with a
fixed `w="…"` and let the layout engine choose break points.

### Lead-in (drop-cap substitute)

True drop-cap is not in the medium. The closest editorial idiom is a
**lead-in**: an oversized accent-colored first sentence introducing
the paragraph, with the body running below at normal weight.

```xml
<VStack gap="10">
  <Text fontFamily="Georgia" fontSize="22" bold="true" color="9A2A1F" lineHeight="1.2">
    The most photographed moment of a release is the launch day.
  </Text>
  <Text fontFamily="Georgia" fontSize="13" color="3A332B" lineHeight="1.5">
    The work that actually defines whether the release was good gets
    almost no coverage.
  </Text>
</VStack>
```

## Sizing recap

| Value   | Meaning                                                  |
| ------- | -------------------------------------------------------- |
| number  | Size in pixels (96 DPI; converted to inches internally). |
| `"50%"` | Percentage of the parent's content box.                  |
| `"max"` | Fill the remaining space along the main axis.            |
| omitted | The Flex layout sizes the child based on its content.    |

## Auto-fit

When a slide's measured content exceeds the slide height, the builder
shrinks in this order:

1. Reduce `<Tr>` / `<Td>` row heights.
2. Reduce text font sizes.
3. Reduce `gap` and `padding`.
4. Apply uniform scale (down to 0.5×; below that, content stays at its natural size and `AUTOFIT_OVERFLOW` is emitted).

Disable with `autoFit: false` in the build options when pixel-perfect
reproducibility matters more than fit.
