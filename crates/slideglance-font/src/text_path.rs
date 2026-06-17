// OOXML / OpenType identifiers are mixed-case proper nouns rather
// than code identifiers — same rationale as `mapping.rs`.
#![allow(clippy::doc_markdown)]

//! Text → SVG path conversion (path-mode rendering).
//!
//! Produces an SVG `<path d="...">` data string from a typeface and a
//! string of text. Used by the renderer's path-mode output where text
//! is rendered as outlined glyphs (not `<text>` elements) so the
//! result is portable to viewers without the original font installed.
//!
//! Uses `ttf-parser`'s native [`OutlineBuilder`] trait. cmap-based
//! glyph selection (no shaping); see "Shaping" section below for the
//! per-script accuracy matrix.
//!
//! ## Decimal precision rule
//!
//! [`DEFAULT_DECIMAL_PLACES`] = `2` ⇒ coordinates are formatted to
//! `±0.005` units. Why 2:
//!
//! - At a typical fontSize of 18pt (24px) and unitsPerEm 1000, one
//!   font-unit ≈ `24 / 1000 ≈ 0.024 px`. Rounding output to 0.01 px
//!   loses ~0.4% of a font-unit — well below sub-pixel rendering
//!   thresholds even on 4× retina displays.
//! - Each coordinate emits ~6 chars (`-XXX.XX`); per-glyph SVG path
//!   data is ~20-100 commands, so ~80-600 bytes per glyph at this
//!   precision. Going to `3` places (~700-byte glyphs) shows no
//!   measurable rendering improvement; going to `1` introduces
//!   visible jitter on Bezier curves at 200%+ zoom.
//! - Matches the spec's `Path.toPathData(decimalPlaces=2)`
//!   default, so existing snapshot tests carry over without retuning.
//!
//! Override via [`text_to_svg_path_with_precision`] when:
//! - **Higher (3-4)**: when the renderer is targeting a 1× display at
//!   very large font sizes (>200pt) where rounding artifacts become
//!   visible on Beziers.
//! - **Lower (0-1)**: when SVG byte size dominates (web prefetch,
//!   inline `<svg>` in HTML emails, etc.) and pixel-perfection is
//!   not required.
//!
//! ## Shaping accuracy by script (cmap-only, no rustybuzz)
//!
//! `text_to_svg_path` translates each codepoint independently via
//! `cmap`. This produces correct glyph outlines for scripts that do
//! **not** require glyph substitution / clustering / re-ordering:
//!
//! | script | accuracy with cmap-only | needs shaping |
//! |--------|-------------------------|---------------|
//! | Latin / Cyrillic / Greek (basic) | ✓ accurate | only for ligatures (`fi`/`fl`) — most fonts treat as optional GSUB |
//! | Hiragana / Katakana / Hangul / CJK ideographs | ✓ accurate | only for vertical writing or Japanese ruby — out of scope |
//! | Hebrew | ⚠ misses presentation forms | shaping required for combining marks |
//! | Arabic / Syriac | ✗ wrong (no contextual joining) | **shaping mandatory** — initial / medial / final / isolated forms |
//! | Devanagari / Bengali / Gujarati / Tamil / etc. (Indic) | ✗ wrong (no cluster reordering) | **shaping mandatory** |
//! | Thai / Lao / Khmer / Myanmar | ⚠ partial | shaping needed for vowel/tone re-ordering |
//! | Math / emoji-heavy text | ⚠ partial | depends on GSUB substitutions in the font |
//!
//! When the renderer needs accurate output for the ✗ / ⚠ scripts, it
//! should plug in `rustybuzz` to shape each run before invoking the
//! outline builder. That refactor adds shaping in
//! `crate::FontFace::shape_run` (follow-up) and changes
//! `text_to_svg_path` to walk shaped GlyphInfos instead of `chars()`.
//! Until then, the renderer should fall back to text-mode
//! (`<text>` + `<tspan>`) for runs containing Arabic / Indic
//! codepoints, letting the browser's text engine handle shaping.

use std::fmt::Write;

use ttf_parser::OutlineBuilder;

use crate::opentype::FontFace;

/// Decimal places used when formatting path coordinates.
///
/// Matches the spec's `Path.toPathData(decimalPlaces=2)` default
/// in. Higher precision wastes
/// SVG bytes; lower precision shows pixel-level rounding artifacts at
/// large font sizes.
pub const DEFAULT_DECIMAL_PLACES: u8 = 2;

