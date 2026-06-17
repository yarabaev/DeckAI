# Composition

When a deck grows past one slide, you'll want reusable, parameterized,
and split-across-files pieces. Five orthogonal mechanisms — all run at
**parse time** with no runtime cost beyond expansion.

| Mechanism | Reuses | Reference syntax |
| --- | --- | --- |
| `<Styles>` | A bag of attributes | `class="name"` on any node |
| `<Templates>` | A subtree (parameterized) | `<Use template="name" k="v" />` |
| `<Master>` | A whole slide backdrop | `master="name"` on `<Slide>` |
| `<Import>` | Another file, anywhere in the tree | `<Import src="path.xml" />` |
| `<If>` / `<Choose>` / `<Foreach>` | Conditional / iterative emission | inline |

## `<Styles>` — named attribute presets

```xml
<Styles>
  <Style name="page"  padding="48" backgroundColor="F8FAFC" />
  <Style name="title" fontSize="40" bold="true" color="0F172A" />
  <Style name="muted" fontSize="18" color="64748B" />
</Styles>

<Slide>
  <VStack class="page" gap="12">
    <Text class="title">Q4 Highlights</Text>
    <Text class="muted">Three things that mattered.</Text>
  </VStack>
</Slide>
```

Rules:
- `<Styles>` sits directly under `<SlideGlance>` (or `<Fragment>` for imports).
- Multiple classes: `class="title primary"` — later classes override earlier ones.
- Per-node attributes override class values.
- A class is a flat attribute set; styles do not nest.

> **Pitfall**: a `<Style>` with a text-style attribute (`color`,
> `fontSize`, `bold`, …) applied to `<VStack>` / `<HStack>` will error
> — those attributes aren't valid on flex containers. Split into two
> classes: one for the container, one for the inner text.

## `<Templates>` — reusable subtrees

```xml
<Templates>
  <Template name="topicCard" params="num,title,body">
    <VStack w="200" padding="12" backgroundColor="FFFFFF"
            border.color="0E0D6A" border.width="2">
      <VStack gap="6" alignItems="center">
        <Shape w="36" h="36" shapeType="ellipse"
               fill.color="E8EAF6" fontSize="12" color="0E0D6A">{num}</Shape>
        <Text fontSize="11" color="0E0D6A" bold="true" textAlign="center">{title}</Text>
        <Text fontSize="9"  color="3C3C3C" textAlign="center">{body}</Text>
      </VStack>
    </VStack>
  </Template>
</Templates>

<Slide>
  <HStack padding="48" gap="12">
    <Use template="topicCard" num="01" title="New ventures" body="Cloud +42% YoY" />
    <Use template="topicCard" num="02" title="Cost discipline" body="OPEX ratio −1.2pp" />
  </HStack>
</Slide>
```

Placeholder substitution:
- `{name}` substitutes in any attribute value or text content.
- Every `<Use>` attribute (except reserved `template`) becomes a placeholder.
- `{{name}}` (double braces) outputs a literal `{name}`.

> **Hyphens in parameter names don't work** — `params="tag-tone"` and
> `{tag-tone}` won't substitute. Use camelCase or snake_case.
>
> **Reserved-name shadowing**: a parameter named `name` may collide
> with placeholder resolution in some contexts. Rename to a more
> specific identifier (`rowName`, `kpiName`, …).

### Slots for multi-element content

Attributes hold strings only. For paragraph or multi-element content,
use `<Slot>`.

```xml
<Template name="card">
  <VStack padding="16" backgroundColor="FFFFFF">
    <Text class="title">{title}</Text>
    <Slot name="body" />
  </VStack>
</Template>

<Use template="card" title="Highlights">
  <Slot name="body">
    <Text>Multiple paragraphs.</Text>
    <Text fontSize="12" color="999999">Even mixed nodes.</Text>
    <Ul><Li>Bullet</Li></Ul>
  </Slot>
</Use>
```

