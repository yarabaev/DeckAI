//! Slide-XML element parser — shape.
//!
//! Extracted from `slide.rs` so the per-element builder lives next
//! to its raw-XML model. The dispatcher in `slide::push_sp_tree_child`
//! routes to the `pub(super)` entry point in this module.

use std::collections::BTreeMap;

use serde::Deserialize;
use slideglance_color::ColorResolver;
use slideglance_model::{FontScheme, FormatScheme, Geometry, PlaceholderStyleInfo};

use crate::archive::PptxArchive;
use crate::effect::{build_effect_list, RawEffectLst};
use crate::fill::{
    build_fill, build_outline, EmptyMarker, FillParseContext, RawFillContainer, RawOutline,
};
use crate::relationships::Relationship;
use crate::shape_geometry::{build_geometry_parts, build_transform, RawXfrm};
use crate::style_reference::{build_resolved_style, RawStyle};
use crate::text_body::{build_hyperlink, build_text_body, RawHlinkClick, RawTextBody};

use super::{find_matching_placeholder, parse_sp_id, parse_truthy, rect_geometry};

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn build_shape(
    sp: &RawSp,
    resolver: &ColorResolver,
    rels: &BTreeMap<String, Relationship>,
    base_path: &str,
    archive: &mut PptxArchive,
    font_scheme: Option<&FontScheme>,
    fmt_scheme: Option<&FormatScheme>,
    placeholder_styles: Option<&[PlaceholderStyleInfo]>,
    parent_group_fill: Option<&slideglance_model::Fill>,
) -> Option<slideglance_model::ShapeElement> {
    let sp_pr_present = sp.sp_pr.is_some();
    let placeholder = sp
        .nv_sp_pr
        .as_ref()
        .and_then(|n| n.nv_pr.as_ref())
        .and_then(|n| n.ph.as_ref());
    let placeholder_type = placeholder.map(|ph| ph.ty.clone().unwrap_or_else(|| "body".to_owned()));
    let placeholder_idx = placeholder
        .and_then(|ph| ph.idx.as_deref())
        .and_then(|s| s.parse::<u32>().ok());

    let mut transform = sp
        .sp_pr
        .as_ref()
        .and_then(|sp_pr| sp_pr.xfrm.as_ref())
        .and_then(build_transform);
    let mut geometry: Geometry = if let Some(sp_pr) = sp.sp_pr.as_ref() {
        build_shape_spr_geometry(sp_pr)
    } else {
        rect_geometry()
    };

    if transform.is_none() {
        if let (Some(ph_type), Some(styles)) = (placeholder_type.as_deref(), placeholder_styles) {
            if let Some(inherited) = find_matching_placeholder(ph_type, placeholder_idx, styles) {
                if let Some(t) = inherited.transform {
                    transform = Some(t);
                }
                if !sp_pr_present {
                    if let Some(g) = inherited.geometry.clone() {
                        geometry = g;
                    }
                }
            }
        }
    }

    let transform = transform?;

    let style_ref = sp
        .style
        .as_ref()
        .and_then(|s| build_resolved_style(s, fmt_scheme, resolver));

    let direct_fill = sp.sp_pr.as_ref().and_then(|sp_pr| {
        if !sp_pr.has_fill_choice() {
            return None;
        }
        let fill_container = sp_pr.fill_container();
        let mut ctx = FillParseContext {
            rels,
            archive,
            base_path,
            group_fill: parent_group_fill,
        };
        build_fill(&fill_container, resolver, Some(&mut ctx))
    });
    let fill = direct_fill.or_else(|| style_ref.as_ref().and_then(|s| s.fill.clone()));

    // OOXML cascade rule: an explicit `<a:ln><a:noFill/></a:ln>` on
    // the shape opts out of the outline entirely, *blocking* the
    // `<p:style>/<a:lnRef>` fallback. Without this guard, every
    // sysDot rounded-rect card (test deck slide 9) inherits the
    // theme line and grows a four-sided dotted border that the
    // reference doesn't have.
    let outline_explicit_none = sp
        .sp_pr
        .as_ref()
        .and_then(|sp_pr| sp_pr.ln.as_ref())
        .is_some_and(|ln| ln.no_fill.is_some());
    let direct_outline = sp
        .sp_pr
        .as_ref()
        .and_then(|sp_pr| sp_pr.ln.as_ref())
        .and_then(|ln| build_outline(ln, resolver));
    let style_outline = style_ref.as_ref().and_then(|s| s.outline.clone());
    // Same merge rule as build_connector: a partial inline `<a:ln>` (e.g.
    // dash-only) keeps its overrides but pulls a missing fill from the
    // shape's `<p:style><a:lnRef>` template instead of stranding the
    // outline as `stroke="none"`.
    let outline = if outline_explicit_none {
        None
    } else {
        match (direct_outline, style_outline) {
            (Some(mut direct), Some(style)) => {
                if direct.fill.is_none() {
                    direct.fill = style.fill;
                }
                Some(direct)
            }
            (Some(direct), None) => Some(direct),
            (None, Some(style)) => Some(style),
            (None, None) => None,
        }
    };

    let direct_effects = sp
        .sp_pr
        .as_ref()
        .and_then(|sp_pr| sp_pr.effect_lst.as_ref())
        .and_then(|e| build_effect_list(e, resolver));
    let effects = direct_effects.or_else(|| style_ref.as_ref().and_then(|s| s.effects.clone()));

    // The shape's `<p:style>/<a:fontRef>` carries the default text
    // color (e.g. `lt1` -> white) that `<a:rPr>`-less runs should
    // inherit. PowerPoint applies this whenever the run, layout
    // lstStyle, and master textStyles all leave the color unset.
    let font_ref_color = style_ref
        .as_ref()
        .and_then(|s| s.font_ref.as_ref())
        .and_then(|fr| fr.color);
    let mut text_body = sp.tx_body.as_ref().and_then(|tb| {
        build_text_body(tb, resolver, Some(rels), font_scheme, None, font_ref_color)
    });
    // Field-wise merge of layout/master placeholder bodyPr explicit
    // attributes into this slide shape's text body. Only attributes the
    // slide left absent in its own `<a:bodyPr>` adopt the inherited
    // value — explicit slide attributes always win. This implements
    // MS-OE376 §5.1.5.1.1's "applicable ancestor element" walk for
    // bodyPr without overriding spec-default fields, which the earlier
    // attempt regressed by transmitting parser defaults.
    if let (Some(body), Some(ph_type), Some(styles)) = (
        text_body.as_mut(),
        placeholder_type.as_deref(),
        placeholder_styles,
    ) {
        let raw = sp.tx_body.as_ref().and_then(|tb| tb.body_pr.as_ref());
        let slide_set =
            |attr: fn(&crate::text_body::RawBodyPr) -> bool| -> bool { raw.is_some_and(&attr) };
        let has_anchor = slide_set(|n| n.anchor.is_some());
        let has_lins = slide_set(|n| n.l_ins.is_some());
        let has_rins = slide_set(|n| n.r_ins.is_some());
        let has_tins = slide_set(|n| n.t_ins.is_some());
        let has_bins = slide_set(|n| n.b_ins.is_some());
        let has_wrap = slide_set(|n| n.wrap.is_some());
        let has_vert = slide_set(|n| n.vert.is_some());
        let has_num_col = slide_set(|n| n.num_col.is_some());
        let has_spc_flp = slide_set(|n| n.spc_first_last_para.is_some());
        let has_compat = slide_set(|n| n.compat_ln_spc.is_some());
        // `<a:normAutofit/>` / `<a:spAutoFit/>` are bodyPr children, not
        // attributes — slide-50 right-header (`ph type="body" idx=16`)
        // leaves bodyPr empty and inherits `<a:normAutofit/>` from
        // slideLayout5. Without this fallback the wrap behaviour
        // diverges (single-line vs PowerPoint's two-line layout).
        let has_autofit = slide_set(|n| n.norm_autofit.is_some() || n.sp_auto_fit.is_some());

        if let Some(inherited) = find_matching_placeholder(ph_type, placeholder_idx, styles) {
            if let Some(pbp) = inherited.body_properties.as_ref() {
                if !has_anchor {
                    if let Some(v) = pbp.anchor {
                        body.body_properties.anchor = v;
                    }
                }
                if !has_lins {
                    if let Some(v) = pbp.margin_left {
                        body.body_properties.margin_left = v;
                    }
                }
                if !has_rins {
                    if let Some(v) = pbp.margin_right {
                        body.body_properties.margin_right = v;
                    }
                }
                if !has_tins {
                    if let Some(v) = pbp.margin_top {
                        body.body_properties.margin_top = v;
                    }
                }
                if !has_bins {
                    if let Some(v) = pbp.margin_bottom {
                        body.body_properties.margin_bottom = v;
                    }
                }
                if !has_wrap {
                    if let Some(v) = pbp.wrap {
                        body.body_properties.wrap = v;
                    }
                }
                if !has_vert {
                    if let Some(v) = pbp.vert {
                        body.body_properties.vert = v;
                    }
                }
                if !has_num_col {
                    if let Some(v) = pbp.num_col {
                        body.body_properties.num_col = v;
                    }
                }
                if !has_spc_flp {
                    if let Some(v) = pbp.spc_first_last_para {
                        body.body_properties.spc_first_last_para = v;
                    }
                }
                if !has_compat {
                    if let Some(v) = pbp.compat_ln_spc {
                        body.body_properties.compat_ln_spc = v;
                    }
                }
                // auto_fit + companions inherit as a group — they all
                // come from the same `<a:normAutofit/>` / `<a:spAutoFit/>`
                // child node. Slide-50 ph=body idx=16 is the test case.
                if !has_autofit {
                    if let Some(v) = pbp.auto_fit {
                        body.body_properties.auto_fit = v;
                        if let Some(fs) = pbp.font_scale {
                            body.body_properties.font_scale = fs;
                        }
                        if let Some(lr) = pbp.ln_spc_reduction {
                            body.body_properties.ln_spc_reduction = lr;
                        }
                    }
                }
            }
        }
    }

    let cnv_pr = sp.nv_sp_pr.as_ref().and_then(|n| n.c_nv_pr.as_ref());
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
    let hyperlink = build_hyperlink(cnv_pr.and_then(|c| c.hlink_click.as_ref()), Some(rels));

    Some(slideglance_model::ShapeElement {
        sp_id,
        transform,
        geometry,
        fill,
        outline,
        text_body,
        effects,
        placeholder_type,
        placeholder_idx,
        alt_text,
        object_name,
        hidden,
        hyperlink,
    })
}

