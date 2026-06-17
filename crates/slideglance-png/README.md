# slideglance-png

SVG → PNG rasterization — `resvg`-backed, deterministic.

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project — published to crates.io once stable.

## What it does

Wraps `resvg` / `usvg` / `tiny_skia` with a deterministic configuration
suitable for PPTX rendering. Project-wide pixel-equivalence settings:

- `usvg::TextRendering::GeometricPrecision`
- `usvg::ShapeRendering::GeometricPrecision`
- `usvg::ImageRendering::OptimizeQuality`
- `Database::load_system_fonts` is **never** called — `fontdb` is populated only from font byte buffers handed in by the caller, so output never depends on the host machine's installed faces.

This is what guarantees Rust-native ↔ WASM bit-equality.

Same input + same options + same fonts → bitwise-identical PNG bytes.

## Status

Pre-release — APIs may change before 1.0.

## License

MIT — see [LICENSE](./LICENSE).
