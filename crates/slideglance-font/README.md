# slideglance-font

Font mapping, measurement, and shaping — OOXML font scheme + system / Google Fonts fallback chain.

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project — published to crates.io once stable.

## What it does

Owns everything between an OOXML font reference (theme tokens or
text-run `typeface` strings) and a renderable glyph: chain resolution,
metrics, fallback selection, shaping. Modules:

| Module          | Role                                                                                                   |
| --------------- | ------------------------------------------------------------------------------------------------------ |
| `mapping`       | `DEFAULT_FONT_MAPPING` table + case-insensitive + full-width-normalized lookup.                        |
| `cjk_fallback`  | Per-OS preinstalled CJK fallback chains for Japanese / Korean / Chinese (Simplified + Traditional). All four CJK scripts are treated equally per the project's CJK Script Equality rule. |
| `system_fonts`  | (Feature `system-fonts`.) Node-only scanner for installed faces; opt-in to keep the WASM bundle small. |
| `metric_match`  | (Feature `metric-match`.) OSS metric-compatible fallback chooser, depends on `font-kit`.               |

Consumed by `slideglance-renderer` (text path), the standalone
`slideglance-measure-wasm` (text-only WASM measurement), and the
JS-side `@slideglance/measure` package.

## Status

Pre-release — APIs may change before 1.0.

## License

MIT — see [LICENSE](./LICENSE).
