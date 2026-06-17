# testing/fixtures/fonts

Test fonts loaded by font-driven tests from a stable path
(`testing/fixtures/fonts/<file>`) without depending on the host's
installed font catalog — that determinism rule matches the production
policy in `crates/slideglance-png/src/lib.rs`.

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project.

## What lives here

The repo ships **no fonts by default**. A fixture is just a TTF / OTF
/ TTC file. Permissively-licensed candidates that have historically
worked here:

- **DejaVu Sans** (`DejaVuSans.ttf`) — Bitstream Vera + DejaVu permissive licenses. Covers extended Latin and basic CJK. <https://dejavu-fonts.github.io/License.html>.
- Any **SIL Open Font License** font (ship `OFL.txt` alongside).
- **Apache-2.0** / **MIT** licensed fonts (Inter, Roboto, etc.).

Avoid copyrighted system fonts (Apple SD Gothic Neo, Microsoft Yi
Baiti, etc.) — those are licensed for OS use only and cannot be
checked in.

## Existing call sites

Search for `testing/fixtures/fonts` in the workspace to find tests
that load from this directory. Without the matching file present they
panic at runtime, but the surrounding crate still compiles.
