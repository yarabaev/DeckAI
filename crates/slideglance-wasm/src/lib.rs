//! WASM entry point for `slideglance`.
//!
//! Re-exports the [`slideglance::parse_pptx`] orchestrator so JS hosts can
//! pass a `Uint8Array` of PPTX bytes and receive a serialized
//! [`slideglance_model::Presentation`] via `serde-wasm-bindgen`.

// wasm-bindgen ABI requires owned types for arguments crossing the JS
// boundary; clippy's needless-pass-by-value lint doesn't recognize the
// constraint, so we silence it module-wide.
#![allow(clippy::missing_const_for_fn, clippy::needless_pass_by_value)]

use serde::Serialize;
use slideglance::{
    convert_to_png, convert_to_svg, parse_pptx, ConvertOptions, PptxDocument as RsPptxDocument,
    SlideRenderOptions, TypefaceUsage,
};
use slideglance_font::{
    standard_resolver_chain, BufferFontResolver, CjkPlatform, FontFace, FontMapping, FontResolver,
    FontStyle, TextMeasurer,
};
use slideglance_png::{svg_to_png, FontData, PngOptions};
use slideglance_utils::Emu;
use std::collections::BTreeMap;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// JS callback that returns the text's pixel advance for the given
    /// font / size / weight. Implemented in the viewer worker via
    /// `OffscreenCanvas.measureText`. The host is responsible for
    /// installing the matching `font-kerning="none"` / `letter-spacing`
    /// settings on the canvas context so the measured width matches
    /// what the browser will render for the same `<text>` element.
    ///
    /// `font_family` / `font_family_ea` are the run's authored Latin /
    /// East-Asian families (either may be `None`); the JS side decides
    /// the fallback chain (typically: deck embed → system Korean font
    /// → sans-serif). `font_size_pt` is in PostScript points; `bold`
    /// is the run's authored boldness.
    ///
    /// Hosts that don't install a measurer should leave `PptxDocument
    /// .new(..., useCanvasMeasurer = false)` and the renderer falls
    /// back to the auto OpenType measurer (deck embed fonts only) or
    /// the heuristic.
    #[wasm_bindgen(js_namespace = self, js_name = "__slideglanceMeasureText")]
    fn js_measure_text(
        text: &str,
        font_family: Option<String>,
        font_family_ea: Option<String>,
        font_family_chain: Option<String>,
        font_size_pt: f64,
        bold: bool,
    ) -> f64;

    /// JS callback that returns `{ ascent, descent, lineGap }` pixel
    /// metrics for the given CSS font declaration string, measured via
    /// `OffscreenCanvas.measureText("Mg")`.
    ///
    /// When both `ascent` and `descent` are 0 the Rust side treats the
    /// result as unavailable and falls back to `HeuristicTextMeasurer`
    /// with a `console.warn`. Returning null / undefined has the same
    /// effect — `JsValue::null()` → `as_f64()` returns `None`.
    #[wasm_bindgen(js_namespace = self, js_name = "__slideglanceMeasureLineMetrics")]
    fn js_measure_line_metrics(font_decl: &str) -> JsValue;
}

/// Vertical metrics extracted from a JS `{ ascent, descent, lineGap }` object.
struct LineMetrics {
    ascent: f64,
    descent: f64,
    line_gap: f64,
}

/// Parse a `JsValue` returned by `__slideglanceMeasureLineMetrics` into `LineMetrics`.
///
/// Returns `None` when both `ascent` and `descent` are 0 (browser returned
/// generic fallback metrics) or when the value is missing required fields.
fn parse_line_metrics(val: JsValue) -> Option<LineMetrics> {
    use js_sys::Reflect;
    let ascent = Reflect::get(&val, &"ascent".into()).ok()?.as_f64()?;
    let descent = Reflect::get(&val, &"descent".into()).ok()?.as_f64()?;
    let line_gap = Reflect::get(&val, &"lineGap".into())
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    // Both zero means the browser returned placeholder metrics — treat as missing.
    if ascent == 0.0 && descent == 0.0 {
        return None;
    }
    Some(LineMetrics {
        ascent,
        descent,
        line_gap,
    })
}

