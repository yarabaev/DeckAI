# Schema gotchas

A running log of `@slideglance/builder` schema surprises. The source
of truth for the schema is
**`packages/builder/src/registry/compiled/index.ts`** (and the Zod
schemas it imports from `types.ts`), **not** the hand-curated
`docs/en/xml-reference.md`. When the two disagree, the registry wins.

If you hit a `ParseXmlError: XML validation failed` — `Unknown
attribute "X"` or `Unknown attribute, did you mean "Y"?` — look here
first.

## When the deck doesn't compile — common parse errors

| Error                                                | Cause                                                                                                       |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `Unknown attribute "letterSpacing"` etc.             | Old attribute name. See the "Attributes that don't exist" table below or the dot-notation forms.            |
| `Did you mean "padding"?` (you wrote `paddingTop`)   | Camel-case shorthand isn't accepted — use dot notation: `padding.top="…"`.                                  |
| `readTagExp returned undefined`                      | `<Foreach items='…'>` JSON contains an unescaped `'` — use `&apos;` or rewrite the prose without apostrophes. |
| `<Master>.<SlideNumber>: Unknown attribute "format"` | SlideNumber accepts only `x` / `y` / `w` / `h` / `fontSize` / `fontFamily` / `color` / `textAlign`.         |
| `Unknown attribute "x1"` on `<MasterLine>`           | Endpoint-pair form not supported — use the positioned-rect form (`x, y, w, h, line.color, line.width`).     |

## Attributes that don't exist (drop them)

| Attribute    | What to do                                                                                                                                                                                                           |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `fontWeight` | Drop. pptxgenjs 4.x has no `fontWeight` option — only `bold: boolean`. Pick the closest static-instance family (e.g. `fontFamily="Inter Tight"` for heavy weights of Inter). |

`letterSpacing`, `flexGrow`, `flexShrink`, `flexBasis` **are now
supported** — see [`grammar.md`](./grammar.md) for the attribute
tables.

## Attributes that need the dot-notation form

| Wrong               | Right                |
| ------------------- | -------------------- |
| `paddingTop="120"`  | `padding.top="120"`  |
| `paddingLeft="48"`  | `padding.left="48"`  |
| `marginTop="48"`    | `margin.top="48"`    |
| `borderColor="333"` | `border.color="333"` |
| `borderWidth="2"`   | `border.width="2"`   |

Combined form works: `padding="80" padding.top="120"`.

## `padding` / `margin` shorthand

CSS-style 2-, 3-, and 4-value shorthand is supported alongside the
single-value form and dot notation:

```xml
<HStack padding="16" />               <!-- uniform -->
<HStack padding="10 0" />             <!-- vertical / horizontal -->
<HStack padding="8 12 16 12" />       <!-- top / right / bottom / left (TRBL) -->
<HStack padding.top="10" padding.bottom="10" />  <!-- dot notation -->
```

Mixing shorthand with dot notation works: `padding="80" padding.top="120"`
keeps 80 on the other three sides while overriding the top.

## Per-side border

Use a separate attribute group per side instead of nested
`border.top.color`. Each accepts the same `color` / `width` /
`dashType` shape as the uniform `border`:

```xml
<VStack
  border.color="E2E8F0" border.width="1"
  borderBottom.color="0F172A" borderBottom.width="2"
/>
```

Per-side overlays compose additively on top of the uniform border —
the renderer emits one extra `<line>` shape per configured side.
The legacy 1-px `<VStack>` / `<MasterLine>` workaround still works
but is no longer required.

## Enum values are camelCase, not kebab-case

| Wrong                            | Right                           |
| -------------------------------- | ------------------------------- |
| `justifyContent="space-between"` | `justifyContent="spaceBetween"` |
| `justifyContent="space-around"`  | `justifyContent="spaceAround"`  |
| `justifyContent="space-evenly"`  | `justifyContent="spaceEvenly"`  |

`alignItems` accepts `start` / `center` / `end` / `stretch` /
`baseline`. The `baseline` value maps to Yoga's `ALIGN_BASELINE` —
useful as the CSS-natural option for mixed-size text rows. Without
a custom Yoga baseline function on text measure nodes, Yoga falls
back to the bottom edge of each child's content box, which is close
to the visual baseline at the leaf `lineHeight=1.0`. The pixel-
perfect path for mixed-size editorial rows remains the
`textVAlign="middle" lineHeight="1.0"` idiom (see grammar.md
§"Mixed-size text rows").

## `<Master>` and its children: the real surface

Spec lives in
`packages/builder/src/parseXml/childAttributeSpecs.ts`.

### `<Master>` (root attributes)

Only: `name`, `margin` (dot notation per side), `backgroundColor`,
`backgroundPath`, `backgroundData`.

### `<MasterText>`

`text` (body string), x, y, w, h, fontSize, fontFamily, color, bold,
italic, underline (dot), strike, highlight, textAlign, lineHeight,
letterSpacing. The body string lives on the `text="…"` attribute,
not as element body content.

### `<MasterRect>`

