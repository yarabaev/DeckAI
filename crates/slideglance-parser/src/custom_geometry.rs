//! `<a:custGeom>` parser — turns OOXML path commands into SVG path strings.
//!
//! Mirrors.

use std::collections::BTreeMap;
use std::f64::consts::PI;

use serde::Deserialize;
use slideglance_model::CustomGeometryPath;

use crate::geometry_formula::{evaluate_guides, resolve_value, GuideDefinition};
use crate::xml::{parse_xml, XmlError};

/// Parses a `<a:custGeom>` body into a list of resolved
/// [`CustomGeometryPath`] entries.
///
/// Returns `Ok(None)` when the input has no `<a:pathLst><a:path>` content.
///
/// # Errors
///
/// Returns [`XmlError`] when the XML is malformed.
pub fn parse_custom_geometry(xml: &str) -> Result<Option<Vec<CustomGeometryPath>>, XmlError> {
    let raw: RawCustGeom = parse_xml(xml)?;
    Ok(build_custom_geometry(&raw))
}

pub(crate) fn build_custom_geometry(raw: &RawCustGeom) -> Option<Vec<CustomGeometryPath>> {
    let path_lst = raw.path_lst.as_ref()?;
    if path_lst.path.is_empty() {
        return None;
    }
    let av_gd = guide_list(raw.av_lst.as_ref());
    let gd_gd = guide_list(raw.gd_lst.as_ref());

    let mut result = Vec::with_capacity(path_lst.path.len());
    for path in &path_lst.path {
        let w = parse_attr_f64(path.w.as_deref());
        let h = parse_attr_f64(path.h.as_deref());
        if w == 0.0 && h == 0.0 {
            continue;
        }
        let vars = evaluate_guides(&av_gd, &gd_gd, w, h);
        let Some(commands) = build_path_commands(path, &vars) else {
            continue;
        };
        result.push(CustomGeometryPath {
            width: w,
            height: h,
            commands,
        });
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn guide_list(node: Option<&RawGuideList>) -> Vec<GuideDefinition> {
    let Some(list) = node else { return Vec::new() };
    list.gd
        .iter()
        .filter_map(|g| {
            let name = g.name.clone()?;
            let fmla = g.fmla.clone()?;
            if name.is_empty() || fmla.is_empty() {
                return None;
            }
            Some(GuideDefinition { name, fmla })
        })
        .collect()
}

fn parse_attr_f64(s: Option<&str>) -> f64 {
    s.and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0)
}

fn build_path_commands(path: &RawPath, vars: &BTreeMap<String, f64>) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut cur_x = 0.0;
    let mut cur_y = 0.0;
    let mut start_x = 0.0;
    let mut start_y = 0.0;

    for cmd in &path.commands {
        match cmd {
            PathCommand::MoveTo(node) => {
                if let Some(pt) = first_point(&node.pt, vars) {
                    parts.push(format!("M {} {}", pt.x, pt.y));
                    cur_x = pt.x;
                    cur_y = pt.y;
                    start_x = pt.x;
                    start_y = pt.y;
                }
            }
            PathCommand::LnTo(node) => {
                if let Some(pt) = first_point(&node.pt, vars) {
                    parts.push(format!("L {} {}", pt.x, pt.y));
                    cur_x = pt.x;
                    cur_y = pt.y;
                }
            }
            PathCommand::CubicBezTo(node) => {
                let pts = points(&node.pt, vars);
                if pts.len() >= 3 {
                    let body: Vec<String> =
                        pts.iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                    parts.push(format!("C {}", body.join(", ")));
                    if let Some(last) = pts.last() {
                        cur_x = last.x;
                        cur_y = last.y;
                    }
                }
            }
            PathCommand::QuadBezTo(node) => {
                let pts = points(&node.pt, vars);
                if pts.len() >= 2 {
                    let body: Vec<String> =
                        pts.iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                    parts.push(format!("Q {}", body.join(", ")));
                    if let Some(last) = pts.last() {
                        cur_x = last.x;
                        cur_y = last.y;
                    }
                }
            }
            PathCommand::ArcTo(node) => {
                if let Some(arc) = convert_arc_to(node, cur_x, cur_y, vars) {
                    parts.push(arc.svg);
                    cur_x = arc.end_x;
                    cur_y = arc.end_y;
                }
            }
            PathCommand::Close => {
                parts.push("Z".to_owned());
                cur_x = start_x;
                cur_y = start_y;
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

#[derive(Copy, Clone)]
struct ResolvedPoint {
    x: f64,
    y: f64,
}

fn first_point(list: &[RawPoint], vars: &BTreeMap<String, f64>) -> Option<ResolvedPoint> {
    list.first().map(|p| resolve_point(p, vars))
}

fn points(list: &[RawPoint], vars: &BTreeMap<String, f64>) -> Vec<ResolvedPoint> {
    list.iter().map(|p| resolve_point(p, vars)).collect()
}

fn resolve_point(point: &RawPoint, vars: &BTreeMap<String, f64>) -> ResolvedPoint {
    ResolvedPoint {
        x: resolve_value(point.x.as_deref().unwrap_or("0"), vars),
        y: resolve_value(point.y.as_deref().unwrap_or("0"), vars),
    }
}

struct ArcResult {
    svg: String,
    end_x: f64,
    end_y: f64,
}

// Local bindings spell out OOXML attribute names (`stAng`, `swAng`, `wR`,
// `hR`); the resulting names are similar by design.
#[allow(clippy::similar_names)]
fn convert_arc_to(
    arc: &RawArcTo,
    cur_x: f64,
    cur_y: f64,
    vars: &BTreeMap<String, f64>,
) -> Option<ArcResult> {
    let w_r = resolve_value(arc.w_r.as_deref().unwrap_or("0"), vars);
    let h_r = resolve_value(arc.h_r.as_deref().unwrap_or("0"), vars);
    let start_ang = resolve_value(arc.st_ang.as_deref().unwrap_or("0"), vars);
    let sweep_ang = resolve_value(arc.sw_ang.as_deref().unwrap_or("0"), vars);

    if w_r == 0.0 && h_r == 0.0 {
        return None;
    }
    if sweep_ang == 0.0 {
        return None;
    }

    let start_deg = start_ang / 60_000.0;
    let sweep_deg = sweep_ang / 60_000.0;
    let start_rad = start_deg * PI / 180.0;
    let end_rad = (start_deg + sweep_deg) * PI / 180.0;

    // Recover the ellipse center from the current point being on the arc at
    // start_rad.
    let cx = cur_x - w_r * start_rad.cos();
    let cy = cur_y - h_r * start_rad.sin();
    let end_x = cx + w_r * end_rad.cos();
    let end_y = cy + h_r * end_rad.sin();

    let large_arc_flag = i32::from(sweep_deg.abs() > 180.0);
    let sweep_flag = i32::from(sweep_deg > 0.0);

    let rx = (w_r * 1000.0).round() / 1000.0;
    let ry = (h_r * 1000.0).round() / 1000.0;
    let ex = (end_x * 1000.0).round() / 1000.0;
    let ey = (end_y * 1000.0).round() / 1000.0;

    Some(ArcResult {
        svg: format!("A {rx} {ry} 0 {large_arc_flag} {sweep_flag} {ex} {ey}"),
        end_x,
        end_y,
    })
}

// --- raw XML shapes ---

// Every field ends in `_lst` because OOXML element names are
// `avLst` / `gdLst` / `pathLst`.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawCustGeom {
    #[serde(rename = "avLst")]
    pub av_lst: Option<RawGuideList>,
    #[serde(rename = "gdLst")]
    pub gd_lst: Option<RawGuideList>,
    #[serde(rename = "pathLst")]
    pub path_lst: Option<RawPathLst>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawGuideList {
    #[serde(default)]
    pub gd: Vec<RawGuide>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawGuide {
    #[serde(rename = "@name")]
    pub name: Option<String>,
    #[serde(rename = "@fmla")]
    pub fmla: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawPathLst {
    #[serde(default)]
    pub path: Vec<RawPath>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawPath {
    #[serde(rename = "@w")]
    pub w: Option<String>,
    #[serde(rename = "@h")]
    pub h: Option<String>,
    #[serde(rename = "$value", default)]
    pub commands: Vec<PathCommand>,
}

/// Source-order list of path commands inside a `<a:path>`. quick-xml's
/// `$value` collects each child element into a Vec preserving order.
#[derive(Debug, Deserialize)]
pub(crate) enum PathCommand {
    #[serde(rename = "moveTo")]
    MoveTo(RawPointHolder),
    #[serde(rename = "lnTo")]
    LnTo(RawPointHolder),
    #[serde(rename = "cubicBezTo")]
    CubicBezTo(RawPointHolder),
    #[serde(rename = "quadBezTo")]
    QuadBezTo(RawPointHolder),
    #[serde(rename = "arcTo")]
    ArcTo(RawArcTo),
    #[serde(rename = "close")]
    Close,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawPointHolder {
    #[serde(default)]
    pub pt: Vec<RawPoint>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawPoint {
    #[serde(rename = "@x")]
    pub x: Option<String>,
    #[serde(rename = "@y")]
    pub y: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawArcTo {
    #[serde(rename = "@wR")]
    pub w_r: Option<String>,
    #[serde(rename = "@hR")]
    pub h_r: Option<String>,
    #[serde(rename = "@stAng")]
    pub st_ang: Option<String>,
    #[serde(rename = "@swAng")]
    pub sw_ang: Option<String>,
}
