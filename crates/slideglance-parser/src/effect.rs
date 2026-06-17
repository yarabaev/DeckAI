//! `<a:effectLst>` parser.
//!
//! Mirrors.

use serde::Deserialize;
use slideglance_color::ColorResolver;
use slideglance_model::{EffectList, Glow, InnerShadow, OuterShadow, SoftEdge};
use slideglance_utils::Emu;

use crate::raw_color::RawColorChoice;
use crate::xml::{parse_xml, XmlError};

const ROTATION_UNIT_PER_DEGREE: f64 = 60_000.0;

/// Parses an `<a:effectLst>` XML body.
///
/// Returns `Ok(None)` when the body parses but contains no recognized effect
/// children (matches the spec). Returns `Err` only when the XML is
/// malformed.
///
/// # Errors
///
/// Returns [`XmlError`] when the input is not well-formed XML.
pub fn parse_effect_list(
    xml: &str,
    resolver: &ColorResolver,
) -> Result<Option<EffectList>, XmlError> {
    let raw: RawEffectLst = parse_xml(xml)?;
    Ok(build_effect_list(&raw, resolver))
}

pub(crate) fn build_effect_list(
    raw: &RawEffectLst,
    resolver: &ColorResolver,
) -> Option<EffectList> {
    let outer_shadow = raw
        .outer_shdw
        .as_ref()
        .and_then(|n| build_outer_shadow(n, resolver));
    let inner_shadow = raw
        .inner_shdw
        .as_ref()
        .and_then(|n| build_inner_shadow(n, resolver));
    let glow = raw.glow.as_ref().and_then(|n| build_glow(n, resolver));
    let soft_edge = raw.soft_edge.as_ref().map(build_soft_edge);

    if outer_shadow.is_none() && inner_shadow.is_none() && glow.is_none() && soft_edge.is_none() {
        return None;
    }
    Some(EffectList {
        outer_shadow,
        inner_shadow,
        glow,
        soft_edge,
    })
}

fn build_outer_shadow(node: &RawOuterShdw, resolver: &ColorResolver) -> Option<OuterShadow> {
    let color_ref = node.color.to_color_ref()?;
    let color = resolver.resolve(&color_ref);
    Some(OuterShadow {
        blur_radius: Emu::new(node.blur_rad.unwrap_or(0)),
        distance: Emu::new(node.dist.unwrap_or(0)),
        direction: f64::from(i32::try_from(node.dir.unwrap_or(0)).unwrap_or(0))
            / ROTATION_UNIT_PER_DEGREE,
        color,
        // OOXML default for `algn` is "b" (bottom).
        alignment: node.algn.clone().unwrap_or_else(|| "b".to_owned()),
        // OOXML default for `rotWithShape` is "1" (true) — only literal "0"
        // disables rotation with the shape.
        rotate_with_shape: node.rot_with_shape.as_deref() != Some("0"),
    })
}

fn build_inner_shadow(node: &RawInnerShdw, resolver: &ColorResolver) -> Option<InnerShadow> {
    let color_ref = node.color.to_color_ref()?;
    let color = resolver.resolve(&color_ref);
    Some(InnerShadow {
        blur_radius: Emu::new(node.blur_rad.unwrap_or(0)),
        distance: Emu::new(node.dist.unwrap_or(0)),
        direction: f64::from(i32::try_from(node.dir.unwrap_or(0)).unwrap_or(0))
            / ROTATION_UNIT_PER_DEGREE,
        color,
    })
}

fn build_glow(node: &RawGlow, resolver: &ColorResolver) -> Option<Glow> {
    let color_ref = node.color.to_color_ref()?;
    let color = resolver.resolve(&color_ref);
    Some(Glow {
        radius: Emu::new(node.rad.unwrap_or(0)),
        color,
    })
}

fn build_soft_edge(node: &RawSoftEdge) -> SoftEdge {
    SoftEdge {
        radius: Emu::new(node.rad.unwrap_or(0)),
    }
}

// --- raw XML shapes (post namespace strip) ---

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawEffectLst {
    #[serde(rename = "outerShdw")]
    pub outer_shdw: Option<RawOuterShdw>,
    #[serde(rename = "innerShdw")]
    pub inner_shdw: Option<RawInnerShdw>,
    pub glow: Option<RawGlow>,
    #[serde(rename = "softEdge")]
    pub soft_edge: Option<RawSoftEdge>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawOuterShdw {
    #[serde(rename = "@blurRad")]
    pub blur_rad: Option<i64>,
    #[serde(rename = "@dist")]
    pub dist: Option<i64>,
    #[serde(rename = "@dir")]
    pub dir: Option<i64>,
    #[serde(rename = "@algn")]
    pub algn: Option<String>,
    #[serde(rename = "@rotWithShape")]
    pub rot_with_shape: Option<String>,
    #[serde(flatten)]
    pub color: RawColorChoice,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawInnerShdw {
    #[serde(rename = "@blurRad")]
    pub blur_rad: Option<i64>,
    #[serde(rename = "@dist")]
    pub dist: Option<i64>,
    #[serde(rename = "@dir")]
    pub dir: Option<i64>,
    #[serde(flatten)]
    pub color: RawColorChoice,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawGlow {
    #[serde(rename = "@rad")]
    pub rad: Option<i64>,
    #[serde(flatten)]
    pub color: RawColorChoice,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawSoftEdge {
    #[serde(rename = "@rad")]
    pub rad: Option<i64>,
}
