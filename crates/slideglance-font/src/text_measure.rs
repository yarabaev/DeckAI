//! Text width / line-height estimation.
//!
//! Mirrors 1:1. Uses the
//! offline-extracted character width tables from
//! [`crate::fallback_metrics`] when the requested font has them, and
//! falls back to a category-based heuristic otherwise.
//!
//! ## Heuristic ratios (no metrics available)
//!
//! | category | ratio (× fontSize) | members |
//! |----------|-------------------|---------|
//! | narrow   | 0.30 | space, `! , . : ; i j l 1 \| ' ( ) [ ] { }` |
//! | normal   | 0.60 | every other Latin / non-CJK code point |
//! | wide     | 1.00 | every CJK code point per [`is_cjk_codepoint`] |
//!
//! Bold faces multiply Latin widths by `1.05`; CJK characters are not
//! widened. This matches TS `BOLD_FACTOR` exactly.
//!
//! ## CJK detection
//!
//! [`is_cjk_codepoint`] covers CJK ideographs, Hiragana / Katakana,
//! full-width Latin / symbols, CJK Compatibility Ideographs, CJK
//! Unified Ideographs Extension B, and the full Hangul ranges. Used
//! both for category assignment and for [`crate::text_wrap`]'s
//! segmentation heuristic.

use crate::fallback_metrics::{get_font_metrics, FontMetrics};
use crate::font_metric::metric_for_family;

/// `96 / 72` — pixel per typographic point (CSS reference DPI).
const PX_PER_PT: f64 = 96.0 / 72.0;
/// Latin width multiplier applied when the run is bold.
pub(crate) const BOLD_FACTOR: f64 = 1.05;
/// Heuristic line-height ratio when no font metrics are available
/// (matches CSS `normal` line-height for default UA fonts).
const DEFAULT_LINE_HEIGHT_RATIO: f64 = 1.2;
/// Heuristic ascender ratio when no font metrics are available.
const DEFAULT_ASCENDER_RATIO: f64 = 1.0;

/// Heuristic width ratio for narrow Latin glyphs (× fontSize).
const NARROW_RATIO: f64 = 0.3;
/// Heuristic width ratio for normal Latin glyphs (× fontSize).
const NORMAL_RATIO: f64 = 0.6;
/// Heuristic width ratio for wide / CJK glyphs (× fontSize).
///
/// `PowerPoint`'s actual rendered advance for CJK ideographs is roughly
/// `0.95 em` for the common KR / SC / TC / JP sans-serif faces (the
/// remaining 0.05 em is glyph side-bearing inside the em-square). Using
/// `1.0` here over-measures Korean / Chinese / Japanese paragraphs by
/// ~5 % and triggers wraps on lines that fit in `PowerPoint` — most
/// visibly on layout placeholders that pack glyphs into a tight box
/// (e.g. Korean `정 성 제 안 서` in a 113 px badge).
const WIDE_RATIO: f64 = 0.95;
/// Width ratio for halfwidth CJK glyphs (U+FF61-FF9F Halfwidth Katakana,
/// U+FFA0-FFDC Halfwidth Hangul). These are half the em-square wide, so
/// they measure more like Latin glyphs than full-width CJK.
pub const HALFWIDTH_RATIO: f64 = 0.5;

// ── Script classification ────────────────────────────────────────────────────

/// The rendering script of a text segment.
///
/// Used by `split_by_script` and the renderer's font-selection pass.
/// CJK Script Equality rule: all four CJK scripts are treated symmetrically.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Script {
    /// Latin / non-CJK — uses the run's primary (`latin`) font.
    Latin,
    /// Hangul (Korean). ISO 15924 `Hang`.
    Korean,
    /// Hiragana / Katakana / Japanese-only ranges. ISO 15924 `Jpan`.
    Japanese,
    /// CJK Unified Ideographs and CJK-majority ranges defaulting to
    /// Simplified Chinese. ISO 15924 `Hans`.
    SimplifiedChinese,
    /// Traditional Chinese when explicitly classified; currently falls
    /// back to `SimplifiedChinese` for shared CJK blocks (no reliable
    /// Unicode range separation without `lang` metadata).
    /// ISO 15924 `Hant`.
    TraditionalChinese,
    /// Private Use Area: U+E000–EFFF and U+F100–F8FF → EA font;
    /// U+F000–F0FF → sym font (handled separately in the renderer).
    Pua,
}

impl Script {
    /// Returns `true` when this script should use the East-Asian (EA)
    /// font rather than the Latin font.
    #[must_use]
    pub fn is_ea(self) -> bool {
        matches!(
            self,
            Script::Korean
                | Script::Japanese
                | Script::SimplifiedChinese
                | Script::TraditionalChinese
                | Script::Pua
        )
    }
}

