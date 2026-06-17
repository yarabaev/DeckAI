# slideglance-color

OOXML theme color resolution and color transforms (`lumMod`, `lumOff`, `tint`, `shade`, `alpha`).

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project — published to crates.io once stable.

## What it does

Implements ECMA-376 §20.1.2.3 (color transforms) and §20.1.6 (theme
color schemes). Pure model-level — no XML parsing.

The parser (`slideglance-parser`) constructs `ColorRef` values from
`<a:srgbClr>` / `<a:schemeClr>` / `<a:hslClr>` / `<a:sysClr>` /
`<a:prstClr>` and feeds them through `ColorResolver` to obtain the
final `ResolvedColor` (RGB + alpha) consumed by the renderer.

Color spaces supported: sRGB (8-bit per channel) and HSL (per W3C CSS
Color Module Level 3 §4.2.4).

## Status

Pre-release — APIs may change before 1.0.

## License

MIT — see [LICENSE](./LICENSE).
