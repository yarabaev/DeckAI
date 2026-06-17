//! Chart-type renderers — cartesian (bar / line / area / scatter / bubble) family.
//!
//! Extracted from `chart/render.rs` so each top-level dispatch case
//! lives next to the SVG emission code it drives. Helpers come from
//! `chart::common`; trendline overlays from `chart::trendline`.

use std::fmt::Write as _;

use slideglance_model::{BarDirection, ChartData};

use crate::color::color_hex;
use crate::svg_builder::escape_xml_text;

use super::common::{
    build_smooth_path_d, compose_data_label, compute_nice_ticks, emit_data_label, fill_attr,
    format_tick_value, get_max_value, point_color, r, render_value_axis_labels,
    resolve_series_data_labels, value_axis_options,
};
use super::trendline::render_trendlines;

#[allow(clippy::too_many_lines)]
pub(super) fn render_bar_chart(chart: &ChartData, x: f64, y: f64, w: f64, h: f64) -> String {
    if chart.series.is_empty() {
        return String::new();
    }
    let max_val = get_max_value(&chart.series);
    if max_val == 0.0 {
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

    let is_horizontal = matches!(chart.bar_direction, Some(BarDirection::Bar));
    let target_count = ((h / 30.0).floor() as i64).max(2) as u32;
    let ticks = compute_nice_ticks(0.0, max_val, target_count);
    let scale_max = *ticks.last().unwrap_or(&max_val);
    if scale_max == 0.0 {
        return String::new();
    }

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

    if is_horizontal {
        for &tick in &ticks {
            let ratio = tick / scale_max;
            if !(-0.001..=1.001).contains(&ratio) {
                continue;
            }
            let tick_x = x + ratio * w;
            let _ = write!(
                out,
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#595959\">{}</text>",
                r(tick_x),
                r(y + h + 15.0),
                escape_xml_text(&format_tick_value(tick))
            );
        }
    } else {
        out.push_str(&render_value_axis_labels(
            &ticks,
            0.0,
            scale_max,
            x,
            y,
            h,
            value_axis_options(chart, w),
        ));
    }

    if is_horizontal {
        let group_h = h / cat_count as f64;
        let bar_h = (group_h * 0.7) / chart.series.len() as f64;
        let group_pad = group_h * 0.15;
        for c in 0..cat_count {
            let label = chart.categories.get(c).map_or("", String::as_str);
            let label_y = y + c as f64 * group_h + group_h / 2.0;
            let _ = write!(
                out,
                "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"12\" fill=\"#595959\">{}</text>",
                r(x - 5.0),
                r(label_y + 4.0),
                escape_xml_text(label)
            );
        }
        for (s_idx, s) in chart.series.iter().enumerate() {
            let labels = resolve_series_data_labels(chart, s);
            for (c, val) in s.values.iter().enumerate() {
                let bar_w = (val / scale_max) * w;
                let bar_x = x;
                let bar_y = y + c as f64 * group_h + group_pad + s_idx as f64 * bar_h;
                let _ = write!(
                    out,
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" {}/>",
                    r(bar_x),
                    r(bar_y),
                    r(bar_w),
                    r(bar_h),
                    fill_attr(&s.color)
                );
                let cat = chart.categories.get(c).map(String::as_str);
                if let Some(text) =
                    compose_data_label(labels.as_ref(), *val, cat, s.name.as_deref(), None)
                {
                    emit_data_label(
                        &mut out,
                        &text,
                        bar_x + bar_w + 4.0,
                        bar_y + bar_h / 2.0 + 4.0,
                        "start",
                    );
                }
            }
        }
    } else {
        let group_w = w / cat_count as f64;
        let bar_w = (group_w * 0.7) / chart.series.len() as f64;
        let group_pad = group_w * 0.15;
        for c in 0..cat_count {
            let label = chart.categories.get(c).map_or("", String::as_str);
            let label_x = x + c as f64 * group_w + group_w / 2.0;
            let _ = write!(
                out,
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#595959\">{}</text>",
                r(label_x),
                r(y + h + 15.0),
                escape_xml_text(label)
            );
        }
        for (s_idx, s) in chart.series.iter().enumerate() {
            let labels = resolve_series_data_labels(chart, s);
            for (c, val) in s.values.iter().enumerate() {
                let bar_h = (val / scale_max) * h;
                let bar_x = x + c as f64 * group_w + group_pad + s_idx as f64 * bar_w;
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
                let cat = chart.categories.get(c).map(String::as_str);
                if let Some(text) =
                    compose_data_label(labels.as_ref(), *val, cat, s.name.as_deref(), None)
                {
                    emit_data_label(&mut out, &text, bar_x + bar_w / 2.0, bar_y - 4.0, "middle");
                }
            }
            // Trendline overlay (vertical bar only — horizontal bars are
            // a rare case and TS skips them too).
            let mut to_px =
                |i: f64, v: f64| (x + i * group_w + group_w / 2.0, y + h - (v / scale_max) * h);
            out.push_str(&render_trendlines(
                s.trendlines.as_deref(),
                &s.values,
                &s.color,
                &mut to_px,
            ));
        }
    }
    out
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
pub(super) fn render_line_chart(chart: &ChartData, x: f64, y: f64, w: f64, h: f64) -> String {
    if chart.series.is_empty() {
        return String::new();
    }
    let max_val = get_max_value(&chart.series);
    if max_val == 0.0 {
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

    let target_count = ((h / 30.0).floor() as i64).max(2) as u32;
    let ticks = compute_nice_ticks(0.0, max_val, target_count);
    let scale_max = *ticks.last().unwrap_or(&max_val);
    if scale_max == 0.0 {
        return String::new();
    }

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
        0.0,
        scale_max,
        x,
        y,
        h,
        value_axis_options(chart, w),
    ));

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

    for s in &chart.series {
        let labels = resolve_series_data_labels(chart, s);
        let pts: Vec<(f64, f64)> = s
            .values
            .iter()
            .enumerate()
            .map(|(i, v)| (x + (i as f64 / divisor) * w, y + h - (v / scale_max) * h))
            .collect();
        let opacity = if s.color.alpha < 1.0 {
            format!(" stroke-opacity=\"{}\"", s.color.alpha)
        } else {
            String::new()
        };
        let stroke_hex = crate::color::color_hex(&s.color);
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
        for (i, p) in pts.iter().enumerate() {
            let _ = write!(
                out,
                "<circle cx=\"{}\" cy=\"{}\" r=\"3\" {}/>",
                r(p.0),
                r(p.1),
                fill_attr(&s.color)
            );
            let cat = chart.categories.get(i).map(String::as_str);
            if let Some(text) =
                compose_data_label(labels.as_ref(), s.values[i], cat, s.name.as_deref(), None)
            {
                emit_data_label(&mut out, &text, p.0, p.1 - 8.0, "middle");
            }
        }
        // Trendline overlay.
        let mut to_px = |i: f64, v: f64| (x + (i / divisor) * w, y + h - (v / scale_max) * h);
        out.push_str(&render_trendlines(
            s.trendlines.as_deref(),
            &s.values,
            &s.color,
            &mut to_px,
        ));
    }
    out
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
pub(super) fn render_area_chart(chart: &ChartData, x: f64, y: f64, w: f64, h: f64) -> String {
    if chart.series.is_empty() {
        return String::new();
    }
    let max_val = get_max_value(&chart.series);
    if max_val == 0.0 {
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
    let target_count = ((h / 30.0).floor() as i64).max(2) as u32;
    let ticks = compute_nice_ticks(0.0, max_val, target_count);
    let scale_max = *ticks.last().unwrap_or(&max_val);
    if scale_max == 0.0 {
        return String::new();
    }
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
        0.0,
        scale_max,
        x,
        y,
        h,
        value_axis_options(chart, w),
    ));

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

    let baseline = y + h;
    for s in &chart.series {
        let pts: Vec<(f64, f64)> = s
            .values
            .iter()
            .enumerate()
            .map(|(i, v)| (x + (i as f64 / divisor) * w, y + h - (v / scale_max) * h))
            .collect();
        if pts.is_empty() {
            continue;
        }
        let mut top = String::new();
        for (i, p) in pts.iter().enumerate() {
            if i > 0 {
                top.push(' ');
            }
            let _ = write!(top, "{},{}", r(p.0), r(p.1));
        }
        let last_x = pts.last().map_or(0.0, |p| p.0);
        let first_x = pts.first().map_or(0.0, |p| p.0);
        let stroke_hex = color_hex(&s.color);
        let fill_opacity = if s.color.alpha < 1.0 {
            s.color.alpha
        } else {
            0.5
        };
        let stroke_opacity = if s.color.alpha < 1.0 {
            format!(" stroke-opacity=\"{}\"", s.color.alpha)
        } else {
            String::new()
        };
        let _ = write!(
            out,
            "<polygon points=\"{top} {},{} {},{}\" fill=\"{stroke_hex}\" fill-opacity=\"{fill_opacity}\" stroke=\"{stroke_hex}\" stroke-width=\"2\"{stroke_opacity}/>",
            r(last_x),
            r(baseline),
            r(first_x),
            r(baseline)
        );
    }
    out
}

#[allow(clippy::many_single_char_names)]
pub(super) fn render_scatter_chart(chart: &ChartData, x: f64, y: f64, w: f64, h: f64) -> String {
    if chart.series.is_empty() {
        return String::new();
    }
    let mut max_x = 0.0_f64;
    let mut max_y = 0.0_f64;
    for s in &chart.series {
        if let Some(xs) = &s.x_values {
            for v in xs {
                max_x = max_x.max(*v);
            }
        }
        for v in &s.values {
            max_y = max_y.max(*v);
        }
    }
    if max_x == 0.0 {
        max_x = 1.0;
    }
    if max_y == 0.0 {
        max_y = 1.0;
    }

    let target = ((h / 30.0).floor() as i64).max(2) as u32;
    let ticks = compute_nice_ticks(0.0, max_y, target);
    let scale_max_y = *ticks.last().unwrap_or(&max_y);

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
        0.0,
        scale_max_y,
        x,
        y,
        h,
        value_axis_options(chart, w),
    ));

    for s in &chart.series {
        let xs = s.x_values.as_deref().unwrap_or(&[]);
        for (i, y_val) in s.values.iter().enumerate() {
            let x_val = xs.get(i).copied().unwrap_or(i as f64);
            let px = x + (x_val / max_x) * w;
            let py = y + h - (y_val / scale_max_y) * h;
            let _ = write!(
                out,
                "<circle cx=\"{}\" cy=\"{}\" r=\"4\" {}/>",
                r(px),
                r(py),
                fill_attr(&s.color)
            );
        }
    }
    out
}

