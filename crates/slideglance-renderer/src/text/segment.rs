//! Single-run text segment -> `<tspan>` (or hyperlink-wrapped variant).
//!
//! Direct port of `renderSegment` (and `needsScriptSplit`) from
//! .

use slideglance_font::{is_sym_pua_codepoint, CjkPlatform, FontMapping, ScriptFontContext};
use slideglance_model::RunProperties;

use crate::svg_builder::{escape_xml_attr, escape_xml_text};
use crate::text::script::split_by_script;
use crate::text::style::build_style_attrs;

// Highlight is rendered as a separate `<rect>` sibling of the `<text>`
// element (see `text/body/mod.rs::render_text_body`). Putting `filter` on
// `<tspan>` was unreliable across browsers — Chromium / WebKit interpret
// the filter region as the parent `<text>` bbox, so the highlight bled
// across the whole paragraph. The rect path is computed from the
// pre-wrapped segments and the same measurer used for layout, so the box
// lines up with the glyphs without depending on browser SVG quirks.

/// True when the run has a Latin / EA font pair worth splitting per
/// script. When both are `None` or equal, the segment renders as one
/// `<tspan>` regardless of code-point composition.
fn needs_script_split(props: &RunProperties) -> bool {
    matches!(
        (&props.font_family, &props.font_family_ea),
        (Some(latin), Some(ea)) if latin != ea
    )
}

/// Render one text run as one or more `<tspan>` elements.
///
/// `prefix` is appended **inside** the opening `<tspan>` of the *first*
/// emitted element — callers use it to inject `x="…" dy="…" text-anchor="…"`
/// at line starts. Subsequent split tspans within the same run inherit
/// position from the SVG renderer's normal flow.
///
/// Hyperlinks wrap the entire span sequence in `<a href="...">…</a>`,
/// matching the spec.
#[must_use]
pub fn render_segment(
    text: &str,
    props: &RunProperties,
    font_scale: f64,
    prefix: &str,
    mapping: &FontMapping,
    cjk_platform: CjkPlatform,
    script_fonts: &ScriptFontContext,
) -> String {
    let inner = if needs_script_split(props) {
        render_split(
            text,
            props,
            font_scale,
            prefix,
            mapping,
            cjk_platform,
            script_fonts,
        )
    } else {
        let styles =
            build_style_attrs(props, font_scale, None, mapping, cjk_platform, script_fonts);
        format!("<tspan {prefix}{styles}>{}</tspan>", escape_xml_text(text))
    };

    if let Some(link) = &props.hyperlink {
        let href = escape_xml_attr(&link.url);
        format!("<a href=\"{href}\">{inner}</a>")
    } else {
        inner
    }
}

