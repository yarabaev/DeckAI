//! `render_text_body` — text-mode entry point.
//!
//! Direct port of the text-mode branch of `renderTextBody` in
//! . Path-mode (glyph-
//! outline) rendering, `WordArt` warp curves (text- and path-mode),
//! `spAutofit` height computation, and `normAutofit` shrink-to-fit are
//! all wired up — this function dispatches to the path-mode helpers
//! when a `FontResolver` is supplied and emits `<text>` / `<tspan>`
//! markup otherwise.
//!
//! Vertical text (`vert`, `vert270`) is wrapped in a `<g rotate>` group as
//! the spec does. EA-vertical / Mongolian / `WordArt`-vertical share
//! the `vert` rotation since the renderer treats them as "90° CW" for the
//! group wrapper.

use slideglance_font::{
    wrap_paragraph_with_chain, CjkPlatform, FontMapping, FontResolver, FontStyle,
    ScriptFontContext, TextMeasurer,
};
use slideglance_model::{BodyProperties, TextBody, TextVerticalType, Transform, WrapMode};
use slideglance_utils::{Emu, Pt};

use std::fmt::Write as _;

use crate::geometry::fmt::n;
use crate::slide_context::SlideRenderContext;
use crate::svg_builder::escape_xml_text;

use super::layout::{
    compute_dy, compute_line_natural_height, effective_line_height_pt, get_alignment_info,
    get_default_ascender_ratio, get_default_font_size, get_default_line_height_ratio,
    get_line_spacing, get_paragraph_font_size, resolve_spacing_px_opt, DEFAULT_LINE_SPACING,
    PX_PER_PT,
};
use super::segment::render_segment;
use super::style::build_bullet_style_attrs;

mod helpers;

use helpers::{
    get_line_font_size_segments, highlight_color_for, inject_default_text_color, is_vertical,
    resolve_bullet_text, serialize_highlight_rects, substitute_field_runs, HighlightRect,
};

