//! Chart element (chart-bearing `<p:graphicFrame>`).

use slideglance_color::ResolvedColor;

use crate::shape::Transform;

/// Chart-bearing graphic frame.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChartElement {
    /// `<p:cNvPr @id>` — slide-scoped unique identifier from the source PPTX.
    /// `None` for legacy decks that omit the attribute.
    pub sp_id: Option<u32>,
    /// Position / size / rotation.
    pub transform: Transform,
    /// Chart payload.
    pub chart: ChartData,
    /// `<p:cNvPr @name>`.
    pub object_name: Option<String>,
    /// `<p:cNvPr @hidden>`.
    pub hidden: bool,
}

/// Top-level chart kind. Maps OOXML `<c:*Chart>` element names.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum ChartType {
    /// `<c:barChart>` (vertical or horizontal — see [`ChartData::bar_direction`]).
    Bar,
    /// `<c:lineChart>`.
    Line,
    /// `<c:pieChart>`.
    Pie,
    /// `<c:doughnutChart>`.
    Doughnut,
    /// `<c:scatterChart>`.
    Scatter,
    /// `<c:bubbleChart>`.
    Bubble,
    /// `<c:areaChart>`.
    Area,
    /// `<c:radarChart>`.
    Radar,
    /// `<c:stockChart>`.
    Stock,
    /// `<c:surfaceChart>`.
    Surface,
    /// `<c:ofPieChart>`.
    OfPie,
}

/// `<c:chart>` payload.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChartData {
    /// Dominant chart type (or first sub-chart in a combo).
    pub chart_type: ChartType,
    /// `<c:title>` plain text, if any.
    pub title: Option<String>,
    /// Series in source order.
    pub series: Vec<ChartSeries>,
    /// Category labels (X axis values for bar/line/area).
    pub categories: Vec<String>,
    /// `<c:barDir>` — `"col"` or `"bar"`.
    pub bar_direction: Option<BarDirection>,
    /// Doughnut hole size (`<c:holeSize>` percent).
    pub hole_size: Option<f64>,
    /// `<c:radarStyle>`.
    pub radar_style: Option<RadarStyle>,
    /// `<c:ofPieType>`.
    pub of_pie_type: Option<OfPieType>,
    /// `<c:secondPieSize>`.
    pub second_pie_size: Option<f64>,
    /// `<c:splitPos>` for `ofPie`.
    pub split_pos: Option<f64>,
    /// `<c:legend>` block, if present.
    pub legend: Option<ChartLegend>,
    /// Category (X for bar/line) axis configuration.
    pub category_axis: Option<ChartAxis>,
    /// Value (Y) axis configuration.
    pub value_axis: Option<ChartAxis>,
    /// Optional secondary value axis (right side) for combo charts.
    pub secondary_value_axis: Option<ChartAxis>,
    /// Plot-level (default) data label settings inherited by all series.
    pub data_labels: Option<ChartDataLabels>,
    /// `true` when the chart contains multiple sub-chart types (e.g.
    /// bar + line). Per-series dispatch via [`ChartSeries::sub_chart_type`].
    pub is_combo: bool,
}

/// `<c:barDir>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum BarDirection {
    /// Vertical columns.
    Col,
    /// Horizontal bars.
    Bar,
}

/// `<c:radarStyle>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum RadarStyle {
    /// Standard radar (line only).
    Standard,
    /// Marker-only radar.
    Marker,
    /// Filled radar.
    Filled,
}

/// `<c:ofPieType>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum OfPieType {
    /// Pie-of-pie.
    Pie,
    /// Bar-of-pie.
    Bar,
}

/// One axis (`<c:catAx>` / `<c:valAx>` / secondary `<c:valAx>`).
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChartAxis {
    /// Optional axis title text.
    pub title: Option<String>,
    /// Whether major gridlines should be drawn perpendicular to this axis.
    pub major_gridlines: Option<bool>,
    /// Whether minor gridlines should be drawn. Currently rendered like major.
    pub minor_gridlines: Option<bool>,
    /// OOXML major tick mark style. Default `Out`.
    pub major_tick_mark: Option<TickMark>,
}

