//! `WordArt` (`<a:bodyPr><a:prstTxWarp/>`) text-mode rendering.
//!
//! Direct port of lines
//! 1782–1870 (`warpPresetToPathD`, `djb2Hash`) and 1723–1773
//! (`renderTextBodyAsWarp`).
//!
//! `WordArt` warps emit text along a curve. In text mode we use SVG's
//! `<textPath>` referencing a `<defs><path>`-defined curve. Path-mode
//! glyph-along-curve rendering ships in
//! [`super::warp_path::render_text_body_as_warp_path`] — the renderer
//! dispatches there automatically when a font resolver is supplied
//! (typically the PNG output path).
//!
//! The supported preset list mirrors the spec exactly. Unknown
//! presets return `None` so `render_text_body` falls through to the
//! regular text rendering pipeline.

use std::fmt::Write as _;

use slideglance_font::{CjkPlatform, FontMapping, ScriptFontContext};
use slideglance_model::{RunProperties, TextBody, Transform};

use crate::color::{alpha_str, color_hex};
use crate::svg_builder::{escape_xml_attr, escape_xml_text};
use crate::text::font_family::build_font_family_value;
use crate::text::layout::PX_PER_PT;

/// Render a [`TextBody`] in `WordArt` text-along-path mode.
///
/// Returns:
/// - `Some(svg)` — text was rendered along the warp curve.
/// - `None` — either the preset is unknown (caller should fall back
///   to regular text rendering) or the body is empty (caller should
///   emit nothing).
///
/// Empty bodies and bodies with only whitespace runs return
/// `Some(String::new)` to mirror the TS contract: the caller treats
/// an empty string as "rendered, but no visible output," whereas
/// `None` triggers the fallback path.
///
/// `mapping` and `cjk_platform` are threaded through to
/// [`build_font_family_value`] for the same Office-aware font
/// resolution the regular text renderer applies.
#[allow(clippy::similar_names)]
#[must_use]
pub fn render_text_body_as_warp(
    text_body: &TextBody,
    transform: &Transform,
    mapping: &FontMapping,
    cjk_platform: CjkPlatform,
    script_fonts: &ScriptFontContext,
) -> Option<String> {
    let bp = &text_body.body_properties;
    let preset = bp.prst_tx_warp.as_deref()?;
    let w = transform.extent_width.to_pixels();
    let h = transform.extent_height.to_pixels();
    let path_d = warp_preset_to_path_d(preset, w, h)?;

    // Flatten visible runs to plain text + capture first run's style.
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

    // Stable ID via djb2 hash of the path so two identical warps share
    // a single `<defs>` entry.
    let path_id = format!("warp-{}", to_base36(djb2_hash(&path_d)));

    let font_size_pt = props.font_size.map_or(24.0, slideglance_utils::Pt::raw);
    let family_value = build_font_family_value(
        &[
            props.font_family.as_deref(),
            props.font_family_ea.as_deref(),
        ],
        mapping,
        cjk_platform,
        script_fonts,
    );
    let mut family_attr = String::new();
    if let Some(fam) = family_value {
        let _ = write!(family_attr, " font-family=\"{fam}\"");
    }
    let weight_attr = if props.bold {
        " font-weight=\"bold\""
    } else {
        ""
    };
    let style_attr = if props.italic {
        " font-style=\"italic\""
    } else {
        ""
    };
    let (fill, fill_opacity) = match &props.color {
        Some(c) if c.alpha < 1.0 => (
            color_hex(c),
            format!(" fill-opacity=\"{}\"", alpha_str(c.alpha)),
        ),
        Some(c) => (color_hex(c), String::new()),
        None => ("#000000".to_string(), String::new()),
    };

    let font_size_px = font_size_pt * PX_PER_PT;

    let mut out = String::new();
    let _ = write!(
        out,
        "<defs><path id=\"{path_id}\" d=\"{}\"/></defs>",
        escape_xml_attr(&path_d)
    );
    let _ = write!(
        out,
        "<text{family_attr} font-size=\"{font_size_px}\"{weight_attr}{style_attr} fill=\"{fill}\"{fill_opacity}>"
    );
    let _ = write!(
        out,
        "<textPath href=\"#{path_id}\" xlink:href=\"#{path_id}\" startOffset=\"50%\" text-anchor=\"middle\">{}</textPath>",
        escape_xml_text(&text)
    );
    out.push_str("</text>");
    Some(out)
}

