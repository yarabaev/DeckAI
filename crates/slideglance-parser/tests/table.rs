//! Smoke tests for `parse_table`.

use slideglance_color::{ColorMap, ColorResolver, ColorScheme, Rgb};
use slideglance_model::Fill;
use slideglance_parser::parse_table;

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
        hlink: Rgb::new(0, 0, 0),
        fol_hlink: Rgb::new(0, 0, 0),
    };
    ColorResolver::new(scheme, ColorMap::default())
}

#[test]
fn returns_none_when_grid_is_empty() {
    let xml = r#"<a:tbl xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:tblGrid/></a:tbl>"#;
    let result = parse_table(xml, &test_resolver(), None, None).unwrap();
    assert!(result.is_none());
}

#[test]
fn parses_columns_and_rows() {
    let xml = r#"<a:tbl xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:tblGrid>
            <a:gridCol w="100000"/>
            <a:gridCol w="200000"/>
        </a:tblGrid>
        <a:tr h="50000">
            <a:tc><a:txBody><a:p><a:r><a:t>A1</a:t></a:r></a:p></a:txBody></a:tc>
            <a:tc><a:txBody><a:p><a:r><a:t>A2</a:t></a:r></a:p></a:txBody></a:tc>
        </a:tr>
        <a:tr h="40000">
            <a:tc><a:txBody><a:p><a:r><a:t>B1</a:t></a:r></a:p></a:txBody></a:tc>
            <a:tc><a:txBody><a:p><a:r><a:t>B2</a:t></a:r></a:p></a:txBody></a:tc>
        </a:tr>
    </a:tbl>"#;
    let table = parse_table(xml, &test_resolver(), None, None)
        .unwrap()
        .unwrap();
    assert_eq!(table.columns.len(), 2);
    assert_eq!(table.columns[0].width.raw(), 100_000);
    assert_eq!(table.columns[1].width.raw(), 200_000);
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0].height.raw(), 50_000);
    assert_eq!(table.rows[0].cells.len(), 2);
    let body = table.rows[0].cells[0].text_body.as_ref().unwrap();
    assert_eq!(body.paragraphs[0].runs[0].text, "A1");
}

#[test]
fn parses_grid_and_row_span() {
    let xml = r#"<a:tbl xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:tblGrid>
            <a:gridCol w="100000"/>
            <a:gridCol w="100000"/>
        </a:tblGrid>
        <a:tr>
            <a:tc gridSpan="2"><a:txBody><a:p/></a:txBody></a:tc>
            <a:tc rowSpan="3" hMerge="1" vMerge="true"><a:txBody><a:p/></a:txBody></a:tc>
        </a:tr>
    </a:tbl>"#;
    let table = parse_table(xml, &test_resolver(), None, None)
        .unwrap()
        .unwrap();
    let cells = &table.rows[0].cells;
    assert_eq!(cells[0].grid_span, 2);
    assert!(cells[1].h_merge);
    assert!(cells[1].v_merge);
    assert_eq!(cells[1].row_span, 3);
}

#[test]
fn parses_cell_fill_and_borders() {
    let xml = r#"<a:tbl xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:tblGrid><a:gridCol w="100000"/></a:tblGrid>
        <a:tr>
            <a:tc>
                <a:txBody><a:p/></a:txBody>
                <a:tcPr>
                    <a:lnT w="6350"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:lnT>
                    <a:lnB w="6350"/>
                    <a:solidFill><a:srgbClr val="00FF00"/></a:solidFill>
                </a:tcPr>
            </a:tc>
        </a:tr>
    </a:tbl>"#;
    let table = parse_table(xml, &test_resolver(), None, None)
        .unwrap()
        .unwrap();
    let cell = &table.rows[0].cells[0];
    let borders = cell.borders.as_ref().unwrap();
    assert_eq!(borders.top.as_ref().unwrap().width.raw(), 6_350);
    // ln_b had no fill — defaults to opaque black.
    let bottom_fill = borders.bottom.as_ref().unwrap().fill.as_ref().unwrap();
    match bottom_fill {
        slideglance_model::OutlineFill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0, 0, 0)),
        slideglance_model::OutlineFill::Gradient(_) => panic!("expected solid"),
    }
    match cell.fill.as_ref().unwrap() {
        Fill::Solid(s) => assert_eq!(s.color.rgb, Rgb::new(0, 0xFF, 0)),
        other => panic!("expected solid green fill, got {other:?}"),
    }
}

#[test]
fn parses_table_style_id_and_options() {
    let xml = r#"<a:tbl xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:tblPr firstRow="0" lastRow="1" bandRow="0">
            <a:tableStyleId>{ABCD-1234}</a:tableStyleId>
        </a:tblPr>
        <a:tblGrid><a:gridCol w="100000"/></a:tblGrid>
    </a:tbl>"#;
    let table = parse_table(xml, &test_resolver(), None, None)
        .unwrap()
        .unwrap();
    assert_eq!(table.table_style_id.as_deref(), Some("{ABCD-1234}"));
    let opts = table.table_style_options.unwrap();
    assert_eq!(opts.first_row, Some(false));
    assert_eq!(opts.last_row, Some(true));
    assert_eq!(opts.first_col, Some(false));
    assert_eq!(opts.band_row, Some(false));
}

#[test]
fn cell_anchor_overrides_body_anchor() {
    use slideglance_model::VerticalAnchor;
    let xml = r#"<a:tbl xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:tblGrid><a:gridCol w="100000"/></a:tblGrid>
        <a:tr>
            <a:tc>
                <a:txBody><a:bodyPr anchor="t"/><a:p><a:r><a:t>x</a:t></a:r></a:p></a:txBody>
                <a:tcPr anchor="ctr"/>
            </a:tc>
        </a:tr>
    </a:tbl>"#;
    let table = parse_table(xml, &test_resolver(), None, None)
        .unwrap()
        .unwrap();
    let body = table.rows[0].cells[0].text_body.as_ref().unwrap();
    assert!(matches!(body.body_properties.anchor, VerticalAnchor::Ctr));
}