/// Render a `<p:txBody>` to an SVG `<text>` (or rotated `<g>` wrapper for
/// vertical text). Empty bodies produce an empty string, matching the TS
/// reference.
///
/// `slide` is consulted for `<a:fld>` substitution; `script_fonts` provides
/// CJK theme fallbacks for run-level font resolution. `mapping` /
/// `cjk_platform` configure the `font-family` value list construction.
///
/// `font_resolver`, when `Some`, switches the renderer to **path-mode**:
/// glyph outlines are emitted as `<path>` data via
/// [`super::path_mode::render_text_body_as_path`] and `WordArt` warps go
/// through [`super::warp_path::render_text_body_as_warp_path`]. This is
/// the codepath required for resvg-based PNG rasterization, which lacks
/// `<textPath>` support and cannot reliably resolve system fonts. When
/// `None`, the existing text-mode pipeline runs unchanged. Mirrors the
/// TS dispatch in lines
/// 136-156 (`renderTextBody`).
///
/// **Out of scope this batch:** full justification and tab stops. Inputs
/// that request these still render — they just aren't applied yet.
//
// The function intentionally has many lines because it follows the TS
// reference's single-pass control flow over paragraphs / wrapped lines /
// segments. Splitting it would obscure the side-by-side comparison; the
// helpers it calls are factored out already.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
#[must_use]
pub fn render_text_body(
    raw_text_body: &TextBody,
    transform: &Transform,
    slide: &SlideRenderContext,
    script_fonts: &ScriptFontContext,
    measurer: &dyn TextMeasurer,
    mapping: &FontMapping,
    cjk_platform: CjkPlatform,
    font_resolver: Option<&dyn FontResolver>,
    is_table_cell: bool,
    // Multiplier applied to the run's font-size *before* the font-size
    // attribute is emitted to SVG. Set to `1.0 / sqrt(scale_x * scale_y)`
    // by `render_group` when this body is rendered inside a group whose
    // `chExt != ext` produces a non-identity SVG `transform="scale(...)"`.
    // The intent matches PowerPoint's behaviour: text point sizes are
    // absolute and do *not* scale with the enclosing group transform.
    // Without this counter-scale, a 21pt subtitle inside a 3.7×-scaled
    // group renders at ~78pt visually — the group scale propagates to
    // every child including text. Pass `1.0` for any text body that is
    // *not* inside a scaled group; nested groups multiply this further.
    // See `slide.rs::render_group` for the producer end. Anisotropic
    // scaling (`scale_x != scale_y`) is approximated by the geometric
    // mean — glyph stretching by the residual ratio remains and is a
    // known limitation.
    font_size_correction: f64,
) -> String {
    let body = substitute_field_runs(raw_text_body, slide);
    let body = inject_default_text_color(body);
    let bp = &body.body_properties;

    // WordArt warp: dispatch warp_path in path-mode (glyph outlines along
    // the curve) or warp text-mode (`<textPath>`) when no font resolver
    // is available. Unknown presets return None and fall through to the
    // regular pipeline.
    if bp.prst_tx_warp.is_some() {
        if let Some(resolver) = font_resolver {
            if let Some(warped) = crate::text::warp_path::render_text_body_as_warp_path(
                &body,
                transform,
                script_fonts,
                measurer,
                resolver,
            ) {
                return warped;
            }
        } else if let Some(warped) = crate::text::warp::render_text_body_as_warp(
            &body,
            transform,
            mapping,
            cjk_platform,
            script_fonts,
        ) {
            return warped;
        }
    }

    // Path-mode: glyph outlines as `<path>` data. Required for resvg PNG
    // rasterization. The reference passes the already-substituted body;
    // render_text_body_as_path runs its own substitute (idempotent on
    // substituted bodies), so passing `&body` doubles the work but keeps
    // the call sites uniform.
    if let Some(resolver) = font_resolver {
        return crate::text::render_text_body_as_path(
            &body,
            transform,
            slide,
            script_fonts,
            measurer,
            resolver,
            mapping,
            cjk_platform,
            is_table_cell,
            font_size_correction,
        );
    }

    let original_width = transform.extent_width.to_pixels();
    let original_height = transform.extent_height.to_pixels();
    let dims = resolve_text_dimensions(bp, original_width, original_height);

    let has_text = body
        .paragraphs
        .iter()
        .any(|p| p.runs.iter().any(|r| !r.text.is_empty()));
    if !has_text {
        return String::new();
    }

    let full_text_width = dims.width - dims.margin_left - dims.margin_right;
    let num_col = bp.num_col.max(1);
    let text_width = if num_col > 1 {
        full_text_width / f64::from(num_col)
    } else {
        full_text_width
    };
    let default_font_size = get_default_font_size(&body);
    let should_wrap = !matches!(bp.wrap, WrapMode::None);

    // normAutofit: ignore the stored fontScale and re-fit from 100% — see
    // `path_mode.rs` for rationale. The two dispatchers must stay in sync
    // so text-mode and path-mode produce the same scale for the same shape.
    let mut font_scale = bp.font_scale;
    let ln_spc_reduction = bp.ln_spc_reduction;
    if matches!(bp.auto_fit, slideglance_model::AutoFit::NormAutofit) && should_wrap {
        let available_height = dims.height - dims.margin_top - dims.margin_bottom;
        font_scale = crate::text::autofit::compute_shrink_to_fit_scale(
            &body,
            default_font_size,
            1.0,
            ln_spc_reduction,
            text_width,
            available_height,
            measurer,
        );
    }
    // Group transform compensation. Multiply once after autofit so the
    // shrink-to-fit calculation operates on the deck-authored sizes (it
    // measures against the local-coord frame, which is also already
    // pre-scale; the SVG `<g transform="scale">` from the parent group
    // restores everything to slide-coord at paint time). See the doc on
    // the parameter for the producer end.
    font_scale *= font_size_correction;
    let scaled_default_font_size_pt = default_font_size * font_scale;
    let default_line_height_ratio = get_default_line_height_ratio(&body, measurer);
    let default_ascender_ratio = get_default_ascender_ratio(&body, measurer);
    let default_natural_height_pt = scaled_default_font_size_pt * default_line_height_ratio;

    let mut tspans = String::new();
    let mut is_first_line = true;
    let mut auto_num_counters: std::collections::HashMap<
        (slideglance_model::AutoNumScheme, u8),
        u32,
    > = std::collections::HashMap::new();
    let mut prev_space_after_px = 0.0_f64;
    let last_para_index = body.paragraphs.len().saturating_sub(1);
    // Highlight overlay tracking. Each `<Mark>` / `<a:highlight>` run is
    // measured at the same point we lay out the tspan and pushed as a
    // separate `<rect>` emitted *before* the `<text>` element so the
    // background paints behind the glyphs without depending on
    // `<tspan filter>` (which browsers route through the parent text's
    // bbox and overshoot the run).
    let mut highlight_rects: Vec<HighlightRect> = Vec::new();
    // `y_start` (the text element's `y` attribute) is finalized AFTER the
    // paragraph loop, so during the loop we track baseline as an *offset*
    // from y_start. After the loop, we add y_start to every rect's y.
    let mut current_baseline_offset = 0.0_f64;
    let measure_seg_width = |seg: &slideglance_font::LineSegment| -> f64 {
        let pt = seg.properties.font_size.map_or(default_font_size, Pt::raw) * font_scale;
        let style = FontStyle {
            bold: seg.properties.bold,
            italic: seg.properties.italic,
        };
        // Pass the same `font-family` chain string that the SVG `<text>`
        // attribute will carry — browser-backed measurers (the wasm canvas
        // measurer used by slideglance/viewer) measure off the chain, so
        // omitting it here would let the rect's x drift relative to where
        // the browser actually places the glyphs. Native (OpenType) backends
        // ignore the chain via the trait's default impl. See KDD-15 for the
        // motivation.
        let pair = [
            seg.properties.font_family.as_deref(),
            seg.properties.font_family_ea.as_deref(),
        ];
        let chain =
            super::font_family::build_font_family_value(&pair, mapping, cjk_platform, script_fonts);
        measurer.measure_text_width_with_chain(
            &seg.text,
            pt,
            style,
            seg.properties.font_family.as_deref(),
            seg.properties.font_family_ea.as_deref(),
            chain.as_deref(),
        )
    };
    // Translate `align.x_pos` + `align.anchor` + the first segment's
    // measured width into the actual SVG x where the line's first glyph
    // starts. Subsequent segments flow naturally after, regardless of the
    // anchor — that's how the browser positions tspans without their own x.
    let line_start_x = |anchor: &str, x_pos: f64, first_seg_w: f64| -> f64 {
        match anchor {
            "middle" => x_pos - first_seg_w / 2.0,
            "end" => x_pos - first_seg_w,
            _ => x_pos,
        }
    };

    for (para_idx, para) in body.paragraphs.iter().enumerate() {
        let para_margin_left = para.properties.margin_left.map_or(0.0, Emu::to_pixels);
        let para_indent = para.properties.indent.map_or(0.0, Emu::to_pixels);
        let text_start_x = dims.margin_left + para_margin_left;
        let bullet_x = text_start_x + para_indent;
        let effective_text_width = text_width - para_margin_left;
        let bullet_text = resolve_bullet_text(&para.properties, &mut auto_num_counters);
        let align = get_alignment_info(
            para.properties.alignment,
            text_start_x,
            effective_text_width,
            dims.width,
            dims.margin_right,
        );
        let para_font_size_pt = get_paragraph_font_size(para, default_font_size) * font_scale;
        let is_first_para = para_idx == 0;
        let is_last_para = para_idx == last_para_index;
        let space_before_px = if is_first_para && !bp.spc_first_last_para {
            0.0
        } else {
            resolve_spacing_px_opt(para.properties.space_before, para_font_size_pt)
        };
        let paragraph_gap_px = prev_space_after_px.max(space_before_px);

        let para_has_text = para.runs.iter().any(|r| !r.text.is_empty());
        if para.runs.is_empty() || !para_has_text {
            let empty_para_height_pt = if para_font_size_pt > 0.0 {
                para_font_size_pt
            } else {
                default_natural_height_pt
            };
            // Empty paragraph (no visible runs) — use the natural-height
            // path regardless of `lnSpc` since there's no font size to
            // multiply by.
            let line_height_pt = effective_line_height_pt(
                empty_para_height_pt,
                empty_para_height_pt,
                DEFAULT_LINE_SPACING,
                false,
                bp.compat_ln_spc,
            );
            let dy = compute_dy(is_first_line, line_height_pt, paragraph_gap_px);
            if !is_first_line {
                current_baseline_offset += line_height_pt * PX_PER_PT + paragraph_gap_px;
            }
            let _ = write!(
                tspans,
                "<tspan x=\"{}\" dy=\"{dy}\" text-anchor=\"{}\"> </tspan>",
                n(align.x_pos),
                align.anchor
            );
            is_first_line = false;
            prev_space_after_px =
                resolve_spacing_px_opt(para.properties.space_after, para_font_size_pt);
            continue;
        }

        if should_wrap {
            let wrapped = wrap_paragraph_with_chain(
                para,
                effective_text_width,
                scaled_default_font_size_pt,
                font_scale,
                measurer,
                &|props| {
                    let pair = [
                        props.font_family.as_deref(),
                        props.font_family_ea.as_deref(),
                    ];
                    super::font_family::build_font_family_value(
                        &pair,
                        mapping,
                        cjk_platform,
                        script_fonts,
                    )
                },
            );
            for (line_idx, line) in wrapped.iter().enumerate() {
                let line_gap_px = if line_idx == 0 { paragraph_gap_px } else { 0.0 };
                if line.segments.is_empty() {
                    let line_height_pt = effective_line_height_pt(
                        scaled_default_font_size_pt,
                        default_natural_height_pt,
                        get_line_spacing(para, ln_spc_reduction),
                        para.properties.line_spacing.is_some(),
                        bp.compat_ln_spc,
                    );
                    let dy = compute_dy(is_first_line, line_height_pt, line_gap_px);
                    if !is_first_line {
                        current_baseline_offset += line_height_pt * PX_PER_PT + line_gap_px;
                    }
                    let _ = write!(
                        tspans,
                        "<tspan x=\"{}\" dy=\"{dy}\" text-anchor=\"{}\"> </tspan>",
                        n(align.x_pos),
                        align.anchor
                    );
                    is_first_line = false;
                    continue;
                }

                if line_idx == 0 {
                    if let Some(bullet_text) = &bullet_text {
                        let line_font_size =
                            get_line_font_size_segments(line, default_font_size) * font_scale;
                        let line_natural_height_pt = compute_line_natural_height(
                            &line.segments,
                            default_font_size,
                            font_scale,
                            measurer,
                        );
                        let line_height_pt = effective_line_height_pt(
                            line_font_size,
                            line_natural_height_pt,
                            get_line_spacing(para, ln_spc_reduction),
                            para.properties.line_spacing.is_some(),
                            bp.compat_ln_spc,
                        );
                        let dy = compute_dy(is_first_line, line_height_pt, paragraph_gap_px);
                        if !is_first_line {
                            current_baseline_offset +=
                                line_height_pt * PX_PER_PT + paragraph_gap_px;
                        }
                        let bullet_styles =
                            build_bullet_style_attrs(&para.properties, line_font_size, font_scale);
                        let space = if bullet_styles.is_empty() { "" } else { " " };
                        let _ = write!(
                            tspans,
                            "<tspan x=\"{}\" dy=\"{dy}\" text-anchor=\"start\"{space}{bullet_styles}>{}</tspan>",
                            n(bullet_x),
                            escape_xml_text(bullet_text)
                        );
                        // Pre-measure for highlight rect emission. Segments
                        // after the bullet flow from align.x_pos (left edge
                        // of the body column) regardless of bullet width.
                        let line_widths: Vec<f64> =
                            line.segments.iter().map(measure_seg_width).collect();
                        let first_w = line_widths.first().copied().unwrap_or(0.0);
                        let mut current_x = line_start_x(align.anchor, align.x_pos, first_w);
                        for (seg_idx, seg) in line.segments.iter().enumerate() {
                            let seg_w = line_widths[seg_idx];
                            if let Some((color, alpha)) = highlight_color_for(&seg.properties) {
                                let pt =
                                    seg.properties.font_size.map_or(default_font_size, Pt::raw)
                                        * font_scale;
                                let font_size_px = pt * PX_PER_PT;
                                highlight_rects.push(HighlightRect {
                                    x: current_x,
                                    y: current_baseline_offset - font_size_px * 0.85,
                                    w: seg_w,
                                    h: font_size_px * 1.1,
                                    color_hex: color,
                                    alpha,
                                });
                            }
                            let prefix = if seg_idx == 0 {
                                format!(
                                    "x=\"{}\" text-anchor=\"{}\" ",
                                    n(align.x_pos),
                                    align.anchor
                                )
                            } else {
                                String::new()
                            };
                            tspans.push_str(&render_segment(
                                &seg.text,
                                &seg.properties,
                                font_scale,
                                &prefix,
                                mapping,
                                cjk_platform,
                                script_fonts,
                            ));
                            current_x += seg_w;
                        }
                        is_first_line = false;
                        continue;
                    }
                }

                // Hoist line-height computation out of the per-segment branch
                // so we can advance `current_baseline_offset` (and therefore
                // emit highlight rects at the correct y) BEFORE the loop.
                let line_natural_height_pt = compute_line_natural_height(
                    &line.segments,
                    default_font_size,
                    font_scale,
                    measurer,
                );
                let line_font_size_pt =
                    get_line_font_size_segments(line, default_font_size) * font_scale;
                let line_height_pt = effective_line_height_pt(
                    line_font_size_pt,
                    line_natural_height_pt,
                    get_line_spacing(para, ln_spc_reduction),
                    para.properties.line_spacing.is_some(),
                    bp.compat_ln_spc,
                );
                let dy_str = compute_dy(is_first_line, line_height_pt, line_gap_px);
                if !is_first_line {
                    current_baseline_offset += line_height_pt * PX_PER_PT + line_gap_px;
                }
                let line_widths: Vec<f64> = line.segments.iter().map(measure_seg_width).collect();
                let first_w = line_widths.first().copied().unwrap_or(0.0);
                let mut current_x = line_start_x(align.anchor, align.x_pos, first_w);
                for (seg_idx, seg) in line.segments.iter().enumerate() {
                    let seg_w = line_widths[seg_idx];
                    if let Some((color, alpha)) = highlight_color_for(&seg.properties) {
                        let pt = seg.properties.font_size.map_or(default_font_size, Pt::raw)
                            * font_scale;
                        let font_size_px = pt * PX_PER_PT;
                        highlight_rects.push(HighlightRect {
                            x: current_x,
                            y: current_baseline_offset - font_size_px * 0.85,
                            w: seg_w,
                            h: font_size_px * 1.1,
                            color_hex: color,
                            alpha,
                        });
                    }
                    let prefix = if seg_idx == 0 {
                        format!(
                            "x=\"{}\" dy=\"{dy_str}\" text-anchor=\"{}\" ",
                            n(align.x_pos),
                            align.anchor
                        )
                    } else {
                        String::new()
                    };
                    tspans.push_str(&render_segment(
                        &seg.text,
                        &seg.properties,
                        font_scale,
                        &prefix,
                        mapping,
                        cjk_platform,
                        script_fonts,
                    ));
                    current_x += seg_w;
                }
                is_first_line = false;
            }
        } else {
            // wrap == "none": emit each non-empty run on the same line.
            let mut first_run_rendered = false;
            if let Some(bullet_text) = &bullet_text {
                let first_run = para.runs.iter().find(|r| !r.text.is_empty());
                let font_size = first_run
                    .and_then(|r| r.properties.font_size.map(slideglance_utils::Pt::raw))
                    .unwrap_or(default_font_size)
                    * font_scale;
                let natural_height_pt = compute_line_natural_height(
                    &para.runs,
                    default_font_size,
                    font_scale,
                    measurer,
                );
                let line_height_pt = effective_line_height_pt(
                    font_size,
                    natural_height_pt,
                    get_line_spacing(para, ln_spc_reduction),
                    para.properties.line_spacing.is_some(),
                    bp.compat_ln_spc,
                );
                let dy = compute_dy(is_first_line, line_height_pt, paragraph_gap_px);
                if !is_first_line {
                    current_baseline_offset += line_height_pt * PX_PER_PT + paragraph_gap_px;
                }
                let bullet_styles =
                    build_bullet_style_attrs(&para.properties, font_size, font_scale);
                let space = if bullet_styles.is_empty() { "" } else { " " };
                let _ = write!(
                    tspans,
                    "<tspan x=\"{}\" dy=\"{dy}\" text-anchor=\"start\"{space}{bullet_styles}>{}</tspan>",
                    n(bullet_x),
                    escape_xml_text(bullet_text)
                );
            }

            // No-wrap path also needs the line baseline advanced when the
            // first run emits its own dy (no bullet branch). To keep the
            // rect math right, measure every run on this single line first
            // so the highlight rect emission below has the same widths
            // we use for x accumulation.
            let nowrap_run_widths: Vec<f64> = para
                .runs
                .iter()
                .map(|r| {
                    let pt = r.properties.font_size.map_or(default_font_size, Pt::raw) * font_scale;
                    let style = FontStyle {
                        bold: r.properties.bold,
                        italic: r.properties.italic,
                    };
                    measurer.measure_text_width(
                        &r.text,
                        pt,
                        style,
                        r.properties.font_family.as_deref(),
                        r.properties.font_family_ea.as_deref(),
                    )
                })
                .collect();
            let nowrap_first_w = para
                .runs
                .iter()
                .position(|r| !r.text.is_empty())
                .map_or(0.0, |i| nowrap_run_widths[i]);
            let mut nowrap_current_x = line_start_x(align.anchor, align.x_pos, nowrap_first_w);
            // The baseline-y advance for the no-bullet branch happens
            // inside the loop below at first-run emission time. Track
            // whether we already advanced so we don't double-count.
            let mut nowrap_baseline_advanced = bullet_text.is_some();
            for (run_idx, run) in para.runs.iter().enumerate() {
                if run.text.is_empty() {
                    continue;
                }
                let run_w = nowrap_run_widths[run_idx];
                let prefix = if first_run_rendered {
                    String::new()
                } else if bullet_text.is_some() {
                    format!("x=\"{}\" text-anchor=\"{}\" ", n(align.x_pos), align.anchor)
                } else {
                    let natural_height_pt = compute_line_natural_height(
                        &para.runs,
                        default_font_size,
                        font_scale,
                        measurer,
                    );
                    let first_run_font_size = para
                        .runs
                        .iter()
                        .find(|r| !r.text.is_empty())
                        .and_then(|r| r.properties.font_size.map(slideglance_utils::Pt::raw))
                        .unwrap_or(default_font_size)
                        * font_scale;
                    let line_height_pt = effective_line_height_pt(
                        first_run_font_size,
                        natural_height_pt,
                        get_line_spacing(para, ln_spc_reduction),
                        para.properties.line_spacing.is_some(),
                        bp.compat_ln_spc,
                    );
                    let dy = compute_dy(is_first_line, line_height_pt, paragraph_gap_px);
                    if !is_first_line && !nowrap_baseline_advanced {
                        current_baseline_offset += line_height_pt * PX_PER_PT + paragraph_gap_px;
                        nowrap_baseline_advanced = true;
                    }
                    format!(
                        "x=\"{}\" dy=\"{dy}\" text-anchor=\"{}\" ",
                        n(align.x_pos),
                        align.anchor
                    )
                };
                if let Some((color, alpha)) = highlight_color_for(&run.properties) {
                    let pt =
                        run.properties.font_size.map_or(default_font_size, Pt::raw) * font_scale;
                    let font_size_px = pt * PX_PER_PT;
                    highlight_rects.push(HighlightRect {
                        x: nowrap_current_x,
                        y: current_baseline_offset - font_size_px * 0.85,
                        w: run_w,
                        h: font_size_px * 1.1,
                        color_hex: color,
                        alpha,
                    });
                }
                tspans.push_str(&render_segment(
                    &run.text,
                    &run.properties,
                    font_scale,
                    &prefix,
                    mapping,
                    cjk_platform,
                    script_fonts,
                ));
                nowrap_current_x += run_w;
                first_run_rendered = true;
            }
            // Suppress unused-warning for run_idx when no runs were highlighted.
            let _ = nowrap_baseline_advanced;
            is_first_line = false;
        }

        prev_space_after_px = if is_last_para && !bp.spc_first_last_para {
            0.0
        } else {
            resolve_spacing_px_opt(para.properties.space_after, para_font_size_pt)
        };
    }

    // Vertical anchor: top / center / bottom. We approximate the total
    // height with `compute_line_natural_height` over each paragraph; an
    // exact `estimate_text_height` (which calls `wrap_paragraph` to count
    // lines) lands when `normAutofit` does, since both share the routine.
    let mut y_start = dims.margin_top;
    let total_text_height = estimate_total_height(
        &body,
        default_font_size,
        should_wrap,
        text_width,
        ln_spc_reduction,
        font_scale,
        measurer,
    );
    if matches!(bp.anchor, slideglance_model::VerticalAnchor::Ctr) {
        y_start = dims.margin_top.max((dims.height - total_text_height) / 2.0);
    } else if matches!(bp.anchor, slideglance_model::VerticalAnchor::B) {
        y_start = dims
            .margin_top
            .max(dims.height - total_text_height - dims.margin_bottom);
    }
    let first_para_font_size_pt = body.paragraphs.first().map_or(default_font_size, |p| {
        get_paragraph_font_size(p, default_font_size)
    }) * font_scale;
    let first_line_baseline_offset_pt = first_para_font_size_pt * default_ascender_ratio;
    // Mirror path-mode: top-anchored shapes get full leading delta, all
    // others (cells regardless of anchor, ctr / bottom shapes) get half
    // — leading is split symmetrically when the block is centered.
    let anchor_top = matches!(bp.anchor, slideglance_model::VerticalAnchor::T);
    let leading_coefficient = if is_table_cell {
        0.5
    } else if anchor_top {
        1.0
    } else {
        0.5
    };
    let first_line_spacing = body.paragraphs.first().map_or(DEFAULT_LINE_SPACING, |p| {
        get_line_spacing(p, ln_spc_reduction)
    });
    let extra_first_line_leading_pt =
        (first_line_spacing - 1.0) * first_para_font_size_pt * leading_coefficient;
    let legacy_y_inc = (first_line_baseline_offset_pt + extra_first_line_leading_pt) * PX_PER_PT;
    // Mirror path_mode.rs: blend legacy + line-height baseline at α=0.5
    // for top-anchored shapes with lnSpc > 100 %. See path_mode.rs for
    // grid-search rationale (132-slide audit, sum diff -0.236 pp).
    // text-mode lacks the is_table_cell flag so we apply the regime
    // unconditionally on top-anchored shapes.
    let alpha_first_line: f64 = 0.5;
    y_start += if alpha_first_line != 0.0 && anchor_top && first_line_spacing > 1.0 {
        let lh_y_inc =
            first_para_font_size_pt * default_line_height_ratio * first_line_spacing * PX_PER_PT;
        legacy_y_inc * (1.0 - alpha_first_line) + lh_y_inc * alpha_first_line
    } else {
        legacy_y_inc
    };

    // Translate the in-loop baseline-offset coordinates into final user
    // space by adding the text element's `y` start. Doing this *after* the
    // loop avoids needing y_start ahead of time (it's vertical-anchor
    // dependent and computed from the wrapped layout above). Each rect
    // sits behind the glyphs of a single highlighted run, exactly the box
    // the path-mode renderer would draw — see
    // `text/path_mode/segment.rs::emit_segment` for the parallel logic.
    for r in &mut highlight_rects {
        r.y += y_start;
    }
    let highlight_overlay = serialize_highlight_rects(&highlight_rects);
    // `font-kerning="none"` + `text-rendering="geometricPrecision"` make
    // the browser drop its own kerning-pair adjustments and sub-pixel
    // snapping, two of the biggest sources of "looks-different-than-PNG"
    // drift between path-mode (resvg / our PNG output) and text-mode
    // (browser SVG rendering). With kerning off, the wrap positions
    // and run-end x's that the renderer computed line up with what the
    // browser places — even when the local font has metric quirks
    // resvg's measurer wouldn't see. Cheap (~30 bytes per <text>) and
    // backwards compatible — viewers that ignore the attributes get the
    // previous behaviour.
    let text_element = format!(
        "{highlight_overlay}<text x=\"0\" y=\"{}\" xml:space=\"preserve\" font-kerning=\"none\" text-rendering=\"geometricPrecision\">{tspans}</text>",
        n(y_start)
    );

    if is_vertical(bp.vert) {
        format!(
            "<g transform=\"translate({}, 0) rotate(90)\">{text_element}</g>",
            n(original_width)
        )
    } else if matches!(bp.vert, TextVerticalType::Vert270) {
        format!(
            "<g transform=\"translate(0, {}) rotate(-90)\">{text_element}</g>",
            n(original_height)
        )
    } else {
        text_element
    }
}