/// Reference font size (in CSS pixels) used when probing line metrics.
/// 16px is the CSS root default and convenient for em-relative ratios:
/// `(ascent + |descent|) / METRICS_FONT_SIZE_PX` yields the same em
/// ratio that path-mode computes via `(face.ascender + |descender|) /
/// upem`. The absolute value is irrelevant — canvas font metrics scale
/// linearly with font size — but it must match the divisor in the
/// line-height ratio formula.
const METRICS_FONT_SIZE_PX: f64 = 16.0;

/// Build a CSS font declaration string for line-metrics probing.
///
/// Uses [`METRICS_FONT_SIZE_PX`] as the reference size; canvas
/// measureText ratios are scale-invariant for most fonts so the
/// absolute size does not matter as long as the formula's divisor
/// matches.
fn build_font_decl_for_metrics(font_family: Option<&str>, font_family_ea: Option<&str>) -> String {
    let family = font_family_ea.or(font_family).unwrap_or("sans-serif");
    // Escape embedded single quotes to keep the CSS font-family value valid.
    format!(
        "{:.0}px '{}'",
        METRICS_FONT_SIZE_PX,
        family.replace('\'', "\\'")
    )
}

/// `TextMeasurer` impl that delegates every measurement to the host's
/// `__slideglanceMeasureText` JS function. Use only when the host installed
/// that function (else every measurement panics).
struct JsCanvasTextMeasurer;

impl TextMeasurer for JsCanvasTextMeasurer {
    fn measure_text_width(
        &self,
        text: &str,
        font_size_pt: f64,
        style: FontStyle,
        font_family: Option<&str>,
        font_family_ea: Option<&str>,
    ) -> f64 {
        // The JS bridge receives only the `bold` flag today; `italic`
        // is plumbed to D4 (KDD-3 wasm bridge) for a follow-up `__slideglanceMeasureText`
        // signature change. For now italic is silently ignored on the JS side
        // — same width as upright.
        js_measure_text(
            text,
            font_family.map(String::from),
            font_family_ea.map(String::from),
            None,
            font_size_pt,
            style.bold,
        )
    }

    /// KDD-15 chain-aware override: when the renderer pre-computes the
    /// CSS `font-family` chain that the SVG `<text>` element will
    /// declare, pass it verbatim to the canvas so measurement and
    /// render share the exact same font fallback order. Eliminates
    /// wrap drift between browser-rendered SVG and the wrap pass.
    fn measure_text_width_with_chain(
        &self,
        text: &str,
        font_size_pt: f64,
        style: FontStyle,
        font_family: Option<&str>,
        font_family_ea: Option<&str>,
        font_family_chain: Option<&str>,
    ) -> f64 {
        js_measure_text(
            text,
            font_family.map(String::from),
            font_family_ea.map(String::from),
            font_family_chain.map(String::from),
            font_size_pt,
            style.bold,
        )
    }

    fn get_line_height_ratio(
        &self,
        font_family: Option<&str>,
        font_family_ea: Option<&str>,
    ) -> f64 {
        let font_decl = build_font_decl_for_metrics(font_family, font_family_ea);
        if let Some(m) = parse_line_metrics(js_measure_line_metrics(&font_decl)) {
            let total = m.ascent + m.descent + m.line_gap;
            if total == 0.0 {
                return slideglance_font::HeuristicTextMeasurer
                    .get_line_height_ratio(font_family, font_family_ea);
            }
            // L2 unification: em-relative ratio matching path-mode
            // formula `(face.ascender + |descender|) / upem`. Worker
            // returns `fontBoundingBox*` metrics (em-aligned, font-wide),
            // so dividing by the reference font size yields the em ratio.
            total / METRICS_FONT_SIZE_PX
        } else {
            web_sys::console::warn_1(
                &"[slideglance] __slideglanceMeasureLineMetrics returned zero metrics \
                  — falling back to heuristic line height"
                    .into(),
            );
            slideglance_font::HeuristicTextMeasurer
                .get_line_height_ratio(font_family, font_family_ea)
        }
    }

