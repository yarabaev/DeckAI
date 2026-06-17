//! `<p:sldLayout>` slide-layout XML parser.
//!
//! Mirrors. As with
//! [`crate::slide_master::parse_slide_master`] the Rust port returns one
//! aggregated [`SlideLayout`] from a single XML deserialization pass.

use std::collections::BTreeMap;

use serde::Deserialize;
use slideglance_color::ColorResolver;
use slideglance_model::{FontScheme, FormatScheme, SlideLayout};

use crate::archive::PptxArchive;
use crate::relationships::{build_rels_path, parse_relationships};
use crate::slide::{build_background, build_sp_tree_elements, RawCSld, SpTreeChild};
use crate::slide_master::collect_placeholder_styles;
use crate::xml::{parse_xml, XmlError};

/// Parses a `<p:sldLayout>` XML body into a [`SlideLayout`] model.
///
/// `layout_path` is the archive-relative path of the layout part (e.g.
/// `"ppt/slideLayouts/slideLayout1.xml"`).
///
/// # Errors
///
/// Returns [`XmlError`] when the input XML or its `.rels` is not
/// well-formed.
pub fn parse_slide_layout(
    xml: &str,
    layout_path: &str,
    archive: &mut PptxArchive,
    resolver: &ColorResolver,
    font_scheme: Option<&FontScheme>,
    fmt_scheme: Option<&FormatScheme>,
) -> Result<SlideLayout, XmlError> {
    let raw: RawSlideLayout = parse_xml(xml)?;
    let rels_path = build_rels_path(layout_path);
    let rels = match archive.xml(&rels_path) {
        Some(rels_xml) => parse_relationships(rels_xml)?,
        None => BTreeMap::new(),
    };

    let name = raw.c_sld.as_ref().and_then(|c| c.name.clone());
    let background = raw
        .c_sld
        .as_ref()
        .and_then(|c| c.bg.as_ref())
        .and_then(|bg| build_background(bg, resolver, &rels, layout_path, archive));

    let empty_children: Vec<SpTreeChild> = Vec::new();
    let sp_tree_children: &[SpTreeChild] = raw
        .c_sld
        .as_ref()
        .and_then(|c| c.sp_tree.as_ref())
        .map_or(empty_children.as_slice(), |t| t.children.as_slice());

    let elements = build_sp_tree_elements(
        sp_tree_children,
        &rels,
        layout_path,
        archive,
        resolver,
        font_scheme,
        fmt_scheme,
        None,
        None,
    );

    let show_master_sp = parse_show_master_sp(raw.show_master_sp.as_deref());
    let placeholder_styles = collect_placeholder_styles(sp_tree_children, resolver);

    Ok(SlideLayout {
        name,
        background,
        elements,
        show_master_sp,
        placeholder_styles,
    })
}

fn parse_show_master_sp(s: Option<&str>) -> bool {
    // OOXML default is `true`; only `0` / `false` flips it off.
    !matches!(s, Some("0" | "false"))
}

#[derive(Debug, Default, Deserialize)]
struct RawSlideLayout {
    #[serde(rename = "@showMasterSp")]
    show_master_sp: Option<String>,
    #[serde(rename = "cSld")]
    c_sld: Option<RawCSld>,
}
