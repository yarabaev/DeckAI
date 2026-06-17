//! Alignment / spacing / line-height helpers for the text body layout.
//!
//! Direct port of the assorted local helpers in
//! . None of these touch
//! [`slideglance_font`]; they only manipulate paragraph and run metadata.

use slideglance_font::TextMeasurer;
use slideglance_model::{
    Paragraph, ParagraphAlignment, ParagraphProperties, RunProperties, SpacingValue, TextBody,
};
use slideglance_utils::Pt;

/// 96 / 72 — pt -> px conversion factor used everywhere for vertical math.
pub(crate) const PX_PER_PT: f64 = 96.0 / 72.0;

/// Default line spacing factor when `<a:lnSpc>` is absent (single line).
pub(crate) const DEFAULT_LINE_SPACING: f64 = 1.0;

/// Default font size when no run carries an explicit size (matches the
/// spec's hard-coded fallback).
pub(crate) const DEFAULT_FONT_SIZE_PT: f64 = 18.0;

/// Information returned by [`get_alignment_info`] — the X coordinate plus
/// the SVG `text-anchor` value to emit on the first tspan of each line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignmentInfo {
    /// The X coordinate of the alignment anchor.
    pub x_pos: f64,
    /// The SVG `text-anchor` value (`"start"`, `"middle"`, `"end"`).
    pub anchor: &'static str,
}

/// Resolve a paragraph's alignment to an `(x, text-anchor)` pair.
#[must_use]
pub fn get_alignment_info(
    alignment: Option<ParagraphAlignment>,
    margin_left_px: f64,
    text_width: f64,
    width: f64,
    margin_right_px: f64,
) -> AlignmentInfo {
    match alignment {
        Some(ParagraphAlignment::Ctr) => AlignmentInfo {
            x_pos: margin_left_px + text_width / 2.0,
            anchor: "middle",
        },
        Some(ParagraphAlignment::R) => AlignmentInfo {
            x_pos: width - margin_right_px,
            anchor: "end",
        },
        // Left and Justify both render as left-aligned `<tspan>`s; full
        // justification is a future-batch concern.
        _ => AlignmentInfo {
            x_pos: margin_left_px,
            anchor: "start",
        },
    }
}

/// Resolve paragraph line spacing to a multiplier factor. `<a:lnSpc>` is
/// stored as `100,000` * factor; we clamp to a 0.5 minimum and apply
/// `<a:normAutofit @lnSpcReduction>` if non-zero.
#[must_use]
pub fn get_line_spacing(para: &Paragraph, ln_spc_reduction: f64) -> f64 {
    let spacing = match para.properties.line_spacing {
        Some(value) => {
            let factor = value / 100_000.0;
            factor.max(0.5)
        }
        None => DEFAULT_LINE_SPACING,
    };
    spacing * (1.0 - ln_spc_reduction)
}

/// Resolve a `<a:spcBef>` / `<a:spcAft>` value to pixels. `Pts` is in
/// 1/100 of a point; `Pct` is a permille percentage of the run's font size.
#[must_use]
pub fn resolve_spacing_px(spacing: SpacingValue, font_size_pt: f64) -> f64 {
    match spacing {
        SpacingValue::Pts { value } => {
            (f64::from(i32::try_from(value.raw()).unwrap_or(0)) / 100.0) * PX_PER_PT
        }
        SpacingValue::Pct { value } => font_size_pt * (value / 100_000.0) * PX_PER_PT,
    }
}

/// `Option<SpacingValue>` → pixels with a 0-px default for `None`. Helper
/// for renderer call sites that work with the inheritance-aware Optional
/// shape on [`slideglance_model::ParagraphProperties::space_before`] /
/// `space_after`.
#[must_use]
pub fn resolve_spacing_px_opt(spacing: Option<SpacingValue>, font_size_pt: f64) -> f64 {
    spacing.map_or(0.0, |s| resolve_spacing_px(s, font_size_pt))
}

