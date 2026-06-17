//! Run-level inline style attribute construction.
//!
//! Direct port of `buildStyleAttrs` and `buildBulletStyleAttrs` from
//! .

use slideglance_font::{CjkPlatform, FontMapping, ScriptFontContext};
use slideglance_model::{ParagraphProperties, RunProperties};

use crate::color::{alpha_str, color_hex};
use crate::geometry::fmt::n;
use crate::svg_builder::escape_xml_attr;
use crate::text::font_family::build_font_family_value;

/// Build the inline `font-*` / fill / decoration / outline attributes for
/// one run. `font_families` overrides the default `[font_family, font_family_ea]`
/// pair when the caller wants to inject a different priority order
/// (e.g. EA-first for a script-split EA segment).
///
/// `script_fonts` carries the deck's theme `<a:font script="…">`
/// typefaces so the emitted `font-family` chain can fall back to the
/// deck's authored CJK faces even when the run only declares a Latin
/// `<a:latin typeface=…/>`. Pass [`ScriptFontContext::empty`] when the
/// theme has no CJK script fonts.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn build_style_attrs(
    props: &RunProperties,
    font_scale: f64,
    font_families: Option<&[Option<&str>]>,
    mapping: &FontMapping,
    cjk_platform: CjkPlatform,
    script_fonts: &ScriptFontContext,
) -> String {
    let mut styles: Vec<String> = Vec::new();

    if let Some(size) = props.font_size {
        let scaled = size.raw() * font_scale;
        styles.push(format!("font-size=\"{}pt\"", n(scaled)));
    }

    // Four-element default priority list: latin, EA, complex-script, sym.
    // Callers may override with a script-specific order via font_families.
    let default_pair: [Option<&str>; 4] = [
        props.font_family.as_deref(),
        props.font_family_ea.as_deref(),
        props.font_family_cs.as_deref(),
        props.font_family_sym.as_deref(),
    ];
    let fonts: &[Option<&str>] = font_families.unwrap_or(&default_pair);
    if let Some(value) = build_font_family_value(fonts, mapping, cjk_platform, script_fonts) {
        styles.push(format!("font-family=\"{value}\""));
    }

    if props.bold {
        styles.push("font-weight=\"bold\"".to_string());
    }
    if props.italic {
        styles.push("font-style=\"italic\"".to_string());
    }

    if let Some(color) = &props.color {
        styles.push(format!("fill=\"{}\"", color_hex(color)));
        if color.alpha < 1.0 {
            styles.push(format!("fill-opacity=\"{}\"", alpha_str(color.alpha)));
        }
    }

    let mut decorations: Vec<&str> = Vec::new();
    if props.underline {
        decorations.push("underline");
    }
    if props.strikethrough {
        decorations.push("line-through");
    }
    if !decorations.is_empty() {
        styles.push(format!("text-decoration=\"{}\"", decorations.join(" ")));
    }

    if props.baseline > 0.0 {
        styles.push("baseline-shift=\"super\"".to_string());
    } else if props.baseline < 0.0 {
        styles.push("baseline-shift=\"sub\"".to_string());
    }

    // `<a:rPr @spc>` is stored as HundredthPt; CSS letter-spacing uses pt.
    if let Some(spc) = props.char_spacing {
        let pt = spc.to_points().raw();
        if pt != 0.0 {
            styles.push(format!("letter-spacing=\"{}pt\"", n(pt)));
        }
    }

    if let Some(outline) = &props.outline {
        let stroke_width = outline.width.to_pixels();
        styles.push(format!("stroke=\"{}\"", color_hex(&outline.color)));
        styles.push(format!("stroke-width=\"{}\"", n(stroke_width)));
        if outline.color.alpha < 1.0 {
            styles.push(format!(
                "stroke-opacity=\"{}\"",
                alpha_str(outline.color.alpha)
            ));
        }
        styles.push("paint-order=\"stroke\"".to_string());
    }

    styles.join(" ")
}

