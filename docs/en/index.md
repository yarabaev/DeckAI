---
title: SlideGlance — Workspace Overview
lang: en
kind: overview
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - Cargo.toml
  - package.json
  - README.md
---

# SlideGlance — Workspace Overview

> Languages: English · *(others pending)*

SlideGlance is a Rust + WebAssembly core that converts PowerPoint
`.pptx` files into SVG and PNG, plus a TypeScript layer that builds
`.pptx` files from declarative XML. The workspace ships across three
tiers: Rust crates, npm packages, and end-user applications.

This document is the entry point for engineering documentation. The
marketing-oriented [root README](../../README.md) is the user-facing
surface; everything under `docs/` is for contributors, integrators,
and reviewers.

## Layered architecture

```
slideglance-utils  → slideglance-color  → slideglance-model
                                              ↓
                                       slideglance-parser
                                              ↓
                              slideglance-font  →  slideglance-renderer
                                                          ↓
                                                  slideglance-png
                                                          ↓
                                slideglance (CLI + lib)  +  slideglance-wasm
```

Each crate depends only on layers below it. The boundary is enforced
by `cargo check --workspace --no-default-features` plus the
inspectable Cargo manifests. For the full picture see
[Architecture](./architecture.md).

## Distribution surfaces

| Surface | Crate / Package | Audience |
|---|---|---|
| `crates.io` library | [`slideglance`](../../crates/slideglance/docs/en/index.md) | Rust users |
| `crates.io` CLI | [`slideglance`](../../crates/slideglance/docs/en/index.md) | Shell users |
| npm core (WASM) | [`@slideglance/core`](../../packages/core/docs/en/index.md) | Web apps |
| npm viewer | [`@slideglance/viewer`](../../packages/viewer/docs/en/index.md) | Web apps |
| npm builder | [`@slideglance/builder`](../../packages/builder/docs/en/index.md) | Authors |
| Chrome extension | [`chrome-extension`](../../apps/chrome-extension/docs/en/index.md) | Browser users |
| Web playground | [`web-playground`](../../apps/web-playground/docs/en/index.md) | Browser users |
| Desktop viewer | [`desktop-viewer`](../../apps/desktop-viewer/docs/en/index.md) | Mac / Windows / Linux |
| VS Code extension | [`vscode-extension`](../../apps/vscode-extension/docs/en/index.md) | Authors in VS Code |
| Landing page | [`landing`](../../apps/landing/docs/en/index.md) | Public web |

See [Distribution](./distribution.md) for the per-channel status and
release pipeline.

## Where to go next

- [Architecture](./architecture.md) — layered model, data flow, design constraints
- [Fonts](./fonts.md) — text measurement, fallback, rendering pipeline
- [Distribution](./distribution.md) — what ships where, build provenance
- [Crates](./crates.md) — every Rust crate
- [Packages](./packages.md) — every npm package
- [Apps](./apps.md) — every end-user app
