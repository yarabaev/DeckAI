//! Standalone wasm-bindgen entry point for slideglance text measurement.
//!
//! Why this crate is separate from `slideglance-wasm`
//! --------------------------------------------------
//! `slideglance-wasm` packages the full PPTX pipeline (parser +
//! renderer + resvg + serde) — measured at ~5 MiB compressed wasm. A
//! consumer that only needs to compute pixel advances for text runs
//! (e.g. an upstream layout engine sharing measurement primitives with
//! the renderer) does not need any of that — measurement only depends
//! on `slideglance-font`. Splitting the wasm boundary here keeps that
//! consumer's bundle ~10× smaller and lets the two ship on independent
//! release cadences without forcing every measurement bump to ride with
//! a parser bump.
//!
//! What ships here
//! ---------------
//! Exactly one wasm-bindgen-exported item: `TextMeasurer`. It wraps
//! `slideglance_font::OpentypeTextMeasurer` so name resolution mirrors
//! the renderer's chain (`Mapped(CjkFallback(Buffer))`); a host that
//! also drives `@slideglance/core`'s `convertPptxToSvg` gets identical
//! advance numbers in measurement and render.

#![allow(clippy::missing_const_for_fn, clippy::needless_pass_by_value)]

use slideglance_font::{
    standard_resolver_chain, BufferFontResolver, CjkPlatform, FontFace, FontMapping, FontStyle,
    HeuristicTextMeasurer, OpentypeTextMeasurer, TextMeasurer,
};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

/// Bold detection cutoff. PPTX `<a:rPr b="1">` maps to CSS weight 700,
/// but TTFs label themselves with `OS/2.usWeightClass` somewhere in the
/// 600 – 900 range (Pretendard-Bold is 700, Noto Sans Bold is 700,
/// Roboto Black is 900). 600 catches everything from Semibold up,
/// matching the cutoff CSS uses for the `bolder` keyword.
const BOLD_WEIGHT_CUTOFF: u16 = 600;

/// JS-facing standalone text measurer.
///
/// Construct once with a set of font byte buffers and reuse across many
/// `measureWidth` calls — the fonts are parsed exactly once at
/// construction time, which matters for callers that drive measurement
/// from a hot path (e.g. a layout engine's wrap callback firing per
/// word).
///
/// Bold variants are detected automatically: any face whose
/// `OS/2.usWeightClass >= 600` is registered into the resolver's
/// bold-variant slot under the same family name as the Regular face.
/// `measureWidth(..., bold=true, ...)` then resolves to the Bold face
/// directly, with no caller-side family rename hack.
#[wasm_bindgen(js_name = TextMeasurer)]
pub struct WasmTextMeasurer {
    inner: OpentypeTextMeasurer,
}