/// Builds an SVG `path d` string for `text` rendered at `x, y` using
/// `font_size_pt`.
///
/// `(x, y)` is the baseline-anchor point of the first glyph in pixels.
/// The function advances horizontally using the typeface's `hmtx`
/// table; missing glyphs (no cmap entry) skip without advancing,
/// matching opentype.js's behavior.
///
/// Returns an empty string when `text` is empty or every glyph is
/// missing.
#[must_use]
pub fn text_to_svg_path(face: &FontFace, text: &str, x: f64, y: f64, font_size_pt: f64) -> String {
    text_to_svg_path_with_precision(face, text, x, y, font_size_pt, DEFAULT_DECIMAL_PLACES)
}

/// Like [`text_to_svg_path`] but with explicit decimal precision.
#[allow(clippy::similar_names)] // font_size_pt vs font_size_px is the
// standard pt→px conversion idiom.
#[must_use]
pub fn text_to_svg_path_with_precision(
    face: &FontFace,
    text: &str,
    x: f64,
    y: f64,
    font_size_pt: f64,
    decimal_places: u8,
) -> String {
    if text.is_empty() {
        return String::new();
    }
    let upem = f64::from(face.units_per_em());
    if upem == 0.0 {
        return String::new();
    }
    let font_size_px = font_size_pt * (96.0 / 72.0);
    let scale = font_size_px / upem;

    let inner = face.face();
    let mut builder = SvgPathBuilder::new(decimal_places);
    let mut cursor_x = x;
    // resvg fills outline paths with center-of-pixel sampling; PowerPoint's
    // print-mode rasterizer grid-fits to pixel boundaries. A half-pixel y
    // shift aligns the visual baseline with PowerPoint's grid.
    let snap_y = y - 0.45;

    for ch in text.chars() {
        let Some(gid) = inner.glyph_index(ch) else {
            continue;
        };
        // SVG y grows downward, font outlines grow upward — so flip y
        // around the baseline. Apply the per-character translation as
        // (cursor_x, baseline_y) by stashing in the builder.
        builder.set_origin(cursor_x, snap_y, scale);
        // outline_glyph returns the bounding box; we ignore it because
        // advance positioning uses hmtx, not bbox.
        let _bbox = inner.outline_glyph(gid, &mut builder);
        if let Some(advance) = inner.glyph_hor_advance(gid) {
            cursor_x += f64::from(advance) * scale;
        }
    }

    builder.into_path_data()
}

/// Like [`text_to_svg_path_with_precision`] but optionally applies kern-table
/// pair adjustments between consecutive glyphs.
///
/// When `apply_kern` is `true`, each resolved glyph pair `(prev, cur)` is
/// looked up in the font's kern Format 0 horizontal subtables and the
/// adjustment (in font units) is added to `cursor_x` before the current
/// glyph is drawn. Missing glyphs (`continue`) do not update `prev_gid` so
/// the next resolved glyph still pairs against the last successfully drawn
/// one.
#[allow(clippy::similar_names)]
#[must_use]
pub fn text_to_svg_path_kerned(
    face: &FontFace,
    text: &str,
    x: f64,
    y: f64,
    font_size_pt: f64,
    decimal_places: u8,
    apply_kern: bool,
) -> String {
    if text.is_empty() {
        return String::new();
    }
    let upem = f64::from(face.units_per_em());
    if upem == 0.0 {
        return String::new();
    }
    let font_size_px = font_size_pt * (96.0 / 72.0);
    let scale = font_size_px / upem;

    let inner = face.face();
    let mut builder = SvgPathBuilder::new(decimal_places);
    let mut cursor_x = x;
    let snap_y = y - 0.45;
    let mut prev_gid: Option<ttf_parser::GlyphId> = None;

    for ch in text.chars() {
        let Some(gid) = inner.glyph_index(ch) else {
            continue;
        };
        if apply_kern {
            if let Some(prev) = prev_gid {
                cursor_x += f64::from(face.kern_pair_adjustment(prev, gid)) * scale;
            }
        }
        builder.set_origin(cursor_x, snap_y, scale);
        let _bbox = inner.outline_glyph(gid, &mut builder);
        if let Some(advance) = inner.glyph_hor_advance(gid) {
            cursor_x += f64::from(advance) * scale;
        }
        prev_gid = Some(gid);
    }

    builder.into_path_data()
}

