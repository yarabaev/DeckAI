# slideglance-emf

EMF / WMF raster-embed extraction — DIB → PNG conversion of bitmap-wrap metafiles.

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project — published to crates.io once stable.

## What it does

PowerPoint frequently wraps a single bitmap (a screenshot, an exported
chart, a pasted picture) inside an EMF or WMF metafile. The metafile
contains exactly one bitmap-bearing record:

- **EMF**: `EMR_STRETCHDIBITS`, `EMR_BITBLT`, `EMR_STRETCHBLT`, `EMR_SETDIBITSTODEVICE`, `EMR_TRANSPARENTBLT`.
- **WMF**: `META_DIBBITBLT`, `META_DIBSTRETCHBLT`, `META_STRETCHDIB`.

The Device-Independent-Bitmap payload is reassembled into a standard
`.bmp` file and converted to PNG.

True vector EMF / WMF (path / line / draw commands) is **not** handled
— callers should fall back to a placeholder rect when this crate
returns `None`. The strategy intentionally trades a small subset of
capability for zero external dependencies and a determinable scope.

## Status

Pre-release — APIs may change before 1.0.

## License

MIT — see [LICENSE](./LICENSE).
