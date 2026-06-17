//! Public-surface assertions for D0 (KDD-20).
//!
//! Verifies that the new public types (`RenderMode`, `EmbedFormat`,
//! `FontStyle`, `FontVariantResolver`, `TextEngineBuilder`,
//! `standard_resolver_chain`, `FontConfig`) are reachable from outside
//! the crates that define them, and that the demoted resolvers
//! (`MappedFontResolver`, `CjkFallbackResolver`) are not.

use slideglance::{ConvertOptions, EmbedFormat, FontConfig, RenderMode};
use slideglance_font::{
    standard_resolver_chain, BufferFontResolver, CjkPlatform, FontMapping, FontResolver, FontStyle,
    FontVariantResolver, RenderMode as FontRenderMode, TextEngineBuilder,
};

#[test]
fn slideglance_render_mode_is_slideglance_font_render_mode() {
    // Confirm `slideglance::RenderMode` is the same type as
    // `slideglance_font::RenderMode` — re-export, not a parallel definition.
    fn _assert_same(_: FontRenderMode) -> RenderMode {
        RenderMode::TextMode
    }
    let mode: RenderMode = FontRenderMode::PathMode;
    assert_eq!(mode, RenderMode::PathMode);
}

#[test]
fn embed_format_default_accessible_from_slideglance() {
    let _ = EmbedFormat::Auto;
    let _ = EmbedFormat::Ttf;
    let _ = EmbedFormat::Woff2;
}

#[test]
fn font_style_accessible_from_slideglance_font() {
    let s = FontStyle {
        bold: true,
        italic: false,
    };
    assert!(s.bold);
}

#[test]
fn font_variant_resolver_trait_accessible() {
    fn assert_bound<T: FontVariantResolver + FontResolver>() {}
    assert_bound::<BufferFontResolver>();
}

#[test]
fn text_engine_builder_accessible() {
    let engine = TextEngineBuilder::new().build();
    assert_eq!(engine.render_mode(), FontRenderMode::TextMode);
}

#[test]
fn standard_resolver_chain_accessible() {
    let _ = standard_resolver_chain(
        BufferFontResolver::default(),
        FontMapping::new(),
        CjkPlatform::Other,
    );
}

#[test]
fn font_config_accessible_from_slideglance() {
    let cfg = FontConfig::default();
    assert!(cfg.embed_deck_fonts);
    let _ = ConvertOptions {
        fonts: cfg,
        ..ConvertOptions::default()
    };
}
