//! SVG -> PNG rasterization for `slideglance`.
//!
//! Wraps [`resvg`] / [`usvg`] / [`tiny_skia`] with a deterministic
//! configuration suitable for PPTX rendering:
//!
//! - **No system fonts.** `usvg::Options::fontdb` is populated only from
//!   font byte buffers handed in by the caller. Calling
//!   `Database::load_system_fonts` would make output depend on the
//!   machine's installed fonts and break Rust-native ↔ WASM bit-equality
//!   (plan.md Phase 6 deliverable).
//! - **Geometric precision.** Text and shape rendering use
//!   [`usvg::TextRendering::GeometricPrecision`] and
//!   [`usvg::ShapeRendering::GeometricPrecision`]; image rendering uses
//!   [`usvg::ImageRendering::OptimizeQuality`]. These are the
//!   project-wide pixel-equivalence settings recorded in
//!   `.plans/00-rust-migration/plan.md` lines 524-532.
//!
//! The spec wraps
//! `@resvg/resvg-wasm` with its defaults. This crate intentionally
//! diverges by setting the rendering options above — the divergence is
//! recorded in plan.md and applies whenever PNG output deterministicness
//! across native vs. WASM is needed.
//!
//! Also note: TS forces `textRenderMode="path"` for PNG because
//! resvg-wasm cannot resolve system fonts during rasterization. Rust
//! achieves the same result by passing `font_resolver: Some(_)` into
//! [`slideglance_renderer::render_text_body`] before SVG generation.

#![deny(missing_docs)]

use std::sync::Arc;

use thiserror::Error;
use usvg::fontdb;
use usvg::fontdb::Source;

/// Default DPI used when the SVG omits explicit width / height. Matches
/// the default `usvg::Options::dpi`.
pub const DEFAULT_DPI: f32 = 96.0;

/// Caller-supplied font byte buffer.
///
/// PPTX rasterization needs deterministic font lookup, so the caller is
/// responsible for providing every face the SVG references. TrueType
/// collections (TTC) are expanded automatically — every face in the
/// buffer is registered.
#[derive(Debug, Clone)]
pub struct FontData {
    /// Raw font file bytes (TTF / OTF / TTC).
    pub bytes: Vec<u8>,
}

impl FontData {
    /// Wrap a TTF / OTF / TTC byte buffer. All faces in collections are
    /// registered.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

/// Conversion options.
///
/// `width` takes precedence over `height` when both are provided —
/// matches the TS contract in. When neither is
/// set the SVG's intrinsic size (read from `<svg width=…>` /
/// `<svg viewBox=…>`) is used.
#[derive(Debug, Default, Clone)]
pub struct PngOptions {
    /// Output width in pixels. Drives the fit mode when set.
    pub width: Option<u32>,
    /// Output height in pixels. Honored only when `width` is `None`.
    pub height: Option<u32>,
    /// Font byte buffers to register with the fontdb. Every glyph the
    /// SVG references must be available here — see crate docs.
    pub fonts: Vec<FontData>,
}

/// Successful PNG rasterization result.
#[derive(Debug, Clone)]
pub struct PngOutput {
    /// PNG-encoded byte buffer (8-bit RGBA).
    pub png: Vec<u8>,
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
}

/// PNG conversion failure.
#[derive(Debug, Error)]
pub enum PngError {
    /// `usvg` failed to parse the SVG document.
    #[error("svg parse error: {0}")]
    Parse(#[from] usvg::Error),
    /// The requested or implied output size was zero or otherwise
    /// non-positive — `tiny_skia::Pixmap::new` rejects this.
    #[error("invalid output dimensions: width={width}, height={height}")]
    InvalidDimensions {
        /// Requested width in pixels.
        width: u32,
        /// Requested height in pixels.
        height: u32,
    },
    /// `tiny_skia` failed to encode the rasterized pixmap to PNG.
    #[error("png encode error: {0}")]
    Encode(String),
}

/// Convert one SVG document to PNG bytes.
///
/// # Errors
///
/// - [`PngError::Parse`] if `usvg` rejects the SVG.
/// - [`PngError::InvalidDimensions`] if the requested or implied output
///   dimensions are zero.
/// - [`PngError::Encode`] if PNG encoding of the rasterized pixmap
///   fails.
pub fn svg_to_png(svg: &str, options: &PngOptions) -> Result<PngOutput, PngError> {
    let usvg_options = build_usvg_options(&options.fonts);
    let tree = usvg::Tree::from_str(svg, &usvg_options)?;

    let intrinsic = tree.size();
    let (target_width, target_height) = target_dimensions(
        options.width,
        options.height,
        intrinsic.width(),
        intrinsic.height(),
    );
    if target_width == 0 || target_height == 0 {
        return Err(PngError::InvalidDimensions {
            width: target_width,
            height: target_height,
        });
    }

    let scale_x = f32::from(u16::try_from(target_width).unwrap_or(u16::MAX)) / intrinsic.width();
    let scale_y = f32::from(u16::try_from(target_height).unwrap_or(u16::MAX)) / intrinsic.height();
    let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);

