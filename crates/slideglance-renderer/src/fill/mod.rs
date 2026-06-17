//! Fill / outline / marker attribute rendering.
//!
//! Direct port of. The TS code
//! uses `crypto.randomUUID()` for `<defs>` IDs; we replace that with the
//! deterministic [`IdGen`] counter (see `id_gen.rs`). Output is
//! "attrs" — the inline attribute fragment (`fill="..."`, `stroke="..."` …) —
//! plus zero or more `<defs>` definitions to splice into the slide's `<defs>`
//! block.

use slideglance_color::ResolvedColor;
use slideglance_model::{
    ArrowSize, ArrowType, DashStyle, Fill, LineCap, LineJoin, Outline, OutlineFill,
};
use std::fmt::Write as _;

use crate::color::{alpha_str, color_hex};
use crate::geometry::fmt::n;
use crate::id_gen::IdGen;

mod gradient;
mod marker;
mod pattern;

use gradient::render_gradient_defs;
use marker::build_marker_def;
use pattern::render_pattern_fill;

/// The "attrs / defs" pair every fill renderer returns.
///
/// `attrs` is appended (after a leading space if non-empty) to the element
/// tag; `defs` is pushed onto the slide's collected `<defs>` list.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FillAttrs {
    /// Inline attribute fragment (no leading space).
    pub attrs: String,
    /// `<defs>` content; empty when no definition is needed.
    pub defs: String,
}

/// Marker (`<marker>` arrow) rendering result.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MarkerResult {
    /// `<defs>` content for any minted `<marker>` definitions.
    pub defs: String,
    /// `marker-start="..."` attribute (empty when no head end).
    pub start_attr: String,
    /// `marker-end="..."` attribute (empty when no tail end).
    pub end_attr: String,
}

/// Render the `fill="..."` (and any associated `<defs>`) attribute fragment
/// for an element. `None` and [`Fill::None`] both produce `fill="none"`.
#[must_use]
pub fn render_fill_attrs(fill: Option<&Fill>, ids: &mut IdGen) -> FillAttrs {
    let Some(fill) = fill else {
        return none_fill();
    };
    match fill {
        Fill::None(_) => none_fill(),
        Fill::Solid(s) => solid_attrs(&s.color),
        Fill::Gradient(g) => {
            let res = render_gradient_defs(g, ids);
            FillAttrs {
                attrs: format!("fill=\"{}\"", res.reference),
                defs: res.defs,
            }
        }
        Fill::Image(img) => {
            // EMF / WMF metafiles: try to extract the embedded raster (DIB → PNG)
            // and treat the result as a regular PNG. Real vector EMFs return
            // None and fall back to a neutral grey fill (spec).
            let rasterized;
            let img = if img.mime_type == "image/emf" || img.mime_type == "image/wmf" {
                match crate::image::try_rasterize_metafile(&img.image_data) {
                    Some(png_data) => {
                        let mut clone = img.clone();
                        clone.mime_type = "image/png".to_string();
                        clone.image_data = png_data;
                        rasterized = clone;
                        &rasterized
                    }
                    None => {
                        return FillAttrs {
                            attrs: "fill=\"#E0E0E0\"".to_string(),
                            defs: String::new(),
                        };
                    }
                }
            } else {
                img
            };
            let id = ids.next_id("imgfill");
            // `<a:blip><a:alphaModFix amt>` — apply via `fill-opacity` on the
            // path that references the pattern. Composing on the path keeps
            // the pattern definition reusable and avoids relying on
            // `<image opacity>` inside `<pattern>` (some renderers ignore
            // opacity on pattern children).
            let fill_opacity_attr = if img.alpha < 1.0 {
                format!(" fill-opacity=\"{}\"", alpha_str(img.alpha.max(0.0)))
            } else {
                String::new()
            };
            if let Some(t) = &img.tile {
                let scale = |v: f64| format!("{}%", n(v * 100.0));
                let defs = format!(
                    "<pattern id=\"{id}\" patternUnits=\"objectBoundingBox\" width=\"{}\" height=\"{}\"><image href=\"data:{};base64,{}\" width=\"100%\" height=\"100%\" preserveAspectRatio=\"none\"/></pattern>",
                    scale(t.sx),
                    scale(t.sy),
                    img.mime_type,
                    img.image_data
                );
                return FillAttrs {
                    attrs: format!("fill=\"url(#{id})\"{fill_opacity_attr}"),
                    defs,
                };
            }
            let defs = format!(
                "<pattern id=\"{id}\" patternContentUnits=\"objectBoundingBox\" width=\"1\" height=\"1\"><image href=\"data:{};base64,{}\" width=\"1\" height=\"1\" preserveAspectRatio=\"none\"/></pattern>",
                img.mime_type, img.image_data
            );
            FillAttrs {
                attrs: format!("fill=\"url(#{id})\"{fill_opacity_attr}"),
                defs,
            }
        }
        Fill::Pattern(p) => render_pattern_fill(p, ids),
    }
}

