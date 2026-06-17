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

use std::fmt::Write as _;

use slideglance_font::{CjkPlatform, FontMapping, FontResolver, ScriptFontContext, TextMeasurer};
use slideglance_model::{Slide, SlideElement, SlideHeaderFooter, SlideSize};

use crate::slide_context::Timestamp;

use crate::chart::render_chart;
use crate::connector::render_connector;
use crate::error::RendererError;
use crate::geometry::fmt::n;
use crate::id_gen::IdGen;
use crate::image::render_image;
use crate::render_result::RenderResult;
use crate::shape::render_shape;
use crate::slide_context::SlideRenderContext;
use crate::svg_builder::escape_xml_attr;
use crate::table::render_table;

mod background;
mod group;

/// Render one [`Slide`] to a single self-contained SVG document.
///
/// The output is a string starting with `<svg …>` and ending with
/// `</svg>`. No XML declaration is emitted (matches TS).
///
/// `total_slides` flows through to per-element field substitution
/// (`<a:fld type="slidenum">` etc.) via [`SlideRenderContext`]. Pass
/// `None` when rendering one slide in isolation; pass `Some(n)` to
/// expose `slidenum` correctly to text body fields.
///
/// # Errors
///
/// Propagates [`RendererError::NotImplemented`] from per-element
/// renderers (effects / spAutofit). The spec never errors here;
/// the Rust port chooses to surface unsupported features rather than
/// silently dropping them.
#[allow(clippy::too_many_arguments)]
pub fn render_slide_to_svg(
    slide: &Slide,
    slide_size: &SlideSize,
    total_slides: Option<u32>,
    script_fonts: &ScriptFontContext,
    measurer: &dyn TextMeasurer,
    mapping: &FontMapping,
    cjk_platform: CjkPlatform,
    font_resolver: Option<&dyn FontResolver>,
    timestamp: Option<Timestamp>,
) -> Result<String, RendererError> {
    let width_px = slide_size.width.to_pixels();
    let height_px = slide_size.height.to_pixels();

    let ctx = SlideRenderContext {
        slide_number: slide.slide_number,
        total_slides,
        header_footer: slide.header_footer.clone(),
        timestamp,
    };

    let mut ids = IdGen::new();
    let mut body = String::new();
    let mut defs = String::new();

    // Background.
    let bg_render =
        background::render_background(slide.background.as_ref(), width_px, height_px, &mut ids);
    body.push_str(&bg_render.content);
    defs.push_str(&bg_render.defs);

    // Top-level elements. Top-level shapes carry an identity group
    // correction (1.0) — only descendants of a `<p:grpSp>` with
    // chExt != ext pick up a non-trivial value via `render_group`.
    for element in &slide.elements {
        if should_skip_for_header_footer(element, slide.header_footer.as_ref()) {
            continue;
        }
        if element_hidden(element) {
            continue;
        }
        let Some(rendered) = render_element(
            element,
            &mut ids,
            &ctx,
            script_fonts,
            measurer,
            mapping,
            cjk_platform,
            font_resolver,
            1.0,
        )?
        else {
            continue;
        };
        body.push_str(&rendered.content);
        defs.push_str(&rendered.defs);
    }

    // Compose the SVG.
    let layout_attr = match &slide.layout_name {
        Some(name) => format!(" data-layout-name=\"{}\"", escape_xml_attr(name)),
        None => String::new(),
    };

    let mut out = String::with_capacity(body.len() + defs.len() + 256);
    let _ = write!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\"{layout_attr}>",
        n(width_px),
        n(height_px),
        n(width_px),
        n(height_px),
    );
    if !defs.is_empty() {
        out.push_str("<defs>");
        out.push_str(&defs);
        out.push_str("</defs>");
    }
    out.push_str(&body);
    out.push_str("</svg>");

    Ok(out)
}

// ---------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------

// ---------------------------------------------------------------------
// Element dispatch
// ---------------------------------------------------------------------

