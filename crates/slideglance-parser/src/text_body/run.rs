//! Run-level builders for `parse_text_body`.
//!
//! `build_run` produces a single [`TextRun`]; `build_run_properties`
//! resolves the rich `<a:rPr>` attribute set; `merge_default_run_properties`
//! folds list-style defaults into a paragraph's first-text-run state.

use std::collections::BTreeMap;

use slideglance_color::ColorResolver;
use slideglance_model::{DefaultRunProperties, FontScheme, RunProperties, TextOutline, TextRun};
use slideglance_utils::{Emu, HundredthPt};

use crate::relationships::Relationship;

use super::{build_hyperlink, parse_attr_i64, parse_optional_bool, RawRunProperties};

/// Build one [`TextRun`] from a `<a:r>` (or `<a:fld>` masquerading
/// as one). Resolves run properties via [`build_run_properties`].
pub(super) fn build_run(
    r_pr: Option<&RawRunProperties>,
    text: String,
    field_type: Option<String>,
    resolver: &ColorResolver,
    rels: Option<&BTreeMap<String, Relationship>>,
    font_scheme: Option<&FontScheme>,
    defaults: Option<&DefaultRunProperties>,
) -> TextRun {
    let properties = build_run_properties(r_pr, resolver, rels, font_scheme, defaults);
    TextRun {
        text,
        properties,
        field_type,
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn build_run_properties(
    r_pr: Option<&RawRunProperties>,
    resolver: &ColorResolver,
    rels: Option<&BTreeMap<String, Relationship>>,
    font_scheme: Option<&FontScheme>,
    defaults: Option<&DefaultRunProperties>,
) -> RunProperties {
    use crate::text_style::resolve_theme_font;

    let Some(node) = r_pr else {
        return RunProperties {
            font_size: defaults.and_then(|d| d.font_size),
            font_family: resolve_theme_font(
                defaults.and_then(|d| d.font_family.as_deref()),
                font_scheme,
            ),
            font_family_ea: resolve_theme_font(
                defaults.and_then(|d| d.font_family_ea.as_deref()),
                font_scheme,
            ),
            font_family_cs: resolve_theme_font(
                defaults.and_then(|d| d.font_family_cs.as_deref()),
                font_scheme,
            ),
            bold: defaults.and_then(|d| d.bold).unwrap_or(false),
            italic: defaults.and_then(|d| d.italic).unwrap_or(false),
            underline: defaults.and_then(|d| d.underline).unwrap_or(false),
            strikethrough: defaults.and_then(|d| d.strikethrough).unwrap_or(false),
            color: None,
            baseline: 0.0,
            hyperlink: None,
            outline: None,
            highlight: defaults.and_then(|d| d.highlight),
            font_family_sym: None,
            kern: None,
            char_spacing: None,
        };
    };

    let highlight_override = node
        .highlight
        .as_ref()
        .and_then(|n| n.color.to_color_ref())
        .map(|cr| resolver.resolve(&cr));
    let highlight = highlight_override.or_else(|| defaults.and_then(|d| d.highlight));

    let font_size = node
        .sz
        .as_deref()
        .and_then(|v| v.parse::<i64>().ok())
        .map(|v| HundredthPt::new(v).to_points())
        .or_else(|| defaults.and_then(|d| d.font_size));
    let latin_typeface = node
        .latin
        .as_ref()
        .and_then(|n| n.typeface.as_deref())
        .or_else(|| defaults.and_then(|d| d.font_family.as_deref()));
    let ea_typeface = node
        .ea
        .as_ref()
        .and_then(|n| n.typeface.as_deref())
        .or_else(|| defaults.and_then(|d| d.font_family_ea.as_deref()));
    let cs_typeface = node
        .cs
        .as_ref()
        .and_then(|n| n.typeface.as_deref())
        .or_else(|| defaults.and_then(|d| d.font_family_cs.as_deref()));
    let sym_typeface = node.sym.as_ref().and_then(|n| n.typeface.as_deref());
    let font_family = resolve_theme_font(latin_typeface, font_scheme);
    let font_family_ea = resolve_theme_font(ea_typeface, font_scheme);
    let font_family_cs = resolve_theme_font(cs_typeface, font_scheme);
    let font_family_sym = sym_typeface.map(str::to_owned);

    let bold = parse_optional_bool(node.b.as_deref())
        .or_else(|| defaults.and_then(|d| d.bold))
        .unwrap_or(false);
    let italic = parse_optional_bool(node.i.as_deref())
        .or_else(|| defaults.and_then(|d| d.italic))
        .unwrap_or(false);
    let has_explicit_underline = node.u.is_some();
    let mut underline = node
        .u
        .as_deref()
        .map(|v| v != "none")
        .or_else(|| defaults.and_then(|d| d.underline))
        .unwrap_or(false);
    let strikethrough = node
        .strike
        .as_deref()
        .map(|v| v != "noStrike")
        .or_else(|| defaults.and_then(|d| d.strikethrough))
        .unwrap_or(false);
    let baseline = node
        .baseline
        .as_deref()
        .and_then(|v| v.parse::<f64>().ok())
        .map_or(0.0, |v| v / 1000.0);

    // @kern is in half-points; store as HundredthPt (1 half-pt = 50 hundredths-pt).
    let kern = node
        .kern
        .as_deref()
        .and_then(|v| v.parse::<i64>().ok())
        .map(|hp| HundredthPt::new(hp * 50));
    // @spc is already in hundredths of a point.
    let char_spacing = node
        .spc
        .as_deref()
        .and_then(|v| v.parse::<i64>().ok())
        .map(HundredthPt::new);

    // Color: <a:solidFill> takes precedence over a direct color child on rPr.
    let mut color = node
        .solid_fill
        .as_ref()
        .and_then(|fill| fill.color.to_color_ref())
        .or_else(|| node.color.to_color_ref())
        .map(|cr| resolver.resolve(&cr));

    let hyperlink = build_hyperlink(node.hlink_click.as_ref(), rels);
    if hyperlink.is_some() {
        if color.is_none() {
            color = Some(resolver.resolve(&slideglance_color::ColorRef::Scheme {
                name: "hlink".to_owned(),
                transform: slideglance_color::ColorTransform::default(),
            }));
        }
        if !has_explicit_underline {
            underline = true;
        }
    }

    let outline = node.ln.as_ref().and_then(|ln| {
        let width = parse_attr_i64(ln.w.as_deref(), 12_700);
        let lc = ln
            .solid_fill
            .as_ref()
            .and_then(|fill| fill.color.to_color_ref())
            .map(|cr| resolver.resolve(&cr))?;
        Some(TextOutline {
            width: Emu::new(width),
            color: lc,
        })
    });

    RunProperties {
        font_size,
        font_family,
        font_family_ea,
        font_family_cs,
        bold,
        italic,
        underline,
        strikethrough,
        color,
        baseline,
        hyperlink,
        outline,
        highlight,
        font_family_sym,
        kern,
        char_spacing,
    }
}

pub(super) fn merge_default_run_properties(
    primary: Option<DefaultRunProperties>,
    secondary: Option<DefaultRunProperties>,
) -> Option<DefaultRunProperties> {
    match (primary, secondary) {
        (None, None) => None,
        (Some(p), None) => Some(p),
        (None, Some(s)) => Some(s),
        (Some(p), Some(s)) => Some(DefaultRunProperties {
            font_size: p.font_size.or(s.font_size),
            font_family: p.font_family.or(s.font_family),
            font_family_ea: p.font_family_ea.or(s.font_family_ea),
            font_family_cs: p.font_family_cs.or(s.font_family_cs),
            bold: p.bold.or(s.bold),
            italic: p.italic.or(s.italic),
            underline: p.underline.or(s.underline),
            strikethrough: p.strikethrough.or(s.strikethrough),
            color: p.color.or(s.color),
            highlight: p.highlight.or(s.highlight),
        }),
    }
}
