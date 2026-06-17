//! Ported from.

use slideglance_color::{ColorMap, ColorResolver, ColorScheme, Rgb, SchemeColorKey};
use slideglance_parser::parse_effect_list;

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
    let map = ColorMap {
        bg1: SchemeColorKey::Lt1,
        tx1: SchemeColorKey::Dk1,
        bg2: SchemeColorKey::Lt2,
        tx2: SchemeColorKey::Dk2,
        accent1: SchemeColorKey::Accent1,
        accent2: SchemeColorKey::Accent2,
        accent3: SchemeColorKey::Accent3,
        accent4: SchemeColorKey::Accent4,
        accent5: SchemeColorKey::Accent5,
        accent6: SchemeColorKey::Accent6,
        hlink: SchemeColorKey::Hlink,
        fol_hlink: SchemeColorKey::FolHlink,
    };
    ColorResolver::new(scheme, map)
}

#[test]
fn returns_none_for_empty_effect_list() {
    let result = parse_effect_list(r"<a:effectLst/>", &test_resolver()).unwrap();
    assert!(result.is_none());
}

#[test]
fn parses_outer_shadow_with_attributes_and_color() {
    let xml = r#"
        <a:effectLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:outerShdw blurRad="50800" dist="38100" dir="2700000" algn="tl" rotWithShape="0">
                <a:srgbClr val="000000">
                    <a:alpha val="40000"/>
                </a:srgbClr>
            </a:outerShdw>
        </a:effectLst>
    "#;
    let result = parse_effect_list(xml, &test_resolver()).unwrap().unwrap();
    let outer = result.outer_shadow.unwrap();
    assert_eq!(outer.blur_radius.raw(), 50_800);
    assert_eq!(outer.distance.raw(), 38_100);
    assert!(
        (outer.direction - 45.0).abs() < 1e-9,
        "direction = {}",
        outer.direction
    );
    assert_eq!(outer.alignment, "tl");
    assert!(!outer.rotate_with_shape);
    assert_eq!(outer.color.rgb, Rgb::new(0, 0, 0));
    assert!((outer.color.alpha - 0.4).abs() < 1e-9);
}

#[test]
fn outer_shadow_defaults_match_ts_reference() {
    let xml = r#"
        <a:effectLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:outerShdw>
                <a:srgbClr val="FF0000"/>
            </a:outerShdw>
        </a:effectLst>
    "#;
    let result = parse_effect_list(xml, &test_resolver()).unwrap().unwrap();
    let outer = result.outer_shadow.unwrap();
    assert_eq!(outer.blur_radius.raw(), 0);
    assert_eq!(outer.distance.raw(), 0);
    assert_eq!(outer.direction, 0.0);
    assert_eq!(outer.alignment, "b");
    assert!(outer.rotate_with_shape, "rotWithShape default is true");
}

#[test]
fn parses_inner_shadow() {
    let xml = r#"
        <a:effectLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:innerShdw blurRad="63500" dist="50800" dir="13500000">
                <a:srgbClr val="000000"/>
            </a:innerShdw>
        </a:effectLst>
    "#;
    let result = parse_effect_list(xml, &test_resolver()).unwrap().unwrap();
    let inner = result.inner_shadow.unwrap();
    assert_eq!(inner.blur_radius.raw(), 63_500);
    assert_eq!(inner.distance.raw(), 50_800);
    assert!((inner.direction - 225.0).abs() < 1e-9);
    assert_eq!(inner.color.rgb, Rgb::new(0, 0, 0));
}

#[test]
fn parses_glow() {
    let xml = r#"
        <a:effectLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:glow rad="63500">
                <a:srgbClr val="00FF00"/>
            </a:glow>
        </a:effectLst>
    "#;
    let result = parse_effect_list(xml, &test_resolver()).unwrap().unwrap();
    let glow = result.glow.unwrap();
    assert_eq!(glow.radius.raw(), 63_500);
    assert_eq!(glow.color.rgb, Rgb::new(0, 0xFF, 0));
}

#[test]
fn parses_soft_edge() {
    let xml = r#"
        <a:effectLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:softEdge rad="25400"/>
        </a:effectLst>
    "#;
    let result = parse_effect_list(xml, &test_resolver()).unwrap().unwrap();
    assert_eq!(result.soft_edge.unwrap().radius.raw(), 25_400);
}

#[test]
fn resolves_scheme_color_in_effect() {
    let xml = r#"
        <a:effectLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:outerShdw blurRad="50800">
                <a:schemeClr val="accent1"/>
            </a:outerShdw>
        </a:effectLst>
    "#;
    let result = parse_effect_list(xml, &test_resolver()).unwrap().unwrap();
    let outer = result.outer_shadow.unwrap();
    assert_eq!(outer.color.rgb, Rgb::new(0x44, 0x72, 0xC4));
}

#[test]
fn applies_lum_mod_transform_in_effect() {
    let xml = r#"
        <a:effectLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:glow rad="63500">
                <a:schemeClr val="accent1">
                    <a:lumMod val="50000"/>
                </a:schemeClr>
            </a:glow>
        </a:effectLst>
    "#;
    let result = parse_effect_list(xml, &test_resolver()).unwrap().unwrap();
    let glow = result.glow.unwrap();
    // accent1 (#4472C4) with lumMod 50% — verified to differ from base color.
    assert_ne!(glow.color.rgb, Rgb::new(0x44, 0x72, 0xC4));
}

#[test]
fn returns_none_when_only_unsupported_effects_present() {
    // A reflection block (not implemented) should not surface anywhere; the
    // effectLst is treated as empty.
    let xml = r#"
        <a:effectLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:reflection blurRad="6350" stA="50000" endA="300" dist="5000" dir="5400000"/>
        </a:effectLst>
    "#;
    let result = parse_effect_list(xml, &test_resolver()).unwrap();
    assert!(result.is_none());
}

#[test]
fn collects_multiple_effects_into_one_list() {
    let xml = r#"
        <a:effectLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:outerShdw blurRad="50800"><a:srgbClr val="FF0000"/></a:outerShdw>
            <a:glow rad="63500"><a:srgbClr val="00FF00"/></a:glow>
            <a:softEdge rad="25400"/>
        </a:effectLst>
    "#;
    let result = parse_effect_list(xml, &test_resolver()).unwrap().unwrap();
    assert!(result.outer_shadow.is_some());
    assert!(result.inner_shadow.is_none());
    assert!(result.glow.is_some());
    assert!(result.soft_edge.is_some());
}
