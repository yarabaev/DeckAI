# What's not in the medium

slideglance targets `.pptx` files. Some visual treatments that work in
other media (browser decks, vector design tools, motion graphics) have
no analog in PowerPoint's file format or are intentionally outside the
builder grammar's scope. The correct response is to substitute, not to
fake.

This page is the **authoritative limitations list**. If a user asks
for one of these, give them the substitute and move on.

## Slide transitions and animations

| Not supported | What to do instead |
| --- | --- |
| Entry / exit animations via the grammar (`data-anim="fade-up"`-style) | Not in the builder. Static layout only. Add transitions in PowerPoint after export (Transitions tab) if needed. |
| Canvas effects (particle burst, knowledge graph, matrix rain, …) | Not in the medium. Replace with a static still that captures the same idea. |
| Slide-to-slide transitions (slide, fade, morph) | Not in the builder grammar. Add in PowerPoint post-export. |
| Click-by-click element reveal | Not in the grammar. Author each reveal stage as a separate slide. |

Reasoning: PPTX *does* support transitions and entrance animations at
the file format level, but the slideglance grammar targets static
visual layout. Authors who need motion add it in PowerPoint after the
deck is generated.

## Presenter / runtime features

| Not supported | What to do instead |
| --- | --- |
| Custom presenter UI (magnetic cards, draggable widgets) | PowerPoint's own presenter view does this. Speaker notes live in `<Notes>`. |
| Live theme cycling (`T` key) | Themes are baked into the `.sgx` at build time. Re-build with a different palette to swap. |
| Single-slide URL preview (`?preview=N`) | Use the **SlideGlance PPTX Viewer** VS Code extension; click any slide thumbnail to preview. |
| Cross-window sync (`BroadcastChannel`) | N/A — PowerPoint owns the presentation runtime. |

The compensating workflow:

1. Author `.sgx` with rich `<Notes>` (150–300 words per slide for talks).
2. Build → `.pptx`.
3. Open in PowerPoint / Keynote.
4. Use the host application's presenter view. Speaker notes appear as authored.

## Typography

| Not supported | What to do instead |
| --- | --- |
| True drop-caps (text flow around a large first letter) | **Lead-in idiom**: oversized accent-colored first sentence, body below. See `grammar.md` § Editorial idioms. |
| Mixed `vertical-align` on inline runs | `textVAlign="middle"` + `lineHeight="1.0"` idiom on every sibling. |
| Per-run / per-character tracking | `letterSpacing` is supported on text frames, lists, shapes, and table cells, but inline runs (`<Span>`, `<B>`, `<A>`, etc.) cannot set their own tracking. The text measurer also ignores tracking for wrap calculations, so large tracking can wrap later than expected. Use separate `<Text>` nodes per tracking style, and keep large tracking on short `noWrap` labels. |
| Variable-font axes (`wght`, `wdth`, slant) | Not exposed as variable axes. Use bold/italic plus separate static font files or faces when the exact axis value matters. |
| Custom OpenType features (`ss01`, `cv03`, alternates) | Not exposed. Pick a font face that ships the desired glyphs as the default. |
| Source-newline preserved in `<Text>` body | Not preserved. XML whitespace collapse turns multi-line source into one paragraph. Stack multiple `<Text>` for explicit breaks. |

## Color

| Not supported | What to do instead |
| --- | --- |
| CSS/Pom-style theme variables (`var(--accent-1)`-style references, `$accent`) | Builder does not resolve token expressions in XML attributes. Use `slideglance-theme.json` to choose the brand palette, then emit concrete hex values into `<Styles>`, `<Master>`, charts, and objects. |
| RGBA / `oklch` / HSL / named colors | 6-digit hex only. Convert beforehand. |
| Gradients on text (`background-clip: text`) | Not in the medium. Use a solid color, or render the gradient text as an `<Svg>` block (width capped at 1024). |
| Linear / radial / conic gradients on backgrounds | Not directly exposed on `<VStack>` etc. Wrap the gradient in an `<Svg>` block sized to the container, or use a pre-rendered PNG as `backgroundImage`. |
| PPTX theme tokens (`accent1`, `dk1`, …) | Not honored. 6-digit hex only. |
| Alpha (RGBA) on text | Not exposed. Use a lower-contrast solid color instead. |
| Alpha on shapes / backgrounds | Use `opacity` 0–1 on the element, or `fill.transparency` 0–1 on shape/master fills when only the fill should be transparent. Avoid relying on RGBA strings. |

## Layout / interaction

| Not supported | What to do instead |
| --- | --- |
| CSS Grid (`grid-template-areas`) | Flexbox via `<VStack>` / `<HStack>` and `<Layer>` for absolute positions. |
| `position: sticky` | N/A — PPTX has no scroll context. |
| Responsive / media-query layouts | One target slide size per deck. Re-export with a different `Document size` for a different aspect ratio. |
| `clip-path` / `mask` | Limited to rectangular image crop modes and built-in rounded corners. For non-rectangular clipping, pre-process the asset. |
| Hover / interactive states | N/A — static medium. |
| Per-deck JS runtime | N/A — the builder runs at build time, not in the recipient's deck. |
| CSS flex shorthand, `calc()`, `minmax()` | `flexGrow`, `flexShrink`, and `flexBasis` are supported, but CSS shorthand and formulas are not. Use explicit builder attributes with fixed, percentage, or `max` dimensions. |
| `alignItems="baseline"` | Not in the runtime enum. Use the tight-lineHeight + `textVAlign="middle"` idiom on row siblings. |

## File format

| Not supported | What to do instead |
| --- | --- |
| `<iframe>` embed | Not in the medium. Take a screenshot, embed as `<Image>`. |
| Scripted or custom hyperlink schemes | `<A href="...">...</A>` on inline runs is supported for allowed schemes (`https:`, `http:`, `mailto:`, `tel:` by default). `javascript:`, `vbscript:`, and unknown schemes are dropped unless explicitly allowlisted by the build environment. |
| Embedded video / audio | Not in the builder grammar. Add in PowerPoint after export. |
| Persistent state per deck (localStorage) | N/A. |

## When slideglance is the wrong tool

Use slideglance when:

- The artifact must be a `.pptx` file (corporate review, executive
  share, email attachment, Google Slides import).
- Recipients will edit slides themselves.
- The deck must print to PDF crisply, or export to images for
  documentation.
- Static visual layout is fine; animations and runtime are not part
  of the brief.

Pick a different medium when:

- The deck must run live in a browser tab with smooth animations.
- Pixel-exact CSS layout matters more than recipient editability.
- Canvas-FX effects are part of the spec.
- The deck is a web artifact (landing page, marketing site, talk
  site).
