//! [`TextMeasurer`] trait + heuristic / opentype-backed implementations.
//!
//! The trait abstracts the
//! `measure_text_width` / `get_line_height_ratio` / `get_ascender_ratio`
//! triplet so callers (e.g. [`crate::wrap_paragraph`]) stay backend-
//! agnostic.
//!
//! Two backends ship in the crate:
//!
//! - [`HeuristicTextMeasurer`] — delegates to [`crate::text_measure`]
//!   (heuristic + offline-extracted fallback metrics). Default for any
//!   environment without ttf data.
//! - [`OpentypeTextMeasurer`] — owns a [`FontResolver`] and queries it
//!   per-name. The resolver is responsible for `mapping` and CJK
//!   fallback walking — measurer only knows "name → Arc<FontFace>".
//!
//! ## Resolver responsibility (single source for name resolution)
//!
//! Earlier revisions had `OpentypeTextMeasurer` walk
//! `mapping::get_mapped_font` + `get_cjk_fallback_fonts` itself, with
//! the same logic also reachable via [`crate::MappedFontResolver`] +
//! [`crate::CjkFallbackResolver`] composition. Two implementations of
//! the same chain risked silent divergence.
//!
//! The current design is single-responsibility: measurer asks the
//! resolver, resolver does **all** name resolution. Construct with
//! [`OpentypeTextMeasurer::from_fonts`] for the standard
//! `Mapped(Cjk(Buffer))` chain, or [`OpentypeTextMeasurer::with_resolver`]
//! when you want to plug in a custom resolver (e.g. a system-font
//! scanner from a future `slideglance-font-fontdb` crate).

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::cjk_fallback::CjkPlatform;
use crate::font_resolver::{
    BufferFontResolver, CjkFallbackResolver, FontResolver, FontVariantResolver, MappedFontResolver,
};
use crate::mapping::FontMapping;
use crate::opentype::FontFace;
use crate::text_measure;
use ttf_parser::Tag;

/// Variable-font axis tag for weight (`wght`).
const AXIS_WGHT: Tag = Tag::from_bytes(b"wght");
/// Variable-font axis tag for italic (`ital`).
const AXIS_ITAL: Tag = Tag::from_bytes(b"ital");

/// Apply style-driven variation axes (`wght`, `ital`) when the face is
/// a variable font. For static faces or default style, returns the
/// original `Arc<FontFace>` unchanged. KDD-13 implementation.
fn apply_style_variations(face: Option<Arc<FontFace>>, style: FontStyle) -> Option<Arc<FontFace>> {
    let arc = face?;
    if !arc.is_variable() || (!style.bold && !style.italic) {
        return Some(arc);
    }
    let mut owned: FontFace = (*arc).clone();
    if style.bold {
        owned.set_variation(AXIS_WGHT, 700.0);
    }
    if style.italic {
        owned.set_variation(AXIS_ITAL, 1.0);
    }
    Some(Arc::new(owned))
}

const PX_PER_PT: f64 = 96.0 / 72.0;
const DEFAULT_LINE_HEIGHT_RATIO: f64 = 1.2;
const DEFAULT_ASCENDER_RATIO: f64 = 1.0;

/// Combined bold + italic state passed to text measurement.
///
/// Replaces the previous `bold: bool` parameter on
/// [`TextMeasurer::measure_text_width`]. Italic is plumbed through D0;
/// italic-aware advance computation lands in D3 (KDD-13) — until then
/// implementations may treat `italic` as informational.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct FontStyle {
    /// Bold weight signal — equivalent to `<a:rPr b="1">`.
    pub bold: bool,
    /// Italic posture signal — equivalent to `<a:rPr i="1">`.
    pub italic: bool,
}

/// Pluggable text-width / line-height / ascender measurement.
///
/// The renderer's wrap pass and baseline-offset pass use this trait to
/// stay independent of the backend.
pub trait TextMeasurer {
    /// Estimated rendered pixel width of `text` at `font_size_pt`.
    fn measure_text_width(
        &self,
        text: &str,
        font_size_pt: f64,
        style: FontStyle,
        font_family: Option<&str>,
        font_family_ea: Option<&str>,
    ) -> f64;