fn render_split(
    text: &str,
    props: &RunProperties,
    font_scale: f64,
    prefix: &str,
    mapping: &FontMapping,
    cjk_platform: CjkPlatform,
    script_fonts: &ScriptFontContext,
) -> String {
    use slideglance_font::Script;
    use std::fmt::Write as _;
    let parts = split_by_script(text);
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        // CJK Script Equality: each script routes to its own theme fallback.
        // spec used a single Jpan fallback for all EA text; we apply
        // the per-script fallback so Korean runs use hang_fallback(), etc.
        let fonts: Vec<Option<&str>> = match part.script {
            Script::Korean => vec![
                props.font_family_ea.as_deref(),
                script_fonts.hang_fallback(),
                props.font_family.as_deref(),
            ],
            Script::Japanese => vec![
                props.font_family_ea.as_deref(),
                script_fonts.jpan_fallback(),
                props.font_family.as_deref(),
            ],
            Script::SimplifiedChinese => vec![
                props.font_family_ea.as_deref(),
                script_fonts.hans_fallback(),
                props.font_family.as_deref(),
            ],
            Script::TraditionalChinese => vec![
                props.font_family_ea.as_deref(),
                script_fonts.hant_fallback(),
                props.font_family.as_deref(),
            ],
            // PUA segments (U+E000-EFFF, U+F100-F8FF) use the EA font stack.
            Script::Pua => vec![
                props.font_family_ea.as_deref(),
                script_fonts.jpan_fallback(),
                props.font_family.as_deref(),
            ],
            Script::Latin => {
                // Sym-PUA (U+F000-F0FF): Symbol / Wingdings codepoints that
                // must use the run's sym font. If the entire part falls in
                // this range and sym is set, prefer it over the Latin font.
                let all_sym_pua = part.text.chars().all(|c| is_sym_pua_codepoint(c as u32));
                if all_sym_pua && props.font_family_sym.is_some() {
                    vec![
                        props.font_family_sym.as_deref(),
                        props.font_family.as_deref(),
                    ]
                } else {
                    vec![
                        props.font_family.as_deref(),
                        props.font_family_ea.as_deref(),
                    ]
                }
            }
        };
        let styles = build_style_attrs(
            props,
            font_scale,
            Some(&fonts),
            mapping,
            cjk_platform,
            script_fonts,
        );
        let prefix_for = if i == 0 { prefix } else { "" };
        let _ = write!(
            out,
            "<tspan {prefix_for}{styles}>{}</tspan>",
            escape_xml_text(&part.text)
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_color::{ResolvedColor, Rgb};
    use slideglance_font::FontMapping;
    use slideglance_model::Hyperlink;
    use slideglance_utils::Pt;

    fn ctx() -> (FontMapping, ScriptFontContext) {
        (FontMapping::new(), ScriptFontContext::empty())
    }

    fn run(text_props: impl FnOnce(&mut RunProperties)) -> RunProperties {
        let mut p = RunProperties::default();
        text_props(&mut p);
        p
    }

    #[test]
    fn simple_run_emits_one_tspan() {
        let (m, s) = ctx();
        let p = run(|p| p.font_size = Some(Pt::new(12.0)));
        let out = render_segment("Hi", &p, 1.0, "x=\"0\" ", &m, CjkPlatform::Other, &s);
        assert!(out.starts_with("<tspan x=\"0\" "));
        assert!(out.contains(">Hi</tspan>"));
        assert!(out.contains("font-size=\"12pt\""));
    }

    #[test]
    fn hyperlink_wraps_in_anchor() {
        let (m, s) = ctx();
        let p = run(|p| {
            p.hyperlink = Some(Hyperlink {
                url: "https://example.com".to_string(),
                tooltip: None,
            });
        });
        let out = render_segment("link", &p, 1.0, "", &m, CjkPlatform::Other, &s);
        assert!(out.starts_with("<a href=\"https://example.com\">"));
        assert!(out.ends_with("</a>"));
    }

    #[test]
    fn no_split_when_only_latin_set() {
        let (m, s) = ctx();
        let p = run(|p| p.font_family = Some("Calibri".to_string()));
        let out = render_segment("Hello", &p, 1.0, "", &m, CjkPlatform::Other, &s);
        assert_eq!(out.matches("<tspan").count(), 1);
    }

    #[test]
    fn split_when_latin_and_ea_differ_and_text_mixed() {
        let (m, s) = ctx();
        let p = run(|p| {
            p.font_family = Some("Calibri".to_string());
            p.font_family_ea = Some("MS Mincho".to_string());
        });
        let out = render_segment("Hello 한국", &p, 1.0, "", &m, CjkPlatform::Other, &s);
        assert!(out.matches("<tspan").count() >= 2);
    }

    #[test]
    fn no_split_when_latin_equals_ea() {
        let (m, s) = ctx();
        let p = run(|p| {
            p.font_family = Some("Arial".to_string());
            p.font_family_ea = Some("Arial".to_string());
        });
        let out = render_segment("hi 한", &p, 1.0, "", &m, CjkPlatform::Other, &s);
        assert_eq!(out.matches("<tspan").count(), 1);
    }

    #[test]
    fn xml_unsafe_text_is_escaped() {
        let (m, s) = ctx();
        let p = RunProperties::default();
        let out = render_segment("a < b & c", &p, 1.0, "", &m, CjkPlatform::Other, &s);
        assert!(out.contains("a &lt; b &amp; c"));
    }

    #[test]
    fn color_with_alpha_round_trips_through_segment() {
        let (m, s) = ctx();
        let p = run(|p| {
            p.color = Some(ResolvedColor::new(Rgb::new(0, 0xFF, 0), 0.4));
        });
        let out = render_segment("X", &p, 1.0, "", &m, CjkPlatform::Other, &s);
        assert!(out.contains("fill=\"#00FF00\""));
        assert!(out.contains("fill-opacity=\"0.4\""));
    }
}
