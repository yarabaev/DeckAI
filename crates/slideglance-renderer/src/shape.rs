//! `<p:sp>` shape rendering.
//!
//! Direct port of. Wraps a
//! geometry SVG element with fill / outline / marker attributes, then
//! optionally appends a text body. `<a:effectLst>` is fully wired
//! through [`render_effects`]; the only `NotImplemented` case is
//! `<a:bodyPr autoFit="spAutoFit"/>`, which depends on a text-body
//! re-flow path that has not yet been ported.

use std::fmt::Write as _;

use slideglance_font::{CjkPlatform, FontMapping, FontResolver, ScriptFontContext, TextMeasurer};
use slideglance_model::{AutoFit, Fill, ShapeElement};

use crate::color::alpha_str;
use crate::effects::render_effects;
use crate::error::RendererError;
use crate::fill::{render_fill_attrs, render_markers, render_outline_attrs, FillAttrs};
use crate::geometry::fmt::n;
use crate::geometry::render_geometry;
use crate::id_gen::IdGen;
use crate::render_result::RenderResult;
use crate::slide_context::SlideRenderContext;
use crate::text::{compute_sp_autofit_height, render_text_body};
use crate::transform::{build_object_name_attr, build_transform_attr};

/// Render one [`ShapeElement`] to an SVG `<g>` fragment plus any
/// associated `<defs>`.
///
/// # Errors
///
/// Reserved for future use. The current implementation always returns
/// `Ok(...)`, but the [`Result`] return type is preserved so future
/// `WordArt`-warp / unsupported-preset paths can surface errors
/// without breaking the dispatcher's signature.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::unnecessary_wraps
)]
pub fn render_shape(
    shape: &ShapeElement,
    ids: &mut IdGen,
    slide: &SlideRenderContext,
    script_fonts: &ScriptFontContext,
    measurer: &dyn TextMeasurer,
    mapping: &FontMapping,
    cjk_platform: CjkPlatform,
    font_resolver: Option<&dyn FontResolver>,
    // Cumulative inverse of the SVG `transform="scale(...)"` factors
    // applied by every ancestor `render_group`. Pass `1.0` for any
    // shape that's *not* inside a scaled group; the value flows through
    // to `render_text_body` so text point sizes stay absolute despite
    // group scaling. See `render_text_body`'s param doc for the full
    // rationale.
    font_size_correction: f64,
) -> Result<RenderResult, RendererError> {
    // spAutofit: TS recomputes the shape height from the natural text
    // layout. Mirror that here by overriding the transform's
    // extent_height with the autofit's required height when the body
    // overflows the original box.
    let effective_transform = if let Some(tb) = &shape.text_body {
        if tb.body_properties.auto_fit == AutoFit::SpAutofit {
            if let Some(required_h) = compute_sp_autofit_height(
                tb,
                shape.transform.extent_width,
                shape.transform.extent_height,
                measurer,
            ) {
                let mut t = shape.transform;
                t.extent_height = required_h;
                t
            } else {
                shape.transform
            }
        } else {
            shape.transform
        }
    } else {
        shape.transform
    };

    let effect_result = render_effects(shape.effects.as_ref(), ids);

    let transform = &effective_transform;
    let w = transform.extent_width.to_pixels();
    let h = transform.extent_height.to_pixels();

    let transform_attr = build_transform_attr(transform);
    // `<a:ln w>` is an absolute stroke width in PowerPoint, but the SVG
    // `<g transform="scale(...)">` chain that comes from each ancestor
    // group's `chExt → ext` ratio also scales the stroke. Multiplying
    // the local stroke-width by `font_size_correction` (already the
    // inverse of that cumulative scale) cancels the upstream scale so
    // the rendered stroke matches the source. Without this, large group
    // scale factors (15×+ on poster-size slides) blow up borders into
    // thick black bars (see Simple Minimal Formal Research Poster).
    let outline_result = render_outline_attrs(shape.outline.as_ref(), ids, font_size_correction);
    let marker_result = render_markers(shape.outline.as_ref(), ids);

    let geometry_svg = render_geometry(&shape.geometry, w, h);

    // Plain-stretch image fills paint via `<clipPath>` + `<image>` instead
    // of `<path fill="url(#imgfill)">`. Reason: the SVG renderer paints a
    // 1-px antialiased hairline along the geometry's straight edges
    // (visible at TL/BR of leaf-shaped images with transparent corners
    // — see `<a:blip>` images on Google Slides poster templates), and
    // `<image>` has no path edge to antialias.
    //
    // Tile / srcRect / non-zero-stretch image fills keep the legacy
    // pattern path. Those forms can't be reproduced with a single
    // `<image>` element — `objectBoundingBox` patterning is required for
    // correct tiling and source cropping. Solid / gradient / pattern
    // fills also keep the legacy inline path because they don't trigger
    // the hairline (the painted body fully covers the path's bbox).
    let image_fill = match shape.fill.as_ref() {
        Some(Fill::Image(img))
            if img.tile.is_none()
                && img.src_rect.is_none()
                && img.stretch.as_ref().is_none_or(is_zero_stretch) =>
        {
            Some(img)
        }
        _ => None,
    };
    let fill_result = if image_fill.is_some() {
        FillAttrs {
            attrs: "fill=\"none\"".to_string(),
            defs: String::new(),
        }
    } else {
        render_fill_attrs(shape.fill.as_ref(), ids)
    };

    let mut defs = String::new();
    defs.push_str(&fill_result.defs);
    defs.push_str(&outline_result.defs);
    defs.push_str(&marker_result.defs);
    defs.push_str(&effect_result.filter_defs);

    let image_clip_id = image_fill.map(|_| ids.next_id("img-clip"));
    if let (Some(clip_id), Some(_)) = (image_clip_id.as_ref(), image_fill) {
        let _ = write!(defs, "<clipPath id=\"{clip_id}\">{geometry_svg}</clipPath>");
    }

    let mut content = String::new();
    let filter_attr = if effect_result.filter_attr.is_empty() {
        String::new()
    } else {
        format!(" {}", effect_result.filter_attr)
    };
    let marker_attr_str = combine_marker_attrs(&marker_result.start_attr, &marker_result.end_attr);
    let name_attr = build_object_name_attr(shape.object_name.as_deref());
    let sp_id_attr = crate::svg_builder::build_sp_id_attr(shape.sp_id);
    let _ = write!(
        content,
        "<g{sp_id_attr} transform=\"{transform_attr}\"{filter_attr}{name_attr}>"
    );

    if let (Some(clip_id), Some(img)) = (image_clip_id.as_ref(), image_fill) {
        write_clipped_image_body(
            &mut content,
            clip_id,
            img,
            shape.outline.as_ref(),
            &geometry_svg,
            &outline_result.attrs,
            &marker_attr_str,
            w,
            h,
        );
    } else if !geometry_svg.is_empty() {
        // Inline fill + outline + marker attributes after the geometry's
        // opening tag name (TS regex /^<(\w+)/).
        let inline = format!(
            "{} {}{}",
            fill_result.attrs, outline_result.attrs, marker_attr_str
        );
        content.push_str(&inject_attrs_after_tag(&geometry_svg, &inline));
    }

    if let Some(tb) = &shape.text_body {
        let text_svg = render_text_body(
            tb,
            transform,
            slide,
            script_fonts,
            measurer,
            mapping,
            cjk_platform,
            font_resolver,
            false, // shape text body — full leading delta
            font_size_correction,
        );
        if !text_svg.is_empty() {
            // PowerPoint's flipH / flipV on a shape mirrors the geometry
            // but the embedded text body still reads in the natural
            // direction (slide 22's "TextBox 89" carries flipH=1 with
            // ordinary Korean bullet text, and the PDF reference shows
            // those bullets unflipped). Our parent group transform bakes
            // the flip into the whole `<g>`, so without compensation the
            // text would read backwards. Counter-flip with the same
            // `translate(w/h) scale(-1/-1)` block applied to the parent
            // — applied a second time inside the group it cancels back
            // to identity, restoring text orientation while keeping the
            // geometry mirrored.
            if transform.flip_h || transform.flip_v {
                let sx = if transform.flip_h { -1 } else { 1 };
                let sy = if transform.flip_v { -1 } else { 1 };
                let tx = if transform.flip_h { w } else { 0.0 };
                let ty = if transform.flip_v { h } else { 0.0 };
                let _ = write!(
                    content,
                    "<g transform=\"translate({tx}, {ty}) scale({sx}, {sy})\">{text_svg}</g>",
                );
            } else {
                content.push_str(&text_svg);
            }
        }
    }

    content.push_str("</g>");

    Ok(RenderResult { content, defs })
}

