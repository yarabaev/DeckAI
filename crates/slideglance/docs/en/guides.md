---
title: slideglance — Guides
lang: en
kind: guides
crate: slideglance
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance/src/convert/
  - crates/slideglance/src/doc.rs
  - crates/slideglance/src/font_usage.rs
---

# slideglance — Guides

## Convert every slide of a deck to SVG

### Goal

Write each slide of `deck.pptx` to a numbered SVG file.

### Code

```rust,no_run
use slideglance::{convert_to_svg, ConvertOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("deck.pptx")?;
    let svgs = convert_to_svg(bytes, &ConvertOptions::default())?;
    for (i, svg) in svgs.iter().enumerate() {
        std::fs::write(format!("slide-{:03}.svg", i + 1), svg)?;
    }
    Ok(())
}
```

### What's happening

`convert_to_svg` runs the full pipeline: parser → model → renderer.
With `ConvertOptions::default()` the renderer is in text-mode and
will reference authored font families by name without embedding
glyph paths. For path-mode rendering (required when the consumer
cannot resolve the fonts), set `options.render_mode =
RenderMode::Path`.

## Convert one slide to PNG with embedded fonts

### Goal

Produce a PNG that renders correctly without any system font
dependency.

### Code

```rust,no_run
use slideglance::{convert_to_png, ConvertOptions, FontData};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("deck.pptx")?;

    let mut opts = ConvertOptions::default();
    opts.fonts.push(FontData::new(std::fs::read("NotoSans-Regular.ttf")?));
    opts.fonts.push(FontData::new(std::fs::read("NotoSans-Bold.ttf")?));

    let pngs = convert_to_png(bytes, &opts)?;
    std::fs::write("slide-1.png", &pngs[0])?;
    Ok(())
}
```

### What's happening

`convert_to_png` runs `convert_to_svg` then passes the SVGs through
`slideglance-png::svg_to_png` with the same font buffers. The PNG
pipeline does not load system fonts, so every font referenced by the
deck must be in `opts.fonts` for text to render correctly.

## List every typeface used in a deck

### Goal

Audit which font families the deck relies on, before rendering.

### Code

```rust,no_run
use slideglance::{build_typeface_usage, parse_pptx};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("deck.pptx")?;
    let presentation = parse_pptx(bytes)?;
    let usage = build_typeface_usage(&presentation);
    for (family, count) in &usage.runs_by_family {
        println!("{family:30} {count} runs");
    }
    Ok(())
}
```

### What's happening

`build_typeface_usage` walks every paragraph default, every run, and
every theme font scheme entry and counts references per family.
`extract_referenced_font_families(&bytes)` is a faster shortcut
when you only need the list (no counts).

## Use `PptxDocument` for incremental rendering

### Goal

A viewer renders one slide at a time on demand.

### Code

```rust,no_run
use slideglance::{PptxDocument, SlideRenderOptions};
use slideglance::ConvertOptions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("deck.pptx")?;
    let doc = PptxDocument::new(bytes, ConvertOptions::default())?;

    for i in 0..doc.slide_count() {
        let rendered = doc.render_slide(i, &SlideRenderOptions::default())?;
        std::fs::write(format!("slide-{i}.svg"), &rendered.svg)?;
    }
    Ok(())
}
```

### What's happening

`PptxDocument::new` does the parse once. `render_slide(i, opts)` is
cheap on the second call for the same slide because intermediate
state (resolved theme, font resolver chain, viewbox) is cached
across calls.
