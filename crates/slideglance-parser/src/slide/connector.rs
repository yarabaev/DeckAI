//! Slide-XML element parser — connector.
//!
//! Extracted from `slide.rs` so the per-element builder lives next
//! to its raw-XML model. The dispatcher in `slide::push_sp_tree_child`
//! routes to the `pub(super)` entry point in this module.

use serde::Deserialize;
use slideglance_color::ColorResolver;
use slideglance_model::{ConnectorElement, FormatScheme};

use crate::effect::build_effect_list;
use crate::fill::build_outline;
use crate::shape_geometry::build_transform;
use crate::style_reference::{build_resolved_style, RawStyle};

use super::shape::{build_shape_spr_geometry, RawCNvPr, RawShapeSpPr};
use super::{parse_sp_id, parse_truthy};

pub(super) fn build_connector(
    cxn: &RawCxnSp,
    resolver: &ColorResolver,
    fmt_scheme: Option<&FormatScheme>,
) -> Option<ConnectorElement> {
    let sp_pr = cxn.sp_pr.as_ref()?;
    let transform = sp_pr.xfrm.as_ref().and_then(build_transform)?;
    let geometry = build_shape_spr_geometry(sp_pr);

    let style_ref = cxn
        .style
        .as_ref()
        .and_then(|s| build_resolved_style(s, fmt_scheme, resolver));
    let outline_explicit_none = sp_pr.ln.as_ref().is_some_and(|ln| ln.no_fill.is_some());
    let direct_outline = sp_pr.ln.as_ref().and_then(|ln| build_outline(ln, resolver));
    let style_outline = style_ref.as_ref().and_then(|s| s.outline.clone());
    // OOXML outline merging: an inline `<a:ln>` may carry partial properties
    // (dash style, end caps, width) but omit the fill, in which case the
    // missing pieces fall back to the connector's `<p:style><a:lnRef>`
    // template. Slide 9's `직선 화살표 연결선` family illustrates the case —
    // `<a:ln><a:prstDash val="sysDot"/><a:tailEnd type="none"/></a:ln>` has
    // no `<a:solidFill>` and used to render as `stroke="none"` because the
    // direct outline replaced the style reference outright. Merging keeps
    // the inline dash style / arrow ends while picking up the accent1
    // colour from the lnRef template.
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
    let direct_effects = sp_pr
        .effect_lst
        .as_ref()
        .and_then(|e| build_effect_list(e, resolver));
    let effects = direct_effects.or_else(|| style_ref.as_ref().and_then(|s| s.effects.clone()));

    let cnv_pr = cxn.nv_cxn_sp_pr.as_ref().and_then(|n| n.c_nv_pr.as_ref());
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

    Some(ConnectorElement {
        sp_id,
        transform,
        geometry,
        outline,
        effects,
        alt_text,
        object_name,
        hidden,
    })
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawCxnSp {
    #[serde(rename = "nvCxnSpPr")]
    pub nv_cxn_sp_pr: Option<RawNvCxnSpPr>,
    #[serde(rename = "spPr")]
    pub sp_pr: Option<RawShapeSpPr>,
    pub style: Option<RawStyle>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawNvCxnSpPr {
    #[serde(rename = "cNvPr")]
    pub c_nv_pr: Option<RawCNvPr>,
}
