# builder-reference-example

End-to-end XML reference deck built with (../../packages/builder). Acts as both a runnable showcase and a smoke test for the public builder API.

## What it does

`main.sgx` is a multi-chapter `.pptx`-equivalent XML document that
exercises every node type the builder supports — text, lists, images,
tables, shapes, charts, lines, icons, inline SVG, layout containers,
and composition primitives (`<Templates>`, `<Use>`, `<Slot>`,
`<Import>`, `<Styles>`, `<If>` / `<Choose>` / `<Foreach>`,
`<Master>`).

A regression in any one surface lands as a visible diff in the
rendered slides, so the example doubles as a release-gate smoke test.

## Build

```sh
pnpm --filter @slideglance/builder-reference-example build
```

Reads `main.sgx`, resolves `<Import>` / `<Use>` against
`templates/` / `chapters/` / `styles/`, and emits the compiled `.pptx`
into `output/`.

## Layout

| Path         | Contents                                                                |
| ------------ | ----------------------------------------------------------------------- |
| `main.sgx`   | Top-level document — slide size, default master, imports, slide order.  |
| `chapters/`  | One XML file per chapter, each defining one or more `<Slide>` elements. |
| `templates/` | Reusable `<Templates>` definitions (cards, diagrams, layouts).          |
| `styles/`    | Named `<Styles>` (typography, colors, decoration).                      |
| `build.ts`   | Compile script — calls `buildPptx` with a hardened import resolver.     |
| `output/`    | Generated `.pptx` files (gitignored).                                   |

