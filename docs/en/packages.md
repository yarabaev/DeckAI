---
title: Packages
lang: en
kind: navigation
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/
---

# Packages

> Part of the [SlideGlance workspace](./index.md).

npm packages — the JavaScript / TypeScript layer that wraps the WASM
core and ships to consumer apps.

| Name | Purpose | Docs |
|---|---|---|
| [`@slideglance/core`](../../packages/core/docs/en/index.md) | Rust → WASM bindings, the runtime everything else depends on | [index](../../packages/core/docs/en/index.md) · [reference](../../packages/core/docs/en/reference.md) · [guides](../../packages/core/docs/en/guides.md) |
| [`@slideglance/measure`](../../packages/measure/docs/en/index.md) | Text-measurement-only WASM, for the builder layer | [index](../../packages/measure/docs/en/index.md) · [reference](../../packages/measure/docs/en/reference.md) · [guides](../../packages/measure/docs/en/guides.md) |
| [`@slideglance/viewer`](../../packages/viewer/docs/en/index.md) | React PPTX viewer component | [index](../../packages/viewer/docs/en/index.md) · [reference](../../packages/viewer/docs/en/reference.md) · [guides](../../packages/viewer/docs/en/guides.md) |
| [`@slideglance/builder`](../../packages/builder/docs/en/index.md) | Declarative XML → editable `.pptx` | [index](../../packages/builder/docs/en/index.md) · [reference](../../packages/builder/docs/en/reference.md) · [guides](../../packages/builder/docs/en/guides.md) |

## Dependency graph

```
@slideglance/core (WASM)
   ↑
@slideglance/viewer       @slideglance/builder
                                ↑
                          @slideglance/measure (WASM)
```

`@slideglance/core` and `@slideglance/measure` are produced by
`wasm-pack` against `crates/slideglance-wasm` and
`crates/slideglance-measure-wasm` respectively. See
[Distribution](./distribution.md#npm--public-packages) for the build pipeline.
