//! Text body -> SVG `<text>` rendering.
//!
//! Direct port of 's text-mode
//! and path-mode pipelines plus the autofit (spAutofit / normAutofit)
//! infrastructure.
//!
//! Module split:
//!
//! - [`auto_num`] — `formatAutoNum` + Roman / Latin numeral helpers
//! - [`script`] — CJK code-point detection and script segmentation
//! - [`font_family`] — `buildFontFamilyValue` (mapping + CJK fallback walking)
//! - [`style`] — `buildStyleAttrs`, `buildBulletStyleAttrs`
//! - [`segment`] — `renderSegment` (one run -> one or more `tspan`s)
//! - [`layout`] — alignment / spacing / line-height helpers
//! - [`autofit`] — `computeSpAutofitHeight` + `computeShrinkToFitScale`
//! - [`body`] — `render_text_body` entry point
//! - [`path_mode`] — glyph-outline path rendering
//! - [`warp`] — `WordArt` text-mode warp via `<textPath>`
//! - `warp_path` — `WordArt` path-mode warp (per-glyph along arc length)

mod auto_num;
pub mod autofit;
pub(crate) mod body;
mod font_family;
pub(crate) mod layout;
mod path_mode;
mod script;
mod segment;
mod style;
pub mod warp;
mod warp_path;

pub use auto_num::format_auto_num;
pub use autofit::{compute_shrink_to_fit_scale, compute_sp_autofit_height};
pub use body::render_text_body;
pub use font_family::{build_font_family_chain, build_font_family_value};
pub use layout::{
    compute_line_natural_height, get_alignment_info, get_default_ascender_ratio,
    get_default_font_size, get_default_line_height_ratio, get_line_spacing,
    get_paragraph_font_size, resolve_spacing_px, AlignmentInfo,
};
pub use path_mode::{
    build_path_fill_attrs, compute_path_line_x, render_bullet_as_path, render_segment_as_path,
    render_text_body_as_path, render_text_decorations, SegmentPath,
};
pub use script::{is_cjk_codepoint, split_by_script};
pub use segment::render_segment;
pub use style::{build_bullet_style_attrs, build_style_attrs};
pub use warp::{render_text_body_as_warp, warp_preset_to_path_d};
pub use warp_path::render_text_body_as_warp_path;
