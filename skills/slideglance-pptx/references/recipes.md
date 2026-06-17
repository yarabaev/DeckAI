# Recipes from the example decks

This page distils patterns, tips, and non-obvious idioms that emerge
from the two runnable example trees in the repo. Use it as a cookbook
when you reach for a layout the existing references describe in the
abstract and want to see how a real deck wires it up.

- [`examples/builder-reference/`](../../../examples/builder-reference/) —
  multi-file reference deck. Every node type and every composition
  primitive (`<Templates>` / `<Use>` / `<Slot>` / `<Import>` / `<If>` /
  `<Choose>` / `<Foreach>` / `<Master>`) appears in at least one
  chapter. Doubles as a regression smoke test.
- [`examples/playground-samples/`](../../../examples/playground-samples/) —
  four scenario decks (`pitch`, `editorial`, `tech-spec`, `workshop`),
  each with its own master chrome, palette, typography, and reusable
  templates. The web playground loads them as starter samples.

Sections below cover what those trees show that the other reference
files (`grammar.md`, `composition.md`, `layouts.md`, `themes.md`,
`schema-gotchas.md`, `lint.md`) don't.

## 1. Master chrome recipes

**Gradient halo (pitch).** Pitch's cover fakes a gradient on a dark
surface by stacking two semi-transparent rounded rects off the
top-right edge; the rest of the cover is just a wordmark + accent
bar. No image asset is needed.

```xml
<!-- playground-samples/pitch/templates/master.xml -->
<Master name="COVER" backgroundColor="0B0B14">
  <MasterRect x="780" y="-200" w="700" h="700"
              fill.color="7C5CFF" fill.transparency="80" />
  <MasterRect x="900" y="-100" w="500" h="500"
              fill.color="2EE6CA" fill.transparency="88" />
  <MasterRect x="56" y="56" w="4" h="20" fill.color="2EE6CA" />
  <MasterText x="68" y="56" w="240" h="20" fontFamily="Inter"
              fontSize="12" bold="true" color="F5F5FA">◆ FLOWBASE</MasterText>
</Master>
```

**Magazine masthead (editorial).** Two `MasterText` rows + a hairline
`MasterRect` at the top, mirrored at the bottom with a centered
section label + folio. The signature is the dual top-bar:
left-aligned issue meta + right-aligned feature name closed by a 1px
hairline.

```xml
<!-- playground-samples/editorial/templates/master.xml -->
<MasterText x="48" y="38" w="300" h="14" fontSize="9" bold="true" color="6B604F">
  QUARTERLY  ·  ISSUE 14  ·  WINTER 2026</MasterText>
<MasterText x="445" y="38" w="300" h="14" fontSize="9" bold="true"
            color="9A2A1F" textAlign="right">THE LONG TAIL OF SHIPPING</MasterText>
<MasterRect x="48" y="58" w="697" h="1" fill.color="C7B998" />
```

**Left accent bar (tech-spec / workshop).** Both decks anchor every
page to a coloured `MasterRect` flush against `x=0`. Workshop uses a
thick `w=160` block on `INTRO` and a thin `w=14` ribbon on `LESSON`;
tech-spec uses `w=12`. The thick/thin pair is the workshop's "section
divider vs. body page" tell.

```xml
<!-- workshop/templates/master.xml -->
<Master name="INTRO"><MasterRect x="0" y="0" w="160" h="720" fill.color="EF6C42"/></Master>
<Master name="LESSON"><MasterRect x="0" y="0" w="14" h="720" fill.color="EF6C42"/></Master>
```

**Recurring pattern.** Every master ends with a `<SlideNumber>` (no
extra placeholder needed) and uses two hairline rects + a centred
section label for the footer strip. When the cover should be
chrome-free (builder-reference), the `COVER` master is declared with
`backgroundColor="FFFFFF"` and no children.

## 2. Template idioms

**Card primitive with full-bleed-fill caveat.** Pitch's `metric-card`
deliberately omits `h` — when stacked siblings of an
`HStack alignItems="stretch"`, `h="100%"` multiplied the column. The
recipe: set width contract (`w="100%"`), pad, gap, no height.

```xml
<!-- pitch/templates/components.xml -->
<Template name="metric-card">
  <VStack class="bg-elev" w="100%" borderRadius="14"
          padding.left="20" padding.right="20"
          padding.top="16" padding.bottom="16" gap="6">
    <Text class="eyebrow" color="2EE6CA">{label}</Text>
    <Text class="metric-num">{value}</Text>
    <Text class="body-soft">{caption}</Text>
  </VStack>
</Template>
```

**Numbered-step ribbon.** Workshop's `step-row` and pitch's
`team-row` share the shape: a fixed-size circle
(`borderRadius=h/2`) on the left + a `w="max"` text column. The
number/initial inside is centred via `textAlign="center"` on a
single line.

