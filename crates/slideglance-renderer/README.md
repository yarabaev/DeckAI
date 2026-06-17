# slideglance-renderer

PPTX model → SVG renderer. Deterministic, supports both text-mode and path-mode output.

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project — published to crates.io once stable.

## What it does

Takes a `slideglance_model::Slide` (or full `Presentation`) and emits
SVG bytes. The slide-level entry point `slide::render_slide_to_svg`
composes element-level renderers (shapes, fills, text, tables, charts,
images) on top of low-level building blocks (`svg_builder`,
`transform`, `viewbox`, `slide_context`).

Determinism contract: same input + same options → bitwise-identical
SVG bytes. No system clock, no randomness, no unordered iteration in
render paths. The renderer never reads system fonts directly — it
consumes pre-loaded font byte buffers from `slideglance-font`.

Output is consumed by `slideglance-png` (rasterization) and the
JS-side viewer (direct DOM injection).

## Status

Pre-release — APIs may change before 1.0.

## License

MIT — see [LICENSE](./LICENSE).