    fn get_ascender_ratio(&self, font_family: Option<&str>, font_family_ea: Option<&str>) -> f64 {
        let font_decl = build_font_decl_for_metrics(font_family, font_family_ea);
        if let Some(m) = parse_line_metrics(js_measure_line_metrics(&font_decl)) {
            let total = m.ascent + m.descent + m.line_gap;
            if total == 0.0 {
                return slideglance_font::HeuristicTextMeasurer
                    .get_ascender_ratio(font_family, font_family_ea);
            }
            // L2 unification: em-relative ratio matching path-mode
            // formula `face.ascender / upem`. With fontBoundingBox
            // metrics, ascent/METRICS_FONT_SIZE_PX is the design ratio.
            m.ascent / METRICS_FONT_SIZE_PX
        } else {
            web_sys::console::warn_1(
                &"[slideglance] __slideglanceMeasureLineMetrics returned zero metrics \
                  — falling back to heuristic ascender ratio"
                    .into(),
            );
            slideglance_font::HeuristicTextMeasurer.get_ascender_ratio(font_family, font_family_ea)
        }
    }
}

/// `wasm-bindgen` start hook. Installs a panic hook that forwards Rust
/// panics to the host's `console.error`, making debugging WASM crashes in
/// browsers tractable.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Detect the host operating system from the JavaScript global's
/// `navigator` fingerprint and map it to the [`CjkPlatform`] variant
/// used by the renderer for per-OS Latin and CJK fallback chains.
///
/// In `wasm32-unknown-unknown` builds [`CjkPlatform::current`] reads
/// [`std::env::consts::OS`], which is `"unknown"` regardless of the
/// host running the browser — the renderer would then fall back to
/// [`CjkPlatform::Other`]'s conservative fallback list (`Arial`,
/// `Liberation Sans`) on every browser, even when the user is on
/// macOS or Windows. Detecting the platform from `navigator` lets the
/// chain emit `Helvetica Neue` / `Segoe UI` / `Liberation Sans`
/// according to the actual host so users see fonts that exist on
/// their machine.
///
/// **Worker compatibility.** The viewer runs every WASM call from
/// inside a Web Worker. Workers don't have a `Window` global —
/// [`web_sys::window`] returns `None` inside one — so this helper
/// reaches the global scope via [`js_sys::global`] (which is the
/// `WorkerGlobalScope` in workers and the `Window` in main threads)
/// and reads `navigator.platform` / `navigator.userAgent` dynamically
/// through [`js_sys::Reflect::get`]. Both `WorkerNavigator` and
/// `Navigator` expose those properties on their interfaces, so the
/// dynamic read works without feature-gating either web-sys
/// interface.
///
/// Strategy:
/// 1. Try `navigator.platform` first — short, stable, and matches
///    on every browser that still supports it. We accept the
///    deprecation: the alternative (`navigator.userAgentData`) is
///    Chromium-only and asynchronous.
/// 2. Fall back to `navigator.userAgent` substring matching for
///    cases where `platform` is `""` (some privacy modes return the
///    empty string).
/// 3. Final fallback is [`CjkPlatform::Other`] — same conservative
///    list the native default uses for unknown targets.
fn detect_browser_platform() -> CjkPlatform {
    let global = js_sys::global();
    let Ok(navigator) = js_sys::Reflect::get(&global, &JsValue::from_str("navigator")) else {
        return CjkPlatform::Other;
    };
    if navigator.is_undefined() || navigator.is_null() {
        return CjkPlatform::Other;
    }

    // navigator.platform — deprecated but still present in every
    // shipping browser. Returns short strings like "MacIntel",
    // "Win32", "Linux x86_64".
    if let Ok(value) = js_sys::Reflect::get(&navigator, &JsValue::from_str("platform")) {
        if let Some(platform) = value.as_string() {
            let lower = platform.to_lowercase();
            if lower.contains("mac") {
                return CjkPlatform::MacOs;
            }
            if lower.starts_with("win") {
                return CjkPlatform::Windows;
            }
            if lower.contains("linux") {
                return CjkPlatform::Linux;
            }
        }
    }

    // navigator.userAgent — fall back when platform is empty /
    // spoofed by privacy-focused browsers / extensions.
    if let Ok(value) = js_sys::Reflect::get(&navigator, &JsValue::from_str("userAgent")) {
        if let Some(ua) = value.as_string() {
            let lower = ua.to_lowercase();
            if lower.contains("mac os x") || lower.contains("macintosh") {
                return CjkPlatform::MacOs;
            }
            if lower.contains("windows") {
                return CjkPlatform::Windows;
            }
            if lower.contains("linux") || lower.contains("x11") {
                return CjkPlatform::Linux;
            }
        }
    }

    CjkPlatform::Other
}

