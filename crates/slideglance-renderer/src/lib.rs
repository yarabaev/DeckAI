//! Model -> SVG renderer.
//!
//! Lowest-level building blocks (`svg_builder`, `transform`, `viewbox`,
//! `slide_context`) are implemented first; element renderers (shapes, fills,
//! text, tables, charts, images) are added in subsequent batches; the
//! slide-level entry point [`slide::render_slide_to_svg`] composes all of
//! them. The output contract mirrors the TypeScript reference at
//! : a single `<svg>` document
//! per slide, no CSS classes (sharp/librsvg compatibility), inline
//! attributes only.
//!
//! [`error::RendererError::NotImplemented`] is reserved for forward-
//! compatibility — no production codepath emits it today. Effects
//! (`<a:effectLst>`), text-body `spAutofit` / `normAutofit`, and
//! `WordArt` warps in both text and path modes are fully wired up.

#![deny(missing_docs)]

pub mod blip_effects;
pub mod chart;
pub mod color;
pub mod connector;
pub mod effects;
pub mod error;
pub mod fill;
pub mod geometry;
pub mod id_gen;
pub mod image;
pub mod render_result;
pub mod shape;
pub mod slide;
pub mod slide_context;
pub mod svg_builder;
pub mod table;
pub mod text;
pub mod transform;
pub mod viewbox;

pub use blip_effects::render_blip_effects;
pub use chart::{render_chart, ChartRenderResult};
pub use color::{alpha_str, color_hex};
pub use connector::render_connector;
pub use effects::{render_effects, EffectResult};
pub use error::RendererError;
pub use fill::{render_fill_attrs, render_markers, render_outline_attrs, FillAttrs, MarkerResult};
pub use geometry::{preset_geometry_svg, render_geometry};
pub use id_gen::IdGen;
pub use image::{render_image, ImageRenderResult};
pub use render_result::RenderResult;
pub use shape::render_shape;
pub use slide::render_slide_to_svg;
pub use slide_context::{
    format_field, slide_field_text, SlideRenderContext, Timestamp, FIELD_SLIDE_NUMBER,
};
pub use svg_builder::{escape_xml_attr, escape_xml_text};
pub use table::{lookup_table_style_preset, render_table, TableElementResult, TableStylePreset};
pub use text::{
    build_font_family_chain, build_font_family_value, render_text_body, render_text_body_as_path,
    render_text_body_as_warp_path,
};
pub use transform::{build_object_name_attr, build_transform_attr};
pub use viewbox::SlideViewBox;
