//! Trendline overlays for cartesian (bar / line / area / combo) charts.
//!
//! Direct port of lines
//! 1380–1687. Implements the six OOXML trendline types:
//!
//! - `linear`: ordinary least-squares fit, drawn as a single line.
//! - `exp`: `y = a · e^(b·x)` via log-linearisation.
//! - `log`: `y = a · ln(x+1) + b` (1-based shift to dodge ln(0)).
//! - `power`: `y = a · x^b` via log-log linearisation.
//! - `poly`: order-N polynomial via Vandermonde + Gaussian elimination
//!   (capped at order 6).
//! - `movingAvg`: trailing window average; emits multiple `M..L..` segments
//!   when intermediate samples are unavailable.
//!
//! All trendlines are drawn dashed (`4,3`) at 1.5 px and 80% stroke
//! opacity to match the TS contract.

use std::fmt::Write as _;

use slideglance_color::ResolvedColor;
use slideglance_model::{ChartTrendline, TrendlineType};

use crate::chart::common::r;
use crate::color::color_hex;

// Sampling density used to convert smooth non-linear fits (exp / log /
// power / poly) into approximate polylines. Linear fits emit two
// endpoints; moving averages reuse the original data resolution.
const SAMPLES_PER_POINT: usize = 4;

/// Render trendline overlays for one cartesian series.
///
/// `to_px` maps a category index `i` (0-based) and a y-axis value `v`
/// to plot-area pixel coordinates. The closure is sampled at every
/// data point for linear / movingAvg fits, and 4× per category for
/// the smooth non-linear curves to approximate them as polylines.
pub(crate) fn render_trendlines(
    trendlines: Option<&[ChartTrendline]>,
    values: &[f64],
    color: &ResolvedColor,
    to_px: &mut dyn FnMut(f64, f64) -> (f64, f64),
) -> String {
    let Some(tls) = trendlines else {
        return String::new();
    };
    if tls.is_empty() || values.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let hex = color_hex(color);

    for tl in tls {
        match tl.trendline_type {
            TrendlineType::MovingAvg => {
                let ma = compute_moving_average(values, tl.period.unwrap_or(2));
                emit_moving_average(&mut out, &ma, &hex, to_px);
            }
            TrendlineType::Exp => {
                let Some(fit) = compute_exponential_fit(values) else {
                    continue;
                };
                let samples = sample_curve(values.len(), SAMPLES_PER_POINT, |x| {
                    fit.a * (fit.b * x).exp()
                });
                emit_curve_path(&mut out, &samples, &hex, to_px);
            }
            TrendlineType::Log => {
                let Some(fit) = compute_logarithmic_fit(values) else {
                    continue;
                };
                let samples = sample_curve(values.len(), SAMPLES_PER_POINT, |x| {
                    fit.a * (x + 1.0).ln() + fit.b
                });
                emit_curve_path(&mut out, &samples, &hex, to_px);
            }
            TrendlineType::Power => {
                let Some(fit) = compute_power_fit(values) else {
                    continue;
                };
                let samples = sample_curve(values.len(), SAMPLES_PER_POINT, |x| {
                    fit.a * (x + 1.0).powf(fit.b)
                });
                emit_curve_path(&mut out, &samples, &hex, to_px);
            }
            TrendlineType::Poly => {
                // PowerPoint defaults to order 2; cap at 6.
                let order = tl.period.unwrap_or(2).clamp(2, 6) as usize;
                let Some(coef) = compute_polynomial_fit(values, order) else {
                    continue;
                };
                let samples = sample_curve(values.len(), SAMPLES_PER_POINT, |x| {
                    evaluate_polynomial(&coef, x)
                });
                emit_curve_path(&mut out, &samples, &hex, to_px);
            }
            TrendlineType::Linear => {
                let Some(fit) = compute_linear_fit(values) else {
                    continue;
                };
                let end_idx = (values.len() - 1) as f64;
                let (sx, sy) = to_px(0.0, fit.intercept);
                let (ex, ey) = to_px(end_idx, fit.intercept + fit.slope * end_idx);
                let _ = write!(
                    out,
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{hex}\" stroke-width=\"1.5\" stroke-dasharray=\"4,3\" stroke-opacity=\"0.8\"/>",
                    r(sx),
                    r(sy),
                    r(ex),
                    r(ey)
                );
            }
        }
    }
    out
}

