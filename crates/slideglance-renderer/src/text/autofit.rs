//! `<a:bodyPr autoFit="...">` height/scale computation.
//!
//! Direct port of lines
//! 855–962:
//!
//! - [`compute_sp_autofit_height`] computes the new shape height needed
//!   to make the text body's natural layout fit. Returns `None` when
//!   the existing height already fits — the caller (shape renderer)
//!   then keeps the original transform unchanged.
//! - [`compute_shrink_to_fit_scale`] runs the iterative font-scale
//! reduction TS performs for `normAutofit`. Called from
//!   [`super::body::render_text_body`] when wrapping is enabled.

use slideglance_font::TextMeasurer;
use slideglance_model::{TextBody, WrapMode};
use slideglance_utils::{Emu, EMU_PER_INCH};

use crate::text::body::{estimate_total_height, resolve_text_dimensions};
use crate::text::layout::get_default_font_size;

/// `<DEFAULT_DPI>` shared with the rest of the renderer (96 px/in).
/// We avoid pulling [`slideglance_utils::DEFAULT_DPI`] (a `u32`) directly so
/// the conversion arithmetic stays as `f64` end-to-end.
const DEFAULT_DPI: f64 = 96.0;

/// Compute the new shape height (in EMU) required so the body's text
/// fits without truncation. Returns `None` when the current
/// `transform.extent_height` already accommodates the natural text
/// layout — the caller leaves the transform alone in that case.
///
/// Mirrors `computeSpAutofitHeight` in the spec.
#[must_use]
pub fn compute_sp_autofit_height(
    text_body: &TextBody,
    width_emu: Emu,
    height_emu: Emu,
    measurer: &dyn TextMeasurer,
) -> Option<Emu> {
    let bp = &text_body.body_properties;
    let has_text = text_body
        .paragraphs
        .iter()
        .any(|p| p.runs.iter().any(|r| !r.text.is_empty()));
    if !has_text {
        return None;
    }
    let original_width_px = width_emu.to_pixels();
    let original_height_px = height_emu.to_pixels();
    let dims = resolve_text_dimensions(bp, original_width_px, original_height_px);

    let full_text_width = dims.width - dims.margin_left - dims.margin_right;
    let num_col = bp.num_col.max(1);
    let text_width = if num_col > 1 {
        full_text_width / f64::from(num_col)
    } else {
        full_text_width
    };
    let default_font_size = get_default_font_size(text_body);
    let should_wrap = !matches!(bp.wrap, WrapMode::None);

    let text_height = estimate_total_height(
        text_body,
        default_font_size,
        should_wrap,
        text_width,
        bp.ln_spc_reduction,
        1.0,
        measurer,
    );
    let required_height_px = text_height + dims.margin_top + dims.margin_bottom;
    if required_height_px <= dims.height {
        return None;
    }
    // Convert px → EMU. Mirrors the spec's
    // `(px / DEFAULT_DPI) * EMU_PER_INCH` at default 96 DPI.
    let required_emu_f =
        (required_height_px / DEFAULT_DPI) * f64::from(i32::try_from(EMU_PER_INCH).unwrap_or(0));
    if !required_emu_f.is_finite() {
        return None;
    }
    let required_emu = required_emu_f.round() as i64;
    Some(Emu::new(required_emu))
}

