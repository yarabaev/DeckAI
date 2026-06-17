# slideglance

End-to-end PPTX → SVG / PNG conversion: parser + model + renderer + rasterizer + CLI binary. The native top-level entry point of the SlideGlance Rust workspace.

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project — published to crates.io once stable.

## What it does

Composes the lower-level crates into a single orchestrator:

```
PPTX bytes
  → slideglance-parser      (ZIP + XML)
  → slideglance-model       (typed data)
  → slideglance-renderer    (SVG)
  → slideglance-png         (PNG)
```

The library entry point `parse_pptx` returns a fully resolved
`Presentation` with every slide already merged with its layout / master
inheritance and text-style chain. `convert_to_svg` and `convert_to_png`
go straight from PPTX bytes to rendered output in one call.

A CLI binary ships under the same crate name for batch conversion and
ad-hoc inspection (`slideglance convert / render / inspect`).

## Install

```toml
[dependencies]
slideglance = "..."   # once published
```

```sh
cargo install slideglance   # CLI binary, once published
```

For now, build from source against the workspace.

## Quick start

```rust
use slideglance::{convert_to_svg, ConvertOptions};

let bytes = std::fs::read("deck.pptx")?;
let svgs = convert_to_svg(bytes, &ConvertOptions::default())?;
for (i, svg) in svgs.iter().enumerate() {
    std::fs::write(format!("slide-{i}.svg"), svg)?;
}
```

PNG conversion (`convert_to_png`) requires a `FontResolver`. See
[`docs/fonts.md`](../../docs/fonts.md) at the workspace root for the
font-priority chain and per-environment guidance.

## Status

Pre-release — APIs may change before 1.0.

## License

MIT — see [LICENSE](./LICENSE).
