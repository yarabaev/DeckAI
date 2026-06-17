//! Ported from
//! (representative cases — full TS file has 30+ assertions).

use slideglance_parser::parse_presentation;
use slideglance_utils::Emu;

#[test]
fn returns_default_slide_size_when_root_missing() {
    let info = parse_presentation("<other/>").unwrap();
    assert_eq!(info.slide_size.width, Emu::new(9_144_000));
    assert_eq!(info.slide_size.height, Emu::new(5_143_500));
    assert!(info.slide_r_ids.is_empty());
}

#[test]
fn parses_explicit_slide_size() {
    let xml = r#"
        <p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
            <p:sldSz cx="12192000" cy="6858000"/>
        </p:presentation>
    "#;
    let info = parse_presentation(xml).unwrap();
    assert_eq!(info.slide_size.width, Emu::new(12_192_000));
    assert_eq!(info.slide_size.height, Emu::new(6_858_000));
}

#[test]
fn falls_back_to_default_when_sld_sz_attrs_missing() {
    let xml = r#"
        <p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
            <p:sldSz/>
        </p:presentation>
    "#;
    let info = parse_presentation(xml).unwrap();
    assert_eq!(info.slide_size.width, Emu::new(9_144_000));
    assert_eq!(info.slide_size.height, Emu::new(5_143_500));
}

#[test]
fn extracts_slide_id_list_via_raw_scan() {
    let xml = r#"
        <p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                        xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
            <p:sldIdLst>
                <p:sldId id="256" r:id="rId2"/>
                <p:sldId id="257" r:id="rId3"/>
                <p:sldId id="258" r:id="rId4"/>
            </p:sldIdLst>
        </p:presentation>
    "#;
    let info = parse_presentation(xml).unwrap();
    assert_eq!(info.slide_r_ids, vec!["rId2", "rId3", "rId4"]);
    assert_eq!(info.slide_id_values, vec![256, 257, 258]);
}

#[test]
fn handles_attribute_order_swapped() {
    let xml = r#"
        <p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                        xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
            <p:sldIdLst>
                <p:sldId r:id="rIdX" id="999"/>
            </p:sldIdLst>
        </p:presentation>
    "#;
    let info = parse_presentation(xml).unwrap();
    assert_eq!(info.slide_r_ids, vec!["rIdX"]);
    assert_eq!(info.slide_id_values, vec![999]);
}

#[test]
fn parses_modify_verifier_protection() {
    let xml = r#"
        <p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
            <p:modifyVerifier algorithmName="SHA-512" hashValue="hashAbc==" saltValue="saltXyz==" spinCount="100000"/>
        </p:presentation>
    "#;
    let info = parse_presentation(xml).unwrap();
    let mv = info.protection.unwrap().modify_verifier.unwrap();
    assert_eq!(mv.algorithm_name.as_deref(), Some("SHA-512"));
    assert_eq!(mv.hash_value.as_deref(), Some("hashAbc=="));
    assert_eq!(mv.salt_value.as_deref(), Some("saltXyz=="));
    assert_eq!(mv.spin_count, Some(100_000));
}

#[test]
fn parses_default_text_style_via_text_style_parser() {
    use slideglance_model::ParagraphAlignment;
    let xml = r#"
        <p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                        xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <p:defaultTextStyle>
                <a:defPPr algn="l"/>
                <a:lvl1pPr algn="ctr"/>
            </p:defaultTextStyle>
        </p:presentation>
    "#;
    let info = parse_presentation(xml).unwrap();
    let style = info.default_text_style.unwrap();
    assert!(matches!(
        style.default_paragraph.unwrap().alignment,
        Some(ParagraphAlignment::L)
    ));
    assert!(matches!(
        style.levels[0].as_ref().unwrap().alignment,
        Some(ParagraphAlignment::Ctr)
    ));
}

#[test]
fn parses_section_list() {
    let xml = r#"
        <p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                        xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main">
            <p:extLst>
                <p:ext uri="{521415D9-36F7-43E2-AB2F-B90AF26B5E84}">
                    <p14:sectionLst>
                        <p14:section name="Default Section" id="{ABC}">
                            <p14:sldIdLst>
                                <p14:sldId id="256"/>
                                <p14:sldId id="257"/>
                            </p14:sldIdLst>
                        </p14:section>
                    </p14:sectionLst>
                </p:ext>
            </p:extLst>
        </p:presentation>
    "#;
    let info = parse_presentation(xml).unwrap();
    let sections = info.sections.unwrap();
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].name, "Default Section");
    assert_eq!(sections[0].slide_ids, vec![256, 257]);
}