/// `<a:stretch><a:fillRect/>` is treated as a no-op stretch when every
/// edge inset is zero — the image fills the shape's bounding box 1:1.
/// Non-zero values displace the image and can't be reproduced by a
/// plain `<image width=W height=H/>` overlay, so those cases stay on
/// the pattern-fill code path.
fn is_zero_stretch(s: &slideglance_model::ImageRect) -> bool {
    s.left == 0.0 && s.top == 0.0 && s.right == 0.0 && s.bottom == 0.0
}

/// Emit the body for a shape whose fill is an `<a:blipFill>`.
///
/// Image fills paint via `<image clip-path="url(#…)">` instead of the
/// pattern-fill `<path fill="url(#imgfill-…)">` form. The pattern path
/// produces a 1-px antialiased hairline along the geometry's straight
/// rectangular edges (visible as a faint gray bbox at TL/BR of leaf-
/// shaped images with transparent corners). Drawing the `<image>`
/// directly under a clipPath has no path edge to antialias.
///
/// When an outline is present the geometry is re-emitted on top of the
/// clipped image with `fill="none"` plus the outline / marker attrs so
/// the stroke wraps the visible shape without re-painting the fill.
#[allow(clippy::too_many_arguments)]
fn write_clipped_image_body(
    content: &mut String,
    clip_id: &str,
    img: &slideglance_model::ImageFill,
    outline: Option<&slideglance_model::Outline>,
    geometry_svg: &str,
    outline_attrs: &str,
    marker_attr_str: &str,
    w: f64,
    h: f64,
) {
    // Stretch the image across the shape's local bounding box. The
    // clipPath cuts it back to the actual geometry. `preserveAspectRatio="none"`
    // mirrors the `<a:stretch><a:fillRect/>` semantic that PowerPoint uses
    // by default for shape-blip fills.
    let opacity_attr = if img.alpha < 1.0 {
        format!(" opacity=\"{}\"", alpha_str(img.alpha.max(0.0)))
    } else {
        String::new()
    };
    let _ = write!(
        content,
        "<image clip-path=\"url(#{clip_id})\" href=\"data:{};base64,{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\"{opacity_attr}/>",
        img.mime_type,
        img.image_data,
        n(w),
        n(h)
    );
    if outline.is_some() && !geometry_svg.is_empty() {
        let inline = format!("fill=\"none\" {outline_attrs}{marker_attr_str}");
        content.push_str(&inject_attrs_after_tag(geometry_svg, &inline));
    }
}