/// First explicit run font size in this paragraph, falling back to
/// `<a:endParaRPr>` and finally the supplied default.
#[must_use]
pub fn get_paragraph_font_size(para: &Paragraph, default_font_size: f64) -> f64 {
    for run in &para.runs {
        if !run.text.is_empty() {
            if let Some(size) = run.properties.font_size {
                return size.raw();
            }
        }
    }
    if let Some(end) = &para.end_para_run_properties {
        if let Some(size) = end.font_size {
            return size.raw();
        }
    }
    default_font_size
}

/// `dy` value for a tspan, in pixels with two-decimal formatting matching
/// the spec's `Number.prototype.toFixed(2)` output. The first line
/// of the entire `<text>` always emits `"0"` (no leading dy) so the
/// renderer can position the baseline via the parent `y` attribute.
///
/// `line_height_pt` is the line's vertical advance in points (the
/// helper that builds it should already account for line-spacing
/// semantics — see [`effective_line_height_pt`]). `paragraph_gap_px`
/// is added once before the line.
#[must_use]
pub fn compute_dy(is_first_line: bool, line_height_pt: f64, paragraph_gap_px: f64) -> String {
    if is_first_line {
        return "0".to_string();
    }
    let dy = line_height_pt * PX_PER_PT + paragraph_gap_px;
    format!("{dy:.2}")
}

/// Resolve the effective line height (in points) for one line, honoring
/// `PowerPoint`'s `<a:lnSpc>` semantics with the bodyPr `compatLnSpc`
/// flag (ECMA-376 §21.1.2.1.1 bodyPr / §21.1.2.2.5 lnSpc).
///
/// `PowerPoint`'s empirical behavior — the canonical reference for the
/// PPTX format — is that `<a:spcPct val="100000"/>` produces baseline-
/// to-baseline distance equal to **`font_size × 1.0`**, NOT the font's
/// natural typo-metric line height. This was verified against
/// `PowerPoint` 2024 rendering of a Google Slides export that uses
/// Anton (typo ratio 1.51). Apache POI / `LibreOffice` / browser
/// `line-height: normal` all use the natural-metric path and visibly
/// disagree with `PowerPoint` on tall display fonts; we follow
/// `PowerPoint` because the deck originates there.
///
/// Mapping summary:
///
/// - **`compatLnSpc=false` (default)** — `PowerPoint`'s default. Use
///   `font_size × spacing_factor`. Anton at `lnSpc=100%` → 1.0× font.
/// - **`compatLnSpc=true`** — Per ECMA-376: "the line spacing
///   determined by the font scene". Decks that explicitly opt in
///   want font-metric-driven spacing, so use natural height ×
///   spacing factor.
///
/// `has_explicit_line_spacing` is retained for callers that still want
/// to log / branch on whether the value was authored vs defaulted, but
/// it no longer drives the formula.
#[must_use]
pub fn effective_line_height_pt(
    raw_font_size_pt: f64,
    natural_line_height_pt: f64,
    line_spacing_factor: f64,
    _has_explicit_line_spacing: bool,
    compat_ln_spc: bool,
) -> f64 {
    if compat_ln_spc {
        natural_line_height_pt * line_spacing_factor
    } else {
        raw_font_size_pt * line_spacing_factor
    }
}

/// First explicit segment font size in `segments`, falling back to the
/// supplied default.
//
// Currently unused — the body renderer reads from `slideglance_font::WrappedLine`
// directly. Kept here as the canonical helper for the upcoming
// non-wrap path-mode batch which also walks `(text, properties)` pairs.
#[allow(dead_code)]
pub(crate) fn get_line_font_size(
    segments: &[(String, RunProperties)],
    default_font_size: f64,
) -> f64 {
    for (_, props) in segments {
        if let Some(size) = props.font_size {
            return size.raw();
        }
    }
    default_font_size
}

/// Maximum `fontSize * lineHeightRatio` in `segments` (in points). The
/// renderer uses this to allocate vertical space proportional to the
/// largest face on the line, falling back to a 1.2 ratio when nothing
/// resolves.
#[must_use]
pub fn compute_line_natural_height<S>(
    segments: &[S],
    default_font_size: f64,
    font_scale: f64,
    measurer: &dyn TextMeasurer,
) -> f64
where
    S: SegmentLike,
{
    let mut max_height = 0.0_f64;
    for seg in segments {
        let props = seg.properties();
        let font_size = props.font_size.map_or(default_font_size, Pt::raw) * font_scale;
        let ratio = measurer.get_line_height_ratio(
            props.font_family.as_deref(),
            props.font_family_ea.as_deref(),
        );
        max_height = max_height.max(font_size * ratio);
    }
    if max_height > 0.0 {
        max_height
    } else {
        default_font_size * font_scale * 1.2
    }
}