/// Build the SVG `d` attribute for a `WordArt` preset within a
/// `w` × `h` text-frame box.
///
/// Returns `None` for presets not implemented by the spec so
/// the caller can fall through to regular text rendering. Coordinates
/// are local to the text-frame origin (top-left).
#[must_use]
pub fn warp_preset_to_path_d(preset: &str, w: f64, h: f64) -> Option<String> {
    let path = match preset {
        "textArchUp" | "textArchUpPour" => {
            // Half ellipse opening downward — text along upper arc.
            let rx = w / 2.0;
            let ry = h;
            format!("M0,{h} A{rx},{ry} 0 0,1 {w},{h}")
        }
        "textArchDown" | "textArchDownPour" => {
            // Half ellipse opening upward — text along lower arc.
            let rx = w / 2.0;
            let ry = h;
            format!("M0,0 A{rx},{ry} 0 0,0 {w},0")
        }
        "textCircle" | "textCirclePour" | "textButton" | "textButtonPour" => {
            // Full circle drawn as two large-arc segments so textPath
            // wraps cleanly.
            let r = w.min(h) / 2.0;
            let cx = w / 2.0;
            let cy = h / 2.0;
            format!(
                "M{lx},{cy} A{r},{r} 0 1,1 {rx},{cy} A{r},{r} 0 1,1 {lx},{cy}",
                lx = cx - r,
                rx = cx + r
            )
        }
        "textCurveUp" => {
            let mid_y = h * 0.85;
            let neg = -h * 0.2;
            let half_w = w / 2.0;
            format!("M0,{mid_y} Q{half_w},{neg} {w},{mid_y}")
        }
        "textCurveDown" => {
            let mid_y = h * 0.15;
            let pos = h * 1.2;
            let half_w = w / 2.0;
            format!("M0,{mid_y} Q{half_w},{pos} {w},{mid_y}")
        }
        "textWave1" | "textWave2" | "textWave4" | "textDoubleWave1" => {
            let mid_y = h / 2.0;
            let amp = h / 4.0;
            let q1x = w / 4.0;
            let q2x = w / 2.0;
            let q3x = (w * 3.0) / 4.0;
            format!(
                "M0,{mid_y} Q{q1x},{} {q2x},{mid_y} Q{q3x},{} {w},{mid_y}",
                mid_y - amp,
                mid_y + amp
            )
        }
        "textTriangle" | "textTriangleInverted" => {
            let peak_y = if preset == "textTriangle" { 0.0 } else { h };
            let half_w = w / 2.0;
            let inv = h - peak_y;
            format!("M0,{inv} L{half_w},{peak_y} L{w},{inv}")
        }
        "textSlantUp" => format!("M0,{} L{w},{}", h * 0.85, h * 0.15),
        "textSlantDown" => format!("M0,{} L{w},{}", h * 0.15, h * 0.85),
        "textInflate" | "textInflateBottom" | "textInflateTop" | "textDeflate"
        | "textDeflateBottom" | "textDeflateTop" | "textFadeUp" | "textFadeDown"
        | "textFadeLeft" | "textFadeRight" => {
            // text-mode (textPath) cannot replicate per-glyph scale —
            // emit a flat baseline. Path-mode warp covers the actual
            // distortion when it lands.
            let y = h * 0.7;
            format!("M0,{y} L{w},{y}")
        }
        _ => return None,
    };
    Some(path)
}

/// Stable 32-bit djb2 hash. Mirrors the spec's
/// `djb2Hash`, returning the same digest on the same input bytes.
#[must_use]
pub(crate) fn djb2_hash(s: &str) -> u32 {
    let mut h: u32 = 5381;
    for byte in s.bytes() {
        h = h.wrapping_mul(33).wrapping_add(u32::from(byte));
    }
    h
}

