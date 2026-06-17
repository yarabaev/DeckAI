//! Paragraph builder for `parse_text_body`.
//!
//! Extracted from `text_body.rs` so the giant `build_paragraph`
//! routine — bullet handling, run dispatch, line-break decisions —
//! lives apart from the surrounding body-property scaffolding.

use std::collections::BTreeMap;

use slideglance_color::ColorResolver;
use slideglance_model::{DefaultTextStyle, FontScheme, Paragraph, ParagraphProperties, TextRun};

use crate::relationships::Relationship;
use crate::text_style::{
    build_default_run_properties, build_paragraph_level_properties, RawParagraphLevel,
};

use super::run::{build_run, build_run_properties, merge_default_run_properties};
use super::{
    extract_text, parse_spacing_node_optional, parse_tab_stops_node, ParagraphChild, RawParagraph,
    RawRunProperties,
};

/// Build one [`Paragraph`] from a `<a:p>` element. Walks the
/// child stream for runs / fields / breaks, applies bullet inheritance
/// from the matching list-style level, and folds in default run
/// properties from `<a:pPr><a:defRPr/>` plus the lstStyle level.
#[allow(clippy::too_many_lines)]
pub(super) fn build_paragraph(
    raw: &RawParagraph,
    resolver: &ColorResolver,
    rels: Option<&BTreeMap<String, Relationship>>,
    font_scheme: Option<&FontScheme>,
    lst_style: Option<&DefaultTextStyle>,
) -> Paragraph {
    // The `pPr` and `endParaRPr` are folded out of the same `$value` stream
    // because quick-xml's serde de cannot mix a Vec<$value enum> with sibling
    // named fields. Take the first occurrence of each.
    let mut p_pr: Option<&RawParagraphLevel> = None;
    let mut end_para_r_pr: Option<&RawRunProperties> = None;
    for child in &raw.children {
        match child {
            ParagraphChild::PPr(node) if p_pr.is_none() => p_pr = Some(node),
            ParagraphChild::EndParaRPr(node) if end_para_r_pr.is_none() => {
                end_para_r_pr = Some(node);
            }
            _ => {}
        }
    }

    // The OOXML `@lvl` lives on the `<a:p>` itself, not inside `<a:pPr>`.
    let level = raw.level.unwrap_or(0);

    let lst_level_props = lst_style
        .and_then(|s| s.levels.get(usize::from(level)))
        .and_then(|opt| opt.as_ref());

    // Bullet: explicit pPr overrides everything; otherwise fall back to the
    // matching list-style level. text_style::build_paragraph_level_properties
    // already handles the same fields, so we lift its result into the
    // paragraph's own properties.
    let p_pr_props = p_pr.and_then(|n| build_paragraph_level_properties(n, Some(resolver)));

    let alignment = p_pr_props
        .as_ref()
        .and_then(|p| p.alignment)
        .or_else(|| lst_level_props.and_then(|p| p.alignment));
    let bullet = p_pr_props
        .as_ref()
        .and_then(|p| p.bullet.clone())
        .or_else(|| lst_level_props.and_then(|p| p.bullet.clone()));
    let bullet_font = p_pr_props
        .as_ref()
        .and_then(|p| p.bullet_font.clone())
        .or_else(|| lst_level_props.and_then(|p| p.bullet_font.clone()));
    // Theme-resolve `+mj-lt` / `+mn-ea` placeholders. Without this the
    // renderer's `font_resolver.resolve("+mj-lt")` returns None and the
    // bullet text (auto-numbered list markers, custom glyphs) silently
    // disappears — observed on slide 4's TOC where every item lost its
    // numbering.
    let bullet_font = crate::text_style::resolve_theme_font(bullet_font.as_deref(), font_scheme);
    let bullet_color = p_pr_props
        .as_ref()
        .and_then(|p| p.bullet_color)
        .or_else(|| lst_level_props.and_then(|p| p.bullet_color));
    let bullet_size_pct = p_pr_props
        .as_ref()
        .and_then(|p| p.bullet_size_pct)
        .or_else(|| lst_level_props.and_then(|p| p.bullet_size_pct));
    let margin_left = p_pr_props
        .as_ref()
        .and_then(|p| p.margin_left)
        .or_else(|| lst_level_props.and_then(|p| p.margin_left));
    let indent = p_pr_props
        .as_ref()
        .and_then(|p| p.indent)
        .or_else(|| lst_level_props.and_then(|p| p.indent));

    // OOXML paragraph-level spacing falls back to the matching `lstStyle` /
    // `bodyStyle` level in the slide's effective text style chain when the
    // explicit `<a:pPr>` doesn't override. Without this fallback the
    // master's `spcBef` / `lnSpc` (~120 % spacing on title decks, 90 %
    // tight spacing for bulleted body lists) was silently dropped, causing
    // bulleted lists to pack 2-3× tighter than PowerPoint produces (most
    // visible on the slide-105 ToC). The bullet / margin / indent fields
    // already chained through `lst_level_props` above; spacing fields use
    // the same fallback.
    //
    // The line_spacing fallback in particular fixes slide-002 title
    // ("평 가 항 목  조 견 표"): the title's lstStyle/lvl1pPr declares
    // `<a:lnSpc spcPct="90000"/>` but the paragraph itself has no lnSpc.
    // Without inheritance, get_line_spacing() defaults to 1.0 → estimated
    // line height overflows the title cell → normAutofit shrinks the
    // text by ~4 % → final font ends up at 21.95pt instead of the 23.43pt
    // PowerPoint actually renders. With inheritance the line height fits
    // and no shrink fires.
    let line_spacing = p_pr
        .and_then(|n| n.ln_spc.as_ref())
        .and_then(|n| n.spc_pct.as_ref())
        .and_then(|n| n.val.as_deref())
        .and_then(|v| v.parse::<f64>().ok())
        .or_else(|| lst_level_props.and_then(|p| p.line_spacing));
    let space_before = parse_spacing_node_optional(p_pr.and_then(|n| n.spc_bef.as_ref()))
        .or_else(|| lst_level_props.and_then(|p| p.space_before));
    let space_after = parse_spacing_node_optional(p_pr.and_then(|n| n.spc_aft.as_ref()))
        .or_else(|| lst_level_props.and_then(|p| p.space_after));
    let tab_stops = parse_tab_stops_node(p_pr.and_then(|n| n.tab_lst.as_ref()));

    let properties = ParagraphProperties {
        alignment,
        line_spacing,
        space_before,
        space_after,
        level,
        bullet,
        bullet_font,
        bullet_color,
        bullet_size_pct,
        margin_left,
        indent,
        tab_stops,
    };

    // Merge defRPr from pPr (primary) and lstStyle level (secondary).
    let p_pr_def_r_pr = p_pr
        .and_then(|n| n.def_r_pr.as_ref())
        .and_then(|n| build_default_run_properties(n, Some(resolver)));
    let lst_def_r_pr = lst_level_props.and_then(|p| p.default_run_properties.clone());
    let merged_defaults = merge_default_run_properties(p_pr_def_r_pr, lst_def_r_pr);

    // Walk the source-order children for runs / fields / breaks.
    let mut runs: Vec<TextRun> = Vec::new();
    for child in &raw.children {
        match child {
            ParagraphChild::R(run) => {
                runs.push(build_run(
                    run.r_pr.as_ref(),
                    extract_text(run.t.as_ref()),
                    None,
                    resolver,
                    rels,
                    font_scheme,
                    merged_defaults.as_ref(),
                ));
            }
            ParagraphChild::Fld(fld) => {
                runs.push(build_run(
                    fld.r_pr.as_ref(),
                    extract_text(fld.t.as_ref()),
                    fld.ty.clone(),
                    resolver,
                    rels,
                    font_scheme,
                    merged_defaults.as_ref(),
                ));
            }
            ParagraphChild::Br(br) => {
                runs.push(build_run(
                    br.r_pr.as_ref(),
                    "\n".to_owned(),
                    None,
                    resolver,
                    rels,
                    font_scheme,
                    merged_defaults.as_ref(),
                ));
            }
            ParagraphChild::PPr(_) | ParagraphChild::EndParaRPr(_) => {}
        }
    }

    let end_para_run_properties = end_para_r_pr.map(|n| {
        build_run_properties(
            Some(n),
            resolver,
            rels,
            font_scheme,
            merged_defaults.as_ref(),
        )
    });

    Paragraph {
        runs,
        properties,
        end_para_run_properties,
    }
}
