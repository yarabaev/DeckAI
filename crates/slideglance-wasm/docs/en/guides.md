---
title: slideglance-wasm — Guides
lang: en
kind: guides
crate: slideglance-wasm
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/slideglance-wasm/src/lib.rs
---

# slideglance-wasm — Guides

## Build the WASM bundle locally

### Goal

Produce the `pkg/` directory that `@slideglance/core` consumes.

### Steps

```sh
wasm-pack build crates/slideglance-wasm --target bundler
```

The output lands in `crates/slideglance-wasm/pkg/`. The npm package
build (`pnpm --filter @slideglance/core build`) symlinks or copies
this directory.

### Expected result

A `pkg/` directory containing `slideglance_wasm_bg.wasm`,
`slideglance_wasm.js`, `package.json`, and `.d.ts` declarations.

## Add a new wasm-bindgen export

### Goal

Surface a new function from `slideglance` to JS hosts.

### Steps

1. Add the new public function to `crates/slideglance/src/`.
2. In `crates/slideglance-wasm/src/lib.rs`, re-export it inside a
   `#[wasm_bindgen]` shim that converts argument and return types
   across the JS boundary (typically `JsValue` for complex shapes,
   `String` / `f64` / `Vec<u8>` for primitives).
3. If the return type contains non-`Copy` Rust types, use
   `serde_wasm_bindgen::to_value` and ensure the type implements
   `serde::Serialize`.
4. Run `wasm-pack build crates/slideglance-wasm --target bundler` and
   verify the generated `.d.ts` carries the new export.
5. Update the corresponding `@slideglance/core` re-export in
   `packages/core/src/`.

### Expected result

`@slideglance/core` exposes the new function. The
[`@slideglance/core` reference](../../../../packages/core/docs/en/reference.md)
must be updated in the same PR.

## Diagnose a "wasm-pack rejected the manifest" error

### Goal

`wasm-pack build` fails with
`invalid type: map, expected a string for key package.license`.

### Cause

`wasm-pack`'s manifest parser does not understand cargo workspace
inheritance (`field.workspace = true`). The fix is to keep every
published-style metadata field on
`crates/slideglance-wasm/Cargo.toml` inline (no `workspace = true`)
and let `scripts/sync-versions.mjs` synchronise the version. The
sibling crate `slideglance-measure-wasm` follows the same pattern.

Do not "fix" this by enabling workspace inheritance — it will
re-break wasm-pack.
