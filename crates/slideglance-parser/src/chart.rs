//! `<c:chartSpace>` chart parser.
//!
//! Mirrors.

use serde::Deserialize;
use slideglance_color::{ColorRef, ColorResolver, ColorTransform, ResolvedColor};
use slideglance_model::{
    AxisGroup, BarDirection, ChartAxis, ChartData, ChartDataLabels, ChartDataPoint, ChartLegend,
    ChartSeries, ChartTrendline, ChartType, LegendPosition, OfPieType, RadarStyle, TickMark,
    TrendlineType,
};

use crate::raw_color::RawColorChoice;
use crate::xml::{parse_xml, XmlError};

const ACCENT_KEYS: [&str; 6] = [
    "accent1", "accent2", "accent3", "accent4", "accent5", "accent6",
];

/// Parses a chart XML body into [`ChartData`].
///
/// # Errors
///
/// Returns [`XmlError`] when the input is not well-formed XML.
pub fn parse_chart(xml: &str, resolver: &ColorResolver) -> Result<Option<ChartData>, XmlError> {
    let raw: RawChartSpace = parse_xml(xml)?;
    Ok(build_chart(&raw, resolver))
}

#[allow(clippy::too_many_lines)]
pub(crate) fn build_chart(raw: &RawChartSpace, resolver: &ColorResolver) -> Option<ChartData> {
    let plot = raw.chart.as_ref()?.plot_area.as_ref()?;
    let title = raw
        .chart
        .as_ref()
        .and_then(|c| c.title.as_ref())
        .and_then(parse_title);

    let mut series_index_counter: u32 = 0;
    let mut all_series: Vec<ChartSeries> = Vec::new();
    let mut categories: Vec<String> = Vec::new();
    let mut sub_charts: Vec<(ChartType, &RawSubChart)> = Vec::new();

    for sub in plot.sub_charts() {
        let (xml_tag, chart_type, node) = sub;
        let _ = xml_tag;
        sub_charts.push((chart_type, node));
    }
    if sub_charts.is_empty() {
        return None;
    }
    let dominant_type = sub_charts[0].0;
    let is_combo = sub_charts.len() > 1;

    let mut bar_direction: Option<BarDirection> = None;
    let mut hole_size: Option<f64> = None;
    let mut radar_style: Option<RadarStyle> = None;
    let mut of_pie_type: Option<OfPieType> = None;
    let mut second_pie_size: Option<f64> = None;
    let mut split_pos: Option<f64> = None;
    let mut data_labels: Option<ChartDataLabels> = None;

    for (idx, (chart_type, node)) in sub_charts.iter().enumerate() {
        for ser in &node.ser {
            let mut s = build_series(ser, *chart_type, series_index_counter, resolver);
            if is_combo {
                s.sub_chart_type = Some(*chart_type);
            }
            all_series.push(s);
            series_index_counter += 1;
        }
        if categories.is_empty() {
            categories = extract_categories(&node.ser);
        }
        if idx == 0 {
            if matches!(chart_type, ChartType::Bar) {
                bar_direction = Some(
                    node.bar_dir
                        .as_ref()
                        .and_then(|n| n.val.as_deref())
                        .and_then(parse_bar_direction)
                        .unwrap_or(BarDirection::Col),
                );
            }
            if matches!(chart_type, ChartType::Doughnut) {
                hole_size = Some(parse_attr_f64(
                    node.hole_size.as_ref().and_then(|n| n.val.as_deref()),
                    50.0,
                ));
            }
            if matches!(chart_type, ChartType::Radar) {
                radar_style = Some(
                    node.radar_style
                        .as_ref()
                        .and_then(|n| n.val.as_deref())
                        .and_then(parse_radar_style)
                        .unwrap_or(RadarStyle::Standard),
                );
            }
            if matches!(chart_type, ChartType::OfPie) {
                of_pie_type = Some(
                    node.of_pie_type
                        .as_ref()
                        .and_then(|n| n.val.as_deref())
                        .and_then(parse_of_pie_type)
                        .unwrap_or(OfPieType::Pie),
                );
                second_pie_size = Some(parse_attr_f64(
                    node.second_pie_size.as_ref().and_then(|n| n.val.as_deref()),
                    75.0,
                ));
                split_pos = Some(parse_attr_f64(
                    node.split_pos.as_ref().and_then(|n| n.val.as_deref()),
                    2.0,
                ));
            }
            data_labels = node.d_lbls.as_ref().and_then(build_data_labels);
        }
    }

    // Axes.
    let category_axis = plot.cat_ax.first().and_then(build_axis);
    let value_axis = plot.val_ax.first().and_then(build_axis);
    let secondary_value_axis = plot.val_ax.get(1).and_then(build_axis);

    // Series → axis group (primary / secondary).
    if plot.val_ax.len() >= 2 {
        let primary_id = plot.val_ax[0].ax_id.as_ref().and_then(|n| n.val.clone());
        let secondary_id = plot.val_ax[1].ax_id.as_ref().and_then(|n| n.val.clone());
        if let (Some(pri), Some(sec)) = (primary_id, secondary_id) {
            let mut serial_index: usize = 0;
            for (_, node) in &sub_charts {
                let refs_secondary = node.ax_id.iter().any(|n| {
                    let v = n.val.as_deref();
                    v == Some(sec.as_str())
                });
                let _ = pri.as_str();
                let count = node.ser.len();
                if refs_secondary {
                    for s in &mut all_series[serial_index..serial_index + count] {
                        s.axis_group = Some(AxisGroup::Secondary);
                    }
                } else {
                    for s in &mut all_series[serial_index..serial_index + count] {
                        s.axis_group = Some(AxisGroup::Primary);
                    }
                }
                serial_index += count;
            }
        }
    }

    let legend = raw
        .chart
        .as_ref()
        .and_then(|c| c.legend.as_ref())
        .map(build_legend);

    Some(ChartData {
        chart_type: dominant_type,
        title,
        series: all_series,
        categories,
        bar_direction,
        hole_size,
        radar_style,
        of_pie_type,
        second_pie_size,
        split_pos,
        legend,
        category_axis,
        value_axis,
        secondary_value_axis,
        data_labels,
        is_combo,
    })
}

