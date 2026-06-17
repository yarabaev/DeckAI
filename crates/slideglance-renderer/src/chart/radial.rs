//! Chart-type renderers — radial (pie / doughnut / of-pie / radar) family.
//!
//! Extracted from `chart/render.rs` so each top-level dispatch case
//! lives next to the SVG emission code it drives. Helpers come from
//! `chart::common`; trendline overlays from `chart::trendline`.

use std::f64::consts::PI;
use std::fmt::Write as _;

use slideglance_color::{ResolvedColor, Rgb};
use slideglance_model::{ChartData, OfPieType, RadarStyle};

use crate::color::color_hex;
use crate::svg_builder::escape_xml_text;

use super::common::{
    compose_data_label, emit_data_label, fill_attr, get_max_value, pie_slice_color,
    point_explosion, r, resolve_series_data_labels,
};

#[allow(clippy::many_single_char_names)]
pub(super) fn render_pie_chart(chart: &ChartData, x: f64, y: f64, w: f64, h: f64) -> String {
    let series = match chart.series.first() {
        Some(s) if !s.values.is_empty() => s,
        _ => return String::new(),
    };
    let total: f64 = series.values.iter().sum();
    if total == 0.0 {
        return String::new();
    }
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let radius = (w.min(h) / 2.0) * 0.85;
    let labels = resolve_series_data_labels(chart, series);
    let mut current_angle = -PI / 2.0;
    let mut out = String::new();
    let single = series.values.len() == 1;
    for (i, val) in series.values.iter().enumerate() {
        let slice_angle = (val / total) * 2.0 * PI;
        let color = pie_slice_color(i, chart);
        let explosion_pct = point_explosion(series, i as u32);
        let offset = if explosion_pct > 0.0 {
            radius * (explosion_pct / 100.0)
        } else {
            0.0
        };
        let mid = current_angle + slice_angle / 2.0;
        let ox = cx + offset * mid.cos();
        let oy = cy + offset * mid.sin();
        if single {
            let _ = write!(
                out,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" {}/>",
                r(ox),
                r(oy),
                r(radius),
                fill_attr(&color)
            );
        } else {
            let x1 = ox + radius * current_angle.cos();
            let y1 = oy + radius * current_angle.sin();
            let x2 = ox + radius * (current_angle + slice_angle).cos();
            let y2 = oy + radius * (current_angle + slice_angle).sin();
            let large_arc = i32::from(slice_angle > PI);
            let _ = write!(
                out,
                "<path d=\"M{},{} L{},{} A{},{} 0 {large_arc},1 {},{} Z\" {}/>",
                r(ox),
                r(oy),
                r(x1),
                r(y1),
                r(radius),
                r(radius),
                r(x2),
                r(y2),
                fill_attr(&color)
            );
        }
        if let Some(labels) = &labels {
            let label_r = radius * 0.65;
            let cat = chart.categories.get(i).map(String::as_str);
            let percent = val / total;
            if let Some(text) = compose_data_label(
                Some(labels),
                *val,
                cat,
                series.name.as_deref(),
                Some(percent),
            ) {
                emit_data_label(
                    &mut out,
                    &text,
                    ox + label_r * mid.cos(),
                    oy + label_r * mid.sin() + 4.0,
                    "middle",
                );
            }
        }
        current_angle += slice_angle;
    }
    out
}

