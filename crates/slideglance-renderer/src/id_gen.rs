//! Deterministic ID generator for `<defs>` cross-references.
//!
//! The spec uses `crypto.randomUUID` to mint IDs for `<linearGradient>`,
//! `<pattern>`, `<marker>`, `<filter>`, and image-fill `<pattern>` definitions,
//! then references them via `url(#...)` from element attributes. Random UUIDs
//! make rendering non-deterministic across runs — the `slideglance` migration plan
//! mandates byte-identical SVG for the same input + options, so the Rust port
//! replaces UUIDs with a per-renderer counter.
//!
//! Each top-level slide render starts a fresh [`IdGen`]; element renderers
//! threaded with `&mut IdGen` request IDs as needed and the output is stable.

/// Monotonically increasing ID minter for SVG `<defs>` cross-references.
#[derive(Debug, Default)]
pub struct IdGen {
    next: u64,
}

impl IdGen {
    /// Construct a fresh generator starting at `0`.
    #[must_use]
    pub const fn new() -> Self {
        Self { next: 0 }
    }

    /// Mint the next ID with the given prefix (e.g. `"grad"`, `"patt"`,
    /// `"marker"`, `"imgfill"`, `"effect"`). Returns `"<prefix>-<n>"` where
    /// `<n>` is a base-10 counter — short, unique, and deterministic.
    pub fn next_id(&mut self, prefix: &str) -> String {
        let id = format!("{prefix}-{}", self.next);
        self.next += 1;
        id
    }

    /// Inspect the next counter without consuming it. Tests use this to
    /// verify a renderer minted exactly the expected number of IDs.
    #[must_use]
    pub const fn peek(&self) -> u64 {
        self.next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_sequential_per_prefix() {
        let mut ids = IdGen::new();
        assert_eq!(ids.next_id("grad"), "grad-0");
        assert_eq!(ids.next_id("grad"), "grad-1");
        assert_eq!(ids.next_id("patt"), "patt-2");
    }

    #[test]
    fn fresh_generator_starts_at_zero() {
        let ids = IdGen::new();
        assert_eq!(ids.peek(), 0);
    }

    #[test]
    fn peek_does_not_consume() {
        let mut ids = IdGen::new();
        let _ = ids.next_id("x");
        assert_eq!(ids.peek(), 1);
        assert_eq!(ids.peek(), 1);
    }
}
