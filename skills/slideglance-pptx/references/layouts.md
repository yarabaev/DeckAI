# Layout recipes

Common slide compositions expressed in slideglance grammar. Start from
the closest recipe and replace content.

## Choosing a layout strategy

| System                                        | When to reach for it                                                                                    |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `<VStack>` / `<HStack>` (Flexbox flow)        | Default for any one-axis flow. Most slide content.                                                      |
| `position="absolute"` inside a flow container | Single overlay anchored to a flow container's bounds (e.g. corner page number).                         |
| `<Layer>` + child `x` / `y`                   | Multiple overlapping elements with arbitrary positions. Diagrams, infographics, freely composed scenes. |
| `<Line>` with `x1` / `y1` / `x2` / `y2`       | A straight line between two specific points (parent-absolute coordinates).                              |

Within a single `<Layer>`, prefer `x` / `y` over `position="absolute"`.

## 1 — Cover (title slide)

```xml
<Slide master="COVER">
  <VStack padding="80" justifyContent="center" gap="16" h="max">
    <Text fontSize="14" color="64748B">QUARTERLY REVIEW</Text>
    <Text fontSize="68" bold="true" color="0F172A" lineHeight="1.05">
      The year we picked depth over reach.
    </Text>
    <Text fontSize="20" color="1F2937" lineHeight="1.4" w="800">
      2026 product report from the platform team.
    </Text>
  </VStack>
</Slide>
```

## 2 — Title + body (single column)

```xml
<Slide>
  <VStack class="page" gap="20">
    <Text class="title">What we shipped</Text>
    <Text class="muted">Three changes that bent the curve.</Text>
    <Ul fontSize="20" class="body">
      <Li>Edge cache hit rate: 71 → 89%.</Li>
      <Li>P95 cold-start: 1.4s → 380ms.</Li>
      <Li>Build minutes per PR: 18 → 7.</Li>
    </Ul>
  </VStack>
</Slide>
```

The workhorse layout. Almost every scenario deck uses this for
content slides.

## 3 — Two-column (HStack 50 / 50)

```xml
<Slide>
  <HStack class="page" gap="48">
    <VStack w="50%" gap="12">
      <Text class="title">What changed</Text>
      <Text class="body">
        Replaced the per-request worker pool with a long-lived event
        loop. Cold-start dropped from 1.4s to under 400ms.
      </Text>
    </VStack>
    <VStack w="50%" gap="12">
      <Image src="./assets/before-after.png" w="100%" />
      <Text fontSize="12" color="64748B" textAlign="center">
        Cold-start P95 over the rollout window.
      </Text>
    </VStack>
  </HStack>
</Slide>
```

For asymmetric splits (60 / 40, 70 / 30), set explicit `w="60%"` on
the primary column. `flexGrow` is not in the schema — use percentage
widths or `w="max"` on one column.

## 4 — KPI grid

```xml
<Templates>
  <Template name="kpi" params="label,value,delta,tone">
    <VStack w="240" padding="20" gap="6"
            backgroundColor="FFFFFF"
            border.color="E2E8F0" border.width="1"
            borderRadius="8">
      <Text fontSize="12" color="64748B">{label}</Text>
      <Text fontSize="36" bold="true" color="0F172A">{value}</Text>
      <Text fontSize="13" color="{tone}">{delta}</Text>
    </VStack>
  </Template>
</Templates>

<Slide>
  <VStack class="page" gap="24">
    <Text class="title">Q4 in numbers</Text>
    <HStack gap="16">
      <Use template="kpi" label="ARR" value="$48.2M" delta="+22% YoY" tone="16A34A" />
      <Use template="kpi" label="Net retention" value="118%" delta="+4pp QoQ" tone="16A34A" />
      <Use template="kpi" label="P95 cold-start" value="380ms" delta="−73% QoQ" tone="16A34A" />
    </HStack>
  </VStack>
</Slide>
```

> **Equal-share variant** — omit `w` on the `kpi` template (and on every
> sibling inside the `<HStack>`) to get the row to auto-distribute the
> available width equally. `<HStack gap="…" alignItems="stretch">` + no
> explicit `w` on the children is the simplest "fill row with N equal
> cards" idiom; `flexGrow` is unnecessary for the equal-share case.

## 5 — Quote / pullquote

```xml
<Slide>
  <VStack class="page" justifyContent="center" alignItems="center" h="max" gap="24" padding="120">
    <Text fontSize="48" lineHeight="1.25" textAlign="center" color="0F172A"
          fontFamily="Georgia" italic="true">
      "The work that defines whether a release was good
      is the year that follows it."
    </Text>
    <Text fontSize="13" color="64748B">Mara Olsen, 2026 retrospective</Text>
  </VStack>
</Slide>
```

