---
title: Distribution Channels
lang: en
kind: overview
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - crates/
  - packages/
  - apps/
  - .github/workflows/
---

# Distribution Channels

Status snapshot of every artefact this monorepo produces and where it lands.
Cross-reference with `.github/workflows/` to see what's automated vs. manual.

> **Versioning state**: every published package and crate currently sits at
> `0.1.0`. No release has shipped to npm or crates.io yet. The 1.0.0 cut-over
> is gated on the XSD URL pin (`https://unpkg.com/@slideglance/builder@^1/builder.xsd`)
> embedded in every `.sgx` template — that URL only starts resolving after
> the first 1.x publish of `@slideglance/builder`.

---

## npm — public packages

All published from `packages/` under the `@slideglance` scope.

| Package | Path | Purpose | Status |
| --- | --- | --- | --- |
| `@slideglance/builder` | `packages/builder` | Declarative XML → PPTX builder (TypeScript) | **manual** — `pnpm --filter @slideglance/builder publish` (gates: `prepublishOnly` runs codegen:check + build + lint + fmt + typecheck + tests) |
| `@slideglance/core` | `packages/core` (= `crates/slideglance-wasm/pkg`) | WASM bindings to the Rust renderer | **manual** — wasm-pack writes `pkg/package.json`; publish from there |
| `@slideglance/measure` | `packages/measure` (= `crates/slideglance-measure-wasm/pkg`) | Text-measurement-only WASM (lighter than `core`) | **manual** — same wasm-pack flow |
| `@slideglance/viewer` | `packages/viewer` | React PPTX viewer shell | **manual** |

**Publish prerequisites** (per package):
- All checks green on `main`
- Version bumped in `package.json` (no automated changesets yet)
- `npm login` with publish permission on the `@slideglance` scope

**Gaps to close** (for the eventual 1.0.0 release):
- Add a `release` workflow that runs the per-package `prepublishOnly` script,
  publishes to npm with `--provenance`, and tags the commit.
- Switch to `changesets` (or `release-please`) so version bumps + CHANGELOG
  entries are PR-driven instead of manual.

---

## crates.io — Rust libraries

Workspace-wide version pinned in `Cargo.toml [workspace.package].version`.

> **Independence from npm**: publishing crates to crates.io is **not** required
> for the npm / GitHub Pages / VS Code / Chrome / desktop-viewer channels. The
> two wasm-bindgen crates are `publish = false` and feed npm via `wasm-pack`
> only; workspace dependencies are declared as
> `{ path = "crates/...", version = "0.1.0" }`, so `cargo build` /
> `wasm-pack build` resolve through `path` and never query the registry.
> crates.io exists so external Rust consumers can pull the renderer / parser /
> CLI via `cargo` — the WebAssembly distribution ships without it. The `^1`
> unpkg XSD pin (see CDN section) is likewise gated on **npm** 1.x, not
> crates.io.

| Crate | Path | Purpose | `publish` |
| --- | --- | --- | --- |
| `slideglance` | `crates/slideglance` | Native CLI + library entrypoint | `true` |
| `slideglance-color` | `crates/slideglance-color` | Theme colour resolution | `true` |
| `slideglance-emf` | `crates/slideglance-emf` | EMF (Windows metafile) decoding | `true` |
| `slideglance-font` | `crates/slideglance-font` | Font measurement / mapping / shaping | `true` |
| `slideglance-model` | `crates/slideglance-model` | Intermediate model | `true` |
| `slideglance-parser` | `crates/slideglance-parser` | ZIP + XML → model | `true` |
| `slideglance-png` | `crates/slideglance-png` | SVG → PNG (resvg) | `true` |
| `slideglance-renderer` | `crates/slideglance-renderer` | Model → SVG | `true` |
| `slideglance-utils` | `crates/slideglance-utils` | EMU/Pt newtypes, unit conversion | `true` |
| `slideglance-wasm` | `crates/slideglance-wasm` | wasm-bindgen entry for `@slideglance/core` | `false` (consumed by npm `core`) |
| `slideglance-measure-wasm` | `crates/slideglance-measure-wasm` | wasm-bindgen entry for `@slideglance/measure` | `false` (consumed by npm `measure`) |

**Publish order** (lower layers first, reversed dependency graph):
1. `slideglance-utils`
2. `slideglance-color`
3. `slideglance-model`
4. `slideglance-emf`
5. `slideglance-font`
6. `slideglance-parser`
7. `slideglance-renderer`
8. `slideglance-png`
9. `slideglance`

Run `cargo publish -p <crate>` for each, in order. Each release waits for the
previous crate's index update (~30 s) before the next dependent can resolve it.

**Gaps to close**:
- No automated `cargo publish` workflow yet — bring it under the same
  release pipeline as the npm packages.

---

## VS Code Marketplace