/// A segment of text with a uniform script classification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextPart {
    /// The text fragment.
    pub text: String,
    /// Script detected for this fragment.
    pub script: Script,
}

/// Classify a single Unicode codepoint into a [`Script`].
///
/// Korean (Hangul) blocks → [`Script::Korean`].
/// Hiragana / Katakana → [`Script::Japanese`].
/// CJK Unified Ideographs and other shared CJK → [`Script::SimplifiedChinese`].
/// PUA U+E000–EFFF or U+F100–F8FF → [`Script::Pua`].
/// Everything else (including U+F000–F0FF sym-PUA) → [`Script::Latin`].
#[must_use]
pub fn classify_script(code_point: u32) -> Script {
    match code_point {
        // Hangul Jamo / Compatibility Jamo / Jamo Extended-A / Syllables / Jamo Extended-B
        // Halfwidth Hangul (U+FFA0-FFDC)
        0x1100..=0x11FF
        | 0x3130..=0x318F
        | 0xA960..=0xA97F
        | 0xAC00..=0xD7A3
        | 0xD7B0..=0xD7FF
        | 0xFFA0..=0xFFDC => Script::Korean,
        // Hiragana / Katakana / Katakana Phonetic Extensions
        0x3040..=0x309F | 0x30A0..=0x30FF | 0x31F0..=0x31FF => Script::Japanese,
        // CJK Unified Ideographs + surrounding CJK blocks / Compatibility Ideographs
        // / Full-width Latin / Extension B
        // (cannot distinguish Hans/Hant from Unicode range alone)
        0x3000..=0x9FFF | 0xF900..=0xFAFF | 0xFF01..=0xFF60 | 0x2_0000..=0x2_A6DF => {
            Script::SimplifiedChinese
        }
        // PUA: U+E000–EFFF (General) and U+F100–F8FF; F000–F0FF is sym-PUA handled by sym font
        0xE000..=0xEFFF | 0xF100..=0xF8FF => Script::Pua,
        _ => Script::Latin,
    }
}

