//! Smoke tests for the extracted text body parser. Full coverage will land
//! alongside slide-parser when slide-level shapes can populate the surrounding
//! context (placeholder inheritance, etc.).

use slideglance_color::{ColorMap, ColorResolver, ColorScheme, Rgb};
use slideglance_model::{ParagraphAlignment, SpacingValue, VerticalAnchor};
use slideglance_parser::parse_text_body;

fn test_resolver() -> ColorResolver {
    let scheme = ColorScheme {
        dk1: Rgb::new(0, 0, 0),
        lt1: Rgb::new(0xFF, 0xFF, 0xFF),
        dk2: Rgb::new(0, 0, 0),
        lt2: Rgb::new(0xFF, 0xFF, 0xFF),
        accent1: Rgb::new(0x44, 0x72, 0xC4),
        accent2: Rgb::new(0, 0, 0),
        accent3: Rgb::new(0, 0, 0),
        accent4: Rgb::new(0, 0, 0),
        accent5: Rgb::new(0, 0, 0),
        accent6: Rgb::new(0, 0, 0),
        hlink: Rgb::new(0x05, 0x63, 0xC1),
        fol_hlink: Rgb::new(0, 0, 0),
    };
    ColorResolver::new(scheme, ColorMap::default())
}

#[test]
fn returns_none_for_empty_paragraph_list() {
    let xml = r#"<p:txBody xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"/>"#;
    let result = parse_text_body(xml, &test_resolver(), None, None, None).unwrap();
    assert!(result.is_none());
}

#[test]
fn parses_single_paragraph_with_run() {
    let xml = r#"<p:txBody xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:bodyPr anchor="ctr" wrap="square"/>
        <a:p><a:r><a:t>Hello world</a:t></a:r></a:p>
    </p:txBody>"#;
    let result = parse_text_body(xml, &test_resolver(), None, None, None)
        .unwrap()
        .unwrap();
    assert_eq!(result.paragraphs.len(), 1);
    let para = &result.paragraphs[0];
    assert_eq!(para.runs.len(), 1);
    assert_eq!(para.runs[0].text, "Hello world");
    assert!(matches!(result.body_properties.anchor, VerticalAnchor::Ctr));
}

#[test]
fn parses_run_properties_attributes() {
    let xml = r#"<p:txBody xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:p>
            <a:r>
                <a:rPr sz="2400" b="1" i="true" u="sng" strike="dblStrike" baseline="30000">
                    <a:latin typeface="Arial"/>
                    <a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>
                </a:rPr>
                <a:t>Styled</a:t>
            </a:r>
        </a:p>
    </p:txBody>"#;
    let result = parse_text_body(xml, &test_resolver(), None, None, None)
        .unwrap()
        .unwrap();
    let run = &result.paragraphs[0].runs[0];
    assert!((run.properties.font_size.unwrap().raw() - 24.0).abs() < 1e-9);
    assert_eq!(run.properties.font_family.as_deref(), Some("Arial"));
    assert!(run.properties.bold);
    assert!(run.properties.italic);
    assert!(run.properties.underline);
    assert!(run.properties.strikethrough);
    assert_eq!(run.properties.color.unwrap().rgb, Rgb::new(0xFF, 0, 0));
    assert!((run.properties.baseline - 30.0).abs() < 1e-9);
}

#[test]
fn paragraph_alignment_falls_back_to_lst_style_level() {
    use slideglance_model::{DefaultParagraphLevelProperties, DefaultTextStyle};
    let lst = DefaultTextStyle {
        default_paragraph: None,
        levels: vec![Some(DefaultParagraphLevelProperties {
            alignment: Some(ParagraphAlignment::Ctr),
            ..DefaultParagraphLevelProperties::default()
        })],
    };
    let xml = r#"<p:txBody xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:p><a:r><a:t>x</a:t></a:r></a:p>
    </p:txBody>"#;
    let result = parse_text_body(xml, &test_resolver(), None, None, Some(&lst))
        .unwrap()
        .unwrap();
    assert!(matches!(
        result.paragraphs[0].properties.alignment,
        Some(ParagraphAlignment::Ctr)
    ));
}

#[test]
fn line_break_run_inserts_newline() {
    let xml = r#"<p:txBody xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:p>
            <a:r><a:t>A</a:t></a:r>
            <a:br/>
            <a:r><a:t>B</a:t></a:r>
        </a:p>
    </p:txBody>"#;
    let result = parse_text_body(xml, &test_resolver(), None, None, None)
        .unwrap()
        .unwrap();
    let runs = &result.paragraphs[0].runs;
    assert_eq!(runs.len(), 3);
    assert_eq!(runs[0].text, "A");
    assert_eq!(runs[1].text, "\n");
    assert_eq!(runs[2].text, "B");
}

#[test]
fn fld_run_carries_field_type() {
    let xml = r#"<p:txBody xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:p>
            <a:fld id="{X}" type="datetime1"><a:t>2026-04-28</a:t></a:fld>
        </a:p>
    </p:txBody>"#;
    let result = parse_text_body(xml, &test_resolver(), None, None, None)
        .unwrap()
        .unwrap();
    let run = &result.paragraphs[0].runs[0];
    assert_eq!(run.text, "2026-04-28");
    assert_eq!(run.field_type.as_deref(), Some("datetime1"));
}

#[test]
fn body_pr_default_margins() {
    let xml = r#"<p:txBody xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:bodyPr/>
        <a:p><a:r><a:t>x</a:t></a:r></a:p>
    </p:txBody>"#;
    let result = parse_text_body(xml, &test_resolver(), None, None, None)
        .unwrap()
        .unwrap();
    assert_eq!(result.body_properties.margin_left.raw(), 91_440);
    assert_eq!(result.body_properties.margin_right.raw(), 91_440);
    assert_eq!(result.body_properties.margin_top.raw(), 45_720);
    assert_eq!(result.body_properties.margin_bottom.raw(), 45_720);
}

#[test]
fn space_before_pts() {
    let xml = r#"<p:txBody xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:p>
            <a:pPr><a:spcBef><a:spcPts val="600"/></a:spcBef></a:pPr>
            <a:r><a:t>x</a:t></a:r>
        </a:p>
    </p:txBody>"#;
    let result = parse_text_body(xml, &test_resolver(), None, None, None)
        .unwrap()
        .unwrap();
    match result.paragraphs[0].properties.space_before {
        Some(SpacingValue::Pts { value }) => assert_eq!(value.raw(), 600),
        Some(SpacingValue::Pct { .. }) => panic!("expected pts spacing"),
        None => panic!("expected explicit spacing"),
    }
}
