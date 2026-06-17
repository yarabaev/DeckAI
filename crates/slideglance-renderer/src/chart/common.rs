//! Shared chart-renderer helpers: tick computation, axis labels, legend,
//! data label composition, color resolution, basic SVG building blocks.

use std::fmt::Write as _;

use slideglance_color::{ResolvedColor, Rgb};
use slideglance_model::{
    ChartData, ChartDataLabels, ChartSeries, ChartType, LegendPosition, TickMark,
};

use crate::color::{alpha_str, color_hex};
use crate::svg_builder::escape_xml_text;

/// Default series palette — Office's accent colors in their standard
/// rotation. Mirrors `DEFAULT_SERIES_COLORS` in the spec.
pub(crate) const DEFAULT_SERIES_COLORS: &[(u8, u8, u8)] = &[
    (0x44, 0x72, 0xC4),
    (0xED, 0x7D, 0x31),
    (0xA5, 0xA5, 0xA5),
    (0xFF, 0xC0, 0x00),
    (0x5B, 0x9B, 0xD5),
    (0x70, 0xAD, 0x47),
];

/// Axis-label rendering options.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ValueAxisLabelOptions {
    /// Plot area width — when set, gridlines are drawn across the full plot.
    pub plot_w: Option<f64>,
    /// Whether to emit horizontal gridlines.
    pub gridlines: bool,
    /// Major tick mark style.
    pub tick_mark: TickMark,
}

/// Decide value-axis options from chart settings — gridlines default `true`
/// when `valueAxis` is unset (matches TS backwards-compatibility behavior).
pub(crate) fn value_axis_options(chart: &ChartData, plot_w: f64) -> ValueAxisLabelOptions {
    let ax = chart.value_axis.as_ref();
    let gridlines = ax.is_none_or(|a| a.major_gridlines.unwrap_or(false));
    let tick_mark = ax.and_then(|a| a.major_tick_mark).unwrap_or(TickMark::Out);
    ValueAxisLabelOptions {
        plot_w: Some(plot_w),
        gridlines,
        tick_mark,
    }
}

/// Compute "nice" tick marks between `min_val` and `max_val` with roughly
/// `target_count` divisions. Matches the TS algorithm exactly.
pub(crate) fn compute_nice_ticks(min_val: f64, max_val: f64, target_count: u32) -> Vec<f64> {
    let range = max_val - min_val;
    if range == 0.0 {
        return vec![min_val];
    }
    let target = f64::from(target_count.max(1));
    let rough_step = range / target;
    let magnitude = 10f64.powf(rough_step.log10().floor());
    let residual = rough_step / magnitude;
    let nice_step = if residual <= 1.5 {
        magnitude
    } else if residual <= 3.0 {
        2.0 * magnitude
    } else if residual <= 7.0 {
        5.0 * magnitude
    } else {
        10.0 * magnitude
    };
    let nice_min = (min_val / nice_step).floor() * nice_step;
    let nice_max = (max_val / nice_step).ceil() * nice_step;
    let mut ticks = Vec::new();
    let mut v = nice_min;
    while v <= nice_max + nice_step * 0.5 {
        ticks.push((v * 1e10).round() / 1e10);
        v += nice_step;
    }
    ticks
}

/// Render Y-axis tick labels + tick marks + (optional) horizontal gridlines.
pub(crate) fn render_value_axis_labels(
    ticks: &[f64],
    scale_min: f64,
    scale_max: f64,
    x: f64,
    y: f64,
    h: f64,
    opts: ValueAxisLabelOptions,
) -> String {
    let range = scale_max - scale_min;
    if range == 0.0 {
        return String::new();
    }
    let mut out = String::new();
    for &tick in ticks {
        let ratio = (tick - scale_min) / range;
        if !(-0.001..=1.001).contains(&ratio) {
            continue;
        }
        let tick_y = y + h - ratio * h;
        let _ = write!(
            out,
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"12\" fill=\"#595959\">{}</text>",
            r(x - 5.0),
            r(tick_y + 4.0),
            escape_xml_text(&format_tick_value(tick))
        );
        if !matches!(opts.tick_mark, TickMark::None) {
            let in_len = if matches!(opts.tick_mark, TickMark::In | TickMark::Cross) {
                3.0
            } else {
                0.0
            };
            let out_len = if matches!(opts.tick_mark, TickMark::Out | TickMark::Cross) {
                3.0
            } else {
                0.0
            };
            let _ = write!(
                out,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#D9D9D9\" stroke-width=\"1\"/>",
                r(x - out_len),
                r(tick_y),
                r(x + in_len),
                r(tick_y)
            );
        }
        if opts.gridlines && tick != 0.0 {
            if let Some(plot_w) = opts.plot_w {
                let _ = write!(
                    out,
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#E5E5E5\" stroke-width=\"0.5\"/>",
                    r(x),
                    r(tick_y),
                    r(x + plot_w),
                    r(tick_y)
                );
            }
        }
    }
    out
}