/// Build inline style attributes for a bullet character. The bullet has
/// distinct OOXML overrides (`buSzPct`, `buFont`, `buClr`).
#[must_use]
pub fn build_bullet_style_attrs(
    props: &ParagraphProperties,
    text_font_size_pt: f64,
    _font_scale: f64,
) -> String {
    let mut styles: Vec<String> = Vec::new();

    if let Some(pct) = props.bullet_size_pct {
        let size = text_font_size_pt * (pct / 100_000.0);
        styles.push(format!("font-size=\"{}pt\"", n(size)));
    }
    if let Some(font) = &props.bullet_font {
        styles.push(format!("font-family=\"{}\"", escape_xml_attr(font)));
    }
    if let Some(color) = &props.bullet_color {
        styles.push(format!("fill=\"{}\"", color_hex(color)));
        if color.alpha < 1.0 {
            styles.push(format!("fill-opacity=\"{}\"", alpha_str(color.alpha)));
        }
    }
    styles.join(" ")
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use slideglance_color::{ResolvedColor, Rgb};
    use slideglance_font::{FontMapping, ScriptFontContext};
    use slideglance_model::{BulletType, RunProperties, TextOutline};
    use slideglance_utils::{Emu, Pt};

    fn empty() -> RunProperties {
        RunProperties::default()
    }

    fn mapping() -> FontMapping {
        FontMapping::new()
    }

    #[test]
    fn empty_run_produces_no_styles() {
        let s = build_style_attrs(
            &empty(),
            1.0,
            None,
            &mapping(),
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
        );
        assert!(s.is_empty());
    }

    #[test]
    fn font_size_includes_pt_suffix() {
        let mut p = empty();
        p.font_size = Some(Pt::new(18.0));
        let s = build_style_attrs(
            &p,
            1.0,
            None,
            &mapping(),
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
        );
        assert!(s.contains("font-size=\"18pt\""));
    }

    #[test]
    fn font_scale_multiplies_size() {
        let mut p = empty();
        p.font_size = Some(Pt::new(20.0));
        let s = build_style_attrs(
            &p,
            0.5,
            None,
            &mapping(),
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
        );
        assert!(s.contains("font-size=\"10pt\""));
    }

    #[test]
    fn bold_emits_font_weight() {
        let mut p = empty();
        p.bold = true;
        assert!(build_style_attrs(
            &p,
            1.0,
            None,
            &mapping(),
            CjkPlatform::Other,
            &ScriptFontContext::empty()
        )
        .contains("font-weight=\"bold\""));
    }

    #[test]
    fn italic_emits_font_style() {
        let mut p = empty();
        p.italic = true;
        assert!(build_style_attrs(
            &p,
            1.0,
            None,
            &mapping(),
            CjkPlatform::Other,
            &ScriptFontContext::empty()
        )
        .contains("font-style=\"italic\""));
    }

    #[test]
    fn color_with_alpha_emits_fill_and_opacity() {
        let mut p = empty();
        p.color = Some(ResolvedColor::new(Rgb::new(0xFF, 0, 0), 0.5));
        let s = build_style_attrs(
            &p,
            1.0,
            None,
            &mapping(),
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
        );
        assert!(s.contains("fill=\"#FF0000\""));
        assert!(s.contains("fill-opacity=\"0.5\""));
    }

    #[test]
    fn decorations_combine() {
        let mut p = empty();
        p.underline = true;
        p.strikethrough = true;
        let s = build_style_attrs(
            &p,
            1.0,
            None,
            &mapping(),
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
        );
        assert!(s.contains("text-decoration=\"underline line-through\""));
    }

    #[test]
    fn baseline_super_and_sub() {
        let mut p = empty();
        p.baseline = 30000.0;
        assert!(build_style_attrs(
            &p,
            1.0,
            None,
            &mapping(),
            CjkPlatform::Other,
            &ScriptFontContext::empty()
        )
        .contains("baseline-shift=\"super\""));
        p.baseline = -25000.0;
        assert!(build_style_attrs(
            &p,
            1.0,
            None,
            &mapping(),
            CjkPlatform::Other,
            &ScriptFontContext::empty()
        )
        .contains("baseline-shift=\"sub\""));
    }

    #[test]
    fn outline_emits_stroke_attrs_with_paint_order() {
        let mut p = empty();
        p.outline = Some(TextOutline {
            width: Emu::new(9_525), // 1 px
            color: ResolvedColor::new(Rgb::new(0, 0, 0), 1.0),
        });
        let s = build_style_attrs(
            &p,
            1.0,
            None,
            &mapping(),
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
        );
        assert!(s.contains("stroke=\"#000000\""));
        assert!(s.contains("stroke-width=\"1\""));
        assert!(s.contains("paint-order=\"stroke\""));
    }

    #[test]
    fn override_font_families_takes_precedence() {
        let mut p = empty();
        p.font_family = Some("Latin".to_string());
        p.font_family_ea = Some("EA".to_string());
        let custom = [Some("OnlyThis")];
        let s = build_style_attrs(
            &p,
            1.0,
            Some(&custom),
            &mapping(),
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
        );
        assert!(s.contains("OnlyThis"));
        assert!(!s.contains("\"EA\""));
    }

    // --- bullet ---

    #[test]
    fn bullet_size_pct_scales_font_size() {
        let mut props = ParagraphProperties::default();
        props.bullet = Some(BulletType::Char {
            char: "•".to_string(),
        });
        props.bullet_size_pct = Some(75_000.0);
        let s = build_bullet_style_attrs(&props, 18.0, 1.0);
        assert!(s.contains("font-size=\"13.5pt\""));
    }

    #[test]
    fn bullet_font_overrides_family() {
        let mut props = ParagraphProperties::default();
        props.bullet_font = Some("Wingdings".to_string());
        let s = build_bullet_style_attrs(&props, 18.0, 1.0);
        assert!(s.contains("font-family=\"Wingdings\""));
    }

    #[test]
    fn bullet_color_with_alpha() {
        let mut props = ParagraphProperties::default();
        props.bullet_color = Some(ResolvedColor::new(Rgb::new(0xFF, 0, 0), 0.5));
        let s = build_bullet_style_attrs(&props, 18.0, 1.0);
        assert!(s.contains("fill=\"#FF0000\""));
        assert!(s.contains("fill-opacity=\"0.5\""));
    }
}
