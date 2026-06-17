# slideglance-parser

PPTX ZIP archive + OOXML XML parser — themes, slides, layouts, masters, charts, tables.

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project — published to crates.io once stable.

## What it does

Reads a `.pptx` byte stream and produces a
`slideglance_model::Presentation` with every slide / layout / master /
theme / chart / table parsed and typed. Two layers:

- **`PptxArchive`** — eagerly extracts XML / `.rels` / Content-Types as UTF-8 strings; exposes media files lazily through a per-call cache.
- **Typed parsers** — wrap `quick-xml`'s serde deserializer. XML namespace prefixes are stripped during parsing.

The crate handles the structural side; layout / master / theme
inheritance resolution lives at the `slideglance` (orchestrator) level.

## Status

Pre-release — APIs may change before 1.0.

## License

MIT — see [LICENSE](./LICENSE).