**Surface-parameterised cards.** Builder-reference's `quadrant-card`,
`tree-node`, `signature-card`, and `metric-card` all take
`surface=` / `class=` as a param so palette decisions live at
call-site:

```xml
<!-- builder-reference/templates/diagram.xml -->
<Template name="quadrant-card">
  <VStack class="{surface}" gap="4" padding="14" h="max" w="100%">
    <Text class="caption" color="{eyebrowColor}">{eyebrow}</Text>
    <Text fontSize="14" bold="true" color="{titleColor}">{title}</Text>
    <Text fontSize="13" color="{bodyColor}">{body}</Text>
  </VStack>
</Template>
```

**Reach for a template when** a layout repeats three or more times
(workshop ch.03 calls this "Rule of three"), when palette varies
row-to-row, or when you want the _content_ to read as data — see KPI
tiles, code blocks, callouts, exercise cards, risk pills, pyramid
tiers.

## 3. Diagram composition

**Pipeline = `Layer` + `Foreach` arrows beneath + `Foreach` tiles on
top.** Arrows drawn first so they sit beneath. The expression
language has no arithmetic, so `x1`/`x2` are pre-computed in the
JSON.

```xml
<!-- builder-reference/chapters/09-diagrams-sequence.xml -->
<Layer w="100%" h="120">
  <Foreach items='[{"x1":"116","x2":"146"},{"x1":"262","x2":"292"}]' as="c">
    <Line x1="{c.x1}" y1="60" x2="{c.x2}" y2="60"
          color="181D26" lineWidth="2" endArrow.type="triangle"/>
  </Foreach>
  <Foreach items='[{"x":"0","bg":"AA2D00","name":"Plan"}]' as="p">
    <VStack x="{p.x}" y="20" w="116" h="80" backgroundColor="{p.bg}" />
  </Foreach>
</Layer>
```

**Decision diamond = `Shape shapeType="diamond"` with baked-in
text.** No separate `Text` overlay; `textAlign` + `textVAlign` centre
the glyph inside the shape. This collapses two nodes into one.

```xml
<!-- builder-reference/chapters/11-diagrams-flow.xml -->
<Shape x="240" y="76" w="160" h="86" shapeType="diamond"
       fill.color="F5E9D4" line.color="D9A441"
       text="Surface available?" textAlign="center" textVAlign="middle"/>
```

**2×2 matrix = two `HStack`s, no `Layer`, no x/y.** Chapter 12's
lede says it out loud. Stretch alignment + a parameterised
`quadrant-card` does the rest.

**Tree connectors.** Tree-style diagrams keep the cells inside a
`Layer` so absolute connector geometry stays exact, but the cells
themselves come from a `tree-node` template — so colour/eyebrow/title
never repeat in the chapter file.

## 4. Control-flow recipes

**`Foreach` + `lastAs` + `If !isLast` connector-hide.** Used in the
vertical timeline (ch.09) and the roadmap recipe (ch.16). The
connector segment drops on the final row without splitting the list:

```xml
<!-- builder-reference/chapters/16-control-flow.xml -->
<Foreach as="m" lastAs="isLast" items='[{"label":"Q1"},{"label":"Q2"},{"label":"Q3"}]'>
  <HStack>
    <VStack class="bg-{m.tone}" />
    <If test="!isLast"><VStack class="bg-hairline" w="2" h="32"/></If>
  </HStack>
</Foreach>
```

**`Choose` inside `Foreach` for per-row dispatch.** Two flavours
appear:

1. `Choose` on a row attribute to pick a sub-template —
   `attr-row-req` vs `attr-row` based on `r.req`.
2. `Choose` to pick a chip style based on `m.status == 'ok'|'warn'`
   else fallback.

The `Otherwise` branch is mandatory only when the data may carry
unknown values.

**Filter via `Foreach`+`If`.** `<If test="m.active">` inside the
body drops rows entirely (4 input → 3 visible). Because the
expression language has no indexing or array helpers beyond
`length()` / `empty()`, filtering is always inline `If`.

## 5. Editorial / typography idioms

**Drop-cap-as-lead-in.** True drop caps need text-flow-around-shape,
which the builder doesn't support; editorial substitutes an
oversized accent-coloured _first line_ and runs the body as a
regular column.

```xml
<!-- editorial/chapters/02-feature.xml -->
<VStack gap="14" w="50%">
  <Text fontFamily="Georgia" fontSize="22" bold="true"
        color="9A2A1F" lineHeight="1.2">The most photographed moment …</Text>
  <Text class="body" lineHeight="1.55">The work that actually defines …</Text>
  <Use template="pull-quote" quote="…" attribution="…"/>
</VStack>
```

