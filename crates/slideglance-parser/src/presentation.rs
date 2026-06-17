//! `ppt/presentation.xml` parser.
//!
//! Mirrors.

use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;
use slideglance_model::{
    EmbeddedFont, ModifyVerifier, PresentationInfo, PresentationSection, Protection, SlideSize,
};
use slideglance_utils::Emu;

use crate::text_style::{build_list_style, RawListStyle};
use crate::xml::{parse_xml, XmlError};

const DEFAULT_SLIDE_WIDTH: i64 = 9_144_000;
const DEFAULT_SLIDE_HEIGHT: i64 = 5_143_500;

/// Parses `ppt/presentation.xml` into a [`PresentationInfo`].
///
/// # Errors
///
/// Returns [`XmlError`] when the XML is malformed.
pub fn parse_presentation(xml: &str) -> Result<PresentationInfo, XmlError> {
    let raw: RawRoot = parse_xml(xml)?;

    let slide_size = match raw.sld_sz {
        Some(s) if s.cx.is_some() && s.cy.is_some() => SlideSize {
            width: Emu::new(s.cx.unwrap_or(DEFAULT_SLIDE_WIDTH)),
            height: Emu::new(s.cy.unwrap_or(DEFAULT_SLIDE_HEIGHT)),
        },
        _ => SlideSize {
            width: Emu::new(DEFAULT_SLIDE_WIDTH),
            height: Emu::new(DEFAULT_SLIDE_HEIGHT),
        },
    };

    // The standard parse path is ambiguous for `<p:sldId id="..." r:id="..."/>`
    // because namespace stripping collapses both attributes to "id". A raw
    // regex over the original XML extracts the (numeric, rId) pair safely.
    let (slide_r_ids, slide_id_values) = scan_slide_ids(xml);

    let embedded_fonts = raw
        .embedded_font_lst
        .as_ref()
        .and_then(parse_embedded_fonts);

    let protection = raw.modify_verifier.as_ref().map(parse_protection);

    let sections = parse_section_list(&raw.ext_lst);

    let default_text_style = raw
        .default_text_style
        .as_ref()
        .and_then(|n| build_list_style(n, None));

    Ok(PresentationInfo {
        slide_size,
        slide_r_ids,
        slide_id_values,
        default_text_style,
        embedded_fonts,
        protection,
        sections,
    })
}

