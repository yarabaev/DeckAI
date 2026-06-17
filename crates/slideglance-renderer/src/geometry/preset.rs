// Lint policy for this module:
// - `match_same_arms`: shape names map to formula-driven outputs; collapsing
//   semantically distinct presets that happen to produce identical formulas
// would obscure the TS parity table.
// - `unreadable_literal`: OOXML adj defaults are 5-digit hundredth-percentages
// (`16667`, `33333`, …); separators reduce readability vs. the TS source.
// - `similar_names`: short letter suffixes (`a1`/`a2`, `x1`/`y1`) match the TS
//   variable layout; renaming would diverge from the reference for no benefit.
#![allow(
    clippy::match_same_arms,
    clippy::unreadable_literal,
    clippy::similar_names
)]

//! Preset geometry generators (`<a:prstGeom prst="...">`).
//!
//! 1:1 port of.
//! `adj` values are stored as 100,000ths (e.g. `50000` = 50%) and indexed by
//! their OOXML name (`adj`, `adj1`, `adj2`, …). When a key is missing the
//! spec falls back to a per-shape default; we mirror those defaults
//! literal-for-literal.
//!
//! Output is **unstyled** — the shape renderer injects `fill=`/`stroke=`
//! attributes via prefix replacement on the leading tag, so individual
//! generators must not emit them. The only exception is `fill-rule="evenodd"`
//! for shapes that need it intrinsically (donut, frame, noSmoking) — that is
//! a structural attribute, not a paint attribute.

use std::collections::BTreeMap;
use std::f64::consts::PI;

use super::fallback_rect;
use super::fmt::n;

/// Adjustment-values map: OOXML name -> `100,000`-th value.
type Adj = BTreeMap<String, f64>;

#[inline]
fn a(adj: &Adj, key: &str, default: f64) -> f64 {
    adj.get(key).copied().unwrap_or(default)
}

#[inline]
fn frac(adj: &Adj, key: &str, default: f64) -> f64 {
    a(adj, key, default) / 100_000.0
}

/// Convert an OOXML 1/60,000-degree angle to radians.
#[inline]
fn ooxml_angle_to_radians(angle60k: f64) -> f64 {
    (angle60k / 60_000.0) * (PI / 180.0)
}

fn regular_polygon(w: f64, h: f64, sides: u32) -> String {
    use std::fmt::Write as _;
    let cx = w / 2.0;
    let cy = h / 2.0;
    let mut pts = String::new();
    for i in 0..sides {
        let angle = (PI * 2.0 * f64::from(i)) / f64::from(sides) - PI / 2.0;
        if i > 0 {
            pts.push(' ');
        }
        let _ = write!(
            pts,
            "{},{}",
            n(cx + cx * angle.cos()),
            n(cy + cy * angle.sin())
        );
    }
    format!("<polygon points=\"{pts}\"/>")
}

fn star_polygon(w: f64, h: f64, points: u32, inner_ratio: f64) -> String {
    use std::fmt::Write as _;
    let cx = w / 2.0;
    let cy = h / 2.0;
    let mut pts = String::new();
    let total = points * 2;
    for i in 0..total {
        let angle = (PI * 2.0 * f64::from(i)) / f64::from(total) - PI / 2.0;
        let r = if i % 2 == 0 { 1.0 } else { inner_ratio };
        if i > 0 {
            pts.push(' ');
        }
        let _ = write!(
            pts,
            "{},{}",
            n(cx + cx * r * angle.cos()),
            n(cy + cy * r * angle.sin())
        );
    }
    format!("<polygon points=\"{pts}\"/>")
}

