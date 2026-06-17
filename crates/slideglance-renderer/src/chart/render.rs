//! Chart top-level dispatcher + per-type renderers.
//!
//! Direct port of. Covers
//! all 11 OOXML chart types plus combo. Trendline overlays attach to
//! cartesian (bar / line / area / combo) series via
//! [`super::trendline::render_trendlines`].

use std::fmt::Write as _;

use slideglance_model::{ChartData, ChartElement, ChartType, LegendPosition};

use crate::svg_builder::escape_xml_text;
use crate::transform::{build_object_name_attr, build_transform_attr};

use super::common::{r, render_legend};

use super::cartesian::{
    render_area_chart, render_bar_chart, render_bubble_chart, render_line_chart,
    render_scatter_chart,
};
use super::radial::{
    render_doughnut_chart, render_of_pie_chart, render_pie_chart, render_radar_chart,
};
use super::specialty::{render_combo_chart, render_stock_chart, render_surface_chart};

/// Result of rendering one [`ChartElement`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ChartRenderResult {
    /// SVG body wrapped in a `<g transform="...">` group.
    pub content: String,
    /// `<defs>` content (currently always empty for charts).
    pub defs: String,
}

/// Render a [`ChartElement`] to SVG. Includes a chart background, axes,
/// title, legend, axis titles, and the chart-type-specific plot.
//
// The function intentionally has many lines because it follows the TS
// reference's single-pass layout flow (margins -> plot dispatch -> legend
// -> axis titles).
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn render_chart(element: &ChartElement) -> ChartRenderResult {
    const AXIS_TITLE_HEIGHT: f64 = 20.0;
    const AXIS_TITLE_WIDTH: f64 = 20.0;
    let chart = &element.chart;
    let w = element.transform.extent_width.to_pixels();
    let h = element.transform.extent_height.to_pixels();
    let transform_attr = build_transform_attr(&element.transform);

    let mut out = String::new();
    let sp_id_attr = crate::svg_builder::build_sp_id_attr(element.sp_id);
    let _ = write!(
        out,
        "<g{sp_id_attr} transform=\"{transform_attr}\"{}>",
        build_object_name_attr(element.object_name.as_deref())
    );
    let _ = write!(
        out,
        "<rect width=\"{}\" height=\"{}\" fill=\"#FFFFFF\" stroke=\"#D9D9D9\" stroke-width=\"0.5\"/>",
        r(w),
        r(h)
    );

    let mut margin = Margins {
        top: 20.0,
        right: 20.0,
        bottom: 30.0,
        left: 50.0,
    };
    if let Some(title) = &chart.title {
        let _ = write!(
            out,
            "<text x=\"{}\" y=\"20\" text-anchor=\"middle\" font-size=\"14\" font-weight=\"bold\" fill=\"#404040\">{}</text>",
            r(w / 2.0),
            escape_xml_text(title)
        );
        margin.top = 40.0;
    }

    if let Some(legend) = chart.legend {
        let labels = legend_labels(chart);
        let longest_chars = labels.iter().map(String::len).max().unwrap_or(0);
        let legend_est_width = (longest_chars as f64 * 7.0 + 30.0)
            .min((w * 0.45).floor())
            .max(80.0);
        match legend.position {
            LegendPosition::B => margin.bottom = 50.0,
            LegendPosition::T => margin.top += 20.0,
            LegendPosition::R | LegendPosition::Tr => margin.right = legend_est_width + 10.0,
            LegendPosition::L => margin.left = margin.left.max(legend_est_width + 10.0),
        }
    }

    if chart
        .category_axis
        .as_ref()
        .and_then(|a| a.title.as_ref())
        .is_some()
    {
        margin.bottom += AXIS_TITLE_HEIGHT;
    }
    if chart
        .value_axis
        .as_ref()
        .and_then(|a| a.title.as_ref())
        .is_some()
    {
        margin.left += AXIS_TITLE_WIDTH;
    }
    if chart.secondary_value_axis.is_some() {
        margin.right = margin.right.max(50.0);
    }
    if chart
        .secondary_value_axis
        .as_ref()
        .and_then(|a| a.title.as_ref())
        .is_some()
    {
        margin.right += AXIS_TITLE_WIDTH;
    }

    let plot_x = margin.left;
    let plot_y = margin.top;
    let plot_w = (w - margin.left - margin.right).max(0.0);
    let plot_h = (h - margin.top - margin.bottom).max(0.0);

    if plot_w > 0.0 && plot_h > 0.0 {
        if chart.is_combo {
            out.push_str(&render_combo_chart(chart, plot_x, plot_y, plot_w, plot_h));
        } else {
            let plot_svg = match chart.chart_type {
                ChartType::Bar => render_bar_chart(chart, plot_x, plot_y, plot_w, plot_h),
                ChartType::Line => render_line_chart(chart, plot_x, plot_y, plot_w, plot_h),
                ChartType::Pie => render_pie_chart(chart, plot_x, plot_y, plot_w, plot_h),
                ChartType::Doughnut => render_doughnut_chart(chart, plot_x, plot_y, plot_w, plot_h),
                ChartType::Area => render_area_chart(chart, plot_x, plot_y, plot_w, plot_h),
                ChartType::Scatter => render_scatter_chart(chart, plot_x, plot_y, plot_w, plot_h),
                ChartType::Bubble => render_bubble_chart(chart, plot_x, plot_y, plot_w, plot_h),
                ChartType::Radar => render_radar_chart(chart, plot_x, plot_y, plot_w, plot_h),
                ChartType::Stock => render_stock_chart(chart, plot_x, plot_y, plot_w, plot_h),
                ChartType::Surface => render_surface_chart(chart, plot_x, plot_y, plot_w, plot_h),
                ChartType::OfPie => render_of_pie_chart(chart, plot_x, plot_y, plot_w, plot_h),
            };
            out.push_str(&plot_svg);
        }
    }

    if let Some(legend) = chart.legend {
        if !chart.series.is_empty() {
            out.push_str(&render_legend(chart, w, h, legend.position));
        }
    }

    if plot_w > 0.0 && plot_h > 0.0 {
        if let Some(title) = chart.category_axis.as_ref().and_then(|a| a.title.as_ref()) {
            let cx = plot_x + plot_w / 2.0;
            let cy = plot_y + plot_h + AXIS_TITLE_HEIGHT + 22.0;
            let _ = write!(
                out,
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"13\" fill=\"#404040\">{}</text>",
                r(cx),
                r(cy),
                escape_xml_text(title)
            );
        }
        if let Some(title) = chart.value_axis.as_ref().and_then(|a| a.title.as_ref()) {
            let tx = plot_x - margin.left + 14.0;
            let ty = plot_y + plot_h / 2.0;
            let _ = write!(
                out,
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"13\" fill=\"#404040\" transform=\"rotate(-90, {}, {})\">{}</text>",
                r(tx),
                r(ty),
                r(tx),
                r(ty),
                escape_xml_text(title)
            );
        }
        if let Some(title) = chart
            .secondary_value_axis
            .as_ref()
            .and_then(|a| a.title.as_ref())
        {
            let tx = plot_x + plot_w + margin.right - 14.0;
            let ty = plot_y + plot_h / 2.0;
            let _ = write!(
                out,
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"13\" fill=\"#404040\" transform=\"rotate(90, {}, {})\">{}</text>",
                r(tx),
                r(ty),
                r(tx),
                r(ty),
                escape_xml_text(title)
            );
        }
    }

    out.push_str("</g>");
    ChartRenderResult {
        content: out,
        defs: String::new(),
    }
}

