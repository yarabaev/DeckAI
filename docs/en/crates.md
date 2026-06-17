---
title: Crates
lang: en
kind: navigation
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/
---

# Crates

> Part of the [SlideGlance workspace](./index.md).

Rust crates in dependency order — lowest layers first. Each row links
to the crate's full docs triple (index / reference / guides).

| Name | Purpose | Docs |
|---|---|---|
| [`slideglance-utils`](../../crates/slideglance-utils/docs/en/index.md) | Unit-aware primitives (`Emu`, `Pt`, `HundredthPt`) | [index](../../crates/slideglance-utils/docs/en/index.md) · [reference](../../crates/slideglance-utils/docs/en/reference.md) · [guides](../../crates/slideglance-utils/docs/en/guides.md) |
| [`slideglance-color`](../../crates/slideglance-color/docs/en/index.md) | Theme color resolution, HSL transforms | [index](../../crates/slideglance-color/docs/en/index.md) · [reference](../../crates/slideglance-color/docs/en/reference.md) · [guides](../../crates/slideglance-color/docs/en/guides.md) |
| [`slideglance-model`](../../crates/slideglance-model/docs/en/index.md) | Intermediate typed model | [index](../../crates/slideglance-model/docs/en/index.md) · [reference](../../crates/slideglance-model/docs/en/reference.md) · [guides](../../crates/slideglance-model/docs/en/guides.md) |
| [`slideglance-parser`](../../crates/slideglance-parser/docs/en/index.md) | ZIP + XML → model | [index](../../crates/slideglance-parser/docs/en/index.md) · [reference](../../crates/slideglance-parser/docs/en/reference.md) · [guides](../../crates/slideglance-parser/docs/en/guides.md) |
| [`slideglance-font`](../../crates/slideglance-font/docs/en/index.md) | Font measurement, mapping, shaping | [index](../../crates/slideglance-font/docs/en/index.md) · [reference](../../crates/slideglance-font/docs/en/reference.md) · [guides](../../crates/slideglance-font/docs/en/guides.md) |
| [`slideglance-renderer`](../../crates/slideglance-renderer/docs/en/index.md) | Model → SVG | [index](../../crates/slideglance-renderer/docs/en/index.md) · [reference](../../crates/slideglance-renderer/docs/en/reference.md) · [guides](../../crates/slideglance-renderer/docs/en/guides.md) |
| [`slideglance-png`](../../crates/slideglance-png/docs/en/index.md) | SVG → PNG (resvg) | [index](../../crates/slideglance-png/docs/en/index.md) · [reference](../../crates/slideglance-png/docs/en/reference.md) · [guides](../../crates/slideglance-png/docs/en/guides.md) |
| [`slideglance-emf`](../../crates/slideglance-emf/docs/en/index.md) | EMF / WMF metafile decoding | [index](../../crates/slideglance-emf/docs/en/index.md) · [reference](../../crates/slideglance-emf/docs/en/reference.md) · [guides](../../crates/slideglance-emf/docs/en/guides.md) |
| [`slideglance`](../../crates/slideglance/docs/en/index.md) | Native CLI + library entry point | [index](../../crates/slideglance/docs/en/index.md) · [reference](../../crates/slideglance/docs/en/reference.md) · [guides](../../crates/slideglance/docs/en/guides.md) |
| [`slideglance-wasm`](../../crates/slideglance-wasm/docs/en/index.md) | `wasm-bindgen` entry for the full pipeline | [index](../../crates/slideglance-wasm/docs/en/index.md) · [reference](../../crates/slideglance-wasm/docs/en/reference.md) · [guides](../../crates/slideglance-wasm/docs/en/guides.md) |
| [`slideglance-measure-wasm`](../../crates/slideglance-measure-wasm/docs/en/index.md) | Text-measurement-only WASM | [index](../../crates/slideglance-measure-wasm/docs/en/index.md) · [reference](../../crates/slideglance-measure-wasm/docs/en/reference.md) · [guides](../../crates/slideglance-measure-wasm/docs/en/guides.md) |

## Dependency graph

```
utils ← color ← model ← parser ← font ← renderer ← png
                                                     ↑
                                slideglance ────────┘
                                slideglance-wasm ────┘

slideglance-emf ← parser (image branch)
slideglance-measure-wasm ← font
```

Lower-layer crates MUST NOT depend on higher-layer crates. See
[Architecture](./architecture.md#layered-overview) for the full
diagram and the reasons each boundary exists.
