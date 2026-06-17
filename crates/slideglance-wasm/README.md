# slideglance-wasm

`wasm-bindgen` entry point — the full PPTX pipeline exposed to JavaScript hosts.

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project — packaged into [`@slideglance/core`](../../packages/core/) on npm.

## What it does

Re-exports the `slideglance::parse_pptx` orchestrator behind a
`wasm-bindgen` ABI so JavaScript hosts can pass a `Uint8Array` of PPTX
bytes and receive a serialized `slideglance_model::Presentation` via
`serde-wasm-bindgen`.

Built once and packaged into `@slideglance/core` across three target
flavors (bundler / web / node) so consumers can pick the loader
matching their toolchain. See `scripts/build-wasm.sh` at the workspace
root for the build orchestration.

A measurement-only sister crate (`slideglance-measure-wasm`) ships
without the parser / renderer for consumers that only need text-width
math — its bundle is roughly 10× smaller.

## Status

Pre-release — APIs may change before 1.0.

## License

MIT — see [LICENSE](./LICENSE).
