---
name: slideglance-pptx
description: Author editable PowerPoint decks (.pptx) declaratively from a small XML grammar (.sgx). Use when the user asks for a presentation, ppt, slides, deck, keynote, pitch, weekly report, tech sharing, lecture deck, or any multi-slide artifact intended for offline review, sharing, or further editing in PowerPoint / Keynote / Google Slides. The build pipeline runs locally (no upload, no SaaS), the output is a real editable .pptx (not screenshot images), and layout decisions use OpenType-aware text measurement so the rendered file wraps the way PowerPoint would.
triggers:
  - "ppt"
  - "pptx"
  - "slides"
  - "deck"
  - "presentation"
  - "keynote"
  - "pitch deck"
  - "weekly report"
  - "tech sharing"
  - "investor deck"
  - "lecture slides"
  - "프레젠테이션"
  - "슬라이드"
  - "발표자료"
  - "powerpoint"
example_prompt: "Build a 10-slide pitch deck about <topic>. Use slideglance-pptx. Before I scaffold, confirm audience and slide count, ask whether there is a brand source or existing slideglance-theme.json, then create or apply that theme before generating _styles.xml."
---

# slideglance-pptx — declarative .pptx authoring

Author **editable PowerPoint files** by writing a small XML grammar called
`.sgx`. The build pipeline (`@slideglance/builder`) compiles the XML to a
real `.pptx` that opens in PowerPoint / Keynote / Google Slides and stays
fully editable by the recipient.

This skill covers the **slideglance authoring surface** — grammar,
layout primitives, masters, styles, templates, control flow, speaker
notes, brand-theme files, and the lint-driven feedback loop. Theme
decisions are recorded in `slideglance-theme.json`; the
[`references/themes.md`](references/themes.md) file defines the theme
format, palette roles, font pairing rules, typography scale, and
spacing system used to generate `<Styles>`.

## What slideglance produces

`.pptx` files. Real, editable, deterministic. Recipients open the
output in PowerPoint / Keynote / Google Slides and can edit slides
themselves — no embedded JS runtime, no required browser, no SaaS.

Trade-offs of the medium (vs. browser-based decks):

- No CSS keyframe / canvas-FX animations and no custom JS runtime —
  motion lives in PowerPoint's own animations tab, added post-export
  if needed.
- Color tokens are 6-digit hex (no `accent1` theme tokens, no
  `var(--accent-1)` references).
- No drop-cap text-flow-around-shape; use the lead-in idiom instead
  (see [`references/limitations.md`](references/limitations.md)).
- Static visual layout only — animated reveals are authored as
  separate slides.

## When to use this skill

Triggers: any request for a presentation / ppt / slides / deck /
keynote / pitch / weekly report / tech sharing / lecture deck — **when
the deliverable should be a real editable `.pptx` file**.

Counter-triggers (do NOT use slideglance):

- "I want a browser-resident interactive presentation with smooth
  animations." — slideglance produces static `.pptx`, not a web deck.
- "I need a Google Slides file specifically." — slideglance produces
  `.pptx`; Google Slides will import it but lossily.
- "I want canvas-FX particle effects on slide entry." — not in the
  slideglance medium.

## Before you author anything — resolve the brand theme

**Do not start writing slides until you understand:**

1. **Content & audience.** Topic, slide count, who is watching (execs / engineers / investors / students / general)?
2. **Brand theme.** Before generating `_styles.xml`, look for
   `slideglance-theme.json` in the working deck directory. If it exists,
   read it and use it as the source of truth for tone, colors, fonts,
   spacing defaults, and master chrome. If it does not exist, create one
   first: derive it from attached resources when provided (brand images,
   website URLs, existing PPTX decks), otherwise choose a palette and
   font pairing from [`references/themes.md`](references/themes.md).
3. **Tone / visual direction.** Corporate / editorial / minimal /
   brutalist / pastel / cyber / academic? If the user does not specify a
   direction and no resources imply one, pick the closest reference theme
   from [`references/themes.md`](references/themes.md) and record the
   decision in `slideglance-theme.json`.