/// Split `text` into runs of contiguous, uniformly-classified [`TextPart`]s.
///
/// Adjacent characters with the same [`Script`] are merged into one
/// `TextPart`. An empty input returns an empty `Vec`.
#[must_use]
pub fn split_by_script(text: &str) -> Vec<TextPart> {
    let mut parts: Vec<TextPart> = Vec::new();
    let mut current = String::new();
    let mut current_script: Option<Script> = None;

    for ch in text.chars() {
        let script = classify_script(ch as u32);
        match current_script {
            None => {
                current_script = Some(script);
                current.push(ch);
            }
            Some(prev) if prev == script => {
                current.push(ch);
            }
            Some(prev) => {
                parts.push(TextPart {
                    text: std::mem::take(&mut current),
                    script: prev,
                });
                current_script = Some(script);
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        if let Some(script) = current_script {
            parts.push(TextPart {
                text: current,
                script,
            });
        }
    }
    parts
}

/// Returns `true` if `code_point` falls in any CJK / Hangul block this
/// crate treats as full-width / wide.
///
/// Mirrors
/// exactly (same set of ranges, same ordering).
#[must_use]
pub fn is_cjk_codepoint(code_point: u32) -> bool {
    // CJK symbols / Hiragana / Katakana / unified ideographs.
    (0x3000..=0x9FFF).contains(&code_point)
        // CJK Compatibility Ideographs.
        || (0xF900..=0xFAFF).contains(&code_point)
        // Full-width Latin / symbols.
        || (0xFF01..=0xFF60).contains(&code_point)
        // CJK Unified Ideographs Extension B.
        || (0x20000..=0x2A6DF).contains(&code_point)
        // Hangul Syllables.
        || (0xAC00..=0xD7A3).contains(&code_point)
        // Hangul Jamo.
        || (0x1100..=0x11FF).contains(&code_point)
        // Hangul Jamo Extended-A.
        || (0xA960..=0xA97F).contains(&code_point)
        // Hangul Jamo Extended-B.
        || (0xD7B0..=0xD7FF).contains(&code_point)
}

/// Returns `true` if `code_point` is in the halfwidth CJK ranges.
///
/// Covers Halfwidth Katakana (U+FF61–FF9F) and Halfwidth Hangul
/// (U+FFA0–FFDC). These glyphs occupy ~0.5 em in width, not a full
/// CJK em-square; use [`HALFWIDTH_RATIO`] instead of [`WIDE_RATIO`].
#[must_use]
pub fn is_halfwidth_cjk_codepoint(code_point: u32) -> bool {
    // Halfwidth Katakana
    (0xFF61..=0xFF9F).contains(&code_point)
        // Halfwidth Hangul (Compatibility Jamo)
        || (0xFFA0..=0xFFDC).contains(&code_point)
}

/// Returns `true` if `code_point` is in a Private Use Area (PUA) range.
///
/// Covers U+E000–F8FF (BMP PUA). Note: U+F000–F0FF is a subset used
/// by the Symbol / Wingdings font family — routed to the sym font.
/// U+F100–F8FF and U+E000–EFFF → EA font stack.
#[must_use]
pub fn is_pua_codepoint(code_point: u32) -> bool {
    (0xE000..=0xF8FF).contains(&code_point)
}

/// Returns `true` if `code_point` is in the Symbol / Wingdings PUA range
/// (U+F000–F0FF), which must be rendered with the run's sym font rather
/// than the EA or Latin font.
#[must_use]
pub fn is_sym_pua_codepoint(code_point: u32) -> bool {
    (0xF000..=0xF0FF).contains(&code_point)
}

/// Returns `true` if `code_point` requires complex shaping (Arabic,
/// Hebrew, Thai, Indic scripts, etc.).
///
/// The renderer uses this to decide whether a complex-script (CS) font
/// override is needed.
#[must_use]
pub fn is_complex_script_codepoint(code_point: u32) -> bool {
    matches!(
        code_point,
        // Arabic
        0x0600..=0x06FF
        | 0x0750..=0x077F
        | 0xFB50..=0xFDFF
        | 0xFE70..=0xFEFF
        // Hebrew
        | 0x0590..=0x05FF
        // Thai
        | 0x0E00..=0x0E7F
        // Devanagari (Hindi etc.)
        | 0x0900..=0x097F
        // Bengali
        | 0x0980..=0x09FF
        // Gujarati
        | 0x0A80..=0x0AFF
        // Tamil
        | 0x0B80..=0x0BFF
        // Telugu
        | 0x0C00..=0x0C7F
        // Kannada
        | 0x0C80..=0x0CFF
        // Malayalam
        | 0x0D00..=0x0D7F
    )
}

/// Returns the natural line-height ratio (in fontSize units) for the
/// run's primary or East-Asian font.
///
/// Resolution order matches TS: prefer `font_family`, fall back to
/// `font_family_ea`. Returns [`DEFAULT_LINE_HEIGHT_RATIO`] (`1.2`) when
/// neither has metrics.
#[must_use]
pub fn get_line_height_ratio(font_family: Option<&str>, font_family_ea: Option<&str>) -> f64 {
    match resolve_metrics(font_family, font_family_ea) {
        Some(m) => {
            (f64::from(m.ascender) + f64::from(m.descender).abs()) / f64::from(m.units_per_em)
        }
        None => DEFAULT_LINE_HEIGHT_RATIO,
    }
}

/// Returns the ascender ratio (in fontSize units) for the run's primary
/// or East-Asian font.
///
/// Used by the renderer to compute the first line's baseline offset
/// from the text-frame top edge — `<text y="...">` puts y on the
/// baseline, not the cap-height top.
#[must_use]
pub fn get_ascender_ratio(font_family: Option<&str>, font_family_ea: Option<&str>) -> f64 {
    match resolve_metrics(font_family, font_family_ea) {
        Some(m) => f64::from(m.ascender) / f64::from(m.units_per_em),
        None => DEFAULT_ASCENDER_RATIO,
    }
}

/// Estimates the rendered pixel width of `text` at `font_size_pt`.
///
/// When the run's `font_family` (or `font_family_ea` for CJK
/// codepoints) maps onto an entry in [`crate::fallback_metrics`], the
/// width is computed metric-accurately. Otherwise the per-character
/// heuristic kicks in. `bold` widens Latin glyphs by [`BOLD_FACTOR`];
/// CJK glyphs pass through.
#[must_use]
pub fn measure_text_width(
    text: &str,
    font_size_pt: f64,
    bold: bool,
    font_family: Option<&str>,
    font_family_ea: Option<&str>,
) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    let base_size_px = font_size_pt * PX_PER_PT;
    let latin_metrics = font_family.and_then(get_font_metrics);
    let ea_metrics = font_family_ea.and_then(get_font_metrics);

    // Static metric catalogue (PANOSE+OS/2) provides an `avg_advance_ratio`
    // for ~80 well-known typefaces. When the run's font isn't in the
    // glyph-accurate `fallback_metrics` table, we still know the font's
    // average glyph width vs em-square — use it as a more accurate
    // "normal" Latin ratio than the generic 0.6 heuristic, which
    // overestimates condensed sans-serifs (e.g. Pretendard 0.50,
    // Inter 0.50) by ~17 % and triggers spurious wraps.
    let latin_avg_ratio = font_family
        .and_then(|f| metric_for_family(f).map(|v| f64::from(v.avg_advance_ratio)))
        .filter(|r| *r > 0.0);
    let ea_avg_ratio = font_family_ea
        .and_then(|f| metric_for_family(f).map(|v| f64::from(v.avg_advance_ratio)))
        .filter(|r| *r > 0.0);

    let mut total = 0.0_f64;
    for ch in text.chars() {
        let cp = ch as u32;
        let is_ea = is_cjk_codepoint(cp);
        let metrics = if is_ea && ea_metrics.is_some() {
            ea_metrics
        } else {
            latin_metrics
        };
        // CJK fallback metrics (Noto Sans KR / SC / TC etc.)
        // ship `widths: HashMap::new()` and a `default_width`
        // sized for full-width glyphs (~0.56 of em). Applying
        // that to a Latin codepoint over-measures by ~30 % and
        // triggers spurious wraps on Latin-heavy lines. When
        // we hit that case, prefer the static metric
        // catalogue's `avg_advance_ratio` for the run's
        // declared font (e.g. Pretendard 0.50) which is the
        // accurate value for the actual rendered face — the
        // CJK metric still wins for CJK codepoints.
        let mut width = if let Some(m) = metrics {
            if !is_ea && m.widths_len() == 0 {
                // No per-glyph widths available on this metric
                // (CJK fallback metrics ship empty `widths` tables
                // because their ASCII coverage is uniformly the
                // CJK em-square width — wrong for narrow Latin
                // glyphs like space / comma / digit / lowercase
                // l). Defer to the heuristic, which uses the
                // catalogue's `avg_advance_ratio` when available
                // and the narrow / normal classification fallback
                // otherwise. Either path is closer to the actual
                // rendered width than the metric's default_width
                // (typically ~0.56 em for CJK fonts).
                measure_char_heuristic_with_avg(cp, base_size_px, latin_avg_ratio)
            } else {
                measure_char_metrics(ch, cp, base_size_px, m)
            }
        } else {
            let avg_ratio = if is_ea { ea_avg_ratio } else { latin_avg_ratio };
            measure_char_heuristic_with_avg(cp, base_size_px, avg_ratio)
        };
        if bold && !is_ea {
            width *= BOLD_FACTOR;
        }
        total += width;
    }
    total
}

fn resolve_metrics(
    font_family: Option<&str>,
    font_family_ea: Option<&str>,
) -> Option<&'static FontMetrics> {
    font_family
        .and_then(get_font_metrics)
        .or_else(|| font_family_ea.and_then(get_font_metrics))
}

