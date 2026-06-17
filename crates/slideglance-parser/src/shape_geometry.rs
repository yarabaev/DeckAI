//! Shape `<a:xfrm>` and `<a:prstGeom>` / `<a:custGeom>` parsing helpers.
//!
//! Mirrors and
//! `parseGeometry`. Extracted so slide-master / slide-layout placeholder
//! parsing can resolve geometry without depending on the full slide-parser.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;
use slideglance_model::{CustomGeometry, Geometry, PresetGeometry, Transform};
use slideglance_utils::Emu;

use crate::custom_geometry::{build_custom_geometry, RawCustGeom};
use crate::xml::{parse_xml, XmlError};

/// Parses an `<a:xfrm>` body into a [`Transform`]. Returns `Ok(None)` when
/// the xfrm is missing `<a:off>` or `<a:ext>` (TS parity).
///
/// # Errors
///
/// Returns [`XmlError`] when the input is not well-formed XML.
pub fn parse_transform(xml: &str) -> Result<Option<Transform>, XmlError> {
    let raw: RawXfrm = parse_xml(xml)?;
    Ok(build_transform(&raw))
}

pub(crate) fn build_transform(raw: &RawXfrm) -> Option<Transform> {
    let off = raw.off.as_ref()?;
    let ext = raw.ext.as_ref()?;
    Some(Transform {
        offset_x: Emu::new(parse_attr_i64(off.x.as_deref(), 0)),
        offset_y: Emu::new(parse_attr_i64(off.y.as_deref(), 0)),
        extent_width: Emu::new(parse_attr_i64(ext.cx.as_deref(), 0)),
        extent_height: Emu::new(parse_attr_i64(ext.cy.as_deref(), 0)),
        rotation: parse_attr_i64(raw.rot.as_deref(), 0) as f64 / 60_000.0,
        flip_h: raw.flip_h.as_deref().is_some_and(parse_bool_attr),
        flip_v: raw.flip_v.as_deref().is_some_and(parse_bool_attr),
    })
}

/// Parses an `<a:spPr>` body into a [`Geometry`]. Falls back to
/// `Geometry::Preset { preset: "rect", adjust_values: {} }` if neither
/// `<a:prstGeom>` nor `<a:custGeom>` is present.
///
/// # Errors
///
/// Returns [`XmlError`] when the input is not well-formed XML.
pub fn parse_geometry(xml: &str) -> Result<Geometry, XmlError> {
    let raw: RawSpPr = parse_xml(xml)?;
    Ok(build_geometry(&raw))
}

pub(crate) fn build_geometry(raw: &RawSpPr) -> Geometry {
    build_geometry_parts(raw.prst_geom.as_ref(), raw.cust_geom.as_ref())
}

/// Build a [`Geometry`] from the optional `<a:prstGeom>` and `<a:custGeom>`
/// children of any `<a:spPr>`-shaped node. Used by callers (slide.rs) that
/// own the parts inside a larger struct and don't want to clone them into a
/// transient [`RawSpPr`].
pub(crate) fn build_geometry_parts(
    prst_geom: Option<&RawPrstGeom>,
    cust_geom: Option<&RawCustGeom>,
) -> Geometry {
    if let Some(prst) = prst_geom {
        let preset = prst.prst.clone().unwrap_or_else(|| "rect".to_owned());
        let mut adjust_values: BTreeMap<String, f64> = BTreeMap::new();
        if let Some(av_lst) = &prst.av_lst {
            for gd in &av_lst.gd {
                let (Some(name), Some(fmla)) = (&gd.name, &gd.fmla) else {
                    continue;
                };
                if let Some(value) = extract_val_from_formula(fmla) {
                    adjust_values.insert(name.clone(), value);
                }
            }
        }
        return Geometry::Preset(PresetGeometry {
            preset,
            adjust_values,
        });
    }
    if let Some(cust) = cust_geom {
        if let Some(paths) = build_custom_geometry(cust) {
            return Geometry::Custom(CustomGeometry { paths });
        }
    }
    Geometry::Preset(PresetGeometry {
        preset: "rect".to_owned(),
        adjust_values: BTreeMap::new(),
    })
}

fn extract_val_from_formula(fmla: &str) -> Option<f64> {
    static FMLA_VAL_RE: OnceLock<Regex> = OnceLock::new();
    let re = FMLA_VAL_RE.get_or_init(|| Regex::new(r"val\s+(\d+)").expect("valid"));
    let caps = re.captures(fmla)?;
    caps.get(1)?.as_str().parse::<f64>().ok()
}

fn parse_attr_i64(s: Option<&str>, default: i64) -> i64 {
    s.and_then(|v| v.parse::<i64>().ok()).unwrap_or(default)
}

fn parse_bool_attr(s: &str) -> bool {
    s == "1" || s == "true"
}

// --- raw XML shapes ---

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawXfrm {
    #[serde(rename = "@rot")]
    pub rot: Option<String>,
    #[serde(rename = "@flipH")]
    pub flip_h: Option<String>,
    #[serde(rename = "@flipV")]
    pub flip_v: Option<String>,
    pub off: Option<RawOffset>,
    pub ext: Option<RawExtent>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawOffset {
    #[serde(rename = "@x")]
    pub x: Option<String>,
    #[serde(rename = "@y")]
    pub y: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawExtent {
    #[serde(rename = "@cx")]
    pub cx: Option<String>,
    #[serde(rename = "@cy")]
    pub cy: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawSpPr {
    /// Reserved for future shape-tree integration when callers want xfrm and
    /// geometry from the same `<a:spPr>` deserialization in one pass.
    #[allow(dead_code)]
    pub xfrm: Option<RawXfrm>,
    #[serde(rename = "prstGeom")]
    pub prst_geom: Option<RawPrstGeom>,
    #[serde(rename = "custGeom")]
    pub cust_geom: Option<RawCustGeom>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawPrstGeom {
    #[serde(rename = "@prst")]
    pub prst: Option<String>,
    #[serde(rename = "avLst")]
    pub av_lst: Option<RawAvLst>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawAvLst {
    #[serde(default)]
    pub gd: Vec<RawGd>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawGd {
    #[serde(rename = "@name")]
    pub name: Option<String>,
    #[serde(rename = "@fmla")]
    pub fmla: Option<String>,
}
