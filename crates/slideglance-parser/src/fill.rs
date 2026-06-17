//! `<a:*Fill>` and `<a:ln>` outline parser.
//!
//! Mirrors.

use std::collections::BTreeMap;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use serde::Deserialize;
use slideglance_color::{ColorResolver, ResolvedColor};
use slideglance_model::{
    ArrowEndpoint, ArrowSize, ArrowType, DashStyle, Fill, GradientFill, GradientStop, GradientType,
    ImageFill, ImageFillTile, ImageFlip, ImageRect, LineCap, LineJoin, NoFill, Outline,
    OutlineFill, PatternFill, SolidFill,
};
use slideglance_utils::Emu;

use crate::archive::PptxArchive;
use crate::raw_color::RawColorChoice;
use crate::relationships::{resolve_relationship_target, Relationship};
use crate::xml::{parse_xml, XmlError};

const ROTATION_UNIT_PER_DEGREE: f64 = 60_000.0;
const DEFAULT_OUTLINE_WIDTH: i64 = 12_700;
const FRACTION_DIVISOR: f64 = 100_000.0;

/// Mutable context passed alongside a fill node, supplying the archive
/// pieces only `<a:blipFill>` requires plus the inheritable `groupFill`.
pub struct FillParseContext<'a> {
    /// Part-level `.rels` map for the part containing the fill node.
    pub rels: &'a BTreeMap<String, Relationship>,
    /// The PPTX archive for media lookup. Needs `&mut` because
    /// [`PptxArchive::media`] caches and decompresses on demand.
    pub archive: &'a mut PptxArchive,
    /// The path of the part the fill node lives in (used to resolve relative
    /// relationship targets, e.g. `../media/image1.png`).
    pub base_path: &'a str,
    /// Inherited `<a:grpFill/>` from the enclosing group, if any.
    pub group_fill: Option<&'a Fill>,
}

/// Parses a wrapper element that may contain one of `<a:noFill/>`,
/// `<a:solidFill>`, `<a:gradFill>`, `<a:blipFill>`, `<a:pattFill>`, or
/// `<a:grpFill/>`.
///
/// Returns `Ok(None)` when no fill child is present (or when the present
/// child needs context that isn't supplied — e.g. `blipFill` without
/// `context`).
///
/// # Errors
///
/// Returns [`XmlError`] when the XML is malformed.
pub fn parse_fill(
    xml: &str,
    resolver: &ColorResolver,
    context: Option<&mut FillParseContext<'_>>,
) -> Result<Option<Fill>, XmlError> {
    let raw: RawFillContainer = parse_xml(xml)?;
    Ok(build_fill(&raw, resolver, context))
}

pub(crate) fn build_fill(
    raw: &RawFillContainer,
    resolver: &ColorResolver,
    context: Option<&mut FillParseContext<'_>>,
) -> Option<Fill> {
    if raw.no_fill.is_some() {
        return Some(Fill::None(NoFill {}));
    }
    if let Some(solid) = &raw.solid_fill {
        return build_solid_fill(solid, resolver).map(Fill::Solid);
    }
    if let Some(grad) = &raw.grad_fill {
        return build_gradient_fill(grad, resolver);
    }
    if raw.blip_fill.is_some() {
        if let (Some(blip), Some(ctx)) = (raw.blip_fill.as_ref(), context) {
            return build_blip_fill(blip, ctx).map(Fill::Image);
        }
        return None;
    }
    if let Some(patt) = &raw.patt_fill {
        return build_pattern_fill(patt, resolver).map(Fill::Pattern);
    }
    if raw.grp_fill.is_some() {
        if let Some(ctx) = context {
            return ctx.group_fill.cloned();
        }
    }
    None
}

fn build_solid_fill(node: &RawSolidFill, resolver: &ColorResolver) -> Option<SolidFill> {
    let color = resolve_color_in_choice(&node.color, resolver)?;
    Some(SolidFill { color })
}

