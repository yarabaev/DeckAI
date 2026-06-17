//! `Fill` attribute renderer — marker subtype.
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

use slideglance_model::{ArrowEndpoint, ArrowType};

use crate::color::alpha_str;
use crate::geometry::fmt::n;

use super::arrow_size_px;

pub(super) fn build_marker_def(
    id: &str,
    end: ArrowEndpoint,
    color: &str,
    alpha: f64,
    is_start: bool,
) -> Option<String> {
    let mw = arrow_size_px(end.length);
    let mh = arrow_size_px(end.width);
    let alpha_attr = if alpha < 1.0 {
        format!(" opacity=\"{}\"", alpha_str(alpha))
    } else {
        String::new()
    };
    let mwf = f64::from(mw);
    let mhf = f64::from(mh);
    // SVG `auto` for marker-start aligns the marker's +x with the OUTGOING
    // tangent (toward the line interior), which is the opposite of a
    // PowerPoint headEnd arrow (which points AWAY from the line). Use
    // `auto-start-reverse` for marker-start to flip that 180° so the
    // arrowhead points outward — and stays correct even when the parent
    // group applies a `scale(1, -1)` for `flipV=1` connectors. marker-end
    // keeps `auto` because marker-end already uses the incoming tangent
    // direction, which matches PowerPoint's tailEnd semantics.
    let orient = if is_start {
        "auto-start-reverse"
    } else {
        "auto"
    };

    let (path, fill_attr) = match end.ty {
        ArrowType::Triangle => (
            format!("M 0 0 L {mw} {} L 0 {mh} Z", n(mhf / 2.0)),
            format!("fill=\"{color}\""),
        ),
        ArrowType::Stealth => (
            format!(
                "M 0 0 L {mw} {} L 0 {mh} L {} {} Z",
                n(mhf / 2.0),
                n(mwf * 0.3),
                n(mhf / 2.0)
            ),
            format!("fill=\"{color}\""),
        ),
        ArrowType::Diamond => (
            format!(
                "M 0 {} L {} 0 L {mw} {} L {} {mh} Z",
                n(mhf / 2.0),
                n(mwf / 2.0),
                n(mhf / 2.0),
                n(mwf / 2.0)
            ),
            format!("fill=\"{color}\""),
        ),
        ArrowType::Oval => {
            return Some(format!(
                "<marker id=\"{id}\" markerWidth=\"{mw}\" markerHeight=\"{mh}\" refX=\"{}\" refY=\"{}\" orient=\"{orient}\" markerUnits=\"userSpaceOnUse\"><ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{color}\"{alpha_attr}/></marker>",
                n(mwf / 2.0),
                n(mhf / 2.0),
                n(mwf / 2.0),
                n(mhf / 2.0),
                n(mwf / 2.0),
                n(mhf / 2.0)
            ));
        }
        ArrowType::Arrow => (
            format!("M 0 0 L {mw} {} L 0 {mh}", n(mhf / 2.0)),
            format!("fill=\"none\" stroke=\"{color}\" stroke-width=\"1\""),
        ),
        ArrowType::None => return None,
    };
    Some(format!(
        "<marker id=\"{id}\" markerWidth=\"{mw}\" markerHeight=\"{mh}\" refX=\"{mw}\" refY=\"{}\" orient=\"{orient}\" markerUnits=\"userSpaceOnUse\"><path d=\"{path}\" {fill_attr}{alpha_attr}/></marker>",
        n(mhf / 2.0)
    ))
}