/// Run the iterative shrink-to-fit calculation TS uses for
/// `normAutofit`. Returns the new font scale; never returns less than
/// `font_scale * 0.1`.
///
/// Mirrors `computeShrinkToFitScale` in the spec: at most five
/// passes of `scale *= availableHeight / textHeight` until the text
/// fits.
#[must_use]
pub fn compute_shrink_to_fit_scale(
    text_body: &TextBody,
    default_font_size: f64,
    font_scale: f64,
    ln_spc_reduction: f64,
    text_width: f64,
    available_height: f64,
    measurer: &dyn TextMeasurer,
) -> f64 {
    if available_height <= 0.0 {
        return font_scale;
    }
    let min_scale = font_scale * 0.1;
    let mut scale = font_scale;
    for _ in 0..5 {
        let text_height = estimate_total_height(
            text_body,
            default_font_size,
            true,
            text_width,
            ln_spc_reduction,
            scale,
            measurer,
        );
        if text_height <= available_height {
            break;
        }
        let new_scale = scale * (available_height / text_height);
        scale = new_scale.max(min_scale);
        if scale <= min_scale {
            break;
        }
    }
    scale
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_font::HeuristicTextMeasurer;
    use slideglance_model::{
        AutoFit, BodyProperties, Paragraph, ParagraphProperties, RunProperties, TextBody, TextRun,
        TextVerticalType, VerticalAnchor, WrapMode,
    };
    use slideglance_utils::Pt;

    fn body_props(auto_fit: AutoFit) -> BodyProperties {
        BodyProperties {
            anchor: VerticalAnchor::T,
            margin_left: Emu::new(0),
            margin_right: Emu::new(0),
            margin_top: Emu::new(0),
            margin_bottom: Emu::new(0),
            wrap: WrapMode::Square,
            auto_fit,
            font_scale: 1.0,
            ln_spc_reduction: 0.0,
            num_col: 1,
            vert: TextVerticalType::Horz,
            spc_first_last_para: false,
            compat_ln_spc: false,
            prst_tx_warp: None,
        }
    }

    fn text_body(text: &str, font_size_pt: f64, paragraphs_n: usize) -> TextBody {
        let run_props = RunProperties {
            font_size: Some(Pt::new(font_size_pt)),
            ..RunProperties::default()
        };
        let make_run = || TextRun {
            text: text.to_string(),
            properties: run_props.clone(),
            field_type: None,
        };
        let paragraphs = (0..paragraphs_n)
            .map(|_| Paragraph {
                runs: vec![make_run()],
                properties: ParagraphProperties::default(),
                end_para_run_properties: None,
            })
            .collect();
        TextBody {
            default_text_color: None,
            paragraphs,
            body_properties: body_props(AutoFit::SpAutofit),
        }
    }

    #[test]
    fn empty_body_returns_none() {
        let body = TextBody {
            default_text_color: None,
            paragraphs: Vec::new(),
            body_properties: body_props(AutoFit::SpAutofit),
        };
        assert!(compute_sp_autofit_height(
            &body,
            Emu::new(914_400),
            Emu::new(914_400),
            &HeuristicTextMeasurer
        )
        .is_none());
    }

    #[test]
    fn whitespace_only_body_returns_none() {
        let body = TextBody {
            default_text_color: None,
            paragraphs: vec![Paragraph {
                runs: vec![TextRun {
                    text: String::new(),
                    properties: RunProperties::default(),
                    field_type: None,
                }],
                properties: ParagraphProperties::default(),
                end_para_run_properties: None,
            }],
            body_properties: body_props(AutoFit::SpAutofit),
        };
        assert!(compute_sp_autofit_height(
            &body,
            Emu::new(914_400),
            Emu::new(914_400),
            &HeuristicTextMeasurer
        )
        .is_none());
    }

    #[test]
    fn fitting_body_returns_none() {
        // 1 inch tall is plenty for a single line of 18pt text.
        let body = text_body("hello", 18.0, 1);
        assert!(compute_sp_autofit_height(
            &body,
            Emu::new(914_400),
            Emu::new(914_400),
            &HeuristicTextMeasurer
        )
        .is_none());
    }

    #[test]
    fn overflowing_body_returns_larger_height() {
        // Many paragraphs of 36pt text in a 0.3" tall box must overflow.
        let body = text_body("Lorem ipsum dolor sit amet", 36.0, 12);
        let small_h = Emu::new(914_400 / 4); // 0.25 inch
        let result = compute_sp_autofit_height(
            &body,
            Emu::new(914_400 * 4),
            small_h,
            &HeuristicTextMeasurer,
        );
        let new_h = result.expect("expected sp-autofit growth");
        assert!(new_h.raw() > small_h.raw(), "new={new_h:?} old={small_h:?}");
    }

    #[test]
    fn shrink_to_fit_returns_input_when_height_unset() {
        let body = text_body("Hello", 18.0, 1);
        let s =
            compute_shrink_to_fit_scale(&body, 18.0, 1.0, 0.0, 500.0, 0.0, &HeuristicTextMeasurer);
        // available_height <= 0 short-circuits.
        assert!((s - 1.0).abs() < 1e-12);
    }

    #[test]
    fn shrink_to_fit_reduces_scale_when_text_overflows() {
        // 12 paragraphs of 24pt text in a 30 px tall box → must shrink.
        let body = text_body("hello world", 24.0, 12);
        let new_scale =
            compute_shrink_to_fit_scale(&body, 24.0, 1.0, 0.0, 500.0, 30.0, &HeuristicTextMeasurer);
        assert!(new_scale < 1.0, "scale={new_scale}");
        // 0.1 floor is enforced.
        assert!(new_scale >= 0.1 - f64::EPSILON);
    }

    #[test]
    fn shrink_to_fit_keeps_scale_when_text_fits() {
        let body = text_body("Hello", 12.0, 1);
        let new_scale = compute_shrink_to_fit_scale(
            &body,
            12.0,
            1.0,
            0.0,
            500.0,
            500.0,
            &HeuristicTextMeasurer,
        );
        assert!((new_scale - 1.0).abs() < 1e-12);
    }
}