fn build_series(
    raw: &RawSeries,
    chart_type: ChartType,
    series_index: u32,
    resolver: &ColorResolver,
) -> ChartSeries {
    let name = raw.tx.as_ref().and_then(parse_series_name);
    let uses_xy = matches!(chart_type, ChartType::Scatter | ChartType::Bubble);
    let values = parse_numeric_data(if uses_xy {
        raw.y_val.as_ref()
    } else {
        raw.val.as_ref()
    });
    let x_values = if uses_xy {
        Some(parse_numeric_data(raw.x_val.as_ref()))
    } else {
        None
    };
    let bubble_sizes = if matches!(chart_type, ChartType::Bubble) {
        Some(parse_numeric_data(raw.bubble_size.as_ref()))
    } else {
        None
    };
    let color = resolve_series_color(raw.sp_pr.as_ref(), series_index, resolver);
    let data_labels = raw.d_lbls.as_ref().and_then(build_data_labels);
    let trendlines = build_trendlines(&raw.trendline);
    let smooth = raw
        .smooth
        .as_ref()
        .and_then(|n| n.val.as_deref())
        .map(|v| v != "0");
    let explosion = raw
        .explosion
        .as_ref()
        .and_then(|n| n.val.as_deref())
        .and_then(|v| v.parse::<f64>().ok());
    let data_points = build_data_points(&raw.d_pt, resolver);

    ChartSeries {
        name,
        values,
        x_values,
        bubble_sizes,
        color,
        data_labels,
        trendlines: if trendlines.is_empty() {
            None
        } else {
            Some(trendlines)
        },
        sub_chart_type: None,
        axis_group: None,
        smooth,
        data_points: if data_points.is_empty() {
            None
        } else {
            Some(data_points)
        },
        explosion,
    }
}

fn build_data_points(raw: &[RawDataPoint], resolver: &ColorResolver) -> Vec<ChartDataPoint> {
    raw.iter()
        .filter_map(|dpt| {
            let idx = dpt
                .idx
                .as_ref()
                .and_then(|n| n.val.as_deref())
                .and_then(|v| v.parse::<u32>().ok())?;
            let color = dpt
                .sp_pr
                .as_ref()
                .and_then(|sp| sp.solid_fill.as_ref())
                .and_then(|fill| fill.color.to_color_ref())
                .map(|cr| resolver.resolve(&cr));
            let explosion = dpt
                .explosion
                .as_ref()
                .and_then(|n| n.val.as_deref())
                .and_then(|v| v.parse::<f64>().ok());
            Some(ChartDataPoint {
                idx,
                color,
                explosion,
            })
        })
        .collect()
}

