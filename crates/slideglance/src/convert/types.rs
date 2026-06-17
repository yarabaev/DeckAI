//! Public types for the conversion pipelines.
//!
//! Extracted from `convert.rs` so the type surface (FontConfig,
//! ConvertOptions, SlideSvg, SlideImage, ConvertError) lives apart
//! from the `convert_to_svg` / `convert_to_png` logic that consumes
//! it.

use slideglance_font::CjkPlatform;
use slideglance_font::{FontMapping, FontResolver, RenderMode, TextMeasurer};
use slideglance_png::PngError;
use slideglance_renderer::{RendererError, Timestamp};

use crate::embedded_fonts::{AdditionalFont, EmbedFormat};
use crate::PptxError;

/// Consolidated font configuration for the conversion pipelines.
///
/// Replaces the five separate font-related fields previously scattered
/// across [`ConvertOptions`] (`font_resolver`, `embed_fonts`,
/// `additional_fonts`, `measurement_fonts`, `png_fonts`). Per KDD-22
/// the resvg fontdb is derived from `inline_fonts` at conversion time;
/// `png_fonts` is therefore no longer a separate field.
pub struct FontConfig {
    /// Path-mode font resolver. Required for PNG; optional for SVG.
    /// `None` means text-mode (glyphs rendered as `<text>` elements).
    pub resolver: Option<Box<dyn FontResolver + Send + Sync>>,
    /// When `true` (default), `<p:embeddedFontLst>` font binaries are
    /// extracted from the archive and inlined into every slide's SVG
    /// as `@font-face` rules with `data:` URLs.
    pub embed_deck_fonts: bool,
    /// Caller-supplied font binaries inlined into every SVG as
    /// `@font-face` rules — even when the deck doesn't embed its own
    /// fonts. Use this to guarantee that the rendered SVG carries the
    /// **actual** typefaces the deck declares.
    pub inline_fonts: Vec<AdditionalFont>,
    /// Extra font byte buffers for the auto-built
    /// [`OpentypeTextMeasurer`] only — they are **not** inlined into
    /// the SVG and not handed to the PNG rasterizer.
    pub measurement_only_fonts: Vec<Vec<u8>>,
    /// Output format for inlined font bytes. See [`EmbedFormat`].
    pub embed_format: EmbedFormat,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            resolver: None,
            // Match the legacy `ConvertOptions::embed_fonts` default
            // (`true`) so existing callers see no behavior change.
            embed_deck_fonts: true,
            inline_fonts: Vec::new(),
            measurement_only_fonts: Vec::new(),
            embed_format: EmbedFormat::Auto,
        }
    }
}

/// Conversion options shared by SVG and PNG pipelines.
///
/// When `fonts.resolver` is `Some(_)` the renderer dispatches to path-
/// mode (glyph outlines as `<path>`). PNG conversion requires this.
///
/// # Removed fields (D0 KDD-12, KDD-22)
///
/// The five legacy flat fields were folded into [`FontConfig`]. The
/// `compile_fail` blocks below stand watch — if a future merge ever
/// re-introduces any of these names on `ConvertOptions`, doc-tests
/// fail and the regression is caught.
///
/// `font_resolver` → `fonts.resolver`:
/// ```compile_fail
/// use slideglance::ConvertOptions;
/// let _ = ConvertOptions {
///     font_resolver: None,
///     ..ConvertOptions::default()
/// };
/// ```
///
/// `png_fonts` → derived from `fonts.inline_fonts`:
/// ```compile_fail
/// use slideglance::ConvertOptions;
/// let _ = ConvertOptions {
///     png_fonts: Vec::new(),
///     ..ConvertOptions::default()
/// };
/// ```
///
/// `measurement_fonts` → `fonts.measurement_only_fonts`:
/// ```compile_fail
/// use slideglance::ConvertOptions;
/// let _: ConvertOptions<'_> = ConvertOptions {
///     measurement_fonts: Vec::<Vec<u8>>::new(),
///     ..ConvertOptions::default()
/// };
/// ```
///
/// `additional_fonts` → `fonts.inline_fonts`:
/// ```compile_fail
/// use slideglance::ConvertOptions;
/// let _: ConvertOptions<'_> = ConvertOptions {
///     additional_fonts: Vec::new(),
///     ..ConvertOptions::default()
/// };
/// ```
///
/// `embed_fonts` → `fonts.embed_deck_fonts`:
/// ```compile_fail
/// use slideglance::ConvertOptions;
/// let _: ConvertOptions<'_> = ConvertOptions {
///     embed_fonts: true,
///     ..ConvertOptions::default()
/// };
/// ```
pub struct ConvertOptions<'a> {
    /// Subset of slides to render (1-based numbers). `None` = all slides.
    pub slides: Option<Vec<u32>>,
    /// Output width in pixels. PNG-only; ignored for SVG output.
    pub width: Option<u32>,
    /// Output height in pixels. Honored only when `width` is `None`.
    /// PNG-only.
    pub height: Option<u32>,
    /// PPTX font family → OSS substitute mapping.
    pub mapping: FontMapping,
    /// CJK platform for font-family list construction.
    pub cjk_platform: CjkPlatform,
    /// Font configuration: resolver, embed flags, inline / measurement
    /// font buffers, and embed format. Replaces the previous five
    /// scattered font-related fields (KDD-12, KDD-22).
    pub fonts: FontConfig,
    /// Text measurer for layout + autofit. `None` defaults to
    /// [`HeuristicTextMeasurer`].
    pub measurer: Option<&'a dyn TextMeasurer>,
    /// Wall-clock timestamp for `datetime{N}` field substitution. `None`
    /// leaves the placeholder text the parser extracted in place. The
    /// renderer never reads the host clock — supplying a deterministic
    /// `Timestamp` is required for VRT and WASM ↔ native bit equality.
    pub timestamp: Option<Timestamp>,
}