fn build_gradient_fill(node: &RawGradFill, resolver: &ColorResolver) -> Option<Fill> {
    let stops = build_gradient_stops(node.gs_lst.as_ref(), resolver)?;
    if let Some(path) = &node.path {
        let (center_x, center_y) = parse_path_center(path);
        return Some(Fill::Gradient(GradientFill {
            stops,
            angle: 0.0,
            gradient_type: GradientType::Radial,
            center_x: Some(center_x),
            center_y: Some(center_y),
        }));
    }
    let angle = node
        .lin
        .as_ref()
        .and_then(|l| l.ang.as_deref())
        .and_then(|s| s.parse::<i64>().ok())
        .map_or(0.0, |a| a as f64 / ROTATION_UNIT_PER_DEGREE);
    Some(Fill::Gradient(GradientFill {
        stops,
        angle,
        gradient_type: GradientType::Linear,
        center_x: None,
        center_y: None,
    }))
}

fn build_gradient_stops(
    gs_lst: Option<&RawGsLst>,
    resolver: &ColorResolver,
) -> Option<Vec<GradientStop>> {
    let lst = gs_lst?;
    let mut out = Vec::with_capacity(lst.gs.len());
    for gs in &lst.gs {
        let position = gs
            .pos
            .as_deref()
            .and_then(|s| s.parse::<i64>().ok())
            .map_or(0.0, |p| p as f64 / FRACTION_DIVISOR);
        if let Some(color) = resolve_color_in_choice(&gs.color, resolver) {
            out.push(GradientStop { position, color });
        }
    }
    Some(out)
}

// Single-letter bindings (l/t/r/b) match the OOXML attribute names
// `<a:fillToRect l="..." t="..." r="..." b="..."/>`.
#[allow(clippy::many_single_char_names)]
fn parse_path_center(path: &RawGradPath) -> (f64, f64) {
    let Some(rect) = &path.fill_to_rect else {
        return (0.5, 0.5);
    };
    let l = parse_rect_attr(rect.l.as_deref());
    let t = parse_rect_attr(rect.t.as_deref());
    let r = parse_rect_attr(rect.r.as_deref());
    let b = parse_rect_attr(rect.b.as_deref());
    let center_x = l.midpoint(FRACTION_DIVISOR - r) / FRACTION_DIVISOR;
    let center_y = t.midpoint(FRACTION_DIVISOR - b) / FRACTION_DIVISOR;
    (center_x, center_y)
}

fn parse_rect_attr(s: Option<&str>) -> f64 {
    s.and_then(|v| v.parse::<i64>().ok())
        .map_or(0.0, |v| v as f64)
}

fn build_blip_fill(node: &RawBlipFill, ctx: &mut FillParseContext<'_>) -> Option<ImageFill> {
    let blip = node.blip.as_ref()?;
    let r_id = blip.embed.as_deref()?;

    let rel = ctx.rels.get(r_id)?;
    let media_path = resolve_relationship_target(ctx.base_path, &rel.target);
    let media_data = ctx.archive.media(&media_path).ok().flatten()?.to_vec();

    let mime_type = mime_for_path(&media_path);
    let image_data = BASE64_STANDARD.encode(&media_data);

    let tile = node.tile.as_ref().map(build_tile);
    let src_rect = parse_rect_fractions(node.src_rect.as_ref());
    let stretch = node.stretch.as_ref().map(|s| {
        parse_rect_fractions(s.fill_rect.as_ref()).unwrap_or(ImageRect {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        })
    });

    Some(ImageFill {
        image_data,
        mime_type,
        tile,
        src_rect,
        stretch,
        alpha: blip.alpha(),
    })
}

fn build_tile(node: &RawTile) -> ImageFillTile {
    ImageFillTile {
        tx: Emu::new(parse_int_attr(node.tx.as_deref(), 0)),
        ty: Emu::new(parse_int_attr(node.ty.as_deref(), 0)),
        sx: parse_int_attr(node.sx.as_deref(), 100_000) as f64 / FRACTION_DIVISOR,
        sy: parse_int_attr(node.sy.as_deref(), 100_000) as f64 / FRACTION_DIVISOR,
        flip: parse_image_flip(node.flip.as_deref()),
        align: node.algn.clone().unwrap_or_else(|| "tl".to_owned()),
    }
}

fn parse_int_attr(s: Option<&str>, default: i64) -> i64 {
    s.and_then(|v| v.parse::<i64>().ok()).unwrap_or(default)
}