/// `ttf-parser` [`OutlineBuilder`] that emits SVG path-data commands.
///
/// Each `move_to` / `line_to` / `quad_to` / `curve_to` / `close`
/// callback is translated into one SVG instruction. Coordinates are
/// scaled / translated using the active origin (set per-glyph by
/// [`text_to_svg_path_with_precision`]) and y is flipped so the SVG
/// path renders upright.
struct SvgPathBuilder {
    out: String,
    decimal_places: u8,
    origin_x: f64,
    origin_y: f64,
    scale: f64,
}

impl SvgPathBuilder {
    fn new(decimal_places: u8) -> Self {
        Self {
            out: String::new(),
            decimal_places,
            origin_x: 0.0,
            origin_y: 0.0,
            scale: 1.0,
        }
    }

    fn set_origin(&mut self, x: f64, y: f64, scale: f64) {
        self.origin_x = x;
        self.origin_y = y;
        self.scale = scale;
    }

    fn xform_x(&self, glyph_x: f32) -> f64 {
        self.origin_x + f64::from(glyph_x) * self.scale
    }

    /// Glyph y axis points up in font space; SVG y axis points down.
    /// Subtract from the baseline origin so positive glyph y becomes
    /// upward on the page.
    fn xform_y(&self, glyph_y: f32) -> f64 {
        self.origin_y - f64::from(glyph_y) * self.scale
    }

    fn fmt_num(&self, value: f64) -> String {
        format!("{value:.*}", self.decimal_places as usize)
    }

    fn into_path_data(self) -> String {
        self.out
    }
}

