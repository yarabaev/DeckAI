//! `Fill` attribute renderer — gradient subtype.
//!
//! Extracted from `fill.rs` so each fill family lives in its own
//! module. Shared types (`FillAttrs`, `MarkerResult`) and the
//! public dispatcher (`render_fill_attrs` / `render_outline_attrs`
//! / `render_markers`) stay in `fill::mod`.

//! Fill / outline / marker attribute rendering.
//!
//! Direct port of. The TS code
//! uses `crypto.randomUUID()` for `<defs>` IDs; we replace that with the
//! deterministic [`IdGen`] counter (see `id_gen.rs`). Output is
//! "attrs" — the inline attribute fragment (`fill="..."`, `stroke="..."` …) —
//! plus zero or more `<defs>` definitions to splice into the slide's `<defs>`
//! block.

use slideglance_model::{GradientFill, GradientType};
use std::f64::consts::PI;
use std::fmt::Write as _;

use crate::color::{alpha_str, color_hex};
use crate::geometry::fmt::n;
use crate::id_gen::IdGen;

use super::GradientRef;

pub(super) fn render_gradient_defs(fill: &GradientFill, ids: &mut IdGen) -> GradientRef {
    let id = ids.next_id("grad");
    // SVG <stop offset> values must be monotonically non-decreasing —
    // out-of-order offsets are clamped to the running max, silently
    // collapsing later stops onto earlier offsets. PowerPoint's
    // gradient stop list is not always sorted (slide 11's down-arrow
    // gradient lists pos=23000, then 65000, then 0 — the 0% stop is
    // clamped to 65% in resvg, which made the arrow's tip render as
    // a transparent rectangle behind the right-side box). Sort by
    // position before emitting so SVG renderers honour every stop.
    let mut sorted: Vec<&slideglance_model::GradientStop> = fill.stops.iter().collect();
    sorted.sort_by(|a, b| {
        a.position
            .partial_cmp(&b.position)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut stops = String::new();
    for s in &sorted {
        let opacity = if s.color.alpha < 1.0 {
            format!(" stop-opacity=\"{}\"", alpha_str(s.color.alpha))
        } else {
            String::new()
        };
        let _ = write!(
            stops,
            "<stop offset=\"{}%\" stop-color=\"{}\"{opacity}/>",
            n(s.position * 100.0),
            color_hex(&s.color)
        );
    }
    let defs = if matches!(fill.gradient_type, GradientType::Radial) {
        let cx = fill.center_x.unwrap_or(0.5) * 100.0;
        let cy = fill.center_y.unwrap_or(0.5) * 100.0;
        let dx = cx.max(100.0 - cx);
        let dy = cy.max(100.0 - cy);
        let r = (dx * dx + dy * dy).sqrt();
        format!(
            "<radialGradient id=\"{id}\" cx=\"{}%\" cy=\"{}%\" r=\"{}%\">{stops}</radialGradient>",
            n(cx),
            n(cy),
            n(r)
        )
    } else {
        let rad = (fill.angle * PI) / 180.0;
        let x1 = 50.0 - rad.cos() * 50.0;
        let y1 = 50.0 - rad.sin() * 50.0;
        let x2 = 50.0 + rad.cos() * 50.0;
        let y2 = 50.0 + rad.sin() * 50.0;
        format!(
            "<linearGradient id=\"{id}\" x1=\"{}%\" y1=\"{}%\" x2=\"{}%\" y2=\"{}%\">{stops}</linearGradient>",
            n(x1),
            n(y1),
            n(x2),
            n(y2)
        )
    };
    GradientRef {
        reference: format!("url(#{id})"),
        defs,
    }
}