    let mut pixmap =
        tiny_skia::Pixmap::new(target_width, target_height).ok_or(PngError::InvalidDimensions {
            width: target_width,
            height: target_height,
        })?;
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let png = pixmap
        .encode_png()
        .map_err(|e| PngError::Encode(e.to_string()))?;
    Ok(PngOutput {
        png,
        width: target_width,
        height: target_height,
    })
}

/// Build the deterministic `usvg::Options` that PPTX rasterization
/// requires. See crate docs for the rationale behind each setting.
//
// The GeometricPrecision / OptimizeQuality settings are an intentional
// divergence from the spec (`@resvg/resvg-wasm` defaults). See
// `.plans/00-rust-migration/plan.md` Phase 6 — "TS 의도적 분기" — for
// why: native ↔ WASM bit-equality + machine-independence.
fn build_usvg_options(fonts: &[FontData]) -> usvg::Options<'static> {
    let mut db = fontdb::Database::new();
    for font in fonts {
        // load_font_source registers every face in collections (TTC).
        // We never call load_system_fonts: caller-supplied bytes only,
        // for deterministic output across machines and the WASM target.
        db.load_font_source(Source::Binary(Arc::new(font.bytes.clone())));
    }
    register_alias_family_names(&mut db, fonts);

    let mut opts = usvg::Options {
        text_rendering: usvg::TextRendering::GeometricPrecision,
        shape_rendering: usvg::ShapeRendering::GeometricPrecision,
        image_rendering: usvg::ImageRendering::OptimizeQuality,
        dpi: DEFAULT_DPI,
        ..usvg::Options::default()
    };
    opts.fontdb = Arc::new(db);
    opts
}

/// Walk every loaded font face and re-register it under every `name`-
/// table family alias the file declares, not just the single
/// "typographic family" `fontdb` keeps by default.
///
/// `fontdb` 0.23 stores one entry per face keyed by the typographic
/// family (`name` id 16; e.g. `"Freesentation"`) plus the localized
/// variant (id 16 in lang 0x0412 → `"프리젠테이션"`). PPTX
/// `<a:latin typeface="…"/>` strings, however, routinely carry the
/// **id-1 family** which embeds the weight subfamily
/// (`"Freesentation 7 Bold"`, `"프리젠테이션 7 Bold"`), the **full
/// name** (id 4), or the **WWS-style family** (id 21). Without this
/// pass `Database::query` misses every weight-suffixed lookup and
/// resvg falls back to its default sans-serif — yielding the wrong
/// glyph shape on Korean / Japanese / Chinese decks even when the
/// exact font is installed.
///
/// The pass is byte-source agnostic (works for caller-supplied fonts,
/// PPTX-embedded fonts, and host system fonts loaded via
/// `dir_to_png`) and writes no global state — every alias is added to
/// the per-render `Database` only.
fn register_alias_family_names(db: &mut fontdb::Database, fonts: &[FontData]) {
    use std::collections::BTreeSet;
    // Snapshot existing (family-name, weight, style, stretch) tuples
    // so we don't keep adding duplicate aliases on repeat invocations
    // or when the typographic family already covers the alias.
    let mut existing: BTreeSet<(String, u16, u16, u8)> = BTreeSet::new();
    for face in db.faces() {
        for (name, _) in &face.families {
            existing.insert((
                name.clone(),
                face.weight.0,
                face.stretch.to_number(),
                style_to_u8(&face.style),
            ));
        }
    }

    for font in fonts {
        let bytes = &font.bytes;
        let face_count = ttf_parser::fonts_in_collection(bytes).unwrap_or(1);
        for face_index in 0..face_count {
            let Ok(face) = ttf_parser::Face::parse(bytes, face_index) else {
                continue;
            };
            let weight = face.weight().to_number();
            let stretch_num = face.width().to_number();
            let style = if face.is_italic() {
                ttf_parser::Style::Italic
            } else if face.is_oblique() {
                ttf_parser::Style::Oblique
            } else {
                ttf_parser::Style::Normal
            };
            let alias_names = collect_family_aliases(&face);
            // Find a representative existing FaceInfo for this face so
            // we can clone its non-name fields (weight / style / etc.)
            // and only swap in the new family list.
            let template_id = db.faces().find_map(|f| {
                if f.weight.0 == weight
                    && f.stretch.to_number() == stretch_num
                    && style_to_u8(&f.style) == style_to_u8(&style)
                    && f.families
                        .iter()
                        .any(|(name, _)| alias_names.iter().any(|alias| alias == name))
                {
                    Some(f.id)
                } else {
                    None
                }
            });
            let Some(template_id) = template_id else {
                continue;
            };
            // Re-register the same byte source under each alias the
            // db doesn't yet cover for (weight, stretch, style).
            for alias in &alias_names {
                let key = (alias.clone(), weight, stretch_num, style_to_u8(&style));
                if existing.contains(&key) {
                    continue;
                }
                let template = db.face(template_id).cloned();
                let Some(mut new_info) = template else {
                    continue;
                };
                new_info.id = fontdb::ID::dummy();
                new_info.families = vec![(alias.clone(), fontdb::Language::English_UnitedStates)];
                db.push_face_info(new_info);
                existing.insert(key);
            }
        }
    }
}

