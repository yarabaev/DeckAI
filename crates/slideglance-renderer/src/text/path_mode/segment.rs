//! Path-mode helpers — segment emission + measurement.
//!
//! Extracted from `path_mode.rs` so the main entry stays focused on
//! the paragraph -> wrapped-line dispatch and emission helpers can
//! be reviewed independently.

use std::fmt::Write as _;
use std::sync::Arc;

use slideglance_font::{
    text_to_svg_path_kerned, text_to_svg_path_with_precision, CjkPlatform, FontFace, FontMapping,
    FontResolver, ScriptFontContext, TextMeasurer,
};
use slideglance_model::{RunProperties, TextVerticalType};

use crate::color::color_hex;
use crate::svg_builder::escape_xml_attr;
use crate::text::layout::PX_PER_PT;
use crate::text::script::split_by_script;

use super::helpers::{needs_script_split, resolve_face};
use super::run_attrs::{build_path_attrs, render_text_decorations, should_synthesize_bold};
use super::SegmentEmitCtx;
use super::DECIMAL_PLACES;
use super::LETTER_SPACING_TRICK_PAD_EM;

/// Result of rendering a single text segment as `<path>` data.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SegmentPath {
    /// The emitted SVG fragment (`<path>` plus any decoration / highlight
    /// elements).
    pub svg: String,
    /// The total advance width of the rendered segment, in pixels.
    pub width: f64,
}

/// Render one text segment (run + script slice) to its `<path>` /
/// decoration tuple. Returns the resulting [`SegmentPath`].
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
#[must_use]
pub fn render_segment_as_path(
    text: &str,
    props: &RunProperties,
    x: f64,
    y: f64,
    font_scale: f64,
    default_font_size: f64,
    font_resolver: &dyn FontResolver,
    script_fonts: &ScriptFontContext,
    measurer: &dyn TextMeasurer,
    _mapping: &FontMapping,
    _cjk_platform: CjkPlatform,
    vert: TextVerticalType,
) -> SegmentPath {
    let font_size = props
        .font_size
        .map_or(default_font_size, slideglance_utils::Pt::raw)
        * font_scale;
    let font_size_px = font_size * PX_PER_PT;
    let processed = text.replace('\t', "    ");

    let y_offset = if props.baseline > 0.0 {
        -font_size_px * 0.35
    } else if props.baseline < 0.0 {
        font_size_px * 0.2
    } else {
        0.0
    };
    let effective_y = y + y_offset;
    let jpan_fallback = script_fonts.jpan_fallback();

    let mut out = String::new();
    let mut total_width = 0.0_f64;

    let ctx = SegmentEmitCtx {
        props,
        x,
        effective_y,
        font_size,
        font_size_px,
        font_resolver,
        measurer,
        jpan_fallback,
    };

    // Letter-spacing trick boost (e.g. slide-2 title "평 가 항 목  조 견
    // 표"). When the *whole run* is the EA-space-EA pattern, PowerPoint
    // widens each space ~0.2 em above the font's nominal advance. We
    // apply this boost when ANY of the dispatched paths below is taken
    // by always script-splitting and inserting per-space padding. The
    // matching extra is summed in `measure_line_width` so center / right
    // alignment uses the same total width.
    let trick_boost_per_space = if is_letter_spacing_trick(&processed) {
        ctx.font_size_px * LETTER_SPACING_TRICK_PAD_EM
    } else {
        0.0
    };
    let apply_trick = trick_boost_per_space > 0.0;

    if matches!(vert, TextVerticalType::EaVert) || apply_trick || needs_script_split(props) {
        let parts = split_by_script(&processed);
        let is_ea_vert = matches!(vert, TextVerticalType::EaVert);
        for part in &parts {
            let part_is_ea = part.script.is_ea();
            let (ff, ff_ea) = if is_ea_vert {
                if part_is_ea {
                    (
                        props
                            .font_family_ea
                            .as_deref()
                            .or(props.font_family.as_deref()),
                        props.font_family_ea.as_deref(),
                    )
                } else {
                    (
                        props.font_family.as_deref(),
                        props.font_family_ea.as_deref(),
                    )
                }
            } else if part_is_ea {
                (
                    props.font_family_ea.as_deref(),
                    props.font_family.as_deref(),
                )
            } else {
                (
                    props.font_family.as_deref(),
                    props.font_family_ea.as_deref(),
                )
            };
            // Trick boost: if this part is space-only and we're in
            // trick mode, distribute boost half-before / half-after the
            // rendered space glyph so center alignment stays optical.
            let part_boost = if apply_trick && !part_is_ea && part.text.chars().all(|c| c == ' ') {
                let n = part.text.chars().count() as f64;
                trick_boost_per_space * n
            } else {
                0.0
            };
            total_width += part_boost / 2.0;
            if part_is_ea && is_ea_vert {
                emit_cjk_upright(&ctx, &part.text, ff, ff_ea, &mut out, &mut total_width);
            } else {
                emit_segment(&ctx, &part.text, ff, ff_ea, &mut out, &mut total_width);
            }
            total_width += part_boost / 2.0;
        }
    } else {
        emit_segment(
            &ctx,
            &processed,
            props.font_family.as_deref(),
            props.font_family_ea.as_deref(),
            &mut out,
            &mut total_width,
        );
    }

    let svg = if let Some(link) = &props.hyperlink {
        if out.is_empty() {
            String::new()
        } else {
            let href = escape_xml_attr(&link.url);
            format!("<a href=\"{href}\">{out}</a>")
        }
    } else {
        out
    };
    SegmentPath {
        svg,
        width: total_width,
    }
}

