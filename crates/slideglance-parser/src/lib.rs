//! ZIP archive access and XML deserialization primitives for slideglance.
//!
//! Phase 3-1 of the migration: provides the lowest layer that the typed model
//! parsers (Phase 3-3) build on.
//!
//! - [`PptxArchive`] reads a `.pptx` (ZIP) blob, eagerly extracting the
//!   XML/`.rels`/Content-Types entries as UTF-8 strings, and exposing media
//!   files lazily through a per-call cache.
//! - [`parse_xml`] is a thin wrapper around `quick-xml`'s serde deserializer
//!   that first strips XML namespace prefixes (mirroring the TypeScript
//!   reference's `removeNSPrefix: true` setting in fast-xml-parser).

#![deny(missing_docs)]

mod archive;
mod blip_effect;
mod chart;
mod custom_geometry;
mod effect;
mod fill;
mod geometry_formula;
mod notes;
mod presentation;
mod raw_color;
mod relationships;
mod shape_geometry;
mod slide;
mod slide_layout;
mod slide_master;
mod style_reference;
mod table;
mod text_body;
mod text_style;
mod text_style_resolver;
mod theme;
mod xml;

pub use archive::{ArchiveError, PptxArchive};
pub use blip_effect::parse_blip_effects;
pub use chart::parse_chart;
pub use custom_geometry::parse_custom_geometry;
pub use effect::parse_effect_list;
pub use fill::{parse_fill, parse_outline, FillParseContext};
pub use geometry_formula::{evaluate_formula, evaluate_guides, resolve_value, GuideDefinition};
pub use notes::parse_notes_text;
pub use presentation::parse_presentation;
pub use relationships::{
    build_rels_path, parse_relationships, resolve_relationship_target, Relationship,
};
pub use shape_geometry::{parse_geometry, parse_transform};
pub use slide::parse_slide;
pub use slide_layout::parse_slide_layout;
pub use slide_master::parse_slide_master;
pub use style_reference::{resolve_shape_style, FontReference, ResolvedStyleReference};
pub use table::parse_table;
pub use text_body::parse_text_body;
pub use text_style::{parse_list_style, resolve_theme_font};
pub use text_style_resolver::{apply_text_style_inheritance, TextStyleContext};
pub use theme::parse_theme;
pub use xml::{parse_xml, strip_namespaces, XmlError};