/// Dispatch one element to its renderer, applying alt-text and
/// hyperlink wrapping after the inner result is computed. Returns
/// `Ok(None)` when the element has no visual contribution (matching the
/// TS `default` branch return of `null`).
#[allow(clippy::too_many_arguments)]
pub(super) fn render_element(
    element: &SlideElement,
    ids: &mut IdGen,
    slide: &SlideRenderContext,
    script_fonts: &ScriptFontContext,
    measurer: &dyn TextMeasurer,
    mapping: &FontMapping,
    cjk_platform: CjkPlatform,
    font_resolver: Option<&dyn FontResolver>,
    // Carries the cumulative inverse of every ancestor `render_group`'s
    // SVG `transform="scale(...)"` factor through to the leaf renderers.
    // Only `render_shape` / `render_table` consume it (text bodies do);
    // image / chart / connector renderings ignore it. Top-level callers
    // pass `1.0`.
    font_size_correction: f64,
) -> Result<Option<RenderResult>, RendererError> {
    let mut result = match element {
        SlideElement::Shape(shape) => render_shape(
            shape,
            ids,
            slide,
            script_fonts,
            measurer,
            mapping,
            cjk_platform,
            font_resolver,
            font_size_correction,
        )?,
        SlideElement::Connector(conn) => render_connector(conn, ids, font_size_correction),
        SlideElement::Group(group) => group::render_group(
            group,
            ids,
            slide,
            script_fonts,
            measurer,
            mapping,
            cjk_platform,
            font_resolver,
            font_size_correction,
        )?,
        SlideElement::Image(img) => {
            let r = render_image(img, ids);
            RenderResult {
                content: r.content,
                defs: r.defs,
            }
        }
        SlideElement::Chart(chart) => {
            let r = render_chart(chart);
            RenderResult {
                content: r.content,
                defs: r.defs,
            }
        }
        SlideElement::Table(table) => {
            let r = render_table(
                table,
                ids,
                slide,
                script_fonts,
                measurer,
                mapping,
                cjk_platform,
                font_resolver,
                font_size_correction,
            );
            RenderResult {
                content: r.content,
                defs: r.defs,
            }
        }
    };

    // Alt-text injection (role="img" aria-label="…"). Only the variants
    // that actually carry `alt_text` participate.
    let alt_text = match element {
        SlideElement::Shape(s) => s.alt_text.as_deref(),
        SlideElement::Connector(c) => c.alt_text.as_deref(),
        SlideElement::Group(g) => g.alt_text.as_deref(),
        SlideElement::Image(i) => i.alt_text.as_deref(),
        // Chart/Table have no alt_text in the model.
        SlideElement::Chart(_) | SlideElement::Table(_) => None,
    };
    if let Some(text) = alt_text {
        if !text.is_empty() {
            result.content = add_aria_label(&result.content, text);
        }
    }

    // Hyperlink wrapping — only ShapeElement carries `hyperlink` in the
    // model (matches TS structural "hyperlink" in element check).
    if let SlideElement::Shape(s) = element {
        if let Some(link) = &s.hyperlink {
            let href = escape_xml_attr(&link.url);
            result.content = format!("<a href=\"{href}\">{}</a>", result.content);
        }
    }

    Ok(Some(result))
}

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

/// Skip slide-number / datetime / footer placeholders when the slide's
/// `<p:hf>` toggles disable them.
fn should_skip_for_header_footer(element: &SlideElement, hf: Option<&SlideHeaderFooter>) -> bool {
    let Some(hf) = hf else {
        return false;
    };
    let SlideElement::Shape(shape) = element else {
        return false;
    };
    let Some(ph) = shape.placeholder_type.as_deref() else {
        return false;
    };
    if ph == "sldNum" && !hf.show_slide_number {
        return true;
    }
    if ph == "dt" && !hf.show_date_time {
        return true;
    }
    if ph == "ftr" && !hf.show_footer {
        return true;
    }
    false
}