4. **Starting point.** Use [`examples/two-column.sgx`](examples/two-column.sgx)
   as the structural scaffold when appropriate; replace content and
   generate `_styles.xml` / `_master.xml` from `slideglance-theme.json`.

A good opening message:

> I can build this `.pptx` for you. Before I scaffold, three quick
> confirmations:
>
> 1. Topic / target slide count / audience?
> 2. Do you have a brand source I should use — logo/image, website,
>    existing PPTX, or an existing `slideglance-theme.json`?
> 3. Tone — clean corporate, magazine-soft editorial, technical dark,
>    pastel lifestyle, brutalist poster?
> 4. Want me to start from the two-column structural scaffold or
>    hand-author from scratch?

Only after those are clear should you scaffold.

## The grammar in 60 seconds

```xml
<?xml version="1.0" encoding="UTF-8"?>
<SlideGlance xmlns="urn:slideglance:builder:v1">
  <Document size="16:9" defaultMaster="CORP" fontFamily="Pretendard" />

  <Master name="CORP" backgroundColor="F8FAFC">
    <MasterRect x="0" y="0" w="1280" h="48" fill.color="0F172A" />
    <MasterText x="48" y="14" w="400" h="24" text="ACME Corp" color="FFFFFF" fontSize="14" />
    <SlideNumber x="1180" y="690" w="60" h="20" fontSize="10" color="64748B" />
  </Master>

  <Styles>
    <Style name="page" padding="80" padding.top="120" />
    <Style name="title" fontSize="40" bold="true" color="0F172A" />
    <Style name="muted" fontSize="18" color="64748B" />
  </Styles>

  <Slide>
    <VStack class="page" gap="12">
      <Text class="title">Q4 Highlights</Text>
      <Text class="muted">Three things that mattered.</Text>
    </VStack>
    <Notes>Open with the why before the numbers. The audience has seen the dashboards.</Notes>
  </Slide>
</SlideGlance>
```

- **Root**: `<SlideGlance>`. Always one.
- **Document** sets slide size, default master, default font.
- **Master** is reusable backdrop chrome (header / footer / page numbers).
- **Styles** are named attribute presets (`class="title"`).
- **Slide** accepts exactly one root child, almost always `<VStack>`, `<HStack>`, or `<Layer>`.
- **Notes** is the speaker-notes drawer. Audience never sees it.

For the full attribute table and visual node reference, see
[`references/grammar.md`](references/grammar.md).

## Core authoring principles

These principles apply to **every** slideglance deck. The master skill
enforces them in its grammar and lint catalog.

### 1 — Theme first, then generated styles

`slideglance-theme.json` is the brand contract for the deck. It is not
directly consumed by the builder; the authoring agent converts it into
`<Document>`, `<Master>`, `<Styles>`, and reusable template defaults.
Do not invent `_styles.xml` ad hoc before either reading or creating
this file.

The theme file carries:

- palette roles (`base`, `surface`, `ink`, `muted`, `accent`,
  `accent2`, `charts`);
- typography (`fontFamily`, optional `headingFontFamily`);
- spacing defaults (`outer`, `sectionGap`, `gap`, `cardPadding`);
- master defaults (background, page number, optional brand chrome);
- source metadata explaining whether the theme came from user input,
  a PPTX, a website, an image, or a reference palette.

Use the typography scale and spacing system defined in
[`references/themes.md`](references/themes.md). Keep the deck to the
named type levels (`display`, `title`, `heading`, `body`, `caption`)
and 8px-based spacing values unless a layout genuinely needs a one-off
measurement.

### 2 — Reusability first: split into files and import

When the deck reaches **2+ slides**, the default starting structure is
a multi-file layout, not one giant `.sgx`. Lift `<Styles>`, `<Templates>`,
`<Master>`, and each `<Slide>` into separate files and `<Import>` them
from a small `main.sgx`. This is not "for large decks only" — small decks
benefit from the discipline because the next deck reuses the parts.

```
deck/
├── main.sgx              # <SlideGlance>, <Document>, imports
├── slideglance-theme.json # brand source for generated styles/master
├── _styles.xml           # <Fragment> with <Styles>
├── _templates.xml        # <Fragment> with <Templates>
├── _master.xml           # <Fragment> with <Master>
└── slides/
    ├── _01-cover.xml
    ├── _02-summary.xml
    └── _03-deep-dive.xml
```