fn parse_image_flip(value: Option<&str>) -> ImageFlip {
    match value {
        Some("x") => ImageFlip::X,
        Some("y") => ImageFlip::Y,
        Some("xy") => ImageFlip::Xy,
        _ => ImageFlip::None,
    }
}

/// Parses OOXML l/t/r/b 1000-th-percent attributes (e.g. `<a:srcRect l="5000"/>`)
/// into 0-1 fractions. Returns `None` when the node is absent or all zeros so
/// callers can omit the field entirely (matches TS behavior).
//
// Single-letter bindings match the OOXML attribute names.
#[allow(clippy::many_single_char_names)]
fn parse_rect_fractions(node: Option<&RawRect>) -> Option<ImageRect> {
    let n = node?;
    let l = parse_int_attr(n.l.as_deref(), 0) as f64 / FRACTION_DIVISOR;
    let t = parse_int_attr(n.t.as_deref(), 0) as f64 / FRACTION_DIVISOR;
    let r = parse_int_attr(n.r.as_deref(), 0) as f64 / FRACTION_DIVISOR;
    let b = parse_int_attr(n.b.as_deref(), 0) as f64 / FRACTION_DIVISOR;
    if l == 0.0 && t == 0.0 && r == 0.0 && b == 0.0 {
        return None;
    }
    Some(ImageRect {
        left: l,
        top: t,
        right: r,
        bottom: b,
    })
}

fn mime_for_path(path: &str) -> String {
    let ext = path
        .rsplit('.')
        .next()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "emf" => "image/emf",
        "wmf" => "image/wmf",
        // PPTX defaults to PNG for unknown extensions per the spec.
        _ => "image/png",
    }
    .to_owned()
}

fn build_pattern_fill(node: &RawPattFill, resolver: &ColorResolver) -> Option<PatternFill> {
    let preset = node.prst.clone().unwrap_or_else(|| "ltDnDiag".to_owned());
    let foreground = resolve_color_in_choice(node.fg_clr.as_ref()?, resolver)?;
    let background = resolve_color_in_choice(node.bg_clr.as_ref()?, resolver)?;
    Some(PatternFill {
        preset,
        foreground_color: foreground,
        background_color: background,
    })
}

fn resolve_color_in_choice(
    choice: &RawColorChoice,
    resolver: &ColorResolver,
) -> Option<ResolvedColor> {
    let color_ref = choice.to_color_ref()?;
    Some(resolver.resolve(&color_ref))
}

/// Parses an `<a:ln>` outline.
///
/// Returns `Ok(None)` when the node is absent or marked `<a:noFill/>`.
///
/// # Errors
///
/// Returns [`XmlError`] when the XML is malformed.
pub fn parse_outline(xml: &str, resolver: &ColorResolver) -> Result<Option<Outline>, XmlError> {
    let raw: RawOutline = parse_xml(xml)?;
    Ok(build_outline(&raw, resolver))
}

pub(crate) fn build_outline(raw: &RawOutline, resolver: &ColorResolver) -> Option<Outline> {
    if raw.no_fill.is_some() {
        return None;
    }
    let width = Emu::new(parse_int_attr(raw.w.as_deref(), DEFAULT_OUTLINE_WIDTH));

    let fill = if let Some(solid) = &raw.solid_fill {
        build_solid_fill(solid, resolver).map(OutlineFill::Solid)
    } else if let Some(grad) = &raw.grad_fill {
        match build_gradient_fill(grad, resolver) {
            Some(Fill::Gradient(g)) => Some(OutlineFill::Gradient(g)),
            _ => None,
        }
    } else {
        None
    };

    let dash_style = raw
        .prst_dash
        .as_ref()
        .and_then(|d| d.val.as_deref())
        .map_or(DashStyle::Solid, parse_dash_style);

    let custom_dash = build_custom_dash(raw.cust_dash.as_ref());
    let line_cap = raw.cap.as_deref().and_then(parse_line_cap);
    let line_join = parse_line_join(raw);
    let head_end = build_arrow_endpoint(raw.head_end.as_ref());
    let tail_end = build_arrow_endpoint(raw.tail_end.as_ref());

    Some(Outline {
        width,
        fill,
        dash_style,
        custom_dash,
        line_cap,
        line_join,
        head_end,
        tail_end,
    })
}