pub(super) fn element_hidden(element: &SlideElement) -> bool {
    match element {
        SlideElement::Shape(s) => s.hidden,
        SlideElement::Connector(c) => c.hidden,
        SlideElement::Group(g) => g.hidden,
        // Chart/Image/Table have no `hidden` flag in the model.
        SlideElement::Chart(_) | SlideElement::Image(_) | SlideElement::Table(_) => false,
    }
}

/// Inject `role="img" aria-label="…"` after the first `<g`, `<image`,
/// or `<path` opening tag's name. Mirrors 's
/// regex `/^<(g|image|path)\b/`.
fn add_aria_label(svg: &str, alt_text: &str) -> String {
    let escaped = escape_xml_attr(alt_text);
    for tag in &["g", "image", "path"] {
        let prefix = format!("<{tag}");
        if let Some(rest) = svg.strip_prefix(prefix.as_str()) {
            // Word boundary: next char must be non-word
            // (whitespace / `>` / `/`).
            if let Some(c) = rest.chars().next() {
                if !c.is_ascii_alphanumeric() && c != '_' {
                    return format!("<{tag} role=\"img\" aria-label=\"{escaped}\"{rest}");
                }
            }
        }
    }
    svg.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_color::{ResolvedColor, Rgb};
    use slideglance_font::{FontMapping, HeuristicTextMeasurer, ScriptFontContext};
    use slideglance_model::{
        Background, ConnectorElement, Fill, Geometry, GroupElement, Hyperlink, ImageFill,
        ImageRect, NoFill, PresetGeometry, ShapeElement, Slide, SlideElement, SlideHeaderFooter,
        SlideSize, SolidFill, Transform,
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

    fn basic_shape(hidden: bool, alt: Option<&str>, hyperlink: Option<&str>) -> ShapeElement {
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
            alt_text: alt.map(str::to_string),
            object_name: None,
            hidden,
            hyperlink: hyperlink.map(|url| Hyperlink {
                url: url.to_string(),
                tooltip: None,
            }),
        }
    }

    fn placeholder_shape(ty: &str) -> ShapeElement {
        let mut s = basic_shape(false, None, None);
        s.placeholder_type = Some(ty.to_string());
        s
    }

    fn slide_with(elements: Vec<SlideElement>) -> Slide {
        Slide {
            slide_number: 1,
            background: None,
            elements,
            show_master_sp: true,
            header_footer: None,
            notes: None,
            layout_name: None,
        }
    }

    fn small_size() -> SlideSize {
        SlideSize {
            width: Emu::new(914_400 * 10),
            height: Emu::new(914_400 * 7),
        }
    }

    fn render(slide: &Slide) -> Result<String, RendererError> {
        render_slide_to_svg(
            slide,
            &small_size(),
            None,
            &ScriptFontContext::empty(),
            &HeuristicTextMeasurer,
            &FontMapping::new(),
            CjkPlatform::Other,
            None,
            None,
        )
    }

    // --- top-level shape ---

    #[test]
    fn empty_slide_emits_default_white_background() {
        let svg = render(&slide_with(vec![])).unwrap();
        assert!(svg.starts_with("<svg "), "{svg}");
        assert!(svg.contains("viewBox=\"0 0 "));
        assert!(svg.contains("fill=\"#FFFFFF\""));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn defs_block_appears_only_when_needed() {
        // No elements, no fills with defs → no <defs> block.
        let svg = render(&slide_with(vec![])).unwrap();
        assert!(!svg.contains("<defs>"));
    }

    #[test]
    fn layout_name_emitted_as_data_attribute_on_svg() {
        let mut s = slide_with(vec![]);
        s.layout_name = Some("Title Slide".to_string());
        let svg = render(&s).unwrap();
        assert!(svg.contains("data-layout-name=\"Title Slide\""), "{svg}");
    }

    // --- backgrounds ---

    #[test]
    fn explicit_no_fill_background_renders_fill_none_rect() {
        // `<a:noFill/>` on the background block (Background.fill =
        // Some(Fill::None)) is distinct from "no background block at
        // all" (Slide.background = None). TS's renderFillAttrs maps
        // Fill::None to `fill="none"`, so the slide ends up with a
        // transparent rect rather than the default white default.
        let mut s = slide_with(vec![]);
        s.background = Some(Background {
            fill: Some(Fill::None(NoFill {})),
        });
        let svg = render(&s).unwrap();
        assert!(svg.contains("<rect width=\""));
        assert!(svg.contains("fill=\"none\""), "{svg}");
    }

    #[test]
    fn missing_fill_block_renders_default_white_rect() {
        // Background.fill = None means the slide had a `<p:bg>` block
        // with no recognised fill — TS treats this the same as no
        // background block: emit the default white rect.
        let mut s = slide_with(vec![]);
        s.background = Some(Background { fill: None });
        let svg = render(&s).unwrap();
        assert!(svg.contains("fill=\"#FFFFFF\""), "{svg}");
    }

    #[test]
    fn solid_background_uses_fill_attribute() {
        let mut s = slide_with(vec![]);
        s.background = Some(Background {
            fill: Some(Fill::Solid(SolidFill {
                color: opaque("#123456"),
            })),
        });
        let svg = render(&s).unwrap();
        assert!(svg.contains("fill=\"#123456\""));
    }

    #[test]
    fn image_background_plain_stretch_emits_image_tag() {
        let mut s = slide_with(vec![]);
        s.background = Some(Background {
            fill: Some(Fill::Image(ImageFill {
                image_data: "AA".to_string(),
                mime_type: "image/png".to_string(),
                tile: None,
                src_rect: None,
                stretch: None,
                alpha: 1.0,
            })),
        });
        let svg = render(&s).unwrap();
        assert!(svg.contains("<image href=\"data:image/png;base64,AA\""));
        assert!(svg.contains("preserveAspectRatio=\"none\""));
    }

    #[test]
    fn image_background_with_src_rect_clips_via_clip_path() {
        let mut s = slide_with(vec![]);
        s.background = Some(Background {
            fill: Some(Fill::Image(ImageFill {
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
            })),
        });
        let svg = render(&s).unwrap();
        assert!(svg.contains("<clipPath id=\"bg-crop-"), "{svg}");
        assert!(svg.contains("clip-path=\"url(#bg-crop-"));
    }

    #[test]
    fn image_background_with_stretch_inset_offsets_image() {
        let mut s = slide_with(vec![]);
        s.background = Some(Background {
            fill: Some(Fill::Image(ImageFill {
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
            })),
        });
        let svg = render(&s).unwrap();
        // Expect a positioned <image> (x="…" y="…") rather than the
        // plain stretch form that omits x/y.
        assert!(svg.contains("<image href="));
        assert!(
            svg.contains(" x=\""),
            "stretch did not produce x attr: {svg}"
        );
    }

    #[test]
    fn image_background_degenerate_src_rect_falls_back_to_white_rect() {
        let mut s = slide_with(vec![]);
        s.background = Some(Background {
            fill: Some(Fill::Image(ImageFill {
                image_data: "AA".to_string(),
                mime_type: "image/png".to_string(),
                tile: None,
                src_rect: Some(ImageRect {
                    left: 0.6,
                    top: 0.0,
                    right: 0.6,
                    bottom: 0.0,
                }),
                stretch: None,
                alpha: 1.0,
            })),
        });
        let svg = render(&s).unwrap();
        assert!(svg.contains("fill=\"#FFFFFF\""));
        assert!(!svg.contains("<image href="));
    }

    // --- header/footer ---

    #[test]
    fn slide_number_placeholder_skipped_when_disabled() {
        let mut s = slide_with(vec![SlideElement::Shape(placeholder_shape("sldNum"))]);
        s.header_footer = Some(SlideHeaderFooter {
            show_slide_number: false,
            show_date_time: true,
            show_footer: true,
            footer_text: None,
            datetime_text: None,
        });
        let svg = render(&s).unwrap();
        // Only the default white background should remain — no shape
        // group emitted for the placeholder.
        let group_count = svg.matches("<g transform=\"").count();
        assert_eq!(group_count, 0, "{svg}");
    }

    #[test]
    fn datetime_placeholder_renders_when_enabled() {
        let mut s = slide_with(vec![SlideElement::Shape(placeholder_shape("dt"))]);
        s.header_footer = Some(SlideHeaderFooter {
            show_slide_number: true,
            show_date_time: true,
            show_footer: true,
            footer_text: None,
            datetime_text: None,
        });
        let svg = render(&s).unwrap();
        assert!(svg.contains("<g transform=\""), "{svg}");
    }

    #[test]
    fn placeholder_renders_when_no_header_footer_block() {
        // No `<p:hf>` block on the slide → all placeholders pass.
        let s = slide_with(vec![SlideElement::Shape(placeholder_shape("sldNum"))]);
        let svg = render(&s).unwrap();
        assert!(svg.contains("<g transform=\""));
    }

    // --- alt-text ---

    #[test]
    fn alt_text_injects_aria_label_after_first_g() {
        let s = slide_with(vec![SlideElement::Shape(basic_shape(
            false,
            Some("hero illustration"),
            None,
        ))]);
        let svg = render(&s).unwrap();
        assert!(
            svg.contains("<g role=\"img\" aria-label=\"hero illustration\" transform=\""),
            "{svg}"
        );
    }

    #[test]
    fn alt_text_escapes_xml_special_characters() {
        let s = slide_with(vec![SlideElement::Shape(basic_shape(
            false,
            Some("X & <Y>"),
            None,
        ))]);
        let svg = render(&s).unwrap();
        assert!(svg.contains("aria-label=\"X &amp; &lt;Y&gt;\""), "{svg}");
    }

    #[test]
    fn empty_alt_text_does_not_inject_aria_label() {
        let s = slide_with(vec![SlideElement::Shape(basic_shape(
            false,
            Some(""),
            None,
        ))]);
        let svg = render(&s).unwrap();
        assert!(!svg.contains("aria-label="));
    }

    // --- hyperlink ---

    #[test]
    fn hyperlink_wraps_shape_in_anchor_tag() {
        let s = slide_with(vec![SlideElement::Shape(basic_shape(
            false,
            None,
            Some("https://example.com"),
        ))]);
        let svg = render(&s).unwrap();
        assert!(
            svg.contains("<a href=\"https://example.com\"><g transform=\""),
            "{svg}"
        );
        assert!(svg.contains("</g></a>"), "{svg}");
    }

    #[test]
    fn hyperlink_url_is_escaped() {
        let s = slide_with(vec![SlideElement::Shape(basic_shape(
            false,
            None,
            Some("https://example.com/?x=1&y=2"),
        ))]);
        let svg = render(&s).unwrap();
        assert!(svg.contains("href=\"https://example.com/?x=1&amp;y=2\""));
    }

    // --- hidden ---

    #[test]
    fn hidden_top_level_shape_is_not_rendered() {
        let s = slide_with(vec![SlideElement::Shape(basic_shape(true, None, None))]);
        let svg = render(&s).unwrap();
        let group_count = svg.matches("<g transform=\"").count();
        assert_eq!(group_count, 0);
    }

    // --- group ---

    #[test]
    fn empty_group_renders_outer_g_only() {
        let group = GroupElement {
            sp_id: None,
            transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(914_400),
                extent_height: Emu::new(914_400),
                ..Transform::default()
            },
            child_transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(914_400),
                extent_height: Emu::new(914_400),
                ..Transform::default()
            },
            children: Vec::new(),
            effects: None,
            alt_text: None,
            object_name: None,
            hidden: false,
        };
        let s = slide_with(vec![SlideElement::Group(group)]);
        let svg = render(&s).unwrap();
        // Outer <g> is the group's own translate/scale wrapper.
        assert!(svg.contains("translate("));
        assert!(svg.contains("scale("));
        // sp_id is None, so the attribute must be absent on the group <g>.
        assert!(!svg.contains("data-sp-id"), "{svg}");
    }

    #[test]
    fn group_emits_data_sp_id_when_present() {
        let group = GroupElement {
            sp_id: Some(99),
            transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(914_400),
                extent_height: Emu::new(914_400),
                ..Transform::default()
            },
            child_transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(914_400),
                extent_height: Emu::new(914_400),
                ..Transform::default()
            },
            children: Vec::new(),
            effects: None,
            alt_text: None,
            object_name: None,
            hidden: false,
        };
        let s = slide_with(vec![SlideElement::Group(group)]);
        let svg = render(&s).unwrap();
        assert!(
            svg.contains("<g data-sp-id=\"99\" transform=\""),
            "data-sp-id missing or in wrong position: {svg}"
        );
    }

    #[test]
    fn group_recurses_into_children() {
        let group = GroupElement {
            sp_id: None,
            transform: Transform {
                offset_x: Emu::new(914_400),
                offset_y: Emu::new(0),
                extent_width: Emu::new(914_400),
                extent_height: Emu::new(914_400),
                ..Transform::default()
            },
            child_transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(914_400),
                extent_height: Emu::new(914_400),
                ..Transform::default()
            },
            children: vec![SlideElement::Shape(basic_shape(false, None, None))],
            effects: None,
            alt_text: None,
            object_name: None,
            hidden: false,
        };
        let s = slide_with(vec![SlideElement::Group(group)]);
        let svg = render(&s).unwrap();
        // The child shape's red fill should appear inside the wrapper.
        assert!(svg.contains("fill=\"#FF0000\""), "{svg}");
    }

    #[test]
    fn group_skips_hidden_children() {
        let group = GroupElement {
            sp_id: None,
            transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(914_400),
                extent_height: Emu::new(914_400),
                ..Transform::default()
            },
            child_transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(914_400),
                extent_height: Emu::new(914_400),
                ..Transform::default()
            },
            children: vec![SlideElement::Shape(basic_shape(true, None, None))],
            effects: None,
            alt_text: None,
            object_name: None,
            hidden: false,
        };
        let s = slide_with(vec![SlideElement::Group(group)]);
        let svg = render(&s).unwrap();
        assert!(!svg.contains("fill=\"#FF0000\""));
    }

    #[test]
    fn group_flip_emits_scale_minus_one() {
        let t = Transform {
            offset_x: Emu::new(0),
            offset_y: Emu::new(0),
            extent_width: Emu::new(914_400),
            extent_height: Emu::new(914_400),
            flip_h: true,
            ..Transform::default()
        };
        let group = GroupElement {
            sp_id: None,
            transform: t,
            child_transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(914_400),
                extent_height: Emu::new(914_400),
                ..Transform::default()
            },
            children: Vec::new(),
            effects: None,
            alt_text: None,
            object_name: None,
            hidden: false,
        };
        let s = slide_with(vec![SlideElement::Group(group)]);
        let svg = render(&s).unwrap();
        assert!(svg.contains("scale(-1, 1)"), "{svg}");
    }

    // --- connector dispatch ---

    #[test]
    fn connector_renders_via_dispatcher() {
        let conn = ConnectorElement {
            sp_id: None,
            transform: Transform {
                offset_x: Emu::new(0),
                offset_y: Emu::new(0),
                extent_width: Emu::new(914_400),
                extent_height: Emu::new(914_400),
                ..Transform::default()
            },
            geometry: Geometry::Preset(PresetGeometry {
                preset: "line".to_string(),
                adjust_values: BTreeMap::new(),
            }),
            outline: None,
            effects: None,
            alt_text: None,
            object_name: None,
            hidden: false,
        };
        let s = slide_with(vec![SlideElement::Connector(conn)]);
        let svg = render(&s).unwrap();
        assert!(svg.contains("<g transform=\""), "{svg}");
    }

    // --- error propagation ---

    #[test]
    fn shape_with_sp_autofit_no_longer_errors() {
        // spAutofit is now wired through compute_sp_autofit_height — an
        // empty body produces no growth and the slide renders without
        // error. This test guards against accidentally re-introducing
        // the old NotImplemented branch.
        use slideglance_model::{AutoFit, BodyProperties, TextBody, TextVerticalType, WrapMode};
        let body_props = BodyProperties {
            anchor: slideglance_model::VerticalAnchor::T,
            margin_left: Emu::new(0),
            margin_right: Emu::new(0),
            margin_top: Emu::new(0),
            margin_bottom: Emu::new(0),
            wrap: WrapMode::Square,
            auto_fit: AutoFit::SpAutofit,
            font_scale: 1.0,
            ln_spc_reduction: 0.0,
            num_col: 1,
            vert: TextVerticalType::Horz,
            spc_first_last_para: false,
            compat_ln_spc: false,
            prst_tx_warp: None,
        };
        let mut shape = basic_shape(false, None, None);
        shape.text_body = Some(TextBody {
            default_text_color: None,
            body_properties: body_props,
            paragraphs: Vec::new(),
        });
        let s = slide_with(vec![SlideElement::Shape(shape)]);
        let _ = render(&s).expect("spAutofit must no longer error");
    }

    #[test]
    fn shape_with_effects_emits_filter_via_dispatcher() {
        use slideglance_model::{EffectList, OuterShadow};
        let mut shape = basic_shape(false, None, None);
        shape.effects = Some(EffectList {
            outer_shadow: Some(OuterShadow {
                blur_radius: Emu::new(127_000),
                distance: Emu::new(50_800),
                direction: 90.0,
                color: opaque("#000000"),
                alignment: "ctr".to_string(),
                rotate_with_shape: false,
            }),
            ..EffectList::default()
        });
        let s = slide_with(vec![SlideElement::Shape(shape)]);
        let svg = render(&s).unwrap();
        assert!(svg.contains("<filter id=\"effect-"), "{svg}");
        assert!(svg.contains(" filter=\"url(#effect-"));
    }

    // --- helper ---

    #[test]
    fn add_aria_label_no_op_for_unknown_first_tag() {
        let svg = "<rect fill=\"red\"/>";
        let out = add_aria_label(svg, "hi");
        assert_eq!(out, svg);
    }

    #[test]
    fn add_aria_label_inserts_after_g() {
        let svg = "<g transform=\"translate(0,0)\"></g>";
        let out = add_aria_label(svg, "hi");
        assert!(
            out.starts_with("<g role=\"img\" aria-label=\"hi\" "),
            "{out}"
        );
    }

    #[test]
    fn add_aria_label_does_not_match_word_boundary_violation() {
        // `<group>` should not be matched as `<g\b` — the `\b` in
        // `/^<(g|image|path)\b/` prevents the prefix from matching when
        // the next char is alphanumeric.
        let svg = "<group attr=\"x\"></group>";
        let out = add_aria_label(svg, "hi");
        assert_eq!(out, svg);
    }

    // --- viewBox ---

    #[test]
    fn view_box_uses_pixel_units() {
        let s = slide_with(vec![]);
        let svg = render(&s).unwrap();
        // 10*914400 EMU width = 960 px @ 96 DPI; 7*914400 = 672 px.
        assert!(svg.contains("viewBox=\"0 0 960 672\""), "{svg}");
        assert!(svg.contains("width=\"960\""));
        assert!(svg.contains("height=\"672\""));
    }
}
