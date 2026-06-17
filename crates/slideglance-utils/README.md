# slideglance-utils

Unit-aware primitives (`Emu`, `Pt`, `HundredthPt`) and conversion helpers for OOXML coordinate systems.

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project — published to crates.io once stable.

## What it does

The lowest layer of the workspace dependency graph. PPTX mixes several
length units that must never be added or compared without conversion:
EMU (English Metric Units, 914,400 per inch), points (`Pt`), and
hundredths of a point (used for paragraph spacing per ECMA-376
§20.1.2.1).

This crate exposes zero-cost newtypes around `i64` / `f64` so unit
confusion fails at compile time, plus conversion helpers
(`Emu::to_pt()`, `Pt::to_emu()`, etc.).

Every other slideglance crate depends on this one — passing raw
numbers across module boundaries is a workspace-wide lint violation.

## Status

Pre-release — APIs may change before 1.0.

## License

MIT — see [LICENSE](./LICENSE).
