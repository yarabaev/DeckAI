---
title: slideglance-parser — Guides
lang: en
kind: guides
crate: slideglance-parser
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-parser/src/archive.rs
  - crates/slideglance-parser/src/presentation.rs
  - crates/slideglance-parser/src/theme.rs
  - crates/slideglance-parser/src/slide/
---

# slideglance-parser — Guides

## Open a PPTX archive and list slide parts

### Goal

Inspect the relationships of a `.pptx` without parsing every slide.

### Code

```rust,no_run
use slideglance_parser::{parse_presentation, PptxArchive};

let bytes = std::fs::read("deck.pptx")?;
let archive = PptxArchive::open(&bytes)?;
let info = parse_presentation(&archive)?;

for path in &info.slide_paths {
    println!("{path}");
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

### What's happening

`PptxArchive::open` walks the ZIP and resolves
`_rels/.rels` → `presentation.xml.rels` so each slide / theme / media
part can be looked up by relationship ID. `parse_presentation`
returns a `PresentationInfo` plus the ordered slide path list.

## Parse a single slide

### Goal

Read `ppt/slides/slide1.xml`, resolve theme / master / layout
inheritance.

### Code

```rust,no_run
use slideglance_parser::{
    parse_presentation, parse_slide, parse_slide_layout, parse_slide_master,
    parse_theme, PptxArchive,
};

let bytes = std::fs::read("deck.pptx")?;
let archive = PptxArchive::open(&bytes)?;
let info = parse_presentation(&archive)?;

let theme = parse_theme(&archive, &info.theme_path)?;
let master = parse_slide_master(&archive, &info.slide_master_paths[0], &theme)?;
let layout = parse_slide_layout(&archive, &info.slide_layout_paths[0], &master)?;
let slide = parse_slide(&archive, &info.slide_paths[0], &layout)?;

println!("{} elements", slide.elements.len());
# Ok::<(), Box<dyn std::error::Error>>(())
```

### What's happening

Slides inherit placeholders, fills, and text styles from their
layout, master, and theme in that order. Each `parse_*` function
threads the upstream parsed value so the downstream can resolve the
inheritance chain. For end-to-end parsing prefer the higher-level
`slideglance::parse_pptx` orchestrator which handles all of the above
plus the chart / notes / media branches.

## Add a new OOXML element parser

### Goal

Support a previously-ignored OOXML construct (e.g. a chart variant).

### Steps

1. Locate or add the corresponding type in `slideglance-model`.
2. Add a new function in the matching parser module (e.g.
   `crates/slideglance-parser/src/chart.rs`).
3. Register the function in
   `crates/slideglance-parser/src/lib.rs`'s `pub use` block.
4. Add a fixture under `testing/fixtures/` exercising the new
   element.
5. Add a unit test asserting the parsed value matches the expected
   model.
6. If the new element affects rendering, update VRT cases under
   `testing/vrt/snapshot/`.

### Expected result

`cargo test --workspace` is green; `cargo doc --no-deps --package slideglance-parser`
shows the new function; the corresponding
[`reference.md`](./reference.md) row is added in the same PR.