// Same as a plain `base_size_px * ratio_for(code_point)` heuristic but
// substitutes the catalogue's `avg_advance_ratio` for `NORMAL_RATIO`
// when one is available. Narrow (space/comma/etc.) and wide (CJK)
// classes keep their existing ratios — the average advance is taken
// across a-z + 0-9 + space, so applying it to those classes would
// distort their measurement.
fn measure_char_heuristic_with_avg(code_point: u32, base_size_px: f64, avg: Option<f64>) -> f64 {
    let ratio = match avg {
        Some(a) if !is_cjk_codepoint(code_point) && !is_narrow_latin(code_point) => a,
        _ => ratio_for(code_point),
    };
    base_size_px * ratio
}

fn measure_char_metrics(ch: char, code_point: u32, base_size_px: f64, m: &FontMetrics) -> f64 {
    if let Some(width) = m.width_of(ch) {
        return f64::from(width) / f64::from(m.units_per_em) * base_size_px;
    }
    if is_cjk_codepoint(code_point) {
        return f64::from(m.cjk_width) / f64::from(m.units_per_em) * base_size_px;
    }
    f64::from(m.default_width) / f64::from(m.units_per_em) * base_size_px
}

fn ratio_for(code_point: u32) -> f64 {
    // Halfwidth CJK is ~0.5 em; check before the full-width CJK check.
    if is_halfwidth_cjk_codepoint(code_point) {
        return HALFWIDTH_RATIO;
    }
    if is_cjk_codepoint(code_point) {
        return WIDE_RATIO;
    }
    if is_narrow_latin(code_point) {
        return NARROW_RATIO;
    }
    NORMAL_RATIO
}

