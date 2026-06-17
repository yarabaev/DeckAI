# slideglance-measure-wasm

Standalone `wasm-bindgen` entry point for SlideGlance text measurement.

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project — packaged into [`@slideglance/measure`](../../packages/measure/) on npm.

## What it does

Exposes `slideglance-font`'s text-width / line-metrics math to
JavaScript hosts (chiefly upstream layout engines that share
measurement primitives with the renderer) without dragging in the
parser / renderer / `resvg` / `serde` weight that `slideglance-wasm`
carries.

Why a separate crate from `slideglance-wasm`:

- `slideglance-wasm` packages the full pipeline at ~5 MiB compressed.
- A measurement-only consumer needs none of that — measurement only depends on `slideglance-font`.
- Splitting the wasm boundary keeps that consumer's bundle ~10× smaller and lets the two ship on independent release cadences without forcing every measurement bump to ride with a parser bump.

Packaged into `@slideglance/measure` across the same three target
flavors as `slideglance-wasm`.

## Status

Pre-release — APIs may change before 1.0.

## License

MIT — see [LICENSE](./LICENSE).
