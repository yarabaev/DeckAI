# Themes and palettes

slideglance uses **6-digit hex colors without `#`**. PPTX theme tokens
(`accent1`, `dk1`, `lt1`, …) are intentionally not supported — the
deterministic-hex constraint locks deck output to the values the
source specifies.

This page defines the brand-theme workflow, the
`slideglance-theme.json` file format, and the palette / typography /
spacing rules used to generate `_styles.xml` and `_master.xml`.

## Brand theme workflow

Before writing `_styles.xml`, decide the deck theme and record it in
`slideglance-theme.json`.

1. **Look for an existing theme file.** If `slideglance-theme.json`
   exists in the deck directory, read it and use it as the source of
   truth. Do not re-pick colors or fonts unless the user explicitly
   asks to revise the theme.
2. **Extract from supplied resources.** If the user provides a logo,
   brand image, website URL, or existing PPTX, derive the palette,
   font direction, and master-background intent from that source and
   save the result to `slideglance-theme.json`.
3. **Pick a reference theme when no resource is supplied.** Choose one
   of the reference palettes below based on the audience, content, and
   requested tone. Record that choice in `source`.
4. **Generate `_styles.xml` and `_master.xml`.** The builder does not
   read `slideglance-theme.json` directly. The authoring workflow
   converts the JSON roles into `<Document>`, `<Master>`, `<Styles>`,
   and reusable templates.

## `slideglance-theme.json` format

Store the file next to `main.sgx`.

```json
{
  "name": "acme-corporate",
  "tone": "corporate",
  "colors": {
    "base": "F8FAFC",
    "surface": "FFFFFF",
    "ink": "0F172A",
    "muted": "64748B",
    "accent": "1D4ED8",
    "accent2": "3B82F6",
    "border": "CBD5E1",
    "success": "16A34A",
    "warning": "D97706",
    "danger": "DC2626",
    "charts": ["1D4ED8", "16A34A", "D97706", "DC2626"]
  },
  "typography": {
    "fontFamily": "Pretendard",
    "headingFontFamily": "Pretendard",
    "monoFontFamily": "IBM Plex Mono"
  },
  "spacing": {
    "outer": 64,
    "outerTop": 80,
    "sectionGap": 32,
    "gap": 16,
    "cardPadding": 24
  },
  "document": {
    "size": "16:9",
    "defaultMaster": "CORP"
  },
  "master": {
    "name": "CORP",
    "backgroundColor": "F8FAFC",
    "slideNumber": true,
    "brandLabel": "ACME Corp"
  },
  "source": {
    "type": "referencePalette",
    "reference": "Corporate clean"
  }
}
```

Field contract:

| Field | Meaning |
| --- | --- |
| `name` | Kebab-case theme name. |
| `tone` | Human-readable tone: `corporate`, `dark-technical`, `editorial`, `pastel`, `brutalist`, `academic`, etc. |
| `colors.base` | Slide background. Avoid pure white unless monochrome is intentional. |
| `colors.surface` | Cards, panels, table cells, elevated areas. |
| `colors.ink` | Primary text. Avoid pure black unless brutalist / monochrome is intentional. |
| `colors.muted` | Captions, metadata, secondary body. Must still pass readable contrast. |
| `colors.accent` | Primary emphasis: bars, section labels, icons, important numbers. |
| `colors.accent2` | Nearby secondary accent. Use for paired accents, not as an unrelated chart color. |
| `colors.border` | Hairlines, table borders, card outlines. |
| `colors.success` / `warning` / `danger` | Semantic colors for status and deltas. |
| `colors.charts` | 3-5 chart colors, with `accent` first when appropriate. |
| `typography.fontFamily` | Body font. Prefer bundled / measured fonts when possible. |
| `typography.headingFontFamily` | Heading font. Defaults to `fontFamily` when omitted. |
| `typography.monoFontFamily` | Code / technical labels. Optional. |
| `spacing` | 8px-based spacing defaults used by page styles and templates. |
| `document` | Defaults for `<Document>`. |
| `master` | Defaults for `_master.xml`; not a raw builder object. |
| `source` | Provenance: `manual`, `brandColor`, `website`, `image`, `masterPptx`, or `referencePalette`. |