fn build_data_labels(raw: &RawDataLabels) -> Option<ChartDataLabels> {
    let flag = |node: Option<&RawValAttr>| -> Option<bool> {
        let n = node?;
        match n.val.as_deref() {
            Some("0" | "false") => Some(false),
            Some("1" | "true") | None => Some(true),
            Some(_) => None,
        }
    };
    let show_value = flag(raw.show_val.as_ref());
    let show_category_name = flag(raw.show_cat_name.as_ref());
    let show_series_name = flag(raw.show_ser_name.as_ref());
    let show_percent = flag(raw.show_percent.as_ref());
    let show_legend_key = flag(raw.show_legend_key.as_ref());
    let position = raw.d_lbl_pos.as_ref().and_then(|n| n.val.clone());
    let separator = raw.separator.as_ref().and_then(|n| n.text.clone());

    if show_value.is_none()
        && show_category_name.is_none()
        && show_series_name.is_none()
        && show_percent.is_none()
        && show_legend_key.is_none()
        && position.is_none()
        && separator.is_none()
    {
        return None;
    }
    Some(ChartDataLabels {
        show_value,
        show_category_name,
        show_series_name,
        show_percent,
        show_legend_key,
        position,
        separator,
    })
}

fn build_trendlines(raw: &[RawTrendline]) -> Vec<ChartTrendline> {
    raw.iter()
        .map(|t| {
            let trendline_type = t
                .trendline_type
                .as_ref()
                .and_then(|n| n.val.as_deref())
                .and_then(parse_trendline_type)
                .unwrap_or(TrendlineType::Linear);
            let period = t
                .period
                .as_ref()
                .and_then(|n| n.val.as_deref())
                .and_then(|v| v.parse::<u32>().ok());
            let name = t.name.as_ref().and_then(|n| n.text.clone());
            ChartTrendline {
                trendline_type,
                period,
                name,
            }
        })
        .collect()
}

fn parse_series_name(tx: &RawSeriesText) -> Option<String> {
    if let Some(str_ref) = tx.str_ref.as_ref() {
        if let Some(cache) = str_ref.str_cache.as_ref() {
            if let Some(pt) = cache.pt.first() {
                return pt.v.as_ref().and_then(|n| n.text.clone());
            }
        }
    }
    if let Some(v) = tx.v.as_ref().and_then(|n| n.text.clone()) {
        return Some(v);
    }
    None
}

fn parse_numeric_data(val: Option<&RawValueData>) -> Vec<f64> {
    let Some(val) = val else { return Vec::new() };
    let Some(num_ref) = val.num_ref.as_ref() else {
        return Vec::new();
    };
    let Some(cache) = num_ref.num_cache.as_ref() else {
        return Vec::new();
    };
    let mut pts: Vec<&RawDataPointValue> = cache.pt.iter().collect();
    pts.sort_by_key(|pt| {
        pt.idx
            .as_deref()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0)
    });
    pts.iter()
        .map(|pt| {
            pt.v.as_ref()
                .and_then(|n| n.text.as_deref())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0)
        })
        .collect()
}

fn extract_categories(series: &[RawSeries]) -> Vec<String> {
    for s in series {
        let Some(cat) = s.cat.as_ref() else { continue };
        let str_cache = cat
            .str_ref
            .as_ref()
            .and_then(|n| n.str_cache.as_ref())
            .or_else(|| cat.num_ref.as_ref().and_then(|n| n.num_cache.as_ref()));
        if let Some(cache) = str_cache {
            let mut pts: Vec<&RawDataPointValue> = cache.pt.iter().collect();
            pts.sort_by_key(|pt| {
                pt.idx
                    .as_deref()
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(0)
            });
            return pts
                .iter()
                .map(|pt| {
                    pt.v.as_ref()
                        .and_then(|n| n.text.clone())
                        .unwrap_or_default()
                })
                .collect();
        }
        if let Some(multi) = cat.multi_lvl_str_ref.as_ref() {
            if let Some(cache) = multi.multi_lvl_str_cache.as_ref() {
                if let Some(first_lvl) = cache.lvl.first() {
                    let mut pts: Vec<&RawDataPointValue> = first_lvl.pt.iter().collect();
                    pts.sort_by_key(|pt| {
                        pt.idx
                            .as_deref()
                            .and_then(|v| v.parse::<u32>().ok())
                            .unwrap_or(0)
                    });
                    return pts
                        .iter()
                        .map(|pt| {
                            pt.v.as_ref()
                                .and_then(|n| n.text.clone())
                                .unwrap_or_default()
                        })
                        .collect();
                }
            }
        }
    }
    Vec::new()
}

