//! `<p:sld>` slide-XML parser — turns a slide part body into a [`Slide`]
//! together with all of its shapes, pictures, connectors, groups, charts, and
//! tables.
//!
//! Mirrors (and its
//! `parseShapeTree` / `parseShape` / `parseImage` / `parseConnector` /
//! `parseGroup` / `parseGraphicFrame` / `parseSmartArt` helpers). Geometry,
//! text body, fill, outline, effect, blip-effect, table, chart, and style
//! reference parsing are delegated to dedicated modules and called via their
//! `build_*` helpers, so this module only handles slide-level structural
//! orchestration.

use std::collections::BTreeMap;

use serde::Deserialize;
use slideglance_color::ColorResolver;
use slideglance_model::{
    Background, FontScheme, FormatScheme, Geometry, PlaceholderStyleInfo, PresetGeometry, Slide,
    SlideElement, SlideHeaderFooter,
};

use crate::archive::PptxArchive;
use crate::fill::{build_fill, FillParseContext, RawFillContainer};
use crate::relationships::{
    build_rels_path, parse_relationships, resolve_relationship_target, Relationship,
};
use crate::xml::{parse_xml, XmlError};

mod connector;
mod graphic;
mod group;
mod image;
mod shape;

// Re-export raw XML types whose ownership moved into the per-element
// sub-modules but which sibling parser modules
// (slide_master.rs / slide_layout.rs) still need to import.
pub(crate) use shape::{RawShapeSpPr, RawSp};

// Sub-module raw types that mod.rs's own SpTreeChild enum / parse
// helpers need to reference. Kept module-local so the whole tree of
// raw types is visible from one place.
use connector::RawCxnSp;
use graphic::RawGraphicFrame;
use group::{RawGrpSp, RawGrpSpPr, RawNvGrpSpPr};
use image::RawPic;
use shape::RawCNvPr;

pub(super) const FRACTION_DIVISOR: f64 = 100_000.0;
pub(super) const TILE_DEFAULT_SCALE: i64 = 100_000;

/// Parses a `<p:sld>` slide XML body into a [`Slide`] model.
///
/// `slide_path` is the archive-relative path of the slide part (e.g.
/// `"ppt/slides/slide1.xml"`); it is used both to locate the part-level
/// `.rels` file and to resolve relative relationship targets when the slide
/// references media or charts.
///
/// `placeholder_styles` carries the master+layout placeholder inheritance map
/// (resolved by the higher-level slide-master/slide-layout parsers). When the
/// slide has placeholder shapes whose `<p:spPr>` block omits `xfrm` or
/// geometry, the parser falls back to the matching entry in this list.
///
/// # Errors
///
/// Returns [`XmlError`] when either the slide XML or its `.rels` file is not
/// well-formed XML.
#[allow(clippy::too_many_arguments)]
pub fn parse_slide(
    slide_xml: &str,
    slide_path: &str,
    slide_number: u32,
    archive: &mut PptxArchive,
    resolver: &ColorResolver,
    font_scheme: Option<&FontScheme>,
    fmt_scheme: Option<&FormatScheme>,
    placeholder_styles: &[PlaceholderStyleInfo],
) -> Result<Slide, XmlError> {
    let raw: RawSlide = parse_xml(slide_xml)?;
    let rels_path = build_rels_path(slide_path);
    let rels = match archive.xml(&rels_path) {
        Some(rels_xml) => parse_relationships(rels_xml)?,
        None => BTreeMap::new(),
    };

    let empty_children: Vec<SpTreeChild> = Vec::new();
    let children_slice: &[SpTreeChild] = raw
        .c_sld
        .as_ref()
        .and_then(|c| c.sp_tree.as_ref())
        .map_or(empty_children.as_slice(), |t| t.children.as_slice());
    let elements = build_sp_tree_elements(
        children_slice,
        &rels,
        slide_path,
        archive,
        resolver,
        font_scheme,
        fmt_scheme,
        Some(placeholder_styles),
        None,
    );

    let background = raw
        .c_sld
        .as_ref()
        .and_then(|c| c.bg.as_ref())
        .and_then(|bg| build_background(bg, resolver, &rels, slide_path, archive));

    let show_master_sp = parse_optional_truthy(raw.show_master_sp.as_deref()).unwrap_or(true);
    let header_footer = raw.hf.as_ref().map(build_slide_header_footer);

    Ok(Slide {
        slide_number,
        background,
        elements,
        show_master_sp,
        header_footer,
        notes: None,
        layout_name: None,
    })
}

