//! Image element rendering.
//!
//! Direct port of. Output is a
//! `<g transform="...">` group containing an `<image>` (or `<rect fill="url(#tile-...)">`
//! when `<a:tile>` is set), plus an EMF/WMF placeholder for vector formats
//! the renderer cannot inline as raster.
//!
//! `<a:effectLst>` (shape-level filters) attaches to the outer `<g>`
//! wrapper; `<a:blipFill>` color adjustments
//! (`grayscl` / `biLevel` / `blur` / `lum` / `duotone` / `clrChange`)
//! attach to an inner `<g>` wrapping the `<image>` (or the tile-pattern
//! `<rect>`) so the two filter chains stay independent.

use std::fmt::Write as _;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use slideglance_model::{ImageElement, ImageFlip, SrcRect, StretchFillRect, TileInfo};

use crate::blip_effects::render_blip_effects;
use crate::color::alpha_str;
use crate::effects::render_effects;
use crate::geometry::fmt::n;
use crate::id_gen::IdGen;
use crate::transform::{build_object_name_attr, build_transform_attr};

/// Builds the inner `<g>` wrapper attributes that carry both the blip-effect
/// filter (when present) and an `<a:alphaModFix>`-derived opacity (when the
/// alpha is less than `1.0`). The wrapper sits between the outer transform
/// group and the actual `<image>` / tile-rect so opacity composes with the
/// image content but stays out of the shape-effect filter chain.
fn build_inner_wrapper_attrs(blip_filter_attr: &str, alpha: f64) -> String {
    let mut out = String::new();
    if !blip_filter_attr.is_empty() {
        out.push_str(blip_filter_attr);
    }
    if alpha < 1.0 {
        let clamped = alpha.max(0.0);
        let _ = write!(out, " opacity=\"{}\"", alpha_str(clamped));
    }
    out
}

/// Rendering result of an [`ImageElement`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImageRenderResult {
    /// SVG body (a `<g>` wrapper).
    pub content: String,
    /// `<defs>` content (e.g. `<pattern>` for tiled images, `<clipPath>` for
    /// `srcRect` cropping).
    pub defs: String,
}

/// Render an [`ImageElement`] to an SVG fragment.
///
/// `ids` is used to mint deterministic IDs for `<pattern>` (tile) and
/// `<clipPath>` (`srcRect`) cross-references. EMF / WMF images render as a
/// non-raster placeholder rectangle since SVG cannot inline those formats.
#[must_use]
pub fn render_image(image: &ImageElement, ids: &mut IdGen) -> ImageRenderResult {
    let w = image.transform.extent_width.to_pixels();
    let h = image.transform.extent_height.to_pixels();
    let transform_attr = build_transform_attr(&image.transform);
    let name_attr = build_object_name_attr(image.object_name.as_deref());
    let sp_id_attr = crate::svg_builder::build_sp_id_attr(image.sp_id);

    // EMF / WMF metafiles: try to extract the embedded raster (DIB → PNG) and
    // rewrite the image to behave like a regular PNG below. Real vector EMFs
    // have no extractable raster — they fall through to the placeholder.
    let rasterized;
    let image = if image.mime_type == "image/emf" || image.mime_type == "image/wmf" {
        match try_rasterize_metafile(&image.image_data) {
            Some(png_data) => {
                let mut clone = image.clone();
                clone.mime_type = "image/png".to_string();
                clone.image_data = png_data;
                rasterized = clone;
                &rasterized
            }
            None => {
                return ImageRenderResult {
                    content: render_placeholder(
                        &image.mime_type,
                        w,
                        h,
                        &transform_attr,
                        &name_attr,
                        &sp_id_attr,
                        image.alt_text.as_deref(),
                    ),
                    defs: String::new(),
                };
            }
        }
    } else {
        image
    };

    let effect_result = render_effects(image.effects.as_ref(), ids);
    let blip_result = render_blip_effects(image.blip_effects.as_ref(), ids);

    let mut defs = String::new();
    defs.push_str(&effect_result.filter_defs);
    defs.push_str(&blip_result.filter_defs);

    let filter_attr = if effect_result.filter_attr.is_empty() {
        String::new()
    } else {
        format!(" {}", effect_result.filter_attr)
    };
    let blip_filter_attr = if blip_result.filter_attr.is_empty() {
        String::new()
    } else {
        format!(" {}", blip_result.filter_attr)
    };
    let inner_attrs = build_inner_wrapper_attrs(&blip_filter_attr, image.alpha);

    if let Some(tile) = &image.tile {
        return render_tiled(
            image,
            tile,
            w,
            h,
            &transform_attr,
            &filter_attr,
            &inner_attrs,
            &name_attr,
            &sp_id_attr,
            ids,
            defs,
        );
    }

    let img_tag = build_image_tag(image, w, h, ids, &mut defs);
    let inner = if inner_attrs.is_empty() {
        img_tag
    } else {
        format!("<g{inner_attrs}>{img_tag}</g>")
    };
    let content = format!(
        "<g{sp_id_attr} transform=\"{transform_attr}\"{filter_attr}{name_attr}>{inner}</g>"
    );
    ImageRenderResult { content, defs }
}