/// First explicit run font size in `paragraphs`, falling back to
/// [`DEFAULT_FONT_SIZE_PT`].
#[must_use]
pub fn get_default_font_size(body: &TextBody) -> f64 {
    for para in &body.paragraphs {
        for run in &para.runs {
            if let Some(size) = run.properties.font_size {
                return size.raw();
            }
        }
    }
    DEFAULT_FONT_SIZE_PT
}

/// First explicit font face's line-height ratio. Defaults to `1.2`.
#[must_use]
pub fn get_default_line_height_ratio(body: &TextBody, measurer: &dyn TextMeasurer) -> f64 {
    for para in &body.paragraphs {
        for run in &para.runs {
            if run.properties.font_family.is_some() || run.properties.font_family_ea.is_some() {
                return measurer.get_line_height_ratio(
                    run.properties.font_family.as_deref(),
                    run.properties.font_family_ea.as_deref(),
                );
            }
        }
    }
    1.2
}

/// First explicit font face's ascender ratio. Defaults to `1.0`.
#[must_use]
pub fn get_default_ascender_ratio(body: &TextBody, measurer: &dyn TextMeasurer) -> f64 {
    for para in &body.paragraphs {
        for run in &para.runs {
            if run.properties.font_family.is_some() || run.properties.font_family_ea.is_some() {
                return measurer.get_ascender_ratio(
                    run.properties.font_family.as_deref(),
                    run.properties.font_family_ea.as_deref(),
                );
            }
        }
    }
    1.0
}

/// Trait representing anything we can extract `RunProperties` from for
/// line-height computation. Kept tiny so renderer code can call
/// [`compute_line_natural_height`] over either tuples (text, properties)
/// or `WrappedLine.segments`.
pub trait SegmentLike {
    /// Returns a reference to this segment's run properties.
    fn properties(&self) -> &RunProperties;
}

impl SegmentLike for (String, RunProperties) {
    fn properties(&self) -> &RunProperties {
        &self.1
    }
}

impl SegmentLike for slideglance_font::LineSegment {
    fn properties(&self) -> &RunProperties {
        &self.properties
    }
}

// Allow passing a slice of TextRun directly (matches TS `para.runs` usage
// inside `computeLineNaturalHeight(para.runs, ...)`).
impl SegmentLike for slideglance_model::TextRun {
    fn properties(&self) -> &RunProperties {
        &self.properties
    }
}

/// Tabs are rendered as four spaces in the spec. Centralized so
/// future changes (e.g. honoring `<a:tabLst>`) update one place.
#[allow(dead_code)] // used by upcoming path-mode batch
pub(crate) const TAB_SPACES: &str = "    ";