All color values are 6-digit uppercase hex without `#`.

## Deriving a theme from sources

### Brand colors supplied directly

Use the first explicit brand color as `colors.accent`. If several
colors are supplied, use the rest as chart candidates, then derive
`base`, `surface`, `ink`, `muted`, `accent2`, and `border` from the
main color and requested tone.

### Existing PPTX

A `.pptx` is a zip archive. Inspect these files when available:

```bash
unzip -p input.pptx ppt/theme/theme1.xml
unzip -p input.pptx ppt/slideMasters/slideMaster1.xml
```

Map OOXML theme values conservatively:

| PPTX theme value | Theme role |
| --- | --- |
| `lt1` / slide master solid background | `colors.base`, `master.backgroundColor` |
| `lt2` | `colors.surface` |
| `dk1` | `colors.ink` |
| `dk2` | muted-color input |
| `accent1` | `colors.accent` |
| `accent2`-`accent6` | `colors.charts` candidates |
| major font | `typography.headingFontFamily` |
| minor font | `typography.fontFamily` |

If the PPTX has an image background or complex chrome, record the PPTX
path in `source` and use `masterPptx` during build when possible. Do
not try to manually redraw complex corporate templates unless the user
asks for it.

### Website or image

For a website, inspect `theme-color`, CSS custom properties
(`--brand`, `--primary`, `--accent`), logo colors, headers, and button
styles. For an image/logo, extract the dominant brand color and one
supporting color. Record the URL or image path in `source`.

### No source supplied

Pick a reference palette from this file. Choose based on audience and
content, not personal preference. Corporate reports default to
Corporate clean; engineering deep dives default to Dark technical or
Corporate clean; magazine / narrative decks default to Editorial
cream; workshops can use Coral / warm or Academic / blueprint.

## Palette quality rules

Use this role system for every deck:

| Role | Use |
| --- | --- |
| `base` | Whole-slide background. |
| `surface` | Cards, tables, panels, figure wells. |
| `ink` | Primary readable text. |
| `muted` | Secondary text, captions, slide numbers. |
| `accent` | Sparse emphasis. Keep total accent area below roughly 10% of a slide. |
| `accent2` | Paired accent in the same color neighborhood. |
| `border` | Hairlines, table borders, card outlines. |
| `charts` | Data series colors. |

Contrast checks:

| Pair | Minimum |
| --- | --- |
| `ink` on `base` | 7:1 preferred, 4.5:1 minimum |
| `ink` on `surface` | 4.5:1 |
| `muted` on `base` | 3:1 |
| `accent` on `base` | 3:1 when used for text or thin marks |

Do not use `colors.charts[1]` blindly as `accent2`. `accent2` should
look like a controlled companion to `accent`, while chart colors often
need broader hue separation.

## Building a palette into `<Styles>`

The recipe is the same for every theme: convert `slideglance-theme.json`
into concrete classes. SlideGlance does not currently have Pom-style
`$accent` token substitution, so `_styles.xml` must contain actual hex
values.

```xml
<Styles>
  <Style name="page"   padding="80" padding.top="120" backgroundColor="F8FAFC" />
  <Style name="title"  fontSize="44" bold="true" color="0F172A" lineHeight="1.05" />
  <Style name="h2"     fontSize="28" bold="true" color="0F172A" />
  <Style name="body"   fontSize="18" color="1F2937" lineHeight="1.5" />
  <Style name="muted"  fontSize="14" color="64748B" />
  <Style name="th"     fontSize="11" color="FFFFFF" bold="true" backgroundColor="0F172A" textAlign="center" />
</Styles>
```

Promote any color literal that appears 4+ times across a deck to its
own `<Style>` (the lint rule `HARDCODED_COLOR` will flag it).

Typical generated styles:

```xml
<Fragment>
  <Styles>
    <Style name="page" padding="64" padding.top="80" backgroundColor="F8FAFC" />
    <Style name="title" fontFamily="Pretendard" fontSize="36" bold="true" color="0F172A" lineHeight="1.1" />
    <Style name="heading" fontFamily="Pretendard" fontSize="20" bold="true" color="0F172A" />
    <Style name="body" fontFamily="Pretendard" fontSize="16" color="1F2937" lineHeight="1.5" />
    <Style name="caption" fontSize="12" color="64748B" />
    <Style name="eyebrow" fontSize="11" bold="true" color="1D4ED8" letterSpacing="0.16" />
    <Style name="card" padding="24" gap="8" backgroundColor="FFFFFF" border.color="CBD5E1" border.width="1" borderRadius="8" />
    <Style name="bg-accent" backgroundColor="1D4ED8" />
    <Style name="text-accent" color="1D4ED8" />
    <Style name="hairline" backgroundColor="CBD5E1" />
  </Styles>
</Fragment>
```

Typical generated master:

```xml
<Fragment>
  <Master name="CORP" backgroundColor="F8FAFC" margin="64" margin.top="80">
    <MasterText x="64" y="30" w="420" h="20" text="ACME Corp"
                fontSize="11" bold="true" color="64748B" />
    <SlideNumber x="1170" y="688" w="64" h="18" fontSize="10" color="64748B" />
  </Master>
</Fragment>
```

Keep `_styles.xml` and `_master.xml` deterministic. If the theme file
does not change, generated style names and values should not drift
between slides.

## Typography scale

Use a small fixed scale per deck. Do not invent new sizes on every
slide.

| Level | Size | Use |
| --- | --- | --- |
| `display` | 44-68, bold | Cover titles, section dividers, KPI numerals. |
| `title` | 28-40, bold | Slide titles. |
| `heading` | 18-22, bold | Card titles, subsection headings. |
| `body` | 14-18 | Paragraphs, bullets, table body. |
| `caption` | 10-12, muted | Metadata, source notes, slide numbers. |

Rules:

- A slide should usually use no more than three type levels.
- Body text uses `lineHeight="1.4"` to `lineHeight="1.55"`.
- Bold is for `display`, `title`, `heading`, and short emphasis only.
  Do not make entire paragraphs bold.
- Use `headingFontFamily` only for display/title/heading styles.
  Body, captions, tables, and labels should use `fontFamily`.
- For rows that mix small labels and larger body text, use
  `lineHeight="1.0"` and `textVAlign="middle"` on both siblings.

## Spacing system

Use 8px multiples. These values cover most decks:

| Token | Values | Use |
| --- | --- | --- |
| `xs` | 4 or 8 | Tight label/icon gaps. |
| `sm` | 12 or 16 | Text stacks, chip gaps. |
| `md` | 24 | Card padding, related blocks. |
| `lg` | 32 | Section gaps, card rows. |
| `xl` | 48 | Dense slide outer padding. |
| `2xl` | 64 | Default slide outer padding. |
| `3xl` | 80 or 96 | Covers / editorial pages. |

Defaults for `slideglance-theme.json`:

```json
"spacing": {
  "outer": 64,
  "outerTop": 80,
  "sectionGap": 32,
  "gap": 16,
  "cardPadding": 24
}
```

Rules:

- Keep outer padding consistent across body slides. Use master
  `margin` when every slide needs the same content inset.
- Nearby elements get small gaps; different sections get large gaps.
- Do not fill empty space by adding decoration. Increase hierarchy or
  let the slide breathe.
- If a bullet list exceeds 5 items or any item wraps beyond 2 lines,
  split the slide or switch to a two-column layout.

## Reference palettes

Each palette below is a **starting point**, not a fixed system. Pick
one, then iterate on the actual deck.

### Corporate clean (default business deck)

```text
Title       0F172A   slate-900
Body        1F2937   slate-800
Muted       64748B   slate-500
Primary     1D4ED8   blue-700
Success     16A34A   green-600
Warning     D97706   amber-600
Danger      DC2626   red-600
Info        0EA5E9   sky-500
Light bg    F8FAFC   slate-50
Border      CBD5E1   slate-300
```

### Dark technical (engineering / dev)

```text
Background  0B1020   near-black indigo
Surface     12172B   indigo-950
Title       E2E8F0   slate-200
Body        CBD5E1   slate-300
Muted       64748B   slate-500
Accent      7DD3FC   sky-300
Warm hl     FCA5A5   red-300
Mono        86EFAC   green-300
```

