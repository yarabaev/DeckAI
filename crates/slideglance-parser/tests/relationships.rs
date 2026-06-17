//! Ported from.

use slideglance_parser::{build_rels_path, parse_relationships, resolve_relationship_target};

// --- build_rels_path ---

#[test]
fn build_rels_path_for_slide() {
    assert_eq!(
        build_rels_path("ppt/slides/slide1.xml"),
        "ppt/slides/_rels/slide1.xml.rels",
    );
}

#[test]
fn build_rels_path_for_slide_layout() {
    assert_eq!(
        build_rels_path("ppt/slideLayouts/slideLayout1.xml"),
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
    );
}

#[test]
fn build_rels_path_for_slide_master() {
    assert_eq!(
        build_rels_path("ppt/slideMasters/slideMaster1.xml"),
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
    );
}

#[test]
fn build_rels_path_for_presentation() {
    assert_eq!(
        build_rels_path("ppt/presentation.xml"),
        "ppt/_rels/presentation.xml.rels",
    );
}

// --- parse_relationships ---

#[test]
fn parses_valid_relationships_xml() {
    let xml = r#"
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
            <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
        </Relationships>
    "#;

    let result = parse_relationships(xml).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(
        result.get("rId1").unwrap().target,
        "../slideLayouts/slideLayout1.xml",
    );
    assert_eq!(result.get("rId2").unwrap().target, "../theme/theme1.xml");
}

#[test]
fn returns_empty_when_relationships_root_missing() {
    let xml = "<other/>";
    let result = parse_relationships(xml).unwrap();
    assert!(result.is_empty());
}

#[test]
fn skips_entries_missing_required_attributes() {
    let xml = r#"
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId1" Type="http://example.com/type" Target="target.xml"/>
            <Relationship Type="http://example.com/type" Target="target2.xml"/>
            <Relationship Id="rId3" Target="target3.xml"/>
            <Relationship Id="rId4" Type="http://example.com/type"/>
        </Relationships>
    "#;
    let result = parse_relationships(xml).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result.get("rId1").unwrap().target, "target.xml");
}

#[test]
fn captures_target_mode_when_present() {
    let xml = r#"
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId7" Type="http://example.com/hyperlink" Target="https://example.com" TargetMode="External"/>
        </Relationships>
    "#;
    let result = parse_relationships(xml).unwrap();
    assert_eq!(
        result.get("rId7").unwrap().target_mode.as_deref(),
        Some("External"),
    );
}

#[test]
fn target_mode_absent_when_not_set() {
    let xml = r#"
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId1" Type="t" Target="x.xml"/>
        </Relationships>
    "#;
    let result = parse_relationships(xml).unwrap();
    assert!(result.get("rId1").unwrap().target_mode.is_none());
}

// --- resolve_relationship_target ---

#[test]
fn resolves_relative_target_with_parent_walk() {
    assert_eq!(
        resolve_relationship_target("ppt/slides/slide1.xml", "../slideLayouts/slideLayout1.xml"),
        "ppt/slideLayouts/slideLayout1.xml",
    );
}

#[test]
fn resolves_relative_target_in_same_dir() {
    assert_eq!(
        resolve_relationship_target("ppt/slides/slide1.xml", "image1.png"),
        "ppt/slides/image1.png",
    );
}

#[test]
fn resolves_absolute_target_strips_leading_slash() {
    assert_eq!(
        resolve_relationship_target("ppt/slides/slide1.xml", "/ppt/media/image1.png"),
        "ppt/media/image1.png",
    );
}

#[test]
fn resolves_double_parent_walk() {
    assert_eq!(
        resolve_relationship_target("ppt/slides/_rels/slide1.xml.rels", "../../media/image1.png",),
        "ppt/media/image1.png",
    );
}

#[test]
fn resolves_dot_segments() {
    assert_eq!(
        resolve_relationship_target("ppt/slides/slide1.xml", "./image1.png"),
        "ppt/slides/image1.png",
    );
}