pub(super) fn emit_segment(
    ctx: &SegmentEmitCtx,
    seg_text: &str,
    font_family: Option<&str>,
    font_family_ea: Option<&str>,
    out: &mut String,
    total_width: &mut f64,
) {
    if seg_text.is_empty() {
        return;
    }
    let (font, italic_applied) = resolve_face(
        ctx.font_resolver,
        ctx.props,
        font_family,
        font_family_ea,
        ctx.jpan_fallback,
    );
    // Width measurement always uses the run's *original* family pair so
    // script-split swaps don't change advance widths.
    let seg_style = slideglance_font::FontStyle {
        bold: ctx.props.bold,
        italic: ctx.props.italic,
    };
    let seg_width = ctx.measurer.measure_text_width(
        seg_text,
        ctx.font_size,
        seg_style,
        ctx.props.font_family.as_deref(),
        ctx.props.font_family_ea.as_deref(),
    );
    let need_skew = ctx.props.italic && !italic_applied;

    if let Some(highlight) = &ctx.props.highlight {
        let bg_y = ctx.effective_y - ctx.font_size_px * 0.85;
        let bg_h = ctx.font_size_px * 1.1;
        let alpha_attr = if highlight.alpha < 1.0 {
            format!(" fill-opacity=\"{:.3}\"", highlight.alpha)
        } else {
            String::new()
        };
        let _ = write!(
            out,
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"{alpha_attr}/>",
            ctx.x + *total_width,
            bg_y,
            seg_width,
            bg_h,
            color_hex(highlight)
        );
    }

    if let Some(face) = font.as_ref() {
        // Apply kern-table adjustments when RunProperties.kern is set and
        // the current font size meets the threshold (OOXML kern attribute is
        // in hundredths of a point; threshold/100 gives the minimum pt size).
        let apply_kern = ctx
            .props
            .kern
            .is_some_and(|t| ctx.font_size >= t.to_points().raw());
        let path_data = text_to_svg_path_kerned(
            face,
            seg_text,
            ctx.x + *total_width,
            ctx.effective_y,
            ctx.font_size,
            DECIMAL_PLACES,
            apply_kern,
        );
        if !path_data.is_empty() {
            let synth_bold = should_synthesize_bold(ctx.props, font.as_deref());
            let fill_attrs = build_path_attrs(ctx.props, ctx.font_size_px, synth_bold);
            if need_skew {
                // Synthesize italic via 12° skew when the font lacks an
                // italic face. The translate compensates for the y-axis
                // shift skewX(-12) introduces at the glyph baseline.
                let dx = 0.2126 * ctx.effective_y;
                let _ = write!(
                    out,
                    "<path d=\"{path_data}\" {fill_attrs} transform=\"translate({dx:.2} 0) skewX(-12)\"/>"
                );
            } else {
                let _ = write!(out, "<path d=\"{path_data}\" {fill_attrs}/>");
            }
        }
    }

    if ctx.props.underline || ctx.props.strikethrough {
        out.push_str(&render_text_decorations(
            ctx.x + *total_width,
            ctx.effective_y,
            seg_width,
            ctx.font_size_px,
            ctx.props,
        ));
    }
    *total_width += seg_width;
}