#[allow(clippy::many_single_char_names)]
pub(super) fn render_bubble_chart(chart: &ChartData, x: f64, y: f64, w: f64, h: f64) -> String {
    if chart.series.is_empty() {
        return String::new();
    }
    let mut max_x = 0.0_f64;
    let mut max_y = 0.0_f64;
    let mut max_bubble = 0.0_f64;
    for s in &chart.series {
        if let Some(xs) = &s.x_values {
            for v in xs {
                max_x = max_x.max(*v);
            }
        }
        for v in &s.values {
            max_y = max_y.max(*v);
        }
        if let Some(sizes) = &s.bubble_sizes {
            for v in sizes {
                max_bubble = max_bubble.max(*v);
            }
        }
    }
    if max_x == 0.0 {
        max_x = 1.0;
    }
    if max_y == 0.0 {
        max_y = 1.0;
    }
    if max_bubble == 0.0 {
        max_bubble = 1.0;
    }

    let max_radius = w.min(h) * 0.08;
    let target = ((h / 30.0).floor() as i64).max(2) as u32;
    let ticks = compute_nice_ticks(0.0, max_y, target);
    let scale_max_y = *ticks.last().unwrap_or(&max_y);

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
        0.0,
        scale_max_y,
        x,
        y,
        h,
        value_axis_options(chart, w),
    ));

    for s in &chart.series {
        let xs = s.x_values.as_deref().unwrap_or(&[]);
        let sizes = s.bubble_sizes.as_deref().unwrap_or(&[]);
        for (i, y_val) in s.values.iter().enumerate() {
            let x_val = xs.get(i).copied().unwrap_or(i as f64);
            let size = sizes.get(i).copied().unwrap_or(1.0);
            let px = x + (x_val / max_x) * w;
            let py = y + h - (y_val / scale_max_y) * h;
            let radius = ((size / max_bubble).sqrt() * max_radius).max(2.0);
            let _ = write!(
                out,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" {} fill-opacity=\"0.6\"/>",
                r(px),
                r(py),
                r(radius),
                fill_attr(&s.color)
            );
        }
    }
    out
}
