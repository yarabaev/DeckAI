//! Path-mode helpers — layout helpers, face resolution, field substitution.
//!
//! Extracted from `path_mode.rs` so the main entry stays focused on
//! the paragraph -> wrapped-line dispatch and emission helpers can
//! be reviewed independently.

use std::sync::Arc;

use slideglance_font::{wrap_paragraph, FontFace, FontResolver, TextMeasurer};
use slideglance_model::{
    BodyProperties, BulletType, Paragraph, ParagraphProperties, RunProperties, TextBody, TextRun,
    TextVerticalType,
};
use slideglance_utils::Emu;

use crate::slide_context::SlideRenderContext;
use crate::text::auto_num::format_auto_num;
use crate::text::layout::{
    compute_line_natural_height, get_default_line_height_ratio, get_line_spacing,
    get_paragraph_font_size, has_visible_bullet, resolve_spacing_px_opt, PX_PER_PT,
};

use super::Dimensions;

pub(super) fn needs_script_split(props: &RunProperties) -> bool {
    matches!(
        (&props.font_family, &props.font_family_ea),
        (Some(latin), Some(ea)) if latin != ea
    )
}

pub(super) fn resolve_face(
    resolver: &dyn FontResolver,
    props: &RunProperties,
    font_family: Option<&str>,
    font_family_ea: Option<&str>,
    jpan_fallback: Option<&str>,
) -> (Option<Arc<FontFace>>, bool) {
    if props.italic {
        let suffixes: &[&str] = if props.bold {
            &[" Bold Italic", " BoldItalic", " Italic Bold"]
        } else {
            &[" Italic"]
        };
        for sfx in suffixes {
            let try_face = |base: Option<&str>| -> Option<Arc<FontFace>> {
                let name = format!("{}{}", base?, sfx);
                resolver.resolve(&name)
            };
            if let Some(face) = try_face(font_family).or_else(|| try_face(font_family_ea)) {
                return (Some(face), true);
            }
        }
    }
    if props.bold {
        let try_bold = |base: Option<&str>| -> Option<Arc<FontFace>> {
            let name = format!("{} Bold", base?);
            resolver.resolve(&name)
        };
        if let Some(face) = try_bold(font_family).or_else(|| try_bold(font_family_ea)) {
            return (Some(face), false);
        }
    }
    let face = font_family
        .and_then(|n| resolver.resolve(n))
        .or_else(|| font_family_ea.and_then(|n| resolver.resolve(n)))
        .or_else(|| jpan_fallback.and_then(|n| resolver.resolve(n)));
    (face, false)
}

pub(super) fn resolve_text_dimensions(bp: &BodyProperties, w: f64, h: f64) -> Dimensions {
    let to_px = Emu::to_pixels;
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

pub(super) fn is_vertical(vert: TextVerticalType) -> bool {
    matches!(
        vert,
        TextVerticalType::Vert
            | TextVerticalType::EaVert
            | TextVerticalType::WordArtVert
            | TextVerticalType::MongolianVert
    )
}

pub(super) fn substitute_field_runs(body: &TextBody, slide: &SlideRenderContext) -> TextBody {
    let needs_clone = body
        .paragraphs
        .iter()
        .any(|p| p.runs.iter().any(|r| r.field_type.is_some()));
    if !needs_clone {
        return body.clone();
    }
    TextBody {
        default_text_color: None,
        paragraphs: body
            .paragraphs
            .iter()
            .map(|p| Paragraph {
                runs: p
                    .runs
                    .iter()
                    .map(|r| {
                        if let Some(field_type) = &r.field_type {
                            if let Some(value) =
                                crate::slide_context::format_field(field_type, slide)
                            {
                                return TextRun {
                                    text: value,
                                    properties: r.properties.clone(),
                                    field_type: r.field_type.clone(),
                                };
                            }
                        }
                        r.clone()
                    })
                    .collect(),
                properties: p.properties.clone(),
                end_para_run_properties: p.end_para_run_properties.clone(),
            })
            .collect(),
        body_properties: body.body_properties.clone(),
    }
}

pub(super) fn resolve_bullet_text(
    props: &ParagraphProperties,
    counters: &mut std::collections::HashMap<(slideglance_model::AutoNumScheme, u8), u32>,
) -> Option<String> {
    let bullet = props.bullet.as_ref()?;
    if !has_visible_bullet(props) {
        return None;
    }
    match bullet {
        BulletType::None => None,
        BulletType::Char { char } => Some(char.clone()),
        BulletType::AutoNum { scheme, start_at } => {
            let key = (*scheme, props.level);
            let current = counters.get(&key).copied().unwrap_or(0);
            let next_val = current + 1;
            counters.insert(key, next_val);
            Some(format_auto_num(*scheme, start_at + next_val - 1))
        }
    }
}

pub(super) fn estimate_total_height(
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
                let lines = wrap_paragraph(
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