x, y, w, h, fill (dot notation: `fill.color="…"`), border (dot
notation), borderRadius (px), opacity (0..1).
`<MasterRect ... fill="HHHHHH">` (flat) **fails** — use
`fill.color="HHHHHH"`. `opacity` maps to fill `transparency`;
`fill.transparency` takes precedence when both are supplied.

### `<MasterLine>`

Accepts either form (the dispatcher folds endpoint-pair into the
positioned-rect form):

- **Positioned rect**: x, y, w, h, line (dot notation).
- **Endpoint pair**: x1, y1, x2, y2, line (dot notation).

```xml
<!-- horizontal hairline (positioned rect) -->
<MasterLine x="80" y="664" w="1120" h="1" line.color="1F2BE0" line.width="1" />
<!-- vertical hairline (positioned rect) -->
<MasterLine x="640" y="0" w="1" h="720" line.color="1F2BE0" line.width="1" />
<!-- diagonal hairline (endpoint pair) -->
<MasterLine x1="80" y1="40" x2="1200" y2="680" line.color="1F2BE0" line.width="1" />
```

For grid patterns, drive a `<Foreach>` over x or y positions and emit
one positioned rect per gridline.

### `<MasterImage>`

src, x, y, w, h. Nothing else.

### `<SlideNumber>`

x, y, w, h, fontSize, fontFamily, color, textAlign. **No `format`,
no `bold`** — pptxgenjs 4.x emits a fixed slide-number placeholder
with no prefix/suffix surface, so `format` cannot be plumbed through.

## `<Document size="…">` conflicts with `<Document w="…" h="…">`

```xml
<!-- wrong -->
<Document size="custom" w="960" h="1280" />
<!-- right -->
<Document w="960" h="1280" />
```

## `<Svg w="…">` is capped at 1024

Larger values throw `Invalid value for attribute "w". Too big:
expected number to be <=1024`.

## `<Shape>` does not accept child elements

Body text only (via body alias or `text="…"` attribute). Branch on
the whole Shape with `<Choose>`, not inside its body.

## `<Br>` is not in the grammar

There is no line-break element. The inline tags the schema recognizes
inside `<Text>` / `<Li>` / `<Td>` body are only `<B>`, `<I>`, `<U>`,
`<S>`, `<Mark>`, `<Span>`, and `<A>`. Authoring `<Br/>` is silently
dropped (it does not error, which makes the omission easy to miss).

Two supported forms instead:

- **`\n` literal in body text** — `<Text>line 1\nline 2</Text>`. The
  parser decodes the escape after collapsing source-whitespace runs,
  so the break survives. `\t` (tab) and `\\` (literal backslash)
  decode the same way; other `\X` sequences pass through verbatim.
  Attribute values are left untouched.
- **Stack of single-line `<Text>`** — preferred when each line carries
  its own style:

  ```xml
  <VStack gap="0">
    <Text class="title">Line one</Text>
    <Text class="title">Line two</Text>
  </VStack>
  ```

See [`grammar.md`](./grammar.md) §"Multi-line headlines and line
breaks" for the full whitespace-normalization rules (source-newline
collapse, same-line-whitespace preservation, leading/trailing trim).

## `<HStack>` / `<VStack>` don't accept text-style attributes

`color`, `fontSize`, `bold`, `textAlign` directly on a flex container
errors. Same applies to a `<Style>` used as a class on a flex
container if the class carries text attrs — split into two classes.

## `<Td>` `padding` / `margin`

Both are accepted; they are aliases for cell inner padding (PPTX
table cells have no concept of outer spacing — PowerPoint calls the
inner padding "cell margin"). When both are supplied, `padding`
wins. Either may be a single number, CSS shorthand, or dot
notation.

## XML escaping inside `<Foreach items='…'>`

1. **Apostrophe inside a string** terminates the attribute early.
   Escape as `&apos;` or rewrite.
2. **`<`, `>`, `&`** must be escaped as `&lt;`, `&gt;`, `&amp;`.
3. The Bash heredoc convention `'\''` does **not** work.

```xml
<!-- right -->
<Foreach items='[{"text":"isn&apos;t supported"}]' as="r">
```

## `<Template>` parameter names

Hyphens don't substitute. Use camelCase or snake_case.

```xml
<!-- wrong: {tag-tone} won't be replaced -->
<Template name="row" params="tag-tone">
  <Shape fill.color="{tag-tone}" />
</Template>

<!-- right -->
<Template name="row" params="tagTone">
  <Shape fill.color="{tagTone}" />
</Template>
```

Some common attribute names (`name`, `value`, `note`) may shadow
internal substitution. Rename to a more specific identifier.

## Verifying a deck builds before you trust it

Structural file count is **not** evidence of correctness. Until you
have a green PNG, the deck is unverified.

```sh
# Build .sgx → .pptx (TypeScript)
# Then render .pptx → PNG from the repository root with the local CLI:
./target/release/slideglance convert ./deck.pptx --output ./out --format png --width 1280
```

If `ParseXmlError` fires, fix every reported `Unknown attribute`
before you believe the deck.

## Adding new gotchas

When you find a new attribute mismatch:

1. Confirm it's a real schema gap — grep
   `packages/builder/src/registry/compiled/index.ts` and
   `childAttributeSpecs.ts`.
2. Append an entry to the right table above.
