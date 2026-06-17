//! Line / outline types: width, dash, caps, joins, arrow endpoints.

use slideglance_utils::Emu;

use crate::fill::{GradientFill, SolidFill};

/// `<a:ln>` outline — solid or gradient stroke with optional caps/dash/arrows.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Outline {
    /// Stroke width (EMU; `<a:ln @w>`).
    pub width: Emu,
    /// Stroke color: solid or gradient. `None` = `<a:noFill/>` outline.
    pub fill: Option<OutlineFill>,
    /// Dash style (`<a:prstDash @val>`).
    pub dash_style: DashStyle,
    /// `<a:custDash>` dash segments (lengths in stroke-width multiples).
    pub custom_dash: Option<Vec<f64>>,
    /// `<a:ln @cap>`.
    pub line_cap: Option<LineCap>,
    /// `<a:ln @cmpd>` — outline join style (mitered / round / bevelled).
    pub line_join: Option<LineJoin>,
    /// Head endpoint arrow.
    pub head_end: Option<ArrowEndpoint>,
    /// Tail endpoint arrow.
    pub tail_end: Option<ArrowEndpoint>,
}

/// Subset of [`crate::fill::Fill`] valid for an outline (no image / pattern).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type", rename_all = "lowercase")
)]
pub enum OutlineFill {
    /// Solid stroke color.
    Solid(SolidFill),
    /// Gradient stroke.
    Gradient(GradientFill),
}

/// `<a:prstDash @val>` enum.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum DashStyle {
    /// Solid (no dashes).
    #[default]
    Solid,
    /// Standard dash.
    Dash,
    /// Dot.
    Dot,
    /// Dash-dot.
    DashDot,
    /// Long dash.
    LgDash,
    /// Long dash + dot.
    LgDashDot,
    /// System dash (renderer-dependent length).
    SysDash,
    /// System dot.
    SysDot,
}

/// `<a:ln @cap>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum LineCap {
    /// Butt cap.
    Butt,
    /// Round cap.
    Round,
    /// Square cap.
    Square,
}

/// `<a:ln @cmpd>`-derived join style.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum LineJoin {
    /// Mitered join.
    Miter,
    /// Round join.
    Round,
    /// Beveled join.
    Bevel,
}

/// Arrow endpoint geometry — type, width, length.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ArrowEndpoint {
    /// Arrow head shape.
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: ArrowType,
    /// Arrow width category.
    pub width: ArrowSize,
    /// Arrow length category.
    pub length: ArrowSize,
}

/// `<a:headEnd|tailEnd @type>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum ArrowType {
    /// No arrow head.
    #[default]
    None,
    /// Filled triangle.
    Triangle,
    /// Stealth (concave triangle).
    Stealth,
    /// Diamond.
    Diamond,
    /// Oval.
    Oval,
    /// Open arrow.
    Arrow,
}

/// `<a:headEnd|tailEnd @w/@len>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum ArrowSize {
    /// Small.
    Sm,
    /// Medium.
    #[default]
    Med,
    /// Large.
    Lg,
}
