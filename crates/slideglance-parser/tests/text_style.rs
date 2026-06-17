//! Ported from.

use slideglance_color::{ColorMap, ColorResolver, ColorScheme, Rgb};
use slideglance_model::{AutoNumScheme, BulletType, FontScheme, ParagraphAlignment};
use slideglance_parser::{parse_list_style, resolve_theme_font};

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

#[test]
fn returns_none_for_empty_list_style() {
    let result = parse_list_style("<a:lstStyle/>", None).unwrap();
    assert!(result.is_none());
}

#[test]
fn parses_def_p_pr_alignment_and_margins() {
    let xml = r#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:defPPr algn="ctr" marL="91440" indent="-91440"/>
    </a:lstStyle>"#;
    let result = parse_list_style(xml, None).unwrap().unwrap();
    let def = result.default_paragraph.unwrap();
    assert!(matches!(def.alignment, Some(ParagraphAlignment::Ctr)));
    assert_eq!(def.margin_left.unwrap().raw(), 91_440);
    assert_eq!(def.indent.unwrap().raw(), -91_440);
    assert_eq!(result.levels.len(), 9);
    assert!(result.levels.iter().all(Option::is_none));
}

#[test]
fn parses_lvl1_through_lvl9_independently() {
    let xml = r#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:lvl1pPr algn="l"/>
        <a:lvl3pPr algn="r"/>
        <a:lvl9pPr algn="just"/>
    </a:lstStyle>"#;
    let result = parse_list_style(xml, None).unwrap().unwrap();
    assert!(matches!(
        result.levels[0].as_ref().unwrap().alignment,
        Some(ParagraphAlignment::L)
    ));
    assert!(result.levels[1].is_none());
    assert!(matches!(
        result.levels[2].as_ref().unwrap().alignment,
        Some(ParagraphAlignment::R)
    ));
    assert!(result.levels[7].is_none());
    assert!(matches!(
        result.levels[8].as_ref().unwrap().alignment,
        Some(ParagraphAlignment::Just)
    ));
}

#[test]
fn parses_bu_none_bullet() {
    let xml = r#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:lvl1pPr><a:buNone/></a:lvl1pPr>
    </a:lstStyle>"#;
    let result = parse_list_style(xml, None).unwrap().unwrap();
    let lvl = result.levels[0].as_ref().unwrap();
    assert!(matches!(lvl.bullet, Some(BulletType::None)));
}

#[test]
fn parses_bu_char_with_explicit_char() {
    let xml = r#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:lvl1pPr><a:buChar char="•"/></a:lvl1pPr>
    </a:lstStyle>"#;
    let result = parse_list_style(xml, None).unwrap().unwrap();
    let lvl = result.levels[0].as_ref().unwrap();
    match &lvl.bullet {
        Some(BulletType::Char { char }) => assert_eq!(char, "•"),
        other => panic!("expected char bullet, got {other:?}"),
    }
}

#[test]
fn parses_bu_char_default_when_attr_missing() {
    let xml = r#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:lvl1pPr><a:buChar/></a:lvl1pPr>
    </a:lstStyle>"#;
    let result = parse_list_style(xml, None).unwrap().unwrap();
    match &result.levels[0].as_ref().unwrap().bullet {
        Some(BulletType::Char { char }) => assert_eq!(char, "\u{2022}"),
        other => panic!("expected default bullet char, got {other:?}"),
    }
}

#[test]
fn parses_bu_auto_num_with_scheme_and_start_at() {
    let xml = r#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:lvl1pPr><a:buAutoNum type="romanUcPeriod" startAt="3"/></a:lvl1pPr>
    </a:lstStyle>"#;
    let result = parse_list_style(xml, None).unwrap().unwrap();
    match &result.levels[0].as_ref().unwrap().bullet {
        Some(BulletType::AutoNum { scheme, start_at }) => {
            assert!(matches!(scheme, AutoNumScheme::RomanUcPeriod));
            assert_eq!(*start_at, 3);
        }
        other => panic!("expected autoNum bullet, got {other:?}"),
    }
}

#[test]
fn invalid_auto_num_scheme_falls_back_to_arabic_period() {
    let xml = r#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:lvl1pPr><a:buAutoNum type="bogusScheme"/></a:lvl1pPr>
    </a:lstStyle>"#;
    let result = parse_list_style(xml, None).unwrap().unwrap();
    match &result.levels[0].as_ref().unwrap().bullet {
        Some(BulletType::AutoNum { scheme, start_at }) => {
            assert!(matches!(scheme, AutoNumScheme::ArabicPeriod));
            assert_eq!(*start_at, 1);
        }
        other => panic!("expected fallback autoNum, got {other:?}"),
    }
}

#[test]
fn parses_bullet_font_and_size_pct() {
    let xml = r#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:lvl1pPr>
            <a:buFont typeface="Wingdings"/>
            <a:buSzPct val="75000"/>
        </a:lvl1pPr>
    </a:lstStyle>"#;
    let result = parse_list_style(xml, None).unwrap().unwrap();
    let lvl = result.levels[0].as_ref().unwrap();
    assert_eq!(lvl.bullet_font.as_deref(), Some("Wingdings"));
    assert!((lvl.bullet_size_pct.unwrap() - 75000.0).abs() < 1e-9);
}

