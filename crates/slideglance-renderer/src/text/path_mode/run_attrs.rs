//! Path-mode helpers — run-attribute builders + bullet / decoration emitters.
//!
//! Extracted from `path_mode.rs` so the main entry stays focused on
//! the paragraph -> wrapped-line dispatch and emission helpers can
//! be reviewed independently.

use std::fmt::Write as _;

use slideglance_font::{
    text_to_svg_path_with_precision, CjkPlatform, FontFace, FontMapping, FontResolver,
};
use slideglance_model::{ParagraphAlignment, ParagraphProperties, RunProperties};

use crate::color::{alpha_str, color_hex};

use super::DECIMAL_PLACES;

/// Build the `fill="..."` (and optional `fill-opacity`) attribute value
/// for one path-mode glyph fragment. Defaults to `#000000` when the run
/// has no explicit color, matching the spec.
#[must_use]
pub fn build_path_fill_attrs(props: &RunProperties) -> String {
    if let Some(color) = &props.color {
        let mut s = format!("fill=\"{}\"", color_hex(color));
        if color.alpha < 1.0 {
            let _ = write!(s, " fill-opacity=\"{}\"", alpha_str(color.alpha));
        }
        s
    } else {
        "fill=\"#000000\"".to_string()
    }
}

/// Build the `fill=…[ stroke=… stroke-width=…]` attribute string for a
/// glyph path. When `synthesize_bold` is true the fill color is also painted
/// as a thin stroke around the glyph outline, reproducing `PowerPoint`'s
/// "faux bold" widening for `<a:rPr b="1">` runs whose resolved face does
/// not already carry the requested weight. The stroke width is intentionally
/// small (`~2.5%` of font size) so well-weighted faces don't muddy.
#[must_use]
pub fn build_path_attrs(props: &RunProperties, font_size_px: f64, synthesize_bold: bool) -> String {
    let mut s = build_path_fill_attrs(props);
    if !synthesize_bold {
        return s;
    }
    let stroke_color = props
        .color
        .as_ref()
        .map_or_else(|| "#000000".to_string(), color_hex);
    let stroke_width = font_size_px * 0.015;
    let _ = write!(
        s,
        " stroke=\"{stroke_color}\" stroke-width=\"{stroke_width:.3}\" paint-order=\"stroke fill\""
    );
    if let Some(color) = props.color.as_ref() {
        if color.alpha < 1.0 {
            let _ = write!(s, " stroke-opacity=\"{}\"", alpha_str(color.alpha));
        }
    }
    s
}

/// Determine whether a path-mode glyph should be stroke-widened to mimic
/// `PowerPoint`'s faux-bold pass. Triggered when the run requests bold
/// (`<a:rPr b="1">`) AND the resolved face weight class is **below 700**
/// (CSS Bold) — when the resolved face already carries Bold weight or
/// heavier, additional stroke causes glyph muddying (visible in 132-slide
/// sweep on titles using "Freesentation 7 Bold" / Apple SD Gothic Neo Bold
/// where weight=700 is reported and stroke widened the glyphs by ~1px,
/// producing red/blue halos that swelled the diff by ~5–7 pp on
/// title-heavy decks like slides 96 / 100).
pub(super) fn should_synthesize_bold(props: &RunProperties, face: Option<&FontFace>) -> bool {
    if !props.bold {
        return false;
    }
    match face {
        Some(f) => f.weight() < 700,
        None => true, // No face resolved → fallback heuristic likely lighter than Bold.
    }
}

/// Emit underline / strikethrough `<line>` elements for one segment.
/// Returns the concatenated SVG fragment (possibly empty).
#[must_use]
pub fn render_text_decorations(
    x: f64,
    y: f64,
    segment_width: f64,
    font_size_px: f64,
    props: &RunProperties,
) -> String {
    let mut out = String::new();
    let stroke_color = props
        .color
        .as_ref()
        .map_or_else(|| "#000000".to_string(), color_hex);
    let stroke_width = (font_size_px * 0.05).max(1.0);
    let opacity_attr = props
        .color
        .as_ref()
        .filter(|c| c.alpha < 1.0)
        .map(|c| format!(" stroke-opacity=\"{}\"", alpha_str(c.alpha)))
        .unwrap_or_default();

    if props.underline {
        let underline_y = y + font_size_px * 0.15;
        let _ = write!(
            out,
            "<line x1=\"{x:.2}\" y1=\"{underline_y:.2}\" x2=\"{:.2}\" y2=\"{underline_y:.2}\" stroke=\"{stroke_color}\" stroke-width=\"{stroke_width:.2}\"{opacity_attr}/>",
            x + segment_width
        );
    }
    if props.strikethrough {
        let strike_y = y - font_size_px * 0.3;
        let _ = write!(
            out,
            "<line x1=\"{x:.2}\" y1=\"{strike_y:.2}\" x2=\"{:.2}\" y2=\"{strike_y:.2}\" stroke=\"{stroke_color}\" stroke-width=\"{stroke_width:.2}\"{opacity_attr}/>",
            x + segment_width
        );
    }
    out
}

/// Compute the line-start X position based on alignment, mirroring TS's
/// `computePathLineX`. Path-mode does not rely on `text-anchor`, so the
/// line's left edge is positioned explicitly per alignment.
#[must_use]
pub fn compute_path_line_x(
    alignment: Option<ParagraphAlignment>,
    text_start_x: f64,
    effective_text_width: f64,
    width: f64,
    margin_right_px: f64,
    line_width: f64,
) -> f64 {
    match alignment {
        Some(ParagraphAlignment::Ctr) => text_start_x + (effective_text_width - line_width) / 2.0,
        Some(ParagraphAlignment::R) => width - margin_right_px - line_width,
        _ => text_start_x,
    }
}

/// Render a bullet character as a single `<path>` element. Returns an
/// empty string when the bullet font cannot be resolved.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn render_bullet_as_path(
    bullet_text: &str,
    x: f64,
    y: f64,
    para_props: &ParagraphProperties,
    text_font_size_pt: f64,
    font_resolver: &dyn FontResolver,
    _mapping: &FontMapping,
    _cjk_platform: CjkPlatform,
    run_font_family: Option<&str>,
    run_font_family_ea: Option<&str>,
) -> String {
    let bullet_font_size = match para_props.bullet_size_pct {
        Some(pct) => text_font_size_pt * (pct / 100_000.0),
        None => text_font_size_pt,
    };

    let face = if let Some(name) = &para_props.bullet_font {
        font_resolver.resolve(name)
    } else {
        run_font_family
            .and_then(|n| font_resolver.resolve(n))
            .or_else(|| run_font_family_ea.and_then(|n| font_resolver.resolve(n)))
    };
    let Some(face) = face else {
        return String::new();
    };

    let path_data =
        text_to_svg_path_with_precision(&face, bullet_text, x, y, bullet_font_size, DECIMAL_PLACES);
    if path_data.is_empty() {
        return String::new();
    }

    let mut attrs = Vec::new();
    if let Some(color) = &para_props.bullet_color {
        attrs.push(format!("fill=\"{}\"", color_hex(color)));
        if color.alpha < 1.0 {
            attrs.push(format!("fill-opacity=\"{}\"", alpha_str(color.alpha)));
        }
    } else {
        attrs.push("fill=\"#000000\"".to_string());
    }
    format!("<path d=\"{path_data}\" {}/>", attrs.join(" "))
}
