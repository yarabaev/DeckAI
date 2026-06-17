//! Ported from.

use slideglance_color::{ColorMap, ColorResolver, ColorScheme, ResolvedColor, Rgb};
use slideglance_model::{EffectList, Fill, FormatScheme, Glow, Outline, OutlineFill, SolidFill};
use slideglance_parser::resolve_shape_style;
use slideglance_utils::Emu;

fn test_resolver() -> ColorResolver {
    let scheme = ColorScheme {
        dk1: Rgb::new(0, 0, 0),
        lt1: Rgb::new(0xFF, 0xFF, 0xFF),
        dk2: Rgb::new(0x44, 0x54, 0x6A),
        lt2: Rgb::new(0xE7, 0xE6, 0xE6),
        accent1: Rgb::new(0x44, 0x72, 0xC4),
        accent2: Rgb::new(0xED, 0x7D, 0x31),
        accent3: Rgb::new(0xA5, 0xA5, 0xA5),
        accent4: Rgb::new(0xFF, 0xC0, 0x00),
        accent5: Rgb::new(0x5B, 0x9B, 0xD5),
        accent6: Rgb::new(0x70, 0xAD, 0x47),
        hlink: Rgb::new(0x05, 0x63, 0xC1),
        fol_hlink: Rgb::new(0x95, 0x4F, 0x72),
    };
    ColorResolver::new(scheme, ColorMap::default())
}

fn test_fmt_scheme() -> FormatScheme {
    FormatScheme {
        // Three ascending fill styles (subtle / moderate / intense).
        fill_styles: vec![
            Fill::Solid(SolidFill {
                color: ResolvedColor::opaque(Rgb::new(0x10, 0x10, 0x10)),
            }),
            Fill::Solid(SolidFill {
                color: ResolvedColor::opaque(Rgb::new(0x20, 0x20, 0x20)),
            }),
            Fill::Solid(SolidFill {
                color: ResolvedColor::opaque(Rgb::new(0x30, 0x30, 0x30)),
            }),
        ],
        ln_styles: vec![
            Outline {
                width: Emu::new(6_350),
                fill: Some(OutlineFill::Solid(SolidFill {
                    color: ResolvedColor::opaque(Rgb::new(0x44, 0x44, 0x44)),
                })),
                dash_style: slideglance_model::DashStyle::Solid,
                custom_dash: None,
                line_cap: None,
                line_join: None,
                head_end: None,
                tail_end: None,
            },
            Outline {
                width: Emu::new(12_700),
                fill: Some(OutlineFill::Solid(SolidFill {
                    color: ResolvedColor::opaque(Rgb::new(0x55, 0x55, 0x55)),
                })),
                dash_style: slideglance_model::DashStyle::Dash,
                custom_dash: None,
                line_cap: None,
                line_join: None,
                head_end: None,
                tail_end: None,
            },
        ],
        effect_styles: vec![
            None,
            Some(EffectList {
                outer_shadow: None,
                inner_shadow: None,
                glow: Some(Glow {
                    radius: Emu::new(50_800),
                    color: ResolvedColor::opaque(Rgb::new(0xFF, 0xFF, 0x00)),
                }),
                soft_edge: None,
            }),
        ],
        bg_fill_styles: vec![
            Fill::Solid(SolidFill {
                color: ResolvedColor::opaque(Rgb::new(0xAA, 0x00, 0x00)),
            }),
            Fill::Solid(SolidFill {
                color: ResolvedColor::opaque(Rgb::new(0xBB, 0x00, 0x00)),
            }),
            Fill::Solid(SolidFill {
                color: ResolvedColor::opaque(Rgb::new(0xCC, 0x00, 0x00)),
            }),
        ],
    }
}

#[test]
fn returns_none_when_fmt_scheme_missing() {
    let xml = r#"<p:style xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                          xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
        <a:fillRef idx="1"><a:schemeClr val="accent1"/></a:fillRef>
    </p:style>"#;
    let result = resolve_shape_style(xml, None, &test_resolver()).unwrap();
    assert!(result.is_none());
}

#[test]
fn returns_none_for_empty_style_block() {
    let xml = r#"<p:style xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                          xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"/>"#;
    let result = resolve_shape_style(xml, Some(&test_fmt_scheme()), &test_resolver()).unwrap();
    assert!(result.is_none());
}

#[test]
fn fill_ref_idx_zero_yields_no_fill() {
    let xml = r#"<p:style xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                          xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
        <a:fillRef idx="0"><a:schemeClr val="accent1"/></a:fillRef>
    </p:style>"#;
    let result = resolve_shape_style(xml, Some(&test_fmt_scheme()), &test_resolver())
        .unwrap()
        .unwrap();
    assert!(result.fill.is_none());
}