/// Convert a `u32` to its lowercase base-36 representation.
fn to_base36(mut n: u32) -> String {
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if n == 0 {
        return "0".to_string();
    }
    let mut buf = [0_u8; 7];
    let mut i = buf.len();
    while n > 0 {
        i -= 1;
        buf[i] = ALPHABET[(n % 36) as usize];
        n /= 36;
    }
    std::str::from_utf8(&buf[i..]).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_color::{ResolvedColor, Rgb};
    use slideglance_font::FontMapping;
    use slideglance_model::{
        AutoFit, BodyProperties, Paragraph, ParagraphProperties, RunProperties, TextBody, TextRun,
        TextVerticalType, Transform, VerticalAnchor, WrapMode,
    };
    use slideglance_utils::{Emu, Pt};

    fn render(body: &TextBody, transform: &Transform) -> Option<String> {
        render_text_body_as_warp(
            body,
            transform,
            &FontMapping::new(),
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
        )
    }

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
            extent_width: Emu::new(914_400),
            extent_height: Emu::new(914_400),
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
        }
    }

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
        // Body has the warp set but no visible runs — TS returns the
        // empty string (renderable, no output) rather than None.
        let body = body_with_text(Some("textArchUp"), "");
        let r = render(&body, &xfrm()).expect("preset known");
        assert!(r.is_empty());
    }

    #[test]
    fn arch_up_emits_textpath_with_arc_path() {
        let body = body_with_text(Some("textArchUp"), "Hello");
        let r = render(&body, &xfrm()).expect("rendered");
        assert!(r.contains("<defs><path id=\"warp-"));
        assert!(r.contains(" A48,96 0 0,1 96,96"), "{r}");
        assert!(r.contains("<textPath href=\"#warp-"));
        assert!(r.contains("Hello"));
    }

    #[test]
    fn font_size_emitted_in_pixels() {
        let body = body_with_text(Some("textArchUp"), "X");
        let r = render(&body, &xfrm()).expect("rendered");
        // 36pt × (96/72) = 48 px.
        assert!(r.contains("font-size=\"48\""), "{r}");
    }

    #[test]
    fn color_alpha_below_one_emits_fill_opacity() {
        let mut body = body_with_text(Some("textArchUp"), "X");
        body.paragraphs[0].runs[0].properties.color =
            Some(ResolvedColor::new(Rgb::from_hex("#0000FF").unwrap(), 0.5));
        let r = render(&body, &xfrm()).expect("rendered");
        assert!(r.contains("fill=\"#0000FF\""));
        assert!(r.contains("fill-opacity=\"0.5\""));
    }

    #[test]
    fn bold_italic_attrs_appear_when_set() {
        let mut body = body_with_text(Some("textArchUp"), "X");
        body.paragraphs[0].runs[0].properties.bold = true;
        body.paragraphs[0].runs[0].properties.italic = true;
        let r = render(&body, &xfrm()).expect("rendered");
        assert!(r.contains("font-weight=\"bold\""));
        assert!(r.contains("font-style=\"italic\""));
    }

    #[test]
    fn arch_down_emits_inverted_arc() {
        let body = body_with_text(Some("textArchDown"), "Hello");
        let r = render(&body, &xfrm()).expect("rendered");
        assert!(r.contains("M0,0 A48,96 0 0,0 96,0"));
    }

    #[test]
    fn circle_emits_two_arc_segments() {
        let body = body_with_text(Some("textCircle"), "Loop");
        let r = render(&body, &xfrm()).expect("rendered");
        let arc_count = r.matches(" A").count();
        assert_eq!(arc_count, 2, "{r}");
    }

    #[test]
    fn wave1_emits_two_quadratic_curves() {
        let body = body_with_text(Some("textWave1"), "Wave");
        let r = render(&body, &xfrm()).expect("rendered");
        assert_eq!(r.matches(" Q").count(), 2, "{r}");
    }

    #[test]
    fn triangle_emits_two_line_segments() {
        let body = body_with_text(Some("textTriangle"), "Tri");
        let r = render(&body, &xfrm()).expect("rendered");
        assert_eq!(r.matches(" L").count(), 2, "{r}");
    }

    #[test]
    fn inflate_falls_back_to_flat_baseline() {
        let body = body_with_text(Some("textInflate"), "Inf");
        let r = render(&body, &xfrm()).expect("rendered");
        // Two endpoints of a horizontal line at y = 0.7 * h.
        assert!(r.contains("M0,67.19999999999999 L96,67.19999999999999"));
    }

    // --- djb2 ---

    #[test]
    fn djb2_hash_handles_empty_input() {
        // djb2 seed is 5381; empty input should yield the seed.
        assert_eq!(djb2_hash(""), 5381);
    }

    #[test]
    fn djb2_hash_is_deterministic() {
        assert_eq!(djb2_hash("abc"), djb2_hash("abc"));
        assert_ne!(djb2_hash("abc"), djb2_hash("abd"));
    }

    #[test]
    fn base36_zero_yields_zero() {
        assert_eq!(to_base36(0), "0");
    }

    #[test]
    fn base36_36_yields_10() {
        assert_eq!(to_base36(36), "10");
    }
}