## 6 — Timeline (Foreach + Template)

```xml
<Templates>
  <Template name="timelineRow" params="date,label,tone,body,isLast">
    <HStack gap="20" alignItems="start">
      <VStack alignItems="center" gap="0" w="80">
        <Shape shapeType="ellipse" w="14" h="14" fill.color="{tone}" />
        <If test="!isLast">
          <VStack w="2" h="80" backgroundColor="E2E8F0" />
        </If>
      </VStack>
      <VStack gap="4">
        <Text fontSize="14" bold="true" color="{tone}">{date}</Text>
        <Text fontSize="18" bold="true" color="0F172A">{label}</Text>
        <Text fontSize="14" color="475569">{body}</Text>
      </VStack>
    </HStack>
  </Template>
</Templates>

<Slide>
  <VStack class="page" gap="16">
    <Text class="title">Rollout timeline</Text>
    <Foreach items='[
      {"date":"Jan","label":"Behind a flag","tone":"6B7280","body":"Internal-only, 1% sampling."},
      {"date":"Feb","label":"Friendly cohort","tone":"1D4ED8","body":"50 design-partner orgs."},
      {"date":"Mar","label":"GA","tone":"16A34A","body":"Open enrollment, default-on for new orgs."}
    ]' as="m" lastAs="isLast">
      <Use template="timelineRow"
           date="{m.date}" label="{m.label}" tone="{m.tone}" body="{m.body}"
           isLast="{isLast}" />
    </Foreach>
  </VStack>
</Slide>
```

## 7 — Table (data-heavy)

```xml
<Slide>
  <VStack class="page" gap="16">
    <Text class="title">Rollout cohorts</Text>
    <Table defaultRowHeight="36" cellBorder.color="CBD5E1" cellBorder.width="1">
      <Col w="160" />
      <Col w="100" />
      <Col w="120" />

      <Tr>
        <Td class="th">Cohort</Td>
        <Td class="th">Org count</Td>
        <Td class="th">P95</Td>
      </Tr>
      <Tr>
        <Td>Internal</Td>
        <Td>1</Td>
        <Td>410ms</Td>
      </Tr>
    </Table>
  </VStack>
</Slide>
```

