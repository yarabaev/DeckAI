# testing/

Test infrastructure shared across the Rust workspace. Two subtrees:

| Path                                            | Purpose                                                                                                                                                                       |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`fixtures/`](./fixtures/README.md)             | PPTX + font fixtures consumed by the renderer / parser / VRT integration tests. Empty by default — checked-in fixtures land here when contributors stage their own decks.    |
| [`vrt/snapshot/`](./vrt/snapshot/README.md)     | Standalone Cargo package that pins the SVG output of every registered fixture slide-by-slide so renderer drift surfaces as a diff.                                            |

The TypeScript-side VRT (deck-level visual regression) lives at
[`packages/builder/vrt/`](../packages/builder/vrt/). Same name,
different language and target — see that subtree's structure for the
builder VRT flavor.