/// Dispatch a preset name to its generator. Unknown presets fall back to a
/// plain `<rect>` matching the local box, mirroring the spec's
/// catch-all behavior.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn preset_geometry_svg(preset: &str, w: f64, h: f64, adj: &Adj) -> String {
    match preset {
        // --- Basic shapes ---
        "rect" => fallback_rect(w, h),
        "ellipse" => format!(
            "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"/>",
            n(w / 2.0),
            n(h / 2.0),
            n(w / 2.0),
            n(h / 2.0)
        ),
        "roundRect" => {
            let raw = a(adj, "adj", 16667.0);
            let aclamped = raw.clamp(0.0, 50000.0);
            let r = (aclamped / 100_000.0) * w.min(h);
            format!(
                "<rect width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\"/>",
                n(w),
                n(h),
                n(r),
                n(r)
            )
        }
        "triangle" => {
            let top_x = frac(adj, "adj", 50000.0) * w;
            format!(
                "<polygon points=\"{},0 {},{} 0,{}\"/>",
                n(top_x),
                n(w),
                n(h),
                n(h)
            )
        }
        "rtTriangle" => format!("<polygon points=\"0,0 {},{} 0,{}\"/>", n(w), n(h), n(h)),
        "diamond" => format!(
            "<polygon points=\"{},0 {},{} {},{} 0,{}\"/>",
            n(w / 2.0),
            n(w),
            n(h / 2.0),
            n(w / 2.0),
            n(h),
            n(h / 2.0)
        ),
        "parallelogram" => {
            let off = frac(adj, "adj", 25000.0) * w;
            format!(
                "<polygon points=\"{},0 {},0 {},{} 0,{}\"/>",
                n(off),
                n(w),
                n(w - off),
                n(h),
                n(h)
            )
        }
        "trapezoid" => {
            let off = frac(adj, "adj", 25000.0) * w;
            format!(
                "<polygon points=\"{},0 {},0 {},{} 0,{}\"/>",
                n(off),
                n(w - off),
                n(w),
                n(h),
                n(h)
            )
        }
        "pentagon" => regular_polygon(w, h, 5),
        "hexagon" => {
            let off = frac(adj, "adj", 25000.0) * w;
            format!(
                "<polygon points=\"{},0 {},0 {},{} {},{} {},{} 0,{}\"/>",
                n(off),
                n(w - off),
                n(w),
                n(h / 2.0),
                n(w - off),
                n(h),
                n(off),
                n(h),
                n(h / 2.0)
            )
        }
        "star4" => {
            let cx = w / 2.0;
            let cy = h / 2.0;
            let ir = 0.38;
            format!(
                "<polygon points=\"{cx_n},0 {x1},{y1} {w_n},{cy_n} {x2},{y2} {cx_n},{h_n} {x3},{y2} 0,{cy_n} {x3},{y1}\"/>",
                cx_n = n(cx),
                cy_n = n(cy),
                w_n = n(w),
                h_n = n(h),
                x1 = n(cx + cx * ir),
                y1 = n(cy - cy * ir),
                x2 = n(cx + cx * ir),
                y2 = n(cy + cy * ir),
                x3 = n(cx - cx * ir),
            )
        }
        "star5" => star_polygon(w, h, 5, 0.38),
        "rightArrow" => {
            // OOXML preset spec for right/left/leftRight/upDown arrows
            // sizes the head LENGTH (depth) by `adj2 * min(w, h) / 50000`,
            // i.e. it is anchored to the *shorter* side, not the full
            // width — without that clamp a long-and-narrow arrow (cx >> cy)
            // ends up with `head_l = adj2 * w` so the head consumes the
            // entire shape and the shaft vanishes (slide 30's
            // ↔ between Front-end / Back-end rendered as a navy diamond
            // because head_l = 0.5 * 79 swallowed both halves of the
            // 79×15 leftRightArrow).
            let head_w = frac(adj, "adj1", 50000.0) * h;
            let head_l = (frac(adj, "adj2", 50000.0) * 2.0 * w.min(h)).min(w);
            let body_top = (h - head_w) / 2.0;
            let body_bottom = h - body_top;
            let shaft_end = w - head_l;
            format!(
                "<polygon points=\"0,{bt} {se},{bt} {se},0 {w_n},{hh} {se},{h_n} {se},{bb} 0,{bb}\"/>",
                bt = n(body_top),
                bb = n(body_bottom),
                se = n(shaft_end),
                w_n = n(w),
                h_n = n(h),
                hh = n(h / 2.0)
            )
        }
        "leftArrow" => {
            let head_w = frac(adj, "adj1", 50000.0) * h;
            let head_l = (frac(adj, "adj2", 50000.0) * 2.0 * w.min(h)).min(w);
            let body_top = (h - head_w) / 2.0;
            let body_bottom = h - body_top;
            format!(
                "<polygon points=\"{hl},{bt} {hl},0 0,{hh} {hl},{h_n} {hl},{bb} {w_n},{bb} {w_n},{bt}\"/>",
                hl = n(head_l),
                bt = n(body_top),
                bb = n(body_bottom),
                hh = n(h / 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "upArrow" => {
            let head_w = frac(adj, "adj1", 50000.0) * w;
            let head_l = frac(adj, "adj2", 50000.0) * h;
            let body_left = (w - head_w) / 2.0;
            let body_right = w - body_left;
            format!(
                "<polygon points=\"{bl},{hl} 0,{hl} {hw},0 {w_n},{hl} {br},{hl} {br},{h_n} {bl},{h_n}\"/>",
                bl = n(body_left),
                br = n(body_right),
                hl = n(head_l),
                hw = n(w / 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "downArrow" => {
            let head_w = frac(adj, "adj1", 50000.0) * w;
            let head_l = frac(adj, "adj2", 50000.0) * h;
            let body_left = (w - head_w) / 2.0;
            let body_right = w - body_left;
            let shaft_end = h - head_l;
            format!(
                "<polygon points=\"{bl},0 {br},0 {br},{se} {w_n},{se} {hw},{h_n} 0,{se} {bl},{se}\"/>",
                bl = n(body_left),
                br = n(body_right),
                se = n(shaft_end),
                hw = n(w / 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "line" => format!("<path d=\"M 0 0 L {} {}\"/>", n(w), n(h)),

        // --- Connector shapes ---
        "straightConnector1" => format!("<path d=\"M 0 0 L {} {}\"/>", n(w), n(h)),
        "bentConnector2" => format!("<path d=\"M 0 0 L {w_n} 0 L {w_n} {h_n}\"/>", w_n = n(w), h_n = n(h)),
        "bentConnector3" => {
            let mid_x = frac(adj, "adj1", 50000.0) * w;
            format!(
                "<path d=\"M 0 0 L {mx} 0 L {mx} {h_n} L {w_n} {h_n}\"/>",
                mx = n(mid_x),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "bentConnector4" => {
            let mid_x = frac(adj, "adj1", 50000.0) * w;
            let mid_y = frac(adj, "adj2", 50000.0) * h;
            format!(
                "<path d=\"M 0 0 L {mx} 0 L {mx} {my} L {w_n} {my} L {w_n} {h_n}\"/>",
                mx = n(mid_x),
                my = n(mid_y),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "bentConnector5" => {
            let mid_x1 = frac(adj, "adj1", 50000.0) * w;
            let mid_y = frac(adj, "adj2", 50000.0) * h;
            let mid_x2 = frac(adj, "adj3", 50000.0) * w;
            format!(
                "<path d=\"M 0 0 L {x1} 0 L {x1} {my} L {x2} {my} L {x2} {h_n} L {w_n} {h_n}\"/>",
                x1 = n(mid_x1),
                x2 = n(mid_x2),
                my = n(mid_y),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "curvedConnector2" => format!(
            "<path d=\"M 0 0 C {w_n} 0 0 {h_n} {w_n} {h_n}\"/>",
            w_n = n(w),
            h_n = n(h)
        ),
        "curvedConnector3" => {
            let mid_x = frac(adj, "adj1", 50000.0) * w;
            format!(
                "<path d=\"M 0 0 C {mx} 0 {mx} {h_n} {w_n} {h_n}\"/>",
                mx = n(mid_x),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "curvedConnector4" => {
            let mid_x = frac(adj, "adj1", 50000.0) * w;
            let mid_y = frac(adj, "adj2", 50000.0) * h;
            format!(
                "<path d=\"M 0 0 C {mx} 0 {mx} {my} {mx} {my} S {w_n} {my} {w_n} {h_n}\"/>",
                mx = n(mid_x),
                my = n(mid_y),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "curvedConnector5" => {
            let mid_x1 = frac(adj, "adj1", 50000.0) * w;
            let mid_y = frac(adj, "adj2", 50000.0) * h;
            let mid_x2 = frac(adj, "adj3", 50000.0) * w;
            format!(
                "<path d=\"M 0 0 C {x1} 0 {x1} {my} {x1} {my} S {x2} {my} {x2} {h_n} S {w_n} {h_n} {w_n} {h_n}\"/>",
                x1 = n(mid_x1),
                x2 = n(mid_x2),
                my = n(mid_y),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "cloud" => {
            let r = w.min(h) * 0.15;
            format!(
                "<rect width=\"{}\" height=\"{}\" rx=\"{}\"/>",
                n(w),
                n(h),
                n(r)
            )
        }
        "heart" => {
            let cx = w / 2.0;
            format!(
                "<path d=\"M {cx_n} {h35} C {cx_n} {h10}, 0 0, 0 {h35} C 0 {h65}, {cx_n} {h85}, {cx_n} {h_n} C {cx_n} {h85}, {w_n} {h65}, {w_n} {h35} C {w_n} 0, {cx_n} {h10}, {cx_n} {h35} Z\"/>",
                cx_n = n(cx),
                h35 = n(h * 0.35),
                h10 = n(h * 0.1),
                h65 = n(h * 0.65),
                h85 = n(h * 0.85),
                w_n = n(w),
                h_n = n(h)
            )
        }

        // --- Additional polygons ---
        "heptagon" => regular_polygon(w, h, 7),
        "octagon" => regular_polygon(w, h, 8),
        "decagon" => regular_polygon(w, h, 10),
        "dodecagon" => regular_polygon(w, h, 12),

        // --- Stars ---
        "star6" => star_polygon(w, h, 6, 0.5),
        "star7" => star_polygon(w, h, 7, 0.38),
        "star8" => star_polygon(w, h, 8, 0.38),
        "star10" => star_polygon(w, h, 10, 0.38),
        "star12" => star_polygon(w, h, 12, 0.38),
        "star16" => star_polygon(w, h, 16, 0.38),
        "star24" => star_polygon(w, h, 24, 0.38),
        "star32" => star_polygon(w, h, 32, 0.38),

        "irregularSeal1" => format!(
            "<polygon points=\"{},{} {},{} {},{} {},0 {},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{} 0,{}\"/>",
            n(w * 0.15), n(h * 0.35),
            n(w * 0.27), n(h * 0.03),
            n(w * 0.38), n(h * 0.28),
            n(w * 0.5),
            n(w * 0.6), n(h * 0.23),
            n(w * 0.73), n(h * 0.08),
            n(w * 0.72), n(h * 0.35),
            n(w), n(h * 0.35),
            n(w * 0.78), n(h * 0.5),
            n(w * 0.95), n(h * 0.7),
            n(w * 0.73), n(h * 0.65),
            n(w * 0.65), n(h),
            n(w * 0.5), n(h * 0.72),
            n(w * 0.35), n(h * 0.95),
            n(w * 0.32), n(h * 0.65),
            n(w * 0.05), n(h * 0.7),
            n(w * 0.18), n(h * 0.5),
            n(h * 0.35),
        ),
        "irregularSeal2" => format!(
            "<polygon points=\"{},{} {},{} {},{} {},0 {},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{} 0,{}\"/>",
            n(w * 0.1), n(h * 0.4),
            n(w * 0.18), n(h * 0.08),
            n(w * 0.32), n(h * 0.3),
            n(w * 0.45),
            n(w * 0.55), n(h * 0.18),
            n(w * 0.72), n(h * 0.05),
            n(w * 0.68), n(h * 0.32),
            n(w), n(h * 0.3),
            n(w * 0.82), n(h * 0.5),
            n(w * 0.98), n(h * 0.68),
            n(w * 0.75), n(h * 0.65),
            n(w * 0.8), n(h * 0.92),
            n(w * 0.55), n(h * 0.75),
            n(w * 0.42), n(h),
            n(w * 0.38), n(h * 0.72),
            n(w * 0.12), n(h * 0.88),
            n(w * 0.22), n(h * 0.6),
            n(h * 0.55),
        ),

        // --- Additional arrows ---
        "leftRightArrow" => {
            // See `rightArrow` comment — head_l anchors to min(w, h) so a
            // very wide arrow keeps a visible shaft instead of collapsing
            // into two head triangles meeting at the centre (the "diamond"
            // we used to render).
            let head_w = frac(adj, "adj1", 50000.0) * h;
            let head_l = (frac(adj, "adj2", 50000.0) * 2.0 * w.min(h)).min(w / 2.0);
            let body_top = (h - head_w) / 2.0;
            let body_bot = h - body_top;
            format!(
                "<polygon points=\"{hl},{bt} {hl},0 0,{hh} {hl},{h_n} {hl},{bb} {whl},{bb} {whl},{h_n} {w_n},{hh} {whl},0 {whl},{bt}\"/>",
                hl = n(head_l),
                bt = n(body_top),
                bb = n(body_bot),
                hh = n(h / 2.0),
                whl = n(w - head_l),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "upDownArrow" => {
            // Same min(w, h) anchoring along the long axis (here height).
            let head_w = frac(adj, "adj1", 50000.0) * w;
            let head_l = (frac(adj, "adj2", 50000.0) * 2.0 * w.min(h)).min(h / 2.0);
            let body_l = (w - head_w) / 2.0;
            let body_r = w - body_l;
            format!(
                "<polygon points=\"{bl},{hl} 0,{hl} {hw},0 {w_n},{hl} {br},{hl} {br},{hhl} {w_n},{hhl} {hw},{h_n} 0,{hhl} {bl},{hhl}\"/>",
                bl = n(body_l),
                br = n(body_r),
                hl = n(head_l),
                hhl = n(h - head_l),
                hw = n(w / 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "notchedRightArrow" => {
            let head_w = frac(adj, "adj1", 50000.0) * h;
            let head_l = frac(adj, "adj2", 50000.0) * w;
            let body_top = (h - head_w) / 2.0;
            let body_bottom = h - body_top;
            let shaft_end = w - head_l;
            let notch = head_l * 0.5;
            format!(
                "<polygon points=\"0,{bt} {se},{bt} {se},0 {w_n},{hh} {se},{h_n} {se},{bb} 0,{bb} {ntc},{hh}\"/>",
                bt = n(body_top),
                bb = n(body_bottom),
                se = n(shaft_end),
                ntc = n(notch),
                hh = n(h / 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "stripedRightArrow" => {
            let head_w = frac(adj, "adj1", 50000.0) * h;
            let head_l = frac(adj, "adj2", 50000.0) * w;
            let body_top = (h - head_w) / 2.0;
            let body_bottom = h - body_top;
            let shaft_end = w - head_l;
            let sw = w * 0.05;
            format!(
                "<path d=\"M 0 {bt} L {sw_n} {bt} L {sw_n} {bb} L 0 {bb} Z M {sw15} {bt} L {sw25} {bt} L {sw25} {bb} L {sw15} {bb} Z M {sw3} {bt} L {se} {bt} L {se} 0 L {w_n} {hh} L {se} {h_n} L {se} {bb} L {sw3} {bb} Z\"/>",
                bt = n(body_top),
                bb = n(body_bottom),
                se = n(shaft_end),
                sw_n = n(sw),
                sw15 = n(sw * 1.5),
                sw25 = n(sw * 2.5),
                sw3 = n(sw * 3.0),
                hh = n(h / 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "chevron" => {
            let off = frac(adj, "adj", 50000.0) * w;
            format!(
                "<polygon points=\"0,0 {wf},0 {w_n},{hh} {wf},{h_n} 0,{h_n} {of},{hh}\"/>",
                wf = n(w - off),
                of = n(off),
                hh = n(h / 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "homePlate" => {
            let off = frac(adj, "adj", 50000.0) * w.min(h);
            format!(
                "<polygon points=\"0,0 {wf},0 {w_n},{hh} {wf},{h_n} 0,{h_n}\"/>",
                wf = n(w - off),
                hh = n(h / 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "leftRightUpArrow" => {
            let head_w = frac(adj, "adj1", 25000.0) * w.min(h);
            let head_l = frac(adj, "adj2", 25000.0) * w.min(h);
            let body_w = frac(adj, "adj3", 25000.0) * w.min(h);
            let cx = w / 2.0;
            let body_half = body_w / 2.0;
            let body_mid = h - head_l - body_w;
            format!(
                "<path d=\"M {cx_n} 0 L {a1} {hl} L {a2} {hl} L {a2} {bm} L {whl} {bm} L {whl} {a3} L {w_n} {a4} L {whl} {a5} L {whl} {bmw} L {a2} {bmw} L {a2} {bmw} L {a6} {bmw} L {a6} {bmw} L {hl} {bmw} L {hl} {a5} L 0 {a4} L {hl} {a3} L {hl} {bm} L {a6} {bm} L {a6} {hl} L {a7} {hl} Z\"/>",
                cx_n = n(cx),
                a1 = n(cx + head_w / 2.0),
                a2 = n(cx + body_half),
                a3 = n(h / 2.0 + body_mid / 2.0 - head_w / 2.0),
                a4 = n(h / 2.0 + body_mid / 2.0),
                a5 = n(h / 2.0 + body_mid / 2.0 + head_w / 2.0),
                a6 = n(cx - body_half),
                a7 = n(cx - head_w / 2.0),
                hl = n(head_l),
                bm = n(body_mid),
                bmw = n(body_mid + body_w),
                whl = n(w - head_l),
                w_n = n(w),
            )
        }
        "quadArrow" => {
            let head_w = frac(adj, "adj1", 22500.0) * w.min(h);
            let head_l = frac(adj, "adj2", 22500.0) * w.min(h);
            let body_w = frac(adj, "adj3", 11250.0) * w.min(h);
            let cx = w / 2.0;
            let cy = h / 2.0;
            let bh = body_w / 2.0;
            format!(
                "<path d=\"M {cx_n} 0 L {a1} {hl} L {a2} {hl} L {a2} {a3} L {whl} {a3} L {whl} {a4} L {w_n} {cy_n} L {whl} {a5} L {whl} {a6} L {a2} {a6} L {a2} {hhl} L {a1} {hhl} L {cx_n} {h_n} L {a7} {hhl} L {a8} {hhl} L {a8} {a6} L {hl} {a6} L {hl} {a5} L 0 {cy_n} L {hl} {a4} L {hl} {a3} L {a8} {a3} L {a8} {hl} L {a7} {hl} Z\"/>",
                cx_n = n(cx),
                cy_n = n(cy),
                a1 = n(cx + head_w / 2.0),
                a2 = n(cx + bh),
                a3 = n(cy - bh),
                a4 = n(cy - head_w / 2.0),
                a5 = n(cy + head_w / 2.0),
                a6 = n(cy + bh),
                a7 = n(cx - head_w / 2.0),
                a8 = n(cx - bh),
                hl = n(head_l),
                hhl = n(h - head_l),
                whl = n(w - head_l),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "bentArrow" => {
            let head_w = frac(adj, "adj1", 25000.0) * h;
            let head_l = frac(adj, "adj2", 25000.0) * w;
            let body_w = frac(adj, "adj3", 25000.0) * h;
            let shaft_end = w - head_l;
            let body_top = head_w / 2.0 - body_w / 2.0;
            let body_bot = head_w / 2.0 + body_w / 2.0;
            format!(
                "<polygon points=\"{se},{bt} {se},0 {w_n},{hw} {se},{hw_full} {se},{bb} {bw},{bb} {bw},{h_n} 0,{h_n} 0,{hbw} 0,{hbw}\"/>",
                se = n(shaft_end),
                bt = n(body_top),
                bb = n(body_bot),
                bw = n(body_w),
                hw = n(head_w / 2.0),
                hw_full = n(head_w),
                hbw = n(h - body_w),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "bendUpArrow" => {
            let head_w = frac(adj, "adj1", 25000.0) * w;
            let head_l = frac(adj, "adj2", 25000.0) * h;
            let body_w = frac(adj, "adj3", 25000.0) * w;
            let cx = w - head_w / 2.0;
            let body_left = cx - body_w / 2.0;
            let body_right = cx + body_w / 2.0;
            format!(
                "<polygon points=\"{cl},{hl} {cx_n},0 {cr},{hl} {br},{hl} {br},{hbw} {bw},{hbw} {bw},{h_n} 0,{h_n} 0,{hbw} {bl},{hbw} {bl},{hl}\"/>",
                cl = n(cx - head_w / 2.0),
                cr = n(cx + head_w / 2.0),
                cx_n = n(cx),
                br = n(body_right),
                bl = n(body_left),
                bw = n(body_w),
                hl = n(head_l),
                hbw = n(h - body_w),
                h_n = n(h)
            )
        }
        "leftUpArrow" => {
            let head_w = frac(adj, "adj1", 25000.0) * w.min(h);
            let head_l = frac(adj, "adj2", 25000.0) * w.min(h);
            let body_w = frac(adj, "adj3", 25000.0) * w.min(h);
            let bh = body_w / 2.0;
            let top_cx = w - head_w / 2.0;
            let left_cy = h - head_w / 2.0;
            format!(
                "<path d=\"M {tc} 0 L {a1} {hl} L {a2} {hl} L {a2} {a3} L {hl} {a3} L {hl} {a4} L 0 {lc} L {hl} {a5} L {hl} {a6} L {a7} {a6} L {a7} {hl} L {a8} {hl} Z\"/>",
                tc = n(top_cx),
                a1 = n(top_cx + head_w / 2.0),
                a2 = n(top_cx + bh),
                a3 = n(left_cy - bh),
                a4 = n(left_cy - head_w / 2.0),
                a5 = n(left_cy + head_w / 2.0),
                a6 = n(left_cy + bh),
                a7 = n(top_cx - bh),
                a8 = n(top_cx - head_w / 2.0),
                hl = n(head_l),
                lc = n(left_cy),
            )
        }
        "uturnArrow" => {
            let head_w = frac(adj, "adj1", 25000.0) * w;
            let head_l = frac(adj, "adj2", 25000.0) * h;
            let body_w = frac(adj, "adj3", 25000.0) * w;
            let arc_r = w * 0.35;
            let cx = w / 2.0;
            let arrow_bot = h;
            let arrow_start = h - head_l;
            let body_right = w - (head_w / 2.0 - body_w / 2.0);
            let body_left = w - (head_w / 2.0 + body_w / 2.0);
            format!(
                "<path d=\"M {a1} {asn} L {a2} {ab} L {w_n} {asn} L {br} {asn} L {br} {ar} A {ar} {ar} 0 0 0 {bw} {ar} L {bw} {asn} L 0 {asn} L 0 {ar} A {cx_n} {cx_n} 0 0 1 {bl} {ar} L {bl} {asn} Z\"/>",
                a1 = n(w - head_w),
                a2 = n(w - head_w / 2.0),
                ab = n(arrow_bot),
                asn = n(arrow_start),
                ar = n(arc_r),
                br = n(body_right),
                bl = n(body_left),
                bw = n(body_w),
                cx_n = n(cx),
                w_n = n(w),
            )
        }

        // --- Flowchart shapes ---
        "flowChartProcess" => fallback_rect(w, h),
        "flowChartAlternateProcess" => {
            let r = w.min(h) / 6.0;
            format!(
                "<rect width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\"/>",
                n(w),
                n(h),
                n(r),
                n(r)
            )
        }
        "flowChartDecision" => format!(
            "<polygon points=\"{},0 {},{} {},{} 0,{}\"/>",
            n(w / 2.0),
            n(w),
            n(h / 2.0),
            n(w / 2.0),
            n(h),
            n(h / 2.0)
        ),
        "flowChartInputOutput" => {
            let off = w / 5.0;
            format!(
                "<polygon points=\"{},0 {},0 {},{} 0,{}\"/>",
                n(off),
                n(w),
                n(w - off),
                n(h),
                n(h)
            )
        }
        "flowChartPredefinedProcess" => {
            let d = w / 8.0;
            format!(
                "<path d=\"M 0 0 L {w_n} 0 L {w_n} {h_n} L 0 {h_n} Z M {d_n} 0 L {d_n} {h_n} M {wd} 0 L {wd} {h_n}\"/>",
                w_n = n(w),
                h_n = n(h),
                d_n = n(d),
                wd = n(w - d)
            )
        }
        "flowChartInternalStorage" => {
            let dx = w / 8.0;
            let dy = h / 8.0;
            format!(
                "<path d=\"M 0 0 L {w_n} 0 L {w_n} {h_n} L 0 {h_n} Z M {dx_n} 0 L {dx_n} {h_n} M 0 {dy_n} L {w_n} {dy_n}\"/>",
                w_n = n(w),
                h_n = n(h),
                dx_n = n(dx),
                dy_n = n(dy)
            )
        }
        "flowChartDocument" => {
            let bh = h * 0.83;
            format!(
                "<path d=\"M 0 0 L {w_n} 0 L {w_n} {bh_n} C {a1} {h_n}, {a2} {a3}, 0 {bh_n} Z\"/>",
                w_n = n(w),
                h_n = n(h),
                bh_n = n(bh),
                a1 = n(w * 0.75),
                a2 = n(w * 0.25),
                a3 = n(h * 0.66)
            )
        }
        "flowChartMultidocument" => {
            let dx = w * 0.1;
            let dy = h * 0.1;
            let bw = w - dx;
            let bh = (h - dy) * 0.83;
            format!(
                "<path d=\"M {dx_n} {dy_n} L {w_n} {dy_n} L {w_n} {a1} C {a2} {a3}, {a4} {a5}, {dx_n} {a1} Z M {a6} {a7} L {dx_n} {a7} L {dx_n} {dy_n} M 0 0 L {a6} 0 L {a6} {a7}\"/>",
                dx_n = n(dx),
                dy_n = n(dy),
                w_n = n(w),
                a1 = n(dy + bh),
                a2 = n(w - bw * 0.25),
                a3 = n(dy + (h - dy)),
                a4 = n(dx + bw * 0.25),
                a5 = n(dy + (h - dy) * 0.66),
                a6 = n(dx / 2.0),
                a7 = n(dy / 2.0)
            )
        }
        "flowChartTerminator" => {
            let r = h / 2.0;
            format!(
                "<rect width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\"/>",
                n(w),
                n(h),
                n(r),
                n(r)
            )
        }
        "flowChartPreparation" => {
            let off = w / 5.0;
            format!(
                "<polygon points=\"{a1},0 {a2},0 {w_n},{hh} {a2},{h_n} {a1},{h_n} 0,{hh}\"/>",
                a1 = n(off),
                a2 = n(w - off),
                hh = n(h / 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "flowChartManualInput" => {
            let top_y = h / 5.0;
            format!(
                "<polygon points=\"0,{ty} {w_n},0 {w_n},{h_n} 0,{h_n}\"/>",
                ty = n(top_y),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "flowChartManualOperation" => {
            let off = w / 5.0;
            format!(
                "<polygon points=\"0,0 {w_n},0 {a1},{h_n} {a2},{h_n}\"/>",
                w_n = n(w),
                h_n = n(h),
                a1 = n(w - off),
                a2 = n(off)
            )
        }
        "flowChartConnector" => format!(
            "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"/>",
            n(w / 2.0),
            n(h / 2.0),
            n(w / 2.0),
            n(h / 2.0)
        ),
        "flowChartOffpageConnector" => {
            let arrow_h = h * 0.2;
            format!(
                "<polygon points=\"0,0 {w_n},0 {w_n},{a1} {a2},{h_n} 0,{a1}\"/>",
                w_n = n(w),
                h_n = n(h),
                a1 = n(h - arrow_h),
                a2 = n(w / 2.0)
            )
        }
        "flowChartPunchedCard" => {
            let cut = w.min(h) * 0.2;
            format!(
                "<polygon points=\"{c},0 {w_n},0 {w_n},{h_n} 0,{h_n} 0,{c}\"/>",
                c = n(cut),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "flowChartPunchedTape" => {
            let wave = h * 0.1;
            format!(
                "<path d=\"M 0 {w_n} C {a1} {nw}, {a2} {a3}, {wd} {w_n} L {wd} {a4} C {a1} {h_n}, {a2} {a5}, 0 {a4} Z\"/>",
                w_n = n(wave),
                wd = n(w),
                nw = n(-wave),
                a1 = n(w * 0.25),
                a2 = n(w * 0.75),
                a3 = n(wave * 3.0),
                a4 = n(h - wave),
                a5 = n(h - wave * 3.0),
                h_n = n(h)
            )
        }
        "flowChartCollate" => format!(
            "<polygon points=\"0,0 {w_n},0 {hw},{hh} {w_n},{h_n} 0,{h_n} {hw},{hh}\"/>",
            w_n = n(w),
            h_n = n(h),
            hw = n(w / 2.0),
            hh = n(h / 2.0)
        ),
        "flowChartSort" => format!(
            "<path d=\"M {hw} 0 L {w_n} {hh} L 0 {hh} Z M 0 {hh} L {w_n} {hh} M {hw} {h_n} L {w_n} {hh} L 0 {hh} Z\"/>",
            hw = n(w / 2.0),
            hh = n(h / 2.0),
            w_n = n(w),
            h_n = n(h)
        ),
        "flowChartExtract" => format!(
            "<polygon points=\"{},0 {},{} 0,{}\"/>",
            n(w / 2.0),
            n(w),
            n(h),
            n(h)
        ),
        "flowChartMerge" => format!(
            "<polygon points=\"0,0 {},0 {},{}\"/>",
            n(w),
            n(w / 2.0),
            n(h)
        ),
        "flowChartOnlineStorage" => {
            let arc_w = w * 0.15;
            format!(
                "<path d=\"M {a} 0 L {w_n} 0 L {w_n} {h_n} L {a} {h_n} A {a} {hh} 0 0 1 {a} 0 Z\"/>",
                a = n(arc_w),
                hh = n(h / 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "flowChartDelay" => {
            let arc_w = w * 0.35;
            format!(
                "<path d=\"M 0 0 L {a1} 0 A {a} {hh} 0 0 1 {a1} {h_n} L 0 {h_n} Z\"/>",
                a = n(arc_w),
                a1 = n(w - arc_w),
                hh = n(h / 2.0),
                h_n = n(h)
            )
        }
        "flowChartDisplay" => {
            let left_w = w * 0.15;
            let arc_w = w * 0.35;
            format!(
                "<path d=\"M {lw} 0 L {wa} 0 A {aw} {hh} 0 0 1 {wa} {h_n} L {lw} {h_n} L 0 {hh} Z\"/>",
                lw = n(left_w),
                wa = n(w - arc_w),
                aw = n(arc_w),
                hh = n(h / 2.0),
                h_n = n(h)
            )
        }
        "flowChartMagneticTape" => {
            let r = w.min(h) / 2.0;
            let cx = w / 2.0;
            let cy = h / 2.0;
            format!(
                "<path d=\"M {a1} {cy_n} A {r_n} {r_n} 0 1 1 {a2} {a3} L {w_n} {cy_n} L {w_n} {h_n} L {a4} {h_n} L {a5} {a6}\"/>",
                a1 = n(cx + r),
                a2 = n(cx + r - 0.01),
                a3 = n(cy + 0.01),
                a4 = n(w - r * 0.3),
                a5 = n(cx + r * (PI / 6.0).cos()),
                a6 = n(cy + r * (PI / 6.0).sin()),
                cy_n = n(cy),
                r_n = n(r),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "flowChartMagneticDisk" => {
            let ry = h * 0.15;
            format!(
                "<path d=\"M 0 {ry_n} A {hw} {ry_n} 0 0 1 {w_n} {ry_n} L {w_n} {a1} A {hw} {ry_n} 0 0 1 0 {a1} Z M 0 {ry_n} A {hw} {ry_n} 0 0 0 {w_n} {ry_n}\"/>",
                ry_n = n(ry),
                hw = n(w / 2.0),
                a1 = n(h - ry),
                w_n = n(w)
            )
        }
        "flowChartMagneticDrum" => {
            let rx = w * 0.15;
            format!(
                "<path d=\"M {rx_n} 0 A {rx_n} {hh} 0 0 0 {rx_n} {h_n} L {wr} {h_n} A {rx_n} {hh} 0 0 0 {wr} 0 Z M {wr} 0 A {rx_n} {hh} 0 0 1 {wr} {h_n}\"/>",
                rx_n = n(rx),
                wr = n(w - rx),
                hh = n(h / 2.0),
                h_n = n(h)
            )
        }
        "flowChartSummingJunction" => {
            let cx = w / 2.0;
            let cy = h / 2.0;
            let rx = w / 2.0;
            let ry = h / 2.0;
            let d = 0.707_f64;
            format!(
                "<path d=\"M {a1} {cy_n} A {rx_n} {ry_n} 0 1 1 {a2} {a3} Z M {a4} {a5} L {a6} {a7} M {a6} {a5} L {a4} {a7}\"/>",
                a1 = n(cx + rx),
                a2 = n(cx + rx - 0.01),
                a3 = n(cy - 0.01),
                a4 = n(cx - rx * d),
                a5 = n(cy - ry * d),
                a6 = n(cx + rx * d),
                a7 = n(cy + ry * d),
                cy_n = n(cy),
                rx_n = n(rx),
                ry_n = n(ry)
            )
        }
        "flowChartOr" => {
            let cx = w / 2.0;
            let cy = h / 2.0;
            let rx = w / 2.0;
            let ry = h / 2.0;
            format!(
                "<path d=\"M {a1} {cy_n} A {rx_n} {ry_n} 0 1 1 {a2} {a3} Z M {cx_n} 0 L {cx_n} {h_n} M 0 {cy_n} L {w_n} {cy_n}\"/>",
                a1 = n(cx + rx),
                a2 = n(cx + rx - 0.01),
                a3 = n(cy - 0.01),
                cx_n = n(cx),
                cy_n = n(cy),
                rx_n = n(rx),
                ry_n = n(ry),
                w_n = n(w),
                h_n = n(h)
            )
        }

        // --- Callout shapes ---
        "wedgeRectCallout" => {
            let tip_x = w / 2.0 + frac(adj, "adj1", -20833.0) * w;
            let tip_y = h / 2.0 + frac(adj, "adj2", 62500.0) * h;
            let bx = w / 2.0;
            let wedge_w = w * 0.06;
            format!(
                "<path d=\"M 0 0 L {w_n} 0 L {w_n} {h_n} L {a1} {h_n} L {tx} {ty} L {a2} {h_n} L 0 {h_n} Z\"/>",
                w_n = n(w),
                h_n = n(h),
                a1 = n(bx + wedge_w),
                a2 = n(bx - wedge_w),
                tx = n(tip_x),
                ty = n(tip_y)
            )
        }
        "wedgeRoundRectCallout" => {
            let tip_x = w / 2.0 + frac(adj, "adj1", -20833.0) * w;
            let tip_y = h / 2.0 + frac(adj, "adj2", 62500.0) * h;
            let r = frac(adj, "adj3", 16667.0) * w.min(h);
            let bx = w / 2.0;
            let wedge_w = w * 0.06;
            format!(
                "<path d=\"M {r_n} 0 L {a1} 0 A {r_n} {r_n} 0 0 1 {w_n} {r_n} L {w_n} {a2} A {r_n} {r_n} 0 0 1 {a1} {h_n} L {a3} {h_n} L {tx} {ty} L {a4} {h_n} L {r_n} {h_n} A {r_n} {r_n} 0 0 1 0 {a2} L 0 {r_n} A {r_n} {r_n} 0 0 1 {r_n} 0 Z\"/>",
                r_n = n(r),
                a1 = n(w - r),
                a2 = n(h - r),
                a3 = n(bx + wedge_w),
                a4 = n(bx - wedge_w),
                tx = n(tip_x),
                ty = n(tip_y),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "wedgeEllipseCallout" => {
            let tip_x = w / 2.0 + frac(adj, "adj1", -20833.0) * w;
            let tip_y = h / 2.0 + frac(adj, "adj2", 62500.0) * h;
            let cx = w / 2.0;
            let cy = h / 2.0;
            let rx = w / 2.0;
            let ry = h / 2.0;
            let angle = (tip_y - cy).atan2(tip_x - cx);
            let wedge_angle = 0.15;
            let x1 = cx + rx * (angle - wedge_angle).cos();
            let y1 = cy + ry * (angle - wedge_angle).sin();
            let x2 = cx + rx * (angle + wedge_angle).cos();
            let y2 = cy + ry * (angle + wedge_angle).sin();
            format!(
                "<path d=\"M {x1_n} {y1_n} L {tx} {ty} L {x2_n} {y2_n} A {rx_n} {ry_n} 0 1 1 {x1_n} {y1_n} Z\"/>",
                x1_n = n(x1),
                y1_n = n(y1),
                x2_n = n(x2),
                y2_n = n(y2),
                tx = n(tip_x),
                ty = n(tip_y),
                rx_n = n(rx),
                ry_n = n(ry)
            )
        }
        "cloudCallout" => {
            let tip_x = w / 2.0 + frac(adj, "adj1", -20833.0) * w;
            let tip_y = h / 2.0 + frac(adj, "adj2", 62500.0) * h;
            let r = w.min(h) * 0.15;
            let bx = w / 2.0;
            let by = h / 2.0;
            let dx = tip_x - bx;
            let dy = tip_y - by;
            let d1x = bx + dx * 0.33;
            let d1y = by + dy * 0.33;
            let d2x = bx + dx * 0.66;
            let d2y = by + dy * 0.66;
            format!(
                "<path d=\"M {r_n} {h_n} A {r_n} {r_n} 0 0 1 0 {a1} A {r_n} {r_n} 0 0 1 {r_n} {a2} L {r_n} {r_n} A {r_n} {r_n} 0 0 1 {a3} 0 L {a4} 0 A {r_n} {r_n} 0 0 1 {a5} {r_n} L {a5} {a2} A {r_n} {r_n} 0 0 1 {a4} {a1} A {r_n} {r_n} 0 0 1 {a4} {h_n} Z M {d1x_n} {d1y_n} m {a6} 0 a {a6} {a6} 0 1 1 -{a7} 0 a {a6} {a6} 0 1 1 {a7} 0 Z M {d2x_n} {d2y_n} m {a8} 0 a {a8} {a8} 0 1 1 -{a9} 0 a {a8} {a8} 0 1 1 {a9} 0 Z\"/>",
                r_n = n(r),
                a1 = n(h - r),
                a2 = n(h - 2.0 * r),
                a3 = n(2.0 * r),
                a4 = n(w - 2.0 * r),
                a5 = n(w - r),
                a6 = n(r * 0.25),
                a7 = n(r * 0.5),
                a8 = n(r * 0.15),
                a9 = n(r * 0.3),
                d1x_n = n(d1x),
                d1y_n = n(d1y),
                d2x_n = n(d2x),
                d2y_n = n(d2y),
                h_n = n(h)
            )
        }
        "borderCallout1" => {
            let y1 = frac(adj, "adj1", 18750.0) * h;
            let x1 = frac(adj, "adj2", -8333.0) * w;
            let y2 = frac(adj, "adj3", 112500.0) * h;
            let x2 = frac(adj, "adj4", -38333.0) * w;
            format!(
                "<path d=\"M 0 0 L {w_n} 0 L {w_n} {h_n} L 0 {h_n} Z M {x1_n} {y1_n} L {x2_n} {y2_n}\"/>",
                w_n = n(w),
                h_n = n(h),
                x1_n = n(x1),
                y1_n = n(y1),
                x2_n = n(x2),
                y2_n = n(y2)
            )
        }
        "borderCallout2" => {
            let y1 = frac(adj, "adj1", 18750.0) * h;
            let x1 = frac(adj, "adj2", -8333.0) * w;
            let y2 = frac(adj, "adj3", 18750.0) * h;
            let x2 = frac(adj, "adj4", -16667.0) * w;
            let y3 = frac(adj, "adj5", 112500.0) * h;
            let x3 = frac(adj, "adj6", -46667.0) * w;
            format!(
                "<path d=\"M 0 0 L {w_n} 0 L {w_n} {h_n} L 0 {h_n} Z M {x1_n} {y1_n} L {x2_n} {y2_n} L {x3_n} {y3_n}\"/>",
                w_n = n(w),
                h_n = n(h),
                x1_n = n(x1),
                y1_n = n(y1),
                x2_n = n(x2),
                y2_n = n(y2),
                x3_n = n(x3),
                y3_n = n(y3)
            )
        }
        "borderCallout3" => {
            let y1 = frac(adj, "adj1", 18750.0) * h;
            let x1 = frac(adj, "adj2", -8333.0) * w;
            let y2 = frac(adj, "adj3", 18750.0) * h;
            let x2 = frac(adj, "adj4", -16667.0) * w;
            let y3 = frac(adj, "adj5", 100000.0) * h;
            let x3 = frac(adj, "adj6", -16667.0) * w;
            let y4 = frac(adj, "adj7", 112963.0) * h;
            let x4 = frac(adj, "adj8", -46667.0) * w;
            format!(
                "<path d=\"M 0 0 L {w_n} 0 L {w_n} {h_n} L 0 {h_n} Z M {x1_n} {y1_n} L {x2_n} {y2_n} L {x3_n} {y3_n} L {x4_n} {y4_n}\"/>",
                w_n = n(w),
                h_n = n(h),
                x1_n = n(x1),
                y1_n = n(y1),
                x2_n = n(x2),
                y2_n = n(y2),
                x3_n = n(x3),
                y3_n = n(y3),
                x4_n = n(x4),
                y4_n = n(y4)
            )
        }

        // --- Arc shapes ---
        "arc" => arc_path(w, h, adj, false, 16_200_000.0, 0.0),
        "chord" => arc_path(w, h, adj, true, 2_700_000.0, 16_200_000.0),
        "pie" => pie_path(w, h, adj),
        "blockArc" => block_arc(w, h, adj),

        // --- Math shapes ---
        "mathPlus" => {
            let t = frac(adj, "adj1", 23520.0);
            let tw = t * w;
            let th = t * h;
            let lx = (w - tw) / 2.0;
            let rx = lx + tw;
            let ty = (h - th) / 2.0;
            let by = ty + th;
            format!(
                "<polygon points=\"{lx_n},0 {rx_n},0 {rx_n},{ty_n} {w_n},{ty_n} {w_n},{by_n} {rx_n},{by_n} {rx_n},{h_n} {lx_n},{h_n} {lx_n},{by_n} 0,{by_n} 0,{ty_n} {lx_n},{ty_n}\"/>",
                lx_n = n(lx),
                rx_n = n(rx),
                ty_n = n(ty),
                by_n = n(by),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "mathMinus" => {
            let t = frac(adj, "adj1", 23520.0);
            let th = t * h;
            let ty = (h - th) / 2.0;
            let by = ty + th;
            format!(
                "<rect x=\"0\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                n(ty),
                n(w),
                n(by - ty)
            )
        }
        "mathMultiply" => {
            let t = frac(adj, "adj1", 23520.0);
            let d = t * w.min(h) * 0.5;
            let cx = w / 2.0;
            let cy = h / 2.0;
            format!(
                "<path d=\"M {cx_n} {a1} L {wd} 0 L {w_n} {d_n} L {a2} {cy_n} L {w_n} {a3} L {wd} {h_n} L {cx_n} {a4} L {d_n} {h_n} L 0 {a3} L {a5} {cy_n} L 0 {d_n} L {d_n} 0 Z\"/>",
                cx_n = n(cx),
                cy_n = n(cy),
                a1 = n(cy - d),
                a2 = n(cx + d),
                a3 = n(h - d),
                a4 = n(cy + d),
                a5 = n(cx - d),
                wd = n(w - d),
                d_n = n(d),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "mathDivide" => {
            let t = frac(adj, "adj1", 23520.0);
            let th = t * h;
            let ty = (h - th) / 2.0;
            let by = ty + th;
            let dot_r = w.min(h) * t * 0.5;
            let cx = w / 2.0;
            let top_dot_y = ty / 2.0;
            let bot_dot_y = h - ty / 2.0;
            format!(
                "<path d=\"M 0 {ty_n} L {w_n} {ty_n} L {w_n} {by_n} L 0 {by_n} Z M {a1} {tdy} A {dr} {dr} 0 1 1 {a2} {a3} Z M {a1} {bdy} A {dr} {dr} 0 1 1 {a2} {a4} Z\"/>",
                ty_n = n(ty),
                by_n = n(by),
                a1 = n(cx + dot_r),
                a2 = n(cx + dot_r - 0.01),
                a3 = n(top_dot_y - 0.01),
                a4 = n(bot_dot_y - 0.01),
                tdy = n(top_dot_y),
                bdy = n(bot_dot_y),
                dr = n(dot_r),
                w_n = n(w),
            )
        }
        "mathEqual" => {
            let t = frac(adj, "adj1", 23520.0);
            let gap = t * h;
            let bar_h = t * h;
            let y1 = h.midpoint(-gap) - bar_h;
            let y2 = h.midpoint(gap);
            format!(
                "<path d=\"M 0 {y1_n} L {w_n} {y1_n} L {w_n} {a1} L 0 {a1} Z M 0 {y2_n} L {w_n} {y2_n} L {w_n} {a2} L 0 {a2} Z\"/>",
                y1_n = n(y1),
                y2_n = n(y2),
                a1 = n(y1 + bar_h),
                a2 = n(y2 + bar_h),
                w_n = n(w)
            )
        }
        "mathNotEqual" => {
            let t = frac(adj, "adj1", 23520.0);
            let gap = t * h;
            let bar_h = t * h;
            let y1 = h.midpoint(-gap) - bar_h;
            let y2 = h.midpoint(gap);
            let slash_w = w * 0.15;
            let sx = w / 2.0 - slash_w / 2.0;
            format!(
                "<path d=\"M 0 {y1_n} L {w_n} {y1_n} L {w_n} {a1} L 0 {a1} Z M 0 {y2_n} L {w_n} {y2_n} L {w_n} {a2} L 0 {a2} Z M {a3} {a4} L {a5} {a4} L {sx_n} {a6} L {a7} {a6} Z\"/>",
                y1_n = n(y1),
                y2_n = n(y2),
                a1 = n(y1 + bar_h),
                a2 = n(y2 + bar_h),
                a3 = n(sx + slash_w),
                a4 = n(y1 - bar_h),
                a5 = n(sx + slash_w + slash_w),
                a6 = n(y2 + bar_h + bar_h),
                a7 = n(sx - slash_w),
                sx_n = n(sx),
                w_n = n(w),
            )
        }

        // --- Other shapes ---
        "plus" => {
            let t = frac(adj, "adj", 25000.0);
            let lx = t * w;
            let rx = w - lx;
            let ty = t * h;
            let by = h - ty;
            format!(
                "<polygon points=\"{lx_n},0 {rx_n},0 {rx_n},{ty_n} {w_n},{ty_n} {w_n},{by_n} {rx_n},{by_n} {rx_n},{h_n} {lx_n},{h_n} {lx_n},{by_n} 0,{by_n} 0,{ty_n} {lx_n},{ty_n}\"/>",
                lx_n = n(lx),
                rx_n = n(rx),
                ty_n = n(ty),
                by_n = n(by),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "corner" => {
            let adj_x = frac(adj, "adj1", 50000.0);
            let adj_y = frac(adj, "adj2", 50000.0);
            let cx = adj_x * w;
            let cy = adj_y * h;
            format!(
                "<polygon points=\"0,0 {cx_n},0 {cx_n},{cy_n} {w_n},{cy_n} {w_n},{h_n} 0,{h_n}\"/>",
                cx_n = n(cx),
                cy_n = n(cy),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "diagStripe" => {
            let d = frac(adj, "adj", 50000.0) * w.min(h);
            format!(
                "<polygon points=\"0,{d_n} {d_n},0 {w_n},0 0,{h_n}\"/>",
                d_n = n(d),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "foldedCorner" => {
            let fold = frac(adj, "adj", 16667.0) * w.min(h);
            format!(
                "<path d=\"M 0 0 L {w_n} 0 L {w_n} {a1} L {a2} {h_n} L 0 {h_n} Z M {a2} {h_n} L {a2} {a1} L {w_n} {a1}\"/>",
                a1 = n(h - fold),
                a2 = n(w - fold),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "plaque" => {
            let r = frac(adj, "adj", 16667.0) * w.min(h);
            format!(
                "<path d=\"M 0 {r_n} A {r_n} {r_n} 0 0 1 {r_n} 0 L {a1} 0 A {r_n} {r_n} 0 0 1 {w_n} {r_n} L {w_n} {a2} A {r_n} {r_n} 0 0 1 {a1} {h_n} L {r_n} {h_n} A {r_n} {r_n} 0 0 1 0 {a2} Z\"/>",
                r_n = n(r),
                a1 = n(w - r),
                a2 = n(h - r),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "can" => {
            let ry = frac(adj, "adj", 25000.0) * h * 0.5;
            format!(
                "<path d=\"M 0 {ry_n} A {hw} {ry_n} 0 0 1 {w_n} {ry_n} L {w_n} {a1} A {hw} {ry_n} 0 0 1 0 {a1} Z M 0 {ry_n} A {hw} {ry_n} 0 0 0 {w_n} {ry_n}\"/>",
                ry_n = n(ry),
                hw = n(w / 2.0),
                a1 = n(h - ry),
                w_n = n(w)
            )
        }
        "cube" => {
            let d = frac(adj, "adj", 25000.0) * w.min(h);
            format!(
                "<path d=\"M 0 {d_n} L {d_n} 0 L {w_n} 0 L {w_n} {a1} L {a2} {h_n} L 0 {h_n} Z M 0 {d_n} L {a2} {d_n} L {w_n} 0 M {a2} {d_n} L {a2} {h_n}\"/>",
                a1 = n(h - d),
                a2 = n(w - d),
                d_n = n(d),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "donut" => {
            let t = frac(adj, "adj", 25000.0);
            let rx = w / 2.0;
            let ry = h / 2.0;
            let irx = rx * (1.0 - t);
            let iry = ry * (1.0 - t);
            let cx = rx;
            let cy = ry;
            format!(
                "<path fill-rule=\"evenodd\" d=\"M {a1} {cy_n} A {rx_n} {ry_n} 0 1 1 {a2} {cy_n} A {rx_n} {ry_n} 0 1 1 {a1} {cy_n} Z M {a3} {cy_n} A {irx_n} {iry_n} 0 1 0 {a4} {cy_n} A {irx_n} {iry_n} 0 1 0 {a3} {cy_n} Z\"/>",
                a1 = n(cx + rx),
                a2 = n(cx - rx),
                a3 = n(cx + irx),
                a4 = n(cx - irx),
                cy_n = n(cy),
                rx_n = n(rx),
                ry_n = n(ry),
                irx_n = n(irx),
                iry_n = n(iry)
            )
        }
        "noSmoking" => {
            let t = frac(adj, "adj", 18750.0);
            let rx = w / 2.0;
            let ry = h / 2.0;
            let cx = rx;
            let cy = ry;
            let irx = rx * (1.0 - t);
            let iry = ry * (1.0 - t);
            let angle = PI / 4.0;
            let lx1 = cx + irx * angle.cos();
            let ly1 = cy - iry * angle.sin();
            let lx2 = cx - irx * angle.cos();
            let ly2 = cy + iry * angle.sin();
            format!(
                "<path fill-rule=\"evenodd\" d=\"M {a1} {cy_n} A {rx_n} {ry_n} 0 1 1 {a2} {cy_n} A {rx_n} {ry_n} 0 1 1 {a1} {cy_n} Z M {a3} {cy_n} A {irx_n} {iry_n} 0 1 0 {a4} {cy_n} A {irx_n} {iry_n} 0 1 0 {a3} {cy_n} Z M {lx1_n} {ly1_n} L {lx2_n} {ly2_n}\"/>",
                a1 = n(cx + rx),
                a2 = n(cx - rx),
                a3 = n(cx + irx),
                a4 = n(cx - irx),
                cy_n = n(cy),
                rx_n = n(rx),
                ry_n = n(ry),
                irx_n = n(irx),
                iry_n = n(iry),
                lx1_n = n(lx1),
                ly1_n = n(ly1),
                lx2_n = n(lx2),
                ly2_n = n(ly2)
            )
        }
        "smileyFace" => {
            let smile = frac(adj, "adj", 4653.0);
            let rx = w / 2.0;
            let ry = h / 2.0;
            let cx = rx;
            let cy = ry;
            let eye_rx = w * 0.06;
            let eye_ry = h * 0.06;
            let eye_y = h * 0.35;
            let left_eye_x = w * 0.35;
            let right_eye_x = w * 0.65;
            let mouth_y = h * 0.6;
            let mouth_w = w * 0.3;
            let mouth_curve = smile * h;
            format!(
                "<path d=\"M {a1} {cy_n} A {rx_n} {ry_n} 0 1 1 {a2} {cy_n} A {rx_n} {ry_n} 0 1 1 {a1} {cy_n} Z M {a3} {ey} A {erx} {ery} 0 1 1 {a4} {ey} A {erx} {ery} 0 1 1 {a3} {ey} Z M {a5} {ey} A {erx} {ery} 0 1 1 {a6} {ey} A {erx} {ery} 0 1 1 {a5} {ey} Z M {a7} {my} C {a8} {a9}, {a10} {a9}, {a11} {my}\"/>",
                a1 = n(cx + rx),
                a2 = n(cx - rx),
                a3 = n(left_eye_x + eye_rx),
                a4 = n(left_eye_x - eye_rx),
                a5 = n(right_eye_x + eye_rx),
                a6 = n(right_eye_x - eye_rx),
                a7 = n(cx - mouth_w),
                a8 = n(cx - mouth_w * 0.5),
                a9 = n(mouth_y + mouth_curve),
                a10 = n(cx + mouth_w * 0.5),
                a11 = n(cx + mouth_w),
                cy_n = n(cy),
                rx_n = n(rx),
                ry_n = n(ry),
                ey = n(eye_y),
                erx = n(eye_rx),
                ery = n(eye_ry),
                my = n(mouth_y)
            )
        }
        "frame" => {
            let t = frac(adj, "adj1", 12500.0) * w.min(h);
            format!(
                "<path fill-rule=\"evenodd\" d=\"M 0 0 L {w_n} 0 L {w_n} {h_n} L 0 {h_n} Z M {t_n} {t_n} L {t_n} {a1} L {a2} {a1} L {a2} {t_n} Z\"/>",
                t_n = n(t),
                a1 = n(h - t),
                a2 = n(w - t),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "bevel" => {
            let t = frac(adj, "adj", 12500.0) * w.min(h);
            format!(
                "<path d=\"M 0 0 L {w_n} 0 L {w_n} {h_n} L 0 {h_n} Z M {t_n} {t_n} L {a1} {t_n} L {a1} {a2} L {t_n} {a2} Z M 0 0 L {t_n} {t_n} M {w_n} 0 L {a1} {t_n} M {w_n} {h_n} L {a1} {a2} M 0 {h_n} L {t_n} {a2}\"/>",
                t_n = n(t),
                a1 = n(w - t),
                a2 = n(h - t),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "halfFrame" => {
            let adj_x = frac(adj, "adj1", 33333.0) * w;
            let adj_y = frac(adj, "adj2", 33333.0) * h;
            format!(
                "<polygon points=\"0,0 {w_n},0 {w_n},{ay} {ax},{ay} {ax},{h_n} 0,{h_n}\"/>",
                ax = n(adj_x),
                ay = n(adj_y),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "snip1Rect" => {
            let d = frac(adj, "adj", 16667.0) * w.min(h);
            format!(
                "<polygon points=\"0,0 {a1},0 {w_n},{d_n} {w_n},{h_n} 0,{h_n}\"/>",
                a1 = n(w - d),
                d_n = n(d),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "snip2SameRect" => {
            let d = frac(adj, "adj1", 16667.0) * w.min(h);
            let d2 = frac(adj, "adj2", 0.0) * w.min(h);
            format!(
                "<polygon points=\"{d_n},0 {a1},0 {w_n},{d_n} {w_n},{a2} {a3},{h_n} {d2_n},{h_n} 0,{a2} 0,{d_n}\"/>",
                d_n = n(d),
                d2_n = n(d2),
                a1 = n(w - d),
                a2 = n(h - d2),
                a3 = n(w - d2),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "snip2DiagRect" => {
            let d1 = frac(adj, "adj1", 16667.0) * w.min(h);
            let d2 = frac(adj, "adj2", 0.0) * w.min(h);
            format!(
                "<polygon points=\"{d1_n},0 {w_n},0 {w_n},{a1} {a2},{h_n} 0,{h_n} 0,{d1_n}\"/>",
                d1_n = n(d1),
                a1 = n(h - d2),
                a2 = n(w - d2),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "snipRoundRect" => {
            let r = frac(adj, "adj1", 16667.0) * w.min(h);
            let d = frac(adj, "adj2", 16667.0) * w.min(h);
            format!(
                "<path d=\"M {r_n} 0 L {a1} 0 L {w_n} {d_n} L {w_n} {h_n} L 0 {h_n} L 0 {r_n} A {r_n} {r_n} 0 0 1 {r_n} 0 Z\"/>",
                r_n = n(r),
                d_n = n(d),
                a1 = n(w - d),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "round1Rect" => {
            let raw = a(adj, "adj", 16667.0);
            let aclamped = raw.clamp(0.0, 50000.0);
            let r = (aclamped / 100_000.0) * w.min(h);
            format!(
                "<path d=\"M 0 0 L {a1} 0 A {r_n} {r_n} 0 0 1 {w_n} {r_n} L {w_n} {h_n} L 0 {h_n} Z\"/>",
                r_n = n(r),
                a1 = n(w - r),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "round2SameRect" => {
            let a1 = a(adj, "adj1", 16667.0).clamp(0.0, 50000.0);
            let a2 = a(adj, "adj2", 0.0).clamp(0.0, 50000.0);
            let r1 = (a1 / 100_000.0) * w.min(h);
            let r2 = (a2 / 100_000.0) * w.min(h);
            format!(
                "<path d=\"M {r1_n} 0 L {a3} 0 A {r1_n} {r1_n} 0 0 1 {w_n} {r1_n} L {w_n} {a4} A {r2_n} {r2_n} 0 0 1 {a5} {h_n} L {r2_n} {h_n} A {r2_n} {r2_n} 0 0 1 0 {a4} L 0 {r1_n} A {r1_n} {r1_n} 0 0 1 {r1_n} 0 Z\"/>",
                r1_n = n(r1),
                r2_n = n(r2),
                a3 = n(w - r1),
                a4 = n(h - r2),
                a5 = n(w - r2),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "round2DiagRect" => {
            let a1 = a(adj, "adj1", 16667.0).clamp(0.0, 50000.0);
            let a2 = a(adj, "adj2", 0.0).clamp(0.0, 50000.0);
            let r1 = (a1 / 100_000.0) * w.min(h);
            let r2 = (a2 / 100_000.0) * w.min(h);
            format!(
                "<path d=\"M {r1_n} 0 L {w_n} 0 L {w_n} {a3} A {r2_n} {r2_n} 0 0 1 {a4} {h_n} L 0 {h_n} L 0 {r1_n} A {r1_n} {r1_n} 0 0 1 {r1_n} 0 Z\"/>",
                r1_n = n(r1),
                r2_n = n(r2),
                a3 = n(h - r2),
                a4 = n(w - r2),
                w_n = n(w),
                h_n = n(h)
            )
        }

        // --- Brackets and braces ---
        "leftBracket" => {
            let r = frac(adj, "adj", 8333.0) * h;
            format!(
                "<path d=\"M {w_n} 0 L {r_n} 0 A {r_n} {r_n} 0 0 0 0 {r_n} L 0 {a1} A {r_n} {r_n} 0 0 0 {r_n} {h_n} L {w_n} {h_n}\"/>",
                r_n = n(r),
                a1 = n(h - r),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "rightBracket" => {
            let r = frac(adj, "adj", 8333.0) * h;
            format!(
                "<path d=\"M 0 0 L {a1} 0 A {r_n} {r_n} 0 0 1 {w_n} {r_n} L {w_n} {a2} A {r_n} {r_n} 0 0 1 {a1} {h_n} L 0 {h_n}\"/>",
                r_n = n(r),
                a1 = n(w - r),
                a2 = n(h - r),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "leftBrace" => {
            // OOXML preset spec: corner radius is `min(w, h) * pin(0,
            // adj1, 25000) / 100000`. Anchoring to `min(w, h)` (not `h`)
            // and clamping adj1 to ≤25000 keeps the radius from blowing
            // past the shape's smaller dimension; without that, slide
            // 46's down-pointing dashed brace (cx=16, cy=678 with
            // adj1=101653 → radius ≈ 690 in our buggy code) collapsed
            // into a near-straight horizontal line spanning the slide.
            let ss = w.min(h);
            let a1 = frac(adj, "adj1", 8333.0).clamp(0.0, 0.25);
            let r = a1 * ss;
            let mid = frac(adj, "adj2", 50000.0) * h;
            format!(
                "<path d=\"M {w_n} 0 A {hw} {r_n} 0 0 0 {hw} {r_n} L {hw} {a1} A {hw} {r_n} 0 0 1 0 {mid_n} A {hw} {r_n} 0 0 1 {hw} {a2} L {hw} {a3} A {hw} {r_n} 0 0 0 {w_n} {h_n}\"/>",
                r_n = n(r),
                mid_n = n(mid),
                a1 = n(mid - r),
                a2 = n(mid + r),
                a3 = n(h - r),
                hw = n(w / 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "rightBrace" => {
            // Same fix as leftBrace — radius scales by min(w, h) and
            // adj1 is clamped to the OOXML-pin 0..25000 range.
            let ss = w.min(h);
            let a1 = frac(adj, "adj1", 8333.0).clamp(0.0, 0.25);
            let r = a1 * ss;
            let mid = frac(adj, "adj2", 50000.0) * h;
            format!(
                "<path d=\"M 0 0 A {hw} {r_n} 0 0 1 {hw} {r_n} L {hw} {a1} A {hw} {r_n} 0 0 0 {w_n} {mid_n} A {hw} {r_n} 0 0 0 {hw} {a2} L {hw} {a3} A {hw} {r_n} 0 0 1 0 {h_n}\"/>",
                r_n = n(r),
                mid_n = n(mid),
                a1 = n(mid - r),
                a2 = n(mid + r),
                a3 = n(h - r),
                hw = n(w / 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "bracketPair" => {
            let r = frac(adj, "adj", 16667.0) * w.min(h);
            format!(
                "<path d=\"M {r_n} 0 A {r_n} {r_n} 0 0 0 0 {r_n} L 0 {a1} A {r_n} {r_n} 0 0 0 {r_n} {h_n} M {a2} 0 A {r_n} {r_n} 0 0 1 {w_n} {r_n} L {w_n} {a1} A {r_n} {r_n} 0 0 1 {a2} {h_n}\"/>",
                r_n = n(r),
                a1 = n(h - r),
                a2 = n(w - r),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "bracePair" => {
            let r = frac(adj, "adj", 8333.0) * w.min(h);
            format!(
                "<path d=\"M {r_n} 0 A {r_n} {r_n} 0 0 0 0 {r_n} L 0 {a1} A {r_n} {r_n} 0 0 1 {a2} {hh} A {r_n} {r_n} 0 0 1 0 {a3} L 0 {a4} A {r_n} {r_n} 0 0 0 {r_n} {h_n} M {a5} 0 A {r_n} {r_n} 0 0 1 {w_n} {r_n} L {w_n} {a1} A {r_n} {r_n} 0 0 0 {a6} {hh} A {r_n} {r_n} 0 0 0 {w_n} {a3} L {w_n} {a4} A {r_n} {r_n} 0 0 1 {a5} {h_n}\"/>",
                r_n = n(r),
                a1 = n(h / 2.0 - r),
                a2 = n(-r),
                a3 = n(h / 2.0 + r),
                a4 = n(h - r),
                a5 = n(w - r),
                a6 = n(w + r),
                hh = n(h / 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }

        // --- Other misc shapes ---
        "lightningBolt" => format!(
            "<polygon points=\"{},0 {},{} {},{} {},{} {},{} {},{} {},0\"/>",
            n(w * 0.55),
            n(w * 0.3),
            n(h * 0.4),
            n(w * 0.52),
            n(h * 0.4),
            n(w * 0.25),
            n(h),
            n(w * 0.75),
            n(h * 0.5),
            n(w * 0.52),
            n(h * 0.5),
            n(w * 0.85)
        ),
        "moon" => {
            let t = frac(adj, "adj", 50000.0) * w;
            let rx = w / 2.0;
            let ry = h / 2.0;
            let irx = t / 2.0;
            format!(
                "<path d=\"M {w_n} 0 A {rx_n} {ry_n} 0 1 0 {w_n} {h_n} A {irx_n} {ry_n} 0 1 1 {w_n} 0 Z\"/>",
                rx_n = n(rx),
                ry_n = n(ry),
                irx_n = n(irx),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "teardrop" => {
            let d = frac(adj, "adj", 100000.0) * w.min(h) * 0.5;
            let rx = w / 2.0;
            let ry = h / 2.0;
            let cx = rx;
            let cy = ry;
            format!(
                "<path d=\"M {cx_n} 0 L {a1} 0 L {w_n} {a2} A {rx_n} {ry_n} 0 1 1 {cx_n} 0 Z\"/>",
                a1 = n(cx + d),
                a2 = n(cy - d + ry),
                cx_n = n(cx),
                rx_n = n(rx),
                ry_n = n(ry),
                w_n = n(w)
            )
        }
        "sun" => {
            let cx = w / 2.0;
            let cy = h / 2.0;
            let mut points: Vec<String> = Vec::with_capacity(16);
            for i in 0..16_u32 {
                let angle = (PI * 2.0 * f64::from(i)) / 16.0 - PI / 2.0;
                let r = if i % 2 == 0 { 1.0 } else { 0.7 };
                points.push(format!(
                    "{},{}",
                    n(cx + cx * r * angle.cos()),
                    n(cy + cy * r * angle.sin())
                ));
            }
            let circle_r = 0.4;
            let mut path = String::from("M ");
            for (i, p) in points.iter().enumerate() {
                if i == 0 {
                    path.push_str(p);
                } else {
                    path.push_str(" L ");
                    path.push_str(p);
                }
            }
            format!(
                "<path d=\"{path} Z M {a1} {cy_n} A {a2} {a3} 0 1 0 {a4} {cy_n} A {a2} {a3} 0 1 0 {a1} {cy_n} Z\"/>",
                a1 = n(cx + cx * circle_r),
                a2 = n(cx * circle_r),
                a3 = n(cy * circle_r),
                a4 = n(cx - cx * circle_r),
                cy_n = n(cy),
            )
        }
        "wave" => {
            let dy = frac(adj, "adj1", 12500.0) * h;
            let dx = frac(adj, "adj2", 0.0) * w;
            format!(
                "<path d=\"M {dx_n} {dy_n} C {a1} 0, {a2} 0, {w_n} {dy_n} L {a3} {a4} C {a5} {h_n}, {a6} {h_n}, 0 {a4} Z\"/>",
                dx_n = n(dx),
                dy_n = n(dy),
                a1 = n(dx + w * 0.25),
                a2 = n(dx + w * 0.5),
                a3 = n(w - dx),
                a4 = n(h - dy),
                a5 = n(w - dx - w * 0.25),
                a6 = n(w - dx - w * 0.5),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "doubleWave" => {
            let dy = frac(adj, "adj1", 6250.0) * h;
            let dx = frac(adj, "adj2", 0.0) * w;
            format!(
                "<path d=\"M {dx_n} {dy_n} C {a1} 0, {a2} {dy2}, {hw} {dy_n} C {a3} 0, {a4} {dy2}, {w_n} {dy_n} L {a5} {a6} C {a7} {h_n}, {a8} {a9}, {hw} {a6} C {a10} {h_n}, {a11} {a9}, 0 {a6} Z\"/>",
                dx_n = n(dx),
                dy_n = n(dy),
                a1 = n(dx + w * 0.167),
                a2 = n(dx + w * 0.333),
                a3 = n(w / 2.0 + w * 0.167),
                a4 = n(w / 2.0 + w * 0.333),
                a5 = n(w - dx),
                a6 = n(h - dy),
                a7 = n(w - dx - w * 0.167),
                a8 = n(w - dx - w * 0.333),
                a9 = n(h - dy * 2.0),
                a10 = n(w / 2.0 - w * 0.167),
                a11 = n(w / 2.0 - w * 0.333),
                hw = n(w / 2.0),
                dy2 = n(dy * 2.0),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "ribbon" => {
            let tab_h = frac(adj, "adj1", 16667.0) * h;
            let tab_w = frac(adj, "adj2", 50000.0) * w;
            let fold_w = tab_w * 0.3;
            format!(
                "<path d=\"M 0 {th_n} L {fw_n} {th15} L {fw_n} {h_n} L {tw_n} {ahth} L {a1} {ahth} L {a2} {h_n} L {a2} {th15} L {w_n} {th_n} L {w_n} 0 L {a1} 0 L {a1} {th_n} L {tw_n} {th_n} L {tw_n} 0 L 0 0 Z\"/>",
                th_n = n(tab_h),
                fw_n = n(fold_w),
                th15 = n(tab_h * 1.5),
                tw_n = n(tab_w),
                ahth = n(h - tab_h),
                a1 = n(w - tab_w),
                a2 = n(w - fold_w),
                w_n = n(w),
                h_n = n(h)
            )
        }
        "ribbon2" => {
            let tab_h = frac(adj, "adj1", 16667.0) * h;
            let tab_w = frac(adj, "adj2", 50000.0) * w;
            let fold_w = tab_w * 0.3;
            format!(
                "<path d=\"M 0 {a1} L {fw_n} {a2} L {fw_n} 0 L {tw_n} {th_n} L {a3} {th_n} L {a4} 0 L {a4} {a2} L {w_n} {a1} L {w_n} {h_n} L {a3} {h_n} L {a3} {a1} L {tw_n} {a1} L {tw_n} {h_n} L 0 {h_n} Z\"/>",
                fw_n = n(fold_w),
                tw_n = n(tab_w),
                th_n = n(tab_h),
                a1 = n(h - tab_h),
                a2 = n(h - tab_h * 1.5),
                a3 = n(w - tab_w),
                a4 = n(w - fold_w),
                w_n = n(w),
                h_n = n(h)
            )
        }

        // Unknown preset -> rectangle fallback (mirrors the spec).
        _ => fallback_rect(w, h),
    }
}

fn arc_path(w: f64, h: f64, adj: &Adj, close: bool, def_st: f64, def_end: f64) -> String {
    let st_ang = ooxml_angle_to_radians(a(adj, "adj1", def_st));
    let end_ang = ooxml_angle_to_radians(a(adj, "adj2", def_end));
    let rx = w / 2.0;
    let ry = h / 2.0;
    let cx = rx;
    let cy = ry;
    let x1 = cx + rx * st_ang.cos();
    let y1 = cy - ry * st_ang.sin();
    let x2 = cx + rx * end_ang.cos();
    let y2 = cy - ry * end_ang.sin();
    let mut sweep = st_ang - end_ang;
    if sweep < 0.0 {
        sweep += 2.0 * PI;
    }
    let large_arc = i32::from(sweep > PI);
    let close_seg = if close { " Z" } else { "" };
    format!(
        "<path d=\"M {x1_n} {y1_n} A {rx_n} {ry_n} 0 {la} 0 {x2_n} {y2_n}{close_seg}\"/>",
        x1_n = n(x1),
        y1_n = n(y1),
        x2_n = n(x2),
        y2_n = n(y2),
        rx_n = n(rx),
        ry_n = n(ry),
        la = large_arc
    )
}

fn pie_path(w: f64, h: f64, adj: &Adj) -> String {
    let st_ang = ooxml_angle_to_radians(a(adj, "adj1", 0.0));
    let end_ang = ooxml_angle_to_radians(a(adj, "adj2", 16_200_000.0));
    let rx = w / 2.0;
    let ry = h / 2.0;
    let cx = rx;
    let cy = ry;
    let x1 = cx + rx * st_ang.cos();
    let y1 = cy - ry * st_ang.sin();
    let x2 = cx + rx * end_ang.cos();
    let y2 = cy - ry * end_ang.sin();
    let mut sweep = st_ang - end_ang;
    if sweep < 0.0 {
        sweep += 2.0 * PI;
    }
    let large_arc = i32::from(sweep > PI);
    format!(
        "<path d=\"M {cx_n} {cy_n} L {x1_n} {y1_n} A {rx_n} {ry_n} 0 {la} 0 {x2_n} {y2_n} Z\"/>",
        cx_n = n(cx),
        cy_n = n(cy),
        x1_n = n(x1),
        y1_n = n(y1),
        x2_n = n(x2),
        y2_n = n(y2),
        rx_n = n(rx),
        ry_n = n(ry),
        la = large_arc
    )
}

fn block_arc(w: f64, h: f64, adj: &Adj) -> String {
    let st_ang = ooxml_angle_to_radians(a(adj, "adj1", 10_800_000.0));
    let end_ang = ooxml_angle_to_radians(a(adj, "adj2", 0.0));
    let thickness = frac(adj, "adj3", 25000.0);
    let rx = w / 2.0;
    let ry = h / 2.0;
    let cx = rx;
    let cy = ry;
    let irx = rx * (1.0 - thickness);
    let iry = ry * (1.0 - thickness);
    let ox1 = cx + rx * st_ang.cos();
    let oy1 = cy - ry * st_ang.sin();
    let ox2 = cx + rx * end_ang.cos();
    let oy2 = cy - ry * end_ang.sin();
    let ix1 = cx + irx * st_ang.cos();
    let iy1 = cy - iry * st_ang.sin();
    let ix2 = cx + irx * end_ang.cos();
    let iy2 = cy - iry * end_ang.sin();
    let mut sweep = st_ang - end_ang;
    if sweep < 0.0 {
        sweep += 2.0 * PI;
    }
    let large_arc = i32::from(sweep > PI);
    format!(
        "<path d=\"M {ox1_n} {oy1_n} A {rx_n} {ry_n} 0 {la} 0 {ox2_n} {oy2_n} L {ix2_n} {iy2_n} A {irx_n} {iry_n} 0 {la} 1 {ix1_n} {iy1_n} Z\"/>",
        ox1_n = n(ox1),
        oy1_n = n(oy1),
        ox2_n = n(ox2),
        oy2_n = n(oy2),
        ix1_n = n(ix1),
        iy1_n = n(iy1),
        ix2_n = n(ix2),
        iy2_n = n(iy2),
        rx_n = n(rx),
        ry_n = n(ry),
        irx_n = n(irx),
        iry_n = n(iry),
        la = large_arc
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> Adj {
        BTreeMap::new()
    }

    fn one(key: &str, value: f64) -> Adj {
        let mut m = BTreeMap::new();
        m.insert(key.to_string(), value);
        m
    }

    fn two(k1: &str, v1: f64, k2: &str, v2: f64) -> Adj {
        let mut m = BTreeMap::new();
        m.insert(k1.to_string(), v1);
        m.insert(k2.to_string(), v2);
        m
    }

    // --- Basic shapes ---

    #[test]
    fn rect_is_exact() {
        assert_eq!(
            preset_geometry_svg("rect", 100.0, 50.0, &empty()),
            "<rect width=\"100\" height=\"50\"/>"
        );
    }

    #[test]
    fn ellipse_dimensions() {
        let svg = preset_geometry_svg("ellipse", 200.0, 100.0, &empty());
        assert!(svg.contains("<ellipse"));
        assert!(svg.contains("cx=\"100\""));
        assert!(svg.contains("rx=\"100\""));
        assert!(svg.contains("ry=\"50\""));
    }

    #[test]
    fn round_rect_default_radius() {
        let svg = preset_geometry_svg("roundRect", 100.0, 100.0, &empty());
        assert!(svg.contains("<rect"));
        assert!(svg.contains("rx="));
        assert!(svg.contains("ry="));
    }

    #[test]
    fn round_rect_clamps_to_50000() {
        let svg = preset_geometry_svg("roundRect", 100.0, 100.0, &one("adj", 112_199.0));
        assert!(svg.contains("rx=\"50\""));
        assert!(svg.contains("ry=\"50\""));

        let neg = preset_geometry_svg("roundRect", 100.0, 100.0, &one("adj", -1.0));
        assert!(neg.contains("rx=\"0\""));
    }

    #[test]
    fn round1_rect_clamps_to_50000() {
        let svg = preset_geometry_svg("round1Rect", 100.0, 100.0, &one("adj", 80_000.0));
        assert!(svg.contains("<path"));
        assert!(svg.contains("A 50 50"));
    }

    #[test]
    fn round2_same_rect_clamps_both() {
        let svg = preset_geometry_svg(
            "round2SameRect",
            100.0,
            100.0,
            &two("adj1", 80_000.0, "adj2", -5_000.0),
        );
        assert!(svg.contains("A 50 50"));
        assert!(svg.contains("A 0 0"));
    }

    #[test]
    fn round2_diag_rect_clamps_both() {
        let svg = preset_geometry_svg(
            "round2DiagRect",
            100.0,
            100.0,
            &two("adj1", 99_999.0, "adj2", -100.0),
        );
        assert!(svg.contains("A 50 50"));
        assert!(svg.contains("A 0 0"));
    }

    #[test]
    fn triangle_emits_polygon() {
        let svg = preset_geometry_svg("triangle", 100.0, 80.0, &empty());
        assert!(svg.contains("<polygon"));
        assert!(svg.contains("points="));
    }

    #[test]
    fn diamond_polygon() {
        assert!(preset_geometry_svg("diamond", 100.0, 100.0, &empty()).contains("<polygon"));
    }

    #[test]
    fn unknown_falls_back_to_rect() {
        assert_eq!(
            preset_geometry_svg("unknownShape", 100.0, 50.0, &empty()),
            "<rect width=\"100\" height=\"50\"/>"
        );
    }

    // --- Polygons / stars ---

    #[test]
    fn polygons_emit_polygon_tag() {
        for name in ["heptagon", "octagon", "decagon", "dodecagon"] {
            assert!(
                preset_geometry_svg(name, 100.0, 100.0, &empty()).contains("<polygon"),
                "{name}"
            );
        }
    }

    #[test]
    fn stars_emit_polygon_tag() {
        for name in [
            "star4", "star5", "star6", "star7", "star8", "star10", "star12", "star16", "star24",
            "star32",
        ] {
            assert!(
                preset_geometry_svg(name, 100.0, 100.0, &empty()).contains("<polygon"),
                "{name}"
            );
        }
    }

    #[test]
    fn irregular_seals_emit_polygon() {
        for name in ["irregularSeal1", "irregularSeal2"] {
            assert!(preset_geometry_svg(name, 100.0, 100.0, &empty()).contains("<polygon"));
        }
    }

    // --- Arrows ---

    #[test]
    fn arrows_emit_polygon_or_path() {
        for name in [
            "rightArrow",
            "leftArrow",
            "upArrow",
            "downArrow",
            "leftRightArrow",
            "upDownArrow",
            "notchedRightArrow",
            "chevron",
            "homePlate",
            "bentArrow",
            "bendUpArrow",
        ] {
            let svg = preset_geometry_svg(name, 200.0, 100.0, &empty());
            assert!(svg.contains("<polygon"), "{name}");
        }
        for name in [
            "stripedRightArrow",
            "quadArrow",
            "leftUpArrow",
            "leftRightUpArrow",
            "uturnArrow",
        ] {
            let svg = preset_geometry_svg(name, 200.0, 200.0, &empty());
            assert!(svg.contains("<path"), "{name}");
        }
    }

    // --- Flowchart ---

    #[test]
    fn flow_chart_process_is_rect() {
        assert_eq!(
            preset_geometry_svg("flowChartProcess", 100.0, 50.0, &empty()),
            "<rect width=\"100\" height=\"50\"/>"
        );
    }

    #[test]
    fn flow_chart_terminator_is_full_rounded_rect() {
        let svg = preset_geometry_svg("flowChartTerminator", 200.0, 60.0, &empty());
        assert!(svg.contains("<rect"));
        assert!(svg.contains("rx=\"30\""));
    }

    #[test]
    fn flow_chart_decision_starts_at_top_center() {
        let svg = preset_geometry_svg("flowChartDecision", 100.0, 100.0, &empty());
        assert!(svg.contains("<polygon"));
        assert!(svg.contains("50,0"));
    }

    #[test]
    fn flow_chart_paths_emit_path_tag() {
        for name in [
            "flowChartPredefinedProcess",
            "flowChartInternalStorage",
            "flowChartDocument",
            "flowChartMultidocument",
            "flowChartPunchedTape",
            "flowChartOnlineStorage",
            "flowChartDelay",
            "flowChartDisplay",
            "flowChartMagneticTape",
            "flowChartMagneticDisk",
            "flowChartMagneticDrum",
            "flowChartSummingJunction",
            "flowChartOr",
            "flowChartSort",
        ] {
            let svg = preset_geometry_svg(name, 100.0, 100.0, &empty());
            assert!(svg.contains("<path"), "{name}");
        }
    }

    #[test]
    fn flow_chart_polygons_emit_polygon_tag() {
        for name in [
            "flowChartPreparation",
            "flowChartManualInput",
            "flowChartManualOperation",
            "flowChartOffpageConnector",
            "flowChartPunchedCard",
            "flowChartCollate",
            "flowChartExtract",
            "flowChartMerge",
        ] {
            let svg = preset_geometry_svg(name, 100.0, 100.0, &empty());
            assert!(svg.contains("<polygon"), "{name}");
        }
    }

    #[test]
    fn flow_chart_connector_is_ellipse() {
        let svg = preset_geometry_svg("flowChartConnector", 100.0, 100.0, &empty());
        assert!(svg.contains("<ellipse"));
    }

    // --- Callouts ---

    #[test]
    fn callouts_emit_path() {
        for name in [
            "wedgeRectCallout",
            "wedgeRoundRectCallout",
            "wedgeEllipseCallout",
            "cloudCallout",
            "borderCallout1",
            "borderCallout2",
            "borderCallout3",
        ] {
            let svg = preset_geometry_svg(name, 200.0, 100.0, &empty());
            assert!(svg.contains("<path"), "{name}");
        }
    }

    // --- Arcs ---

    #[test]
    fn arc_shapes_emit_path() {
        for name in ["arc", "chord", "pie", "blockArc"] {
            assert!(preset_geometry_svg(name, 100.0, 100.0, &empty()).contains("<path"));
        }
    }

    #[test]
    fn pie_with_custom_angles() {
        let svg = preset_geometry_svg(
            "pie",
            100.0,
            100.0,
            &two("adj1", 5_400_000.0, "adj2", 16_200_000.0),
        );
        assert!(svg.contains("<path"));
        assert!(svg.contains("A 50 50"));
    }

    // --- Math ---

    #[test]
    fn math_plus_is_polygon() {
        assert!(preset_geometry_svg("mathPlus", 100.0, 100.0, &empty()).contains("<polygon"));
    }

    #[test]
    fn math_minus_is_rect() {
        assert!(preset_geometry_svg("mathMinus", 100.0, 100.0, &empty()).contains("<rect"));
    }

    #[test]
    fn math_paths() {
        for name in ["mathMultiply", "mathDivide", "mathEqual", "mathNotEqual"] {
            assert!(
                preset_geometry_svg(name, 100.0, 100.0, &empty()).contains("<path"),
                "{name}"
            );
        }
    }

    // --- Other shapes ---

    #[test]
    fn plus_polygon() {
        assert!(preset_geometry_svg("plus", 100.0, 100.0, &empty()).contains("<polygon"));
    }

    #[test]
    fn corner_and_snip_polygons() {
        for name in [
            "corner",
            "halfFrame",
            "snip1Rect",
            "snip2SameRect",
            "snip2DiagRect",
        ] {
            let svg = preset_geometry_svg(name, 100.0, 100.0, &empty());
            assert!(svg.contains("<polygon") || svg.contains("<path"));
        }
    }

    #[test]
    fn donut_uses_evenodd_fill_rule() {
        let svg = preset_geometry_svg("donut", 100.0, 100.0, &empty());
        assert!(svg.contains("fill-rule=\"evenodd\""));
    }

    #[test]
    fn frame_uses_evenodd_fill_rule() {
        assert!(
            preset_geometry_svg("frame", 100.0, 100.0, &empty()).contains("fill-rule=\"evenodd\"")
        );
    }

    #[test]
    fn no_smoking_uses_evenodd_fill_rule() {
        assert!(preset_geometry_svg("noSmoking", 100.0, 100.0, &empty())
            .contains("fill-rule=\"evenodd\""));
    }

    #[test]
    fn brackets_emit_path() {
        for name in [
            "leftBracket",
            "rightBracket",
            "leftBrace",
            "rightBrace",
            "bracketPair",
            "bracePair",
        ] {
            assert!(preset_geometry_svg(name, 20.0, 100.0, &empty()).contains("<path"));
        }
    }

    #[test]
    fn waves_and_ribbons_emit_path() {
        for name in ["wave", "doubleWave", "ribbon", "ribbon2"] {
            assert!(preset_geometry_svg(name, 200.0, 100.0, &empty()).contains("<path"));
        }
    }

    #[test]
    fn diag_stripe_polygon() {
        assert!(preset_geometry_svg("diagStripe", 100.0, 100.0, &empty()).contains("<polygon"));
    }

    #[test]
    fn line_emits_path() {
        let svg = preset_geometry_svg("line", 200.0, 100.0, &empty());
        assert_eq!(svg, "<path d=\"M 0 0 L 200 100\"/>");
    }
}