/// Pixel-space layout box for a text body.
///
/// `width` / `height` are the text frame's outer dimensions (already
/// swapped for vertical text); the four margins are the inner padding
/// in pixels. Computed by [`resolve_text_dimensions`] and exposed to
/// the autofit module so it can compute the same dimensions without
/// duplicating the vertical-text axis swap.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Dimensions {
    pub width: f64,
    pub height: f64,
    pub margin_left: f64,
    pub margin_right: f64,
    pub margin_top: f64,
    pub margin_bottom: f64,
}

pub(crate) fn resolve_text_dimensions(bp: &BodyProperties, w: f64, h: f64) -> Dimensions {
    let to_px = |e: Emu| e.to_pixels();
    if is_vertical(bp.vert) {
        return Dimensions {
            width: h,
            height: w,
            margin_left: to_px(bp.margin_top),
            margin_right: to_px(bp.margin_bottom),
            margin_top: to_px(bp.margin_right),
            margin_bottom: to_px(bp.margin_left),
        };
    }
    if matches!(bp.vert, TextVerticalType::Vert270) {
        return Dimensions {
            width: h,
            height: w,
            margin_left: to_px(bp.margin_bottom),
            margin_right: to_px(bp.margin_top),
            margin_top: to_px(bp.margin_left),
            margin_bottom: to_px(bp.margin_right),
        };
    }
    Dimensions {
        width: w,
        height: h,
        margin_left: to_px(bp.margin_left),
        margin_right: to_px(bp.margin_right),
        margin_top: to_px(bp.margin_top),
        margin_bottom: to_px(bp.margin_bottom),
    }
}