/// Format a tick value with K/M/B suffixes for large magnitudes; integer
/// values render without decimals.
pub(crate) fn format_tick_value(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1.0e9 {
        format!("{}B", n_round(value / 1.0e9))
    } else if abs >= 1.0e6 {
        format!("{}M", n_round(value / 1.0e6))
    } else if abs >= 1.0e4 {
        format!("{}K", n_round(value / 1000.0))
    } else if value.fract() == 0.0 && value.is_finite() {
        format!("{}", value as i64)
    } else {
        n_round(value)
    }
}

fn n_round(v: f64) -> String {
    let rounded = (v * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 && rounded.is_finite() {
        return format!("{}", rounded as i64);
    }
    format!("{rounded}")
}

/// Maximum value across all series.
pub(crate) fn get_max_value(series: &[ChartSeries]) -> f64 {
    series
        .iter()
        .flat_map(|s| s.values.iter().copied())
        .fold(0.0_f64, f64::max)
}

/// Build a `fill="..."` (and optional `fill-opacity`) attribute pair.
pub(crate) fn fill_attr(color: &ResolvedColor) -> String {
    let alpha = if color.alpha < 1.0 {
        format!(" fill-opacity=\"{}\"", alpha_str(color.alpha))
    } else {
        String::new()
    };
    format!("fill=\"{}\"{alpha}", color_hex(color))
}

/// Round `v` to two decimal places, matching TS `round(n)`.
#[must_use]
pub(crate) fn r(v: f64) -> String {
    let rounded = (v * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 && rounded.is_finite() {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

/// Build a Catmull-Rom-derived smooth cubic-Bezier path through `points`.
/// Returns the SVG `d` attribute body (no surrounding `<path>`). For 0–2
/// input points falls back to a polyline (`M..L..`).
///
/// Mirrors `buildSmoothPathD` in.
//
// `cp1x` / `cp1y` / `cp2x` / `cp2y` mirror the spec's control-point
// names; the similar prefixes are intentional and renaming them would
// obscure the parity.
#[allow(clippy::similar_names)]
pub(crate) fn build_smooth_path_d(points: &[(f64, f64)]) -> String {
    if points.is_empty() {
        return String::new();
    }
    if points.len() < 3 {
        let mut out = String::new();
        for (i, (px, py)) in points.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            let _ = write!(
                out,
                "{}{},{}",
                if i == 0 { 'M' } else { 'L' },
                r(*px),
                r(*py)
            );
        }
        return out;
    }
    let mut out = String::new();
    let _ = write!(out, "M{},{}", r(points[0].0), r(points[0].1));
    for i in 0..(points.len() - 1) {
        let p0 = if i == 0 { points[i] } else { points[i - 1] };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 < points.len() {
            points[i + 2]
        } else {
            points[i + 1]
        };
        let cp1x = p1.0 + (p2.0 - p0.0) / 6.0;
        let cp1y = p1.1 + (p2.1 - p0.1) / 6.0;
        let cp2x = p2.0 - (p3.0 - p1.0) / 6.0;
        let cp2y = p2.1 - (p3.1 - p1.1) / 6.0;
        let _ = write!(
            out,
            " C{},{} {},{} {},{}",
            r(cp1x),
            r(cp1y),
            r(cp2x),
            r(cp2y),
            r(p2.0),
            r(p2.1)
        );
    }
    out
}

/// Resolve a single series data point's color, honoring `<c:dPt>` overrides.
pub(crate) fn point_color(series: &ChartSeries, idx: u32) -> ResolvedColor {
    if let Some(dps) = &series.data_points {
        for dp in dps {
            if dp.idx == idx {
                if let Some(color) = dp.color {
                    return color;
                }
            }
        }
    }
    series.color
}

/// Resolve the explosion percentage for one slice (per-point override
/// wins over series-level explosion).
pub(crate) fn point_explosion(series: &ChartSeries, idx: u32) -> f64 {
    if let Some(dps) = &series.data_points {
        for dp in dps {
            if dp.idx == idx {
                if let Some(e) = dp.explosion {
                    return e;
                }
            }
        }
    }
    series.explosion.unwrap_or(0.0)
}

/// Pie/doughnut/ofPie slice color, falling back to the rotating palette.
pub(crate) fn pie_slice_color(index: usize, chart: &ChartData) -> ResolvedColor {
    let Some(series) = chart.series.first() else {
        return palette_color(index);
    };
    if let Some(dps) = &series.data_points {
        for dp in dps {
            if dp.idx as usize == index {
                if let Some(color) = dp.color {
                    return color;
                }
            }
        }
    }
    palette_color(index)
}

fn palette_color(index: usize) -> ResolvedColor {
    let (r, g, b) = DEFAULT_SERIES_COLORS[index % DEFAULT_SERIES_COLORS.len()];
    ResolvedColor::new(Rgb::new(r, g, b), 1.0)
}

/// Merge chart-level + series-level data label settings, returning `None`
/// when neither side has any.
pub(crate) fn resolve_series_data_labels(
    chart: &ChartData,
    series: &ChartSeries,
) -> Option<ChartDataLabels> {
    if chart.data_labels.is_none() && series.data_labels.is_none() {
        return None;
    }
    let chart_lbl = chart.data_labels.clone().unwrap_or_default();
    let series_lbl = series.data_labels.clone().unwrap_or_default();
    Some(ChartDataLabels {
        show_value: series_lbl.show_value.or(chart_lbl.show_value),
        show_category_name: series_lbl
            .show_category_name
            .or(chart_lbl.show_category_name),
        show_series_name: series_lbl.show_series_name.or(chart_lbl.show_series_name),
        show_percent: series_lbl.show_percent.or(chart_lbl.show_percent),
        show_legend_key: series_lbl.show_legend_key.or(chart_lbl.show_legend_key),
        position: series_lbl.position.or(chart_lbl.position),
        separator: series_lbl.separator.or(chart_lbl.separator),
    })
}

/// Compose a data label string given the active settings + value/category/
/// series-name/percent context. Returns `None` when nothing should be shown.
pub(crate) fn compose_data_label(
    labels: Option<&ChartDataLabels>,
    value: f64,
    category: Option<&str>,
    series_name: Option<&str>,
    percent: Option<f64>,
) -> Option<String> {
    let labels = labels?;
    let sep = labels.separator.as_deref().unwrap_or(", ");
    let mut parts: Vec<String> = Vec::new();
    if labels.show_series_name.unwrap_or(false) {
        if let Some(name) = series_name {
            if !name.is_empty() {
                parts.push(name.to_string());
            }
        }
    }
    if labels.show_category_name.unwrap_or(false) {
        if let Some(cat) = category {
            if !cat.is_empty() {
                parts.push(cat.to_string());
            }
        }
    }
    if labels.show_value.unwrap_or(false) {
        parts.push(format_tick_value(value));
    }
    if labels.show_percent.unwrap_or(false) {
        if let Some(pct) = percent {
            parts.push(format!("{}%", (pct * 100.0).round() as i64));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(sep))
    }
}

/// Append a `<text>` element representing one data label.
pub(crate) fn emit_data_label(out: &mut String, text: &str, cx: f64, cy: f64, anchor: &str) {
    let _ = write!(
        out,
        "<text x=\"{}\" y=\"{}\" text-anchor=\"{anchor}\" font-size=\"11\" fill=\"#404040\">{}</text>",
        r(cx),
        r(cy),
        escape_xml_text(text)
    );
}

/// Render the chart legend (right / left / top-right vertical, top / bottom
/// horizontal). Returns the SVG fragment or empty string when there are no
/// entries.
pub(crate) fn render_legend(
    chart: &ChartData,
    chart_w: f64,
    chart_h: f64,
    position: LegendPosition,
) -> String {
    struct Entry {
        label: String,
        color: ResolvedColor,
    }
    let entries: Vec<Entry> = if matches!(
        chart.chart_type,
        ChartType::Pie | ChartType::Doughnut | ChartType::OfPie
    ) {
        chart
            .categories
            .iter()
            .enumerate()
            .map(|(i, cat)| Entry {
                label: cat.clone(),
                color: pie_slice_color(i, chart),
            })
            .collect()
    } else {
        chart
            .series
            .iter()
            .enumerate()
            .map(|(i, s)| Entry {
                label: s
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("Series {}", i + 1)),
                color: s.color,
            })
            .collect()
    };
    if entries.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    if matches!(
        position,
        LegendPosition::R | LegendPosition::L | LegendPosition::Tr
    ) {
        let row_h = 18.0;
        let legend_h = entries.len() as f64 * row_h;
        let longest = entries.iter().map(|e| e.label.len()).max().unwrap_or(0);
        let legend_w = ((chart_w * 0.45).floor())
            .min(longest as f64 * 7.0 + 30.0)
            .max(80.0);
        let legend_x = if matches!(position, LegendPosition::L) {
            10.0
        } else {
            chart_w - legend_w
        };
        let legend_y = if matches!(position, LegendPosition::Tr) {
            30.0
        } else {
            ((chart_h - legend_h) / 2.0).max(30.0)
        };
        for (i, e) in entries.iter().enumerate() {
            let row_y = legend_y + i as f64 * row_h;
            let _ = write!(
                out,
                "<rect x=\"{}\" y=\"{}\" width=\"12\" height=\"12\" {}/><text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#595959\">{}</text>",
                r(legend_x),
                r(row_y),
                fill_attr(&e.color),
                r(legend_x + 16.0),
                r(row_y + 10.0),
                escape_xml_text(&e.label)
            );
        }
        return out;
    }
    let entry_width = 100.0;
    let total = entries.len() as f64 * entry_width;
    let start_x = ((chart_w - total) / 2.0).max(5.0);
    let legend_y = if matches!(position, LegendPosition::T) {
        25.0
    } else {
        chart_h - 15.0
    };
    for (i, e) in entries.iter().enumerate() {
        let ex = start_x + i as f64 * entry_width;
        let _ = write!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"12\" height=\"12\" {}/><text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#595959\">{}</text>",
            r(ex),
            r(legend_y - 6.0),
            fill_attr(&e.color),
            r(ex + 16.0),
            r(legend_y + 4.0),
            escape_xml_text(&e.label)
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_ticks_basic() {
        let ticks = compute_nice_ticks(0.0, 100.0, 5);
        assert!(ticks.contains(&0.0));
        assert!(ticks.contains(&100.0));
    }

    #[test]
    fn nice_ticks_zero_range() {
        assert_eq!(compute_nice_ticks(50.0, 50.0, 5), vec![50.0]);
    }

    #[test]
    fn format_tick_with_units() {
        assert_eq!(format_tick_value(1_500.0), "1500");
        assert_eq!(format_tick_value(15_000.0), "15K");
        assert_eq!(format_tick_value(1_500_000.0), "1.5M");
        assert_eq!(format_tick_value(2_000_000_000.0), "2B");
    }

    #[test]
    fn round_helper_matches_two_decimal_truncation() {
        assert_eq!(r(1.234), "1.23");
        assert_eq!(r(1.0), "1");
        assert_eq!(r(1.5), "1.5");
    }

    #[test]
    fn fill_attr_omits_opacity_when_opaque() {
        let c = ResolvedColor::new(Rgb::new(0xFF, 0, 0), 1.0);
        assert_eq!(fill_attr(&c), "fill=\"#FF0000\"");
    }

    #[test]
    fn fill_attr_includes_opacity_when_translucent() {
        let c = ResolvedColor::new(Rgb::new(0, 0, 0), 0.25);
        assert_eq!(fill_attr(&c), "fill=\"#000000\" fill-opacity=\"0.25\"");
    }

    #[test]
    fn palette_color_rotates_on_overflow() {
        // 6 entries -> index 6 wraps back to index 0 (#4472C4).
        let c0 = palette_color(0);
        let c6 = palette_color(6);
        assert_eq!(c0.rgb, c6.rgb);
    }
}