/// Returns true if `props` describes a paragraph with an active bullet
/// (i.e. bullet block is `Char` or `AutoNum`, not `None`).
#[must_use]
pub fn has_visible_bullet(props: &ParagraphProperties) -> bool {
    matches!(
        props.bullet,
        Some(
            slideglance_model::BulletType::Char { .. }
                | slideglance_model::BulletType::AutoNum { .. }
        )
    )
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use slideglance_font::HeuristicTextMeasurer;
    use slideglance_model::{BulletType, Paragraph, ParagraphProperties, TextRun};
    use slideglance_utils::HundredthPt;

    fn paragraph(props: ParagraphProperties, runs: Vec<TextRun>) -> Paragraph {
        Paragraph {
            runs,
            properties: props,
            end_para_run_properties: None,
        }
    }

    fn empty_run(text: &str) -> TextRun {
        TextRun {
            text: text.to_string(),
            properties: RunProperties::default(),
            field_type: None,
        }
    }

    #[test]
    fn alignment_left_default() {
        let info = get_alignment_info(None, 10.0, 100.0, 200.0, 5.0);
        assert_eq!(info.x_pos, 10.0);
        assert_eq!(info.anchor, "start");
    }

    #[test]
    fn alignment_center() {
        let info = get_alignment_info(Some(ParagraphAlignment::Ctr), 10.0, 100.0, 200.0, 5.0);
        assert_eq!(info.x_pos, 60.0);
        assert_eq!(info.anchor, "middle");
    }

    #[test]
    fn alignment_right() {
        let info = get_alignment_info(Some(ParagraphAlignment::R), 10.0, 100.0, 200.0, 5.0);
        assert_eq!(info.x_pos, 195.0);
        assert_eq!(info.anchor, "end");
    }

    #[test]
    fn line_spacing_default_is_one() {
        let para = paragraph(ParagraphProperties::default(), vec![]);
        assert!((get_line_spacing(&para, 0.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_spacing_clamps_to_half() {
        let mut props = ParagraphProperties::default();
        props.line_spacing = Some(10_000.0); // 0.1 -> clamped to 0.5
        let para = paragraph(props, vec![]);
        assert!((get_line_spacing(&para, 0.0) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn line_spacing_applies_reduction() {
        let mut props = ParagraphProperties::default();
        props.line_spacing = Some(150_000.0);
        let para = paragraph(props, vec![]);
        // 1.5 * (1 - 0.2) = 1.2 — float multiplication leaves a tiny residual,
        // so allow a 1e-12 tolerance.
        assert!((get_line_spacing(&para, 0.2) - 1.2).abs() < 1e-12);
    }

    #[test]
    fn resolve_spacing_pts() {
        let v = SpacingValue::Pts {
            value: HundredthPt::new(1200), // 12 pt
        };
        // 12 pt * (96 / 72) = 16 px
        assert!((resolve_spacing_px(v, 18.0) - 16.0).abs() < 1e-9);
    }

    #[test]
    fn resolve_spacing_pct() {
        // 50% of 18pt = 9 pt = 12 px
        let v = SpacingValue::Pct { value: 50_000.0 };
        assert!((resolve_spacing_px(v, 18.0) - 12.0).abs() < 1e-9);
    }

    #[test]
    fn paragraph_font_size_picks_first_runs_size() {
        let mut run_a = empty_run("a");
        run_a.properties.font_size = Some(Pt::new(24.0));
        let para = paragraph(ParagraphProperties::default(), vec![run_a, empty_run("b")]);
        assert_eq!(get_paragraph_font_size(&para, 18.0), 24.0);
    }

    #[test]
    fn paragraph_font_size_falls_back_to_end_props() {
        let mut end = RunProperties::default();
        end.font_size = Some(Pt::new(36.0));
        let para = Paragraph {
            runs: vec![empty_run("")],
            properties: ParagraphProperties::default(),
            end_para_run_properties: Some(end),
        };
        assert_eq!(get_paragraph_font_size(&para, 18.0), 36.0);
    }

    #[test]
    fn paragraph_font_size_default_when_nothing_set() {
        let para = paragraph(ParagraphProperties::default(), vec![empty_run("hi")]);
        assert_eq!(get_paragraph_font_size(&para, 18.0), 18.0);
    }

    #[test]
    fn dy_first_line_zero() {
        assert_eq!(compute_dy(true, 12.0, 0.0), "0");
    }

    #[test]
    fn dy_subsequent_line_uses_two_decimal_places() {
        // 12 pt * (96/72) = 16.00 px, no paragraph gap
        assert_eq!(compute_dy(false, 12.0, 0.0), "16.00");
    }

    #[test]
    fn dy_includes_paragraph_gap() {
        // 12 pt * (96/72) = 16, plus 5 px gap -> 21.00
        assert_eq!(compute_dy(false, 12.0, 5.0), "21.00");
    }

    #[test]
    fn effective_line_height_uses_font_size_by_default() {
        // PowerPoint's empirical default: `font_size × spacing_factor`.
        // 34.5pt × 1.2 = 41.4pt — the natural-height value (49.95pt) is
        // ignored because compat_ln_spc=false (the spec default).
        let h = effective_line_height_pt(34.5, 49.95, 1.2, true, false);
        assert!(
            (h - 41.4).abs() < 1e-6,
            "expected font_size × factor = 41.4, got {h}",
        );
    }

    #[test]
    fn effective_line_height_compat_uses_natural_height() {
        // `<a:bodyPr compatLnSpc="1">` opts into "font scene"-driven
        // line spacing per ECMA-376. The formula then becomes
        // `natural_line_height × factor`. 49.95pt × 1.2 = 59.94pt.
        let h = effective_line_height_pt(34.5, 49.95, 1.2, true, true);
        assert!(
            (h - 59.94).abs() < 1e-6,
            "expected natural × factor = 59.94, got {h}",
        );
    }

    #[test]
    fn effective_line_height_implicit_uses_font_size() {
        // No explicit `<a:lnSpc>` (factor=1.0): the default still uses
        // font_size for the base, matching PowerPoint's "single line
        // spacing".
        let h = effective_line_height_pt(34.5, 49.95, 1.0, false, false);
        assert!(
            (h - 34.5).abs() < 1e-6,
            "expected font_size × 1.0 = 34.5, got {h}",
        );
    }

    #[test]
    fn line_natural_height_uses_max() {
        let mut a = RunProperties::default();
        a.font_size = Some(Pt::new(12.0));
        let mut b = RunProperties::default();
        b.font_size = Some(Pt::new(24.0));
        let segs = vec![("x".to_string(), a), ("y".to_string(), b)];
        let measurer = HeuristicTextMeasurer;
        let h = compute_line_natural_height(&segs, 18.0, 1.0, &measurer);
        // 24 * 1.2 = 28.8
        assert!((h - 28.8).abs() < 1e-9);
    }

    #[test]
    fn line_natural_height_falls_back_when_no_metrics() {
        let segs: Vec<(String, RunProperties)> = vec![];
        let measurer = HeuristicTextMeasurer;
        let h = compute_line_natural_height(&segs, 18.0, 1.0, &measurer);
        // 18 * 1.0 * 1.2 = 21.6
        assert!((h - 21.6).abs() < 1e-9);
    }

    #[test]
    fn line_font_size_picks_first_specified() {
        let mut a = RunProperties::default();
        a.font_size = Some(Pt::new(20.0));
        let segs = vec![
            ("x".to_string(), RunProperties::default()),
            ("y".to_string(), a),
        ];
        assert_eq!(get_line_font_size(&segs, 18.0), 20.0);
    }

    #[test]
    fn default_font_size_falls_back_to_18() {
        let body = TextBody {
            default_text_color: None,
            paragraphs: vec![paragraph(
                ParagraphProperties::default(),
                vec![empty_run("hi")],
            )],
            body_properties: slideglance_model::BodyProperties {
                anchor: slideglance_model::VerticalAnchor::T,
                margin_left: slideglance_utils::Emu::new(0),
                margin_right: slideglance_utils::Emu::new(0),
                margin_top: slideglance_utils::Emu::new(0),
                margin_bottom: slideglance_utils::Emu::new(0),
                wrap: slideglance_model::WrapMode::Square,
                auto_fit: slideglance_model::AutoFit::NoAutofit,
                font_scale: 1.0,
                ln_spc_reduction: 0.0,
                num_col: 1,
                vert: slideglance_model::TextVerticalType::Horz,
                spc_first_last_para: false,
                compat_ln_spc: false,
                prst_tx_warp: None,
            },
        };
        assert_eq!(get_default_font_size(&body), 18.0);
    }

    #[test]
    fn has_visible_bullet_for_char_and_autonum() {
        let mut props = ParagraphProperties::default();
        assert!(!has_visible_bullet(&props));
        props.bullet = Some(BulletType::None);
        assert!(!has_visible_bullet(&props));
        props.bullet = Some(BulletType::Char {
            char: "•".to_string(),
        });
        assert!(has_visible_bullet(&props));
        props.bullet = Some(BulletType::AutoNum {
            scheme: slideglance_model::AutoNumScheme::ArabicPeriod,
            start_at: 1,
        });
        assert!(has_visible_bullet(&props));
    }
}