A `<Slot>` with children defines a default emitted when no slot is
supplied:

```xml
<Template name="card">
  <VStack>
    <Slot name="body"><Text>(no body)</Text></Slot>
  </VStack>
</Template>
```

### Recursion + scope

- Templates are global within the document. Collected before any expansion, so a template can `<Use>` another template defined later or imported from another file.
- A template body may invoke another template. Recursion is bounded at depth **32** to catch cycles.
- After expansion, `<Slide>` still requires exactly one root child.
- `<Templates>` blocks must sit at `<SlideGlance>` or `<Fragment>` root.

`maxTemplateNodes` (default 100,000) caps total nodes produced by
`<Use>` expansion. Decks that exceed it emit
`TEMPLATE_EXPANSION_LIMIT`.

## Control flow

### `<If test="…">`

Emits body when `test` is truthy. Falsy: `false`, `null`, `undefined`,
`0`, `""`, empty array.

```xml
<If test="!isLast">
  <VStack class="bg-hairline" w="2" h="48" />
</If>
```

### `<Choose>` / `<When>` / `<Otherwise>`

First-match branch.

```xml
<Choose>
  <When test="tone == 'coral'">
    <Text class="caption" color="AA2D00">{date}</Text>
  </When>
  <When test="tone == 'forest'">
    <Text class="caption" color="0A2E0E">{date}</Text>
  </When>
  <Otherwise>
    <Text class="caption" color="6B4A1A">{date}</Text>
  </Otherwise>
</Choose>
```

### `<Foreach items="…" as="m">`

```xml
<Foreach items='[
  {"label":"Q1","tone":"coral", "date":"JAN", "title":"…"},
  {"label":"Q2","tone":"forest","date":"APR", "title":"…"},
  {"label":"Q3","tone":"mustard","date":"JUL","title":"…"}
]' as="m" indexAs="i" lastAs="isLast">
  <Use template="timeline-row"
       label="{m.label}" tone="{m.tone}" date="{m.date}" title="{m.title}"
       isLast="{isLast}" />
</Foreach>
```

| Attribute | Required | Notes |
| --- | --- | --- |
| `items` | Yes | JSON array — inline literal or `"{ref}"` to a parent attribute. |
| `as` | Yes | Iteration variable name. |
| `indexAs` | No | 0-based index. |
| `firstAs` | No | Boolean for the first iteration. |
| `lastAs` | No | Boolean for the last iteration. |

> **`<`, `>`, `&` inside JSON strings must be escaped** as `&lt;`,
> `&gt;`, `&amp;`. The XML parser treats them as structural
> characters even inside attribute values.
>
> **Apostrophe inside JSON strings** prematurely terminates the
> single-quoted attribute. Escape as `&apos;` or rewrite the prose.

### Expression grammar (`test=`, `items=` after substitution)

| Form | Example | Notes |
| --- | --- | --- |
| Identifier / dotted path | `m`, `m.tone.shade` | Walks objects; `undefined` past null / missing. |
| Literals | `"text"`, `'text'`, `42`, `3.14`, `true`, `false`, `null` | Strings support `\"`, `\'`, `\n`, `\t` escapes. |
| Comparisons | `==`, `!=`, `<`, `<=`, `>`, `>=` | `==` / `!=` coerce string ↔ number. |
| Logical | `&&`, `\|\|`, `!` | Short-circuits. |
| Helpers | `empty(x)`, `not(x)`, `length(x)` | `empty` is true for null / undefined / `""` / `[]` / `{}`. |
| Parens | `(expr)` | Standard grouping. |

Intentionally absent: arithmetic, regex, indexing (`[]`), string
concatenation, ternary. If you need that, generate the XML from
TypeScript instead.

## `<Import>` — split a deck across files