**Pull quote.** A 2px `bg-accent` VStack as a left rule + a `w="max"`
VStack carrying the quote + attribution. No icon, no border — the
rule does the work.

**Figure with caption.** A grey `bg-figure` block with a placeholder
italic caption inside, followed by a `class="caption"` line below
(`FIGURE 01 · …`). Same `Georgia` italic carries the placeholder
copy and the body's lead-in.

**Hero masthead.** Editorial cover stacks two `Text class="hero"`
lines, the second italic + accent-coloured, then a `hairline` VStack
with `margin.top`, then a metadata row of small-caps separated by
1px `bg-rule` VStacks (`FEATURE · 5,400 WORDS · …`).

## 6. Multi-file workflow conventions

**Naming + import order.** Every `main.sgx` imports in the same
order: **styles → master → templates → chapters**. Chapters are
`NN-topic.xml`; styles and templates are intent-named (`colors`,
`typography`, `master`, `page`, `components`). The order matters
because templates may consume styles and chapters consume templates.

**Per-deck `defaultMaster`.** All four playground decks set
`<Document size="…" defaultMaster="STAGE|BODY|DOC|LESSON"/>` so
chapters can skip `master="…"` on every `<Slide>`. The
builder-reference deck omits `defaultMaster` because chapter covers
explicitly switch between `BODY` and `COVER`.

**Styles granularity.** Playground decks ship **3** style files
(`colors` + `typography` + `layout`, ~70–90 lines total).
Builder-reference ships **4** (adds `decor.xml`, 126 lines of card
surfaces, chips, hairlines, code styling). The split-out `decor.xml`
is what emerges once visual vocabulary outgrows utility classes —
until then, fold everything into `layout.xml`.

**Templates granularity.** Playground decks ship **3** template
files (`master` + `page` + `components`). Builder-reference ships
**7** (`layout`, `page`, `section`, `code`, `feature`, `diagram`,
`master`). Splitting starts paying off around 8–10 templates; below
that, one `components.xml` is enough.

## 7. Non-obvious gotchas (recipe-level)

- **`<Templates>` nested inside a slide body are silently ignored.**
  Chapter 16 emits `TEMPLATES_NOT_AT_ROOT`. Declare inline-demo
  templates at the chapter `<Fragment>` root.
- **Placeholder substitution lands in attribute values too**, not
  just text — `class="bg-{m.tone}"`, `color="{m.labelColor}"`,
  `x="{p.x}"` all work and are used heavily in chapters 09/10/16.
- **Omit `h` on cards inside `HStack alignItems="stretch"`** —
  `h="100%"` multiplies (see `pitch/templates/components.xml`
  `metric-card` comment).
- **Off-canvas children inside a `<Layer>` inflate autoFit's
  overshoot.** Keep `position="absolute"` icons inside slide bounds
  (see `workshop/templates/page.xml` cover-wrench comment).
- **`<Icon variant="circle-filled">` inflates the outer box to
  `size × 1.75`** — pick `size` accordingly so card height stays
  stable (see `workshop/templates/components.xml` exercise-card
  comment).
- **No arithmetic in the expression language** — pre-compute
  `x1`/`x2` and equivalent offsets in the JSON literal (see chapter
  09 pipeline comment).
- **Escape JSON apostrophes as `&apos;`** inside `items='[…]'`, and
  wrap literal `{ident}` substrings as `{{ident}}` to skip
  placeholder substitution (chapter 16 ESCAPING callout).
- **Cross-size row alignment trick.** When a row mixes a 10pt caption
  with 13pt body and you want them sitting on the same optical
  midline, set `lineHeight="1.0"` + `textVAlign="middle"` on both —
  used in `pitch/components.xml` `team-row` and
  `tech-spec/components.xml` `kv-row`.

## 8. Documentation-deck pattern (meta)

Skip this section when authoring a normal pitch / report / deck —
it documents the meta system the `builder-reference` deck uses to
present itself as a self-describing reference. Useful when you want
to build an _attribute-reference deck for your own DSL_, schema
overview deck, internal tool documentation, or similar
"each chapter explains one element" structure.

**The template family.** `builder-reference/templates/` ships six
templates that compose every chapter:

