# slideglance-model

OOXML PPTX semantic model: shapes, fills, text, tables, charts, themes, slides.

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project — published to crates.io once stable.

## What it does

Pure data types — the result of parsing a `.pptx` archive — used by the
renderer and other consumers. Construction is the parser's job
(`slideglance-parser`); this crate intentionally has no parsing logic
and depends only on `slideglance-utils` (unit types) and
`slideglance-color` (resolved colors).

Type design follows OOXML / ECMA-376 nomenclature with idiomatic Rust
adaptations:

- Tagged unions become enums with `#[serde(tag = "type", rename_all)]`.
- `string | null` becomes `Option<String>`.
- Length-typed values use `slideglance_utils::{Emu, Pt, HundredthPt}` instead of raw integers.
- Resolved colors come from `slideglance_color::ResolvedColor`.

## Status

Pre-release — APIs may change before 1.0.

## License

MIT — see [LICENSE](./LICENSE).
