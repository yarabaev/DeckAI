---
title: "@slideglance/builder — Composition"
lang: en
kind: guides
package: builder
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/builder/src/parseXml/
---

# Composition

Once a deck grows past one slide, you'll want pieces of it to be reusable, parameterizable, and split across files. The builder provides three orthogonal mechanisms for this — and a small set of control-flow tags for "almost identical" content.

| Mechanism                         | Reuse what                            | Reference syntax                       |
| --------------------------------- | ------------------------------------- | -------------------------------------- |
| `<Styles>`                        | A bag of attributes                   | `class="name"` on any node             |
| `<Templates>`                     | A subtree (parameterized by `{name}`) | `<Use template="name" name="value" />` |
| `<Master>`                        | A whole slide backdrop                | `master="name"` on `<Slide>`           |
| `<Import>`                        | Another file, anywhere in the tree    | `<Import src="path.xml" />`            |
| `<If>` / `<Choose>` / `<Foreach>` | Conditional / iterative emission      | inline in templates and slides         |

All five run at parse time. They incur no runtime cost beyond expansion, and produce the same compiled output as inlining everything by hand.

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

- `<Styles>` is a direct child of `<SlideGlance>` (or `<Fragment>` for imports).
- Multiple classes: `class="title primary"` — later classes override earlier ones.
- Per-node attributes override class values.
- A class is a flat set of attributes; styles do not nest.

## `<Templates>` — reusable subtrees

A `<Template>` is a parameterized fragment. `<Use>` instantiates it.

```xml
<Templates>
  <Template name="topicCard" params="num,title,body">
    <VStack w="200" padding="12" backgroundColor="FFFFFF"
            border.color="0E0D6A" border.width="2">
      <VStack gap="6" alignItems="center">
        <Shape w="36" h="36" shapeType="ellipse" fill.color="E8EAF6" fontSize="12" color="0E0D6A">{num}</Shape>
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
    <Use template="topicCard" num="03" title="Talent" body="Retention 94%" />
  </HStack>
</Slide>
```

### Placeholder substitution

- `{name}` substitutes in **any attribute value or text content**.
- Every `<Use>` attribute (except the reserved `template`) becomes a placeholder. `params="..."` is informational.
- A `{name}` with no matching `<Use>` attribute is a parse error.
- `{{name}}` (double braces) outputs a literal `{name}`.

### Slots for multi-element content

Attributes hold strings only. For paragraph-length or multi-element content, use `<Slot>`.

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

A template `<Slot>` with children defines a default emitted when no slot is supplied:

```xml
<Template name="card">
  <VStack>
    <Slot name="body"><Text>(no body)</Text></Slot>
  </VStack>
</Template>
```

`<Slot name="X">{x}</Slot>` (default = single placeholder) idiomatically allows either an attribute (short) or a slot (long) to provide the same content.

### Forward references and recursion

- Templates are global within the document. They are collected before any expansion, so a template can `<Use>` another template defined later or imported from another file.
- A template body may invoke another template via `<Use>`. Recursion is bounded at depth **32** to catch cycles.
- After expansion, `<Slide>` still requires exactly one root child. Design templates so each `<Use>` produces a single root.

> `<Templates>` blocks must sit at `<SlideGlance>` or `<Fragment>` root. Nested `<Templates>` inside `<Slide>` / `<VStack>` / etc. are silently dropped with a `TEMPLATES_NOT_AT_ROOT` diagnostic.

### Expansion limit

`maxTemplateNodes` (default 100,000) caps total nodes produced by `<Use>` expansion. Decks that exceed it emit `TEMPLATE_EXPANSION_LIMIT` and the surplus is aborted.

## Control flow

Three tags emit conditional or iterative content. They run in the same parse-time pass as `<Use>` and read the same scope.

### `<If test="expr">…</If>`

Emits its body when `expr` is truthy. Falsy values: `false`, `null`, `undefined`, `0`, `""`, empty array.

```xml
<If test="!isLast">
  <VStack class="bg-hairline" w="2" h="48" />
</If>
```

### `<Choose>` / `<When>` / `<Otherwise>`

First-match branch. The body of the first `<When>` whose `test` is truthy is emitted; if none match and `<Otherwise>` is present, its body is emitted.

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

At most one `<Otherwise>` per `<Choose>`.

### `<Foreach items="..." as="m">`

Repeats its body once per element of `items`.

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

| Attribute | Required | Notes                                                           |
| --------- | -------- | --------------------------------------------------------------- |
| `items`   | Yes      | JSON array — inline literal or `"{ref}"` to a parent attribute. |
| `as`      | Yes      | Iteration variable name.                                        |
| `indexAs` | No       | 0-based index variable.                                         |
| `firstAs` | No       | Boolean for the first iteration.                                |
| `lastAs`  | No       | Boolean for the last iteration.                                 |

