// Tests for <a:rPr @spc> letter-spacing rendering (IP-25, IP-12).
use slideglance_font::{CjkPlatform, ScriptFontContext};
use slideglance_model::RunProperties;
use slideglance_renderer::text::build_style_attrs;
use slideglance_utils::HundredthPt;
use std::collections::BTreeMap;

#[test]
fn spc_200_emits_letter_spacing_in_svg() {
    // spc="200" in OOXML = 200 hundredths-of-a-point = 2pt.
    let props = RunProperties {
        char_spacing: Some(HundredthPt::new(200)),
        ..RunProperties::default()
    };
    let mapping = BTreeMap::new();
    let script_fonts = ScriptFontContext::empty();
    let attrs = build_style_attrs(
        &props,
        1.0,
        None,
        &mapping,
        CjkPlatform::Other,
        &script_fonts,
    );

    assert!(
        attrs.contains("letter-spacing=\"2pt\""),
        "expected letter-spacing=\"2pt\" in SVG attrs, got: {attrs}"
    );
}

#[test]
fn spc_negative_emits_condensed_letter_spacing() {
    // spc="-50" = -0.5pt condensed spacing.
    let props = RunProperties {
        char_spacing: Some(HundredthPt::new(-50)),
        ..RunProperties::default()
    };
    let mapping = BTreeMap::new();
    let script_fonts = ScriptFontContext::empty();
    let attrs = build_style_attrs(
        &props,
        1.0,
        None,
        &mapping,
        CjkPlatform::Other,
        &script_fonts,
    );

    assert!(
        attrs.contains("letter-spacing=\"-0.5pt\""),
        "expected letter-spacing=\"-0.5pt\" in SVG attrs, got: {attrs}"
    );
}

#[test]
fn spc_zero_does_not_emit_letter_spacing() {
    // spc="0" must not emit letter-spacing at all (style.rs skips pt == 0.0).
    let props = RunProperties {
        char_spacing: Some(HundredthPt::new(0)),
        ..RunProperties::default()
    };
    let mapping = BTreeMap::new();
    let script_fonts = ScriptFontContext::empty();
    let attrs = build_style_attrs(
        &props,
        1.0,
        None,
        &mapping,
        CjkPlatform::Other,
        &script_fonts,
    );

    assert!(
        !attrs.contains("letter-spacing"),
        "unexpected letter-spacing in SVG attrs for spc=0: {attrs}"
    );
}

#[test]
fn no_spc_does_not_emit_letter_spacing() {
    // No spc attribute at all must not emit letter-spacing.
    let props = RunProperties::default();
    let mapping = BTreeMap::new();
    let script_fonts = ScriptFontContext::empty();
    let attrs = build_style_attrs(
        &props,
        1.0,
        None,
        &mapping,
        CjkPlatform::Other,
        &script_fonts,
    );

    assert!(
        !attrs.contains("letter-spacing"),
        "unexpected letter-spacing in SVG attrs when spc absent: {attrs}"
    );
}
