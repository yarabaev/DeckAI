//! Group element rendering.
//!
//! Extracted from `slide.rs`. Recursively dispatches back into
//! `super::render_element` for each child, scaled by the group's
//! transform compensation factor.

use std::fmt::Write as _;

use slideglance_font::{CjkPlatform, FontMapping, FontResolver, ScriptFontContext, TextMeasurer};
use slideglance_model::GroupElement;

use crate::error::RendererError;
use crate::geometry::fmt::n;
use crate::id_gen::IdGen;
use crate::render_result::RenderResult;
use crate::slide_context::SlideRenderContext;
use crate::transform::build_object_name_attr;

use super::{element_hidden, render_element};

/// Render a `<p:grpSp>` group element. Recursively dispatches each
/// child back through [`super::render_element`], applying the
/// group's transform stack (translate / rotate / flip / scale) plus
/// the inherited font-size correction so descendant `<text>` runs
/// don't get visually rescaled by the group's `<g transform>`.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_group(
    group: &GroupElement,
    ids: &mut IdGen,
    slide: &SlideRenderContext,
    script_fonts: &ScriptFontContext,
    measurer: &dyn TextMeasurer,
    mapping: &FontMapping,
    cjk_platform: CjkPlatform,
    font_resolver: Option<&dyn FontResolver>,
    // Cumulative correction inherited from outer groups; this group
    // multiplies it by `1 / sqrt(scale_x * scale_y)` and forwards the
    // result to its children. See note below for the rationale.
    parent_font_size_correction: f64,
) -> Result<RenderResult, RendererError> {
    let x = group.transform.offset_x.to_pixels();
    let y = group.transform.offset_y.to_pixels();
    let w = group.transform.extent_width.to_pixels();
    let h = group.transform.extent_height.to_pixels();
    let ch_w = group.child_transform.extent_width.to_pixels();
    let ch_h = group.child_transform.extent_height.to_pixels();
    let ch_x = group.child_transform.offset_x.to_pixels();
    let ch_y = group.child_transform.offset_y.to_pixels();

    let scale_x = if ch_w == 0.0 { 1.0 } else { w / ch_w };
    let scale_y = if ch_h == 0.0 { 1.0 } else { h / ch_h };

    // Group transform text-size compensation — intentional divergence
    // from the spec's `renderGroup` (svg-renderer.ts).
    //
    // This group's SVG `<g transform="scale(scale_x, scale_y)">` will
    // expand every descendant's coordinate space, including the
    // `font-size` attribute on `<text>` nodes (font-size is a length in
    // user space and SVG scales lengths). PowerPoint, in contrast,
    // treats font point sizes as ABSOLUTE — they do not scale with a
    // group's chExt → ext ratio. PowerPoint's own native rendering of
    // the Slidesgo "Blue and Green Business Infographic" template (and
    // the macOS Quick Look renderer + the deck's embedded
    // `docProps/thumbnail.jpeg`) all show the 21pt subtitle at 21pt,
    // even though the enclosing group has a ~3.7× scale.
    //
    // To match that behaviour without rewriting every layout function
    // to track absolute vs. local-coordinate sizing, we compose a
    // running counter-scale (`font_size_correction`) and pass it to
    // text rendering. Each text body multiplies its `font_scale` by
    // this value before emitting `font-size`, so the final visual size
    // equals `font_pt × correction × group_scale = font_pt`.
    //
    // For non-uniform group scales (`scale_x != scale_y`) we use the
    // geometric mean. The residual anisotropy still stretches glyphs
    // along one axis — accepted as a known limitation, documented in
    // .plans/00-rust-migration/plan.md.
    let scale_uniform = if scale_x > 0.0 && scale_y > 0.0 {
        (scale_x * scale_y).sqrt()
    } else {
        1.0
    };
    let child_font_size_correction = if scale_uniform == 0.0 {
        parent_font_size_correction
    } else {
        parent_font_size_correction / scale_uniform
    };

    let mut transform_parts: Vec<String> = Vec::new();
    transform_parts.push(format!("translate({}, {})", n(x), n(y)));

    if group.transform.rotation != 0.0 {
        transform_parts.push(format!(
            "rotate({}, {}, {})",
            n(group.transform.rotation),
            n(w / 2.0),
            n(h / 2.0)
        ));
    }

    if group.transform.flip_h || group.transform.flip_v {
        let sx: i32 = if group.transform.flip_h { -1 } else { 1 };
        let sy: i32 = if group.transform.flip_v { -1 } else { 1 };
        let tx = if group.transform.flip_h { w } else { 0.0 };
        let ty = if group.transform.flip_v { h } else { 0.0 };
        transform_parts.push(format!("translate({}, {})", n(tx), n(ty)));
        transform_parts.push(format!("scale({sx}, {sy})"));
    }

    transform_parts.push(format!("scale({}, {})", n(scale_x), n(scale_y)));
    transform_parts.push(format!("translate({}, {})", n(-ch_x), n(-ch_y)));

    let mut content = String::new();
    let mut defs = String::new();
    let sp_id_attr = crate::svg_builder::build_sp_id_attr(group.sp_id);
    let _ = write!(
        content,
        "<g{} transform=\"{}\"{}>",
        sp_id_attr,
        transform_parts.join(" "),
        build_object_name_attr(group.object_name.as_deref())
    );

    for child in &group.children {
        if element_hidden(child) {
            continue;
        }
        let child_result = render_element(
            child,
            ids,
            slide,
            script_fonts,
            measurer,
            mapping,
            cjk_platform,
            font_resolver,
            child_font_size_correction,
        )?;
        if let Some(r) = child_result {
            content.push_str(&r.content);
            defs.push_str(&r.defs);
        }
    }

    content.push_str("</g>");
    Ok(RenderResult { content, defs })
}
