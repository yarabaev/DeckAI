---
title: slideglance-parser — Reference
lang: en
kind: reference
crate: slideglance-parser
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-parser/src/lib.rs
  - crates/slideglance-parser/src/archive.rs
  - crates/slideglance-parser/src/xml.rs
  - crates/slideglance-parser/src/presentation.rs
  - crates/slideglance-parser/src/slide/
  - crates/slideglance-parser/src/slide_layout.rs
  - crates/slideglance-parser/src/slide_master.rs
  - crates/slideglance-parser/src/theme.rs
  - crates/slideglance-parser/src/shape_geometry.rs
  - crates/slideglance-parser/src/fill.rs
  - crates/slideglance-parser/src/effect.rs
  - crates/slideglance-parser/src/blip_effect.rs
  - crates/slideglance-parser/src/chart.rs
  - crates/slideglance-parser/src/table.rs
  - crates/slideglance-parser/src/text_body/
  - crates/slideglance-parser/src/text_style.rs
  - crates/slideglance-parser/src/text_style_resolver.rs
  - crates/slideglance-parser/src/custom_geometry.rs
  - crates/slideglance-parser/src/geometry_formula.rs
  - crates/slideglance-parser/src/notes.rs
  - crates/slideglance-parser/src/raw_color.rs
  - crates/slideglance-parser/src/relationships.rs
  - crates/slideglance-parser/src/style_reference.rs
---

# slideglance-parser — Reference

## Crate layout

```
crates/slideglance-parser/
├── src/
│   ├── lib.rs                    # re-exports
│   ├── archive.rs                # PptxArchive (ZIP layer), ArchiveError
│   ├── xml.rs                    # parse_xml, strip_namespaces, XmlError
│   ├── relationships.rs          # .rels parser
│   ├── raw_color.rs              # OOXML color element parser
│   ├── presentation.rs           # presentation.xml
│   ├── theme.rs                  # theme/themeN.xml
│   ├── slide_master.rs           # slideMasterN.xml
│   ├── slide_layout.rs           # slideLayoutN.xml
│   ├── slide/                    # slideN.xml (subdirectory)
│   ├── notes.rs                  # notesSlideN.xml
│   ├── shape_geometry.rs         # <a:xfrm>, <a:prstGeom>
│   ├── custom_geometry.rs        # <a:custGeom>
│   ├── geometry_formula.rs       # <a:guide>, <a:gd> formula evaluation
│   ├── fill.rs                   # <a:solidFill>, <a:gradFill>, <a:blipFill>, …
│   ├── effect.rs                 # <a:effectLst>, <a:effectDag>
│   ├── blip_effect.rs            # <a:blipFill> image effects
│   ├── chart.rs                  # chart{N}.xml
│   ├── table.rs                  # <a:tbl> element
│   ├── text_body.rs              # <a:txBody>
│   ├── text_style.rs             # text style lookups + theme font resolution
│   ├── text_style_resolver.rs    # paragraph/run inheritance chain
│   └── style_reference.rs        # shape style preset resolution
└── Cargo.toml
```

## Public items

### Archive layer

- `pub struct PptxArchive` — owning view over a parsed ZIP. Cheap to
  hand to per-part parsers.
- `pub enum ArchiveError` — ZIP / file-not-found / invalid relationships.

### XML primitives

- `pub fn parse_xml(bytes: &[u8]) -> Result<…, XmlError>`
- `pub fn strip_namespaces(...)` — convenience for tests
- `pub enum XmlError`

### Per-part parsers

| Function | Produces |
|---|---|
| `parse_presentation` | `PresentationInfo` + relationship map |
| `parse_theme` | `Theme` |
| `parse_slide_master` | `SlideMaster` |
| `parse_slide_layout` | `SlideLayout` |
| `parse_slide` | `Slide` |
| `parse_notes_text` | Notes slide body |

### Element parsers

`parse_geometry`, `parse_transform`, `parse_fill`, `parse_outline`,
`FillParseContext`, `parse_effect_list`, `parse_blip_effects`,
`parse_chart`, `parse_table`, `parse_text_body`, `parse_list_style`,
`parse_custom_geometry`.

### Style resolution helpers

- `resolve_theme_font(...)` — resolves `<a:latin typeface="+mj-lt"/>` etc.
- `apply_text_style_inheritance(...)` with `TextStyleContext`
- `resolve_shape_style(...)` returning `ResolvedStyleReference` /
  `FontReference`
- `evaluate_formula`, `evaluate_guides`, `resolve_value`, `GuideDefinition`

### Relationships helpers

`relationships::*` — relationship-id resolution and target lookup.
See [`src/relationships.rs`](../../src/relationships.rs) for the
exact re-export list.

## Error handling

Every parser returns `Result<T, E>` with a per-module error enum.
Most consumers wrap them into `slideglance::PptxError` via `?`.

## CJK script equality

Theme font extraction reads every CJK script entry equally — `Jpan`,
`Hang`, `Hans`, `Hant`. No script gets a special case; all four are
emitted into the resolved `FontScheme`. See the
[`docs/en/fonts.md` font scheme section](../../../../docs/en/fonts.md)
for the wider pipeline.