fn is_narrow_latin(cp: u32) -> bool {
    matches!(
        cp,
        0x20 // space
        | 0x21 // !
        | 0x2C // ,
        | 0x2D // - (hyphen-minus): proportional sans-serif renders this
                // narrower than NORMAL_RATIO; PowerPoint measures it
                // similarly to a comma, and treating it as `normal`
                // (0.6 em) over-measures phone-number / date strings
                // by ~5 % and triggers spurious wraps in narrow cells.
        | 0x2E // .
        | 0x3A // :
        | 0x3B // ;
        | 0x69 // i
        | 0x6A // j
        | 0x6C // l
        | 0x31 // 1
        | 0x7C // |
        | 0x27 // '
        | 0x28 // (
        | 0x29 // )
        | 0x5B // [
        | 0x5D // ]
        | 0x7B // {
        | 0x7D // }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(actual: f64, expected: f64, places: i32) {
        let tolerance = 10f64.powi(-places) * 5.0;
        assert!(
            (actual - expected).abs() < tolerance,
            "expected {expected}, got {actual} (tolerance={tolerance})"
        );
    }

    // -- measure_text_width: heuristic ----------------------------------------

    #[test]
    fn empty_string_zero_width() {
        assert_eq!(measure_text_width("", 18.0, false, None, None), 0.0);
    }

    #[test]
    fn ascii_heuristic() {
        // 'H','e','o' = normal(0.6), 'l','l' = narrow(0.3)
        // (3*0.6 + 2*0.3) * 18 * 96/72 = 2.4 * 24 = 57.6
        let w = measure_text_width("Hello", 18.0, false, None, None);
        approx_eq(w, 57.6, 1);
    }

    #[test]
    fn cjk_heuristic() {
        let w = measure_text_width("漢字", 18.0, false, None, None);
        approx_eq(w, 2.0 * WIDE_RATIO * 18.0 * PX_PER_PT, 1);
    }

    #[test]
    fn mixed_heuristic() {
        let w = measure_text_width("A漢", 18.0, false, None, None);
        // 1 normal Latin (0.6) + 1 CJK (WIDE_RATIO 0.95).
        approx_eq(w, (NORMAL_RATIO + WIDE_RATIO) * 18.0 * PX_PER_PT, 1);
    }

    #[test]
    fn bold_widens_latin_only() {
        let normal = measure_text_width("Test", 18.0, false, None, None);
        let bold = measure_text_width("Test", 18.0, true, None, None);
        approx_eq(bold, normal * 1.05, 1);
    }

    #[test]
    fn bold_does_not_widen_cjk() {
        let normal = measure_text_width("漢字", 18.0, false, None, None);
        let bold = measure_text_width("漢字", 18.0, true, None, None);
        approx_eq(bold, normal, 5);
    }

    #[test]
    fn bold_mixed_only_latin_widens() {
        let latin = measure_text_width("A", 18.0, false, None, None);
        let cjk = measure_text_width("漢", 18.0, false, None, None);
        let mixed_bold = measure_text_width("A漢", 18.0, true, None, None);
        approx_eq(mixed_bold, latin * 1.05 + cjk, 1);
    }

    #[test]
    fn space_is_narrow() {
        let w = measure_text_width(" ", 18.0, false, None, None);
        approx_eq(w, 0.3 * 18.0 * PX_PER_PT, 1);
    }

    #[test]
    fn hiragana_is_wide() {
        let w = measure_text_width("あ", 18.0, false, None, None);
        approx_eq(w, WIDE_RATIO * 18.0 * PX_PER_PT, 1);
    }

    #[test]
    fn katakana_is_wide() {
        let w = measure_text_width("ア", 18.0, false, None, None);
        approx_eq(w, WIDE_RATIO * 18.0 * PX_PER_PT, 1);
    }

    #[test]
    fn proportional_to_font_size() {
        let w12 = measure_text_width("A", 12.0, false, None, None);
        let w24 = measure_text_width("A", 24.0, false, None, None);
        approx_eq(w24, w12 * 2.0, 1);
    }

    // -- measure_text_width: with metrics -------------------------------------

    #[test]
    fn calibri_metrics_for_a() {
        // Carlito: A=1185, unitsPerEm=2048
        let w = measure_text_width("A", 18.0, false, Some("Calibri"), None);
        let expected = 1185.0 / 2048.0 * 18.0 * PX_PER_PT;
        approx_eq(w, expected, 1);
    }

    #[test]
    fn metrics_differs_from_heuristic() {
        // Carlito A=1185/2048 vs heuristic normal ratio 0.6 — at fontSize 18
        // the difference is ~0.5px (TS test asserts `not.toBeCloseTo(0)` ⇒
        // tolerance 0.5, so a strictly greater check would tip on rounding;
        // 0.4 is the conservative analogue that still proves divergence).
        let metrics = measure_text_width("A", 18.0, false, Some("Calibri"), None);
        let heuristic = measure_text_width("A", 18.0, false, None, None);
        assert!(
            (metrics - heuristic).abs() > 0.4,
            "metrics={metrics} heuristic={heuristic}"
        );
    }

    #[test]
    fn unknown_font_falls_back_to_heuristic() {
        let with_unknown = measure_text_width("A", 18.0, false, Some("UnknownFont"), None);
        let pure_heuristic = measure_text_width("A", 18.0, false, None, None);
        approx_eq(with_unknown, pure_heuristic, 5);
    }

    #[test]
    fn null_font_falls_back_to_heuristic() {
        let with_none = measure_text_width("A", 18.0, false, None, None);
        let pure_heuristic = measure_text_width("A", 18.0, false, None, None);
        approx_eq(with_none, pure_heuristic, 5);
    }

    #[test]
    fn arial_metrics_for_a() {
        // LiberationSans: A=1366, unitsPerEm=2048
        let w = measure_text_width("A", 18.0, false, Some("Arial"), None);
        let expected = 1366.0 / 2048.0 * 18.0 * PX_PER_PT;
        approx_eq(w, expected, 1);
    }

    #[test]
    fn cjk_uses_cjk_width() {
        // Carlito: cjkWidth=2048 / unitsPerEm=2048 → 1.0 * fontSizePx
        let w = measure_text_width("漢", 18.0, false, Some("Calibri"), None);
        let expected = 18.0 * PX_PER_PT;
        approx_eq(w, expected, 1);
    }

    #[test]
    fn bold_applies_factor_to_metrics() {
        let normal = measure_text_width("Test", 18.0, false, Some("Calibri"), None);
        let bold = measure_text_width("Test", 18.0, true, Some("Calibri"), None);
        approx_eq(bold, normal * 1.05, 1);
    }

    #[test]
    fn metrics_sum_for_hello() {
        // Carlito: H=1276, e=1019, l=470, l=470, o=1080 → 4315/2048
        let w = measure_text_width("Hello", 18.0, false, Some("Calibri"), None);
        let expected = f64::from(1276 + 1019 + 470 + 470 + 1080) / 2048.0 * 18.0 * PX_PER_PT;
        approx_eq(w, expected, 1);
    }

    #[test]
    fn missing_char_uses_default_width() {
        // U+0100 (Ā) is not in Carlito's 200-entry table → uses defaultWidth=991.
        let w = measure_text_width("\u{0100}", 18.0, false, Some("Calibri"), None);
        let expected = 991.0 / 2048.0 * 18.0 * PX_PER_PT;
        approx_eq(w, expected, 1);
    }

    // -- measure_text_width: ea fallback --------------------------------------

    #[test]
    fn ea_metrics_for_cjk_in_mixed_run() {
        // NotoSansJP: cjkWidth=1000 / unitsPerEm=1000 = 1.0 * fontSize
        let w = measure_text_width("漢", 18.0, false, Some("Calibri"), Some("Noto Sans JP"));
        let expected = 18.0 * PX_PER_PT;
        approx_eq(w, expected, 1);
    }

    #[test]
    fn latin_uses_latin_metrics_with_ea_set() {
        // A → Calibri (Carlito), even when EA font is also given.
        let w = measure_text_width("A", 18.0, false, Some("Calibri"), Some("Noto Sans JP"));
        let expected = 1185.0 / 2048.0 * 18.0 * PX_PER_PT;
        approx_eq(w, expected, 1);
    }

    #[test]
    fn mixed_uses_per_character_metrics() {
        let w = measure_text_width("A漢", 18.0, false, Some("Calibri"), Some("Noto Sans JP"));
        let latin = 1185.0 / 2048.0 * 18.0 * PX_PER_PT;
        let cjk = 1.0 * 18.0 * PX_PER_PT;
        approx_eq(w, latin + cjk, 1);
    }

    #[test]
    fn null_ea_uses_latin_for_cjk() {
        // Carlito cjkWidth=2048 / unitsPerEm=2048 = 1.0 * fontSize.
        let w = measure_text_width("漢", 18.0, false, Some("Calibri"), None);
        let expected = 18.0 * PX_PER_PT;
        approx_eq(w, expected, 1);
    }

    // -- get_line_height_ratio ------------------------------------------------

    #[test]
    fn line_height_calibri() {
        // Carlito: (1950 + 550) / 2048
        let r = get_line_height_ratio(Some("Calibri"), None);
        approx_eq(r, (1950.0 + 550.0) / 2048.0, 5);
    }

    #[test]
    fn line_height_arial() {
        // LiberationSans: (1854 + 434) / 2048
        let r = get_line_height_ratio(Some("Arial"), None);
        approx_eq(r, (1854.0 + 434.0) / 2048.0, 5);
    }

    #[test]
    fn line_height_uses_ea_when_primary_missing() {
        // NotoSansJP: (1160 + 288) / 1000
        let r = get_line_height_ratio(None, Some("Meiryo"));
        approx_eq(r, (1160.0 + 288.0) / 1000.0, 5);
    }

    #[test]
    fn line_height_prefers_primary_over_ea() {
        let r = get_line_height_ratio(Some("Calibri"), Some("Meiryo"));
        approx_eq(r, (1950.0 + 550.0) / 2048.0, 5);
    }

    #[test]
    fn line_height_unknown_returns_default() {
        assert_eq!(
            get_line_height_ratio(Some("UnknownFont"), None),
            DEFAULT_LINE_HEIGHT_RATIO
        );
    }

    #[test]
    fn line_height_both_none_returns_default() {
        assert_eq!(get_line_height_ratio(None, None), DEFAULT_LINE_HEIGHT_RATIO);
    }

    // -- get_ascender_ratio ---------------------------------------------------

    #[test]
    fn ascender_calibri() {
        let r = get_ascender_ratio(Some("Calibri"), None);
        approx_eq(r, 1950.0 / 2048.0, 5);
    }

    #[test]
    fn ascender_arial() {
        let r = get_ascender_ratio(Some("Arial"), None);
        approx_eq(r, 1854.0 / 2048.0, 5);
    }

    #[test]
    fn ascender_uses_ea_when_primary_missing() {
        let r = get_ascender_ratio(None, Some("Meiryo"));
        approx_eq(r, 1160.0 / 1000.0, 5);
    }

    #[test]
    fn ascender_prefers_primary_over_ea() {
        let r = get_ascender_ratio(Some("Calibri"), Some("Meiryo"));
        approx_eq(r, 1950.0 / 2048.0, 5);
    }

    #[test]
    fn ascender_unknown_returns_default() {
        assert_eq!(
            get_ascender_ratio(Some("UnknownFont"), None),
            DEFAULT_ASCENDER_RATIO
        );
    }

    #[test]
    fn ascender_both_none_returns_default() {
        assert_eq!(get_ascender_ratio(None, None), DEFAULT_ASCENDER_RATIO);
    }

    #[test]
    fn ascender_smaller_than_line_height() {
        let asc = get_ascender_ratio(Some("Calibri"), None);
        let lh = get_line_height_ratio(Some("Calibri"), None);
        assert!(asc < lh, "ascender {asc} < line_height {lh}");
    }

    // -- Script enum / classify_script ----------------------------------------

    #[test]
    fn script_enum_has_six_variants() {
        // Compile-time proof that all six variants exist.
        let variants = [
            Script::Latin,
            Script::Korean,
            Script::Japanese,
            Script::SimplifiedChinese,
            Script::TraditionalChinese,
            Script::Pua,
        ];
        assert_eq!(variants.len(), 6);
    }

    #[test]
    fn classify_latin_ascii() {
        assert_eq!(classify_script('A' as u32), Script::Latin);
        assert_eq!(classify_script('z' as u32), Script::Latin);
        assert_eq!(classify_script(' ' as u32), Script::Latin);
    }

    #[test]
    fn classify_hangul_syllables_as_korean() {
        assert_eq!(classify_script('한' as u32), Script::Korean);
        assert_eq!(classify_script('글' as u32), Script::Korean);
        assert_eq!(classify_script(0xAC00), Script::Korean);
        assert_eq!(classify_script(0xD7A3), Script::Korean);
    }

    #[test]
    fn classify_hangul_jamo_as_korean() {
        assert_eq!(classify_script(0x1100), Script::Korean);
        assert_eq!(classify_script(0x11FF), Script::Korean);
    }

    #[test]
    fn classify_hiragana_as_japanese() {
        assert_eq!(classify_script('あ' as u32), Script::Japanese);
        assert_eq!(classify_script(0x3040), Script::Japanese);
        assert_eq!(classify_script(0x309F), Script::Japanese);
    }

    #[test]
    fn classify_katakana_as_japanese() {
        assert_eq!(classify_script('ア' as u32), Script::Japanese);
        assert_eq!(classify_script(0x30A0), Script::Japanese);
        assert_eq!(classify_script(0x30FF), Script::Japanese);
    }

    #[test]
    fn classify_cjk_unified_as_simplified_chinese() {
        // CJK ideographs outside Hiragana/Katakana/Hangul default to Hans.
        assert_eq!(classify_script('日' as u32), Script::SimplifiedChinese);
        assert_eq!(classify_script('中' as u32), Script::SimplifiedChinese);
    }

    #[test]
    fn classify_pua_general() {
        assert_eq!(classify_script(0xE000), Script::Pua);
        assert_eq!(classify_script(0xEFFF), Script::Pua);
        assert_eq!(classify_script(0xF100), Script::Pua);
        assert_eq!(classify_script(0xF8FF), Script::Pua);
    }

    #[test]
    fn classify_sym_pua_as_latin() {
        // U+F000–F0FF is the sym-PUA range; left to Script::Latin so the
        // renderer can route to the sym font stack separately.
        assert_eq!(classify_script(0xF000), Script::Latin);
        assert_eq!(classify_script(0xF0FF), Script::Latin);
    }

    #[test]
    fn script_is_ea() {
        assert!(!Script::Latin.is_ea());
        assert!(Script::Korean.is_ea());
        assert!(Script::Japanese.is_ea());
        assert!(Script::SimplifiedChinese.is_ea());
        assert!(Script::TraditionalChinese.is_ea());
        assert!(Script::Pua.is_ea());
    }

    // -- split_by_script -------------------------------------------------------

    #[test]
    fn split_empty_returns_empty() {
        assert!(split_by_script("").is_empty());
    }

    #[test]
    fn split_pure_latin() {
        let parts = split_by_script("hello");
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].script, Script::Latin);
        assert_eq!(parts[0].text, "hello");
    }

    #[test]
    fn split_pure_korean() {
        let parts = split_by_script("한국어");
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].script, Script::Korean);
    }

    #[test]
    fn split_pure_japanese() {
        let parts = split_by_script("あいう");
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].script, Script::Japanese);
    }

    #[test]
    fn split_mixed_latin_korean() {
        let parts = split_by_script("Hello 한국");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].script, Script::Latin);
        assert_eq!(parts[0].text, "Hello ");
        assert_eq!(parts[1].script, Script::Korean);
        assert_eq!(parts[1].text, "한국");
    }

    #[test]
    fn split_mixed_three_scripts() {
        let parts = split_by_script("Hi 한 あ");
        // "Hi " Latin, "한" Korean, " " Latin, "あ" Japanese
        assert!(parts.len() >= 3);
        assert_eq!(parts[0].script, Script::Latin);
        assert_eq!(parts[0].text, "Hi ");
        assert_eq!(parts[1].script, Script::Korean);
    }

    // -- T5: is_halfwidth_cjk_codepoint / is_pua / is_complex ------------------

    #[test]
    fn halfwidth_katakana_is_halfwidth() {
        assert!(is_halfwidth_cjk_codepoint(0xFF61));
        assert!(is_halfwidth_cjk_codepoint(0xFF9F));
        assert!(is_halfwidth_cjk_codepoint(0xFF70)); // ｰ mid range
    }

    #[test]
    fn halfwidth_hangul_is_halfwidth() {
        assert!(is_halfwidth_cjk_codepoint(0xFFA0));
        assert!(is_halfwidth_cjk_codepoint(0xFFDC));
    }

    #[test]
    fn full_width_katakana_is_not_halfwidth() {
        assert!(!is_halfwidth_cjk_codepoint('ア' as u32)); // U+30A2
    }

    #[test]
    fn latin_is_not_halfwidth() {
        assert!(!is_halfwidth_cjk_codepoint('A' as u32));
    }

    #[test]
    fn pua_e000_is_pua() {
        assert!(is_pua_codepoint(0xE000));
        assert!(is_pua_codepoint(0xEFFF));
        assert!(is_pua_codepoint(0xF000));
        assert!(is_pua_codepoint(0xF8FF));
    }

    #[test]
    fn latin_is_not_pua() {
        assert!(!is_pua_codepoint('A' as u32));
        assert!(!is_pua_codepoint(0x3000));
    }

    #[test]
    fn arabic_is_complex_script() {
        assert!(is_complex_script_codepoint(0x0600));
        assert!(is_complex_script_codepoint(0x06FF));
    }

    #[test]
    fn hebrew_is_complex_script() {
        assert!(is_complex_script_codepoint(0x0590));
        assert!(is_complex_script_codepoint(0x05FF));
    }

    #[test]
    fn thai_is_complex_script() {
        assert!(is_complex_script_codepoint(0x0E00));
        assert!(is_complex_script_codepoint(0x0E7F));
    }

    #[test]
    fn latin_is_not_complex_script() {
        assert!(!is_complex_script_codepoint('A' as u32));
        assert!(!is_complex_script_codepoint('z' as u32));
    }

    #[test]
    fn halfwidth_cjk_uses_halfwidth_ratio() {
        let hw = measure_text_width("\u{FF61}", 18.0, false, None, None);
        approx_eq(hw, HALFWIDTH_RATIO * 18.0 * PX_PER_PT, 1);
    }

    // -- is_cjk_codepoint -----------------------------------------------------

    #[test]
    fn is_cjk_for_unified_ideographs() {
        // 漢 = U+6F22
        assert!(is_cjk_codepoint(0x6F22));
        // 字 = U+5B57
        assert!(is_cjk_codepoint(0x5B57));
    }

    #[test]
    fn is_cjk_for_hiragana() {
        // あ = U+3042
        assert!(is_cjk_codepoint(0x3042));
    }

    #[test]
    fn is_cjk_for_katakana() {
        // ア = U+30A2
        assert!(is_cjk_codepoint(0x30A2));
    }

    #[test]
    fn is_cjk_for_hangul_syllables() {
        // 한 = U+D55C
        assert!(is_cjk_codepoint(0xD55C));
        // 글 = U+AE00
        assert!(is_cjk_codepoint(0xAE00));
    }

    #[test]
    fn is_cjk_for_full_width_latin() {
        // Ｐ = U+FF30
        assert!(is_cjk_codepoint(0xFF30));
    }

    #[test]
    fn is_cjk_false_for_latin() {
        assert!(!is_cjk_codepoint('A' as u32));
        assert!(!is_cjk_codepoint(' ' as u32));
        assert!(!is_cjk_codepoint('!' as u32));
    }
}