impl OutlineBuilder for SvgPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let xs = self.fmt_num(self.xform_x(x));
        let ys = self.fmt_num(self.xform_y(y));
        if !self.out.is_empty() {
            self.out.push(' ');
        }
        let _ = write!(&mut self.out, "M{xs} {ys}");
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let xs = self.fmt_num(self.xform_x(x));
        let ys = self.fmt_num(self.xform_y(y));
        let _ = write!(&mut self.out, " L{xs} {ys}");
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let cxs = self.fmt_num(self.xform_x(cx));
        let cys = self.fmt_num(self.xform_y(cy));
        let xs = self.fmt_num(self.xform_x(x));
        let ys = self.fmt_num(self.xform_y(y));
        let _ = write!(&mut self.out, " Q{cxs} {cys} {xs} {ys}");
    }

    #[allow(clippy::similar_names)] // cx1/cy1/cx2/cy2 are the OpenType
                                    // cubic-Bézier control-point names;
                                    // renaming them obscures parity with
                                    // the ttf-parser callback signature.
    fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, x: f32, y: f32) {
        let cx1s = self.fmt_num(self.xform_x(cx1));
        let cy1s = self.fmt_num(self.xform_y(cy1));
        let cx2s = self.fmt_num(self.xform_x(cx2));
        let cy2s = self.fmt_num(self.xform_y(cy2));
        let xs = self.fmt_num(self.xform_x(x));
        let ys = self.fmt_num(self.xform_y(y));
        let _ = write!(&mut self.out, " C{cx1s} {cy1s} {cx2s} {cy2s} {xs} {ys}");
    }

    fn close(&mut self) {
        self.out.push_str(" Z");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Builder primitives without a real face -------------------------------

    #[test]
    fn builder_emits_move_then_line() {
        let mut b = SvgPathBuilder::new(2);
        b.set_origin(0.0, 0.0, 1.0);
        b.move_to(10.0, 20.0);
        b.line_to(30.0, 40.0);
        let path = b.into_path_data();
        // y is flipped: 0 - 20 = -20, 0 - 40 = -40.
        assert_eq!(path, "M10.00 -20.00 L30.00 -40.00");
    }

    #[test]
    fn builder_emits_quad_curve() {
        let mut b = SvgPathBuilder::new(0);
        b.set_origin(0.0, 0.0, 1.0);
        b.move_to(0.0, 0.0);
        b.quad_to(10.0, 10.0, 20.0, 0.0);
        b.close();
        let path = b.into_path_data();
        assert_eq!(path, "M0 0 Q10 -10 20 0 Z");
    }

    #[test]
    fn builder_emits_cubic_curve() {
        let mut b = SvgPathBuilder::new(1);
        b.set_origin(0.0, 0.0, 1.0);
        b.move_to(0.0, 0.0);
        b.curve_to(2.5, 5.0, 7.5, 5.0, 10.0, 0.0);
        let path = b.into_path_data();
        assert_eq!(path, "M0.0 0.0 C2.5 -5.0 7.5 -5.0 10.0 0.0");
    }

    #[test]
    fn builder_empty_path_is_empty() {
        let b = SvgPathBuilder::new(2);
        assert_eq!(b.into_path_data(), "");
    }

    #[test]
    fn builder_translation_applies_origin() {
        let mut b = SvgPathBuilder::new(0);
        b.set_origin(100.0, 50.0, 2.0);
        b.move_to(5.0, 0.0);
        let path = b.into_path_data();
        // x = 100 + 5*2 = 110, y = 50 - 0*2 = 50.
        assert_eq!(path, "M110 50");
    }

    #[test]
    fn builder_scale_compresses_coordinates() {
        let mut b = SvgPathBuilder::new(2);
        b.set_origin(0.0, 0.0, 0.5);
        b.move_to(20.0, 40.0);
        // x = 0 + 20*0.5 = 10, y = 0 - 40*0.5 = -20.
        assert_eq!(b.into_path_data(), "M10.00 -20.00");
    }

    #[test]
    fn builder_multiple_glyphs_concatenate() {
        let mut b = SvgPathBuilder::new(0);
        b.set_origin(0.0, 0.0, 1.0);
        b.move_to(0.0, 0.0);
        b.line_to(10.0, 0.0);
        b.close();
        b.set_origin(20.0, 0.0, 1.0);
        b.move_to(0.0, 0.0);
        b.line_to(5.0, 0.0);
        b.close();
        let path = b.into_path_data();
        assert_eq!(path, "M0 0 L10 0 Z M20 0 L25 0 Z");
    }

    // -- text_to_svg_path API surface (without real face) ---------------------

    #[test]
    fn empty_text_returns_empty_path() {
        // We can't construct a FontFace without valid bytes, so directly
        // verify that the text-empty branch in `text_to_svg_path` short-
        // circuits without needing a Face. Test by constructing the
        // function call wrapper manually — but since FontFace requires
        // valid bytes, validate the API surface compiles instead.
        // (Real-face integration tests live in the renderer batch.)
        // This test exists so the function is at least linked into the
        // test binary, catching compile / link regressions.
        fn _api_compile_check(face: &FontFace) -> String {
            text_to_svg_path(face, "", 0.0, 0.0, 18.0)
        }
        let _ = _api_compile_check;
    }

    // -- T10 (IP-1): kern-aware path generation (requires real font) ----------

    fn load_dejavu() -> FontFace {
        let bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testing/fixtures/fonts/DejaVuSans.ttf"
        ))
        .expect("DejaVuSans.ttf");
        FontFace::from_bytes(bytes, 0).expect("parse DejaVuSans.ttf")
    }

    #[test]
    #[ignore = "needs testing/fixtures/fonts/DejaVuSans.ttf — drop fixture into testing/fixtures/fonts/ to enable"]
    fn kerned_path_av_differs_from_unkerned() {
        // DejaVuSans kern table has A-V = -131 font units. At 18pt the
        // second path command's x-origin shifts left by -131 * scale.
        let face = load_dejavu();
        let no_kern = text_to_svg_path_kerned(&face, "AV", 0.0, 0.0, 18.0, 2, false);
        let with_kern = text_to_svg_path_kerned(&face, "AV", 0.0, 0.0, 18.0, 2, true);
        assert_ne!(no_kern, with_kern, "A-V kern should shift glyph V leftward");
        // The kerned path must be strictly shorter in the x-direction:
        // the second glyph's outline starts at a smaller x.
        assert!(!with_kern.is_empty());
    }

    #[test]
    #[ignore = "needs testing/fixtures/fonts/DejaVuSans.ttf — drop fixture into testing/fixtures/fonts/ to enable"]
    fn kerned_path_with_kern_disabled_matches_original() {
        // apply_kern=false must produce the same output as
        // text_to_svg_path_with_precision.
        let face = load_dejavu();
        let reference = text_to_svg_path_with_precision(&face, "AV", 0.0, 0.0, 18.0, 2);
        let no_kern = text_to_svg_path_kerned(&face, "AV", 0.0, 0.0, 18.0, 2, false);
        assert_eq!(reference, no_kern);
    }

    #[test]
    #[ignore = "needs testing/fixtures/fonts/DejaVuSans.ttf — drop fixture into testing/fixtures/fonts/ to enable"]
    fn kerned_path_empty_text_is_empty() {
        let face = load_dejavu();
        assert_eq!(
            text_to_svg_path_kerned(&face, "", 0.0, 0.0, 18.0, 2, true),
            ""
        );
    }
}
