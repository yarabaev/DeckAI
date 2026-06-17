//! Chart-type renderers — specialty (stock / surface / combo) family.
//!
//! Extracted from `chart/render.rs` so each top-level dispatch case
//! lives next to the SVG emission code it drives. Helpers come from
//! `chart::common`; trendline overlays from `chart::trendline`.

use std::fmt::Write as _;

use slideglance_model::{AxisGroup, ChartData, ChartSeries, ChartType};

use crate::color::color_hex;
use crate::svg_builder::escape_xml_text;

use super::common::{
    build_smooth_path_d, compute_nice_ticks, fill_attr, format_tick_value, point_color, r,
    render_value_axis_labels, value_axis_options,
};

#[allow(clippy::many_single_char_names)]
pub(super) fn render_stock_chart(chart: &ChartData, x: f64, y: f64, w: f64, h: f64) -> String {
    // Stock chart expects three series in order: High (0), Low (1), Close (2).
    if chart.series.len() < 3 {
        return String::new();
    }
    let high = &chart.series[0];
    let low = &chart.series[1];
    let close = &chart.series[2];
    let cat_count = if chart.categories.is_empty() {
        high.values.len()
    } else {
        chart.categories.len()
    };
    if cat_count == 0 {
        return String::new();
    }

    let mut min_val = f64::INFINITY;
    let mut max_val = f64::NEG_INFINITY;
    for s in [high, low, close] {
        for v in &s.values {
            if *v < min_val {
                min_val = *v;
            }
            if *v > max_val {
                max_val = *v;
            }
        }
    }
    if max_val == min_val {
        return String::new();
    }

    let target = ((h / 30.0).floor() as i64).max(2) as u32;
    let ticks = compute_nice_ticks(min_val, max_val, target);
    let scale_min = *ticks.first().unwrap_or(&min_val);
    let scale_max = *ticks.last().unwrap_or(&max_val);

    let mut out = String::new();
    let _ = write!(
        out,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#D9D9D9\" stroke-width=\"1\"/>",
        r(x),
        r(y + h),
        r(x + w),
        r(y + h)
    );
    let _ = write!(
        out,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#D9D9D9\" stroke-width=\"1\"/>",
        r(x),
        r(y),
        r(x),
        r(y + h)
    );
    out.push_str(&render_value_axis_labels(
        &ticks,
        scale_min,
        scale_max,
        x,
        y,
        h,
        value_axis_options(chart, w),
    ));

    let group_w = w / cat_count as f64;
    for c in 0..cat_count {
        let label = chart.categories.get(c).map_or("", String::as_str);
        let label_x = x + (c as f64 + 0.5) * group_w;
        let _ = write!(
            out,
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#595959\">{}</text>",
            r(label_x),
            r(y + h + 15.0),
            escape_xml_text(label)
        );
    }

    let range = scale_max - scale_min;
    for c in 0..cat_count {
        let cx = x + (c as f64 + 0.5) * group_w;
        let high_v = high.values.get(c).copied().unwrap_or(0.0);
        let low_v = low.values.get(c).copied().unwrap_or(0.0);
        let close_v = close.values.get(c).copied().unwrap_or(0.0);
        let high_y = y + h - ((high_v - scale_min) / range) * h;
        let low_y = y + h - ((low_v - scale_min) / range) * h;
        let close_y = y + h - ((close_v - scale_min) / range) * h;
        let _ = write!(
            out,
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#404040\" stroke-width=\"2\"/>",
            r(cx),
            r(high_y),
            r(cx),
            r(low_y)
        );
        let tick_w = group_w * 0.2;
        let _ = write!(
            out,
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#404040\" stroke-width=\"2\"/>",
            r(cx),
            r(close_y),
            r(cx + tick_w),
            r(close_y)
        );
    }
    out
}

