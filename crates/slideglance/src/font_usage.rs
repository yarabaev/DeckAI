//! Font-usage report — which deck-referenced typefaces resolve to which
//! actual rendering fonts.
//!
//! The PPTX renderer emits a CSS `font-family` fallback chain in every
//! `<text>` element. Browsers walk that chain and use the first
//! installed font; a path-mode rasterizer (resvg) consults the supplied
//! [`slideglance_font::FontResolver`] for glyph outlines. In both cases the
//! actual font that ends up on screen is **not** necessarily the
//! authored typeface — it can be a metric-compatible substitute, a
//! CJK platform fallback, or a generic family.
//!
//! [`TypefaceUsage`] surfaces both pieces of information:
//!
//! - `fallback_chain` — the ordered list of names a browser would
//!   walk. Hosts probe each entry via `document.fonts.check()` to
//!   identify the effectively-rendered font.
//! - `resolved_family` — the [`slideglance_font::FontResolver::resolve`]
//!   answer for the requested name. `Some(family)` means the
//!   path-mode rasterizer would use that face; `None` means no
//!   resolver entry, so text-mode rendering is up to the host.
//!
//! Hosts (the `@slideglance/viewer` status bar, the CLI's `--font-report`
//! flag) use this to show users which fonts diverge from the
//! authored ones, so they can install missing fonts or accept the
//! substitute knowingly.

use serde::{Deserialize, Serialize};
use slideglance_font::{CjkPlatform, FontMapping, FontResolver, ScriptFontContext};
use slideglance_renderer::build_font_family_chain;

/// One typeface referenced by the deck and how it resolves through the
/// font fallback chain.
///
/// See module docs for the role of each field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypefaceUsage {
    /// The typeface name as it appears in the deck — e.g. the value of
    /// `<a:latin typeface="Calibri"/>` or `<a:font script="Hang"
    /// typeface="맑은 고딕"/>`.
    pub requested: String,
    /// Ordered list of font-family names that the SVG `font-family`
    /// attribute lists for this typeface, before the trailing generic
    /// (`sans-serif` / `serif`). Populated by the same logic that the
    /// renderer uses, so what a host sees here matches exactly what
    /// browsers walk. Empty when `requested` produces no chain (e.g.
    /// empty string).
    pub fallback_chain: Vec<String>,
    /// `Some(family_name)` when a path-mode [`FontResolver`] returns a
    /// face for `requested`. `None` when no resolver was supplied or
    /// none of its layers had bytes for this name.
    ///
    /// In text-mode (browser) rendering this field is informational
    /// only — the actually-rendered font is whichever entry in
    /// `fallback_chain` is installed on the client. Hosts probe the
    /// chain via `document.fonts.check()` to find that entry.
    pub resolved_family: Option<String>,
}

/// Build a [`TypefaceUsage`] report for the given list of typeface
/// names. The list is typically the output of
/// [`crate::extract_referenced_font_families`] — every distinct
/// typeface the deck mentions in any `typeface="…"` attribute.
///
/// `mapping`, `cjk_platform`, and `script_fonts` must match the values
/// used by the renderer for this deck so the chain reflects what the
/// SVG actually emits. `resolver` is the same path-mode resolver
/// passed to `render_slide`; pass `None` for text-mode-only hosts.
#[must_use]
pub fn build_typeface_usage(
    requested: &[String],
    mapping: &FontMapping,
    cjk_platform: CjkPlatform,
    script_fonts: &ScriptFontContext,
    resolver: Option<&dyn FontResolver>,
) -> Vec<TypefaceUsage> {
    requested
        .iter()
        .map(|name| {
            let chain = build_font_family_chain(
                &[Some(name.as_str())],
                mapping,
                cjk_platform,
                script_fonts,
            );
            let resolved_family = resolver
                .and_then(|r| r.resolve(name))
                .and_then(|face| face.family_name());
            TypefaceUsage {
                requested: name.clone(),
                fallback_chain: chain,
                resolved_family,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_font::{BufferFontResolver, ScriptFontContext};

    #[test]
    fn empty_input_yields_empty_report() {
        let usage = build_typeface_usage(
            &[],
            &FontMapping::new(),
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
            None,
        );
        assert!(usage.is_empty());
    }

    #[test]
    fn single_typeface_produces_chain_with_metric_fallback() {
        let usage = build_typeface_usage(
            &["Calibri".to_string()],
            &FontMapping::new(),
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
            None,
        );
        assert_eq!(usage.len(), 1);
        let entry = &usage[0];
        assert_eq!(entry.requested, "Calibri");
        // The renderer's chain logic adds Carlito as the metric-compatible
        // fallback for Calibri. Verifying the head is enough — the rest
        // of the chain content is exercised by font_family.rs tests.
        assert!(entry.fallback_chain.contains(&"Calibri".to_string()));
        assert!(entry.fallback_chain.contains(&"Carlito".to_string()));
        // No resolver supplied -> no path-mode resolution.
        assert_eq!(entry.resolved_family, None);
    }

    #[test]
    fn resolved_family_is_none_when_resolver_misses() {
        let resolver = BufferFontResolver::new();
        let usage = build_typeface_usage(
            &["NonexistentFont".to_string()],
            &FontMapping::new(),
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
            Some(&resolver),
        );
        assert_eq!(usage.len(), 1);
        assert_eq!(usage[0].resolved_family, None);
    }

    #[test]
    fn requested_typefaces_preserved_in_input_order() {
        let names = vec!["Calibri".to_string(), "Cambria".to_string()];
        let usage = build_typeface_usage(
            &names,
            &FontMapping::new(),
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
            None,
        );
        assert_eq!(usage[0].requested, "Calibri");
        assert_eq!(usage[1].requested, "Cambria");
    }
}
