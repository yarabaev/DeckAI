---
title: slideglance-renderer — Reference
lang: en
kind: reference
crate: slideglance-renderer
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-renderer/src/lib.rs
  - crates/slideglance-renderer/src/slide/
  - crates/slideglance-renderer/src/shape.rs
  - crates/slideglance-renderer/src/text/
  - crates/slideglance-renderer/src/fill/
  - crates/slideglance-renderer/src/geometry/
  - crates/slideglance-renderer/src/chart/
  - crates/slideglance-renderer/src/table/
  - crates/slideglance-renderer/src/effects.rs
  - crates/slideglance-renderer/src/connector.rs
  - crates/slideglance-renderer/src/image.rs
  - crates/slideglance-renderer/src/color.rs
  - crates/slideglance-renderer/src/transform.rs
  - crates/slideglance-renderer/src/viewbox.rs
  - crates/slideglance-renderer/src/svg_builder.rs
  - crates/slideglance-renderer/src/id_gen.rs
  - crates/slideglance-renderer/src/blip_effects.rs
  - crates/slideglance-renderer/src/render_result.rs
  - crates/slideglance-renderer/src/slide_context.rs
  - crates/slideglance-renderer/src/error.rs
---

# slideglance-renderer — Reference

## Crate layout

```
crates/slideglance-renderer/
├── src/
│   ├── lib.rs              # re-exports
│   ├── slide.rs            # render_slide_to_svg
│   ├── slide_context.rs    # SlideContext, RenderConfig, …
│   ├── viewbox.rs          # SlideViewBox
│   ├── error.rs            # RendererError
│   ├── render_result.rs    # RenderResult
│   ├── id_gen.rs           # IdGen — deterministic id generator
│   ├── svg_builder.rs      # escape_xml_text, escape_xml_attr
│   ├── transform.rs        # build_transform_attr, build_object_name_attr
│   ├── color.rs            # color_hex, alpha_str
│   ├── shape.rs            # render_shape (dispatcher)
│   ├── connector.rs        # render_connector
│   ├── image.rs            # render_image, ImageRenderResult
│   ├── effects.rs          # render_effects, EffectResult
│   ├── blip_effects.rs     # render_blip_effects
│   ├── geometry/           # preset_geometry_svg, render_geometry
│   ├── fill/               # render_fill_attrs, render_markers, render_outline_attrs
│   ├── text/               # render_text_body (+ sub-modules)
│   ├── table/              # render_table, TableStylePreset, …
│   └── chart/              # render_chart, ChartRenderResult
└── Cargo.toml
```

## Top-level functions

| Function | Purpose |
|---|---|
| `render_slide_to_svg(slide, ctx) -> Result<String, RendererError>` | Entry point — one slide → one SVG string |
| `render_shape(shape, ctx) -> RenderResult` | Dispatch to the correct shape renderer |
| `render_connector(...)` | Line / arrow / curved connector |
| `render_image(...) -> ImageRenderResult` | `<image>` element with blip effects |
| `render_chart(...) -> ChartRenderResult` | Chart SVG fragment |
| `render_table(...) -> TableElementResult` | Table SVG fragment |
| `render_geometry(...)`, `preset_geometry_svg(...)` | Geometry → path |
| `render_effects(...) -> EffectResult` | `<filter>` chain |
| `render_blip_effects(...)` | Image-specific effect filters |
| `render_fill_attrs(...) -> FillAttrs`, `render_outline_attrs(...)` | Fill / stroke attribute strings |
| `render_markers(...) -> MarkerResult` | Arrow / marker `<defs>` |

## Top-level types

| Type | Purpose |
|---|---|
| `RendererError` | All renderer failure modes |
| `RenderResult` | Carries the SVG fragment + defs + ids the caller must merge |
| `SlideViewBox` | Slide viewBox geometry |
| `IdGen` | Deterministic id allocator — never random |
| `FillAttrs`, `MarkerResult`, `EffectResult`, `ImageRenderResult`, `ChartRenderResult`, `TableElementResult`, `TableStylePreset` | Per-feature result records |

## Helpers

- `escape_xml_text(s)`, `escape_xml_attr(s)` — XML escaping
- `build_transform_attr(transform)` — `transform=…` builder
- `build_object_name_attr(name)` — `aria-label` for element naming
- `color_hex(...)`, `alpha_str(...)` — color formatting

## Renderer configuration

The renderer takes a context value (often `SlideContext` or
`Default::default()` plus settings) covering: target render mode
(text vs path — fed through to `slideglance-font::TextEngine`), the
viewport, and font resolver. See [`src/slide_context.rs`](../../src/slide_context.rs).

## Determinism rules

- All ids flow through `IdGen` — never `Uuid`, never time.
- Iterate `BTreeMap`, never `HashMap`, when order is observable.
- Floats are formatted with a fixed precision (see
  `slideglance-utils::constants` for the precision constants).

For full text/sub-module exports (e.g. inside `text::`,
`fill::`, `geometry::`, `chart::`, `table::`), see
[`src/lib.rs`](../../src/lib.rs).