// === Background / HeaderFooter ===========================================

pub(crate) fn build_background(
    bg: &RawBg,
    resolver: &ColorResolver,
    rels: &BTreeMap<String, Relationship>,
    base_path: &str,
    archive: &mut PptxArchive,
) -> Option<Background> {
    let bg_pr = bg.bg_pr.as_ref()?;
    let mut ctx = FillParseContext {
        rels,
        archive,
        base_path,
        group_fill: None,
    };
    let fill = bg_pr
        .fill
        .as_ref()
        .and_then(|f| build_fill(f, resolver, Some(&mut ctx)));
    Some(Background { fill })
}

fn build_slide_header_footer(hf: &RawSlideHf) -> SlideHeaderFooter {
    // OOXML default for these attrs is `true` per the schema.
    SlideHeaderFooter {
        show_slide_number: parse_optional_truthy(hf.sld_num.as_deref()).unwrap_or(true),
        show_date_time: parse_optional_truthy(hf.dt.as_deref()).unwrap_or(true),
        show_footer: parse_optional_truthy(hf.ftr.as_deref()).unwrap_or(true),
        footer_text: None,
        datetime_text: None,
    }
}

// === Shape tree =========================================================

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_sp_tree_elements(
    children: &[SpTreeChild],
    rels: &BTreeMap<String, Relationship>,
    base_path: &str,
    archive: &mut PptxArchive,
    resolver: &ColorResolver,
    font_scheme: Option<&FontScheme>,
    fmt_scheme: Option<&FormatScheme>,
    placeholder_styles: Option<&[PlaceholderStyleInfo]>,
    parent_group_fill: Option<&slideglance_model::Fill>,
) -> Vec<SlideElement> {
    let mut elements: Vec<SlideElement> = Vec::new();
    for child in children {
        push_sp_tree_child(
            child,
            &mut elements,
            rels,
            base_path,
            archive,
            resolver,
            font_scheme,
            fmt_scheme,
            placeholder_styles,
            parent_group_fill,
        );
    }
    elements
}

#[allow(clippy::too_many_arguments)]
fn push_sp_tree_child(
    child: &SpTreeChild,
    elements: &mut Vec<SlideElement>,
    rels: &BTreeMap<String, Relationship>,
    base_path: &str,
    archive: &mut PptxArchive,
    resolver: &ColorResolver,
    font_scheme: Option<&FontScheme>,
    fmt_scheme: Option<&FormatScheme>,
    placeholder_styles: Option<&[PlaceholderStyleInfo]>,
    parent_group_fill: Option<&slideglance_model::Fill>,
) {
    match child {
        SpTreeChild::Sp(sp) => {
            if let Some(shape) = shape::build_shape(
                sp,
                resolver,
                rels,
                base_path,
                archive,
                font_scheme,
                fmt_scheme,
                placeholder_styles,
                parent_group_fill,
            ) {
                elements.push(SlideElement::Shape(shape));
            }
        }
        SpTreeChild::Pic(pic) => {
            if let Some(image) = image::build_image(pic, rels, base_path, archive, resolver) {
                elements.push(SlideElement::Image(image));
            }
        }
        SpTreeChild::CxnSp(cxn) => {
            if let Some(connector) = connector::build_connector(cxn, resolver, fmt_scheme) {
                elements.push(SlideElement::Connector(connector));
            }
        }
        SpTreeChild::GrpSp(grp) => {
            if let Some(group) = group::build_group(
                grp,
                rels,
                base_path,
                archive,
                resolver,
                font_scheme,
                fmt_scheme,
                placeholder_styles,
                parent_group_fill,
            ) {
                elements.push(SlideElement::Group(group));
            }
        }
        SpTreeChild::GraphicFrame(gf) => {
            if let Some(element) = graphic::build_graphic_frame(
                gf,
                rels,
                base_path,
                archive,
                resolver,
                font_scheme,
                fmt_scheme,
            ) {
                elements.push(element);
            }
        }
        SpTreeChild::AlternateContent(ac) => {
            // mc:AlternateContent: pick the first <Choice> and process its
            // children in source order — TS.
            if let Some(choice) = ac.choice.first() {
                for sub in &choice.children {
                    push_sp_tree_child(
                        sub,
                        elements,
                        rels,
                        base_path,
                        archive,
                        resolver,
                        font_scheme,
                        fmt_scheme,
                        placeholder_styles,
                        parent_group_fill,
                    );
                }
            }
        }
    }
}

