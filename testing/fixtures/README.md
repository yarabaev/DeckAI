# testing/fixtures

PPTX (and supporting) test fixtures used by the workspace test suite.

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project.

## What lives here

This directory exists to give tests a stable path
(`testing/fixtures/...`) to load from. The repository ships **no
fixtures by default** — contributors drop their own `.pptx` files here
and the existing test scaffolding picks them up.

## Where fixtures are referenced

- **Workspace tests** — several `#[cfg(test)]` blocks under `crates/*/src/**` call `std::fs::read("testing/fixtures/<name>.pptx")` (or build the path via `concat!(env!("CARGO_MANIFEST_DIR"), …)`). Without the file present, those tests panic at runtime; the surrounding production code still compiles cleanly.
- **VRT runner** — `testing/vrt/snapshot/src/lib.rs` exposes `pub const CASES: &[VrtCase]` (currently empty). Append a `VrtCase { name, fixture: "testing/fixtures/<name>.pptx", slides }` entry to register a deck for snapshot regression coverage, then run `UPDATE_SNAPSHOTS=1 pnpm vrt` once to materialize the baseline.

## Suggested fixture sources

A fixture is just a `.pptx` (or `.docx` / `.xlsx` for cross-format
work). Keep them small and license-clear:

- Author a minimal deck in PowerPoint / Keynote / LibreOffice and save as `.pptx`.
- Strip embedded media you don't need — large bitmaps inflate the repo. `unzip` the deck, drop unneeded files under `ppt/media/`, re-zip.
- Avoid customer / proprietary content — CI logs and snapshot diffs surface anything visible in the slide.

## Subdirectories

| Path                              | Contents                                                                       |
| --------------------------------- | ------------------------------------------------------------------------------ |
| [`fonts/`](./fonts/README.md)     | TTF / OTF / TTC files used by font-driven tests.                               |