#[allow(clippy::many_single_char_names)]
pub(super) fn render_doughnut_chart(chart: &ChartData, x: f64, y: f64, w: f64, h: f64) -> String {
    let series = match chart.series.first() {
        Some(s) if !s.values.is_empty() => s,
        _ => return String::new(),
    };
    let total: f64 = series.values.iter().sum();
    if total == 0.0 {
        return String::new();
    }
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let outer_r = (w.min(h) / 2.0) * 0.85;
    let hole_size = chart.hole_size.unwrap_or(50.0);
    let inner_r = outer_r * (hole_size / 100.0);
    let mut current_angle = -PI / 2.0;
    let mut out = String::new();
    let single = series.values.len() == 1;
    for (i, val) in series.values.iter().enumerate() {
        let slice_angle = (val / total) * 2.0 * PI;
        let color = pie_slice_color(i, chart);
        if single {
            let _ = write!(
                out,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" {}/><circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#FFFFFF\"/>",
                r(cx),
                r(cy),
                r(outer_r),
                fill_attr(&color),
                r(cx),
                r(cy),
                r(inner_r)
            );
        } else {
            let ox1 = cx + outer_r * current_angle.cos();
            let oy1 = cy + outer_r * current_angle.sin();
            let ox2 = cx + outer_r * (current_angle + slice_angle).cos();
            let oy2 = cy + outer_r * (current_angle + slice_angle).sin();
            let ix1 = cx + inner_r * (current_angle + slice_angle).cos();
            let iy1 = cy + inner_r * (current_angle + slice_angle).sin();
            let ix2 = cx + inner_r * current_angle.cos();
            let iy2 = cy + inner_r * current_angle.sin();
            let large_arc = i32::from(slice_angle > PI);
            let _ = write!(
                out,
                "<path d=\"M{},{} A{},{} 0 {large_arc},1 {},{} L{},{} A{},{} 0 {large_arc},0 {},{} Z\" {}/>",
                r(ox1),
                r(oy1),
                r(outer_r),
                r(outer_r),
                r(ox2),
                r(oy2),
                r(ix1),
                r(iy1),
                r(inner_r),
                r(inner_r),
                r(ix2),
                r(iy2),
                fill_attr(&color)
            );
        }
        current_angle += slice_angle;
    }
    out
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
pub(super) fn render_of_pie_chart(chart: &ChartData, x: f64, y: f64, w: f64, h: f64) -> String {
    let series = match chart.series.first() {
        Some(s) if !s.values.is_empty() => s,
        _ => return String::new(),
    };
    let total: f64 = series.values.iter().sum();
    if total == 0.0 {
        return String::new();
    }
    let split_pos = chart.split_pos.unwrap_or(2.0) as usize;
    let second_pie_size = chart.second_pie_size.unwrap_or(75.0);
    let is_bar_of_pie = chart.of_pie_type == Some(OfPieType::Bar);

    let split_idx = series.values.len().saturating_sub(split_pos);
    let primary_values: &[f64] = &series.values[..split_idx];
    let secondary_values: &[f64] = &series.values[split_idx..];
    let secondary_total: f64 = secondary_values.iter().sum();

    let pie_w = w * 0.45;
    let pie_cx = x + pie_w / 2.0;
    let pie_cy = y + h / 2.0;
    let pie_r = (pie_w.min(h) / 2.0) * 0.85;

    let mut out = String::new();
    let mut current_angle = -PI / 2.0;
    for (i, val) in primary_values.iter().enumerate() {
        let slice_angle = (val / total) * 2.0 * PI;
        let color = pie_slice_color(i, chart);
        let x1 = pie_cx + pie_r * current_angle.cos();
        let y1 = pie_cy + pie_r * current_angle.sin();
        let x2 = pie_cx + pie_r * (current_angle + slice_angle).cos();
        let y2 = pie_cy + pie_r * (current_angle + slice_angle).sin();
        let large_arc = i32::from(slice_angle > PI);
        let _ = write!(
            out,
            "<path d=\"M{},{} L{},{} A{},{} 0 {large_arc},1 {},{} Z\" {}/>",
            r(pie_cx),
            r(pie_cy),
            r(x1),
            r(y1),
            r(pie_r),
            r(pie_r),
            r(x2),
            r(y2),
            fill_attr(&color)
        );
        current_angle += slice_angle;
    }

    let other_angle_start = current_angle;
    let other_slice_angle = (secondary_total / total) * 2.0 * PI;
    let other_color = ResolvedColor::new(Rgb::new(0xD9, 0xD9, 0xD9), 1.0);
    if primary_values.is_empty() && !secondary_values.is_empty() {
        let _ = write!(
            out,
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" {}/>",
            r(pie_cx),
            r(pie_cy),
            r(pie_r),
            fill_attr(&other_color)
        );
    } else if secondary_total > 0.0 {
        let x1 = pie_cx + pie_r * other_angle_start.cos();
        let y1 = pie_cy + pie_r * other_angle_start.sin();
        let x2 = pie_cx + pie_r * (other_angle_start + other_slice_angle).cos();
        let y2 = pie_cy + pie_r * (other_angle_start + other_slice_angle).sin();
        let large_arc = i32::from(other_slice_angle > PI);
        let _ = write!(
            out,
            "<path d=\"M{},{} L{},{} A{},{} 0 {large_arc},1 {},{} Z\" {}/>",
            r(pie_cx),
            r(pie_cy),
            r(x1),
            r(y1),
            r(pie_r),
            r(pie_r),
            r(x2),
            r(y2),
            fill_attr(&other_color)
        );
    }

    let sec_w = w * 0.25;
    let sec_h = h * (second_pie_size / 100.0) * 0.85;
    let sec_x = x + w * 0.65;
    let sec_cy = y + h / 2.0;

    let line_start_x = pie_cx + pie_r * other_angle_start.cos();
    let line_start_y = pie_cy + pie_r * other_angle_start.sin();
    let line_end_start_x = pie_cx + pie_r * (other_angle_start + other_slice_angle).cos();
    let line_end_start_y = pie_cy + pie_r * (other_angle_start + other_slice_angle).sin();
    let _ = write!(
        out,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#A6A6A6\" stroke-width=\"1\"/>",
        r(line_start_x),
        r(line_start_y),
        r(sec_x),
        r(sec_cy - sec_h / 2.0)
    );
    let _ = write!(
        out,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#A6A6A6\" stroke-width=\"1\"/>",
        r(line_end_start_x),
        r(line_end_start_y),
        r(sec_x),
        r(sec_cy + sec_h / 2.0)
    );

    if is_bar_of_pie {
        let mut bar_y = sec_cy - sec_h / 2.0;
        for (i, val) in secondary_values.iter().enumerate() {
            let bar_h = if secondary_total > 0.0 {
                (val / secondary_total) * sec_h
            } else {
                0.0
            };
            let color = pie_slice_color(split_idx + i, chart);
            let _ = write!(
                out,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" {}/>",
                r(sec_x),
                r(bar_y),
                r(sec_w),
                r(bar_h),
                fill_attr(&color)
            );
            bar_y += bar_h;
        }
    } else {
        let sec_pie_cx = sec_x + sec_w / 2.0;
        let sec_r = sec_w.min(sec_h) / 2.0;
        let mut sec_angle = -PI / 2.0;
        if secondary_values.len() == 1 {
            let color = pie_slice_color(split_idx, chart);
            let _ = write!(
                out,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" {}/>",
                r(sec_pie_cx),
                r(sec_cy),
                r(sec_r),
                fill_attr(&color)
            );
        } else {
            for (i, val) in secondary_values.iter().enumerate() {
                let slice_angle = if secondary_total > 0.0 {
                    (val / secondary_total) * 2.0 * PI
                } else {
                    0.0
                };
                let color = pie_slice_color(split_idx + i, chart);
                let sx1 = sec_pie_cx + sec_r * sec_angle.cos();
                let sy1 = sec_cy + sec_r * sec_angle.sin();
                let sx2 = sec_pie_cx + sec_r * (sec_angle + slice_angle).cos();
                let sy2 = sec_cy + sec_r * (sec_angle + slice_angle).sin();
                let large_arc = i32::from(slice_angle > PI);
                let _ = write!(
                    out,
                    "<path d=\"M{},{} L{},{} A{},{} 0 {large_arc},1 {},{} Z\" {}/>",
                    r(sec_pie_cx),
                    r(sec_cy),
                    r(sx1),
                    r(sy1),
                    r(sec_r),
                    r(sec_r),
                    r(sx2),
                    r(sy2),
                    fill_attr(&color)
                );
                sec_angle += slice_angle;
            }
        }
    }
    out
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
pub(super) fn render_radar_chart(chart: &ChartData, x: f64, y: f64, w: f64, h: f64) -> String {
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

    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let radius = (w.min(h) / 2.0) * 0.85;
    let grid_levels: i32 = 5;

    let mut out = String::new();
    for level in 1..=grid_levels {
        let lr = (radius / f64::from(grid_levels)) * f64::from(level);
        let _ = write!(
            out,
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"#D9D9D9\" stroke-width=\"0.5\"/>",
            r(cx),
            r(cy),
            r(lr)
        );
    }

    for i in 0..cat_count {
        let angle = (i as f64 / cat_count as f64) * 2.0 * PI - PI / 2.0;
        let ax = cx + radius * angle.cos();
        let ay = cy + radius * angle.sin();
        let _ = write!(
            out,
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#D9D9D9\" stroke-width=\"0.5\"/>",
            r(cx),
            r(cy),
            r(ax),
            r(ay)
        );
        let label = chart.categories.get(i).map_or("", String::as_str);
        if !label.is_empty() {
            let label_r = radius + 12.0;
            let lx = cx + label_r * angle.cos();
            let ly = cy + label_r * angle.sin();
            let cos_a = angle.cos();
            let anchor = if cos_a.abs() < 0.01 {
                "middle"
            } else if cos_a > 0.0 {
                "start"
            } else {
                "end"
            };
            let _ = write!(
                out,
                "<text x=\"{}\" y=\"{}\" text-anchor=\"{anchor}\" font-size=\"12\" fill=\"#595959\">{}</text>",
                r(lx),
                r(ly + 4.0),
                escape_xml_text(label)
            );
        }
    }

    let is_filled = chart.radar_style == Some(RadarStyle::Filled);
    let show_markers = chart.radar_style == Some(RadarStyle::Marker);

    for s in &chart.series {
        let mut points = String::new();
        let mut coords: Vec<(f64, f64)> = Vec::new();
        for i in 0..cat_count {
            let val = s.values.get(i).copied().unwrap_or(0.0);
            let angle = (i as f64 / cat_count as f64) * 2.0 * PI - PI / 2.0;
            let lr = (val / max_val) * radius;
            let px = cx + lr * angle.cos();
            let py = cy + lr * angle.sin();
            if !points.is_empty() {
                points.push(' ');
            }
            let _ = write!(points, "{},{}", r(px), r(py));
            coords.push((px, py));
        }
        let stroke_hex = color_hex(&s.color);
        let stroke_opacity = if s.color.alpha < 1.0 {
            format!(" stroke-opacity=\"{}\"", s.color.alpha)
        } else {
            String::new()
        };
        if is_filled {
            let _ = write!(
                out,
                "<polygon points=\"{points}\" fill=\"{stroke_hex}\" fill-opacity=\"0.3\" stroke=\"{stroke_hex}\" stroke-width=\"2\"{stroke_opacity}/>"
            );
        } else {
            let _ = write!(
                out,
                "<polygon points=\"{points}\" fill=\"none\" stroke=\"{stroke_hex}\" stroke-width=\"2\"{stroke_opacity}/>"
            );
        }
        if show_markers {
            for (px, py) in &coords {
                let _ = write!(
                    out,
                    "<circle cx=\"{}\" cy=\"{}\" r=\"3\" {}/>",
                    r(*px),
                    r(*py),
                    fill_attr(&s.color)
                );
            }
        }
    }
    out
}