fn parse_dash_style(s: &str) -> DashStyle {
    match s {
        "dash" => DashStyle::Dash,
        "dot" => DashStyle::Dot,
        "dashDot" => DashStyle::DashDot,
        "lgDash" => DashStyle::LgDash,
        "lgDashDot" => DashStyle::LgDashDot,
        "sysDash" => DashStyle::SysDash,
        "sysDot" => DashStyle::SysDot,
        _ => DashStyle::Solid,
    }
}

fn build_custom_dash(node: Option<&RawCustDash>) -> Option<Vec<f64>> {
    let n = node?;
    if n.ds.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(n.ds.len() * 2);
    for ds in &n.ds {
        let d = parse_int_attr(ds.d.as_deref(), 100_000) as f64 / FRACTION_DIVISOR;
        let sp = parse_int_attr(ds.sp.as_deref(), 100_000) as f64 / FRACTION_DIVISOR;
        out.push(d);
        out.push(sp);
    }
    Some(out)
}

fn parse_line_cap(s: &str) -> Option<LineCap> {
    match s {
        "flat" => Some(LineCap::Butt),
        "sq" => Some(LineCap::Square),
        "rnd" => Some(LineCap::Round),
        _ => None,
    }
}

fn parse_line_join(raw: &RawOutline) -> Option<LineJoin> {
    if raw.round.is_some() {
        return Some(LineJoin::Round);
    }
    if raw.bevel.is_some() {
        return Some(LineJoin::Bevel);
    }
    if raw.miter.is_some() {
        return Some(LineJoin::Miter);
    }
    None
}

fn build_arrow_endpoint(node: Option<&RawArrowEndpoint>) -> Option<ArrowEndpoint> {
    let n = node?;
    let ty = parse_arrow_type(n.ty.as_deref().unwrap_or("none"));
    if matches!(ty, ArrowType::None) {
        return None;
    }
    Some(ArrowEndpoint {
        ty,
        width: parse_arrow_size(n.w.as_deref()),
        length: parse_arrow_size(n.len.as_deref()),
    })
}

fn parse_arrow_type(s: &str) -> ArrowType {
    match s {
        "triangle" => ArrowType::Triangle,
        "stealth" => ArrowType::Stealth,
        "diamond" => ArrowType::Diamond,
        "oval" => ArrowType::Oval,
        "arrow" => ArrowType::Arrow,
        _ => ArrowType::None,
    }
}

fn parse_arrow_size(s: Option<&str>) -> ArrowSize {
    match s {
        Some("sm") => ArrowSize::Sm,
        Some("lg") => ArrowSize::Lg,
        _ => ArrowSize::Med,
    }
}

// --- raw XML shapes (post namespace strip) ---