// === Shape (sp) =========================================================

// === Image (pic) ========================================================

// === Connector (cxnSp) ==================================================

// === Group (grpSp) ======================================================

// === GraphicFrame (chart / table / smartArt) ============================

pub(super) fn resolve_drawing_path(
    data_path: &str,
    archive: &PptxArchive,
    slide_rels: &BTreeMap<String, Relationship>,
    slide_path: &str,
) -> Option<String> {
    let data_rels_path = build_rels_path(data_path);
    if let Some(rels_xml) = archive.xml(&data_rels_path) {
        if let Ok(data_rels) = parse_relationships(rels_xml) {
            for rel in data_rels.values() {
                if rel.ty.contains("diagramDrawing") {
                    return Some(resolve_relationship_target(data_path, &rel.target));
                }
            }
        }
    }
    // Fallback: look for diagramDrawing in the slide's own rels.
    for rel in slide_rels.values() {
        if rel.ty.contains("diagramDrawing") {
            return Some(resolve_relationship_target(slide_path, &rel.target));
        }
    }
    None
}

// === Placeholder fallback ===============================================

pub(super) fn find_matching_placeholder<'a>(
    ph_type: &str,
    ph_idx: Option<u32>,
    styles: &'a [PlaceholderStyleInfo],
) -> Option<&'a PlaceholderStyleInfo> {
    if let Some(idx) = ph_idx {
        if let Some(found) = styles
            .iter()
            .find(|s| s.placeholder_idx == Some(idx) && s.transform.is_some())
        {
            return Some(found);
        }
        if let Some(found) = styles.iter().find(|s| s.placeholder_idx == Some(idx)) {
            return Some(found);
        }
    }

    if let Some(found) = styles
        .iter()
        .find(|s| s.placeholder_type == ph_type && s.transform.is_some())
    {
        return Some(found);
    }

    let fallback = match ph_type {
        "ctrTitle" => Some("title"),
        "subTitle" => Some("body"),
        _ => None,
    };
    if let Some(ft) = fallback {
        if let Some(found) = styles
            .iter()
            .find(|s| s.placeholder_type == ft && s.transform.is_some())
        {
            return Some(found);
        }
    }

    if let Some(found) = styles.iter().find(|s| s.placeholder_type == ph_type) {
        return Some(found);
    }
    if let Some(ft) = fallback {
        return styles.iter().find(|s| s.placeholder_type == ft);
    }
    None
}

// === small parsers ======================================================

pub(super) fn rect_geometry() -> Geometry {
    Geometry::Preset(PresetGeometry {
        preset: "rect".to_owned(),
        adjust_values: BTreeMap::new(),
    })
}

pub(super) fn parse_attr_i64(s: Option<&str>, default: i64) -> i64 {
    s.and_then(|v| v.parse::<i64>().ok()).unwrap_or(default)
}

pub(super) fn parse_truthy(s: &str) -> bool {
    s == "1" || s == "true"
}

/// Extract the `cNvPr/@id` attribute as `u32`. Returns `None` when the
/// `cNvPr` element is missing, the `@id` attribute is absent, or the
/// value cannot be parsed as a non-negative 32-bit integer (per
/// ECMA-376, `cNvPr/@id` is `xsd:unsignedInt`).
pub(super) fn parse_sp_id(cnv_pr: Option<&RawCNvPr>) -> Option<u32> {
    cnv_pr
        .and_then(|c| c.id.as_deref())
        .and_then(|s| s.parse::<u32>().ok())
}