fn resolve_series_color(
    sp_pr: Option<&RawSpPr>,
    series_index: u32,
    resolver: &ColorResolver,
) -> ResolvedColor {
    if let Some(sp) = sp_pr {
        if let Some(fill) = sp.solid_fill.as_ref() {
            if let Some(cr) = fill.color.to_color_ref() {
                return resolver.resolve(&cr);
            }
        }
        if let Some(cr) = sp.color.to_color_ref() {
            return resolver.resolve(&cr);
        }
    }
    let accent = ACCENT_KEYS[(series_index as usize) % ACCENT_KEYS.len()];
    resolver.resolve(&ColorRef::Scheme {
        name: accent.to_owned(),
        transform: ColorTransform::default(),
    })
}

fn parse_title(node: &RawTitle) -> Option<String> {
    let rich = node.tx.as_ref()?.rich.as_ref()?;
    if rich.p.is_empty() {
        return None;
    }
    let mut buf = String::new();
    for paragraph in &rich.p {
        for run in &paragraph.r {
            if let Some(t) = run.t.as_ref().and_then(|n| n.text.as_deref()) {
                buf.push_str(t);
            }
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

fn build_legend(node: &RawLegend) -> ChartLegend {
    let position = node
        .legend_pos
        .as_ref()
        .and_then(|n| n.val.as_deref())
        .and_then(parse_legend_position)
        .unwrap_or(LegendPosition::B);
    ChartLegend { position }
}

fn build_axis(node: &RawAxis) -> Option<ChartAxis> {
    let title = node.title.as_ref().and_then(parse_title);
    let major_gridlines = node.major_gridlines.is_some().then_some(true);
    let minor_gridlines = node.minor_gridlines.is_some().then_some(true);
    let major_tick_mark = node
        .major_tick_mark
        .as_ref()
        .and_then(|n| n.val.as_deref())
        .and_then(parse_tick_mark);
    if title.is_none()
        && major_gridlines.is_none()
        && minor_gridlines.is_none()
        && major_tick_mark.is_none()
    {
        return None;
    }
    Some(ChartAxis {
        title,
        major_gridlines,
        minor_gridlines,
        major_tick_mark,
    })
}

fn parse_bar_direction(s: &str) -> Option<BarDirection> {
    match s {
        "col" => Some(BarDirection::Col),
        "bar" => Some(BarDirection::Bar),
        _ => None,
    }
}

fn parse_radar_style(s: &str) -> Option<RadarStyle> {
    match s {
        "standard" => Some(RadarStyle::Standard),
        "marker" => Some(RadarStyle::Marker),
        "filled" => Some(RadarStyle::Filled),
        _ => None,
    }
}

fn parse_of_pie_type(s: &str) -> Option<OfPieType> {
    match s {
        "pie" => Some(OfPieType::Pie),
        "bar" => Some(OfPieType::Bar),
        _ => None,
    }
}

fn parse_trendline_type(s: &str) -> Option<TrendlineType> {
    match s {
        "linear" => Some(TrendlineType::Linear),
        "exp" => Some(TrendlineType::Exp),
        "log" => Some(TrendlineType::Log),
        "poly" => Some(TrendlineType::Poly),
        "power" => Some(TrendlineType::Power),
        "movingAvg" => Some(TrendlineType::MovingAvg),
        _ => None,
    }
}

fn parse_legend_position(s: &str) -> Option<LegendPosition> {
    match s {
        "b" => Some(LegendPosition::B),
        "t" => Some(LegendPosition::T),
        "l" => Some(LegendPosition::L),
        "r" => Some(LegendPosition::R),
        "tr" => Some(LegendPosition::Tr),
        _ => None,
    }
}

fn parse_tick_mark(s: &str) -> Option<TickMark> {
    match s {
        "none" => Some(TickMark::None),
        "in" => Some(TickMark::In),
        "out" => Some(TickMark::Out),
        "cross" => Some(TickMark::Cross),
        _ => None,
    }
}

fn parse_attr_f64(s: Option<&str>, default: f64) -> f64 {
    s.and_then(|v| v.parse::<f64>().ok()).unwrap_or(default)
}

// --- raw XML shapes ---

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawChartSpace {
    pub chart: Option<RawChartRoot>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawChartRoot {
    pub title: Option<RawTitle>,
    pub legend: Option<RawLegend>,
    #[serde(rename = "plotArea")]
    pub plot_area: Option<RawPlotArea>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawTitle {
    pub tx: Option<RawTxRich>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawTxRich {
    pub rich: Option<RawRichBody>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawRichBody {
    #[serde(default)]
    pub p: Vec<RawRichParagraph>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawRichParagraph {
    #[serde(default)]
    pub r: Vec<RawRichRun>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawRichRun {
    pub t: Option<RawText>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawText {
    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawLegend {
    #[serde(rename = "legendPos")]
    pub legend_pos: Option<RawValAttr>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawPlotArea {
    #[serde(default, rename = "barChart")]
    pub bar_chart: Vec<RawSubChart>,
    #[serde(default, rename = "bar3DChart")]
    pub bar3d_chart: Vec<RawSubChart>,
    #[serde(default, rename = "lineChart")]
    pub line_chart: Vec<RawSubChart>,
    #[serde(default, rename = "line3DChart")]
    pub line3d_chart: Vec<RawSubChart>,
    #[serde(default, rename = "pieChart")]
    pub pie_chart: Vec<RawSubChart>,
    #[serde(default, rename = "pie3DChart")]
    pub pie3d_chart: Vec<RawSubChart>,
    #[serde(default, rename = "doughnutChart")]
    pub doughnut_chart: Vec<RawSubChart>,
    #[serde(default, rename = "scatterChart")]
    pub scatter_chart: Vec<RawSubChart>,
    #[serde(default, rename = "bubbleChart")]
    pub bubble_chart: Vec<RawSubChart>,
    #[serde(default, rename = "areaChart")]
    pub area_chart: Vec<RawSubChart>,
    #[serde(default, rename = "area3DChart")]
    pub area3d_chart: Vec<RawSubChart>,
    #[serde(default, rename = "radarChart")]
    pub radar_chart: Vec<RawSubChart>,
    #[serde(default, rename = "stockChart")]
    pub stock_chart: Vec<RawSubChart>,
    #[serde(default, rename = "surfaceChart")]
    pub surface_chart: Vec<RawSubChart>,
    #[serde(default, rename = "surface3DChart")]
    pub surface3d_chart: Vec<RawSubChart>,
    #[serde(default, rename = "ofPieChart")]
    pub of_pie_chart: Vec<RawSubChart>,
    #[serde(default, rename = "catAx")]
    pub cat_ax: Vec<RawAxis>,
    #[serde(default, rename = "valAx")]
    pub val_ax: Vec<RawAxis>,
}

impl RawPlotArea {
    /// Iterates over every populated sub-chart in `CHART_TYPE_MAP` order.
    fn sub_charts(&self) -> Vec<(&'static str, ChartType, &RawSubChart)> {
        let entries: [(&'static str, ChartType, &Vec<RawSubChart>); 16] = [
            ("barChart", ChartType::Bar, &self.bar_chart),
            ("bar3DChart", ChartType::Bar, &self.bar3d_chart),
            ("lineChart", ChartType::Line, &self.line_chart),
            ("line3DChart", ChartType::Line, &self.line3d_chart),
            ("pieChart", ChartType::Pie, &self.pie_chart),
            ("pie3DChart", ChartType::Pie, &self.pie3d_chart),
            ("doughnutChart", ChartType::Doughnut, &self.doughnut_chart),
            ("scatterChart", ChartType::Scatter, &self.scatter_chart),
            ("bubbleChart", ChartType::Bubble, &self.bubble_chart),
            ("areaChart", ChartType::Area, &self.area_chart),
            ("area3DChart", ChartType::Area, &self.area3d_chart),
            ("radarChart", ChartType::Radar, &self.radar_chart),
            ("stockChart", ChartType::Stock, &self.stock_chart),
            ("surfaceChart", ChartType::Surface, &self.surface_chart),
            ("surface3DChart", ChartType::Surface, &self.surface3d_chart),
            ("ofPieChart", ChartType::OfPie, &self.of_pie_chart),
        ];
        let mut out = Vec::new();
        for (tag, ty, list) in entries {
            for n in list {
                out.push((tag, ty, n));
            }
        }
        out
    }
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawSubChart {
    #[serde(default)]
    pub ser: Vec<RawSeries>,
    #[serde(default, rename = "axId")]
    pub ax_id: Vec<RawValAttr>,
    #[serde(rename = "barDir")]
    pub bar_dir: Option<RawValAttr>,
    #[serde(rename = "holeSize")]
    pub hole_size: Option<RawValAttr>,
    #[serde(rename = "radarStyle")]
    pub radar_style: Option<RawValAttr>,
    #[serde(rename = "ofPieType")]
    pub of_pie_type: Option<RawValAttr>,
    #[serde(rename = "secondPieSize")]
    pub second_pie_size: Option<RawValAttr>,
    #[serde(rename = "splitPos")]
    pub split_pos: Option<RawValAttr>,
    #[serde(rename = "dLbls")]
    pub d_lbls: Option<RawDataLabels>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawSeries {
    pub tx: Option<RawSeriesText>,
    pub val: Option<RawValueData>,
    #[serde(rename = "yVal")]
    pub y_val: Option<RawValueData>,
    #[serde(rename = "xVal")]
    pub x_val: Option<RawValueData>,
    #[serde(rename = "bubbleSize")]
    pub bubble_size: Option<RawValueData>,
    pub cat: Option<RawCategoryData>,
    #[serde(rename = "spPr")]
    pub sp_pr: Option<RawSpPr>,
    #[serde(rename = "dLbls")]
    pub d_lbls: Option<RawDataLabels>,
    #[serde(default)]
    pub trendline: Vec<RawTrendline>,
    pub smooth: Option<RawValAttr>,
    pub explosion: Option<RawValAttr>,
    #[serde(default, rename = "dPt")]
    pub d_pt: Vec<RawDataPoint>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawSeriesText {
    #[serde(rename = "strRef")]
    pub str_ref: Option<RawStrRef>,
    pub v: Option<RawText>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawStrRef {
    #[serde(rename = "strCache")]
    pub str_cache: Option<RawStrCache>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawStrCache {
    #[serde(default)]
    pub pt: Vec<RawDataPointValue>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawDataPointValue {
    #[serde(rename = "@idx")]
    pub idx: Option<String>,
    pub v: Option<RawText>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawValueData {
    #[serde(rename = "numRef")]
    pub num_ref: Option<RawNumRef>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawNumRef {
    #[serde(rename = "numCache")]
    pub num_cache: Option<RawStrCache>,
}

// Every field ends in `_ref` because the OOXML element names are
// `strRef` / `numRef` / `multiLvlStrRef`.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawCategoryData {
    #[serde(rename = "strRef")]
    pub str_ref: Option<RawStrRef>,
    #[serde(rename = "numRef")]
    pub num_ref: Option<RawNumRef>,
    #[serde(rename = "multiLvlStrRef")]
    pub multi_lvl_str_ref: Option<RawMultiLvlStrRef>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawMultiLvlStrRef {
    #[serde(rename = "multiLvlStrCache")]
    pub multi_lvl_str_cache: Option<RawMultiLvlCache>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawMultiLvlCache {
    #[serde(default)]
    pub lvl: Vec<RawStrCache>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawSpPr {
    #[serde(rename = "solidFill")]
    pub solid_fill: Option<RawSolidFillNode>,
    #[serde(flatten)]
    pub color: RawColorChoice,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawSolidFillNode {
    #[serde(flatten)]
    pub color: RawColorChoice,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawDataLabels {
    #[serde(rename = "showVal")]
    pub show_val: Option<RawValAttr>,
    #[serde(rename = "showCatName")]
    pub show_cat_name: Option<RawValAttr>,
    #[serde(rename = "showSerName")]
    pub show_ser_name: Option<RawValAttr>,
    #[serde(rename = "showPercent")]
    pub show_percent: Option<RawValAttr>,
    #[serde(rename = "showLegendKey")]
    pub show_legend_key: Option<RawValAttr>,
    #[serde(rename = "dLblPos")]
    pub d_lbl_pos: Option<RawValAttr>,
    pub separator: Option<RawText>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawTrendline {
    #[serde(rename = "trendlineType")]
    pub trendline_type: Option<RawValAttr>,
    pub period: Option<RawValAttr>,
    pub name: Option<RawText>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawDataPoint {
    pub idx: Option<RawValAttr>,
    #[serde(rename = "spPr")]
    pub sp_pr: Option<RawSpPr>,
    pub explosion: Option<RawValAttr>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawValAttr {
    #[serde(rename = "@val")]
    pub val: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawAxis {
    pub title: Option<RawTitle>,
    #[serde(rename = "majorGridlines")]
    pub major_gridlines: Option<EmptyMarker>,
    #[serde(rename = "minorGridlines")]
    pub minor_gridlines: Option<EmptyMarker>,
    #[serde(rename = "majorTickMark")]
    pub major_tick_mark: Option<RawValAttr>,
    #[serde(rename = "axId")]
    pub ax_id: Option<RawValAttr>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct EmptyMarker {}