fn build_image_tag(
    image: &ImageElement,
    w: f64,
    h: f64,
    ids: &mut IdGen,
    defs: &mut String,
) -> String {
    if let Some(src) = image.src_rect {
        return build_src_rect_image(image, src, w, h, ids, defs);
    }
    if let Some(stretch) = image.stretch {
        return build_stretched_image(image, stretch, w, h);
    }
    format!(
        "<image href=\"data:{};base64,{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\"/>",
        image.mime_type,
        image.image_data,
        n(w),
        n(h)
    )
}

fn build_src_rect_image(
    image: &ImageElement,
    src: SrcRect,
    w: f64,
    h: f64,
    ids: &mut IdGen,
    defs: &mut String,
) -> String {
    // `<a:srcRect>` crops the source by per-edge fractions. We emulate the
    // crop with a clipPath at the visible box plus a rescaled `<image>`
    // positioned so the visible region lines up with `(0, 0, w, h)`.
    let clip_id = ids.next_id("crop");
    let visible_x = (1.0 - src.left - src.right).max(f64::MIN_POSITIVE);
    let visible_y = (1.0 - src.top - src.bottom).max(f64::MIN_POSITIVE);
    let scaled_w = (w / visible_x).round();
    let scaled_h = (h / visible_y).round();
    let img_x = (-src.left * scaled_w).round();
    let img_y = (-src.top * scaled_h).round();
    let _ = write!(
        defs,
        "<clipPath id=\"{clip_id}\"><rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\"/></clipPath>",
        n(w),
        n(h)
    );
    format!(
        "<image clip-path=\"url(#{clip_id})\" href=\"data:{};base64,{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\"/>",
        image.mime_type,
        image.image_data,
        n(img_x),
        n(img_y),
        n(scaled_w),
        n(scaled_h)
    )
}

fn build_stretched_image(image: &ImageElement, stretch: StretchFillRect, w: f64, h: f64) -> String {
    let img_x = (w * stretch.left).round();
    let img_y = (h * stretch.top).round();
    let img_w = (w * (1.0 - stretch.left - stretch.right)).round();
    let img_h = (h * (1.0 - stretch.top - stretch.bottom)).round();
    format!(
        "<image href=\"data:{};base64,{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\"/>",
        image.mime_type,
        image.image_data,
        n(img_x),
        n(img_y),
        n(img_w),
        n(img_h)
    )
}

