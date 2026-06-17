//! `Fill` attribute renderer — pattern subtype.
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

use slideglance_model::PatternFill;
use std::fmt::Write as _;

use crate::color::{alpha_str, color_hex};
use crate::id_gen::IdGen;

use super::{FillAttrs, PatternContent};

pub(super) fn render_pattern_fill(fill: &PatternFill, ids: &mut IdGen) -> FillAttrs {
    let fg = color_hex(&fill.foreground_color);
    let bg = color_hex(&fill.background_color);
    let fg_alpha_attr = if fill.foreground_color.alpha < 1.0 {
        format!(" opacity=\"{}\"", alpha_str(fill.foreground_color.alpha))
    } else {
        String::new()
    };

    let Some(content) = pattern_content(&fill.preset, &fg, &fg_alpha_attr) else {
        // No SVG body for this preset -> treat like a solid fill of the
        // foreground color, mirroring the TS fallback.
        let mut attrs = format!("fill=\"{fg}\"");
        if fill.foreground_color.alpha < 1.0 {
            let _ = write!(
                attrs,
                " fill-opacity=\"{}\"",
                alpha_str(fill.foreground_color.alpha)
            );
        }
        return FillAttrs {
            attrs,
            defs: String::new(),
        };
    };

    let id = ids.next_id("patt");
    let bg_alpha = if fill.background_color.alpha < 1.0 {
        format!(
            " fill-opacity=\"{}\"",
            alpha_str(fill.background_color.alpha)
        )
    } else {
        String::new()
    };
    let defs = format!(
        "<pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" width=\"{size}\" height=\"{size}\"><rect width=\"{size}\" height=\"{size}\" fill=\"{bg}\"{bg_alpha}/>{body}</pattern>",
        size = content.size,
        body = content.svg
    );
    FillAttrs {
        attrs: format!("fill=\"url(#{id})\""),
        defs,
    }
}

pub(super) fn pattern_content(preset: &str, fg: &str, fg_alpha: &str) -> Option<PatternContent> {
    const S: u32 = 8;
    const SW: u32 = 1;
    let line = |x1: i32, y1: i32, x2: i32, y2: i32| {
        format!(
            "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"{fg}\" stroke-width=\"{SW}\"{fg_alpha}/>"
        )
    };
    match preset {
        "ltHorz" | "horz" => Some(PatternContent { svg: line(0, 4, 8, 4), size: S }),
        "ltVert" | "vert" => Some(PatternContent { svg: line(4, 0, 4, 8), size: S }),
        "ltDnDiag" | "dnDiag" => Some(PatternContent { svg: line(0, 0, 8, 8), size: S }),
        "ltUpDiag" | "upDiag" => Some(PatternContent { svg: line(0, 8, 8, 0), size: S }),
        "dkHorz" => Some(PatternContent {
            svg: format!("{}{}", line(0, 2, 8, 2), line(0, 6, 8, 6)),
            size: S,
        }),
        "dkVert" => Some(PatternContent {
            svg: format!("{}{}", line(2, 0, 2, 8), line(6, 0, 6, 8)),
            size: S,
        }),
        "dkDnDiag" => Some(PatternContent {
            svg: format!("{}{}", line(0, 0, 8, 8), line(-4, 0, 4, 8)),
            size: S,
        }),
        "dkUpDiag" => Some(PatternContent {
            svg: format!("{}{}", line(0, 8, 8, 0), line(4, 8, 12, 0)),
            size: S,
        }),
        "cross" | "smGrid" => Some(PatternContent {
            svg: format!("{}{}", line(0, 4, 8, 4), line(4, 0, 4, 8)),
            size: S,
        }),
        "lgGrid" => Some(PatternContent {
            svg: format!("{}{}", line(0, 0, 16, 0), line(0, 0, 0, 16)),
            size: 16,
        }),
        "diagCross" => Some(PatternContent {
            svg: format!("{}{}", line(0, 0, 8, 8), line(0, 8, 8, 0)),
            size: S,
        }),
        "pct5" => Some(PatternContent {
            svg: format!("<rect x=\"0\" y=\"0\" width=\"1\" height=\"1\" fill=\"{fg}\"{fg_alpha}/>"),
            size: S,
        }),
        "pct10" => Some(PatternContent {
            svg: format!(
                "<rect x=\"0\" y=\"0\" width=\"1\" height=\"1\" fill=\"{fg}\"{fg_alpha}/><rect x=\"4\" y=\"4\" width=\"1\" height=\"1\" fill=\"{fg}\"{fg_alpha}/>"
            ),
            size: S,
        }),
        "pct20" => Some(PatternContent {
            svg: format!(
                "<rect x=\"0\" y=\"0\" width=\"2\" height=\"2\" fill=\"{fg}\"{fg_alpha}/><rect x=\"4\" y=\"4\" width=\"2\" height=\"2\" fill=\"{fg}\"{fg_alpha}/>"
            ),
            size: S,
        }),
        "pct25" => Some(PatternContent {
            svg: format!(
                "<rect x=\"0\" y=\"0\" width=\"2\" height=\"2\" fill=\"{fg}\"{fg_alpha}/><rect x=\"4\" y=\"0\" width=\"2\" height=\"2\" fill=\"{fg}\"{fg_alpha}/><rect x=\"2\" y=\"4\" width=\"2\" height=\"2\" fill=\"{fg}\"{fg_alpha}/><rect x=\"6\" y=\"4\" width=\"2\" height=\"2\" fill=\"{fg}\"{fg_alpha}/>"
            ),
            size: S,
        }),
        "pct30" | "pct40" | "pct50" | "pct60" | "pct70" | "pct75" | "pct80" | "pct90" => {
            // Trim "pct" prefix; the remainder is always 2 digits in OOXML.
            let pct_val: f64 = preset.trim_start_matches("pct").parse().ok()?;
            let alpha = pct_val / 100.0;
            Some(PatternContent {
                svg: format!(
                    "<rect width=\"{S}\" height=\"{S}\" fill=\"{fg}\" opacity=\"{}\"{fg_alpha}/>",
                    alpha_str(alpha)
                ),
                size: S,
            })
        }
        _ => None,
    }
}
