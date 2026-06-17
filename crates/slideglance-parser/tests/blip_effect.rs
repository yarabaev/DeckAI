//! Ported from.

use slideglance_color::{ColorMap, ColorResolver, ColorScheme, Rgb, SchemeColorKey};
use slideglance_parser::parse_blip_effects;

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
fn returns_none_for_empty_blip() {
    let result = parse_blip_effects("<a:blip/>", &test_resolver()).unwrap();
    assert!(result.is_none());
}

#[test]
fn parses_grayscale_marker() {
    let xml = r#"
        <a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:grayscl/>
        </a:blip>
    "#;
    let result = parse_blip_effects(xml, &test_resolver()).unwrap().unwrap();
    assert!(result.grayscale);
}

#[test]
fn parses_bi_level_with_explicit_threshold() {
    let xml = r#"
        <a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:biLevel thresh="40000"/>
        </a:blip>
    "#;
    let result = parse_blip_effects(xml, &test_resolver()).unwrap().unwrap();
    let bi = result.bi_level.unwrap();
    assert!((bi.threshold - 0.4).abs() < 1e-9);
}

#[test]
fn bi_level_default_threshold_is_50pct() {
    let xml = r#"
        <a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:biLevel/>
        </a:blip>
    "#;
    let result = parse_blip_effects(xml, &test_resolver()).unwrap().unwrap();
    assert!((result.bi_level.unwrap().threshold - 0.5).abs() < 1e-9);
}

#[test]
fn parses_blur_with_grow_default_true() {
    let xml = r#"
        <a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:blur rad="50800"/>
        </a:blip>
    "#;
    let result = parse_blip_effects(xml, &test_resolver()).unwrap().unwrap();
    let blur = result.blur.unwrap();
    assert_eq!(blur.radius.raw(), 50_800);
    assert!(blur.grow, "grow defaults to true when attr absent");
}

#[test]
fn parses_blur_with_grow_zero() {
    let xml = r#"
        <a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:blur rad="25400" grow="0"/>
        </a:blip>
    "#;
    let result = parse_blip_effects(xml, &test_resolver()).unwrap().unwrap();
    assert!(!result.blur.unwrap().grow);
}

#[test]
fn parses_lum_brightness_and_contrast() {
    let xml = r#"
        <a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:lum bright="20000" contrast="-10000"/>
        </a:blip>
    "#;
    let result = parse_blip_effects(xml, &test_resolver()).unwrap().unwrap();
    let lum = result.lum.unwrap();
    assert!((lum.brightness - 0.2).abs() < 1e-9);
    assert!((lum.contrast + 0.1).abs() < 1e-9);
}

#[test]
fn parses_duotone_with_two_srgb_colors() {
    let xml = r#"
        <a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:duotone>
                <a:srgbClr val="000000"/>
                <a:srgbClr val="FFFFFF"/>
            </a:duotone>
        </a:blip>
    "#;
    let result = parse_blip_effects(xml, &test_resolver()).unwrap().unwrap();
    let duo = result.duotone.unwrap();
    assert_eq!(duo.color1.rgb, Rgb::new(0, 0, 0));
    assert_eq!(duo.color2.rgb, Rgb::new(0xFF, 0xFF, 0xFF));
}

#[test]
fn parses_duotone_with_preset_colors() {
    let xml = r#"
        <a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:duotone>
                <a:prstClr val="red"/>
                <a:prstClr val="blue"/>
            </a:duotone>
        </a:blip>
    "#;
    let result = parse_blip_effects(xml, &test_resolver()).unwrap().unwrap();
    let duo = result.duotone.unwrap();
    assert_eq!(duo.color1.rgb, Rgb::new(0xFF, 0, 0));
    assert_eq!(duo.color2.rgb, Rgb::new(0, 0, 0xFF));
}

#[test]
fn parses_duotone_with_scheme_color() {
    let xml = r#"
        <a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:duotone>
                <a:schemeClr val="accent1"/>
                <a:schemeClr val="accent2"/>
            </a:duotone>
        </a:blip>
    "#;
    let result = parse_blip_effects(xml, &test_resolver()).unwrap().unwrap();
    let duo = result.duotone.unwrap();
    assert_eq!(duo.color1.rgb, Rgb::new(0x44, 0x72, 0xC4));
    assert_eq!(duo.color2.rgb, Rgb::new(0xED, 0x7D, 0x31));
}

#[test]
fn duotone_with_only_one_color_is_ignored() {
    let xml = r#"
        <a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:duotone>
                <a:srgbClr val="FF0000"/>
            </a:duotone>
        </a:blip>
    "#;
    // A 1-color duotone produces no effect, and since it is the only effect
    // present the whole BlipEffects collapses to None (matches TS behavior).
    let result = parse_blip_effects(xml, &test_resolver()).unwrap();
    assert!(
        result.is_none(),
        "blip with only invalid duotone should yield None"
    );
}

#[test]
fn parses_clr_change() {
    let xml = r#"
        <a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:clrChange>
                <a:clrFrom><a:srgbClr val="FF0000"/></a:clrFrom>
                <a:clrTo><a:srgbClr val="00FF00"/></a:clrTo>
            </a:clrChange>
        </a:blip>
    "#;
    let result = parse_blip_effects(xml, &test_resolver()).unwrap().unwrap();
    let cc = result.clr_change.unwrap();
    assert_eq!(cc.clr_from.rgb, Rgb::new(0xFF, 0, 0));
    assert_eq!(cc.clr_to.rgb, Rgb::new(0, 0xFF, 0));
}

#[test]
fn collects_multiple_effects_simultaneously() {
    let xml = r#"
        <a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <a:grayscl/>
            <a:blur rad="25400"/>
            <a:lum bright="20000" contrast="0"/>
        </a:blip>
    "#;
    let result = parse_blip_effects(xml, &test_resolver()).unwrap().unwrap();
    assert!(result.grayscale);
    assert!(result.blur.is_some());
    assert!(result.lum.is_some());
    assert!(result.duotone.is_none());
}
