//! Slide-XML element parser — group.
//!
//! Extracted from `slide.rs` so the per-element builder lives next
//! to its raw-XML model. The dispatcher in `slide::push_sp_tree_child`
//! routes to the `pub(super)` entry point in this module.

use std::collections::BTreeMap;

use serde::Deserialize;
use slideglance_color::ColorResolver;
use slideglance_model::{FontScheme, FormatScheme, GroupElement, PlaceholderStyleInfo, Transform};
use slideglance_utils::Emu;

use crate::archive::PptxArchive;
use crate::effect::{build_effect_list, RawEffectLst};
use crate::fill::{build_fill, FillParseContext, RawFillContainer};
use crate::relationships::Relationship;

use super::shape::RawCNvPr;
use super::{build_sp_tree_elements, SpTreeChild};
use super::{parse_attr_i64, parse_sp_id, parse_truthy};

#[allow(clippy::too_many_arguments)]
pub(super) fn build_group(
    grp: &RawGrpSp,
    rels: &BTreeMap<String, Relationship>,
    base_path: &str,
    archive: &mut PptxArchive,
    resolver: &ColorResolver,
    font_scheme: Option<&FontScheme>,
    fmt_scheme: Option<&FormatScheme>,
    placeholder_styles: Option<&[PlaceholderStyleInfo]>,
    parent_group_fill: Option<&slideglance_model::Fill>,
) -> Option<GroupElement> {
    let grp_sp_pr = grp.grp_sp_pr.as_ref()?;
    let xfrm = grp_sp_pr.xfrm.as_ref()?;
    let transform = build_grp_transform(xfrm)?;

    let child_off_x = xfrm
        .ch_off
        .as_ref()
        .map_or(0, |n| parse_attr_i64(n.x.as_deref(), 0));
    let child_off_y = xfrm
        .ch_off
        .as_ref()
        .map_or(0, |n| parse_attr_i64(n.y.as_deref(), 0));
    let child_ext_w = xfrm
        .ch_ext
        .as_ref()
        .map_or(transform.extent_width.raw(), |n| {
            parse_attr_i64(n.cx.as_deref(), transform.extent_width.raw())
        });
    let child_ext_h = xfrm
        .ch_ext
        .as_ref()
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

    let group_fill = grp_sp_pr.fill.as_ref().and_then(|f| {
        let mut ctx = FillParseContext {
            rels,
            archive,
            base_path,
            group_fill: parent_group_fill,
        };
        build_fill(f, resolver, Some(&mut ctx))
    });

    let children = build_sp_tree_elements(
        &grp.children,
        rels,
        base_path,
        archive,
        resolver,
        font_scheme,
        fmt_scheme,
        placeholder_styles,
        group_fill.as_ref(),
    );

    let effects = grp_sp_pr
        .effect_lst
        .as_ref()
        .and_then(|e| build_effect_list(e, resolver));

    let cnv_pr = grp.nv_grp_sp_pr.as_ref().and_then(|n| n.c_nv_pr.as_ref());
    let sp_id = parse_sp_id(cnv_pr);
    let alt_text = cnv_pr
        .and_then(|c| c.descr.clone())
        .filter(|s| !s.is_empty());
    let object_name = cnv_pr
        .and_then(|c| c.name.clone())
        .filter(|s| !s.is_empty());
    let hidden = cnv_pr
        .and_then(|c| c.hidden.as_deref())
        .is_some_and(parse_truthy);

    Some(GroupElement {
        sp_id,
        transform,
        child_transform,
        children,
        effects,
        alt_text,
        object_name,
        hidden,
    })
}

pub(super) fn build_grp_transform(xfrm: &RawGrpXfrm) -> Option<Transform> {
    let off = xfrm.off.as_ref()?;
    let ext = xfrm.ext.as_ref()?;
    Some(Transform {
        offset_x: Emu::new(parse_attr_i64(off.x.as_deref(), 0)),
        offset_y: Emu::new(parse_attr_i64(off.y.as_deref(), 0)),
        extent_width: Emu::new(parse_attr_i64(ext.cx.as_deref(), 0)),
        extent_height: Emu::new(parse_attr_i64(ext.cy.as_deref(), 0)),
        rotation: parse_attr_i64(xfrm.rot.as_deref(), 0) as f64 / 60_000.0,
        flip_h: xfrm.flip_h.as_deref().is_some_and(parse_truthy),
        flip_v: xfrm.flip_v.as_deref().is_some_and(parse_truthy),
    })
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawGrpSp {
    #[serde(rename = "nvGrpSpPr")]
    pub nv_grp_sp_pr: Option<RawNvGrpSpPr>,
    #[serde(rename = "grpSpPr")]
    pub grp_sp_pr: Option<RawGrpSpPr>,
    #[serde(rename = "$value", default)]
    pub children: Vec<SpTreeChild>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawNvGrpSpPr {
    #[serde(rename = "cNvPr")]
    pub c_nv_pr: Option<RawCNvPr>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawGrpSpPr {
    pub xfrm: Option<RawGrpXfrm>,
    #[serde(flatten)]
    pub fill: Option<RawFillContainer>,
    #[serde(rename = "effectLst")]
    pub effect_lst: Option<RawEffectLst>,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub(crate) struct RawGrpXfrm {
    #[serde(rename = "@rot")]
    pub rot: Option<String>,
    #[serde(rename = "@flipH")]
    pub flip_h: Option<String>,
    #[serde(rename = "@flipV")]
    pub flip_v: Option<String>,
    pub off: Option<RawXY>,
    pub ext: Option<RawCxCy>,
    #[serde(rename = "chOff")]
    pub ch_off: Option<RawXY>,
    #[serde(rename = "chExt")]
    pub ch_ext: Option<RawCxCy>,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub(crate) struct RawXY {
    #[serde(rename = "@x")]
    pub x: Option<String>,
    #[serde(rename = "@y")]
    pub y: Option<String>,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub(crate) struct RawCxCy {
    #[serde(rename = "@cx")]
    pub cx: Option<String>,
    #[serde(rename = "@cy")]
    pub cy: Option<String>,
}