A single-file `.sgx` is acceptable **only** for a true one-slide deck
or a quick sketch. Anything else: split.

### 3 — Master slides over per-slide chrome

Every deck declares at least one `<Master>` and uses `defaultMaster=`
on `<Document>`. Header bars, footer text, page numbers, watermarks,
brand chrome — **always** live in `<Master>`, never repeated on every
`<Slide>`. Multiple masters (LIGHT / DARK / COVER / SECTION) are
cheap; declare them when the deck has chapter dividers.

If the deck inherits from a corporate template, pass that template's
bytes as `masterPptx` to `buildPptx` — the builder extracts the
template's background and uses it. Don't re-implement corporate chrome
inline.

### 4 — Styles + Templates: name everything that repeats

- **`<Styles>`** for any color / size / padding that appears in 2+ places.
  Per-node literal attributes are acceptable for genuine one-offs; the
  lint rule `HARDCODED_COLOR` warns when a literal hex appears in 4+ places.
- **`<Templates>` + `<Use>`** for any subtree pattern that appears in 2+
  places — cards, timeline rows, KPI tiles, agenda items, section
  dividers. Pair with `<Foreach>` when iterating over data.
- **`<Slot>`** when a template needs paragraph-length or multi-element
  content; placeholder substitution is for short attribute values only.

### 5 — Let layout do the work: no hardcoded sizes or positions

Hardcoded `x` / `y` / `w` / `h` is the **last resort**, not the first move.

Preferred, in order:

1. **Flex containers** — `<VStack>` / `<HStack>` with `gap`, `padding`,
   `alignItems`, `justifyContent`. Children size themselves to content.
2. **Flex sizing primitives** — `w="50%"`, `flexGrow="1"`, `w="max"`.
   Percentages and grow-shares scale with the container.
3. **`<Master>` margins** — `<Master margin="48" margin.top="120">` defines
   the slide's content area inset; slide bodies render inside it.
4. **`<Layer>` with `x` / `y`** — only when you genuinely need overlapping
   elements at arbitrary positions: diagrams, infographics, freely composed
   scenes. Pixel-perfect numeric layout is acceptable here because the
   composition has no natural flow.

Anti-patterns:

- Fixed pixel widths (`w="240"`) on every column instead of `w="50%"` /
  `flexGrow`.
- Repeated `<Shape x="…" y="…">` rows that should be a `<Foreach>`.
- `<Layer>` used to "fix" a layout that Flex would handle.

### 6 — Trust autofit, then promote to a real overflow fix

Build with autofit enabled (default). If the deck overflows, the
builder shrinks in this order: row heights → font sizes → gaps /
padding → uniform scale down to 0.5×. Treat the resulting auto-fit
diagnostic (`AUTOFIT_OVERFLOW`) as a signal to **edit the source**, not
to disable autofit. Split the slide, trim the content, or promote a
single-column layout to two columns.

Disable autofit (`autoFit: false`) only when pixel-exact reproducibility
matters more than fit — typically for printable handouts where the
page count is fixed.

## Other authoring rules

1. **Speaker notes go in `<Notes>`, never on the slide.** Audience-facing
   slides contain only audience-facing content (titles, body, data,
   images). Anything that starts with "as you can see" / "what I want to
   highlight" / "during the demo I'll show" belongs in `<Notes>`.
2. **Line breaks inside `<Text>` — `\n` or a stack, not `<Br>`.** Source
   newlines and indentation inside a `<Text>` body collapse to a single
   space; same-line whitespace is preserved; leading / trailing
   whitespace is trimmed. To force a deliberate break, use either a
   `\n` literal in the body (`<Text>line 1\nline 2</Text>`) or a stack
   of single-line `<Text>` inside a `<VStack gap="0">` when each line
   needs its own style. `<Br/>` is silently dropped. See
   [`references/grammar.md`](references/grammar.md) §"Multi-line
   headlines and line breaks".

## Build, render, review

The contract is: **a deck is not authored until you have a green PNG
in your hand**. The structural file count is not evidence.