impl Default for ConvertOptions<'_> {
    fn default() -> Self {
        Self {
            slides: None,
            width: None,
            height: None,
            mapping: FontMapping::new(),
            // Auto-detect the host OS so the SVG `font-family` chain
            // includes the local equivalents of every PPTX-authored
            // CJK typeface (Yu Gothic → Hiragino Sans on macOS, etc.).
            // Deterministic-output callers (VRT, byte-level diffs)
            // should override this to `CjkPlatform::Other` to keep
            // snapshots stable across machines.
            cjk_platform: CjkPlatform::current(),
            fonts: FontConfig::default(),
            measurer: None,
            timestamp: None,
        }
    }
}

/// One rendered slide as SVG + per-slide metadata. Mirrors TS
/// `SlideSvg`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideSvg {
    /// 1-based slide number.
    pub slide_number: u32,
    /// Self-contained SVG document.
    pub svg: String,
    /// Speaker notes from `notesSlide{N}.xml`, if present.
    pub notes: Option<String>,
    /// Layout name (`<p:cSld @name>` on the slide layout), if present.
    pub layout_name: Option<String>,
    /// Section name from `<p14:section>`, if the slide belongs to a
    /// section.
    pub section_name: Option<String>,
    /// Render mode actually used for this slide (KDD-11). `PathMode`
    /// when a font resolver was active; `TextMode` otherwise.
    pub render_mode: RenderMode,
    /// `true` when the host's system-font fallback chain was reached for
    /// at least one run on the slide (KDD-7, R-C2). Default `false`;
    /// D3-T13 sets this to `true` once the renderer can detect FSP step
    /// 4 (system fallback) actually firing during glyph extraction.
    pub fallback_used: bool,
}

/// One rendered slide as PNG bytes + per-slide metadata. Mirrors TS
/// `SlideImage`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideImage {
    /// 1-based slide number.
    pub slide_number: u32,
    /// PNG-encoded byte buffer (8-bit RGBA).
    pub png: Vec<u8>,
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Speaker notes (same as [`SlideSvg::notes`]).
    pub notes: Option<String>,
    /// Layout name (same as [`SlideSvg::layout_name`]).
    pub layout_name: Option<String>,
    /// Section name (same as [`SlideSvg::section_name`]).
    pub section_name: Option<String>,
}

/// Failures reachable from the conversion pipelines.
#[derive(Debug)]
pub enum ConvertError {
    /// PPTX parse / archive failure.
    Pptx(PptxError),
    /// Renderer reported a `NotImplemented` or other failure for one
    /// slide. The slide number is included for context.
    Renderer {
        /// 1-based slide number that failed.
        slide_number: u32,
        /// Underlying renderer error.
        source: RendererError,
    },
    /// PNG rasterization failure for one slide.
    Png {
        /// 1-based slide number that failed.
        slide_number: u32,
        /// Underlying PNG error.
        source: PngError,
    },
    /// `convert_to_png` was called without a `font_resolver`. resvg
    /// cannot rasterize text without glyph outlines, so PNG conversion
    /// requires path-mode.
    FontResolverRequiredForPng,
}

impl std::fmt::Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pptx(e) => write!(f, "pptx parse error: {e}"),
            Self::Renderer {
                slide_number,
                source,
            } => write!(f, "renderer error on slide {slide_number}: {source:?}"),
            Self::Png {
                slide_number,
                source,
            } => write!(f, "png error on slide {slide_number}: {source}"),
            Self::FontResolverRequiredForPng => write!(
 f,
 "convert_to_png requires a font_resolver — resvg cannot render text without glyph outlines"
 ),
        }
    }
}

impl std::error::Error for ConvertError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pptx(e) => Some(e),
            Self::Png { source, .. } => Some(source),
            // RendererError doesn't currently impl Error; it's still useful
            // via Debug. Promote when RendererError gets std::error::Error.
            Self::Renderer { .. } | Self::FontResolverRequiredForPng => None,
        }
    }
}

impl From<PptxError> for ConvertError {
    fn from(e: PptxError) -> Self {
        Self::Pptx(e)
    }
}