pub(super) fn build_shape_spr_geometry(sp_pr: &RawShapeSpPr) -> Geometry {
    build_geometry_parts(sp_pr.prst_geom.as_ref(), sp_pr.cust_geom.as_ref())
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawSp {
    #[serde(rename = "nvSpPr")]
    pub nv_sp_pr: Option<RawNvSpPr>,
    #[serde(rename = "spPr")]
    pub sp_pr: Option<RawShapeSpPr>,
    pub style: Option<RawStyle>,
    #[serde(rename = "txBody")]
    pub tx_body: Option<RawTextBody>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawNvSpPr {
    #[serde(rename = "cNvPr")]
    pub c_nv_pr: Option<RawCNvPr>,
    #[serde(rename = "nvPr")]
    pub nv_pr: Option<RawNvPr>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawNvPr {
    pub ph: Option<RawPh>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawPh {
    #[serde(rename = "@type")]
    pub ty: Option<String>,
    #[serde(rename = "@idx")]
    pub idx: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawCNvPr {
    /// `cNvPr/@id` (ECMA-376 `xsd:unsignedInt`). Threaded into the model
    /// via `parse_sp_id` at every `build_*` site so renderers can emit
    /// stable per-element identifiers (e.g. `data-sp-id`).
    #[serde(rename = "@id")]
    pub id: Option<String>,
    #[serde(rename = "@descr")]
    pub descr: Option<String>,
    #[serde(rename = "@name")]
    pub name: Option<String>,
    #[serde(rename = "@hidden")]
    pub hidden: Option<String>,
    #[serde(rename = "hlinkClick")]
    pub hlink_click: Option<RawHlinkClick>,
}

/// `<a:spPr>` with the union of fields used by shapes / connectors:
/// transform, geometry, fills (flattened — same pattern as table.rs:tcPr),
/// outline, and effect list. `scene3d` / `sp3d` are accepted but currently
/// unused (see TS warn fallback).
#[allow(clippy::struct_field_names)]
#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawShapeSpPr {
    pub xfrm: Option<RawXfrm>,
    #[serde(rename = "prstGeom")]
    pub prst_geom: Option<crate::shape_geometry::RawPrstGeom>,
    #[serde(rename = "custGeom")]
    pub cust_geom: Option<crate::custom_geometry::RawCustGeom>,
    // Fill choice — quick-xml's serde flatten over Option<RawFillContainer>
    // silently drops `gradFill` on slide-level <p:spPr> (verified by
    // `gradient_flatten_repro::gradfill_under_sppr_via_flatten`). Inline each
    // child explicitly so the deserializer doesn't lose gradient/blip/pattern
    // children. The companion `fill_container()` helper recombines them into
    // a `RawFillContainer` for `build_fill`.
    #[serde(rename = "noFill")]
    pub no_fill: Option<EmptyMarker>,
    #[serde(rename = "solidFill")]
    pub solid_fill: Option<crate::fill::RawSolidFill>,
    #[serde(rename = "gradFill")]
    pub grad_fill: Option<crate::fill::RawGradFill>,
    #[serde(rename = "blipFill")]
    pub blip_fill: Option<crate::fill::RawBlipFill>,
    #[serde(rename = "pattFill")]
    pub patt_fill: Option<crate::fill::RawPattFill>,
    #[serde(rename = "grpFill")]
    pub grp_fill: Option<EmptyMarker>,
    pub ln: Option<RawOutline>,
    #[serde(rename = "effectLst")]
    pub effect_lst: Option<RawEffectLst>,
    /// Currently unused — scene3d is not yet implemented; we only accept the
    /// element so the deserializer tolerates fixtures that include it.
    #[allow(dead_code)]
    pub scene3d: Option<EmptyMarker>,
    /// Currently unused — sp3d is not yet implemented.
    #[allow(dead_code)]
    pub sp3d: Option<EmptyMarker>,
}

impl RawShapeSpPr {
    /// Recombine the inlined fill-choice fields into a `RawFillContainer` for
    /// `build_fill` consumption. Exists because `#[serde(flatten)]` over
    /// `Option<RawFillContainer>` silently drops `gradFill` on slide-level
    /// `<p:spPr>` (see `gradient_flatten_repro` test).
    pub(crate) fn fill_container(&self) -> RawFillContainer {
        RawFillContainer {
            no_fill: self.no_fill.clone(),
            solid_fill: self.solid_fill.clone(),
            grad_fill: self.grad_fill.clone(),
            blip_fill: self.blip_fill.clone(),
            patt_fill: self.patt_fill.clone(),
            grp_fill: self.grp_fill.clone(),
        }
    }

    /// `true` when at least one fill choice element was present.
    pub(crate) fn has_fill_choice(&self) -> bool {
        self.no_fill.is_some()
            || self.solid_fill.is_some()
            || self.grad_fill.is_some()
            || self.blip_fill.is_some()
            || self.patt_fill.is_some()
            || self.grp_fill.is_some()
    }
}