#[allow(clippy::many_single_char_names, clippy::similar_names)]
pub(super) fn render_surface_chart(chart: &ChartData, x: f64, y: f64, w: f64, h: f64) -> String {
    if chart.series.is_empty() {
        return String::new();
    }
    let rows = chart.series.len();
    let cols = if chart.categories.is_empty() {
        chart
            .series
            .iter()
            .map(|s| s.values.len())
            .max()
            .unwrap_or(0)
    } else {
        chart.categories.len()
    };
    if cols == 0 {
        return String::new();
    }
    let mut min_val = f64::INFINITY;
    let mut max_val = f64::NEG_INFINITY;
    for s in &chart.series {
        for v in &s.values {
            if *v < min_val {
                min_val = *v;
            }
            if *v > max_val {
                max_val = *v;
            }
        }
    }
    if min_val == max_val {
        max_val = min_val + 1.0;
    }
    let cell_w = w / cols as f64;
    let cell_h = h / rows as f64;

    let mut out = String::new();
    for (r_idx, s) in chart.series.iter().enumerate() {
        for c in 0..cols {
            let val = s.values.get(c).copied().unwrap_or(0.0);
            let t = (val - min_val) / (max_val - min_val);
            let color = heatmap_color(t);
            let cx_pos = x + c as f64 * cell_w;
            let cy_pos = y + r_idx as f64 * cell_h;
            let _ = write!(
                out,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{color}\" stroke=\"#FFFFFF\" stroke-width=\"0.5\"/>",
                r(cx_pos),
                r(cy_pos),
                r(cell_w),
                r(cell_h)
            );
        }
    }
    for c in 0..cols {
        let label = chart.categories.get(c).map_or("", String::as_str);
        if !label.is_empty() {
            let label_x = x + (c as f64 + 0.5) * cell_w;
            let _ = write!(
                out,
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#595959\">{}</text>",
                r(label_x),
                r(y + h + 15.0),
                escape_xml_text(label)
            );
        }
    }
    for (r_idx, s) in chart.series.iter().enumerate() {
        let label = s.name.as_deref().unwrap_or("");
        if !label.is_empty() {
            let label_y = y + (r_idx as f64 + 0.5) * cell_h;
            let _ = write!(
                out,
                "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"12\" fill=\"#595959\">{}</text>",
                r(x - 5.0),
                r(label_y + 4.0),
                escape_xml_text(label)
            );
        }
    }
    out
}