/// Returns the crate version.
///
/// Smoke-test entry point for verifying that the WASM module loads correctly
/// in JS hosts before any real conversion APIs are wired up.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Converts an EMU value to pixels at 96 DPI.
///
/// Exposed as an end-to-end smoke test that the `slideglance-utils` crate links into
/// WASM correctly.
#[wasm_bindgen(js_name = emuToPixels)]
pub fn emu_to_pixels(emu: f64) -> f64 {
    Emu::from_f64(emu).to_pixels()
}

/// Parse a PPTX byte stream into a JS-side [`Presentation`] object.
///
/// `bytes` is consumed: the WASM caller can release the original
/// `Uint8Array` once this function returns. The returned `JsValue` is the
/// serde-serialized [`slideglance_model::Presentation`] structure (uses
/// [`serde-wasm-bindgen`] under the hood — call sites get plain JS objects
/// rather than `JsValue::Object` references).
///
/// # Errors
///
/// Returns a JS-side `Error` whose `message` is the underlying
/// [`slideglance::PptxError`] formatted via `Display` (zip parse failure,
/// missing `ppt/presentation.xml`, malformed XML, …).
#[wasm_bindgen(js_name = parsePptxData)]
pub fn parse_pptx_data(bytes: Vec<u8>) -> Result<JsValue, JsError> {
    let presentation = parse_pptx(bytes).map_err(|e| JsError::new(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&presentation).map_err(|e| JsError::new(&e.to_string()))
}

/// One slide's SVG output. Mirrors `slideglance::SlideSvg` for JS consumers
/// (serde-serialized to a plain JS object via serde-wasm-bindgen).
#[derive(Serialize)]
struct SlideSvgJs {
    slide_number: u32,
    svg: String,
    notes: Option<String>,
    layout_name: Option<String>,
    section_name: Option<String>,
}

/// One slide's PNG output. Mirrors `slideglance::SlideImage`. The PNG bytes
/// are serialized as `Vec<u8>` — serde-wasm-bindgen lifts that to a JS
/// `Uint8Array`.
#[derive(Serialize)]
struct SlideImageJs {
    slide_number: u32,
    #[serde(with = "serde_bytes")]
    png: Vec<u8>,
    width: u32,
    height: u32,
    notes: Option<String>,
    layout_name: Option<String>,
    section_name: Option<String>,
}

/// Build a `BufferFontResolver` populated with the supplied font byte
/// buffers. Each buffer's first face is registered under every family
/// name it advertises, so callers can pass system-extracted TTFs and
/// have them resolved by family. TTC collections are expanded.
fn build_font_resolver(fonts: &[Vec<u8>]) -> Result<Box<dyn FontResolver + Send + Sync>, JsError> {
    let mut buffer = BufferFontResolver::new();
    for (i, bytes) in fonts.iter().enumerate() {
        let face = FontFace::from_bytes(bytes.clone(), 0)
            .map_err(|e| JsError::new(&format!("font {i} parse error: {e}")))?;
        // family_name() is None for fonts without a `name` table; skip
        // those rather than registering under an empty key, which would
        // collide across multiple unnamed fonts.
        let Some(family) = face.family_name() else {
            continue;
        };
        buffer.insert_arc(family, Arc::new(face));
    }
    Ok(Box::new(standard_resolver_chain(
        buffer,
        FontMapping::new(),
        detect_browser_platform(),
    )))
}

// `TextMeasurer` previously shipped here. It moved to the dedicated
// `slideglance-measure-wasm` crate (npm package `@slideglance/measure`)
// so consumers that only need text measurement do not pull in the full
// PPTX parser + renderer + resvg payload that this crate links. This
// crate (`slideglance-wasm`, npm `@slideglance/core`) keeps everything
// PPTX-specific; for measurement-only use cases install
// `@slideglance/measure` instead.

/// Convert a PPTX byte stream to one SVG document per slide.
///
/// Returns a JS array of `{ slideNumber, svg, notes?, layoutName?,
/// sectionName? }` objects, mirroring the TS `convertPptxToSvg` shape.
///
/// `slides` filters by 1-based slide number; pass an empty array for
/// "all slides". `fonts` enables path-mode rendering when non-empty;
/// each entry is a TTF/OTF/TTC byte buffer registered by family name.
///
/// # Errors
///
/// JS-side `Error` whose `message` is the underlying
/// [`slideglance::ConvertError`] formatted via `Display`.
#[wasm_bindgen(js_name = convertPptxToSvg)]
pub fn convert_pptx_to_svg(
    bytes: Vec<u8>,
    slides: Vec<u32>,
    fonts: Vec<js_sys::Uint8Array>,
) -> Result<JsValue, JsError> {
    let font_buffers: Vec<Vec<u8>> = fonts.iter().map(js_sys::Uint8Array::to_vec).collect();
    let resolver = if font_buffers.is_empty() {
        None
    } else {
        Some(build_font_resolver(&font_buffers)?)
    };

    let opts = ConvertOptions {
        slides: if slides.is_empty() {
            None
        } else {
            Some(slides)
        },
        fonts: slideglance::FontConfig {
            resolver,
            ..slideglance::FontConfig::default()
        },
        // Override `Other` (the wasm32-unknown-unknown default) with
        // the browser-detected platform so per-OS font fallback lists
        // match the actual host. See `detect_browser_platform`.
        cjk_platform: detect_browser_platform(),
        ..ConvertOptions::default()
    };
    let result = convert_to_svg(bytes, &opts).map_err(|e| JsError::new(&e.to_string()))?;
    let js: Vec<SlideSvgJs> = result
        .into_iter()
        .map(|s| SlideSvgJs {
            slide_number: s.slide_number,
            svg: s.svg,
            notes: s.notes,
            layout_name: s.layout_name,
            section_name: s.section_name,
        })
        .collect();
    serde_wasm_bindgen::to_value(&js).map_err(|e| JsError::new(&e.to_string()))
}

/// Convert a PPTX byte stream to one PNG byte buffer per slide.
///
/// `width` overrides the intrinsic slide width (preserving aspect
/// ratio); `height` is honored only when `width` is unset. `fonts` is
/// **required** — PNG rasterization always runs in path-mode and
/// resvg cannot resolve system fonts.
///
/// # Errors
///
/// JS-side `Error` whose `message` is the underlying
/// [`slideglance::ConvertError`] formatted via `Display`. Returns
/// `FontResolverRequiredForPng` when `fonts` is empty.
#[wasm_bindgen(js_name = convertPptxToPng)]
pub fn convert_pptx_to_png(
    bytes: Vec<u8>,
    slides: Vec<u32>,
    width: Option<u32>,
    height: Option<u32>,
    fonts: Vec<js_sys::Uint8Array>,
) -> Result<JsValue, JsError> {
    let font_buffers: Vec<Vec<u8>> = fonts.iter().map(js_sys::Uint8Array::to_vec).collect();
    if font_buffers.is_empty() {
        web_sys::console::warn_1(
            &"[slideglance] convertPptxToPng called with no font buffers — text glyphs \
              will be absent in the PNG output. Pass TTF/OTF buffers via the `fonts` \
              argument."
                .into(),
        );
    }
    let resolver = if font_buffers.is_empty() {
        None
    } else {
        Some(build_font_resolver(&font_buffers)?)
    };
    // KDD-22: rasterizer fontdb is derived from `inline_fonts`. Wrap
    // every supplied buffer as an `AdditionalFont` registered under the
    // first family name the font's `name` table reports.
    let inline_fonts: Vec<slideglance::AdditionalFont> = font_buffers
        .iter()
        .filter_map(|bytes| {
            let typeface = slideglance_font::all_face_family_names(bytes)
                .into_iter()
                .next()?;
            Some(slideglance::AdditionalFont::regular(
                typeface,
                bytes.clone(),
            ))
        })
        .collect();

    let opts = ConvertOptions {
        slides: if slides.is_empty() {
            None
        } else {
            Some(slides)
        },
        width,
        height,
        fonts: slideglance::FontConfig {
            resolver,
            inline_fonts,
            ..slideglance::FontConfig::default()
        },
        ..ConvertOptions::default()
    };
    let result = convert_to_png(bytes, &opts).map_err(|e| JsError::new(&e.to_string()))?;
    let js: Vec<SlideImageJs> = result
        .into_iter()
        .map(|s| SlideImageJs {
            slide_number: s.slide_number,
            png: s.png,
            width: s.width,
            height: s.height,
            notes: s.notes,
            layout_name: s.layout_name,
            section_name: s.section_name,
        })
        .collect();
    serde_wasm_bindgen::to_value(&js).map_err(|e| JsError::new(&e.to_string()))
}

/// Rasterize an SVG document to PNG bytes via the deterministic
/// `slideglance-png` pipeline.
///
/// `svg` is the SVG source as a UTF-8 string. `width` overrides the
/// intrinsic SVG width (preserving aspect ratio); pass `None` for the
/// SVG's own size. `fonts` is a list of font byte buffers — every glyph
/// the SVG references must be in this list, since the rasterizer never
/// touches the host's system fonts.
///
/// # Errors
///
/// Returns a JS-side `Error` whose `message` is the underlying
/// [`slideglance_png::PngError`] formatted via `Display` (SVG parse failure,
/// invalid output dimensions, PNG encoder failure).
#[wasm_bindgen(js_name = svgToPng)]
pub fn svg_to_png_wasm(
    svg: &str,
    width: Option<u32>,
    height: Option<u32>,
    fonts: Vec<js_sys::Uint8Array>,
) -> Result<Vec<u8>, JsError> {
    let font_data: Vec<FontData> = fonts
        .into_iter()
        .map(|a| FontData::new(a.to_vec()))
        .collect();
    let options = PngOptions {
        width,
        height,
        fonts: font_data,
    };
    let out = svg_to_png(svg, &options).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(out.png)
}

/// Probe the JS global for the two canvas-measurer callbacks and emit a
/// `console.warn` for each one that is missing.
///
/// Called when `use_canvas_measurer = true` so callers get an actionable
/// message instead of silent heuristic fallback.
fn check_canvas_measurer_callbacks() {
    use js_sys::Reflect;
    let global = js_sys::global();
    let measure_text_ok =
        Reflect::get(&global, &"__slideglanceMeasureText".into()).is_ok_and(|v| v.is_function());
    let measure_metrics_ok = Reflect::get(&global, &"__slideglanceMeasureLineMetrics".into())
        .is_ok_and(|v| v.is_function());
    if !measure_text_ok {
        // Missing __slideglanceMeasureText causes a TypeError on the first renderSlide
        // call (JS extern throws when the global is undefined). This warn fires
        // eagerly at document construction so callers can diagnose the issue before
        // the first render attempt.
        web_sys::console::warn_1(
            &"[slideglance] useCanvasMeasurer=true but __slideglanceMeasureText is not \
              registered — first renderSlide call will fail with a TypeError. \
              Install @slideglance/viewer or register the callback manually."
                .into(),
        );
    }
    if !measure_metrics_ok {
        // Missing __slideglanceMeasureLineMetrics causes a TypeError on the first
        // get_line_height_ratio / get_ascender_ratio call. Warn eagerly.
        web_sys::console::warn_1(
            &"[slideglance] useCanvasMeasurer=true but __slideglanceMeasureLineMetrics is \
              not registered — line-height measurement will fail on first render. \
              Install @slideglance/viewer or register the callback manually."
                .into(),
        );
    }
}

/// JS-facing wrapper around [`slideglance::PptxDocument`] for stateful,
/// per-slide rendering. Construct once with `new PptxDocument(bytes,
/// measurementFonts)` from JavaScript and reuse the instance across
/// `renderSlide` calls — parsing the archive happens exactly once.
#[wasm_bindgen]
pub struct PptxDocument {
    inner: RsPptxDocument,
    /// When `true`, every `renderSlide` call substitutes the host's
    /// canvas-backed measurer for the auto OpenType / heuristic one.
    /// Defaults to `false` so non-browser hosts (Node.js) keep the
    /// previous behaviour.
    use_canvas_measurer: bool,
}

#[derive(Serialize)]
struct WireMediaBlob {
    mime: String,
    #[serde(with = "serde_bytes")]
    bytes: Vec<u8>,
}

/// Wire form of [`slideglance::TypefaceUsage`] — same fields, `snake_case`
/// preserved on the JS side because the viewer types already use
/// `requested` / `fallback_chain` / `resolved_family` verbatim.
#[derive(Serialize)]
struct TypefaceUsageJs {
    requested: String,
    fallback_chain: Vec<String>,
    resolved_family: Option<String>,
}

/// Wire form of [`slideglance::CompressedEmbeddedFont`] — keyed in
/// camelCase (`family`, `payload`) because the viewer worker reads
/// these directly without a TypefaceUsage-style `snake_case` adapter.
#[derive(Serialize)]
struct MtxCompressedFontJs {
    family: String,
    weight: String,
    style: String,
    #[serde(with = "serde_bytes")]
    payload: Vec<u8>,
}

#[derive(Serialize)]
struct WireRenderedSlide {
    slide_number: u32,
    svg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    layout_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    section_name: Option<String>,
    media: BTreeMap<String, WireMediaBlob>,
}

#[wasm_bindgen]
impl PptxDocument {
    /// Parse a PPTX byte stream and prepare it for incremental
    /// rendering. `measurement_fonts` is an array of `Uint8Array`
    /// font byte buffers used only for text measurement (system-font
    /// substitutes); pass an empty array to skip.
    ///
    /// `use_canvas_measurer` switches every wrap measurement onto a
    /// JS-side bridge that calls `__slideglanceMeasureText` (typically
    /// implemented with `OffscreenCanvas.measureText` in a worker).
    /// When this is `true` the host MUST install that function on the
    /// global scope before any `renderSlide` call. Use this in the
    /// browser viewer so wrap positions match what the same browser
    /// will actually render — even when the deck's authored fonts
    /// aren't embedded and the renderer falls back through the system
    /// font chain. Leave `false` (default) for Node.js / non-browser
    /// hosts that have no canvas.
    #[wasm_bindgen(constructor)]
    pub fn new(
        bytes: Vec<u8>,
        measurement_fonts: Vec<js_sys::Uint8Array>,
        use_canvas_measurer: Option<bool>,
    ) -> Result<Self, JsError> {
        let use_canvas = use_canvas_measurer.unwrap_or(false);
        if use_canvas {
            check_canvas_measurer_callbacks();
        }
        let measurement: Vec<Vec<u8>> = measurement_fonts.into_iter().map(|a| a.to_vec()).collect();
        let inner = RsPptxDocument::parse(bytes, &[], &measurement, true)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self {
            inner,
            use_canvas_measurer: use_canvas,
        })
    }

    /// Number of slides in the deck.
    #[wasm_bindgen(js_name = slideCount)]
    pub fn slide_count(&self) -> u32 {
        self.inner.slide_count()
    }

    /// Per-typeface report describing how every deck-referenced
    /// typeface name resolves through the SVG `font-family` fallback
    /// chain.
    ///
    /// Returns an array of `{ requested, fallback_chain, resolved_family }`
    /// objects. Hosts (e.g. the viewer's status bar) walk
    /// `fallback_chain` against `document.fonts.check()` to identify
    /// the actually-rendered font for each authored typeface and
    /// surface the mapping to the user.
    ///
    /// `resolved_family` is always `null` here — the WASM document
    /// holds no path-mode resolver. Native CLI hosts that want
    /// resolved-family info should call the Rust-side
    /// `PptxDocument::font_usage` directly with their resolver.
    #[wasm_bindgen(js_name = fontUsage)]
    pub fn font_usage(&self) -> Result<JsValue, JsError> {
        let usage = self
            .inner
            .font_usage(&FontMapping::new(), detect_browser_platform(), None);
        let wire: Vec<TypefaceUsageJs> = usage
            .into_iter()
            .map(
                |TypefaceUsage {
                     requested,
                     fallback_chain,
                     resolved_family,
                 }| TypefaceUsageJs {
                    requested,
                    fallback_chain,
                    resolved_family,
                },
            )
            .collect();
        serde_wasm_bindgen::to_value(&wire).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Deck-wide `<defs>…@font-face…</defs>` block. Empty when the
    /// deck has no embedded fonts.
    #[wasm_bindgen(js_name = fontDefs)]
    pub fn font_defs(&self) -> String {
        self.inner.font_defs().to_string()
    }

    /// `MicroType` Express compressed `<p:embeddedFont>` payloads the
    /// Rust pipeline can't decompress (Agfa Monotype LZ77 + adaptive
    /// Huffman). Returned as a JS array of
    /// `{ family, weight, style, payload }` objects where `payload` is
    /// a `Uint8Array` of the post-EOT-header bytes (XOR de-obfuscation
    /// already applied).
    ///
    /// Browser hosts that bundle `mtx-decompressor` walk this list,
    /// decode each `payload`, and register the resulting TTF as a
    /// `FontFace`. Hosts without that capability ignore the list and
    /// the renderer falls back through the family-name chain. Always
    /// returns an empty array when the deck has no embedded fonts.
    #[wasm_bindgen(js_name = mtxCompressedFonts)]
    pub fn mtx_compressed_fonts(&self) -> Result<JsValue, JsError> {
        let entries: Vec<MtxCompressedFontJs> = self
            .inner
            .compressed_embedded_fonts()
            .iter()
            .map(|f| MtxCompressedFontJs {
                family: f.typeface.clone(),
                weight: f.weight.to_string(),
                style: f.style.to_string(),
                payload: f.payload.clone(),
            })
            .collect();
        serde_wasm_bindgen::to_value(&entries).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Render one 1-based slide. Returns `null` when `slide` is out
    /// of range.
    #[wasm_bindgen(js_name = renderSlide)]
    pub fn render_slide(
        &self,
        slide: u32,
        external_media: bool,
        include_font_defs: bool,
    ) -> Result<JsValue, JsError> {
        let canvas_measurer;
        let measurer_ref: Option<&dyn TextMeasurer> = if self.use_canvas_measurer {
            canvas_measurer = JsCanvasTextMeasurer;
            Some(&canvas_measurer as &dyn TextMeasurer)
        } else {
            None
        };
        let opts = SlideRenderOptions {
            external_media,
            include_font_defs,
            measurer: measurer_ref,
            // Default-resolved CjkPlatform::current() is `Other` in
            // wasm32-unknown-unknown — explicitly override with the
            // browser-detected platform so the renderer's chain uses
            // the correct OS-specific Latin and CJK fallback lists.
            cjk_platform: Some(detect_browser_platform()),
            ..SlideRenderOptions::default()
        };
        let rendered = self
            .inner
            .render_slide(slide, &opts)
            .map_err(|e| JsError::new(&e.to_string()))?;
        let Some(rendered) = rendered else {
            return Ok(JsValue::NULL);
        };
        let media: BTreeMap<String, WireMediaBlob> = rendered
            .media
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    WireMediaBlob {
                        mime: v.mime,
                        bytes: v.bytes,
                    },
                )
            })
            .collect();
        let wire = WireRenderedSlide {
            slide_number: rendered.slide_number,
            svg: rendered.svg,
            notes: rendered.notes,
            layout_name: rendered.layout_name,
            section_name: rendered.section_name,
            media,
        };
        serde_wasm_bindgen::to_value(&wire).map_err(|e| JsError::new(&e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::build_font_decl_for_metrics;

    #[test]
    fn font_decl_uses_ea_over_latin() {
        let s = build_font_decl_for_metrics(Some("Calibri"), Some("맑은 고딕"));
        assert_eq!(s, "16px '맑은 고딕'");
    }

    #[test]
    fn font_decl_falls_back_to_latin_when_no_ea() {
        let s = build_font_decl_for_metrics(Some("Calibri"), None);
        assert_eq!(s, "16px 'Calibri'");
    }

    #[test]
    fn font_decl_uses_sans_serif_when_both_none() {
        let s = build_font_decl_for_metrics(None, None);
        assert_eq!(s, "16px 'sans-serif'");
    }

    #[test]
    fn font_decl_escapes_single_quotes() {
        let s = build_font_decl_for_metrics(Some("Font'Name"), None);
        assert_eq!(s, "16px 'Font\\'Name'");
    }
}