/// Render the `stroke=*`/`stroke-width=*`/dash/cap/join attribute fragment
/// for an outline, plus any gradient-stroke `<defs>`. `None` -> `stroke="none"`.
///
/// `stroke_scale` is the inverse of the cumulative `<g transform="scale(...)">`
/// chain applied by ancestor groups. SVG strokes scale with the parent
/// transform, but `PowerPoint` treats `<a:ln w>` as an absolute length
/// (independent of any parent group's chExt → ext ratio). Multiplying
/// the local stroke-width by `stroke_scale` cancels the upstream scale
/// so the rendered stroke matches the source. Pass `1.0` for callers
/// outside a scaled group (e.g. unit tests, top-level connectors).
#[must_use]
pub fn render_outline_attrs(
    outline: Option<&Outline>,
    ids: &mut IdGen,
    stroke_scale: f64,
) -> FillAttrs {
    let Some(outline) = outline else {
        return FillAttrs {
            attrs: "stroke=\"none\"".to_string(),
            defs: String::new(),
        };
    };
    let width_px = outline.width.to_pixels() * stroke_scale;
    let mut parts: Vec<String> = vec![format!("stroke-width=\"{}\"", n(width_px))];
    let mut defs = String::new();

    match &outline.fill {
        Some(OutlineFill::Solid(s)) => {
            parts.push(format!("stroke=\"{}\"", color_hex(&s.color)));
            if s.color.alpha < 1.0 {
                parts.push(format!("stroke-opacity=\"{}\"", alpha_str(s.color.alpha)));
            }
        }
        Some(OutlineFill::Gradient(g)) => {
            let res = render_gradient_defs(g, ids);
            parts.push(format!("stroke=\"{}\"", res.reference));
            defs = res.defs;
        }
        None => parts.push("stroke=\"none\"".to_string()),
    }

    if let Some(custom) = &outline.custom_dash {
        if !custom.is_empty() {
            let mut dash = String::new();
            for (i, v) in custom.iter().enumerate() {
                if i > 0 {
                    dash.push(' ');
                }
                let _ = write!(dash, "{}", n(v * width_px));
            }
            parts.push(format!("stroke-dasharray=\"{dash}\""));
        }
    } else if outline.dash_style != DashStyle::Solid {
        if let Some(dash) = dash_array(outline.dash_style, width_px) {
            parts.push(format!("stroke-dasharray=\"{dash}\""));
        }
    }

    if let Some(cap) = outline.line_cap {
        parts.push(format!("stroke-linecap=\"{}\"", line_cap_str(cap)));
    }
    if let Some(join) = outline.line_join {
        parts.push(format!("stroke-linejoin=\"{}\"", line_join_str(join)));
    }

    FillAttrs {
        attrs: parts.join(" "),
        defs,
    }
}