/// `<c:majorTickMark>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum TickMark {
    /// No tick marks.
    None,
    /// Inside the plot area.
    In,
    /// Outside the plot area.
    #[default]
    Out,
    /// Crossing the axis.
    Cross,
}

/// One data series (`<c:ser>`).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChartSeries {
    /// `<c:tx>` series name (plain text or formula result).
    pub name: Option<String>,
    /// Numeric values (`<c:val><c:numRef>` evaluated).
    pub values: Vec<f64>,
    /// X values (scatter / bubble).
    pub x_values: Option<Vec<f64>>,
    /// Bubble sizes.
    pub bubble_sizes: Option<Vec<f64>>,
    /// Default series color.
    pub color: ResolvedColor,
    /// Per-series data label settings (overrides chart-level).
    pub data_labels: Option<ChartDataLabels>,
    /// Trendline overlays for this series.
    pub trendlines: Option<Vec<ChartTrendline>>,
    /// Sub-chart type when the parent chart is a combo.
    pub sub_chart_type: Option<ChartType>,
    /// Whether the series belongs to the secondary value axis.
    pub axis_group: Option<AxisGroup>,
    /// `<c:smooth val="1"/>`.
    pub smooth: Option<bool>,
    /// Per-data-point overrides (color / explosion). Sparse, indexed by `idx`.
    pub data_points: Option<Vec<ChartDataPoint>>,
    /// Pie/Doughnut explosion percentage applied to all slices.
    pub explosion: Option<f64>,
}

/// `<c:axisGroup>` (we use it as a primary/secondary discriminator).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum AxisGroup {
    /// Primary value axis (default).
    #[default]
    Primary,
    /// Secondary value axis.
    Secondary,
}

/// `<c:dPt>` per-point override.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChartDataPoint {
    /// `<c:idx val>` — zero-based data point index in the series.
    pub idx: u32,
    /// Optional point color override.
    pub color: Option<ResolvedColor>,
    /// Pie/Doughnut explosion percentage for this slice.
    pub explosion: Option<f64>,
}

/// `<c:dLbls>` data label settings.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChartDataLabels {
    /// `<c:showVal val="1"/>`.
    pub show_value: Option<bool>,
    /// `<c:showCatName>`.
    pub show_category_name: Option<bool>,
    /// `<c:showSerName>`.
    pub show_series_name: Option<bool>,
    /// `<c:showPercent>`.
    pub show_percent: Option<bool>,
    /// `<c:showLegendKey>`.
    pub show_legend_key: Option<bool>,
    /// `<c:dLblPos val>`: `t/b/l/r/ctr/inEnd/outEnd/bestFit/inBase`.
    pub position: Option<String>,
    /// Text/value separator (default `", "`).
    pub separator: Option<String>,
}

/// `<c:trendline>`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChartTrendline {
    /// Trendline curve type.
    pub trendline_type: TrendlineType,
    /// Window size for moving averages.
    pub period: Option<u32>,
    /// Optional custom name.
    pub name: Option<String>,
}

/// `<c:trendlineType>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum TrendlineType {
    /// Linear regression.
    Linear,
    /// Exponential.
    Exp,
    /// Logarithmic.
    Log,
    /// Polynomial.
    Poly,
    /// Power.
    Power,
    /// Moving average.
    MovingAvg,
}

/// `<c:legend>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChartLegend {
    /// Legend position.
    pub position: LegendPosition,
}

/// `<c:legendPos val>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum LegendPosition {
    /// Bottom.
    B,
    /// Top.
    T,
    /// Left.
    L,
    /// Right.
    #[default]
    R,
    /// Top-right (legacy).
    Tr,
}
