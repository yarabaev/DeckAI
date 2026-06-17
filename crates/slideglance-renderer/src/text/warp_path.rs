//! Path-mode `WordArt` (`<a:bodyPr><a:prstTxWarp/>`) text rendering.
//!
//! Direct port of `renderTextBodyAsWarpPath`, `WARP_CURVES`,
//! `curveArcLength`, and `tForArcLength` from
//! (lines 1356-1711).
//!
//! Path-mode warp differs from text-mode warp ([`super::warp`]) by
//! emitting per-glyph SVG `<path>` outlines placed along the warp
//! curve instead of relying on `<textPath>`. This is required for
//! resvg-based PNG rasterization, which lacks `<textPath>` support.
//!
//! Each glyph is centered at its arc-length midpoint and rotated by
//! the curve's tangent so it reads naturally along the path. Per-glyph
//! `scaleY` and `opacity` are honored for the inflate/deflate/fade
//! preset families.

use std::fmt::Write as _;

use slideglance_font::{
    text_to_svg_path_with_precision, FontResolver, ScriptFontContext, TextMeasurer,
};
use slideglance_model::{RunProperties, TextBody, Transform};

use crate::text::layout::PX_PER_PT;
use crate::text::path_mode::build_path_fill_attrs;

/// Decimal precision for translate / rotate / scale numeric attributes.
/// Matches the spec's `.toFixed(2)` calls.
const DECIMAL_PLACES: usize = 2;

/// Glyph-path precision passed to [`text_to_svg_path_with_precision`].
/// Matches TS `path.toPathData(2)`.
const PATH_DATA_PRECISION: u8 = 2;

/// Inset the curves slightly so glyph ascenders don't clip the bounding
/// box of the text body. Matches TS `WARP_BOX_PAD = 0.85`.
const WARP_BOX_PAD: f64 = 0.85;

/// Sample count used by [`curve_arc_length`]. Matches TS default.
const ARC_LENGTH_SAMPLES: usize = 64;

/// Sample count used by [`t_for_arc_length`]. Matches TS default.
const T_FOR_ARC_LENGTH_SAMPLES: usize = 256;

/// One sampled point along a warp curve: position, tangent angle, and
/// optional per-glyph distortion (`scale_y` for inflate/deflate, opacity
/// for fade variants). `scale_x` is reserved by the TS contract — the
/// reference reads `pt.scaleX ?? 1` even though no preset assigns it.
#[derive(Clone, Copy)]
struct WarpPoint {
    x: f64,
    y: f64,
    /// Tangent angle in radians.
    angle: f64,
    scale_x: f64,
    scale_y: f64,
    opacity: f64,
}

impl WarpPoint {
    fn at(x: f64, y: f64, angle: f64) -> Self {
        Self {
            x,
            y,
            angle,
            scale_x: 1.0,
            scale_y: 1.0,
            opacity: 1.0,
        }
    }

    fn with_scale_y(mut self, sy: f64) -> Self {
        self.scale_y = sy;
        self
    }

    fn with_opacity(mut self, o: f64) -> Self {
        self.opacity = o;
        self
    }
}

