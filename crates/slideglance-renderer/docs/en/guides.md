---
title: slideglance-renderer — Guides
lang: en
kind: guides
crate: slideglance-renderer
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-renderer/src/slide/
  - crates/slideglance-renderer/src/text/
  - crates/slideglance-renderer/src/id_gen.rs
---

# slideglance-renderer — Guides

## Render a single slide to SVG

### Goal

Take a parsed `Slide` and produce one SVG string.

### Code

```rust,no_run
use slideglance_renderer::render_slide_to_svg;
# use slideglance_model::Slide;
# fn example(slide: &Slide) -> Result<(), Box<dyn std::error::Error>> {

let svg = render_slide_to_svg(slide, &Default::default())?;
std::fs::write("slide.svg", &svg)?;
# Ok(()) }
```

### What's happening

`render_slide_to_svg` constructs an `IdGen` for the slide,
walks `slide.elements`, dispatches each through `render_shape` /
`render_connector` / `render_image` / `render_table` /
`render_chart`, and merges every fragment + `<defs>` + filter chain
into a single document. The output is deterministic — the same slide
+ same context always produces the same byte stream.

## Force path-mode text rendering

### Goal

The destination is PNG and the rasterizer must not depend on
installed fonts.

### Code

```rust,no_run
use slideglance_font::{RenderMode, TextEngineBuilder};
use slideglance_renderer::render_slide_to_svg;
# fn example(slide: &slideglance_model::Slide) -> Result<(), Box<dyn std::error::Error>> {

let text_engine = TextEngineBuilder::new()
    .render_mode(RenderMode::Path)
    .build(/* resolver, measurer, … */);

let mut ctx = slideglance_renderer::slide_context::SlideContext::default();
ctx.text_engine = Some(std::sync::Arc::new(text_engine));

let svg = render_slide_to_svg(slide, &ctx)?;
# Ok(()) }
```

### What's happening

With `RenderMode::Path`, every text run is rasterised to outline
paths during SVG generation. The resulting SVG no longer carries
`<text>` elements, so any rasterizer (including `slideglance-png`)
reproduces the pixels without needing font files at the rasterizer
stage.

## Add a new preset shape

### Goal

Support an OOXML preset geometry the renderer does not yet
recognise (e.g. a new flowchart shape).

### Steps

1. Locate the OOXML preset name (e.g. `flowChartManualOperation`).
2. Add the geometry definition in
   `crates/slideglance-renderer/src/geometry/preset.rs` — the path
   commands plus the formula evaluator inputs.
3. Add a unit test exercising the new preset.
4. Add a VRT case under `testing/vrt/snapshot/`.
5. Run `cargo test --workspace` and the VRT update script.

### Expected result

`render_geometry` returns a non-empty path for the new preset and
the VRT reference image matches the spec output.