#[allow(clippy::too_many_arguments)]
fn render_tiled(
    image: &ImageElement,
    tile: &TileInfo,
    w: f64,
    h: f64,
    transform_attr: &str,
    filter_attr: &str,
    inner_attrs: &str,
    name_attr: &str,
    sp_id_attr: &str,
    ids: &mut IdGen,
    mut defs: String,
) -> ImageRenderResult {
    let pattern_id = ids.next_id("tile");
    let tile_w = (w * tile.sx).round();
    let tile_h = (h * tile.sy).round();
    let offset_x = tile.tx.to_pixels();
    let offset_y = tile.ty.to_pixels();
    let img_transform = match tile.flip {
        ImageFlip::X => format!(" transform=\"translate({}, 0) scale(-1, 1)\"", n(tile_w)),
        ImageFlip::Y => format!(" transform=\"translate(0, {}) scale(1, -1)\"", n(tile_h)),
        ImageFlip::Xy => format!(
            " transform=\"translate({}, {}) scale(-1, -1)\"",
            n(tile_w),
            n(tile_h)
        ),
        ImageFlip::None => String::new(),
    };
    let _ = write!(
        defs,
        "<pattern id=\"{pattern_id}\" patternUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"><image href=\"data:{};base64,{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\"{img_transform}/></pattern>",
        n(offset_x),
        n(offset_y),
        n(tile_w),
        n(tile_h),
        image.mime_type,
        image.image_data,
        n(tile_w),
        n(tile_h)
    );
    let rect = format!(
        "<rect width=\"{}\" height=\"{}\" fill=\"url(#{pattern_id})\"/>",
        n(w),
        n(h)
    );
    let inner = if inner_attrs.is_empty() {
        rect
    } else {
        format!("<g{inner_attrs}>{rect}</g>")
    };
    let content = format!(
        "<g{sp_id_attr} transform=\"{transform_attr}\"{filter_attr}{name_attr}>{inner}</g>"
    );
    ImageRenderResult { content, defs }
}

/// Attempt to extract a raster bitmap from an EMF/WMF base64 payload and
/// re-encode it as base64 PNG. Returns `None` for genuine vector metafiles
/// or any decode/extract/encode error — caller falls back to placeholder.
pub(crate) fn try_rasterize_metafile(image_data_b64: &str) -> Option<String> {
    let raw = BASE64_STANDARD.decode(image_data_b64).ok()?;
    let png = slideglance_emf::extract_raster(&raw)?;
    Some(BASE64_STANDARD.encode(&png))
}