// Every field ends in `_fill` because the OOXML element names are
// `noFill` / `solidFill` / `gradFill` / `blipFill` / `pattFill` / `grpFill`
// — the suffix is the source-XML mapping the deserializer relies on.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Default, Clone, Deserialize)]
pub(crate) struct RawFillContainer {
    #[serde(rename = "noFill")]
    pub no_fill: Option<EmptyMarker>,
    #[serde(rename = "solidFill")]
    pub solid_fill: Option<RawSolidFill>,
    #[serde(rename = "gradFill")]
    pub grad_fill: Option<RawGradFill>,
    #[serde(rename = "blipFill")]
    pub blip_fill: Option<RawBlipFill>,
    #[serde(rename = "pattFill")]
    pub patt_fill: Option<RawPattFill>,
    #[serde(rename = "grpFill")]
    pub grp_fill: Option<EmptyMarker>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub(crate) struct EmptyMarker {}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawSolidFill {
    #[serde(flatten)]
    pub color: RawColorChoice,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub(crate) struct RawGradFill {
    #[serde(rename = "gsLst")]
    pub gs_lst: Option<RawGsLst>,
    pub lin: Option<RawLin>,
    pub path: Option<RawGradPath>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawGsLst {
    #[serde(default)]
    pub gs: Vec<RawGs>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawGs {
    #[serde(rename = "@pos")]
    pub pos: Option<String>,
    #[serde(flatten)]
    pub color: RawColorChoice,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawLin {
    #[serde(rename = "@ang")]
    pub ang: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawGradPath {
    #[serde(rename = "fillToRect")]
    pub fill_to_rect: Option<RawRect>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawRect {
    #[serde(rename = "@l")]
    pub l: Option<String>,
    #[serde(rename = "@t")]
    pub t: Option<String>,
    #[serde(rename = "@r")]
    pub r: Option<String>,
    #[serde(rename = "@b")]
    pub b: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawBlipFill {
    pub blip: Option<RawBlip>,
    pub tile: Option<RawTile>,
    #[serde(rename = "srcRect")]
    pub src_rect: Option<RawRect>,
    pub stretch: Option<RawStretch>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawBlip {
    /// `<a:blip r:embed="rIdN"/>` — namespace stripping collapses the
    /// `r:embed` attribute to the local part `embed`.
    #[serde(rename = "@embed")]
    pub embed: Option<String>,
    /// `<a:alphaModFix amt="..."/>` — fixes the image alpha to a specific
    /// percentage. `amt` is `ST_PositiveFixedPercentage` (0–100000 ⇒
    /// 0–100%). Missing element / missing `amt` defaults to 100%.
    #[serde(rename = "alphaModFix")]
    pub alpha_mod_fix: Option<RawAlphaModFix>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct RawAlphaModFix {
    #[serde(rename = "@amt")]
    pub amt: Option<String>,
}

impl RawBlip {
    /// Resolves the alpha multiplier from `<a:alphaModFix amt>`. Defaults
    /// to `1.0` (fully opaque) when the element or `amt` is missing.
    /// `amt` is clamped to `[0, 100000]` to guard against malformed inputs.
    pub(crate) fn alpha(&self) -> f64 {
        let Some(node) = self.alpha_mod_fix.as_ref() else {
            return 1.0;
        };
        let Some(raw) = node.amt.as_deref().and_then(|s| s.parse::<i64>().ok()) else {
            return 1.0;
        };
        (raw.clamp(0, 100_000) as f64) / FRACTION_DIVISOR
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawTile {
    #[serde(rename = "@tx")]
    pub tx: Option<String>,
    #[serde(rename = "@ty")]
    pub ty: Option<String>,
    #[serde(rename = "@sx")]
    pub sx: Option<String>,
    #[serde(rename = "@sy")]
    pub sy: Option<String>,
    #[serde(rename = "@flip")]
    pub flip: Option<String>,
    #[serde(rename = "@algn")]
    pub algn: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawStretch {
    #[serde(rename = "fillRect")]
    pub fill_rect: Option<RawRect>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawPattFill {
    #[serde(rename = "@prst")]
    pub prst: Option<String>,
    #[serde(rename = "fgClr")]
    pub fg_clr: Option<RawColorChoice>,
    #[serde(rename = "bgClr")]
    pub bg_clr: Option<RawColorChoice>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub(crate) struct RawOutline {
    #[serde(rename = "@w")]
    pub w: Option<String>,
    #[serde(rename = "@cap")]
    pub cap: Option<String>,
    #[serde(rename = "noFill")]
    pub no_fill: Option<EmptyMarker>,
    #[serde(rename = "solidFill")]
    pub solid_fill: Option<RawSolidFill>,
    #[serde(rename = "gradFill")]
    pub grad_fill: Option<RawGradFill>,
    #[serde(rename = "prstDash")]
    pub prst_dash: Option<RawPrstDash>,
    #[serde(rename = "custDash")]
    pub cust_dash: Option<RawCustDash>,
    pub round: Option<EmptyMarker>,
    pub bevel: Option<EmptyMarker>,
    pub miter: Option<EmptyMarker>,
    #[serde(rename = "headEnd")]
    pub head_end: Option<RawArrowEndpoint>,
    #[serde(rename = "tailEnd")]
    pub tail_end: Option<RawArrowEndpoint>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawPrstDash {
    #[serde(rename = "@val")]
    pub val: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawCustDash {
    #[serde(default)]
    pub ds: Vec<RawDs>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawDs {
    #[serde(rename = "@d")]
    pub d: Option<String>,
    #[serde(rename = "@sp")]
    pub sp: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawArrowEndpoint {
    #[serde(rename = "@type")]
    pub ty: Option<String>,
    #[serde(rename = "@w")]
    pub w: Option<String>,
    #[serde(rename = "@len")]
    pub len: Option<String>,
}
