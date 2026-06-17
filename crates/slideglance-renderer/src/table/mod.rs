//! Table element rendering: cell grid, merges, borders, cell text.
//!
//! Direct port of plus
//! . Output is a `<g transform="...">` group
//! containing per-cell `<rect>` background, `<line>` borders, and a
//! translated `<g>` wrapper around each cell's text body.

mod presets;
mod render;

pub use presets::{lookup_table_style_preset, TableStylePreset};
pub use render::{render_table, TableElementResult};