fn collect_family_aliases(face: &ttf_parser::Face<'_>) -> Vec<String> {
    let names = face.names();
    let mut out: Vec<String> = Vec::new();
    for i in 0..names.len() {
        let Some(rec) = names.get(i) else { continue };
        // id 1: Family, 4: Full Name, 16: Typographic Family, 21: WWS
        if !matches!(rec.name_id, 1 | 4 | 16 | 21) {
            continue;
        }
        let Some(s) = rec.to_string() else { continue };
        let s = s.trim();
        if s.is_empty() {
            continue;
        }
        if !out.iter().any(|x| x == s) {
            out.push(s.to_string());
        }
    }
    out
}

/// Style enum is duplicated between `ttf-parser` (raw font tables) and
/// `usvg::fontdb` (loaded face info). Normalize to the same `u8` so
/// `existing` set comparisons work across both worlds.
trait StyleAsU8 {
    fn as_u8(&self) -> u8;
}
impl StyleAsU8 for ttf_parser::Style {
    fn as_u8(&self) -> u8 {
        match self {
            ttf_parser::Style::Normal => 0,
            ttf_parser::Style::Italic => 1,
            ttf_parser::Style::Oblique => 2,
        }
    }
}
impl StyleAsU8 for fontdb::Style {
    fn as_u8(&self) -> u8 {
        match self {
            fontdb::Style::Normal => 0,
            fontdb::Style::Italic => 1,
            fontdb::Style::Oblique => 2,
        }
    }
}
fn style_to_u8<S: StyleAsU8>(s: &S) -> u8 {
    s.as_u8()
}