fn scan_slide_ids(xml: &str) -> (Vec<String>, Vec<i64>) {
    // The Rust `regex` crate intentionally lacks lookaround, so the TS
    // single-pattern lookahead trick is split into:
    //   1. find the `<p:sldIdLst>...</p:sldIdLst>` block,
    //   2. find each `<p:sldId ... />` element inside it,
    //   3. extract the `id` and `r:id` (or `p:id`) attributes from each tag's
    //      attribute span — order-independent.
    static SLD_ID_LIST: OnceLock<Regex> = OnceLock::new();
    static SLD_ID_TAG: OnceLock<Regex> = OnceLock::new();
    static ID_ATTR: OnceLock<Regex> = OnceLock::new();
    static R_ID_ATTR: OnceLock<Regex> = OnceLock::new();

    let list_re = SLD_ID_LIST
        .get_or_init(|| Regex::new(r"(?s)<p:sldIdLst\b[^>]*>.*?</p:sldIdLst>").expect("valid"));
    let tag_re = SLD_ID_TAG.get_or_init(|| Regex::new(r"<p:sldId\b([^>]*?)/?>").expect("valid"));
    let id_re = ID_ATTR.get_or_init(|| Regex::new(r#"\bid="(\d+)""#).expect("valid"));
    let r_id_re =
        R_ID_ATTR.get_or_init(|| Regex::new(r#"\b(?:r:id|p:id)="([^"]+)""#).expect("valid"));

    let mut r_ids = Vec::new();
    let mut id_values = Vec::new();
    let Some(list) = list_re.find(xml) else {
        return (r_ids, id_values);
    };
    for caps in tag_re.captures_iter(list.as_str()) {
        let attrs = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        let numeric = id_re
            .captures(attrs)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<i64>().ok());
        let r_id = r_id_re
            .captures(attrs)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_owned());
        if let (Some(n), Some(rid)) = (numeric, r_id) {
            r_ids.push(rid);
            id_values.push(n);
        }
    }
    (r_ids, id_values)
}

fn parse_embedded_fonts(node: &RawEmbeddedFontList) -> Option<Vec<EmbeddedFont>> {
    let result: Vec<EmbeddedFont> = node
        .embedded_font
        .iter()
        .filter_map(|entry| {
            let font = entry.font.as_ref()?;
            let mut out = EmbeddedFont {
                typeface: font.typeface.clone().unwrap_or_default(),
                ..EmbeddedFont::default()
            };
            out.panose.clone_from(&font.panose);
            out.pitch_family = font.pitch_family;
            out.charset = font.charset;
            out.regular_r_id = entry.regular.as_ref().and_then(rid_value);
            out.bold_r_id = entry.bold.as_ref().and_then(rid_value);
            out.italic_r_id = entry.italic.as_ref().and_then(rid_value);
            out.bold_italic_r_id = entry.bold_italic.as_ref().and_then(rid_value);
            out.regular_subsetted = entry.regular.as_ref().is_some_and(rid_subsetted);
            out.bold_subsetted = entry.bold.as_ref().is_some_and(rid_subsetted);
            out.italic_subsetted = entry.italic.as_ref().is_some_and(rid_subsetted);
            out.bold_italic_subsetted = entry.bold_italic.as_ref().is_some_and(rid_subsetted);
            Some(out)
        })
        .collect();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn rid_value(node: &RawRid) -> Option<String> {
    // After namespace stripping the attribute is just `id`; before stripping
    // it was `r:id`. Either way it ends up in `node.id`.
    node.id.clone()
}

fn rid_subsetted(node: &RawRid) -> bool {
    // `@subsetted` is "1" when the face was subset-embedded (OOXML §14.2.5).
    node.subsetted.as_deref() == Some("1")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_presentation_xml(embedded_fonts_block: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:sldSz cx="9144000" cy="6858000"/>
  <p:notesSz cx="6858000" cy="9144000"/>
  {embedded_fonts_block}
</p:presentation>"#
        )
    }

    /// B.10: @subsetted="1" must set the corresponding *_subsetted flag to true.
    #[test]
    fn parse_embedded_fonts_reads_subsetted_flag() {
        let xml = minimal_presentation_xml(
            r#"<p:embeddedFontLst>
              <p:embeddedFont>
                <p:font typeface="TestFont"/>
                <p:regular r:id="rId1" subsetted="1"/>
                <p:bold r:id="rId2" subsetted="0"/>
                <p:italic r:id="rId3"/>
              </p:embeddedFont>
            </p:embeddedFontLst>"#,
        );
        let info = parse_presentation(&xml).expect("parse must succeed");
        let fonts = info.embedded_fonts.expect("embedded_fonts must be Some");
        assert_eq!(fonts.len(), 1);
        let f = &fonts[0];
        assert_eq!(f.typeface, "TestFont");
        assert!(f.regular_subsetted, "regular subsetted=1 must be true");
        assert!(!f.bold_subsetted, "bold subsetted=0 must be false");
        assert!(
            !f.italic_subsetted,
            "italic with no subsetted attr must be false"
        );
        assert!(!f.bold_italic_subsetted, "missing variant must be false");
    }

    /// B.10: all four faces without @subsetted must all be false.
    #[test]
    fn parse_embedded_fonts_no_subsetted_defaults_to_false() {
        let xml = minimal_presentation_xml(
            r#"<p:embeddedFontLst>
              <p:embeddedFont>
                <p:font typeface="AnotherFont"/>
                <p:regular r:id="rId1"/>
                <p:bold r:id="rId2"/>
              </p:embeddedFont>
            </p:embeddedFontLst>"#,
        );
        let info = parse_presentation(&xml).expect("parse must succeed");
        let fonts = info.embedded_fonts.expect("embedded_fonts must be Some");
        let f = &fonts[0];
        assert!(!f.regular_subsetted);
        assert!(!f.bold_subsetted);
        assert!(!f.italic_subsetted);
        assert!(!f.bold_italic_subsetted);
    }

    /// B.10: no <p:embeddedFontLst> element → `embedded_fonts` is None.
    #[test]
    fn parse_embedded_fonts_absent_returns_none() {
        let xml = minimal_presentation_xml("");
        let info = parse_presentation(&xml).expect("parse must succeed");
        assert!(info.embedded_fonts.is_none());
    }
}

fn parse_protection(node: &RawModifyVerifier) -> Protection {
    Protection {
        modify_verifier: Some(ModifyVerifier {
            algorithm_name: node.algorithm_name.clone(),
            hash_value: node.hash_value.clone(),
            salt_value: node.salt_value.clone(),
            spin_count: node.spin_count,
        }),
    }
}

fn parse_section_list(ext_lst: &[RawExtList]) -> Option<Vec<PresentationSection>> {
    for ext_list in ext_lst {
        for ext in &ext_list.ext {
            let Some(section_lst) = ext.section_lst.as_ref() else {
                continue;
            };
            let mut out = Vec::new();
            for sec in &section_lst.section {
                let slide_ids = sec
                    .sld_id_lst
                    .as_ref()
                    .map(|lst| lst.sld_id.iter().filter_map(|s| s.id).collect::<Vec<_>>())
                    .unwrap_or_default();
                out.push(PresentationSection {
                    name: sec.name.clone().unwrap_or_default(),
                    slide_ids,
                });
            }
            if !out.is_empty() {
                return Some(out);
            }
        }
    }
    None
}

// --- raw XML shapes (post namespace strip) ---

#[derive(Debug, Default, Deserialize)]
struct RawRoot {
    #[serde(rename = "sldSz")]
    sld_sz: Option<RawSldSz>,
    #[serde(rename = "embeddedFontLst")]
    embedded_font_lst: Option<RawEmbeddedFontList>,
    #[serde(rename = "modifyVerifier")]
    modify_verifier: Option<RawModifyVerifier>,
    #[serde(rename = "defaultTextStyle")]
    default_text_style: Option<RawListStyle>,
    // `<p:extLst>` may appear multiple times in malformed/variant inputs;
    // serde collects every sibling into the Vec automatically.
    #[serde(default, rename = "extLst")]
    ext_lst: Vec<RawExtList>,
}

#[derive(Debug, Deserialize)]
struct RawSldSz {
    #[serde(rename = "@cx")]
    cx: Option<i64>,
    #[serde(rename = "@cy")]
    cy: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RawEmbeddedFontList {
    #[serde(default, rename = "embeddedFont")]
    embedded_font: Vec<RawEmbeddedFontEntry>,
}

#[derive(Debug, Deserialize)]
struct RawEmbeddedFontEntry {
    font: Option<RawFont>,
    regular: Option<RawRid>,
    bold: Option<RawRid>,
    italic: Option<RawRid>,
    #[serde(rename = "boldItalic")]
    bold_italic: Option<RawRid>,
}

#[derive(Debug, Deserialize)]
struct RawFont {
    #[serde(rename = "@typeface")]
    typeface: Option<String>,
    #[serde(rename = "@panose")]
    panose: Option<String>,
    #[serde(rename = "@pitchFamily")]
    pitch_family: Option<i32>,
    #[serde(rename = "@charset")]
    charset: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct RawRid {
    #[serde(rename = "@id")]
    id: Option<String>,
    /// `@subsetted="1"` — face was subset-embedded in the PPTX archive.
    #[serde(rename = "@subsetted")]
    subsetted: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawModifyVerifier {
    #[serde(rename = "@algorithmName")]
    algorithm_name: Option<String>,
    #[serde(rename = "@hashValue")]
    hash_value: Option<String>,
    #[serde(rename = "@saltValue")]
    salt_value: Option<String>,
    #[serde(rename = "@spinCount")]
    spin_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RawExtList {
    #[serde(default)]
    ext: Vec<RawExt>,
}

#[derive(Debug, Deserialize)]
struct RawExt {
    #[serde(rename = "sectionLst")]
    section_lst: Option<RawSectionList>,
}

#[derive(Debug, Deserialize)]
struct RawSectionList {
    #[serde(default)]
    section: Vec<RawSection>,
}

#[derive(Debug, Deserialize)]
struct RawSection {
    #[serde(rename = "@name")]
    name: Option<String>,
    #[serde(rename = "sldIdLst")]
    sld_id_lst: Option<RawSectionSldIdLst>,
}

#[derive(Debug, Deserialize)]
struct RawSectionSldIdLst {
    #[serde(default, rename = "sldId")]
    sld_id: Vec<RawSectionSldId>,
}

#[derive(Debug, Deserialize)]
struct RawSectionSldId {
    #[serde(rename = "@id")]
    id: Option<i64>,
}