fn render_placeholder(
    mime_type: &str,
    w: f64,
    h: f64,
    transform_attr: &str,
    name_attr: &str,
    sp_id_attr: &str,
    alt_text: Option<&str>,
) -> String {
    // EMF / WMF (Windows vector metafiles) are not yet rasterized.
    // Emit a neutral light-grey rect at the picture's bounding box so
    // the rest of the slide composes correctly (positions / sizes
    // unchanged) without injecting a noisy "EMF unsupported" badge
    // into every affected slide. The earlier badge (icon + label +
    // alt-text) was useful while debugging the renderer but it
    // visually dominates pages whose EMF image is the centerpiece.
    let _ = mime_type;
    let _ = alt_text;
    format!(
        "<g{sp_id_attr} transform=\"{transform_attr}\"{name_attr}><rect width=\"{}\" height=\"{}\" fill=\"#F5F5F5\" stroke=\"#BDBDBD\" stroke-width=\"1\" stroke-dasharray=\"4,3\"/></g>",
        n(w),
        n(h)
    )
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use slideglance_model::{ImageElement, Transform};
    use slideglance_utils::Emu;

    fn xfrm(w: i64, h: i64) -> Transform {
        Transform {
            offset_x: Emu::new(0),
            offset_y: Emu::new(0),
            extent_width: Emu::new(w),
            extent_height: Emu::new(h),
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
        }
    }

    fn png_image(w: i64, h: i64) -> ImageElement {
        ImageElement {
            sp_id: None,
            transform: xfrm(w, h),
            image_data: "AAAA".to_string(),
            mime_type: "image/png".to_string(),
            effects: None,
            blip_effects: None,
            src_rect: None,
            alt_text: None,
            object_name: None,
            hidden: false,
            stretch: None,
            tile: None,
            alpha: 1.0,
        }
    }

    #[test]
    fn plain_image_emits_image_tag_with_full_box() {
        let img = png_image(914_400, 914_400);
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(res.content.starts_with("<g transform="));
        assert!(res
            .content
            .contains("<image href=\"data:image/png;base64,AAAA\""));
        assert!(res.content.contains("width=\"96\""));
        assert!(res.content.contains("height=\"96\""));
        assert!(res.content.ends_with("</g>"));
        assert!(res.defs.is_empty());
    }

    #[test]
    fn stretch_inset_offsets_image_inside_box() {
        let mut img = png_image(914_400, 914_400);
        img.stretch = Some(StretchFillRect {
            left: 0.1,
            top: 0.0,
            right: 0.0,
            bottom: 0.2,
        });
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        // 96 px * 0.1 inset on left = 9.6 -> rounds to 10.
        assert!(res.content.contains("x=\"10\""));
        assert!(res.content.contains("y=\"0\""));
        // width = 96 * (1 - 0.1 - 0) = 86.4 -> rounds to 86.
        assert!(res.content.contains("width=\"86\""));
        assert!(res.content.contains("height=\"77\""));
    }

    #[test]
    fn src_rect_applies_clip_and_rescale() {
        let mut img = png_image(914_400, 914_400);
        img.src_rect = Some(SrcRect {
            left: 0.25,
            top: 0.25,
            right: 0.25,
            bottom: 0.25,
        });
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(res.defs.contains("<clipPath id=\"crop-0\""));
        assert!(res.content.contains("clip-path=\"url(#crop-0)\""));
        // Visible box is 50% × 50%, so scaled image is 192 × 192.
        assert!(res.content.contains("width=\"192\""));
        assert!(res.content.contains("height=\"192\""));
    }

    #[test]
    fn tile_creates_pattern_def() {
        let mut img = png_image(1_828_800, 914_400);
        img.tile = Some(TileInfo {
            tx: Emu::new(0),
            ty: Emu::new(0),
            sx: 0.5,
            sy: 1.0,
            flip: ImageFlip::None,
            align: "tl".to_string(),
        });
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(res.defs.contains("<pattern id=\"tile-0\""));
        assert!(res.defs.contains("width=\"96\""));
        assert!(res
            .content
            .contains("<rect width=\"192\" height=\"96\" fill=\"url(#tile-0)\"/>"));
    }

    #[test]
    fn tile_flip_xy_emits_double_negative_scale() {
        let mut img = png_image(914_400, 914_400);
        img.tile = Some(TileInfo {
            tx: Emu::new(0),
            ty: Emu::new(0),
            sx: 0.5,
            sy: 0.5,
            flip: ImageFlip::Xy,
            align: "tl".to_string(),
        });
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(res.defs.contains("scale(-1, -1)"));
    }

    #[test]
    fn emf_renders_blank_placeholder_rect() {
        let mut img = png_image(1_828_800, 914_400);
        img.mime_type = "image/emf".to_string();
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        // EMF / WMF placeholders are intentionally silent now — no text
        // label, no icon — so the rest of the slide composes cleanly
        // around the unrendered metafile rather than a debug badge.
        assert!(res.content.contains("fill=\"#F5F5F5\""));
        assert!(!res.content.contains(">EMF</text>"));
        assert!(!res.content.contains("unsupported"));
    }

    #[test]
    fn wmf_renders_blank_placeholder_rect() {
        let mut img = png_image(1_828_800, 914_400);
        img.mime_type = "image/wmf".to_string();
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(res.content.contains("fill=\"#F5F5F5\""));
        assert!(!res.content.contains(">WMF</text>"));
    }

    #[test]
    fn placeholder_alt_text_is_silenced() {
        let mut img = png_image(1_905_000, 1_905_000);
        img.mime_type = "image/emf".to_string();
        img.alt_text = Some("Custom alt".to_string());
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        // alt text is no longer surfaced in-canvas; it's still
        // available on the model for `aria-label` injection upstream.
        assert!(!res.content.contains(">Custom alt</text>"));
    }

    #[test]
    fn compact_placeholder_omits_subtext_and_icon() {
        // 100×60 px box has min_dim=60 < 120, so compact branch fires.
        let mut img = png_image(952_500, 571_500);
        img.mime_type = "image/emf".to_string();
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(!res.content.contains("<svg"));
        assert!(!res.content.contains("unsupported"));
    }

    #[test]
    fn object_name_propagates_to_data_attribute() {
        let mut img = png_image(914_400, 914_400);
        img.object_name = Some("Image 1".to_string());
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(res.content.contains("data-object-name=\"Image 1\""));
    }

    #[test]
    fn alpha_lt_one_wraps_image_in_opacity_group() {
        let mut img = png_image(914_400, 914_400);
        img.alpha = 0.05;
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        // Alpha < 1.0 introduces an inner <g opacity="..."> wrapper around
        // the <image>. The outer wrapper still carries transform/data-sp-id
        // exactly as before.
        assert!(
            res.content.contains("<g opacity=\"0.05\">"),
            "expected opacity wrapper at 0.05, got: {}",
            res.content
        );
        assert!(res.content.contains("<image href=\"data:image/png"));
    }

    #[test]
    fn alpha_one_does_not_introduce_opacity_wrapper() {
        // Default alpha = 1.0 (fully opaque) — must keep the legacy SVG
        // shape, no extra <g> wrapper, so existing snapshots stay stable.
        let img = png_image(914_400, 914_400);
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(!res.content.contains("opacity="), "{}", res.content);
    }

    #[test]
    fn alpha_combines_with_blip_filter_on_same_inner_group() {
        use slideglance_model::BlipEffects;
        let mut img = png_image(914_400, 914_400);
        img.alpha = 0.5;
        img.blip_effects = Some(BlipEffects {
            grayscale: true,
            ..BlipEffects::default()
        });
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        // Both attributes attach to the inner <g> wrapping the <image>.
        assert!(
            res.content.contains("filter=\"url(#blip-effect-")
                && res.content.contains("opacity=\"0.5\""),
            "expected both attrs on inner group: {}",
            res.content
        );
    }

    #[test]
    fn alpha_lt_one_with_src_rect_still_emits_clip_path_image() {
        // `<a:srcRect>` runs through a different inner-image builder than
        // the plain stretch/no-stretch path. Make sure the alpha wrapper
        // still composes correctly when the inner element is a clipped
        // `<image>` rather than the plain one.
        let mut img = png_image(914_400, 914_400);
        img.alpha = 0.5;
        img.src_rect = Some(SrcRect {
            left: 0.1,
            top: 0.1,
            right: 0.1,
            bottom: 0.1,
        });
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(res.defs.contains("<clipPath id=\"crop-"), "{}", res.defs);
        assert!(
            res.content.contains("<g opacity=\"0.5\">")
                && res.content.contains("clip-path=\"url(#crop-"),
            "{}",
            res.content
        );
    }

    #[test]
    fn alpha_lt_one_on_tiled_image_attaches_to_inner_group() {
        let mut img = png_image(914_400, 914_400);
        img.alpha = 0.25;
        img.tile = Some(TileInfo {
            tx: Emu::new(0),
            ty: Emu::new(0),
            sx: 0.5,
            sy: 0.5,
            flip: ImageFlip::None,
            align: "tl".to_string(),
        });
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(
            res.content.contains("<g opacity=\"0.25\">"),
            "expected tile rect to be wrapped in opacity group: {}",
            res.content
        );
        assert!(res.content.contains("<rect"));
    }

    #[test]
    fn shape_effects_attach_to_outer_group() {
        use slideglance_color::{ResolvedColor, Rgb};
        use slideglance_model::{EffectList, OuterShadow};
        let mut img = png_image(914_400, 914_400);
        img.effects = Some(EffectList {
            outer_shadow: Some(OuterShadow {
                blur_radius: Emu::new(127_000),
                distance: Emu::new(50_800),
                direction: 90.0,
                color: ResolvedColor::new(Rgb::from_hex("#000000").unwrap(), 1.0),
                alignment: "ctr".to_string(),
                rotate_with_shape: false,
            }),
            ..EffectList::default()
        });
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(
            res.content.starts_with("<g transform=\"")
                && res.content.contains(" filter=\"url(#effect-"),
            "{}",
            res.content
        );
        assert!(res.defs.contains("<filter id=\"effect-"));
    }

    #[test]
    fn blip_effects_attach_to_inner_group_around_image() {
        use slideglance_model::BlipEffects;
        let mut img = png_image(914_400, 914_400);
        img.blip_effects = Some(BlipEffects {
            grayscale: true,
            ..BlipEffects::default()
        });
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        // The blip filter wraps the <image> tag in an inner <g>.
        assert!(
            res.content.contains("<g filter=\"url(#blip-effect-"),
            "inner blip group missing: {}",
            res.content
        );
        assert!(res.defs.contains("<filter id=\"blip-effect-"));
    }

    #[test]
    fn shape_effects_and_blip_effects_chain_independently() {
        use slideglance_color::{ResolvedColor, Rgb};
        use slideglance_model::{BlipEffects, EffectList, Glow};
        let mut img = png_image(914_400, 914_400);
        img.effects = Some(EffectList {
            glow: Some(Glow {
                radius: Emu::new(127_000),
                color: ResolvedColor::new(Rgb::from_hex("#FFFFFF").unwrap(), 1.0),
            }),
            ..EffectList::default()
        });
        img.blip_effects = Some(BlipEffects {
            grayscale: true,
            ..BlipEffects::default()
        });
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        // Outer <g> carries the shape filter; inner <g> wraps the
        // <image> with the blip filter.
        assert!(res.content.contains(" filter=\"url(#effect-"));
        assert!(res.content.contains("<g filter=\"url(#blip-effect-"));
        // Both filters end up in defs.
        assert!(res.defs.contains("<filter id=\"effect-"));
        assert!(res.defs.contains("<filter id=\"blip-effect-"));
    }

    #[test]
    fn tiled_image_with_blip_effect_wraps_rect_in_inner_group() {
        use slideglance_model::BlipEffects;
        let mut img = png_image(914_400, 914_400);
        img.tile = Some(TileInfo {
            tx: Emu::new(0),
            ty: Emu::new(0),
            sx: 0.5,
            sy: 0.5,
            flip: ImageFlip::None,
            align: "tl".to_string(),
        });
        img.blip_effects = Some(BlipEffects {
            grayscale: true,
            ..BlipEffects::default()
        });
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        // The tile rect should be wrapped in <g filter="url(#blip-effect-…)">.
        assert!(
            res.content.contains("<g filter=\"url(#blip-effect-"),
            "{}",
            res.content
        );
        assert!(res.content.contains("<rect"));
    }

    fn malicious_svg_b64(payload: &str) -> String {
        use base64::engine::general_purpose::STANDARD as B64;
        use base64::Engine as _;
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">{payload}</svg>"#
        );
        B64.encode(svg.as_bytes())
    }

    fn svg_image(w: i64, h: i64, b64: String) -> ImageElement {
        ImageElement {
            sp_id: None,
            transform: xfrm(w, h),
            image_data: b64,
            mime_type: "image/svg+xml".to_string(),
            effects: None,
            blip_effects: None,
            src_rect: None,
            alt_text: None,
            object_name: None,
            hidden: false,
            stretch: None,
            tile: None,
            alpha: 1.0,
        }
    }

    #[test]
    fn svg_xss_script_tag_absent_from_output() {
        let b64 = malicious_svg_b64("<script>alert(1)</script>");
        let img = svg_image(914_400, 914_400, b64);
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(
            !res.content.contains("<script>"),
            "script tag must not appear: {}",
            res.content
        );
    }

    #[test]
    fn svg_xss_onload_attr_absent_from_output() {
        let b64 = malicious_svg_b64(r#"<rect onload="alert(1)" width="100" height="100"/>"#);
        let img = svg_image(914_400, 914_400, b64);
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(
            !res.content.contains("onload="),
            "onload attr must not appear: {}",
            res.content
        );
    }

    #[test]
    fn svg_xss_javascript_href_absent_from_output() {
        let b64 = malicious_svg_b64(r#"<a href="javascript:alert(1)"><text>click</text></a>"#);
        let img = svg_image(914_400, 914_400, b64);
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(
            !res.content.contains("javascript:"),
            "javascript: href must not appear: {}",
            res.content
        );
    }

    #[test]
    fn svg_xss_data_html_href_absent_from_output() {
        let b64 = malicious_svg_b64(
            r#"<a xlink:href="data:text/html,<script>1</script>"><text>x</text></a>"#,
        );
        let img = svg_image(914_400, 914_400, b64);
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(
            !res.content.contains("data:text/html"),
            "data:text/html must not appear: {}",
            res.content
        );
    }

    #[test]
    fn sp_id_emitted_on_blip_image_outer_group() {
        // Plain (blip-fill) image path: the outermost <g> wrapper carries
        // data-sp-id so selection overlays can address the picture stably.
        let mut img = png_image(914_400, 914_400);
        img.sp_id = Some(7);
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(
            res.content.starts_with("<g data-sp-id=\"7\" transform=\""),
            "expected outer <g> to start with data-sp-id=\"7\": {}",
            res.content
        );
    }

    #[test]
    fn sp_id_emitted_on_tile_pattern_outer_group() {
        // Tile-pattern path: render_tiled() builds its own outer wrapper —
        // verify it picks up the same data-sp-id.
        let mut img = png_image(914_400, 914_400);
        img.sp_id = Some(13);
        img.tile = Some(TileInfo {
            tx: Emu::new(0),
            ty: Emu::new(0),
            sx: 0.5,
            sy: 0.5,
            flip: ImageFlip::None,
            align: "tl".to_string(),
        });
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(
            res.content.starts_with("<g data-sp-id=\"13\" transform=\""),
            "expected tile outer <g> to start with data-sp-id=\"13\": {}",
            res.content
        );
    }

    #[test]
    fn sp_id_emitted_on_emf_placeholder_group() {
        // Vector-only EMF (no extractable raster) hits render_placeholder().
        // The minimal AAAA payload is not a valid DIB-bearing EMF, so
        // try_rasterize_metafile returns None and the placeholder branch fires.
        let mut img = png_image(914_400, 914_400);
        img.sp_id = Some(99);
        img.mime_type = "image/emf".to_string();
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(
            res.content.starts_with("<g data-sp-id=\"99\" transform=\""),
            "expected placeholder <g> to start with data-sp-id=\"99\": {}",
            res.content
        );
        // Sanity-check we are actually on the placeholder branch.
        assert!(res.content.contains("fill=\"#F5F5F5\""));
    }

    #[test]
    fn benign_svg_renders_as_external_image_reference() {
        use base64::engine::general_purpose::STANDARD as B64;
        use base64::Engine as _;
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><circle cx="50" cy="50" r="40"/></svg>"#;
        let b64 = B64.encode(svg.as_bytes());
        let img = svg_image(914_400, 914_400, b64.clone());
        let mut ids = IdGen::new();
        let res = render_image(&img, &mut ids);
        assert!(
            res.content.contains("data:image/svg+xml;base64,"),
            "SVG must be emitted as external image: {}",
            res.content
        );
        assert!(
            !res.content.contains("<svg"),
            "SVG must NOT be inlined: {}",
            res.content
        );
        assert!(res.content.contains("width=\"96\""));
        assert!(res.content.contains("height=\"96\""));
    }
}
