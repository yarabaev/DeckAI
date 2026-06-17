//! Background fills + image-fills for the slide stage.
//!
//! Extracted from `slide.rs` so the dispatcher stays focused on element
//! routing. Both helpers share the same fill / blip-effect plumbing
//! that shapes also use, but they emit a full-stage `<rect>` instead
//! of a per-shape one.

//! Slide-level rendering — the public end-to-end entry point.
//!
//! Direct port of. Builds
//! one SVG document per slide:
//!
//! 1. Open `<svg viewBox="0 0 W H" …>` (with optional `data-layout-name`).
//! 2. Render the background (solid / gradient / pattern / image / default
//!    white) into an early `<rect>` (or `<image>` for image backgrounds).
//! 3. Walk every top-level element, skipping placeholders that the slide's
//!    `<p:hf>` toggles disable, then dispatch to per-element renderers.
//! 4. Wrap each result with optional alt-text (`role="img" aria-label="…"`)
//!    and hyperlink (`<a href="…">…</a>`) decorations.
//! 5. Inject the accumulated `<defs>` block right after the opening
//!    `<svg>` tag and close.
//!
//! Group elements recurse through [`render_group`] which dispatches each
//! child back through [`render_element`].
//!
//! Effects, image-background `srcRect` cropping, and `spAutofit` mirror
//! the TS contract; per-feature `NotImplemented` errors only surface for
//! cases the TS itself implements (effects / spAutofit) — TS-equivalent
//! background-image variants are all wired up.

use slideglance_model::{Background, Fill, ImageFill};

use crate::fill::render_fill_attrs;
use crate::geometry::fmt::n;
use crate::id_gen::IdGen;
use crate::render_result::RenderResult;

/// Render the slide background — the early `<rect>` (or `<image>`)
/// that paints behind every shape on the slide. Returns a
/// [`RenderResult`] so the caller can push both the visible content
/// and any defs (gradient stops, image-fill `<pattern>`) into the
/// outer `<svg>`.
pub(super) fn render_background(
    background: Option<&Background>,
    width: f64,
    height: f64,
    ids: &mut IdGen,
) -> RenderResult {
    let Some(bg) = background else {
        return RenderResult {
            content: format!(
                "<rect width=\"{}\" height=\"{}\" fill=\"#FFFFFF\"/>",
                n(width),
                n(height)
            ),
            defs: String::new(),
        };
    };
    let Some(fill) = &bg.fill else {
        // Explicit `<a:noFill/>` background — TS treats this the same as
        // no background block (renders default white).
        return RenderResult {
            content: format!(
                "<rect width=\"{}\" height=\"{}\" fill=\"#FFFFFF\"/>",
                n(width),
                n(height)
            ),
            defs: String::new(),
        };
    };

    if let Fill::Image(img) = fill {
        return render_background_image(img, width, height, ids);
    }

    let fill_result = render_fill_attrs(Some(fill), ids);
    RenderResult {
        content: format!(
            "<rect width=\"{}\" height=\"{}\" {}/>",
            n(width),
            n(height),
            fill_result.attrs
        ),
        defs: fill_result.defs,
    }
}

/// Image background mirrors — independently
/// honors `srcRect` (crop on the source) and `stretch.fillRect`
/// (position/scale on the slide).
pub(super) fn render_background_image(
    bg: &ImageFill,
    width: f64,
    height: f64,
    ids: &mut IdGen,
) -> RenderResult {
    let has_src_rect = bg.src_rect.is_some();
    let has_stretch_inset = bg
        .stretch
        .is_some_and(|s| s.left != 0.0 || s.top != 0.0 || s.right != 0.0 || s.bottom != 0.0);

    if !has_src_rect && !has_stretch_inset {
        return RenderResult {
            content: format!(
                "<image href=\"data:{};base64,{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\"/>",
                bg.mime_type,
                bg.image_data,
                n(width),
                n(height)
            ),
            defs: String::new(),
        };
    }

    let s = bg.stretch.unwrap_or_default();
    let fl = s.left;
    let ft = s.top;
    let fr = s.right;
    let fb = s.bottom;
    let draw_x = width * fl;
    let draw_y = height * ft;
    let draw_w = width * (1.0 - fl - fr);
    let draw_h = height * (1.0 - ft - fb);

    if let Some(src) = bg.src_rect {
        let sl = src.left;
        let st = src.top;
        let sr = src.right;
        let sb = src.bottom;
        let visible_x = 1.0 - sl - sr;
        let visible_y = 1.0 - st - sb;
        if visible_x <= 0.0 || visible_y <= 0.0 {
            // Degenerate crop — match TS by emitting only a white
            // rectangle for the visible draw area.
            return RenderResult {
                content: format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#FFFFFF\"/>",
                    n(draw_x),
                    n(draw_y),
                    n(draw_w),
                    n(draw_h)
                ),
                defs: String::new(),
            };
        }
        let scaled_w = draw_w / visible_x;
        let scaled_h = draw_h / visible_y;
        let img_x = draw_x - sl * scaled_w;
        let img_y = draw_y - st * scaled_h;
        let clip_id = ids.next_id("bg-crop");
        let defs = format!(
            "<clipPath id=\"{clip_id}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath>",
            n(draw_x),
            n(draw_y),
            n(draw_w),
            n(draw_h),
        );
        let content = format!(
            "<image clip-path=\"url(#{clip_id})\" href=\"data:{};base64,{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\"/>",
            bg.mime_type,
            bg.image_data,
            n(img_x),
            n(img_y),
            n(scaled_w),
            n(scaled_h),
        );
        return RenderResult { content, defs };
    }

    // No srcRect, only fillRect inset — emit positioned image.
    RenderResult {
        content: format!(
            "<image href=\"data:{};base64,{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\"/>",
            bg.mime_type,
            bg.image_data,
            n(draw_x),
            n(draw_y),
            n(draw_w),
            n(draw_h),
        ),
        defs: String::new(),
    }
}
