# SlideGlance VRT (snapshot)

Standalone Cargo package that pins the SVG output of every registered fixture slide-by-slide so accidental renderer drift surfaces as a diff against a checked-in baseline.

Part of the [SlideGlance](https://github.com/SlideGlance/slideglance) project.

## What lives here

The runner machinery (`VrtCase` struct, `render_case`,
`assert_snapshot`, `snapshot_dir`, `slideglance-vrt-render` binary) is
generic. **No fixtures or baselines ship in this repo by default** —
`CASES` in `src/lib.rs` is empty and `snapshots/` does not exist yet.
Add fixtures to enable coverage.

## Adding a fixture

1. Drop the `.pptx` into `testing/fixtures/`.
2. Append a `VrtCase` entry to `CASES` in `src/lib.rs`:
   ```rust
   VrtCase {
       name: "<short-name>",
       fixture: "testing/fixtures/<file>.pptx",
       slides: None,         // or `Some(&[1, 3, 7])` for a subset
   },
   ```
3. Add a matching `#[test] fn vrt_<name>()` in your own `tests/snapshot.rs`.
4. Materialize the baseline:
   ```sh
   UPDATE_SNAPSHOTS=1 cargo test --release \
       --manifest-path testing/vrt/snapshot/Cargo.toml
   ```
5. Re-run without `UPDATE_SNAPSHOTS` to verify the diff is empty.

## Run

```sh
# Verify (default — fails on diff).
cargo test --release --manifest-path testing/vrt/snapshot/Cargo.toml

# Regenerate baselines after an intentional renderer change.
UPDATE_SNAPSHOTS=1 cargo test --release \
    --manifest-path testing/vrt/snapshot/Cargo.toml
```

`pnpm vrt` and `pnpm vrt:update` wrap the same commands from the repo
root.

## Why standalone

The crate is excluded from the main workspace `members` list so the
default `cargo test --workspace` doesn't materialize the snapshot tree
on every run. The price is that tests opt in via the explicit
`--manifest-path`; the gain is that day-to-day workspace tests stay
fast.

## Determinism

`convert_to_svg` is the input. The renderer runs in text-mode (no font
resolver passed) so SVG output stays locale-independent across host
font catalogs — see the determinism rule in `CLAUDE.md`. Repeated runs
without code changes are byte-identical.