#[test]
fn fill_ref_idx_one_picks_first_fill_with_override_color() {
    let xml = r#"<p:style xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                          xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
        <a:fillRef idx="1"><a:srgbClr val="FF0000"/></a:fillRef>
    </p:style>"#;
    let result = resolve_shape_style(xml, Some(&test_fmt_scheme()), &test_resolver())
        .unwrap()
        .unwrap();
    match result.fill.unwrap() {
        Fill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0xFF, 0, 0)),
        other => panic!("expected solid override, got {other:?}"),
    }
}

#[test]
fn fill_ref_idx_three_picks_third_fill_without_override() {
    let xml = r#"<p:style xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                          xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
        <a:fillRef idx="3"/>
    </p:style>"#;
    let result = resolve_shape_style(xml, Some(&test_fmt_scheme()), &test_resolver())
        .unwrap()
        .unwrap();
    match result.fill.unwrap() {
        Fill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0x30, 0x30, 0x30)),
        other => panic!("expected third template fill, got {other:?}"),
    }
}

#[test]
fn fill_ref_idx_1001_selects_bg_fill() {
    let xml = r#"<p:style xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                          xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
        <a:fillRef idx="1001"/>
    </p:style>"#;
    let result = resolve_shape_style(xml, Some(&test_fmt_scheme()), &test_resolver())
        .unwrap()
        .unwrap();
    match result.fill.unwrap() {
        Fill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0xAA, 0, 0)),
        other => panic!("expected first bg fill, got {other:?}"),
    }
}

#[test]
fn fill_ref_out_of_range_yields_none() {
    let xml = r#"<p:style xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                          xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
        <a:fillRef idx="99"/>
    </p:style>"#;
    let result = resolve_shape_style(xml, Some(&test_fmt_scheme()), &test_resolver())
        .unwrap()
        .unwrap();
    assert!(result.fill.is_none());
}

#[test]
fn ln_ref_picks_template_and_overrides_color() {
    let xml = r#"<p:style xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                          xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
        <a:lnRef idx="2"><a:srgbClr val="00FF00"/></a:lnRef>
    </p:style>"#;
    let result = resolve_shape_style(xml, Some(&test_fmt_scheme()), &test_resolver())
        .unwrap()
        .unwrap();
    let outline = result.outline.unwrap();
    assert_eq!(outline.width.raw(), 12_700);
    match outline.fill.unwrap() {
        OutlineFill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0, 0xFF, 0)),
        OutlineFill::Gradient(_) => panic!("expected solid override"),
    }
}

#[test]
fn effect_ref_picks_template_by_index() {
    let xml = r#"<p:style xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                          xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
        <a:effectRef idx="2"/>
    </p:style>"#;
    let result = resolve_shape_style(xml, Some(&test_fmt_scheme()), &test_resolver())
        .unwrap()
        .unwrap();
    let effects = result.effects.unwrap();
    assert!(effects.glow.is_some());
}

#[test]
fn effect_ref_idx_one_returns_none_when_first_template_is_empty() {
    let xml = r#"<p:style xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                          xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
        <a:effectRef idx="1"/>
    </p:style>"#;
    let result = resolve_shape_style(xml, Some(&test_fmt_scheme()), &test_resolver())
        .unwrap()
        .unwrap();
    assert!(result.effects.is_none());
}

#[test]
fn font_ref_carries_idx_and_color() {
    let xml = r#"<p:style xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                          xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
        <a:fontRef idx="major"><a:schemeClr val="lt1"/></a:fontRef>
    </p:style>"#;
    let result = resolve_shape_style(xml, Some(&test_fmt_scheme()), &test_resolver())
        .unwrap()
        .unwrap();
    let font = result.font_ref.unwrap();
    assert_eq!(font.idx, "major");
    assert_eq!(font.color.unwrap().rgb, Rgb::new(0xFF, 0xFF, 0xFF));
}

#[test]
fn font_ref_defaults_to_minor_when_attr_absent() {
    let xml = r#"<p:style xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                          xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
        <a:fontRef/>
    </p:style>"#;
    let result = resolve_shape_style(xml, Some(&test_fmt_scheme()), &test_resolver())
        .unwrap()
        .unwrap();
    let font = result.font_ref.unwrap();
    assert_eq!(font.idx, "minor");
    assert!(font.color.is_none());
}