#[test]
fn parses_bullet_color_when_resolver_supplied() {
    let xml = r#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:lvl1pPr><a:buClr><a:srgbClr val="FF0000"/></a:buClr></a:lvl1pPr>
    </a:lstStyle>"#;
    let resolver = test_resolver();
    let result = parse_list_style(xml, Some(&resolver)).unwrap().unwrap();
    let lvl = result.levels[0].as_ref().unwrap();
    assert_eq!(lvl.bullet_color.unwrap().rgb, Rgb::new(0xFF, 0, 0));
}

#[test]
fn skips_bullet_color_when_resolver_absent() {
    // With no resolver, `<a:buClr>` is dropped. Since this lvl1pPr has no
    // other property, the level itself collapses to `None` (matches the TS
    // reference behavior of returning undefined for an empty level).
    let xml = r#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:lvl1pPr><a:buClr><a:srgbClr val="FF0000"/></a:buClr></a:lvl1pPr>
    </a:lstStyle>"#;
    let result = parse_list_style(xml, None).unwrap();
    assert!(
        result.is_none(),
        "buClr is the only property and resolver is missing, so lvl1pPr is empty"
    );
}

#[test]
fn skips_bullet_color_when_resolver_absent_keeps_other_props() {
    let xml = r#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:lvl1pPr algn="l">
            <a:buClr><a:srgbClr val="FF0000"/></a:buClr>
        </a:lvl1pPr>
    </a:lstStyle>"#;
    let result = parse_list_style(xml, None).unwrap().unwrap();
    let lvl = result.levels[0].as_ref().unwrap();
    assert!(lvl.bullet_color.is_none());
    assert!(matches!(lvl.alignment, Some(ParagraphAlignment::L)));
}

#[test]
fn parses_def_r_pr_font_attrs_and_color() {
    let xml = r#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:lvl1pPr>
            <a:defRPr sz="2400" b="1" i="true" u="sng" strike="dblStrike">
                <a:latin typeface="Calibri"/>
                <a:ea typeface="Yu Gothic"/>
                <a:cs typeface="Arial"/>
                <a:solidFill><a:schemeClr val="accent1"/></a:solidFill>
            </a:defRPr>
        </a:lvl1pPr>
    </a:lstStyle>"#;
    let resolver = test_resolver();
    let result = parse_list_style(xml, Some(&resolver)).unwrap().unwrap();
    let lvl = result.levels[0].as_ref().unwrap();
    let def = lvl.default_run_properties.as_ref().unwrap();
    // sz="2400" is in 100ths of a point → 24 pt.
    assert!((def.font_size.unwrap().raw() - 24.0).abs() < 1e-9);
    assert_eq!(def.font_family.as_deref(), Some("Calibri"));
    assert_eq!(def.font_family_ea.as_deref(), Some("Yu Gothic"));
    assert_eq!(def.font_family_cs.as_deref(), Some("Arial"));
    assert_eq!(def.bold, Some(true));
    assert_eq!(def.italic, Some(true));
    assert_eq!(def.underline, Some(true));
    assert_eq!(def.strikethrough, Some(true));
    assert_eq!(def.color.unwrap().rgb, Rgb::new(0x44, 0x72, 0xC4));
}

#[test]
fn def_r_pr_underline_none_means_no_underline() {
    let xml = r#"<a:lstStyle xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:lvl1pPr><a:defRPr u="none"/></a:lvl1pPr>
    </a:lstStyle>"#;
    let result = parse_list_style(xml, None).unwrap().unwrap();
    let def = result.levels[0]
        .as_ref()
        .unwrap()
        .default_run_properties
        .as_ref()
        .unwrap();
    assert_eq!(def.underline, Some(false));
}

// --- resolve_theme_font ---

fn font_scheme() -> FontScheme {
    FontScheme {
        major_font: "Calibri Light".to_owned(),
        minor_font: "Calibri".to_owned(),
        major_font_ea: Some("Yu Gothic Light".to_owned()),
        minor_font_ea: Some("Yu Gothic".to_owned()),
        major_font_cs: Some("Times New Roman".to_owned()),
        minor_font_cs: Some("Arial".to_owned()),
        ..FontScheme::default()
    }
}

#[test]
fn resolve_major_latin_token() {
    assert_eq!(
        resolve_theme_font(Some("+mj-lt"), Some(&font_scheme())).as_deref(),
        Some("Calibri Light"),
    );
}

#[test]
fn resolve_minor_ea_token() {
    assert_eq!(
        resolve_theme_font(Some("+mn-ea"), Some(&font_scheme())).as_deref(),
        Some("Yu Gothic"),
    );
}

#[test]
fn resolve_unknown_token_passes_through() {
    assert_eq!(
        resolve_theme_font(Some("Calibri"), Some(&font_scheme())).as_deref(),
        Some("Calibri"),
    );
}

#[test]
fn resolve_with_no_scheme_returns_input() {
    assert_eq!(
        resolve_theme_font(Some("+mj-lt"), None).as_deref(),
        Some("+mj-lt"),
    );
}