Use `<Styles>` for `class="th"` to keep per-cell markup tight. `<Td>`
accepts both `padding` and `margin` as aliases for the cell's inner
spacing (PPTX table cells have no outer-spacing concept — see
[`schema-gotchas.md`](./schema-gotchas.md) §"`<Td>` `padding` /
`margin`"). When both are present, `padding` wins.

## 8 — Diagram (Layer + Shape + Line)

```xml
<Slide>
  <Layer w="1280" h="720">
    <Shape shapeType="rect" x="0" y="0" w="1280" h="80" fill.color="0F172A" />
    <Text  x="48" y="24" w="600" fontSize="28" bold="true" color="FFFFFF">System overview</Text>

    <Shape shapeType="roundRect" x="120" y="220" w="240" h="120"
           fill.color="DBEAFE" line.color="1D4ED8" line.width="2"
           fontSize="18" bold="true" color="1D4ED8" textAlign="center">Ingest</Shape>
    <Shape shapeType="roundRect" x="520" y="220" w="240" h="120"
           fill.color="FEF3C7" line.color="D97706" line.width="2"
           fontSize="18" bold="true" color="D97706" textAlign="center">Process</Shape>

    <Line x1="360" y1="280" x2="520" y2="280" lineWidth="2"
          color="334155" endArrow.type="triangle" />
  </Layer>
</Slide>
```

> Reading order in PowerPoint follows document source order, not
> visual position. Place decorative backgrounds first, informational
> shapes last. Mark backgrounds `isDecorative="true"`.

## 9 — Chart slide

```xml
<Slide>
  <VStack class="page" gap="16">
    <Text class="title">Quarterly revenue</Text>
    <Chart chartType="bar" w="1000" h="420"
           showTitle="false"
           showLegend="true" legendPos="bottom">
      <ChartSeries name="2025">
        <ChartDataPoint label="Q1" value="120" />
        <ChartDataPoint label="Q2" value="135" />
        <ChartDataPoint label="Q3" value="148" />
      </ChartSeries>
    </Chart>
  </VStack>
</Slide>
```

`chartType` supports `bar`, `line`, `pie`, `area`, `doughnut`, `radar`.

## 10 — Section divider

```xml
<Slide master="DARK">
  <VStack padding="120" justifyContent="center" h="max" gap="16">
    <Text fontSize="14" color="94A3B8">PART TWO</Text>
    <Text fontSize="80" bold="true" color="FFFFFF" lineHeight="1.05">
      What we learned.
    </Text>
  </VStack>
</Slide>
```

## 11 — Vertical card (3:4 / 9:16 for social)

```xml
<Document size="custom" w="960" h="1280" />
```

> **Pitfall**: `<Document>` cannot accept both `size="..."` and
> `w/h` — use one or the other. Drop `size="custom"` and provide
> only `w` / `h`.

```xml
<Slide>
  <VStack padding="64" gap="20" h="max" backgroundColor="FEF8F1">
    <Text fontSize="11" color="C2410C">01 · SLOW LIVING</Text>
    <Text fontSize="56" bold="true" color="431407" lineHeight="1.05" fontFamily="Playfair Display">
      The morning you don't owe anyone.
    </Text>
    <Image src="./assets/coffee.jpg" w="100%" sizing.type="cover" h="540" borderRadius="24" />
  </VStack>
</Slide>
```

## 12 — Agenda

Use an agenda slide when the deck has three or more sections. Keep the
structure quiet: section number, section title, one short phrase.

```xml
<Templates>
  <Template name="agendaItem" params="num,title,body">
    <HStack gap="20" alignItems="center">
      <Text w="56" fontSize="22" bold="true" color="1D4ED8" textAlign="right">{num}</Text>
      <VStack gap="4">
        <Text fontSize="20" bold="true" color="0F172A">{title}</Text>
        <Text fontSize="14" color="64748B">{body}</Text>
      </VStack>
    </HStack>
  </Template>
</Templates>

<Slide>
  <VStack class="page" gap="32">
    <Text class="title">Agenda</Text>
    <VStack gap="18">
      <Use template="agendaItem" num="01." title="Context" body="Why the decision matters now." />
      <Use template="agendaItem" num="02." title="Options" body="The paths we evaluated." />
      <Use template="agendaItem" num="03." title="Recommendation" body="What changes next quarter." />
    </VStack>
  </VStack>
</Slide>
```

Use `01.` rather than bare `01` when exporting through workflows that
may normalize leading zeroes in rendered previews.

## 13 — Key message

A key-message slide carries one idea. It is useful after a dense data
slide, before a recommendation, or as a chapter summary.

```xml
<Slide>
  <VStack class="page" justifyContent="center" h="max" gap="24" w="900">
    <Text fontSize="14" bold="true" color="1D4ED8" letterSpacing="0.14">
      KEY MESSAGE
    </Text>
    <Text fontSize="48" bold="true" color="0F172A" lineHeight="1.12">
      We do not need a bigger process. We need a shorter feedback loop.
    </Text>
    <Text class="body" color="475569">
      The data points to one constraint: handoffs. Every proposal below
      removes one handoff from the release path.
    </Text>
  </VStack>
</Slide>
```

Do not add three cards just to fill the canvas. Give the message room
and make the hierarchy obvious.

## 14 — Comparison cards

Use this for plans, vendors, architectures, before/after states, or
trade-off analysis. Two or three cards are the upper bound for a
single slide.

```xml
<Templates>
  <Template name="compareCard" params="label,title,body,tone">
    <VStack w="100%" padding="24" gap="12"
            backgroundColor="FFFFFF"
            border.color="{tone}" border.width="2"
            borderRadius="8">
      <Text fontSize="12" bold="true" color="{tone}">{label}</Text>
      <Text fontSize="20" bold="true" color="0F172A">{title}</Text>
      <Text fontSize="15" color="475569" lineHeight="1.45">{body}</Text>
    </VStack>
  </Template>
</Templates>

<Slide>
  <VStack class="page" gap="24">
    <Text class="title">Two viable paths</Text>
    <HStack gap="24" alignItems="stretch">
      <Use template="compareCard" label="OPTION A" title="Centralize"
           tone="1D4ED8" body="One platform team owns the pipeline and exposes paved-road templates." />
      <Use template="compareCard" label="OPTION B" title="Federate"
           tone="16A34A" body="Each product team keeps autonomy but adopts shared release contracts." />
    </HStack>
  </VStack>
</Slide>
```

For equal-width cards, omit explicit widths on children or use
percentage widths consistently. Do not hand-place cards in a `Layer`
unless the comparison itself is a diagram.

## 15 — Process row

SlideGlance does not have Pom's `ProcessArrow` node. Build process
slides from a row template, optional connector lines, and `Foreach`.

```xml
<Templates>
  <Template name="processStep" params="num,label,body,tone">
    <VStack w="100%" padding="18" gap="8"
            backgroundColor="FFFFFF"
            border.color="{tone}" border.width="1"
            borderRadius="8">
      <Text fontSize="12" bold="true" color="{tone}">{num}</Text>
      <Text fontSize="18" bold="true" color="0F172A">{label}</Text>
      <Text fontSize="13" color="64748B">{body}</Text>
    </VStack>
  </Template>
</Templates>

<Slide>
  <VStack class="page" gap="24">
    <Text class="title">Release path</Text>
    <HStack gap="16" alignItems="stretch">
      <Use template="processStep" num="01." label="Plan" tone="64748B" body="Define the rollback contract." />
      <Use template="processStep" num="02." label="Canary" tone="1D4ED8" body="Ship to the smallest safe cohort." />
      <Use template="processStep" num="03." label="Observe" tone="D97706" body="Watch health and error budgets." />
      <Use template="processStep" num="04." label="Expand" tone="16A34A" body="Roll out with an abort path." />
    </HStack>
  </VStack>
</Slide>
```

When exact arrow geometry matters, switch the row body to a `<Layer>`
and draw `<Line>` or `<Connector>` elements beneath the step cards.

## 16 — Summary / CTA

The closing slide should restate the decision and name the next
action. Avoid a generic "thank you" slide unless the user asks for it.

```xml
<Slide>
  <VStack class="page" justifyContent="center" h="max" gap="28">
    <Text fontSize="14" bold="true" color="1D4ED8" letterSpacing="0.14">
      NEXT ACTION
    </Text>
    <Text fontSize="44" bold="true" color="0F172A" lineHeight="1.12">
      Approve the canary plan and start with the payments cohort.
    </Text>
    <HStack gap="16" alignItems="stretch">
      <VStack class="card" w="33%">
        <Text class="caption">OWNER</Text>
        <Text class="heading">Platform</Text>
      </VStack>
      <VStack class="card" w="33%">
        <Text class="caption">DATE</Text>
        <Text class="heading">July 8</Text>
      </VStack>
      <VStack class="card" w="33%">
        <Text class="caption">CHECKPOINT</Text>
        <Text class="heading">48 hours</Text>
      </VStack>
    </HStack>
  </VStack>
</Slide>
```

## 17 — Reusable section header

For decks with five or more body slides, reuse the same header
structure. Change only the section label and title.

```xml
<Templates>
  <Template name="sectionHeader" params="eyebrow,title">
    <VStack gap="8">
      <Text fontSize="11" bold="true" color="1D4ED8" letterSpacing="0.16">{eyebrow}</Text>
      <Text class="title">{title}</Text>
    </VStack>
  </Template>
</Templates>

<Slide>
  <VStack class="page" gap="32">
    <Use template="sectionHeader" eyebrow="02. OPTIONS" title="What we evaluated" />
    <Text class="body">The recommendation comes from three constraints: latency, ownership, and rollback cost.</Text>
  </VStack>
</Slide>
```

Keep `fontSize`, `letterSpacing`, `color`, and vertical gap identical
across slides. This is the simplest way to make a multi-slide deck
feel intentional.

## 18 — Decorative background layer

Use a `Layer` when the slide needs visual atmosphere without adding
content. Decorative shapes should appear first in source order and
carry `isDecorative="true"`.

```xml
<Slide>
  <Layer w="1280" h="720">
    <Shape isDecorative="true" shapeType="ellipse"
           x="-180" y="-160" w="460" h="460"
           fill.color="1D4ED8" opacity="0.15" line.width="0" />
    <Shape isDecorative="true" shapeType="rect"
           x="64" y="58" w="56" h="4"
           fill.color="1D4ED8" line.width="0" />

    <VStack x="0" y="0" w="1280" h="720"
            padding="80" justifyContent="center" gap="20">
      <Text fontSize="14" bold="true" color="1D4ED8" letterSpacing="0.14">SECTION 01</Text>
      <Text fontSize="60" bold="true" color="0F172A" lineHeight="1.05">A sharper release loop.</Text>
      <Text fontSize="18" color="475569" w="760">The plan removes handoffs without hiding operational risk.</Text>
    </VStack>
  </Layer>
</Slide>
```

SlideGlance does not expose native background gradients on normal
containers. For soft gradient-like decoration, layer translucent
shapes, use an inline `<Svg>`, or use a pre-rendered image.

## Layout-rule summary

- **One root child per `<Slide>`.** Almost always `<VStack>`, `<HStack>`, or `<Layer>`.
- **Reach for `<Layer>` only for diagrams / infographics.** Flex containers handle 90% of slide content.
- **Reading order = source order.** Critical when using `<Layer>` for accessibility.
- **Promote repeated patterns to `<Templates>` after 2 occurrences.** Beyond that, the lint catches `HARDCODED_COLOR` repetition.
- **Use percentage widths or flex attributes inside `<HStack>` columns.** `w="50%"`, `w="max"`, and `flexGrow="1"` are all valid; prefer the simplest option that stays responsive to the parent width.