/// Render arrow-marker `<marker>` definitions plus their referencing
/// attributes. `None` outline or no head/tail ends -> empty result.
#[must_use]
pub fn render_markers(outline: Option<&Outline>, ids: &mut IdGen) -> MarkerResult {
    let Some(outline) = outline else {
        return MarkerResult::default();
    };
    if outline.head_end.is_none() && outline.tail_end.is_none() {
        return MarkerResult::default();
    }

    // Match the spec: stroke color = solid stroke fill, or first
    // gradient stop, default `#000000`.
    let (color, alpha) = match &outline.fill {
        Some(OutlineFill::Solid(s)) => (color_hex(&s.color), s.color.alpha),
        Some(OutlineFill::Gradient(g)) if !g.stops.is_empty() => {
            (color_hex(&g.stops[0].color), g.stops[0].color.alpha)
        }
        _ => ("#000000".to_string(), 1.0),
    };

    let mut defs = String::new();
    let mut start_attr = String::new();
    let mut end_attr = String::new();

    if let Some(head) = outline.head_end {
        if head.ty != ArrowType::None {
            let id = ids.next_id("marker");
            // marker-start uses `orient="auto-start-reverse"` so the marker
            // apex points AWAY from the line interior (PowerPoint headEnd
            // semantics). Default `auto` would align the apex with the
            // outgoing tangent — i.e. INTO the line — which is the wrong
            // direction for an arrowhead. The reverse variant also makes
            // marker direction stable under parent `scale(1, -1)` flips
            // (`flipV=1` connectors): SVG `auto` would invert the
            // user-space tangent and flip the arrow as well, so a
            // `flipV=1` vertical headEnd on slide 6 ended up pointing into
            // the line instead of toward the target box.
            if let Some(def) = build_marker_def(&id, head, &color, alpha, true) {
                defs.push_str(&def);
                start_attr = format!("marker-start=\"url(#{id})\"");
            }
        }
    }
    if let Some(tail) = outline.tail_end {
        if tail.ty != ArrowType::None {
            let id = ids.next_id("marker");
            if let Some(def) = build_marker_def(&id, tail, &color, alpha, false) {
                defs.push_str(&def);
                end_attr = format!("marker-end=\"url(#{id})\"");
            }
        }
    }

    MarkerResult {
        defs,
        start_attr,
        end_attr,
    }
}

fn none_fill() -> FillAttrs {
    FillAttrs {
        attrs: "fill=\"none\"".to_string(),
        defs: String::new(),
    }
}

fn solid_attrs(color: &ResolvedColor) -> FillAttrs {
    let mut attrs = format!("fill=\"{}\"", color_hex(color));
    if color.alpha < 1.0 {
        let _ = write!(attrs, " fill-opacity=\"{}\"", alpha_str(color.alpha));
    }
    FillAttrs {
        attrs,
        defs: String::new(),
    }
}

pub(super) struct GradientRef {
    reference: String,
    defs: String,
}

fn dash_array(style: DashStyle, w: f64) -> Option<String> {
    let pattern: &[u32] = match style {
        DashStyle::Dash => &[4, 3],
        DashStyle::Dot => &[1, 3],
        DashStyle::DashDot => &[4, 3, 1, 3],
        DashStyle::LgDash => &[8, 3],
        DashStyle::LgDashDot => &[8, 3, 1, 3],
        DashStyle::SysDash => &[3, 1],
        DashStyle::SysDot => &[1, 1],
        DashStyle::Solid => return None,
    };
    let mut out = String::new();
    for (i, v) in pattern.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let _ = write!(out, "{}", n(f64::from(*v) * w));
    }
    Some(out)
}

fn line_cap_str(cap: LineCap) -> &'static str {
    match cap {
        LineCap::Butt => "butt",
        LineCap::Round => "round",
        LineCap::Square => "square",
    }
}

fn line_join_str(join: LineJoin) -> &'static str {
    match join {
        LineJoin::Miter => "miter",
        LineJoin::Round => "round",
        LineJoin::Bevel => "bevel",
    }
}