    /// Like [`Self::measure_text_width`] but accepts a pre-built CSS
    /// `font-family` chain string (KDD-15). Browser-backed measurers
    /// (canvas / wasm) use the chain verbatim so their measurement
    /// matches the rendered SVG `font-family` attribute exactly,
    /// eliminating wrap drift between measurement and render.
    ///
    /// The default implementation ignores the chain and forwards to
    /// [`Self::measure_text_width`] — native OpenType / heuristic
    /// backends do not need it because they consult font bytes
    /// directly via `font_family` / `font_family_ea`.
    ///
    /// Implementations should override this method only when the
    /// measurement backend is browser-driven (e.g. `OffscreenCanvas`).
    fn measure_text_width_with_chain(
        &self,
        text: &str,
        font_size_pt: f64,
        style: FontStyle,
        font_family: Option<&str>,
        font_family_ea: Option<&str>,
        font_family_chain: Option<&str>,
    ) -> f64 {
        let _ = font_family_chain;
        self.measure_text_width(text, font_size_pt, style, font_family, font_family_ea)
    }

    /// Natural line-height (× fontSize) for the run's primary or EA
    /// font. Defaults to `1.2` when no metrics resolve.
    fn get_line_height_ratio(&self, font_family: Option<&str>, font_family_ea: Option<&str>)
        -> f64;

    /// Ascender ratio (× fontSize) for the run's primary or EA font.
    /// Defaults to `1.0` when no metrics resolve.
    fn get_ascender_ratio(&self, font_family: Option<&str>, font_family_ea: Option<&str>) -> f64;
}

/// Heuristic + fallback-metrics backend.
///
/// Delegates every method to [`crate::text_measure`]. Renderer chooses
/// this when no real ttf data is available.
#[derive(Debug, Default, Clone, Copy)]
pub struct HeuristicTextMeasurer;

impl TextMeasurer for HeuristicTextMeasurer {
    fn measure_text_width(
        &self,
        text: &str,
        font_size_pt: f64,
        style: FontStyle,
        font_family: Option<&str>,
        font_family_ea: Option<&str>,
    ) -> f64 {
        // Italic is accepted but not yet used here — D3 will add an
        // italic advance correction. Bold is still the only signal that
        // changes the heuristic width today.
        text_measure::measure_text_width(
            text,
            font_size_pt,
            style.bold,
            font_family,
            font_family_ea,
        )
    }

    fn get_line_height_ratio(
        &self,
        font_family: Option<&str>,
        font_family_ea: Option<&str>,
    ) -> f64 {
        text_measure::get_line_height_ratio(font_family, font_family_ea)
    }

    fn get_ascender_ratio(&self, font_family: Option<&str>, font_family_ea: Option<&str>) -> f64 {
        text_measure::get_ascender_ratio(font_family, font_family_ea)
    }
}

/// Opentype-backed measurer. Holds a [`FontResolver`] and a per-call
/// fallback face.
///
/// All name resolution (direct lookup, mapping, CJK fallback) is the
/// resolver's concern; the measurer just asks
/// `resolver.resolve(name)` and arithmetic-uses the returned face.
/// When **no** face resolves for either the latin or EA font, the
/// measurer falls back to [`HeuristicTextMeasurer`] so callers always
/// get a usable width / ratio.
pub struct OpentypeTextMeasurer {
    resolver: Arc<dyn FontVariantResolver + Send + Sync>,
    default_font: Option<Arc<FontFace>>,
}

impl OpentypeTextMeasurer {
    /// Builds the measurer with the standard
    /// `Mapped(CjkFallback(Buffer))` resolver chain wrapped around the
    /// supplied font map.
    ///
    /// `font_mapping` is the `FontMapping` to apply (pass
    /// [`crate::mapping::default_font_mapping`] for the standard PPTX
    /// → OSS mapping, or [`FontMapping::new`] for empty). `cjk_platform`
    /// selects the CJK fallback chain (`CjkPlatform::current()` for
    /// the host OS).
    #[must_use]
    pub fn from_fonts(
        fonts: BTreeMap<String, FontFace>,
        default_font: Option<FontFace>,
        font_mapping: FontMapping,
        cjk_platform: CjkPlatform,
    ) -> Self {
        let mut buffer = BufferFontResolver::new();
        for (name, face) in fonts {
            buffer.insert(name, face);
        }
        let cjk = CjkFallbackResolver::new(buffer, cjk_platform);
        let mapped = MappedFontResolver::new(cjk, font_mapping);
        Self {
            resolver: Arc::new(mapped),
            default_font: default_font.map(Arc::new),
        }
    }

    /// Builds the measurer with a caller-supplied resolver. Use this
    /// when you want a custom chain (e.g. inserting a system-font
    /// scanner between Buffer and the pan-CJK alias resolver).
    #[must_use]
    pub fn with_resolver(
        resolver: impl FontVariantResolver + Send + Sync + 'static,
        default_font: Option<Arc<FontFace>>,
    ) -> Self {
        Self {
            resolver: Arc::new(resolver),
            default_font,
        }
    }

    fn resolve_font(&self, name: Option<&str>) -> Option<Arc<FontFace>> {
        let name = name?;
        if name.is_empty() {
            return None;
        }
        self.resolver.resolve(name)
    }