| Extension | Path | Marketplace identifier |
| --- | --- | --- |
| SlideGlance PPTX Viewer | `apps/vscode-extension` | publisher `slideglance`, displayName "SlideGlance PPTX Viewer" |

**Publish**: manual via `vsce publish` (or `vsce package` → upload `.vsix`
through the Marketplace UI). CI workflow `ci-vscode-extension.yml` runs
build + lint + typecheck on every change but does not deploy.

**Gaps to close**:
- Add a `release` workflow that runs `vsce publish` with `VSCE_PAT`.
- Pin the matching `@slideglance/builder` version in the bundle so the
  preview's lint output stays consistent with the user's source tree.

---

## Chrome Web Store

| Extension | Path | Manifest |
| --- | --- | --- |
| Slideglance Chrome Extension | `apps/chrome-extension` | MV3 (Manifest V3) |

**Publish**: manual via the
[Chrome Web Store dashboard](https://chrome.google.com/webstore/devconsole/) —
upload a `.zip` of the build output. No automated workflow.

**Gaps to close**:
- Add a `release` workflow that builds, zips, and uploads via the
  [Chrome Web Store Publish API](https://developer.chrome.com/docs/webstore/api).

---

## GitHub Pages

Served at the repository's Pages URL. Workflow: `.github/workflows/pages.yml`.

| Sub-site | Source | Mount |
| --- | --- | --- |
| Landing page | `apps/landing` (Vite SPA) | `/` |
| Web playground | `apps/web-playground` (Vite SPA, loads `@slideglance/viewer` + WASM) | `/playground/` (assembled into the Pages artifact) |
| Viewer static bundle | `packages/viewer` build output | served from the playground's assets |

**Trigger**: push to `main` (also `workflow_dispatch`). Builds the WASM, the
JS workspaces, assembles the Pages artifact, and deploys via
`actions/deploy-pages`.

**Status**: **automated**, no manual steps.

**Gaps to close**:
- Currently no custom domain. The XSD migration would benefit from
  `schema.slideglance.dev/v1/builder.xsd` if a domain is later registered —
  the Pages workflow can serve `schema/v1/builder.xsd` alongside the
  landing site once that is wired up.

---

## GitHub Releases

| Artefact | Source | Trigger |
| --- | --- | --- |
| SlideGlance PPTX Viewer (Tauri desktop) | `apps/desktop-viewer` | `.github/workflows/tauri-build.yml` — matrix `macos-latest` / `windows-latest` / `ubuntu-latest`. Uploads installers as workflow artefacts (a Release attachment step is the natural next addition). |

**Status**: **automated build** (installers produced on every push that matches
the workflow trigger), **manual release attach** (the workflow uploads to the
workflow run, not to a Releases tag — promote when the release pipeline is
formalised).

---

## CDN — schema artefacts surfaced via unpkg

Generated by `pnpm --filter @slideglance/builder run codegen` and shipped at the
package root so the unpkg URL pin is stable.

| Artefact | Path inside the npm package | unpkg URL (with major-version pin) |
| --- | --- | --- |
| XML Schema (XSD) | `builder.xsd` | `https://unpkg.com/@slideglance/builder@^1/builder.xsd` |
| JSON Schema | `builder.schema.json` | `https://unpkg.com/@slideglance/builder@^1/builder.schema.json` |
| Reference markdown | `reference.md` | `https://unpkg.com/@slideglance/builder@^1/reference.md` |
| Lint catalog (hand-curated) | `docs/lint.md` | `https://unpkg.com/@slideglance/builder@^1/docs/lint.md` |

These URLs power:
- `xsi:schemaLocation` in every `.sgx` template (Red Hat XML extension validation)
- third-party tooling that points at the schema artefacts published with the package
- Generic linking from third-party tooling

**Caveat**: `@^1` only starts resolving after the first 1.x publish of
`@slideglance/builder`. Until then, validators that respect `schemaLocation`
hints will see a 404. `schemaLocation` is advisory in the XML Schema
specification, so unresolved hints don't break builds — only "validate
against the linked schema" workflows in editors are affected.

---

## Summary — what's automated vs. manual today

| Channel | Automated? | Workflow |
| --- | --- | --- |
| GitHub Pages | ✅ | `.github/workflows/pages.yml` |
| Tauri desktop build | ✅ build, ❌ release attach | `.github/workflows/tauri-build.yml` |
| npm (4 packages) | ❌ | none |
| crates.io (9 crates) | ❌ | none |
| VS Code Marketplace | ❌ | none |
| Chrome Web Store | ❌ | none |

**Recommended next step**: a single `release.yml` workflow keyed off git tags
that fans out: `cargo publish` (in dependency order) → `wasm-pack publish` for
`core` and `measure` → npm publish for the remaining `@slideglance` packages →
`vsce publish` → Chrome Web Store API upload → Tauri release attach.
