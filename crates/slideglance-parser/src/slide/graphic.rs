//! Slide-XML element parser — graphic.
//!
//! Extracted from `slide.rs` so the per-element builder lives next
//! to its raw-XML model. The dispatcher in `slide::push_sp_tree_child`
//! routes to the `pub(super)` entry point in this module.

use std::collections::BTreeMap;

use serde::Deserialize;
use slideglance_color::ColorResolver;
use slideglance_model::{
    ChartElement, FontScheme, FormatScheme, GroupElement, SlideElement, TableElement, Transform,
};
use slideglance_utils::Emu;

use crate::archive::PptxArchive;
use crate::chart::{build_chart, RawChartSpace};
use crate::relationships::{
    build_rels_path, parse_relationships, resolve_relationship_target, Relationship,
};
use crate::shape_geometry::{build_transform, RawXfrm};
use crate::table::{build_table, RawTbl};

use super::shape::RawCNvPr;
use super::RawDrawing;
use super::{
    build_sp_tree_elements, parse_attr_i64, parse_sp_id, parse_truthy, resolve_drawing_path,
};
use crate::xml::parse_xml;

pub(super) fn build_graphic_frame(
    gf: &RawGraphicFrame,
    rels: &BTreeMap<String, Relationship>,
    base_path: &str,
    archive: &mut PptxArchive,
    resolver: &ColorResolver,
    font_scheme: Option<&FontScheme>,
    fmt_scheme: Option<&FormatScheme>,
) -> Option<SlideElement> {
    let xfrm = gf.xfrm.as_ref()?;
    let transform = build_transform(xfrm)?;
    let graphic = gf.graphic.as_ref()?;
    let graphic_data = graphic.graphic_data.as_ref()?;

    let cnv_pr = gf
        .nv_graphic_frame_pr
        .as_ref()
        .and_then(|n| n.c_nv_pr.as_ref());
    let sp_id = parse_sp_id(cnv_pr);
    let object_name = cnv_pr
        .and_then(|c| c.name.clone())
        .filter(|s| !s.is_empty());
    let hidden = cnv_pr
        .and_then(|c| c.hidden.as_deref())
        .is_some_and(parse_truthy);

    if let Some(chart_ref) = graphic_data.chart.as_ref() {
        let r_id = chart_ref.id.as_deref()?;
        let rel = rels.get(r_id)?;
        let chart_path = resolve_relationship_target(base_path, &rel.target);
        let chart_xml = archive.xml(&chart_path)?.to_owned();
        let raw_chart: RawChartSpace = parse_xml(&chart_xml).ok()?;
        let chart = build_chart(&raw_chart, resolver)?;
        return Some(SlideElement::Chart(ChartElement {
            sp_id,
            transform,
            chart,
            object_name,
            hidden,
        }));
    }

    if let Some(tbl) = graphic_data.tbl.as_ref() {
        let table = build_table(tbl, resolver, Some(rels), font_scheme)?;
        return Some(SlideElement::Table(TableElement {
            sp_id,
            transform,
            table,
            object_name,
            hidden,
        }));
    }

    if graphic_data
        .uri
        .as_deref()
        .is_some_and(|uri| uri == "http://schemas.openxmlformats.org/drawingml/2006/diagram")
    {
        return build_smart_art(
            graphic_data,
            transform,
            rels,
            base_path,
            archive,
            resolver,
            font_scheme,
            fmt_scheme,
            object_name,
            hidden,
        )
        .map(SlideElement::Group);
    }

    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_smart_art(
    graphic_data: &RawGraphicData,
    transform: Transform,
    rels: &BTreeMap<String, Relationship>,
    base_path: &str,
    archive: &mut PptxArchive,
    resolver: &ColorResolver,
    font_scheme: Option<&FontScheme>,
    fmt_scheme: Option<&FormatScheme>,
    object_name: Option<String>,
    hidden: bool,
) -> Option<GroupElement> {
    let rel_ids = graphic_data.rel_ids.as_ref()?;
    let dm_id = rel_ids.dm.as_deref()?;
    let dm_rel = rels.get(dm_id)?;
    let data_path = resolve_relationship_target(base_path, &dm_rel.target);

    let drawing_path = resolve_drawing_path(&data_path, archive, rels, base_path)?;
    let drawing_xml = archive.xml(&drawing_path)?.to_owned();
    let raw_drawing: RawDrawing = parse_xml(&drawing_xml).ok()?;

    let drawing_rels_path = build_rels_path(&drawing_path);
    let drawing_rels = match archive.xml(&drawing_rels_path) {
        Some(rels_xml) => parse_relationships(rels_xml).ok()?,
        None => BTreeMap::new(),
    };

    let sp_tree = raw_drawing.sp_tree.as_ref()?;

    // Group child transform comes from the drawing's <a:grpSpPr><a:xfrm>...
    let grp_xfrm = sp_tree.grp_sp_pr.as_ref().and_then(|n| n.xfrm.as_ref());
    let child_off_x = grp_xfrm
        .and_then(|x| x.ch_off.as_ref())
        .map_or(0, |n| parse_attr_i64(n.x.as_deref(), 0));
    let child_off_y = grp_xfrm
        .and_then(|x| x.ch_off.as_ref())
        .map_or(0, |n| parse_attr_i64(n.y.as_deref(), 0));
    let child_ext_w = grp_xfrm
        .and_then(|x| x.ch_ext.as_ref())
        .map_or(transform.extent_width.raw(), |n| {
            parse_attr_i64(n.cx.as_deref(), transform.extent_width.raw())
        });
    let child_ext_h = grp_xfrm
        .and_then(|x| x.ch_ext.as_ref())
        .map_or(transform.extent_height.raw(), |n| {
            parse_attr_i64(n.cy.as_deref(), transform.extent_height.raw())
        });
    let child_transform = Transform {
        offset_x: Emu::new(child_off_x),
        offset_y: Emu::new(child_off_y),
        extent_width: Emu::new(child_ext_w),
        extent_height: Emu::new(child_ext_h),
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
    };

    let children = build_sp_tree_elements(
        &sp_tree.children,
        &drawing_rels,
        &drawing_path,
        archive,
        resolver,
        font_scheme,
        fmt_scheme,
        None,
        None,
    );
    if children.is_empty() {
        return None;
    }

    Some(GroupElement {
        sp_id: None,
        transform,
        child_transform,
        children,
        effects: None,
        alt_text: None,
        object_name,
        hidden,
    })
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawGraphicFrame {
    #[serde(rename = "nvGraphicFramePr")]
    pub nv_graphic_frame_pr: Option<RawNvGraphicFramePr>,
    pub xfrm: Option<RawXfrm>,
    pub graphic: Option<RawGraphic>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawNvGraphicFramePr {
    #[serde(rename = "cNvPr")]
    pub c_nv_pr: Option<RawCNvPr>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawGraphic {
    #[serde(rename = "graphicData")]
    pub graphic_data: Option<RawGraphicData>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawGraphicData {
    #[serde(rename = "@uri")]
    pub uri: Option<String>,
    pub chart: Option<RawChartRef>,
    pub tbl: Option<RawTbl>,
    #[serde(rename = "relIds")]
    pub rel_ids: Option<RawRelIds>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawChartRef {
    #[serde(rename = "@id")]
    pub id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawRelIds {
    #[serde(rename = "@dm")]
    pub dm: Option<String>,
}