    fn fallback_face<'a>(
        &'a self,
        latin: Option<&'a Arc<FontFace>>,
        ea: Option<&'a Arc<FontFace>>,
    ) -> Option<&'a FontFace> {
        latin
            .map(AsRef::as_ref)
            .or(ea.map(AsRef::as_ref))
            .or(self.default_font.as_deref())
    }
}

impl TextMeasurer for OpentypeTextMeasurer {
    #[allow(clippy::similar_names)] // font_size_pt vs font_size_px is the
                                    // intentional pt→px conversion idiom.
    fn measure_text_width(
        &self,
        text: &str,
        font_size_pt: f64,
        style: FontStyle,
        font_family: Option<&str>,
        font_family_ea: Option<&str>,
    ) -> f64 {
        // When a bold variant face is registered, use it directly for
        // Latin runs. resolve_variant returns None when no variant has
        // been registered (current D0 placeholder state); callers get
        // accurate advances from the regular face without a BOLD_FACTOR
        // correction.
        let latin_face = if style.bold {
            font_family
                .and_then(|n| self.resolver.resolve_variant(n, style))
                .or_else(|| self.resolve_font(font_family))
        } else {
            self.resolve_font(font_family)
        };
        let ea_face = if style.bold {
            font_family_ea
                .and_then(|n| self.resolver.resolve_variant(n, style))
                .or_else(|| self.resolve_font(font_family_ea))
        } else {
            self.resolve_font(font_family_ea)
        };

        // Variable-font axis selection (KDD-13). When the resolved face
        // is variable (`fvar` table present) and the run requests bold or
        // italic, derive a variation-pinned clone so glyph advances reflect
        // the requested instance instead of the default. No-op on
        // non-variable fonts (return original Arc unchanged).
        let latin_face = apply_style_variations(latin_face, style);
        let ea_face = apply_style_variations(ea_face, style);
        let fallback = self.fallback_face(latin_face.as_ref(), ea_face.as_ref());
        let Some(fallback) = fallback else {
            // Nothing resolves — defer to the heuristic so callers still
            // get a usable number.
            return HeuristicTextMeasurer.measure_text_width(
                text,
                font_size_pt,
                style,
                font_family,
                font_family_ea,
            );
        };
        let font_size_px = font_size_pt * PX_PER_PT;
        let mut total = 0.0_f64;
        for ch in text.chars() {
            let cp = ch as u32;
            let is_ea = text_measure::is_cjk_codepoint(cp);
            let face: &FontFace = if is_ea {
                ea_face.as_deref().unwrap_or(fallback)
            } else {
                latin_face.as_deref().unwrap_or(fallback)
            };
            let upem = f64::from(face.units_per_em());
            if upem == 0.0 {
                continue;
            }
            let scale = font_size_px / upem;
            let advance = face
                .char_hor_advance(ch)
                // Missing glyph: use unitsPerEm * 0.6 — matches the
                // heuristic `normal` ratio so the measurement remains
                // continuous with the no-font case.
                .map_or(upem * 0.6, f64::from);
            // Italic advance correction lives in future work (variable
            // font + italic measurement). For now `style.italic` is
            // accepted but does not modify the advance.
            total += advance * scale;
        }
        total
    }

    fn get_line_height_ratio(
        &self,
        font_family: Option<&str>,
        font_family_ea: Option<&str>,
    ) -> f64 {
        let latin = self.resolve_font(font_family);
        let ea = self.resolve_font(font_family_ea);
        let Some(face) = self.fallback_face(latin.as_ref(), ea.as_ref()) else {
            return DEFAULT_LINE_HEIGHT_RATIO;
        };
        let upem = f64::from(face.units_per_em());
        if upem == 0.0 {
            return DEFAULT_LINE_HEIGHT_RATIO;
        }
        (f64::from(face.ascender()) + f64::from(face.descender()).abs()) / upem
    }

