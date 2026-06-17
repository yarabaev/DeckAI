//! Font mapping, measurement, shaping, and resolution for slideglance.
//!
//! This crate is the Rust port of plus the
//! `text-wrap` / `text-measure` utilities. It owns
//! everything related to turning OOXML font references (theme tokens,
//! text-run `typeface` strings) into renderable glyph metrics, fallback
//! chains, and shaped runs.
//!
//! ## Modules
//!
//! - [`mapping`] — `DEFAULT_FONT_MAPPING` + case-insensitive / full-width
//!   normalized lookup.
//! - [`cjk_fallback`] — per-OS preinstalled CJK fallback chains for
//!   Japanese / Korean, plus Chinese (SC / TC) entries per the
//!   project's CJK Script Equality rule.
//! - [`script_context`] — theme script-font context. Stores every
//!   script (Jpan / Hang / Hans / Hant + arbitrary ISO 15924) rather
//!   than collapsing to a Jpan-only special case.
//! - [`fallback_metrics`] — offline-extracted character-width tables
//!   for Carlito / Liberation Sans / Liberation Serif / Noto Sans JP.
//! - [`text_measure`] — heuristic + metric-aware width / line-height
//!   calculation.
//! - [`text_wrap`] — greedy paragraph wrap with CJK char-boundary
//!   breaks and a pluggable [`text_wrap::TextMeasurer`] trait.
//! - [`opentype`] — `ttf-parser` wrapper that owns the font bytes and
//!   exposes scalar metrics, glyph index lookup, advance, and family
//!   name.
//!
//! Subsequent batches add TTC parsing, an opentype-backed measurer,
//! the font collector, the `FontResolver` trait, rustybuzz shaping, and
//! Google Fonts fetcher integration.
//!
//! ## CJK Script Equality
//!
//! Where the TypeScript reference handles `Jpan` only (e.g.
//! ), this crate intentionally treats `Jpan`,
//! `Hang`, `Hans`, and `Hant` symmetrically per the project rule in
//! `CLAUDE.md`. Affected modules document the divergence inline.
//!
//! ## Demoted resolvers (KDD-20)
//!
//! `MappedFontResolver` and `CjkFallbackResolver` are intentionally
//! `pub(crate)`: external callers compose chains via the public
//! [`standard_resolver_chain`] factory. The doc-tests below verify the
//! demotion holds.
//!
//! ```compile_fail
//! // MappedFontResolver must not be reachable from the public surface.
//! let _ = slideglance_font::MappedFontResolver::new(
//!     slideglance_font::BufferFontResolver::default(),
//!     slideglance_font::FontMapping::new(),
//! );
//! ```
//!
//! ```compile_fail
//! // CjkFallbackResolver must not be reachable from the public surface.
//! let _ = slideglance_font::CjkFallbackResolver::new(
//!     slideglance_font::BufferFontResolver::default(),
//!     slideglance_font::CjkPlatform::Other,
//! );
//! ```

#![deny(missing_docs)]

pub mod cjk_fallback;
pub mod collector;
pub mod fallback_metrics;
pub mod font_fetcher;
pub mod font_metric;
pub mod font_resolver;
pub mod latin_defaults;
pub mod mapping;
pub mod opentype;
pub mod script_context;
pub mod text_engine;
pub mod text_measure;
pub mod text_measurer;
pub mod text_path;
pub mod text_wrap;
pub mod ttc;

/// Host-system font discovery. Opt-in via the `system-fonts` cargo
/// feature; enabling breaks deterministic output across machines.
#[cfg(feature = "system-fonts")]
pub mod system_fonts;

#[cfg(feature = "system-fonts")]
pub use system_fonts::{
    load_system_font_bytes, load_system_font_bytes_for_families, load_system_fonts,
};

pub use cjk_fallback::{get_cjk_fallback_fonts, CjkPlatform};
pub use collector::{collect_used_fonts, ThemeFonts, UsedFonts};
pub use fallback_metrics::{get_font_metrics, get_metrics_fallback_font, FontMetrics};
pub use font_fetcher::{FetcherFontResolver, FontFetcher};
pub use font_metric::{
    find_best_metric_match, known_metric_fonts, metric_distance, metric_for_family,
    system_font_metrics, FontMetricVector, MetricResolver, Panose,
};
pub use font_resolver::{
    standard_resolver_chain, BufferFontResolver, ChainFontResolver, FontResolver,
    FontVariantResolver,
};
pub use latin_defaults::get_latin_os_defaults;
pub use mapping::{
    create_font_mapping, default_font_mapping, get_mapped_font, FontMapping, DEFAULT_FONT_MAPPING,
};
pub use opentype::{all_face_family_names, FontError, FontFace};
pub use script_context::{ScriptFontContext, CJK_SCRIPT_CODES};
pub use text_engine::{RenderMode, TextEngine, TextEngineBuilder};
pub use text_measure::{
    classify_script, get_ascender_ratio, get_line_height_ratio, is_cjk_codepoint,
    is_complex_script_codepoint, is_halfwidth_cjk_codepoint, is_pua_codepoint,
    is_sym_pua_codepoint, measure_text_width, split_by_script, Script, TextPart, HALFWIDTH_RATIO,
};
pub use text_measurer::{FontStyle, HeuristicTextMeasurer, OpentypeTextMeasurer, TextMeasurer};
pub use text_path::{
    text_to_svg_path, text_to_svg_path_kerned, text_to_svg_path_with_precision,
    DEFAULT_DECIMAL_PLACES,
};
pub use text_wrap::{
    wrap_paragraph, wrap_paragraph_with_chain, LineSegment, WrappedLine, DEFAULT_FONT_SIZE,
};
pub use ttc::{extract_first_ttc_face, extract_ttc_faces, is_ttc, parse_font_data, ttc_face_count};