fn emit_curve_path(
    out: &mut String,
    samples: &[(f64, f64)],
    hex: &str,
    to_px: &mut dyn FnMut(f64, f64) -> (f64, f64),
) {
    if samples.len() < 2 {
        return;
    }
    let mut d = String::new();
    for (i, (sx, sy)) in samples.iter().enumerate() {
        let (px, py) = to_px(*sx, *sy);
        if i == 0 {
            let _ = write!(d, "M{},{}", r(px), r(py));
        } else {
            let _ = write!(d, " L{},{}", r(px), r(py));
        }
    }
    let _ = write!(
        out,
        "<path d=\"{d}\" fill=\"none\" stroke=\"{hex}\" stroke-width=\"1.5\" stroke-dasharray=\"4,3\" stroke-opacity=\"0.8\"/>"
    );
}

fn emit_moving_average(
    out: &mut String,
    ma: &[Option<f64>],
    hex: &str,
    to_px: &mut dyn FnMut(f64, f64) -> (f64, f64),
) {
    let mut prev: Option<(f64, f64)> = None;
    let mut segments = String::new();
    for (i, v) in ma.iter().enumerate() {
        let Some(v) = v else {
            prev = None;
            continue;
        };
        let pt = to_px(i as f64, *v);
        if let Some((px, py)) = prev {
            if !segments.is_empty() {
                segments.push(' ');
            }
            let _ = write!(segments, "M{},{} L{},{}", r(px), r(py), r(pt.0), r(pt.1));
        }
        prev = Some(pt);
    }
    if !segments.is_empty() {
        let _ = write!(
            out,
            "<path d=\"{segments}\" fill=\"none\" stroke=\"{hex}\" stroke-width=\"1.5\" stroke-dasharray=\"4,3\" stroke-opacity=\"0.8\"/>"
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct LinearFit {
    slope: f64,
    intercept: f64,
}

// `sum_x` / `sum_y` / `sum_xy` / `sum_xx` are the textbook
// least-squares accumulators; renaming would obscure the formulas.
#[allow(clippy::similar_names)]
fn compute_linear_fit(values: &[f64]) -> Option<LinearFit> {
    let n = values.len();
    if n < 2 {
        return None;
    }
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    let mut sum_xx = 0.0;
    for (i, v) in values.iter().enumerate() {
        let xi = i as f64;
        let yi = *v;
        sum_x += xi;
        sum_y += yi;
        sum_xy += xi * yi;
        sum_xx += xi * xi;
    }
    let n_f = n as f64;
    let denom = n_f * sum_xx - sum_x * sum_x;
    if denom == 0.0 {
        return None;
    }
    let slope = (n_f * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / n_f;
    Some(LinearFit { slope, intercept })
}

#[derive(Debug, Clone, Copy)]
struct ExpLogFit {
    a: f64,
    b: f64,
}

fn compute_exponential_fit(values: &[f64]) -> Option<ExpLogFit> {
    let mut xs = Vec::new();
    let mut lns = Vec::new();
    for (i, v) in values.iter().enumerate() {
        if *v > 0.0 {
            xs.push(i as f64);
            lns.push(v.ln());
        }
    }
    if xs.len() < 2 {
        return None;
    }
    let fit = linear_fit_xy(&xs, &lns)?;
    Some(ExpLogFit {
        a: fit.intercept.exp(),
        b: fit.slope,
    })
}

fn compute_logarithmic_fit(values: &[f64]) -> Option<ExpLogFit> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for (i, v) in values.iter().enumerate() {
        xs.push(((i + 1) as f64).ln());
        ys.push(*v);
    }
    if xs.len() < 2 {
        return None;
    }
    let fit = linear_fit_xy(&xs, &ys)?;
    Some(ExpLogFit {
        a: fit.slope,
        b: fit.intercept,
    })
}

fn compute_power_fit(values: &[f64]) -> Option<ExpLogFit> {
    let mut xs = Vec::new();
    let mut lns = Vec::new();
    for (i, v) in values.iter().enumerate() {
        if *v > 0.0 {
            xs.push(((i + 1) as f64).ln());
            lns.push(v.ln());
        }
    }
    if xs.len() < 2 {
        return None;
    }
    let fit = linear_fit_xy(&xs, &lns)?;
    Some(ExpLogFit {
        a: fit.intercept.exp(),
        b: fit.slope,
    })
}

// Polynomial fit follows the spec's normal-equations + Gaussian
// elimination layout. Indexed loops mirror the row/column iteration over
// the matrix `a`; converting to iterator chains would obscure the parity
// because each step reads/writes neighbour rows.
#[allow(clippy::needless_range_loop, clippy::cast_possible_wrap)]
fn compute_polynomial_fit(values: &[f64], order: usize) -> Option<Vec<f64>> {
    let n = values.len();
    if n == 0 {
        return None;
    }
    let k = (order + 1).clamp(2, n);
    let mut a = vec![vec![0.0_f64; k]; k];
    let mut b = vec![0.0_f64; k];
    for (i, yi) in values.iter().enumerate() {
        let xi = i as f64;
        for row in 0..k {
            for col in 0..k {
                a[row][col] += xi.powi((row + col) as i32);
            }
            b[row] += xi.powi(row as i32) * yi;
        }
    }
    // Gaussian elimination with partial pivoting.
    for i in 0..k {
        let mut pivot = i;
        for rr in (i + 1)..k {
            if a[rr][i].abs() > a[pivot][i].abs() {
                pivot = rr;
            }
        }
        if a[pivot][i].abs() < 1e-12 {
            return None;
        }
        if pivot != i {
            a.swap(i, pivot);
            b.swap(i, pivot);
        }
        for rr in (i + 1)..k {
            let factor = a[rr][i] / a[i][i];
            for cc in i..k {
                a[rr][cc] -= factor * a[i][cc];
            }
            b[rr] -= factor * b[i];
        }
    }
    let mut coef = vec![0.0_f64; k];
    for i in (0..k).rev() {
        let mut sum = b[i];
        for cc in (i + 1)..k {
            sum -= a[i][cc] * coef[cc];
        }
        coef[i] = sum / a[i][i];
    }
    Some(coef)
}

fn evaluate_polynomial(coef: &[f64], x: f64) -> f64 {
    let mut result = 0.0;
    let mut xp = 1.0;
    for c in coef {
        result += c * xp;
        xp *= x;
    }
    result
}

#[allow(clippy::similar_names)]
fn linear_fit_xy(xs: &[f64], ys: &[f64]) -> Option<LinearFit> {
    let n = xs.len();
    if n < 2 || ys.len() != n {
        return None;
    }
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    let mut sum_xx = 0.0;
    for (xi, yi) in xs.iter().zip(ys.iter()) {
        sum_x += *xi;
        sum_y += *yi;
        sum_xy += xi * yi;
        sum_xx += xi * xi;
    }
    let n_f = n as f64;
    let denom = n_f * sum_xx - sum_x * sum_x;
    if denom == 0.0 {
        return None;
    }
    let slope = (n_f * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / n_f;
    Some(LinearFit { slope, intercept })
}

fn compute_moving_average(values: &[f64], period: u32) -> Vec<Option<f64>> {
    let safe_period = (period.max(2) as usize).min(values.len().max(1));
    let mut out = Vec::with_capacity(values.len());
    for i in 0..values.len() {
        if i + 1 < safe_period {
            out.push(None);
            continue;
        }
        let window = &values[(i + 1 - safe_period)..=i];
        let sum: f64 = window.iter().sum();
        out.push(Some(sum / safe_period as f64));
    }
    out
}

fn sample_curve(
    length: usize,
    samples_per_point: usize,
    mut f: impl FnMut(f64) -> f64,
) -> Vec<(f64, f64)> {
    let total_samples = (length.saturating_sub(1) * samples_per_point + 1).max(2);
    let mut out = Vec::with_capacity(total_samples);
    let denom = (total_samples - 1) as f64;
    let span = length.saturating_sub(1) as f64;
    for s in 0..total_samples {
        let x = if denom == 0.0 {
            0.0
        } else {
            span * s as f64 / denom
        };
        let y = f(x);
        if y.is_finite() {
            out.push((x, y));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_color::Rgb;

    fn rgb(hex: &str) -> ResolvedColor {
        ResolvedColor::new(Rgb::from_hex(hex).unwrap(), 1.0)
    }

    #[test]
    fn empty_trendlines_yields_empty() {
        let out = render_trendlines(None, &[1.0, 2.0, 3.0], &rgb("#000000"), &mut |i, v| (i, v));
        assert!(out.is_empty());
    }

    #[test]
    fn linear_emits_single_line() {
        let tls = vec![ChartTrendline {
            trendline_type: TrendlineType::Linear,
            period: None,
            name: None,
        }];
        let out = render_trendlines(
            Some(&tls),
            &[1.0, 2.0, 3.0, 4.0],
            &rgb("#000000"),
            &mut |i, v| (i, v),
        );
        assert!(out.starts_with("<line "), "{out}");
        assert!(out.contains("stroke-dasharray=\"4,3\""));
    }

    #[test]
    fn linear_fit_recovers_slope_one() {
        let f = compute_linear_fit(&[0.0, 1.0, 2.0, 3.0]).unwrap();
        assert!((f.slope - 1.0).abs() < 1e-9);
        assert!(f.intercept.abs() < 1e-9);
    }

    #[test]
    fn moving_average_emits_path() {
        let tls = vec![ChartTrendline {
            trendline_type: TrendlineType::MovingAvg,
            period: Some(2),
            name: None,
        }];
        let out = render_trendlines(
            Some(&tls),
            &[1.0, 3.0, 5.0, 7.0],
            &rgb("#FF0000"),
            &mut |i, v| (i * 10.0, v),
        );
        assert!(out.contains("<path d=\""), "{out}");
        assert!(out.contains("stroke=\"#FF0000\""));
    }

    #[test]
    fn moving_average_skips_first_period_minus_one_points() {
        let ma = compute_moving_average(&[1.0, 3.0, 5.0, 7.0], 3);
        assert_eq!(ma.len(), 4);
        assert!(ma[0].is_none());
        assert!(ma[1].is_none());
        // (1 + 3 + 5) / 3 = 3
        assert!((ma[2].unwrap() - 3.0).abs() < 1e-9);
        // (3 + 5 + 7) / 3 = 5
        assert!((ma[3].unwrap() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn exponential_fit_handles_pure_exp_data() {
        // y = 2 * e^(0.5 * x)
        let xs: Vec<f64> = (0..6).map(f64::from).collect();
        let values: Vec<f64> = xs.iter().map(|x| 2.0 * (0.5 * x).exp()).collect();
        let fit = compute_exponential_fit(&values).unwrap();
        assert!((fit.a - 2.0).abs() < 1e-6);
        assert!((fit.b - 0.5).abs() < 1e-6);
    }

    #[test]
    fn power_fit_handles_pure_power_data() {
        // y = 3 * (i+1)^2
        let values: Vec<f64> = (0..6).map(|i| 3.0 * f64::from(i + 1).powi(2)).collect();
        let fit = compute_power_fit(&values).unwrap();
        assert!((fit.a - 3.0).abs() < 1e-6);
        assert!((fit.b - 2.0).abs() < 1e-6);
    }

    #[test]
    fn polynomial_fit_recovers_quadratic() {
        // y = 1 + 2x + 3x^2
        let coef_in = [1.0, 2.0, 3.0];
        let values: Vec<f64> = (0..6)
            .map(|i| evaluate_polynomial(&coef_in, f64::from(i)))
            .collect();
        let coef = compute_polynomial_fit(&values, 2).unwrap();
        for (a, b) in coef.iter().zip(coef_in.iter()) {
            assert!((a - b).abs() < 1e-6, "coef mismatch: {a} vs {b}");
        }
    }

    #[test]
    fn polynomial_fit_caps_at_data_size() {
        // Asking for order 6 with only 3 points should still produce a fit
        // (caps to k = n).
        let coef = compute_polynomial_fit(&[1.0, 4.0, 9.0], 6).unwrap();
        assert!(!coef.is_empty());
    }

    #[test]
    fn poly_emits_dashed_path() {
        let tls = vec![ChartTrendline {
            trendline_type: TrendlineType::Poly,
            period: Some(2),
            name: None,
        }];
        let out = render_trendlines(
            Some(&tls),
            &[1.0, 4.0, 9.0, 16.0],
            &rgb("#000000"),
            &mut |i, v| (i, v),
        );
        assert!(out.contains("<path d=\""));
        assert!(out.contains("stroke-dasharray=\"4,3\""));
    }

    #[test]
    fn evaluate_polynomial_horner_style() {
        let coef = [1.0, 2.0, 3.0];
        // y = 1 + 2*2 + 3*4 = 17
        assert!((evaluate_polynomial(&coef, 2.0) - 17.0).abs() < 1e-9);
    }

    #[test]
    fn sample_curve_scales_across_data_range() {
        let samples = sample_curve(5, 4, |x| x * 2.0);
        // (5 - 1) * 4 + 1 = 17 samples
        assert_eq!(samples.len(), 17);
        // First sample at x=0, last at x=4.
        assert!((samples[0].0 - 0.0).abs() < 1e-9);
        assert!((samples.last().unwrap().0 - 4.0).abs() < 1e-9);
    }
}