pub(super) fn heatmap_color(t: f64) -> String {
    let clamped = t.clamp(0.0, 1.0);
    let (rc, gc, bc) = if clamped < 0.25 {
        let s = clamped / 0.25;
        (0_u8, (s * 255.0).round() as u8, 255_u8)
    } else if clamped < 0.5 {
        let s = (clamped - 0.25) / 0.25;
        (0_u8, 255_u8, ((1.0 - s) * 255.0).round() as u8)
    } else if clamped < 0.75 {
        let s = (clamped - 0.5) / 0.25;
        ((s * 255.0).round() as u8, 255_u8, 0_u8)
    } else {
        let s = (clamped - 0.75) / 0.25;
        (255_u8, ((1.0 - s) * 255.0).round() as u8, 0_u8)
    };
    format!("#{rc:02X}{gc:02X}{bc:02X}")
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
pub(super) fn render_combo_chart(chart: &ChartData, x: f64, y: f64, w: f64, h: f64) -> String {
    if chart.series.is_empty() {
        return String::new();
    }
    let cat_count = if chart.categories.is_empty() {
        chart
            .series
            .iter()
            .map(|s| s.values.len())
            .max()
            .unwrap_or(0)
    } else {
        chart.categories.len()
    };
    if cat_count == 0 {
        return String::new();
    }

    let primary_series: Vec<&ChartSeries> = chart
        .series
        .iter()
        .filter(|s| s.axis_group != Some(AxisGroup::Secondary))
        .collect();
    let secondary_series: Vec<&ChartSeries> = chart
        .series
        .iter()
        .filter(|s| s.axis_group == Some(AxisGroup::Secondary))
        .collect();
    let primary_max = max_value_refs(&primary_series);
    let secondary_max = max_value_refs(&secondary_series);
    if primary_max == 0.0 && secondary_max == 0.0 {
        return String::new();
    }

    let target = ((h / 30.0).floor() as i64).max(2) as u32;
    let primary_ticks = compute_nice_ticks(0.0, primary_max.max(1.0), target);
    let primary_scale = *primary_ticks.last().unwrap_or(&1.0);
    let has_secondary = !secondary_series.is_empty();
    let secondary_ticks = if has_secondary {
        compute_nice_ticks(0.0, secondary_max.max(1.0), target)
    } else {
        Vec::new()
    };
    let secondary_scale = if has_secondary {
        *secondary_ticks.last().unwrap_or(&1.0)
    } else {
        1.0
    };

    let mut out = String::new();
    let _ = write!(
        out,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#D9D9D9\" stroke-width=\"1\"/>",
        r(x),
        r(y + h),
        r(x + w),
        r(y + h)
    );
    let _ = write!(
        out,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#D9D9D9\" stroke-width=\"1\"/>",
        r(x),
        r(y),
        r(x),
        r(y + h)
    );
    out.push_str(&render_value_axis_labels(
        &primary_ticks,
        0.0,
        primary_scale,
        x,
        y,
        h,
        value_axis_options(chart, w),
    ));

    if has_secondary {
        let _ = write!(
            out,
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#D9D9D9\" stroke-width=\"1\"/>",
            r(x + w),
            r(y),
            r(x + w),
            r(y + h)
        );
        let range = secondary_scale;
        for tick in &secondary_ticks {
            let ratio = tick / range;
            if !(-0.001..=1.001).contains(&ratio) {
                continue;
            }
            let tick_y = y + h - ratio * h;
            let _ = write!(
                out,
                "<text x=\"{}\" y=\"{}\" text-anchor=\"start\" font-size=\"12\" fill=\"#595959\">{}</text>",
                r(x + w + 5.0),
                r(tick_y + 4.0),
                escape_xml_text(&format_tick_value(*tick))
            );
            let _ = write!(
                out,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#D9D9D9\" stroke-width=\"1\"/>",
                r(x + w),
                r(tick_y),
                r(x + w + 3.0),
                r(tick_y)
            );
        }
    }

    let divisor = if cat_count > 1 {
        (cat_count - 1) as f64
    } else {
        1.0
    };
    for c in 0..cat_count {
        let label = chart.categories.get(c).map_or("", String::as_str);
        let label_x = x + (c as f64 / divisor) * w;
        let _ = write!(
            out,
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#595959\">{}</text>",
            r(label_x),
            r(y + h + 15.0),
            escape_xml_text(label)
        );
    }

    let bar_series_count = chart
        .series
        .iter()
        .filter(|s| s.sub_chart_type == Some(ChartType::Bar))
        .count();
    let group_w = w / cat_count as f64;
    let bar_w = if bar_series_count > 0 {
        (group_w * 0.7) / bar_series_count as f64
    } else {
        0.0
    };
    let group_pad = group_w * 0.15;

    let mut bar_idx = 0_usize;
    for s in &chart.series {
        let scale = if s.axis_group == Some(AxisGroup::Secondary) {
            secondary_scale
        } else {
            primary_scale
        };
        if s.sub_chart_type == Some(ChartType::Bar) {
            for (c, val) in s.values.iter().enumerate() {
                let bar_h = (val / scale) * h;
                let bar_x = x + c as f64 * group_w + group_pad + bar_idx as f64 * bar_w;
                let bar_y = y + h - bar_h;
                let pt_color = point_color(s, c as u32);
                let _ = write!(
                    out,
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" {}/>",
                    r(bar_x),
                    r(bar_y),
                    r(bar_w),
                    r(bar_h),
                    fill_attr(&pt_color)
                );
            }
            bar_idx += 1;
            continue;
        }

        let pts: Vec<(f64, f64)> = s
            .values
            .iter()
            .enumerate()
            .map(|(i, v)| (x + (i as f64 / divisor) * w, y + h - (v / scale) * h))
            .collect();
        let opacity = if s.color.alpha < 1.0 {
            format!(" stroke-opacity=\"{}\"", s.color.alpha)
        } else {
            String::new()
        };
        let stroke_hex = color_hex(&s.color);
        if s.smooth.unwrap_or(false) && pts.len() >= 2 {
            let d = build_smooth_path_d(&pts);
            let _ = write!(
                out,
                "<path d=\"{d}\" fill=\"none\" stroke=\"{stroke_hex}\" stroke-width=\"2\"{opacity}/>"
            );
        } else {
            let mut points_str = String::new();
            for (i, p) in pts.iter().enumerate() {
                if i > 0 {
                    points_str.push(' ');
                }
                let _ = write!(points_str, "{},{}", r(p.0), r(p.1));
            }
            let _ = write!(
                out,
                "<polyline points=\"{points_str}\" fill=\"none\" stroke=\"{stroke_hex}\" stroke-width=\"2\"{opacity}/>"
            );
        }
        for p in &pts {
            let _ = write!(
                out,
                "<circle cx=\"{}\" cy=\"{}\" r=\"3\" {}/>",
                r(p.0),
                r(p.1),
                fill_attr(&s.color)
            );
        }
    }
    out
}

pub(super) fn max_value_refs(series: &[&ChartSeries]) -> f64 {
    series
        .iter()
        .flat_map(|s| s.values.iter().copied())
        .fold(0.0_f64, f64::max)
}