/// Sample the warp curve identified by `preset` at parameter `t ∈ [0, 1]`
/// within a `w` × `h` text-frame box. Returns `None` for unknown presets.
///
/// Direct port of the TS `WARP_CURVES` lookup table — every supported
/// preset is reproduced with the same equations and constants. Behavior
/// must match TS to byte-equivalent precision. Single-letter `t`/`w`/`h`
/// names mirror the math notation in the reference.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn warp_curve_at(preset: &str, t: f64, w: f64, h: f64) -> Option<WarpPoint> {
    use std::f64::consts::PI;
    let p = match preset {
        "textArchUp" | "textArchUpPour" => {
            // Half ellipse, opening downward; theta runs from PI (left) to 0 (right).
            let rx = (w / 2.0) * WARP_BOX_PAD;
            let ry = h * WARP_BOX_PAD;
            let cx = w / 2.0;
            let base_y = h - (h - ry) / 2.0;
            let theta = PI - t * PI;
            WarpPoint::at(
                cx + rx * theta.cos(),
                base_y - ry * theta.sin(),
                (ry * theta.cos()).atan2(rx * theta.sin()),
            )
        }
        "textArchDown" | "textArchDownPour" => {
            // Half ellipse opening upward; theta runs from PI (left) to 0 (right).
            let rx = (w / 2.0) * WARP_BOX_PAD;
            let ry = h * WARP_BOX_PAD;
            let cx = w / 2.0;
            let top_y = (h - ry) / 2.0;
            let theta = PI - t * PI;
            WarpPoint::at(
                cx + rx * theta.cos(),
                top_y + ry * theta.sin(),
                (-ry * theta.cos()).atan2(rx * theta.sin()),
            )
        }
        "textCircle" | "textCirclePour" | "textButton" | "textButtonPour" => {
            // Full circle, starting at top center, going clockwise.
            let r = (w.min(h) / 2.0) * WARP_BOX_PAD;
            let cx = w / 2.0;
            let cy = h / 2.0;
            let theta = -PI / 2.0 + t * PI * 2.0;
            WarpPoint::at(cx + r * theta.cos(), cy + r * theta.sin(), theta + PI / 2.0)
        }
        "textCurveUp" => {
            // Quadratic Bezier from (0, h*0.85) to (w, h*0.85) with control (w/2, -h*0.2).
            let p0y = h * 0.85;
            let p1y = -h * 0.2;
            let x = t * w;
            let omt = 1.0 - t;
            let y = omt * omt * p0y + 2.0 * omt * t * p1y + t * t * p0y;
            let dy = 2.0 * omt * (p1y - p0y) + 2.0 * t * (p0y - p1y);
            let dx = w;
            WarpPoint::at(x, y, dy.atan2(dx))
        }
        "textCurveDown" => {
            let p0y = h * 0.15;
            let p1y = h * 1.2;
            let x = t * w;
            let omt = 1.0 - t;
            let y = omt * omt * p0y + 2.0 * omt * t * p1y + t * t * p0y;
            let dy = 2.0 * omt * (p1y - p0y) + 2.0 * t * (p0y - p1y);
            let dx = w;
            WarpPoint::at(x, y, dy.atan2(dx))
        }
        "textWave1" | "textWave2" | "textWave4" | "textDoubleWave1" => {
            let mid_y = h / 2.0;
            let amp = h / 4.0;
            let x = t * w;
            let y = mid_y - amp * (t * PI * 2.0).sin();
            let dy = -amp * (t * PI * 2.0).cos() * PI * 2.0;
            let dx = w;
            WarpPoint::at(x, y, dy.atan2(dx))
        }
        "textTriangle" => {
            // Two-segment polyline. Pad the apex inside the box.
            let base_y = h * (1.0 - (1.0 - WARP_BOX_PAD) / 2.0);
            let peak_y = h * ((1.0 - WARP_BOX_PAD) / 2.0);
            if t < 0.5 {
                let u = t * 2.0;
                WarpPoint::at(
                    u * (w / 2.0),
                    base_y + (peak_y - base_y) * u,
                    (peak_y - base_y).atan2(w / 2.0),
                )
            } else {
                let u = (t - 0.5) * 2.0;
                WarpPoint::at(
                    w / 2.0 + u * (w / 2.0),
                    peak_y + (base_y - peak_y) * u,
                    (base_y - peak_y).atan2(w / 2.0),
                )
            }
        }
        "textTriangleInverted" => {
            let base_y = h * ((1.0 - WARP_BOX_PAD) / 2.0);
            let peak_y = h * (1.0 - (1.0 - WARP_BOX_PAD) / 2.0);
            if t < 0.5 {
                let u = t * 2.0;
                WarpPoint::at(
                    u * (w / 2.0),
                    base_y + (peak_y - base_y) * u,
                    (peak_y - base_y).atan2(w / 2.0),
                )
            } else {
                let u = (t - 0.5) * 2.0;
                WarpPoint::at(
                    w / 2.0 + u * (w / 2.0),
                    peak_y + (base_y - peak_y) * u,
                    (base_y - peak_y).atan2(w / 2.0),
                )
            }
        }
        "textSlantUp" => {
            let start_y = h * 0.85;
            let end_y = h * 0.15;
            WarpPoint::at(
                t * w,
                start_y + (end_y - start_y) * t,
                (end_y - start_y).atan2(w),
            )
        }
        "textSlantDown" => {
            let start_y = h * 0.15;
            let end_y = h * 0.85;
            WarpPoint::at(
                t * w,
                start_y + (end_y - start_y) * t,
                (end_y - start_y).atan2(w),
            )
        }
        // Inflate/Deflate — flat baseline with non-uniform per-glyph Y scale.
        "textInflate" => {
            let scale_y = 1.0 + 0.35 * (t * PI).sin();
            WarpPoint::at(t * w, h * 0.55, 0.0).with_scale_y(scale_y)
        }
        "textInflateBottom" => {
            let scale_y = 1.0 + 0.35 * (t * PI).sin();
            WarpPoint::at(t * w, h * 0.75, 0.0).with_scale_y(scale_y)
        }
        "textInflateTop" => {
            let scale_y = 1.0 + 0.35 * (t * PI).sin();
            WarpPoint::at(t * w, h * 0.35, 0.0).with_scale_y(scale_y)
        }
        "textDeflate" => {
            let scale_y = 1.0 - 0.3 * (t * PI).sin();
            WarpPoint::at(t * w, h * 0.55, 0.0).with_scale_y(scale_y)
        }
        "textDeflateBottom" => {
            let scale_y = 1.0 - 0.3 * (t * PI).sin();
            WarpPoint::at(t * w, h * 0.75, 0.0).with_scale_y(scale_y)
        }
        "textDeflateTop" => {
            let scale_y = 1.0 - 0.3 * (t * PI).sin();
            WarpPoint::at(t * w, h * 0.35, 0.0).with_scale_y(scale_y)
        }
        // Fade — opacity gradient along the direction axis.
        "textFadeRight" | "textFadeDown" => {
            let opacity = (1.0_f64 - 0.9 * t).max(0.1);
            WarpPoint::at(t * w, h * 0.55, 0.0).with_opacity(opacity)
        }
        "textFadeLeft" | "textFadeUp" => {
            let opacity = (0.1_f64 + 0.9 * t).max(0.1);
            WarpPoint::at(t * w, h * 0.55, 0.0).with_opacity(opacity)
        }
        _ => return None,
    };
    Some(p)
}