| Template         | Role                                                                                                                                                                                           |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `section-{tone}` | Chapter-cover slide with number, kicker, title, subtitle, epigraph, four "inside" bullets, and a meta strip. One template per tone (`forest`/`cream`/`mustard`/`peach`/`mint`/`dark`/`coral`). |
| `page`           | Standard chapter page with `eyebrow`, `kicker`, `title`, `lede`, and a `<Slot name="default">` for the body.                                                                                   |
| `col2-editorial` | Two-column body layout with `<Slot name="left">` + `<Slot name="right">` for the page body.                                                                                                    |
| `h2`             | Sub-section heading; just a styled `<Text>` taking a single `text=` param.                                                                                                                     |
| `attr-row`       | One row of the attribute table — three columns (`attr`, `type`, `desc`).                                                                                                                       |
| `attr-row-req`   | Same as `attr-row` plus a red "required" pip; dispatched via `<Choose>` on `r.req`.                                                                                                            |

**Chapter shape.** Each chapter file is a `<Fragment>` containing
one or more `<Use template="section-…">` followed by one or more
`<Use template="page">`. Inside the page slot, `<Use template="col2-editorial">`
splits the body into two columns, each calling `h2` for sub-section
titles and `<Foreach>` + `attr-row[-req]` for attribute tables.

```xml
<!-- examples/builder-reference/chapters/05-icon-svg.xml — distilled -->
<Fragment>
  <Use template="section-peach" number="05"
       kicker="Pictograms + inline vectors"
       title="Icon &amp; Svg." subtitle="…"
       inside1="…" inside2="…" inside3="…" inside4="…"
       epigraph="…" epigraphAttrib="…"
       meta="Icon · Svg" leftMeta="2 leaf nodes · …" />

  <Use template="page" eyebrow="05 / Icon" kicker="Catalogue"
       title="Icon — built-in pictograms" lede="Renders a curated icon by name…">
    <Slot name="default">
      <Use template="col2-editorial">
        <Slot name="left">
          <Use template="h2" text="Attributes" />
          <Foreach as="r" items='[
            {"attr":"name","type":"string","desc":"…","req":true},
            {"attr":"size","type":"number","desc":"…"}
          ]'>
            <Choose>
              <When test="r.req"><Use template="attr-row-req" attr="{r.attr}" type="{r.type}" desc="{r.desc}" /></When>
              <Otherwise><Use template="attr-row" attr="{r.attr}" type="{r.type}" desc="{r.desc}" /></Otherwise>
            </Choose>
          </Foreach>
        </Slot>
        <Slot name="right"><!-- demo of the attribute --></Slot>
      </Use>
    </Slot>
  </Use>
</Fragment>
```

**Why this is non-obvious.** A first pass at a docs deck usually
hand-writes every attribute row. The reference deck instead drives
the table from a JSON literal inside `<Foreach>` and dispatches the
"required" decoration via `<Choose>`. The result: adding an
attribute to the docs is a one-line edit in the JSON, not a hand
edit of two or three places.

**Section template tones.** The seven `section-{tone}` variants
share an identical shape but differ in palette. They map 1:1 to a
chapter's editorial mood:

| Tone      | Used by chapters (example deck) |
| --------- | ------------------------------- |
| `forest`  | 02-text                         |
| `cream`   | 03-list                         |
| `mustard` | 04-image                        |
| `peach`   | 05-icon-svg                     |
| `mint`    | 06-shape-line, 15-master-notes  |
| `dark`    | 07-table, 14-templates-styles   |
| `coral`   | 08-chart, 13-recipes            |

Rotating tones across chapters keeps a 17-chapter document visually
varied without bespoke per-chapter art direction. Reuse the same
spread when porting to your own DSL — pick 5–7 palette tones up
front, give each a `section-{tone}` template, then let chapter
order alternate between them.

**When to reach for this pattern.** When you have 5+ "elements"
(grammar nodes, API endpoints, design tokens, etc.) that each need
the same shape of page (cover → attributes → demo) and you want a
single authoring change to update both the metadata table and the
visual chrome. Below 5 elements, hand-authoring each chapter is
faster.

## Source files

| Recipe area        | Source file(s)                                                                                                                                                                   |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Master chrome (§1) | `playground-samples/{pitch,editorial,tech-spec,workshop}/templates/master.xml`                                                                                                   |
| Templates (§2)     | `playground-samples/{pitch,editorial,tech-spec,workshop}/templates/components.xml`; `examples/builder-reference/templates/{diagram,feature,page,section,code,layout,master}.xml` |
| Diagrams (§3)      | `examples/builder-reference/chapters/{09-diagrams-sequence,10-diagrams-hierarchy,11-diagrams-flow,12-diagrams-matrix}.xml`                                                       |
| Control flow (§4)  | `examples/builder-reference/chapters/16-control-flow.xml`                                                                                                                        |
| Editorial (§5)     | `playground-samples/editorial/{chapters,templates,styles}/*.xml`                                                                                                                 |
| Workflow (§6)      | every `main.sgx` + each deck's `styles/` and `templates/` directories                                                                                                            |
| Gotchas (§7)       | in-file comments at the cited paths                                                                                                                                              |