#[wasm_bindgen(js_class = TextMeasurer)]
impl WasmTextMeasurer {
    /// Build the measurer from font byte buffers. Each buffer is a
    /// TTF/OTF; the first face's family name (per the OpenType `name`
    /// table) becomes its key in the resolver. Buffers without a
    /// `name` table are silently skipped.
    ///
    /// `family_names` is an optional parallel array overriding the
    /// resolver key per buffer. When supplied it must have the same
    /// length as `fonts`; an empty / undefined entry means "fall back
    /// to the face's `family_name()`". Callers normally pass `None` —
    /// Bold faces are auto-routed to the bold-variant slot via their
    /// `OS/2.usWeightClass`. Override only when the face's `name`
    /// table family does not match the deck-side family the run will
    /// reference.
    ///
    /// # Errors
    ///
    /// Returns a JS-side `Error` whose `message` is either the
    /// `ttf-parser` failure for any buffer that fails to parse, or a
    /// length-mismatch description when `family_names` is supplied
    /// with a different length than `fonts`.
    #[wasm_bindgen(constructor)]
    pub fn new(
        fonts: Vec<js_sys::Uint8Array>,
        family_names: Option<Vec<String>>,
    ) -> Result<WasmTextMeasurer, JsError> {
        console_error_panic_hook::set_once();
        if let Some(names) = family_names.as_ref() {
            if names.len() != fonts.len() {
                return Err(JsError::new(&format!(
                    "family_names length ({}) does not match fonts length ({})",
                    names.len(),
                    fonts.len()
                )));
            }
        }
        let mut buffer = BufferFontResolver::new();
        for (i, ua) in fonts.iter().enumerate() {
            let bytes = ua.to_vec();
            let face = FontFace::from_bytes(bytes, 0)
                .map_err(|e| JsError::new(&format!("font {i} parse error: {e}")))?;
            let override_key = family_names
                .as_deref()
                .and_then(|n| n.get(i))
                .filter(|s| !s.is_empty())
                .cloned();
            let key = match override_key {
                Some(custom) => custom,
                None => match face.family_name() {
                    Some(name) => name,
                    None => continue,
                },
            };
            if face.weight() >= BOLD_WEIGHT_CUTOFF {
                buffer.insert_bold_variant(key, face);
            } else {
                buffer.insert(key, face);
            }
        }
        // Wrap the pre-populated buffer in the standard
        // `Mapped(CjkFallback(Buffer))` chain so name-mapping + CJK
        // fallback work the same way they do for `convertPptxToSvg`.
        // Using `with_resolver` here (instead of `from_fonts`) preserves
        // the bold-variant slots populated above; `from_fonts` builds
        // its own buffer from a name→face map and would discard them.
        let resolver = standard_resolver_chain(buffer, FontMapping::new(), CjkPlatform::Other);
        let inner = OpentypeTextMeasurer::with_resolver(resolver, None::<Arc<FontFace>>);
        Ok(WasmTextMeasurer { inner })
    }

    /// Pixel advance of `text` rendered at `font_size_pt`. `font_family`
    /// is the run's Latin family, `font_family_ea` the East-Asian
    /// family; either may be `null`/`undefined`. `bold` and `italic`
    /// flags drive variant lookup (Bold faces auto-registered by the
    /// constructor) and variable-axis selection on faces that expose
    /// `wght` / `ital` axes.
    #[wasm_bindgen(js_name = measureWidth)]
    pub fn measure_width(
        &self,
        text: &str,
        font_size_pt: f64,
        bold: bool,
        italic: bool,
        font_family: Option<String>,
        font_family_ea: Option<String>,
    ) -> f64 {
        let style = FontStyle { bold, italic };
        self.inner.measure_text_width(
            text,
            font_size_pt,
            style,
            font_family.as_deref(),
            font_family_ea.as_deref(),
        )
    }

    /// Natural line height as a multiple of the font size, derived from
    /// the resolved face's vertical metrics
    /// (`(ascender + |descender| + line_gap) / units_per_em`). Defaults
    /// to `HeuristicTextMeasurer`'s value when neither family resolves.
    #[wasm_bindgen(js_name = lineHeightRatio)]
    pub fn line_height_ratio(
        &self,
        font_family: Option<String>,
        font_family_ea: Option<String>,
    ) -> f64 {
        self.inner
            .get_line_height_ratio(font_family.as_deref(), font_family_ea.as_deref())
    }

    /// Ascender height as a multiple of the font size
    /// (`ascender / units_per_em`). Defaults to
    /// `HeuristicTextMeasurer`'s value when neither family resolves.
    #[wasm_bindgen(js_name = ascenderRatio)]
    pub fn ascender_ratio(
        &self,
        font_family: Option<String>,
        font_family_ea: Option<String>,
    ) -> f64 {
        self.inner
            .get_ascender_ratio(font_family.as_deref(), font_family_ea.as_deref())
    }
}

/// Returns the crate version. Smoke-test entry point.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// Silence `unused_imports` when the heuristic isn't directly referenced
// in tests — the type is part of the public surface that JS docs
// reference, and leaving it imported keeps the `cargo doc` link cheap.
#[allow(dead_code)]
type _AnchorHeuristic = HeuristicTextMeasurer;