```xml
<SlideGlance>
  <Document size="16:9" />
  <Import src="./_styles.xml" />
  <Import src="./_templates.xml" />

  <Slide>
    <VStack class="page">
      <Text class="title">Quarterly report</Text>
      <Import src="./_topic-cards.xml" />
    </VStack>
  </Slide>
</SlideGlance>
```

Imported files require a `<Fragment>` root (or `<SlideGlance>` if also
runnable standalone). Children are inlined; root attributes are
ignored.

```xml
<!-- _styles.xml -->
<Fragment>
  <Styles>
    <Style name="page" padding="48" />
  </Styles>
</Fragment>
```

Imports work anywhere in the tree. Resolver is supplied via
`buildPptx` options and must enforce a base directory for untrusted
input (see [`lint.md`](./lint.md) and the builder's Security guide).

Recursion is bounded at depth **16** (lower than `<Use>` because
imports involve I/O). Cycles are detected by absolute path.

## Recommended multi-file structure

```
deck/
├── main.sgx              # <SlideGlance> root, <Document>, <Master>, slide list
├── _styles.xml           # <Fragment> with <Styles>
├── _templates.xml        # <Fragment> with <Templates>
└── slides/
    ├── _01-cover.xml     # <Fragment> with one <Slide>
    ├── _02-summary.xml
    └── _03-deep-dive.xml
```

The
[reference deck](https://github.com/SlideGlance/slideglance/tree/main/examples/builder-reference)
demonstrates this at production scale.

## Master slides

Reusable backdrop chrome — header bars, footer text, page numbers,
watermarks.

```xml
<SlideGlance>
  <Document size="16:9" defaultMaster="CORP" />

  <Master name="CORP" backgroundColor="F8FAFC" margin="48" margin.top="120">
    <MasterRect x="0" y="0" w="1280" h="48" fill.color="0F172A" />
    <MasterText x="48"   y="14" w="400" h="24" text="ACME Corp" color="FFFFFF" fontSize="14" />
    <SlideNumber x="1180" y="690" w="60" h="20" fontSize="10" color="64748B" />
  </Master>

  <Slide>
    <VStack><Text fontSize="40" bold="true">Q4 Highlights</Text></VStack>
  </Slide>
</SlideGlance>
```

`<Master>` children:

| Element | Purpose |
| --- | --- |
| `<MasterRect>` | Filled rectangle. `fill.color="HHHHHH"` — NOT flat `fill="HHHHHH"`. |
| `<MasterText>` | Static text box. |
| `<MasterImage>` | Logo or watermark. |
| `<MasterLine>` | Decorative line. Positioned-rect form (`x, y, w, h, line.color, line.width`) — NOT endpoint pairs. |
| `<SlideNumber>` | Auto page number. Accepts `x, y, w, h, fontSize, fontFamily, color`. **No `format`, no `textAlign`.** |

See [`schema-gotchas.md`](./schema-gotchas.md) for the full Master-children attribute table.

`<Master>` background options:
- `backgroundColor="HHHHHH"` — solid color
- `backgroundPath="./path.jpg"` — file path
- `backgroundData="data:image/png;base64,…"` — inline

### Multiple masters

```xml
<Master name="LIGHT" backgroundColor="FFFFFF">…</Master>
<Master name="DARK"  backgroundColor="0F172A">…</Master>

<Slide master="LIGHT"><VStack><Text color="0F172A">Light slide</Text></VStack></Slide>
<Slide master="DARK"> <VStack><Text color="FFFFFF">Dark slide</Text></VStack></Slide>
```

### Master from existing PPTX

```ts
import { readFileSync } from "node:fs";

await buildPptx(xml, { w: 1280, h: 720 }, {
  masterPptx: readFileSync("./corporate-template.pptx"),
});
```

The builder extracts the first slide's background and applies it as a
default master. Failures emit `MASTER_PPTX_PARSE_FAILED`.

### Content area margins

```xml
<Master name="CORP" margin="48" margin.top="120">…</Master>
```

`margin` defines the slide's content area inset.