pub(super) fn emit_cjk_upright(
    ctx: &SegmentEmitCtx,
    seg_text: &str,
    font_family: Option<&str>,
    font_family_ea: Option<&str>,
    out: &mut String,
    total_width: &mut f64,
) {
    if seg_text.is_empty() {
        return;
    }
    let bold_family = if ctx.props.bold {
        font_family.map(|f| format!("{f} Bold"))
    } else {
        None
    };
    let bold_family_ea = if ctx.props.bold {
        font_family_ea.map(|f| format!("{f} Bold"))
    } else {
        None
    };
    let font: Option<Arc<FontFace>> = bold_family
        .as_deref()
        .and_then(|n| ctx.font_resolver.resolve(n))
        .or_else(|| {
            bold_family_ea
                .as_deref()
                .and_then(|n| ctx.font_resolver.resolve(n))
        })
        .or_else(|| {
            resolve_face(
                ctx.font_resolver,
                ctx.props,
                font_family,
                font_family_ea,
                ctx.jpan_fallback,
            )
            .0
        });
    let synth_bold = should_synthesize_bold(ctx.props, font.as_deref());
    let fill_attrs = build_path_attrs(ctx.props, ctx.font_size_px, synth_bold);

    let vertical_style = slideglance_font::FontStyle {
        bold: ctx.props.bold,
        italic: ctx.props.italic,
    };
    for ch in seg_text.chars() {
        let mut buf = [0_u8; 4];
        let ch_str: &str = ch.encode_utf8(&mut buf);
        let char_width = ctx.measurer.measure_text_width(
            ch_str,
            ctx.font_size,
            vertical_style,
            ctx.props.font_family.as_deref(),
            ctx.props.font_family_ea.as_deref(),
        );
        if let Some(highlight) = &ctx.props.highlight {
            let bg_y = ctx.effective_y - ctx.font_size_px * 0.85;
            let bg_h = ctx.font_size_px * 1.1;
            let alpha_attr = if highlight.alpha < 1.0 {
                format!(" fill-opacity=\"{:.3}\"", highlight.alpha)
            } else {
                String::new()
            };
            let _ = write!(
                out,
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"{alpha_attr}/>",
                ctx.x + *total_width,
                bg_y,
                char_width,
                bg_h,
                color_hex(highlight)
            );
        }
        if let Some(face) = font.as_ref() {
            let char_x = ctx.x + *total_width;
            let path_data = text_to_svg_path_with_precision(
                face,
                ch_str,
                char_x,
                ctx.effective_y,
                ctx.font_size,
                DECIMAL_PLACES,
            );
            if !path_data.is_empty() {
                let cx = char_x + char_width / 2.0;
                let cy = ctx.effective_y
                    - (ctx.font_size_px
                        * (f64::from(face.ascender()) + f64::from(face.descender())))
                        / 2.0
                        / f64::from(face.units_per_em());
                let _ = write!(
                    out,
                    "<g transform=\"rotate(-90, {cx:.2}, {cy:.2})\"><path d=\"{path_data}\" {fill_attrs}/></g>"
                );
            }
        }
        if ctx.props.underline || ctx.props.strikethrough {
            out.push_str(&render_text_decorations(
                ctx.x + *total_width,
                ctx.effective_y,
                char_width,
                ctx.font_size_px,
                ctx.props,
            ));
        }
        *total_width += char_width;
    }
}

/// Sum of segment widths across one wrapped or non-wrapped line.
pub(super) fn measure_line_width<'a, I>(
    segments: I,
    default_font_size: f64,
    font_scale: f64,
    measurer: &dyn TextMeasurer,
) -> f64
where
    I: IntoIterator<Item = (&'a str, &'a RunProperties)>,
{
    let mut total = 0.0;
    for (text, props) in segments {
        let font_size = props
            .font_size
            .map_or(default_font_size, slideglance_utils::Pt::raw)
            * font_scale;
        let style = slideglance_font::FontStyle {
            bold: props.bold,
            italic: props.italic,
        };
        let raw = measurer.measure_text_width(
            text,
            font_size,
            style,
            props.font_family.as_deref(),
            props.font_family_ea.as_deref(),
        );
        let font_size_px = font_size * PX_PER_PT;
        total += raw + letter_spacing_trick_extra(text, font_size_px);
    }
    total
}

pub(super) fn letter_spacing_trick_extra(text: &str, font_size_px: f64) -> f64 {
    if !is_letter_spacing_trick(text) {
        return 0.0;
    }
    let n_spaces = text.chars().filter(|c| *c == ' ').count();
    n_spaces as f64 * font_size_px * LETTER_SPACING_TRICK_PAD_EM
}

/// Detect the `EA (whitespace+ EA)+` pattern. Requires ≥3 EA glyphs and
/// no non-EA, non-space characters. Latin words and bullet runs miss.
pub(super) fn is_letter_spacing_trick(text: &str) -> bool {
    use crate::text::script::is_cjk_codepoint;
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 5 {
        return false;
    }
    let mut i = 0;
    if !is_cjk_codepoint(chars[i] as u32) {
        return false;
    }
    let mut ea_count = 1;
    i += 1;
    while i < chars.len() {
        if chars[i] != ' ' {
            return false;
        }
        while i < chars.len() && chars[i] == ' ' {
            i += 1;
        }
        if i >= chars.len() {
            // Trailing whitespace — allow but pattern must already satisfy ≥3 EA.
            break;
        }
        if !is_cjk_codepoint(chars[i] as u32) {
            return false;
        }
        ea_count += 1;
        i += 1;
    }
    ea_count >= 3
}