Each iteration produces an independent subtree, so attribute mutations never leak between rows.

### Expression grammar

`test=` and `items=` (after substitution) accept a small expression language.

| Form                     | Example                                                   | Notes                                                      |
| ------------------------ | --------------------------------------------------------- | ---------------------------------------------------------- |
| Identifier / dotted path | `m`, `m.tone.shade`                                       | Walks objects; returns `undefined` past null / missing.    |
| Literals                 | `"text"`, `'text'`, `42`, `3.14`, `true`, `false`, `null` | Strings support `\"`, `\'`, `\n`, `\t` escapes.            |
| Comparisons              | `==`, `!=`, `<`, `<=`, `>`, `>=`                          | `==` / `!=` coerce string ↔ number.                        |
| Logical                  | `&&`, `\|\|`, `!`                                         | Short-circuits.                                            |
| Helpers                  | `empty(x)`, `not(x)`, `length(x)`                         | `empty` is true for null / undefined / `""` / `[]` / `{}`. |
| Parens                   | `(expr)`                                                  | Standard grouping.                                         |

Intentionally absent: arithmetic, regex, indexing (`[]`), string concatenation, ternary. If you need that, generate the XML from TypeScript instead.

### Placeholder paths

`{name}` and `{name.deep.path}` substitute into attribute values and text content. Object / array values stringify to JSON; primitives use `String()`.

> **Object iteration variables don't survive a `<Use>` boundary.** Pass scalar fields explicitly (`title="{m.title}"` rather than `m="{m}"`). The placeholder substitution runs _into_ the template body; passing an object as a parameter would require re-parsing JSON inside the template, which the engine intentionally does not support.

## `<Import>` — split a deck across files

`<Import src="path.xml" />` splices the content of another file in place at parse time.

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

### Imported file format

Imported files require a `<Fragment>` root (or `<SlideGlance>` if the file is also runnable standalone — its children are inlined and root attributes are ignored).

```xml
<!-- _styles.xml -->
<Fragment>
  <Styles>
    <Style name="page" padding="48" />
  </Styles>
</Fragment>
```

```xml
<!-- _slide-summary.xml -->
<Fragment>
  <Slide>
    <VStack class="page"><Text>Summary</Text></VStack>
  </Slide>
</Fragment>
```

Other roots (or multiple top-level elements) are rejected with a parse error.

### Placement

`<Import>` is valid **anywhere** in the tree — at the `<SlideGlance>` root, inside a `<Slide>`, inside a container, even inside a `<Template>` body. The imported children replace the `<Import>` element.

### Resolver

Imports require a synchronous resolver passed via `buildPptx` options:

```ts
import { readFileSync, realpathSync } from "node:fs";
import { dirname, resolve, sep } from "node:path";
import { buildPptx, type ImportResolver } from "@slideglance/builder";

const allowedBaseDir = resolve("./slides");
const allowedBase = realpathSync(allowedBaseDir);

const resolveImport: ImportResolver = (src, fromPath) => {
  const baseDir = fromPath ? dirname(fromPath) : process.cwd();
  const candidate = resolve(baseDir, src);
  const absolute = realpathSync(candidate);
  if (!absolute.startsWith(allowedBase + sep)) {
    throw new Error(`Import outside allowed directory: ${src}`);
  }
  return { content: readFileSync(absolute, "utf8"), path: absolute };
};

const documentPath = resolve(allowedBaseDir, "main.sgx");
await buildPptx(
  readFileSync(documentPath, "utf8"),
  { w: 1280, h: 720 },
  { resolveImport, sourcePath: documentPath },
);
```

The absolute `path` (after `realpathSync` normalization) is used for cycle detection.

> **Security**: when processing untrusted XML, the resolver must enforce a base directory and reject paths that escape it. See [Security](./security.md).

### Notes

- Imports expand **before** `<Templates>` collection and `<Use>` expansion, so an imported file may contribute `<Styles>`, `<Templates>`, `<Master>`, `<Slide>`, or any content fragment.
- Nested imports work. Recursion depth is bounded at **16** (lower than the `<Use>` cap of 32 because imports involve I/O).
- Cycles are detected by absolute path returned from the resolver. They report a parse error.
- `<Import>` without a resolver supplied to `buildPptx` produces a clear error at parse time.

## Putting it together

A typical multi-file structure:

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

`main.sgx`:

```xml
<SlideGlance>
  <Document size="16:9" defaultMaster="CORP" />
  <Import src="./_styles.xml" />
  <Import src="./_templates.xml" />

  <Master name="CORP">…</Master>

  <Import src="./slides/_01-cover.xml" />
  <Import src="./slides/_02-summary.xml" />
  <Import src="./slides/_03-deep-dive.xml" />
</SlideGlance>
```

The [reference deck](../../../../examples/builder-reference/) demonstrates this pattern at production scale.