pub(crate) fn estimate_total_height(
    body: &TextBody,
    default_font_size: f64,
    should_wrap: bool,
    text_width: f64,
    ln_spc_reduction: f64,
    font_scale: f64,
    measurer: &dyn TextMeasurer,
) -> f64 {
    let default_ratio = get_default_line_height_ratio(body, measurer);
    let scaled_default_for_wrap = default_font_size * font_scale;
    let mut total = 0.0_f64;
    let mut prev_space_after_px = 0.0_f64;
    for (idx, para) in body.paragraphs.iter().enumerate() {
        let line_spacing = get_line_spacing(para, ln_spc_reduction);
        let is_empty = !para.runs.iter().any(|r| !r.text.is_empty());
        let natural_height_pt = if is_empty {
            para.end_para_run_properties
                .as_ref()
                .and_then(|p| p.font_size.map(|s| s.raw() * font_scale * default_ratio))
                .unwrap_or_else(|| {
                    compute_line_natural_height(&para.runs, default_font_size, font_scale, measurer)
                })
        } else {
            compute_line_natural_height(&para.runs, default_font_size, font_scale, measurer)
        };
        let safe_height_pt = if natural_height_pt > 0.0 {
            natural_height_pt
        } else {
            default_font_size * font_scale * default_ratio
        };
        let line_height = safe_height_pt * PX_PER_PT * line_spacing;

        let line_count =
            if should_wrap && !para.runs.is_empty() && para.runs.iter().any(|r| !r.text.is_empty())
            {
                // estimate_total_height does not have mapping/cjk_platform/
                // script_fonts in scope — chain is irrelevant for line counting,
                // so we use the chain-less wrap entry point. The rendering pass
                // above (line ~235) uses wrap_paragraph_with_chain.
                let lines = slideglance_font::wrap_paragraph(
                    para,
                    text_width,
                    scaled_default_for_wrap,
                    font_scale,
                    measurer,
                );
                lines.len()
            } else {
                1
            };
        total += f64::from(u32::try_from(line_count).unwrap_or(1)) * line_height;

        if idx > 0 {
            let para_font_size_pt = get_paragraph_font_size(para, default_font_size) * font_scale;
            let space_before_px =
                resolve_spacing_px_opt(para.properties.space_before, para_font_size_pt);
            total += prev_space_after_px.max(space_before_px);
        }
        let para_font_size_after_pt = get_paragraph_font_size(para, default_font_size) * font_scale;
        prev_space_after_px =
            resolve_spacing_px_opt(para.properties.space_after, para_font_size_after_pt);
    }
    total
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use slideglance_color::{ResolvedColor, Rgb};
    use slideglance_font::{FontMapping, HeuristicTextMeasurer};
    use slideglance_model::{
        BodyProperties, BulletType, Paragraph, ParagraphAlignment, ParagraphProperties,
        RunProperties, TextRun, VerticalAnchor, WrapMode,
    };
    use slideglance_utils::Pt;

    fn empty_body() -> TextBody {
        TextBody {
            default_text_color: None,
            paragraphs: vec![],
            body_properties: BodyProperties {
                anchor: VerticalAnchor::T,
                margin_left: Emu::new(0),
                margin_right: Emu::new(0),
                margin_top: Emu::new(0),
                margin_bottom: Emu::new(0),
                wrap: WrapMode::Square,
                auto_fit: slideglance_model::AutoFit::NoAutofit,
                font_scale: 1.0,
                ln_spc_reduction: 0.0,
                num_col: 1,
                vert: TextVerticalType::Horz,
                spc_first_last_para: false,
                compat_ln_spc: false,
                prst_tx_warp: None,
            },
        }
    }

    fn run(text: &str) -> TextRun {
        TextRun {
            text: text.to_string(),
            properties: RunProperties::default(),
            field_type: None,
        }
    }

    fn paragraph(props: ParagraphProperties, runs: Vec<TextRun>) -> Paragraph {
        Paragraph {
            runs,
            properties: props,
            end_para_run_properties: None,
        }
    }

    fn xfrm(w_emu: i64, h_emu: i64) -> Transform {
        Transform {
            offset_x: Emu::new(0),
            offset_y: Emu::new(0),
            extent_width: Emu::new(w_emu),
            extent_height: Emu::new(h_emu),
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
        }
    }

    fn render(body: &TextBody, transform: &Transform) -> String {
        let slide = SlideRenderContext::new(1);
        let script_fonts = ScriptFontContext::empty();
        let measurer = HeuristicTextMeasurer;
        let mapping = FontMapping::new();
        render_text_body(
            body,
            transform,
            &slide,
            &script_fonts,
            &measurer,
            &mapping,
            CjkPlatform::Other,
            None,
            false,
            1.0,
        )
    }

    #[test]
    fn empty_body_returns_empty_string() {
        let out = render(&empty_body(), &xfrm(914_400, 914_400));
        assert!(out.is_empty());
    }

    #[test]
    fn body_with_only_blank_paragraphs_returns_empty() {
        let mut body = empty_body();
        body.paragraphs = vec![paragraph(ParagraphProperties::default(), vec![run("")])];
        let out = render(&body, &xfrm(914_400, 914_400));
        assert!(out.is_empty());
    }

    #[test]
    fn single_run_produces_text_with_one_tspan() {
        let mut body = empty_body();
        body.paragraphs = vec![paragraph(
            ParagraphProperties::default(),
            vec![run("Hello")],
        )];
        let out = render(&body, &xfrm(9_144_000, 5_143_500));
        assert!(out.starts_with("<text "));
        assert!(out.contains(">Hello</tspan>"));
        assert!(out.ends_with("</text>"));
    }

    #[test]
    fn vertical_text_wraps_in_rotated_group() {
        let mut body = empty_body();
        body.body_properties.vert = TextVerticalType::Vert;
        body.paragraphs = vec![paragraph(ParagraphProperties::default(), vec![run("Hi")])];
        let out = render(&body, &xfrm(914_400, 1_828_800));
        assert!(out.starts_with("<g transform=\"translate("));
        assert!(out.contains("rotate(90)"));
        assert!(out.ends_with("</g>"));
    }

    #[test]
    fn vert270_uses_negative_rotation() {
        let mut body = empty_body();
        body.body_properties.vert = TextVerticalType::Vert270;
        body.paragraphs = vec![paragraph(ParagraphProperties::default(), vec![run("Hi")])];
        let out = render(&body, &xfrm(914_400, 1_828_800));
        assert!(out.contains("rotate(-90)"));
    }

    #[test]
    fn alignment_center_emits_middle_anchor() {
        let mut props = ParagraphProperties::default();
        props.alignment = Some(ParagraphAlignment::Ctr);
        let mut body = empty_body();
        body.paragraphs = vec![paragraph(props, vec![run("Hi")])];
        let out = render(&body, &xfrm(9_144_000, 5_143_500));
        assert!(out.contains("text-anchor=\"middle\""));
    }

    #[test]
    fn auto_num_bullet_increments_per_paragraph() {
        let mut bullet_props = ParagraphProperties::default();
        bullet_props.bullet = Some(BulletType::AutoNum {
            scheme: slideglance_model::AutoNumScheme::ArabicPeriod,
            start_at: 1,
        });
        let mut body = empty_body();
        body.paragraphs = vec![
            paragraph(bullet_props.clone(), vec![run("first")]),
            paragraph(bullet_props, vec![run("second")]),
        ];
        let out = render(&body, &xfrm(9_144_000, 5_143_500));
        assert!(out.contains(">1.</tspan>"));
        assert!(out.contains(">2.</tspan>"));
    }

    #[test]
    fn char_bullet_appears_before_text() {
        let mut props = ParagraphProperties::default();
        props.bullet = Some(BulletType::Char {
            char: "•".to_string(),
        });
        let mut body = empty_body();
        body.paragraphs = vec![paragraph(props, vec![run("Hello")])];
        let out = render(&body, &xfrm(9_144_000, 5_143_500));
        let bullet_idx = out.find('•').expect("bullet in output");
        let hello_idx = out.find("Hello").expect("text in output");
        assert!(bullet_idx < hello_idx);
    }

    #[test]
    fn field_run_substitutes_slide_number() {
        let mut body = empty_body();
        let mut field_run = TextRun {
            text: "<#>".to_string(),
            properties: RunProperties::default(),
            field_type: Some("slidenum".to_string()),
        };
        field_run.properties.color = Some(ResolvedColor::new(Rgb::new(0, 0, 0), 1.0));
        body.paragraphs = vec![paragraph(ParagraphProperties::default(), vec![field_run])];
        let slide = SlideRenderContext::new(7);
        let out = render_text_body(
            &body,
            &xfrm(9_144_000, 5_143_500),
            &slide,
            &ScriptFontContext::empty(),
            &HeuristicTextMeasurer,
            &FontMapping::new(),
            CjkPlatform::Other,
            None,
            false,
            1.0,
        );
        assert!(out.contains(">7</tspan>"));
        assert!(!out.contains("&lt;#&gt;"));
    }

    #[test]
    fn multiple_runs_concatenate_into_one_paragraph() {
        let mut body = empty_body();
        let mut bold = run("World");
        bold.properties.bold = true;
        body.paragraphs = vec![paragraph(
            ParagraphProperties::default(),
            vec![run("Hello "), bold],
        )];
        let out = render(&body, &xfrm(9_144_000, 5_143_500));
        assert!(out.contains("Hello"));
        assert!(out.contains("font-weight=\"bold\""));
    }

    #[test]
    fn font_size_propagates_to_tspan() {
        let mut body = empty_body();
        let mut r = run("X");
        r.properties.font_size = Some(Pt::new(24.0));
        body.paragraphs = vec![paragraph(ParagraphProperties::default(), vec![r])];
        let out = render(&body, &xfrm(9_144_000, 5_143_500));
        assert!(out.contains("font-size=\"24pt\""));
    }

    #[test]
    fn vertical_anchor_centered_shifts_y_start() {
        let mut body = empty_body();
        body.body_properties.anchor = VerticalAnchor::Ctr;
        body.paragraphs = vec![paragraph(ParagraphProperties::default(), vec![run("Hi")])];
        let top = render(&body, &xfrm(9_144_000, 5_143_500));
        body.body_properties.anchor = VerticalAnchor::T;
        let center = render(&body, &xfrm(9_144_000, 5_143_500));
        // The two outputs should differ in their `<text x="0" y="...">` value.
        assert_ne!(top, center);
    }

    #[test]
    fn empty_paragraph_emits_nbsp_tspan() {
        let mut body = empty_body();
        body.paragraphs = vec![
            paragraph(ParagraphProperties::default(), vec![run("Hello")]),
            paragraph(ParagraphProperties::default(), vec![]),
            paragraph(ParagraphProperties::default(), vec![run("World")]),
        ];
        let out = render(&body, &xfrm(9_144_000, 5_143_500));
        // Each empty paragraph emits a placeholder tspan with a single space.
        assert!(out.matches("> </tspan>").count() >= 1);
    }
}