/// Estimate the total arc length of a warp curve by sampling. Used to
/// map cumulative glyph advance widths to a `t` parameter on the curve.
///
/// Direct port of TS `curveArcLength`.
fn curve_arc_length(preset: &str, w: f64, h: f64) -> f64 {
    let Some(mut prev) = warp_curve_at(preset, 0.0, w, h) else {
        return 0.0;
    };
    let mut length = 0.0;
    for i in 1..=ARC_LENGTH_SAMPLES {
        // Unwrap is safe: warp_curve_at returns Some when preset is supported,
        // and we already checked that with the t=0 sample above.
        let cur = warp_curve_at(preset, i as f64 / ARC_LENGTH_SAMPLES as f64, w, h)
            .expect("preset already validated by t=0 sample");
        length += (cur.x - prev.x).hypot(cur.y - prev.y);
        prev = cur;
    }
    length
}

/// Solve `arcLengthFromStart(t) = target_len` for `t` via iterative
/// scan on a sampled length table. Direct port of TS `tForArcLength`.
fn t_for_arc_length(preset: &str, w: f64, h: f64, target_len: f64) -> f64 {
    let Some(mut prev) = warp_curve_at(preset, 0.0, w, h) else {
        return 1.0;
    };
    let mut cumulative = 0.0;
    for i in 1..=T_FOR_ARC_LENGTH_SAMPLES {
        let t = i as f64 / T_FOR_ARC_LENGTH_SAMPLES as f64;
        let cur = warp_curve_at(preset, t, w, h).expect("preset already validated by t=0 sample");
        let seg = (cur.x - prev.x).hypot(cur.y - prev.y);
        if cumulative + seg >= target_len {
            let ratio = if seg == 0.0 {
                0.0
            } else {
                (target_len - cumulative) / seg
            };
            return ((i - 1) as f64 + ratio) / T_FOR_ARC_LENGTH_SAMPLES as f64;
        }
        cumulative += seg;
        prev = cur;
    }
    1.0
}