/// Resolve the (width, height) the rasterizer should target, mirroring
/// TS :
///
/// - explicit width → fit-to-width preserving aspect ratio.
/// - explicit height → fit-to-height preserving aspect ratio.
/// - neither → intrinsic SVG size, rounded to the nearest pixel.
fn target_dimensions(
    requested_width: Option<u32>,
    requested_height: Option<u32>,
    intrinsic_width: f32,
    intrinsic_height: f32,
) -> (u32, u32) {
    if let Some(w) = requested_width {
        let scale = f32::from(u16::try_from(w).unwrap_or(u16::MAX)) / intrinsic_width;
        let h = (intrinsic_height * scale).round().max(1.0) as u32;
        return (w, h);
    }
    if let Some(h) = requested_height {
        let scale = f32::from(u16::try_from(h).unwrap_or(u16::MAX)) / intrinsic_height;
        let w = (intrinsic_width * scale).round().max(1.0) as u32;
        return (w, h);
    }
    (
        intrinsic_width.round().max(1.0) as u32,
        intrinsic_height.round().max(1.0) as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal SVG: a 100x50 red rectangle.
    const RED_RECT_SVG: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50" width="100" height="50">
 <rect width="100" height="50" fill="#FF0000"/>
</svg>"##;

    /// PNG file signature bytes.
    const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    fn assert_png_signature(png: &[u8]) {
        assert!(png.len() > 8, "PNG too short: {} bytes", png.len());
        assert_eq!(&png[..8], &PNG_SIGNATURE, "not a PNG signature");
    }

    #[test]
    fn red_rect_intrinsic_size() {
        let out = svg_to_png(RED_RECT_SVG, &PngOptions::default()).expect("render");
        assert_eq!(out.width, 100);
        assert_eq!(out.height, 50);
        assert_png_signature(&out.png);
    }

    #[test]
    fn red_rect_explicit_width_scales_height_proportionally() {
        let opts = PngOptions {
            width: Some(200),
            ..PngOptions::default()
        };
        let out = svg_to_png(RED_RECT_SVG, &opts).expect("render");
        assert_eq!(out.width, 200);
        assert_eq!(out.height, 100);
        assert_png_signature(&out.png);
    }

    #[test]
    fn red_rect_explicit_height_scales_width_proportionally() {
        let opts = PngOptions {
            height: Some(100),
            ..PngOptions::default()
        };
        let out = svg_to_png(RED_RECT_SVG, &opts).expect("render");
        assert_eq!(out.width, 200);
        assert_eq!(out.height, 100);
    }

    #[test]
    fn width_takes_precedence_over_height() {
        // TS contract: width wins.
        let opts = PngOptions {
            width: Some(200),
            height: Some(999),
            ..PngOptions::default()
        };
        let out = svg_to_png(RED_RECT_SVG, &opts).expect("render");
        assert_eq!(out.width, 200);
        assert_eq!(out.height, 100);
    }

    #[test]
    fn invalid_svg_returns_parse_error() {
        let err = svg_to_png("<not-svg/>", &PngOptions::default()).expect_err("must fail");
        matches!(err, PngError::Parse(_));
    }

    #[test]
    fn output_is_deterministic() {
        // Two consecutive renders of the same SVG must be byte-identical.
        let a = svg_to_png(RED_RECT_SVG, &PngOptions::default()).expect("render");
        let b = svg_to_png(RED_RECT_SVG, &PngOptions::default()).expect("render");
        assert_eq!(a.png, b.png, "non-deterministic output");
    }

    #[test]
    fn render_pixel_is_red() {
        // Decode the PNG and confirm the (0, 0) pixel is the red we
        // requested. Catches regressions where e.g. the wrong color
        // space or alpha is emitted.
        let out = svg_to_png(RED_RECT_SVG, &PngOptions::default()).expect("render");
        // PNG -> RGBA pixmap; tiny_skia's Pixmap::decode_png returns the raw bitmap.
        let pixmap = tiny_skia::Pixmap::decode_png(&out.png).expect("decode");
        let pixel = pixmap.pixel(0, 0).expect("origin pixel");
        assert_eq!(pixel.red(), 0xFF, "red channel");
        assert_eq!(pixel.green(), 0x00, "green channel");
        assert_eq!(pixel.blue(), 0x00, "blue channel");
    }

    #[test]
    fn target_dimensions_neither_returns_intrinsic() {
        assert_eq!(target_dimensions(None, None, 100.0, 50.0), (100, 50));
    }

    #[test]
    fn target_dimensions_with_width_scales_height() {
        assert_eq!(target_dimensions(Some(200), None, 100.0, 50.0), (200, 100));
    }

    #[test]
    fn target_dimensions_with_height_scales_width() {
        assert_eq!(target_dimensions(None, Some(100), 100.0, 50.0), (200, 100));
    }

    #[test]
    fn target_dimensions_width_wins_when_both_set() {
        assert_eq!(
            target_dimensions(Some(200), Some(999), 100.0, 50.0),
            (200, 100)
        );
    }

    #[test]
    fn target_dimensions_clamps_zero_to_one_pixel() {
        // Vanishingly small intrinsic, no override -> at least 1px each.
        assert_eq!(target_dimensions(None, None, 0.4, 0.4), (1, 1));
    }

    #[test]
    fn font_data_wraps_bytes_verbatim() {
        let f = FontData::new(vec![1, 2, 3]);
        assert_eq!(f.bytes, vec![1, 2, 3]);
    }

    #[test]
    fn empty_fonts_yields_empty_db() {
        let opts = build_usvg_options(&[]);
        assert_eq!(opts.fontdb.len(), 0);
    }

    #[test]
    fn invalid_font_bytes_register_zero_faces_without_panic() {
        // Garbage bytes are silently rejected by fontdb's parser — the
        // database stays empty, no panic. Exercises the load_font_source
        // path with non-font input.
        let bogus = FontData::new(vec![0xFF; 32]);
        let opts = build_usvg_options(&[bogus]);
        assert_eq!(opts.fontdb.len(), 0);
    }
}