### Editorial cream (magazine / long-form)

```text
Paper       F8F3E8   warm cream
Surface     FFFEF8   pure cream
Ink         3A332B   coffee
Title       1B1410   espresso
Accent      9A2A1F   brick red
Soft        BFB59E   stone
Rule        D6CFB8   muted gold
```

Pair with a serif display family (`Georgia`, `Playfair Display`,
`Tiempos`) for titles and a sans family (`Inter`, `Pretendard`) for
body. The mixed-size row idiom in
[`grammar.md`](./grammar.md#editorial-idioms) is mandatory here.

### Macaron pastel (lifestyle / soft)

```text
Cream       FEF8F1
Peach       FFD8C2
Mint        CFEBD8
Sky         CBE2F0
Lilac       DAD0F0
Lemon       F4EAB8
Rose        F0CBD8
Ink         431407   warm brown
Sub         8B5A2B   sandstone
```

### Brutalist (high-contrast / poster)

```text
Bg          F2F0EA   raw paper
Ink         0A0908   true black
Hot         F35F1C   safety orange
Cool        1A2A6C   navy
Caution     FFD400   sign yellow
Rule        0A0908   true black (4px+)
```

Use heavy, narrow display fonts (`Inter Tight`, `Druk`, `Anton`),
oversize titles, and minimal padding.

### Safety / alert

```text
Bg          0A0908
Surface     1A1612
Hot         F35F1C
Caution     FFD400
Danger      DC2626
Ink         F8F3E8
Muted       9CA3AF
```

### Academic / blueprint

```text
Bg          0E2A47   deep blueprint blue
Ink         F8F3E8   cream
Hairline    7BA7CC   blueprint hairline
Accent      F4A261   ochre
Soft        D4E6F2   pale blueprint
```

### Coral / warm

```text
Bg          FFF1EA
Surface     FFE3D2
Coral       FF6B3D
Ink         2B1810
Sub         8B5A2B
Mint        CDE9D6   (accent secondary)
```

### Monochrome

```text
Paper       FFFFFF
Ink         0A0908
Mid         3F3F46
Soft        D4D4D8
Accent      0A0908   (the contrast itself is the accent)
```

Pair with a single typeface (Helvetica Now / Inter / GT America) and
heavy use of typographic hierarchy.

## Fonts

Bundled and measured exactly:

- **Pretendard** — Korean + Latin sans-serif. Recommended default.
- **Noto Sans JP** — Japanese sans-serif.

Anything else uses a heuristic measurer (CJK = 1em, alphanumeric =
0.5em) which can drift from PowerPoint's actual rendered width by a
few percent. PowerPoint resolves the actual font at render time on
the recipient's machine — so either install / embed your chosen
family, or stick to ubiquitous fallbacks (Arial, Helvetica, Georgia,
Times, Calibri).

When the deck must render byte-identically on recipients' machines,
embed the font via the master PPTX option (`masterPptx` in
`buildPptx`).

### Pairing recipes

| Mood | Display (titles) | Body |
| --- | --- | --- |
| Corporate clean | Pretendard / Inter | Pretendard / Inter |
| Editorial magazine | Playfair Display / Georgia | Inter / Pretendard |
| Tech / engineering | Inter / IBM Plex Sans | IBM Plex Mono (code) + Inter |
| Lifestyle / soft | Playfair Italic / Cormorant | Inter Light / Manrope |
| Brutalist / poster | Anton / Druk / Bebas Neue | Helvetica / Inter |
| Academic | Source Serif Pro / Crimson | Source Sans Pro / Inter |

Editorial decks should pair **exactly two** families (display + body).
The lint rule `INCONSISTENT_FONT` flags 3+.

## Anti-patterns

- **Don't use semi-transparent text colors for "muted" body.** Use a
  flat hex (e.g. `64748B`) instead of opacity. PowerPoint renders
  alpha-mixed text differently across versions.
- **Don't put real PPTX theme tokens (`accent1`, etc.) in `color="…"`.**
  They are not honored.
- **Don't pair three serif families.** Two is editorial; three is
  noise.
- **Don't hard-code colors in 10 places.** Lift to `<Styles>` after
  the third occurrence.