    fn get_ascender_ratio(&self, font_family: Option<&str>, font_family_ea: Option<&str>) -> f64 {
        let latin = self.resolve_font(font_family);
        let ea = self.resolve_font(font_family_ea);
        let Some(face) = self.fallback_face(latin.as_ref(), ea.as_ref()) else {
            return DEFAULT_ASCENDER_RATIO;
        };
        let upem = f64::from(face.units_per_em());
        if upem == 0.0 {
            return DEFAULT_ASCENDER_RATIO;
        }
        f64::from(face.ascender()) / upem
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_opentype() -> OpentypeTextMeasurer {
        OpentypeTextMeasurer::from_fonts(
            BTreeMap::new(),
            None,
            FontMapping::new(),
            CjkPlatform::Linux,
        )
    }

    // -- HeuristicTextMeasurer parity with text_measure module ---------------

    #[test]
    fn heuristic_measurer_matches_module_ascii() {
        let measurer = HeuristicTextMeasurer;
        let trait_width =
            measurer.measure_text_width("Hello", 18.0, FontStyle::default(), None, None);
        let module_width = text_measure::measure_text_width("Hello", 18.0, false, None, None);
        assert_eq!(trait_width, module_width);
    }

    #[test]
    fn font_style_default_is_neither_bold_nor_italic() {
        let s = FontStyle::default();
        assert!(!s.bold);
        assert!(!s.italic);
    }

    #[test]
    fn font_style_struct_carries_bold_and_italic() {
        let bold_only = FontStyle {
            bold: true,
            italic: false,
        };
        assert!(bold_only.bold);
        assert!(!bold_only.italic);
        let italic_only = FontStyle {
            bold: false,
            italic: true,
        };
        assert!(!italic_only.bold);
        assert!(italic_only.italic);
    }

    #[test]
    fn heuristic_measurer_uses_font_style_bold() {
        let m = HeuristicTextMeasurer;
        let plain = m.measure_text_width("Hello", 12.0, FontStyle::default(), None, None);
        let bold = m.measure_text_width(
            "Hello",
            12.0,
            FontStyle {
                bold: true,
                italic: false,
            },
            None,
            None,
        );
        assert!(bold > plain, "bold={bold} should exceed plain={plain}");
    }

    #[test]
    fn apply_style_variations_returns_none_for_none_input() {
        let result = apply_style_variations(
            None,
            FontStyle {
                bold: true,
                italic: true,
            },
        );
        assert!(result.is_none());
    }

    #[test]
    fn heuristic_measurer_line_height_default() {
        let measurer = HeuristicTextMeasurer;
        let lh = measurer.get_line_height_ratio(None, None);
        assert_eq!(lh, DEFAULT_LINE_HEIGHT_RATIO);
    }

    #[test]
    fn heuristic_measurer_ascender_default() {
        let measurer = HeuristicTextMeasurer;
        let asc = measurer.get_ascender_ratio(None, None);
        assert_eq!(asc, DEFAULT_ASCENDER_RATIO);
    }

    #[test]
    fn heuristic_measurer_uses_calibri_metrics() {
        let measurer = HeuristicTextMeasurer;
        let lh = measurer.get_line_height_ratio(Some("Calibri"), None);
        let expected = (1950.0 + 550.0) / 2048.0;
        assert!((lh - expected).abs() < 1e-9);
    }

    // -- OpentypeTextMeasurer with no fonts → falls back to heuristic --------

    #[test]
    fn empty_opentype_falls_back_to_heuristic_width() {
        let measurer = empty_opentype();
        let opentype_width =
            measurer.measure_text_width("Hello", 18.0, FontStyle::default(), None, None);
        let heuristic_width = HeuristicTextMeasurer.measure_text_width(
            "Hello",
            18.0,
            FontStyle::default(),
            None,
            None,
        );
        assert_eq!(opentype_width, heuristic_width);
    }

    #[test]
    fn empty_opentype_returns_default_line_height() {
        let measurer = empty_opentype();
        assert_eq!(
            measurer.get_line_height_ratio(None, None),
            DEFAULT_LINE_HEIGHT_RATIO
        );
    }

    #[test]
    fn empty_opentype_returns_default_ascender() {
        let measurer = empty_opentype();
        assert_eq!(
            measurer.get_ascender_ratio(None, None),
            DEFAULT_ASCENDER_RATIO
        );
    }

    // -- resolver delegation: empty input / empty resolver returns None ------

    #[test]
    fn resolve_font_none_for_empty_input() {
        let measurer = empty_opentype();
        assert!(measurer.resolve_font(None).is_none());
        assert!(measurer.resolve_font(Some("")).is_none());
    }

    #[test]
    fn resolve_font_none_when_resolver_has_no_face() {
        // Buffer has no fonts → MappedResolver tries Calibri, mapping
        // gives Carlito, Buffer still misses → None.
        let measurer = empty_opentype();
        assert!(measurer.resolve_font(Some("Calibri")).is_none());
    }

    // -- with_resolver lets callers inject custom chains --------------------

    #[test]
    fn with_resolver_accepts_custom_chain() {
        // Use a no-op resolver (BufferFontResolver with no fonts) to
        // verify the constructor compiles and the type-check works.
        let resolver = BufferFontResolver::new();
        let measurer = OpentypeTextMeasurer::with_resolver(resolver, None);
        assert!(measurer.resolve_font(Some("Anything")).is_none());
    }
}