// --- Pattern fill ---

pub(super) struct PatternContent {
    svg: String,
    size: u32,
}

// --- Marker (arrow endpoint) ---

pub(super) const fn arrow_size_px(size: ArrowSize) -> u32 {
    match size {
        ArrowSize::Sm => 5,
        ArrowSize::Med => 8,
        ArrowSize::Lg => 12,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_color::Rgb;
    use slideglance_model::{
        ArrowEndpoint, ArrowSize, ArrowType, DashStyle, GradientFill, GradientStop, GradientType,
        ImageFill, ImageFillTile, ImageFlip, LineCap, LineJoin, NoFill, Outline, OutlineFill,
        PatternFill, SolidFill,
    };
    use slideglance_utils::Emu;

    fn opaque(hex: &str) -> ResolvedColor {
        ResolvedColor::new(Rgb::from_hex(hex).unwrap(), 1.0)
    }
    fn translucent(hex: &str, alpha: f64) -> ResolvedColor {
        ResolvedColor::new(Rgb::from_hex(hex).unwrap(), alpha)
    }

    // --- render_fill_attrs ---

    #[test]
    fn null_fill_is_none() {
        let mut ids = IdGen::new();
        let r = render_fill_attrs(None, &mut ids);
        assert_eq!(r.attrs, "fill=\"none\"");
        assert!(r.defs.is_empty());
        assert_eq!(ids.peek(), 0);
    }

    #[test]
    fn explicit_no_fill() {
        let mut ids = IdGen::new();
        let r = render_fill_attrs(Some(&Fill::None(NoFill {})), &mut ids);
        assert_eq!(r.attrs, "fill=\"none\"");
    }

    #[test]
    fn solid_opaque() {
        let mut ids = IdGen::new();
        let f = Fill::Solid(SolidFill {
            color: opaque("#FF0000"),
        });
        let r = render_fill_attrs(Some(&f), &mut ids);
        assert_eq!(r.attrs, "fill=\"#FF0000\"");
        assert!(r.defs.is_empty());
    }

    #[test]
    fn solid_with_alpha_emits_fill_opacity() {
        let mut ids = IdGen::new();
        let f = Fill::Solid(SolidFill {
            color: translucent("#FF0000", 0.5),
        });
        let r = render_fill_attrs(Some(&f), &mut ids);
        assert!(r.attrs.contains("fill=\"#FF0000\""));
        assert!(r.attrs.contains("fill-opacity=\"0.5\""));
    }

    #[test]
    fn gradient_linear_emits_def_and_url_ref() {
        let mut ids = IdGen::new();
        let f = Fill::Gradient(GradientFill {
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: opaque("#FF0000"),
                },
                GradientStop {
                    position: 1.0,
                    color: opaque("#0000FF"),
                },
            ],
            angle: 90.0,
            gradient_type: GradientType::Linear,
            center_x: None,
            center_y: None,
        });
        let r = render_fill_attrs(Some(&f), &mut ids);
        assert_eq!(r.attrs, "fill=\"url(#grad-0)\"");
        assert!(r.defs.contains("<linearGradient id=\"grad-0\""));
        assert!(r.defs.contains("stop-color=\"#FF0000\""));
        assert!(r.defs.contains("stop-color=\"#0000FF\""));
    }

    #[test]
    fn gradient_radial_uses_radial_element() {
        let mut ids = IdGen::new();
        let f = Fill::Gradient(GradientFill {
            stops: vec![GradientStop {
                position: 0.0,
                color: opaque("#000000"),
            }],
            angle: 0.0,
            gradient_type: GradientType::Radial,
            center_x: Some(0.5),
            center_y: Some(0.5),
        });
        let r = render_fill_attrs(Some(&f), &mut ids);
        assert!(r.defs.starts_with("<radialGradient"));
    }

    #[test]
    fn image_emf_falls_back_to_grey() {
        let mut ids = IdGen::new();
        let f = Fill::Image(ImageFill {
            image_data: "AA".to_string(),
            mime_type: "image/emf".to_string(),
            tile: None,
            src_rect: None,
            stretch: None,
            alpha: 1.0,
        });
        let r = render_fill_attrs(Some(&f), &mut ids);
        assert_eq!(r.attrs, "fill=\"#E0E0E0\"");
        assert!(r.defs.is_empty());
    }

    #[test]
    fn image_stretch_emits_pattern_with_unit_box() {
        let mut ids = IdGen::new();
        let f = Fill::Image(ImageFill {
            image_data: "AA".to_string(),
            mime_type: "image/png".to_string(),
            tile: None,
            src_rect: None,
            stretch: None,
            alpha: 1.0,
        });
        let r = render_fill_attrs(Some(&f), &mut ids);
        assert!(r.defs.contains("<pattern id=\"imgfill-0\""));
        assert!(r.defs.contains("patternContentUnits=\"objectBoundingBox\""));
        assert!(r.defs.contains("data:image/png;base64,AA"));
    }

    #[test]
    fn image_fill_with_alpha_lt_one_emits_fill_opacity() {
        // `<a:blip><a:alphaModFix amt="5000"/>` ⇒ ImageFill.alpha = 0.05.
        // The renderer applies it as `fill-opacity` on the path-level
        // attributes so the same pattern definition stays reusable.
        let mut ids = IdGen::new();
        let f = Fill::Image(ImageFill {
            image_data: "AA".to_string(),
            mime_type: "image/png".to_string(),
            tile: None,
            src_rect: None,
            stretch: None,
            alpha: 0.05,
        });
        let r = render_fill_attrs(Some(&f), &mut ids);
        assert!(
            r.attrs.contains("fill-opacity=\"0.05\""),
            "expected fill-opacity, got: {}",
            r.attrs
        );
        assert!(r.attrs.starts_with("fill=\"url(#imgfill-"));
    }

    #[test]
    fn image_fill_with_alpha_one_does_not_emit_fill_opacity() {
        let mut ids = IdGen::new();
        let f = Fill::Image(ImageFill {
            image_data: "AA".to_string(),
            mime_type: "image/png".to_string(),
            tile: None,
            src_rect: None,
            stretch: None,
            alpha: 1.0,
        });
        let r = render_fill_attrs(Some(&f), &mut ids);
        assert!(!r.attrs.contains("fill-opacity"), "{}", r.attrs);
    }

    #[test]
    fn image_tile_uses_object_bounding_box_and_scale() {
        let mut ids = IdGen::new();
        let f = Fill::Image(ImageFill {
            image_data: "BB".to_string(),
            mime_type: "image/png".to_string(),
            tile: Some(ImageFillTile {
                tx: Emu::new(0),
                ty: Emu::new(0),
                sx: 0.5,
                sy: 0.5,
                flip: ImageFlip::None,
                align: "tl".to_string(),
            }),
            src_rect: None,
            stretch: None,
            alpha: 1.0,
        });
        let r = render_fill_attrs(Some(&f), &mut ids);
        assert!(r.defs.contains("width=\"50%\""));
        assert!(r.defs.contains("height=\"50%\""));
    }

    #[test]
    fn pattern_known_preset_emits_pattern_def() {
        let mut ids = IdGen::new();
        let f = Fill::Pattern(PatternFill {
            preset: "ltHorz".to_string(),
            foreground_color: opaque("#000000"),
            background_color: opaque("#FFFFFF"),
        });
        let r = render_fill_attrs(Some(&f), &mut ids);
        assert!(r.attrs.starts_with("fill=\"url(#patt-0)\""));
        assert!(r.defs.contains("<pattern id=\"patt-0\""));
        assert!(r.defs.contains("<line"));
    }

    #[test]
    fn pattern_unknown_preset_falls_back_to_solid_fg() {
        let mut ids = IdGen::new();
        let f = Fill::Pattern(PatternFill {
            preset: "totallyUnknown".to_string(),
            foreground_color: opaque("#123456"),
            background_color: opaque("#FFFFFF"),
        });
        let r = render_fill_attrs(Some(&f), &mut ids);
        assert!(r.attrs.contains("fill=\"#123456\""));
        assert!(r.defs.is_empty());
    }

    #[test]
    fn pattern_pct_alias_uses_alpha() {
        let mut ids = IdGen::new();
        let f = Fill::Pattern(PatternFill {
            preset: "pct50".to_string(),
            foreground_color: opaque("#000000"),
            background_color: opaque("#FFFFFF"),
        });
        let r = render_fill_attrs(Some(&f), &mut ids);
        assert!(r.defs.contains("opacity=\"0.5\""));
    }

    // --- render_outline_attrs ---

    fn solid_outline() -> Outline {
        Outline {
            width: Emu::new(12_700),
            fill: Some(OutlineFill::Solid(SolidFill {
                color: opaque("#000000"),
            })),
            dash_style: DashStyle::Solid,
            custom_dash: None,
            line_cap: None,
            line_join: None,
            head_end: None,
            tail_end: None,
        }
    }

    #[test]
    fn null_outline_is_stroke_none() {
        let mut ids = IdGen::new();
        let r = render_outline_attrs(None, &mut ids, 1.0);
        assert_eq!(r.attrs, "stroke=\"none\"");
    }

    #[test]
    fn solid_outline_has_width_and_color() {
        let mut ids = IdGen::new();
        let r = render_outline_attrs(Some(&solid_outline()), &mut ids, 1.0);
        // 12,700 EMU = 1 pt = 1.333... px at 96 DPI.
        assert!(r.attrs.starts_with("stroke-width=\"1.3333333333333333\""));
        assert!(r.attrs.contains("stroke=\"#000000\""));
    }

    #[test]
    fn outline_with_alpha_includes_stroke_opacity() {
        let mut ids = IdGen::new();
        let mut o = solid_outline();
        o.fill = Some(OutlineFill::Solid(SolidFill {
            color: translucent("#FF0000", 0.5),
        }));
        let r = render_outline_attrs(Some(&o), &mut ids, 1.0);
        assert!(r.attrs.contains("stroke-opacity=\"0.5\""));
    }

    #[test]
    fn outline_dash_emits_stroke_dasharray() {
        let mut ids = IdGen::new();
        let mut o = solid_outline();
        o.dash_style = DashStyle::Dash;
        let r = render_outline_attrs(Some(&o), &mut ids, 1.0);
        assert!(r.attrs.contains("stroke-dasharray="));
    }

    #[test]
    fn outline_custom_dash_overrides_preset() {
        let mut ids = IdGen::new();
        let mut o = solid_outline();
        // Choose a width / multiplier combination that yields integer pixel
        // values so we can assert the exact serialized form regardless of
        // float precision.
        o.width = Emu::new(9_525); // 1 px @ 96 DPI
        o.custom_dash = Some(vec![2.0, 1.5]);
        let r = render_outline_attrs(Some(&o), &mut ids, 1.0);
        assert!(
            r.attrs.contains("stroke-dasharray=\"2 1.5\""),
            "{}",
            r.attrs
        );
    }

    #[test]
    fn outline_line_cap_and_join_are_emitted() {
        let mut ids = IdGen::new();
        let mut o = solid_outline();
        o.line_cap = Some(LineCap::Round);
        o.line_join = Some(LineJoin::Bevel);
        let r = render_outline_attrs(Some(&o), &mut ids, 1.0);
        assert!(r.attrs.contains("stroke-linecap=\"round\""));
        assert!(r.attrs.contains("stroke-linejoin=\"bevel\""));
    }

    #[test]
    fn outline_no_fill_branch_emits_stroke_none() {
        let mut ids = IdGen::new();
        let mut o = solid_outline();
        o.fill = None;
        let r = render_outline_attrs(Some(&o), &mut ids, 1.0);
        assert!(r.attrs.contains("stroke=\"none\""));
    }

    #[test]
    fn outline_gradient_emits_def_and_url() {
        let mut ids = IdGen::new();
        let mut o = solid_outline();
        o.fill = Some(OutlineFill::Gradient(GradientFill {
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: opaque("#FF0000"),
                },
                GradientStop {
                    position: 1.0,
                    color: opaque("#0000FF"),
                },
            ],
            angle: 0.0,
            gradient_type: GradientType::Linear,
            center_x: None,
            center_y: None,
        }));
        let r = render_outline_attrs(Some(&o), &mut ids, 1.0);
        assert!(r.attrs.contains("stroke=\"url(#grad-0)\""));
        assert!(r.defs.starts_with("<linearGradient"));
    }

    // --- render_markers ---

    #[test]
    fn markers_none_when_no_outline() {
        let mut ids = IdGen::new();
        let r = render_markers(None, &mut ids);
        assert!(r.defs.is_empty());
        assert!(r.start_attr.is_empty());
        assert!(r.end_attr.is_empty());
    }

    #[test]
    fn markers_none_when_no_endpoints() {
        let mut ids = IdGen::new();
        let r = render_markers(Some(&solid_outline()), &mut ids);
        assert!(r.defs.is_empty());
    }

    #[test]
    fn marker_triangle_head_end() {
        let mut ids = IdGen::new();
        let mut o = solid_outline();
        o.head_end = Some(ArrowEndpoint {
            ty: ArrowType::Triangle,
            width: ArrowSize::Med,
            length: ArrowSize::Med,
        });
        let r = render_markers(Some(&o), &mut ids);
        assert!(r.start_attr.starts_with("marker-start=\"url(#marker-0)\""));
        assert!(r.defs.contains("<marker id=\"marker-0\""));
        assert!(r.defs.contains("<path d=\"M 0 0"));
    }

    #[test]
    fn marker_oval_emits_ellipse() {
        let mut ids = IdGen::new();
        let mut o = solid_outline();
        o.tail_end = Some(ArrowEndpoint {
            ty: ArrowType::Oval,
            width: ArrowSize::Sm,
            length: ArrowSize::Lg,
        });
        let r = render_markers(Some(&o), &mut ids);
        assert!(r.end_attr.starts_with("marker-end=\"url(#marker-0)\""));
        assert!(r.defs.contains("<ellipse"));
    }

    #[test]
    fn marker_arrow_uses_stroke_only() {
        let mut ids = IdGen::new();
        let mut o = solid_outline();
        o.tail_end = Some(ArrowEndpoint {
            ty: ArrowType::Arrow,
            width: ArrowSize::Med,
            length: ArrowSize::Med,
        });
        let r = render_markers(Some(&o), &mut ids);
        assert!(r.defs.contains("fill=\"none\""));
        assert!(r.defs.contains("stroke=\"#000000\""));
    }

    #[test]
    fn marker_color_falls_back_to_first_gradient_stop() {
        let mut ids = IdGen::new();
        let mut o = solid_outline();
        o.fill = Some(OutlineFill::Gradient(GradientFill {
            stops: vec![GradientStop {
                position: 0.0,
                color: opaque("#FF0000"),
            }],
            angle: 0.0,
            gradient_type: GradientType::Linear,
            center_x: None,
            center_y: None,
        }));
        o.head_end = Some(ArrowEndpoint {
            ty: ArrowType::Triangle,
            width: ArrowSize::Med,
            length: ArrowSize::Med,
        });
        let r = render_markers(Some(&o), &mut ids);
        // The gradient itself is not minted by render_markers — it only
        // borrows the first stop's color. The marker def references the
        // resolved hex value directly.
        assert!(r.defs.contains("fill=\"#FF0000\""));
    }
}