pub(super) fn parse_optional_truthy(s: Option<&str>) -> Option<bool> {
    match s? {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

// === Raw XML types ======================================================

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawSlide {
    #[serde(rename = "@showMasterSp")]
    pub show_master_sp: Option<String>,
    #[serde(rename = "cSld")]
    pub c_sld: Option<RawCSld>,
    pub hf: Option<RawSlideHf>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawCSld {
    /// `<p:cSld @name>` — only meaningful for slide layouts; slides
    /// themselves do not surface it through their model. Kept on the raw
    /// type so the same struct can be reused by slide-layout-parser.
    #[serde(rename = "@name")]
    #[allow(dead_code)]
    pub name: Option<String>,
    pub bg: Option<RawBg>,
    #[serde(rename = "spTree")]
    pub sp_tree: Option<RawSpTree>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawBg {
    #[serde(rename = "bgPr")]
    pub bg_pr: Option<RawBgPr>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawBgPr {
    #[serde(flatten)]
    pub fill: Option<RawFillContainer>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawSlideHf {
    #[serde(rename = "@sldNum")]
    pub sld_num: Option<String>,
    #[serde(rename = "@dt")]
    pub dt: Option<String>,
    #[serde(rename = "@ftr")]
    pub ftr: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawSpTree {
    /// `<p:nvGrpSpPr>` — only meaningful for slide masters / layouts where
    /// the spTree's own `cNvPr.name` is preserved. Slides do not consume it.
    #[serde(rename = "nvGrpSpPr")]
    #[allow(dead_code)]
    pub nv_grp_sp_pr: Option<RawNvGrpSpPr>,
    #[serde(rename = "grpSpPr")]
    pub grp_sp_pr: Option<RawGrpSpPr>,
    #[serde(rename = "$value", default)]
    pub children: Vec<SpTreeChild>,
}

/// Source-order children of `<p:spTree>` / `<a:grpSp>`. Mirrors the TS
/// reference's preserveOrder parsing — quick-xml's `$value` enum collects
/// every shape variant in document order.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Deserialize)]
pub(crate) enum SpTreeChild {
    #[serde(rename = "sp")]
    Sp(RawSp),
    #[serde(rename = "pic")]
    Pic(RawPic),
    #[serde(rename = "cxnSp")]
    CxnSp(RawCxnSp),
    /// `<p:grpSp>` is recursive; it carries children of the same enum so we
    /// box it to break the cycle.
    #[serde(rename = "grpSp")]
    GrpSp(Box<RawGrpSp>),
    #[serde(rename = "graphicFrame")]
    GraphicFrame(RawGraphicFrame),
    #[serde(rename = "AlternateContent")]
    AlternateContent(RawAlternateContent),
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawAlternateContent {
    #[serde(rename = "Choice", default)]
    pub choice: Vec<RawAlternateChoice>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawAlternateChoice {
    #[serde(rename = "$value", default)]
    pub children: Vec<SpTreeChild>,
}

// Diagram drawing wrapper: `<dgm:drawing>` (after namespace strip → `drawing`).
#[derive(Debug, Default, Deserialize)]
pub(super) struct RawDrawing {
    #[serde(rename = "spTree")]
    sp_tree: Option<RawSpTree>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn raw_cnvpr_parses_id_attribute() {
        let xml = r#"<cNvPr id="42" name="Title 1"/>"#;
        let parsed: super::RawCNvPr = quick_xml::de::from_str(xml).unwrap();
        assert_eq!(parsed.id.as_deref(), Some("42"));
        assert_eq!(parsed.name.as_deref(), Some("Title 1"));
    }
}

#[cfg(test)]
mod gradient_flatten_repro {
    use super::*;
    use crate::xml::{parse_xml, strip_namespaces};

    #[test]
    fn gradfill_under_sppr_via_flatten() {
        let xml = r#"<spPr bwMode="auto">
<xfrm><off x="0" y="1612915"/><ext cx="7559675" cy="1637179"/></xfrm>
<prstGeom prst="rect"><avLst/></prstGeom>
<gradFill>
<gsLst>
<gs pos="0"><schemeClr val="accent1"><lumMod val="5000"/><lumOff val="95000"/><alpha val="0"/></schemeClr></gs>
<gs pos="100000"><schemeClr val="bg1"/></gs>
</gsLst>
<lin ang="16200000" scaled="0"/>
</gradFill>
<ln><noFill/></ln>
</spPr>"#;
        let stripped = strip_namespaces(xml).expect("strip");
        let parsed: RawShapeSpPr = parse_xml(&stripped).expect("parse");
        // gradFill should be detected via the inlined fill-choice fields.
        assert!(parsed.has_fill_choice(), "no fill choice captured");
        assert!(
            parsed.grad_fill.is_some(),
            "gradFill should be parsed; got {parsed:#?}"
        );
        let stops = parsed
            .grad_fill
            .as_ref()
            .and_then(|g| g.gs_lst.as_ref())
            .map_or(0, |l| l.gs.len());
        assert_eq!(stops, 2, "two stops expected");
    }
}
