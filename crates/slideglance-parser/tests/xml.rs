//! Ported from.
//!
//! The Rust port uses serde-derived structs as the deserialization target
//! (the spec deserializes into a dynamic JSON tree), so the tests
//! verify the equivalent semantic outcomes — namespace stripping, attribute
//! access, and array vs. single-element handling.

use serde::Deserialize;
use slideglance_parser::{parse_xml, strip_namespaces};

// --- strip_namespaces ---

#[test]
fn strip_removes_element_prefix() {
    let stripped = strip_namespaces(r#"<p:sp xmlns:p="http://x"><p:nvSpPr/></p:sp>"#).unwrap();
    assert!(stripped.contains("<sp>"), "got: {stripped}");
    assert!(stripped.contains("<nvSpPr/>"), "got: {stripped}");
    assert!(!stripped.contains("p:"), "got: {stripped}");
}

#[test]
fn strip_drops_xmlns_declaration_attributes() {
    let stripped =
        strip_namespaces(r#"<root xmlns="http://x" xmlns:a="http://a"><child/></root>"#).unwrap();
    assert!(!stripped.contains("xmlns"), "got: {stripped}");
}

#[test]
fn strip_preserves_non_namespace_attributes() {
    let stripped = strip_namespaces(r#"<shape type="rect" id="1"/>"#).unwrap();
    assert!(stripped.contains(r#"type="rect""#), "got: {stripped}");
    assert!(stripped.contains(r#"id="1""#), "got: {stripped}");
}

#[test]
fn strip_removes_attribute_prefix() {
    let stripped = strip_namespaces(r#"<a:foo xmlns:r="http://r" r:id="rId1"/>"#).unwrap();
    assert!(stripped.contains(r#"id="rId1""#), "got: {stripped}");
    assert!(!stripped.contains("r:id"), "got: {stripped}");
}

// --- parse_xml deserialization ---

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Root {
    child: String,
}

#[test]
fn parses_simple_xml_into_struct() {
    let result: Root = parse_xml(r"<root><child>value</child></root>").unwrap();
    assert_eq!(
        result,
        Root {
            child: "value".into()
        }
    );
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Shape {
    #[serde(rename = "@type")]
    ty: String,
    #[serde(rename = "@id")]
    id: String,
}

#[test]
fn preserves_attributes_after_strip() {
    let result: Shape = parse_xml(r#"<shape type="rect" id="1"/>"#).unwrap();
    assert_eq!(
        result,
        Shape {
            ty: "rect".into(),
            id: "1".into(),
        }
    );
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Sp {
    #[serde(rename = "nvSpPr")]
    nv_sp_pr: NvSpPr,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Default)]
struct NvSpPr {}

#[test]
fn parses_namespaced_input_after_strip() {
    let result: Sp = parse_xml(r#"<p:sp xmlns:p="http://x"><p:nvSpPr/></p:sp>"#).unwrap();
    assert_eq!(
        result,
        Sp {
            nv_sp_pr: NvSpPr {}
        }
    );
}

// --- single-vs-multiple array handling ---

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct SpTree {
    #[serde(default)]
    sp: Vec<SpInner>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct SpInner {
    #[serde(rename = "nvSpPr", default)]
    _nv_sp_pr: Option<NvSpPr>,
}

#[test]
fn vec_target_collects_single_element_as_one_item() {
    let result: SpTree = parse_xml("<spTree><sp><nvSpPr/></sp></spTree>").unwrap();
    assert_eq!(result.sp.len(), 1);
}

#[test]
fn vec_target_collects_multiple_elements() {
    let result: SpTree =
        parse_xml("<spTree><sp><nvSpPr/></sp><sp><nvSpPr/></sp></spTree>").unwrap();
    assert_eq!(result.sp.len(), 2);
}

#[test]
fn vec_target_collects_zero_elements_when_absent() {
    let result: SpTree = parse_xml("<spTree/>").unwrap();
    assert_eq!(result.sp.len(), 0);
}

// --- regression: non-array tag remains a scalar ---

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct SingleHolder {
    single: String,
}

#[test]
fn scalar_target_reads_single_text_value() {
    let result: SingleHolder = parse_xml("<root><single>value</single></root>").unwrap();
    assert_eq!(result.single, "value");
}