/// Path-mode `WordArt`: extract each glyph as an SVG path and place it
/// along the warp curve. Each glyph is centered at its arc-length
/// midpoint and rotated by the curve's tangent.
///
/// Returns:
/// - `Some(svg)` — text was rendered along the warp curve.
/// - `None` — preset is unsupported (caller should fall through to
/// regular path-mode rendering), or no font face could be resolved
/// (caller should fall back to text-mode warp).
///
/// Empty bodies and bodies whose runs contain only empty strings return
/// `Some("")` — mirrors the TS contract (renderable, but no visible
/// output) versus `None` (caller fallback).
#[allow(clippy::similar_names, clippy::too_many_lines)]
#[must_use]
pub fn render_text_body_as_warp_path(
    text_body: &TextBody,
    transform: &Transform,
    script_fonts: &ScriptFontContext,
    measurer: &dyn TextMeasurer,
    font_resolver: &dyn FontResolver,
) -> Option<String> {
    let preset = text_body.body_properties.prst_tx_warp.as_deref()?;
    // Probe the preset table — unknown presets bail before any expensive work.
    warp_curve_at(preset, 0.0, 1.0, 1.0)?;

    let w = transform.extent_width.to_pixels();
    let h = transform.extent_height.to_pixels();

    // Flatten visible runs to plain text + capture first run's style.
    // WordArt is almost always single-style; the spec takes the
    // first run's properties as authoritative.
    let mut text = String::new();
    let mut first_props: Option<&RunProperties> = None;
    for p in &text_body.paragraphs {
        for r in &p.runs {
            if r.text.is_empty() {
                continue;
            }
            if first_props.is_none() {
                first_props = Some(&r.properties);
            }
            text.push_str(&r.text);
        }
    }
    if text.is_empty() {
        return Some(String::new());
    }
    let props = first_props?;

    let font_size_pt = props.font_size.map_or(24.0, slideglance_utils::Pt::raw);
    let font_size_px = font_size_pt * PX_PER_PT;

    // Resolve the glyph face. TS `renderTextBodyAsWarpPath` uses the
    // `resolveFont(latin, ea, jpanFallback)` chain — no italic/bold
    // suffix lookup, no per-segment script split. Mirror that here.
    let face = props
        .font_family
        .as_deref()
        .and_then(|n| font_resolver.resolve(n))
        .or_else(|| {
            props
                .font_family_ea
                .as_deref()
                .and_then(|n| font_resolver.resolve(n))
        })
        .or_else(|| {
            script_fonts
                .jpan_fallback()
                .and_then(|n| font_resolver.resolve(n))
        })?;

    // Per-character advance widths drive arc-length distribution.
    let chars: Vec<char> = text.chars().collect();
    let mut char_widths = Vec::with_capacity(chars.len());
    let mut total_natural_width = 0.0;
    let style = slideglance_font::FontStyle {
        bold: props.bold,
        italic: props.italic,
    };
    for ch in &chars {
        let buf = ch.to_string();
        let cw = measurer.measure_text_width(
            &buf,
            font_size_pt,
            style,
            props.font_family.as_deref(),
            props.font_family_ea.as_deref(),
        );
        char_widths.push(cw);
        total_natural_width += cw;
    }
    if total_natural_width == 0.0 {
        return Some(String::new());
    }

    // Scale glyphs to fit along the curve. Capped at 1.0 so short
    // labels don't balloon. Matches TS's `Math.min(1, ratio)`.
    let arc_len = curve_arc_length(preset, w, h);
    let scale = (arc_len / total_natural_width).min(1.0);
    let scaled_font_pt = font_size_pt * scale;
    let scaled_font_px = font_size_px * scale;

    // Center the text along the curve.
    let total_scaled_width = total_natural_width * scale;
    let start_offset = (arc_len - total_scaled_width) / 2.0;

    let fill_attrs = build_path_fill_attrs(props);
    let mut parts = String::new();
    let mut cursor = start_offset;
    for (i, ch) in chars.iter().enumerate() {
        let advance = char_widths[i] * scale;
        // Place the glyph at its arc-length midpoint.
        let t = t_for_arc_length(preset, w, h, cursor + advance / 2.0);
        let pt = warp_curve_at(preset, t, w, h).expect("preset already validated above");

        let buf = ch.to_string();
        let path_data = text_to_svg_path_with_precision(
            &face,
            &buf,
            0.0,
            0.0,
            scaled_font_pt,
            PATH_DATA_PRECISION,
        );
        if path_data.is_empty() {
            cursor += advance;
            continue;
        }

        let angle_deg = pt.angle.to_degrees();
        let sx = pt.scale_x;
        let sy = pt.scale_y;
        // Apply scale around the glyph's vertical mid-line (~0.35×fontSize
        // above the baseline) so inflate/deflate expand symmetrically.
        let glyph_mid_y = scaled_font_px * 0.35;
        let mut scale_attr = String::new();
        if (sx - 1.0).abs() > f64::EPSILON || (sy - 1.0).abs() > f64::EPSILON {
            let _ = write!(
                scale_attr,
                " translate(0 {neg_mid:.dec$}) scale({sx} {sy}) translate(0 {pos_mid:.dec$})",
                neg_mid = -glyph_mid_y,
                pos_mid = glyph_mid_y,
                dec = DECIMAL_PLACES,
            );
        }
        let opacity_attr = if pt.opacity < 1.0 {
            format!(" fill-opacity=\"{:.3}\"", pt.opacity)
        } else {
            String::new()
        };
        // Translate so the glyph's origin sits at the curve point, rotate
        // around that origin by the tangent angle, optionally scale, then
        // shift left by half the advance so the glyph centers on the
        // curve sample.
        let _ = write!(
 parts,
 "<g transform=\"translate({x:.dec$} {y:.dec$}) rotate({deg:.dec$}){scale_attr} translate({nx:.dec$} 0)\"><path d=\"{path_data}\" {fill_attrs}{opacity_attr}/></g>",
 x = pt.x,
 y = pt.y,
 deg = angle_deg,
 nx = -advance / 2.0,
 dec = DECIMAL_PLACES,
 );
        cursor += advance;
    }

    Some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_color::{ResolvedColor, Rgb};
    use slideglance_font::{BufferFontResolver, HeuristicTextMeasurer, ScriptFontContext};
    use slideglance_model::{
        AutoFit, BodyProperties, Paragraph, ParagraphProperties, RunProperties, TextBody, TextRun,
        TextVerticalType, Transform, VerticalAnchor, WrapMode,
    };
    use slideglance_utils::{Emu, Pt};

    fn body_props_with_warp(preset: Option<&str>) -> BodyProperties {
        BodyProperties {
            anchor: VerticalAnchor::T,
            margin_left: Emu::new(0),
            margin_right: Emu::new(0),
            margin_top: Emu::new(0),
            margin_bottom: Emu::new(0),
            wrap: WrapMode::Square,
            auto_fit: AutoFit::NoAutofit,
            font_scale: 1.0,
            ln_spc_reduction: 0.0,
            num_col: 1,
            vert: TextVerticalType::Horz,
            spc_first_last_para: false,
            compat_ln_spc: false,
            prst_tx_warp: preset.map(str::to_string),
        }
    }

    fn body_with_text(preset: Option<&str>, text: &str) -> TextBody {
        let props = RunProperties {
            font_size: Some(Pt::new(36.0)),
            font_family: Some("Vera".to_string()),
            color: Some(ResolvedColor::new(Rgb::from_hex("#FF0000").unwrap(), 1.0)),
            ..RunProperties::default()
        };
        TextBody {
            default_text_color: None,
            paragraphs: vec![Paragraph {
                runs: vec![TextRun {
                    text: text.to_string(),
                    properties: props,
                    field_type: None,
                }],
                properties: ParagraphProperties::default(),
                end_para_run_properties: None,
            }],
            body_properties: body_props_with_warp(preset),
        }
    }

    fn xfrm() -> Transform {
        Transform {
            offset_x: Emu::new(0),
            offset_y: Emu::new(0),
            extent_width: Emu::new(2_743_200),
            extent_height: Emu::new(914_400),
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
        }
    }

    fn render(body: &TextBody, transform: &Transform) -> Option<String> {
        let resolver = BufferFontResolver::new();
        render_text_body_as_warp_path(
            body,
            transform,
            &ScriptFontContext::empty(),
            &HeuristicTextMeasurer,
            &resolver,
        )
    }

    // --- Dispatch path (no font face required for these branches) ---

    #[test]
    fn no_preset_returns_none() {
        let body = body_with_text(None, "Hello");
        assert!(render(&body, &xfrm()).is_none());
    }

    #[test]
    fn unknown_preset_returns_none() {
        let body = body_with_text(Some("definitelyNotAWarpPreset"), "Hello");
        assert!(render(&body, &xfrm()).is_none());
    }

    #[test]
    fn empty_text_returns_empty_string() {
        // No visible runs -> Some("") (renderable, no output).
        let body = body_with_text(Some("textArchUp"), "");
        let r = render(&body, &xfrm()).expect("preset known");
        assert!(r.is_empty());
    }

    #[test]
    fn no_font_resolved_returns_none() {
        // BufferFontResolver::new() resolves nothing; expect None even
        // though preset and text are valid.
        let body = body_with_text(Some("textArchUp"), "Hello");
        assert!(render(&body, &xfrm()).is_none());
    }

    // --- WARP_CURVES coverage (no face needed) ---

    #[test]
    fn all_supported_presets_dispatch() {
        for preset in [
            "textArchUp",
            "textArchUpPour",
            "textArchDown",
            "textArchDownPour",
            "textCircle",
            "textCirclePour",
            "textButton",
            "textButtonPour",
            "textCurveUp",
            "textCurveDown",
            "textWave1",
            "textWave2",
            "textWave4",
            "textDoubleWave1",
            "textTriangle",
            "textTriangleInverted",
            "textSlantUp",
            "textSlantDown",
            "textInflate",
            "textInflateBottom",
            "textInflateTop",
            "textDeflate",
            "textDeflateBottom",
            "textDeflateTop",
            "textFadeRight",
            "textFadeLeft",
            "textFadeDown",
            "textFadeUp",
        ] {
            for t in [0.0, 0.5, 1.0] {
                assert!(
                    warp_curve_at(preset, t, 100.0, 50.0).is_some(),
                    "preset {preset} at t={t} should be supported"
                );
            }
        }
    }

    #[test]
    fn unknown_preset_in_curve_table_returns_none() {
        assert!(warp_curve_at("textNotReal", 0.5, 100.0, 50.0).is_none());
    }

    #[test]
    fn arch_up_at_t_zero_starts_at_left_edge() {
        let p = warp_curve_at("textArchUp", 0.0, 100.0, 50.0).expect("supported");
        // theta = PI -> cos = -1 -> x = 50 - 50*WARP_BOX_PAD = 50 - 42.5 = 7.5
        assert!((p.x - 7.5).abs() < 1e-9, "{p:?}", p = (p.x, p.y));
    }

    #[test]
    fn arch_up_at_t_one_ends_at_right_edge() {
        let p = warp_curve_at("textArchUp", 1.0, 100.0, 50.0).expect("supported");
        // theta = 0 -> cos = 1 -> x = 50 + 42.5 = 92.5
        assert!((p.x - 92.5).abs() < 1e-9);
    }

    #[test]
    fn inflate_sets_scale_y_above_one() {
        let p = warp_curve_at("textInflate", 0.5, 100.0, 50.0).expect("supported");
        assert!(p.scale_y > 1.0, "scale_y = {}", p.scale_y);
    }

    #[test]
    fn deflate_sets_scale_y_below_one() {
        let p = warp_curve_at("textDeflate", 0.5, 100.0, 50.0).expect("supported");
        assert!(p.scale_y < 1.0, "scale_y = {}", p.scale_y);
    }

    #[test]
    fn fade_right_starts_full_opacity_ends_dim() {
        let p0 = warp_curve_at("textFadeRight", 0.0, 100.0, 50.0).expect("supported");
        let p1 = warp_curve_at("textFadeRight", 1.0, 100.0, 50.0).expect("supported");
        assert!((p0.opacity - 1.0).abs() < 1e-9);
        assert!((p1.opacity - 0.1).abs() < 1e-9);
    }

    #[test]
    fn fade_left_starts_dim_ends_full() {
        let p0 = warp_curve_at("textFadeLeft", 0.0, 100.0, 50.0).expect("supported");
        let p1 = warp_curve_at("textFadeLeft", 1.0, 100.0, 50.0).expect("supported");
        assert!((p0.opacity - 0.1).abs() < 1e-9);
        assert!((p1.opacity - 1.0).abs() < 1e-9);
    }

    // --- arc-length helpers ---

    #[test]
    fn arc_length_is_positive_for_supported_preset() {
        let len = curve_arc_length("textArchUp", 100.0, 50.0);
        assert!(len > 0.0);
    }

    #[test]
    fn arc_length_is_zero_for_unsupported_preset() {
        let len = curve_arc_length("textNonexistent", 100.0, 50.0);
        assert_eq!(len, 0.0);
    }

    #[test]
    fn t_for_arc_length_at_zero_is_near_zero() {
        let t = t_for_arc_length("textArchUp", 100.0, 50.0, 0.0);
        assert!(t < 0.1);
    }

    #[test]
    fn t_for_arc_length_saturates_at_one() {
        let t = t_for_arc_length("textArchUp", 100.0, 50.0, 1e9);
        assert_eq!(t, 1.0);
    }

    #[test]
    fn t_for_arc_length_unsupported_preset_returns_one() {
        let t = t_for_arc_length("textNonexistent", 100.0, 50.0, 1.0);
        assert_eq!(t, 1.0);
    }
}