fn legend_labels(chart: &ChartData) -> Vec<String> {
    if matches!(
        chart.chart_type,
        ChartType::Pie | ChartType::Doughnut | ChartType::OfPie
    ) {
        chart.categories.clone()
    } else {
        chart
            .series
            .iter()
            .enumerate()
            .map(|(i, s)| {
                s.name
                    .clone()
                    .unwrap_or_else(|| format!("Series {}", i + 1))
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
struct Margins {
    top: f64,
    right: f64,
    bottom: f64,
    left: f64,
}

// Bar chart follows the spec's single function with two branches
// for vertical (column) vs horizontal layout. Splitting would mostly
// duplicate the axis / label scaffolding.
// `chart`/`x`/`y`/`w`/`h` mirror the spec signature; renaming
// would only obscure parity.
#[allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names
)]
#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use slideglance_color::{ResolvedColor, Rgb};
    use slideglance_model::{
        AxisGroup, BarDirection, ChartAxis, ChartLegend, ChartSeries, ChartType, RadarStyle,
        Transform,
    };
    use slideglance_utils::Emu;

    fn xfrm(w: i64, h: i64) -> Transform {
        Transform {
            offset_x: Emu::new(0),
            offset_y: Emu::new(0),
            extent_width: Emu::new(w),
            extent_height: Emu::new(h),
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
        }
    }

    fn series(values: Vec<f64>, hex: &str) -> ChartSeries {
        ChartSeries {
            name: Some("S".to_string()),
            values,
            x_values: None,
            bubble_sizes: None,
            color: ResolvedColor::new(Rgb::from_hex(hex).unwrap(), 1.0),
            data_labels: None,
            trendlines: None,
            sub_chart_type: None,
            axis_group: None,
            smooth: None,
            data_points: None,
            explosion: None,
        }
    }

    fn chart_data(chart_type: ChartType, series_vec: Vec<ChartSeries>) -> ChartData {
        ChartData {
            chart_type,
            title: None,
            series: series_vec,
            categories: vec!["A".to_string(), "B".to_string(), "C".to_string()],
            bar_direction: None,
            hole_size: None,
            radar_style: None,
            of_pie_type: None,
            second_pie_size: None,
            split_pos: None,
            legend: None,
            category_axis: None,
            value_axis: None,
            secondary_value_axis: None,
            data_labels: None,
            is_combo: false,
        }
    }

    fn element(chart: ChartData) -> ChartElement {
        ChartElement {
            sp_id: None,
            transform: xfrm(9_144_000, 5_143_500),
            chart,
            object_name: None,
            hidden: false,
        }
    }

    #[test]
    fn empty_chart_emits_only_wrapper_and_background() {
        let mut c = chart_data(ChartType::Bar, vec![]);
        c.categories.clear();
        let res = render_chart(&element(c));
        assert!(res.content.starts_with("<g transform=\""));
        assert!(res.content.contains("<rect width="));
        assert!(res.content.ends_with("</g>"));
        assert!(!res.content.contains("<line"));
    }

    #[test]
    fn chart_emits_data_sp_id_when_present() {
        let mut c = chart_data(ChartType::Bar, vec![]);
        c.categories.clear();
        let mut e = element(c);
        e.sp_id = Some(77);
        let res = render_chart(&e);
        assert!(
            res.content.contains("data-sp-id=\"77\""),
            "data-sp-id missing: {}",
            res.content
        );
        // Attribute lands on the outer <g> immediately after the tag name.
        assert!(
            res.content.starts_with("<g data-sp-id=\"77\" transform=\""),
            "unexpected ordering: {}",
            res.content
        );
    }

    #[test]
    fn bar_chart_emits_bars() {
        let c = chart_data(
            ChartType::Bar,
            vec![series(vec![10.0, 20.0, 30.0], "#FF0000")],
        );
        let res = render_chart(&element(c));
        assert!(res.content.matches("<rect").count() >= 4); // 1 background + 3 bars
        assert!(res.content.contains("fill=\"#FF0000\""));
    }

    #[test]
    fn line_chart_emits_polyline_and_circles() {
        let c = chart_data(
            ChartType::Line,
            vec![series(vec![1.0, 2.0, 3.0], "#00FF00")],
        );
        let res = render_chart(&element(c));
        assert!(res.content.contains("<polyline"));
        assert_eq!(res.content.matches("<circle").count(), 3);
    }

    #[test]
    fn pie_chart_emits_paths_or_circle_per_slice() {
        let c = chart_data(
            ChartType::Pie,
            vec![series(vec![25.0, 25.0, 50.0], "#000000")],
        );
        let res = render_chart(&element(c));
        // 3 slices -> 3 path elements (no single-circle fallback).
        assert_eq!(res.content.matches("<path").count(), 3);
    }

    #[test]
    fn pie_single_slice_uses_circle() {
        let c = chart_data(ChartType::Pie, vec![series(vec![100.0], "#000000")]);
        let res = render_chart(&element(c));
        assert!(res.content.contains("<circle"));
    }

    #[test]
    fn doughnut_emits_paths_with_inner_radius() {
        let c = chart_data(
            ChartType::Doughnut,
            vec![series(vec![1.0, 2.0, 3.0], "#000000")],
        );
        let res = render_chart(&element(c));
        // 3 path elements (one per slice).
        assert_eq!(res.content.matches("<path").count(), 3);
    }

    #[test]
    fn title_renders_above_chart() {
        let mut c = chart_data(ChartType::Bar, vec![series(vec![1.0], "#000000")]);
        c.title = Some("My Chart".to_string());
        let res = render_chart(&element(c));
        assert!(res.content.contains(">My Chart</text>"));
    }

    #[test]
    fn legend_at_right_renders_swatches() {
        let mut c = chart_data(
            ChartType::Bar,
            vec![series(vec![1.0], "#FF0000"), series(vec![2.0], "#00FF00")],
        );
        c.legend = Some(ChartLegend {
            position: LegendPosition::R,
        });
        let res = render_chart(&element(c));
        // Each series should produce one swatch <rect> + one label <text>.
        assert!(res.content.contains("fill=\"#FF0000\""));
        assert!(res.content.contains("fill=\"#00FF00\""));
    }

    #[test]
    fn category_axis_title_renders_below_plot() {
        let mut c = chart_data(ChartType::Bar, vec![series(vec![1.0, 2.0], "#000000")]);
        c.category_axis = Some(ChartAxis {
            title: Some("Categories".to_string()),
            ..ChartAxis::default()
        });
        let res = render_chart(&element(c));
        assert!(res.content.contains(">Categories</text>"));
    }

    #[test]
    fn area_chart_emits_filled_polygon() {
        let c = chart_data(
            ChartType::Area,
            vec![series(vec![1.0, 2.0, 3.0], "#FF0000")],
        );
        let res = render_chart(&element(c));
        assert!(
            res.content.contains("<polygon points=\""),
            "{}",
            res.content
        );
        assert!(res.content.contains("fill=\"#FF0000\""));
    }

    #[test]
    fn scatter_chart_emits_circles_per_point() {
        let mut s = series(vec![10.0, 20.0, 30.0], "#0000FF");
        s.x_values = Some(vec![1.0, 2.0, 3.0]);
        let c = chart_data(ChartType::Scatter, vec![s]);
        let res = render_chart(&element(c));
        assert!(res.content.matches("<circle ").count() >= 3);
    }

    #[test]
    fn bubble_chart_uses_size_array_for_radius() {
        let mut s = series(vec![10.0, 20.0], "#FF8800");
        s.x_values = Some(vec![1.0, 2.0]);
        s.bubble_sizes = Some(vec![100.0, 400.0]);
        let c = chart_data(ChartType::Bubble, vec![s]);
        let res = render_chart(&element(c));
        assert!(res.content.contains("fill-opacity=\"0.6\""));
    }

    #[test]
    fn radar_chart_renders_concentric_grid_and_polygon() {
        let c = chart_data(
            ChartType::Radar,
            vec![series(vec![1.0, 2.0, 3.0, 4.0], "#00AA00")],
        );
        let res = render_chart(&element(c));
        // 5 concentric grid circles
        assert!(
            res.content.matches("<circle cx=").count() >= 5,
            "{}",
            res.content
        );
        assert!(res.content.contains("<polygon points="));
    }

    #[test]
    fn radar_filled_emits_fill_opacity_03() {
        let mut c = chart_data(
            ChartType::Radar,
            vec![series(vec![1.0, 2.0, 3.0], "#00AA00")],
        );
        c.radar_style = Some(RadarStyle::Filled);
        let res = render_chart(&element(c));
        assert!(res.content.contains("fill-opacity=\"0.3\""));
    }

    #[test]
    fn stock_chart_requires_three_series() {
        let c = chart_data(ChartType::Stock, vec![series(vec![10.0], "#000000")]);
        let res = render_chart(&element(c));
        // Only the chart frame + axes (if any), no hi-lo lines.
        assert!(!res.content.contains("stroke=\"#404040\""));
    }

    #[test]
    fn stock_chart_with_high_low_close_emits_hi_lo_lines() {
        let high = series(vec![5.0, 8.0, 12.0], "#000000");
        let low = series(vec![1.0, 2.0, 3.0], "#000000");
        let close = series(vec![3.0, 5.0, 9.0], "#000000");
        let c = chart_data(ChartType::Stock, vec![high, low, close]);
        let res = render_chart(&element(c));
        assert!(res.content.contains("stroke=\"#404040\""));
    }

    #[test]
    fn surface_chart_emits_heatmap_cells() {
        let s1 = series(vec![1.0, 2.0, 3.0], "#000000");
        let s2 = series(vec![4.0, 5.0, 6.0], "#000000");
        let c = chart_data(ChartType::Surface, vec![s1, s2]);
        let res = render_chart(&element(c));
        // 2 rows x 3 cols = 6 heatmap rects (plus the chart frame rect).
        assert!(res.content.matches("<rect").count() >= 7);
    }

    #[test]
    fn of_pie_chart_emits_secondary_pie_or_bar() {
        let c = chart_data(
            ChartType::OfPie,
            vec![series(vec![10.0, 20.0, 5.0, 5.0], "#000000")],
        );
        let res = render_chart(&element(c));
        // Connector line between pies should exist.
        assert!(res.content.contains("stroke=\"#A6A6A6\""));
    }

    #[test]
    fn combo_chart_dispatches_per_subchart_type() {
        let mut bar = series(vec![1.0, 2.0, 3.0], "#FF0000");
        bar.sub_chart_type = Some(ChartType::Bar);
        let mut line = series(vec![10.0, 20.0, 30.0], "#0000FF");
        line.sub_chart_type = Some(ChartType::Line);
        let mut c = chart_data(ChartType::Bar, vec![bar, line]);
        c.is_combo = true;
        let res = render_chart(&element(c));
        // Bar series → at least one rect inside the plot area.
        assert!(res.content.matches("<rect").count() >= 2);
        // Line series → polyline emitted.
        assert!(res.content.contains("<polyline points="));
    }

    #[test]
    fn combo_chart_secondary_axis_emits_right_side_axis() {
        let mut primary = series(vec![1.0, 2.0, 3.0], "#FF0000");
        primary.sub_chart_type = Some(ChartType::Bar);
        let mut secondary = series(vec![100.0, 200.0, 300.0], "#0000FF");
        secondary.sub_chart_type = Some(ChartType::Line);
        secondary.axis_group = Some(AxisGroup::Secondary);
        let mut c = chart_data(ChartType::Bar, vec![primary, secondary]);
        c.is_combo = true;
        let res = render_chart(&element(c));
        // The right-side secondary axis should have ticks via labels at
        // text-anchor="start".
        assert!(
            res.content.contains("text-anchor=\"start\""),
            "{}",
            res.content
        );
    }

    #[test]
    fn line_chart_with_smooth_emits_path_d() {
        let mut s = series(vec![1.0, 2.0, 3.0, 4.0], "#000000");
        s.smooth = Some(true);
        let c = chart_data(ChartType::Line, vec![s]);
        let res = render_chart(&element(c));
        assert!(res.content.contains("<path d=\""));
        assert!(!res.content.contains("<polyline"));
    }

    #[test]
    fn line_chart_with_trendline_emits_dashed_overlay() {
        let mut s = series(vec![1.0, 2.0, 3.0, 4.0], "#000000");
        s.trendlines = Some(vec![slideglance_model::ChartTrendline {
            trendline_type: slideglance_model::TrendlineType::Linear,
            period: None,
            name: None,
        }]);
        let c = chart_data(ChartType::Line, vec![s]);
        let res = render_chart(&element(c));
        assert!(res.content.contains("stroke-dasharray=\"4,3\""));
    }

    #[test]
    fn horizontal_bar_chart_uses_horizontal_layout() {
        let mut c = chart_data(ChartType::Bar, vec![series(vec![10.0, 20.0], "#000000")]);
        c.bar_direction = Some(BarDirection::Bar);
        let res = render_chart(&element(c));
        // Horizontal bars should have the value-axis labels along the bottom
        // (text-anchor="middle"). Just verify the chart renders without
        // panicking and produces some bars.
        assert!(res.content.matches("<rect").count() >= 3);
    }

    #[test]
    fn empty_series_list_emits_no_plot() {
        let c = chart_data(ChartType::Bar, vec![]);
        let res = render_chart(&element(c));
        assert!(!res.content.contains("<polyline"));
    }

    #[test]
    fn chart_with_max_zero_skips_plot() {
        let c = chart_data(ChartType::Bar, vec![series(vec![0.0, 0.0], "#000000")]);
        let res = render_chart(&element(c));
        // Background rect + axes only, no bar rects added beyond the
        // background.
        assert_eq!(res.content.matches("fill=\"#000000\"").count(), 0);
    }
}