```
┌─────────────┐   ┌──────────────┐   ┌─────────────┐   ┌──────────┐
│  Edit .sgx  │ → │  buildPptx + │ → │   render    │ → │  visual  │
│             │   │     lint     │   │   to PNG    │   │  review  │
└─────────────┘   └──────────────┘   └─────────────┘   └──────────┘
                       ↓ errors?           ↓ wrong?         ↓ wrong?
                       └──── back to .sgx ─┘                │
                                                            │
                              ↑─────────────────────────────┘
```

Treat **every** diagnostic as a bug. `lint.ruleset = "recommended"`
catches overflow, baseline misalignment, low contrast, font-family
drift, and hardcoded-color repetition before they become "why does
this look broken" review cycles.

The concrete pipeline — `buildPptx` invocation, `<Import>` resolution,
diagnostic severity policy, CLI PNG rendering with `--font` flags, and
VS Code live preview — lives in
[`references/build-and-render.md`](references/build-and-render.md).
Lint rules and autofix patterns live in
[`references/lint.md`](references/lint.md). When a `.sgx` file fails
to parse, the common error catalog is in
[`references/schema-gotchas.md`](references/schema-gotchas.md).

## Theme variants

If you want a specific design language, create or update
`slideglance-theme.json` first, then generate `_styles.xml` and
`_master.xml` from it. The full theme-file format, brand-source
extraction rules, font guidance, typography scale, and spacing system
are in [`references/themes.md`](references/themes.md).
For schema gotchas while customizing, consult
[`references/schema-gotchas.md`](references/schema-gotchas.md). For
medium-level features that are not in scope (drop-caps,
keyframe-style entrance animations, etc.), see
[`references/limitations.md`](references/limitations.md) for the
recommended workaround.

## For contributors (working on the slideglance codebase itself)

This skill primarily teaches **authors** how to write `.sgx` and
build `.pptx`. If you are working on the slideglance source tree
(`packages/builder/`, `apps/vscode-extension/`, etc.) rather than
authoring decks, see [`references/development.md`](references/development.md)
for the contributor handbook — builder pipeline architecture, the
Feature Addition Checklist, key internal types, text-measurement
internals, PNG preview workflow, and the VS Code extension build
matrix. The project-level `CLAUDE.md` points contributors at this
same file.

## File structure

```
slideglance-pptx/
├── SKILL.md                  (this file)
├── references/
│   ├── grammar.md            (full XML reference: visual nodes, attributes, containers)
│   ├── composition.md        (Styles, Templates, Master, Import, If / Choose / Foreach)
│   ├── layouts.md            (idiomatic layout recipes — cover, title-body, two-column, KPI grid, timeline, etc.)
│   ├── recipes.md            (patterns, tips, idioms distilled from the runnable example decks)
│   ├── themes.md             (palette and font guidance — pick one and apply via <Styles>)
│   ├── limitations.md        (what is NOT in the medium and the recommended substitutes)
│   ├── lint.md               (lint rules + autofix patterns)
│   ├── schema-gotchas.md     (parse errors, attributes that don't exist, dot-notation forms)
│   ├── build-and-render.md   (buildPptx invocation, CLI PNG rendering, VS Code live preview)
│   └── development.md        (contributor reference for working on the builder / vscode-extension)
└── examples/
    ├── minimal.sgx           (single-slide hello world)
    └── two-column.sgx        (HStack 50/50 with title and body, a common layout starting point)
```

For richer end-to-end starting points beyond the two scaffolds above,
read the runnable decks in the workspace itself:

- [`examples/builder-reference/`](../../examples/builder-reference/) —
  every node type + every composition primitive across 17 chapters.
  Use as the source-of-truth when porting an idiom.
- [`examples/playground-samples/`](../../examples/playground-samples/) —
  four scenario decks (`pitch`, `editorial`, `tech-spec`, `workshop`),
  each with its own master chrome, palette, and template library. The
  fastest way to bootstrap a new deck is to copy one of these and
  swap content / palette.

[`references/recipes.md`](references/recipes.md) distils the recurring
patterns from both trees into a single cookbook.
