//! Element-level rendering result.
//!
//! Mirrors. Each element
//! renderer (shape, connector, group, image, chart, table, …) returns one
//! [`RenderResult`]: the inline SVG fragment plus the `<defs>` snippets
//! it minted.
//!
//! The TS port stores `defs` as `string[]` so each level can `push` /
//! `splice`. The Rust port uses a single concatenated [`String`] for two
//! reasons:
//! 1. Every existing per-element renderer (`fill`, `image`, `table`,
//!    `chart`) already returns `defs: String`; staying with `String` lets
//!    the slide renderer fold their output without re-allocating into a
//!    `Vec<String>`.
//! 2. The slide renderer always concatenates with the empty separator
//!    when writing the `<defs>` block, so the `Vec<String>` form would
//!    collapse to the same wire output anyway.

/// SVG body + accumulated `<defs>` content produced by an element
/// renderer.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RenderResult {
    /// Inline SVG fragment for this element (typically a `<g>` wrapper).
    pub content: String,
    /// `<defs>` content collected while rendering. May be empty.
    pub defs: String,
}

impl RenderResult {
    /// Construct an empty result.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}