/// Build the `marker-start=… marker-end=…` fragment that follows the
/// inlined fill/outline attributes. Empty when neither end has a marker.
fn combine_marker_attrs(start_attr: &str, end_attr: &str) -> String {
    match (start_attr.is_empty(), end_attr.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!(" {start_attr}"),
        (true, false) => format!(" {end_attr}"),
        (false, false) => format!(" {start_attr} {end_attr}"),
    }
}

/// Insert `attrs` (already space-prefixed-friendly content, no leading
/// space) after the opening tag's name. Mirrors the TS regex
/// `geometrySvg.replace(/^<(\w+)/, '<$1 …')`. The geometry renderer
/// always emits a tag whose name is followed by either a space or `>`,
/// so a hand-rolled scan is cheaper than pulling in a regex crate and
/// keeps the renderer's only "stringly-typed" splice in one place.
fn inject_attrs_after_tag(svg: &str, attrs: &str) -> String {
    let bytes = svg.as_bytes();
    if bytes.first() != Some(&b'<') {
        return svg.to_string();
    }
    let mut i = 1;
    while i < bytes.len() {
        let c = bytes[i];
        // ASCII tag-name chars only — geometry renderer never emits XML
        // namespace prefixes (no `:`) or HTML-style hyphens here.
        if c.is_ascii_alphanumeric() || c == b'_' {
            i += 1;
        } else {
            break;
        }
    }
    if i == 1 {
        // No tag name (e.g. literal "<>") — leave untouched.
        return svg.to_string();
    }
    let mut out = String::with_capacity(svg.len() + attrs.len() + 1);
    out.push_str(&svg[..i]);
    out.push(' ');
    out.push_str(attrs);
    out.push_str(&svg[i..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_color::{ResolvedColor, Rgb};
    use slideglance_font::{FontMapping, HeuristicTextMeasurer, ScriptFontContext};
    use slideglance_model::{
        BodyProperties, DashStyle, EffectList, Fill, Geometry, ImageFill, OuterShadow, Outline,
        OutlineFill, PresetGeometry, ShapeElement, SolidFill, TextBody, TextVerticalType,
        Transform, VerticalAnchor, WrapMode,
    };
    use slideglance_utils::Emu;
    use std::collections::BTreeMap;

    fn rect_geometry() -> Geometry {
        Geometry::Preset(PresetGeometry {
            preset: "rect".to_string(),
            adjust_values: BTreeMap::new(),
        })
    }

    fn opaque(hex: &str) -> ResolvedColor {
        ResolvedColor::new(Rgb::from_hex(hex).unwrap(), 1.0)
    }

    fn basic_shape() -> ShapeElement {
        ShapeElement {
            sp_id: None,
            transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(914_400),
                extent_height: Emu::new(914_400),
                ..Transform::default()
            },
            geometry: rect_geometry(),
            fill: Some(Fill::Solid(SolidFill {
                color: opaque("#FF0000"),
            })),
            outline: None,
            text_body: None,
            effects: None,
            placeholder_type: None,
            placeholder_idx: None,
            alt_text: None,
            object_name: None,
            hidden: false,
            hyperlink: None,
        }
    }

    fn body_props_with(auto_fit: AutoFit) -> BodyProperties {
        BodyProperties {
            anchor: VerticalAnchor::T,
            margin_left: Emu::new(0),
            margin_right: Emu::new(0),
            margin_top: Emu::new(0),
            margin_bottom: Emu::new(0),
            wrap: WrapMode::Square,
            auto_fit,
            font_scale: 1.0,
            ln_spc_reduction: 0.0,
            num_col: 1,
            vert: TextVerticalType::Horz,
            spc_first_last_para: false,
            compat_ln_spc: false,
            prst_tx_warp: None,
        }
    }

    fn render_default(shape: &ShapeElement) -> Result<RenderResult, RendererError> {
        let mut ids = IdGen::new();
        render_shape(
            shape,
            &mut ids,
            &SlideRenderContext::new(1),
            &ScriptFontContext::empty(),
            &HeuristicTextMeasurer,
            &FontMapping::new(),
            CjkPlatform::Other,
            None,
            1.0,
        )
    }

    #[test]
    fn solid_rect_emits_inlined_fill_attribute() {
        let r = render_default(&basic_shape()).unwrap();
        assert!(
            r.content.starts_with("<g transform=\""),
            "content={}",
            r.content
        );
        assert!(
            r.content.contains("fill=\"#FF0000\""),
            "fill missing: {}",
            r.content
        );
        assert!(r.content.contains("stroke=\"none\""));
        assert!(r.content.ends_with("</g>"));
    }

    #[test]
    fn shape_emits_data_sp_id_when_present() {
        let mut shape = basic_shape();
        shape.sp_id = Some(123);
        let r = render_default(&shape).unwrap();
        assert!(
            r.content.contains("data-sp-id=\"123\""),
            "data-sp-id missing: {}",
            r.content
        );
        // Attribute lands on the outer <g> immediately after the tag name.
        assert!(
            r.content.starts_with("<g data-sp-id=\"123\" transform=\""),
            "unexpected ordering: {}",
            r.content
        );
    }

    #[test]
    fn shape_omits_data_sp_id_when_none() {
        let mut shape = basic_shape();
        shape.sp_id = None;
        let r = render_default(&shape).unwrap();
        assert!(
            !r.content.contains("data-sp-id"),
            "data-sp-id should be absent: {}",
            r.content
        );
    }

    #[test]
    fn empty_effect_list_is_silent() {
        let mut s = basic_shape();
        s.effects = Some(EffectList::default());
        let r = render_default(&s).unwrap();
        assert!(!r.content.contains("filter="));
    }

    #[test]
    fn populated_effect_list_emits_filter_attribute_and_def() {
        let mut s = basic_shape();
        s.effects = Some(EffectList {
            outer_shadow: Some(OuterShadow {
                blur_radius: Emu::new(127_000),
                distance: Emu::new(50_800),
                direction: 5_400.0 / 60_000.0,
                color: opaque("#000000"),
                alignment: "bl".to_string(),
                rotate_with_shape: false,
            }),
            ..EffectList::default()
        });
        let r = render_default(&s).unwrap();
        assert!(
            r.content.contains(" filter=\"url(#effect-"),
            "{}",
            r.content
        );
        assert!(r.defs.contains("<filter id=\"effect-"), "{}", r.defs);
        assert!(r.defs.contains("outerShadowMerge"));
    }

    #[test]
    fn sp_autofit_with_empty_body_passes_through_unchanged() {
        // Empty bodies have no text height to estimate, so the autofit
        // helper returns None and the original transform is used.
        let mut s = basic_shape();
        s.text_body = Some(TextBody {
            default_text_color: None,
            body_properties: body_props_with(AutoFit::SpAutofit),
            paragraphs: Vec::new(),
        });
        let _ = render_default(&s).expect("empty spAutofit body must not error");
    }

    #[test]
    fn norm_autofit_does_not_error() {
        // Only spAutofit is special-cased in ; normAutofit
        // is handled inside text-renderer's measurement logic, so the
        // shape renderer should pass it through unchanged.
        let mut s = basic_shape();
        s.text_body = Some(TextBody {
            default_text_color: None,
            body_properties: body_props_with(AutoFit::NormAutofit),
            paragraphs: Vec::new(),
        });
        let _ = render_default(&s).expect("normAutofit must not error");
    }

    #[test]
    fn object_name_appears_as_data_attribute() {
        let mut s = basic_shape();
        s.object_name = Some("Hero CTA".to_string());
        let r = render_default(&s).unwrap();
        assert!(
            r.content.contains("data-object-name=\"Hero CTA\""),
            "{}",
            r.content
        );
    }

    #[test]
    fn unknown_preset_falls_back_to_rect_with_inlined_attrs() {
        // The geometry renderer's catch-all returns a `<rect>` for any
        // preset it does not recognise, so the styled element is still
        // emitted with the shape's fill / outline attributes. This
        // mirrors the spec's catch-all behaviour and verifies
        // that `inject_attrs_after_tag` works on the fallback shape.
        let mut s = basic_shape();
        s.geometry = Geometry::Preset(PresetGeometry {
            preset: "definitelyNoSuchPreset".to_string(),
            adjust_values: BTreeMap::new(),
        });
        let r = render_default(&s).unwrap();
        assert!(
            r.content.contains("<rect fill=\"#FF0000\""),
            "{}",
            r.content
        );
    }

    // --- image-fill: clipPath + <image> branch ---

    fn image_fill_with_alpha(alpha: f64) -> Fill {
        Fill::Image(ImageFill {
            image_data: "AA".to_string(),
            mime_type: "image/png".to_string(),
            tile: None,
            src_rect: None,
            stretch: None,
            alpha,
        })
    }

    #[test]
    fn image_fill_emits_clip_path_def_and_image_overlay() {
        // Image fill must NOT inline `fill="url(#imgfill)"` into the
        // geometry path. Instead the renderer emits a `<clipPath>` whose
        // body is the geometry plus an `<image clip-path="..."/>` so the
        // image is drawn directly without painting a path edge — this is
        // what removes the 1-px hairline that the pattern-fill approach
        // produced at the path's straight edges.
        let mut s = basic_shape();
        s.fill = Some(image_fill_with_alpha(1.0));
        let r = render_default(&s).unwrap();
        assert!(
            r.defs.contains("<clipPath id=\"img-clip-"),
            "expected clipPath def, got: {}",
            r.defs
        );
        assert!(
            r.content.contains("<image clip-path=\"url(#img-clip-"),
            "expected clipped image overlay, got: {}",
            r.content
        );
        assert!(
            !r.content.contains("fill=\"url(#imgfill"),
            "image fill must not be referenced via pattern URL: {}",
            r.content
        );
    }

    #[test]
    fn image_fill_with_alpha_lt_one_emits_opacity_on_overlay() {
        let mut s = basic_shape();
        s.fill = Some(image_fill_with_alpha(0.05));
        let r = render_default(&s).unwrap();
        assert!(
            r.content.contains("opacity=\"0.05\""),
            "expected opacity attr on image overlay: {}",
            r.content
        );
    }

    #[test]
    fn image_fill_with_outline_re_emits_geometry_for_stroke() {
        // Outline must still render on top of the clipped image, so the
        // geometry is re-emitted with `fill="none"` plus the outline attrs.
        let mut s = basic_shape();
        s.fill = Some(image_fill_with_alpha(1.0));
        s.outline = Some(Outline {
            width: Emu::new(12_700),
            fill: Some(OutlineFill::Solid(SolidFill {
                color: opaque("#0000FF"),
            })),
            dash_style: DashStyle::Solid,
            custom_dash: None,
            line_cap: None,
            line_join: None,
            head_end: None,
            tail_end: None,
        });
        let r = render_default(&s).unwrap();
        assert!(
            r.content.contains("<image clip-path=\"url(#img-clip-"),
            "image overlay missing: {}",
            r.content
        );
        assert!(
            r.content.contains("fill=\"none\"") && r.content.contains("stroke=\"#0000FF\""),
            "outline path missing: {}",
            r.content
        );
    }

    #[test]
    fn image_fill_with_tile_keeps_pattern_path_to_preserve_tiling() {
        // `<a:tile>` semantics can't be reproduced with a single
        // `<image width=W height=H/>` — falling back to the pattern
        // path keeps tiled shape-blip fills visually correct.
        use slideglance_model::{ImageFillTile, ImageFlip};
        use slideglance_utils::Emu;
        let mut s = basic_shape();
        s.fill = Some(Fill::Image(ImageFill {
            image_data: "AA".to_string(),
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
        }));
        let r = render_default(&s).unwrap();
        // Tiled image fill stays on the pattern code path: the
        // `imgfill-N` pattern reference must appear, the clip-path
        // overlay must NOT.
        assert!(
            r.content.contains("fill=\"url(#imgfill-"),
            "tiled fill must use pattern: {}",
            r.content
        );
        assert!(
            !r.content.contains("clip-path=\"url(#img-clip-"),
            "tiled fill must not use clip-path overlay: {}",
            r.content
        );
    }

    #[test]
    fn image_fill_with_src_rect_keeps_pattern_path() {
        // `<a:srcRect>` crops the image; the clip-path overlay would
        // emit the un-cropped image. Pattern path stays in charge.
        use slideglance_model::ImageRect;
        let mut s = basic_shape();
        s.fill = Some(Fill::Image(ImageFill {
            image_data: "AA".to_string(),
            mime_type: "image/png".to_string(),
            tile: None,
            src_rect: Some(ImageRect {
                left: 0.1,
                top: 0.1,
                right: 0.1,
                bottom: 0.1,
            }),
            stretch: None,
            alpha: 1.0,
        }));
        let r = render_default(&s).unwrap();
        assert!(
            r.content.contains("fill=\"url(#imgfill-"),
            "srcRect must use pattern: {}",
            r.content
        );
        assert!(
            !r.content.contains("clip-path=\"url(#img-clip-"),
            "srcRect must not use clip-path overlay: {}",
            r.content
        );
    }

    #[test]
    fn image_fill_with_zero_stretch_uses_clip_path_overlay() {
        // Zero-inset `<a:stretch><a:fillRect/>` is the no-op stretch
        // form (image fills shape bbox 1:1) and is safe for the
        // clip-path overlay path. Verifies the regression guard is
        // not over-restrictive.
        use slideglance_model::ImageRect;
        let mut s = basic_shape();
        s.fill = Some(Fill::Image(ImageFill {
            image_data: "AA".to_string(),
            mime_type: "image/png".to_string(),
            tile: None,
            src_rect: None,
            stretch: Some(ImageRect {
                left: 0.0,
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
            }),
            alpha: 1.0,
        }));
        let r = render_default(&s).unwrap();
        assert!(
            r.content.contains("clip-path=\"url(#img-clip-"),
            "zero-inset stretch must use clip-path overlay: {}",
            r.content
        );
    }

    #[test]
    fn image_fill_with_nonzero_stretch_keeps_pattern_path() {
        // Non-zero stretch insets displace the image from the bbox; the
        // current clip-path overlay does not honour the offset, so we
        // keep those cases on the pattern path.
        use slideglance_model::ImageRect;
        let mut s = basic_shape();
        s.fill = Some(Fill::Image(ImageFill {
            image_data: "AA".to_string(),
            mime_type: "image/png".to_string(),
            tile: None,
            src_rect: None,
            stretch: Some(ImageRect {
                left: 0.1,
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
            }),
            alpha: 1.0,
        }));
        let r = render_default(&s).unwrap();
        assert!(
            r.content.contains("fill=\"url(#imgfill-"),
            "non-zero stretch must use pattern: {}",
            r.content
        );
    }

    #[test]
    fn image_fill_with_no_outline_skips_geometry_path_emission() {
        // No outline -> only the clipped image is drawn. The geometry
        // path itself must not appear in the body (it lives only inside
        // <clipPath> in <defs>).
        let mut s = basic_shape();
        s.fill = Some(image_fill_with_alpha(1.0));
        s.outline = None;
        let r = render_default(&s).unwrap();
        // The body content (between the outer <g>...</g>) should contain
        // exactly one <image> and zero <rect> / <path> / <ellipse> etc.
        // Reuse the rect-preset geometry: a `<rect>` would only appear if
        // we were emitting the path-as-stroked-outline branch.
        let body = &r.content;
        assert!(
            body.contains("<image clip-path=\"url(#img-clip-"),
            "{}",
            body
        );
        // The geometry only lives in defs as the clipPath child.
        assert!(
            !body.contains("<rect fill=") && !body.contains("<rect stroke="),
            "rect emitted in body: {body}"
        );
    }

    // --- helper tests ---

    #[test]
    fn inject_attrs_inserts_after_first_tag_name() {
        let svg = "<rect width=\"10\"/>";
        let out = inject_attrs_after_tag(svg, "fill=\"red\"");
        assert_eq!(out, "<rect fill=\"red\" width=\"10\"/>");
    }

    #[test]
    fn inject_attrs_handles_self_closing_with_no_attrs() {
        let svg = "<g/>";
        let out = inject_attrs_after_tag(svg, "x=\"1\"");
        assert_eq!(out, "<g x=\"1\"/>");
    }

    #[test]
    fn inject_attrs_no_op_for_non_tag_input() {
        let svg = "plain text";
        let out = inject_attrs_after_tag(svg, "fill=\"red\"");
        assert_eq!(out, "plain text");
    }

    #[test]
    fn combine_marker_attrs_handles_all_four_arities() {
        assert_eq!(combine_marker_attrs("", ""), "");
        assert_eq!(
            combine_marker_attrs("marker-start=\"url(#a)\"", ""),
            " marker-start=\"url(#a)\""
        );
        assert_eq!(
            combine_marker_attrs("", "marker-end=\"url(#b)\""),
            " marker-end=\"url(#b)\""
        );
        assert_eq!(
            combine_marker_attrs("marker-start=\"url(#a)\"", "marker-end=\"url(#b)\""),
            " marker-start=\"url(#a)\" marker-end=\"url(#b)\""
        );
    }
}
