//! `<p:style>` reference resolver — looks up `fillRef` / `lnRef` /
//! `effectRef` / `fontRef` indices into a theme's `<a:fmtScheme>` and
//! produces the concrete fill / outline / effect / font values.
//!
//! Mirrors.

use serde::Deserialize;
use slideglance_color::{ColorResolver, ResolvedColor};
use slideglance_model::{EffectList, Fill, FormatScheme, Outline, OutlineFill, SolidFill};

use crate::raw_color::RawColorChoice;
use crate::xml::{parse_xml, XmlError};

/// Result of resolving a `<p:style>` element against a `FormatScheme`.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedStyleReference {
    /// Resolved fill from `fillRef`. `None` when `idx="0"` or the index is
    /// out of range.
    pub fill: Option<Fill>,
    /// Resolved outline from `lnRef`.
    pub outline: Option<Outline>,
    /// Resolved effect list from `effectRef`.
    pub effects: Option<EffectList>,
    /// Resolved `fontRef` (idx token + override color).
    pub font_ref: Option<FontReference>,
}

/// `<a:fontRef>` payload — the OOXML `idx` token (`"major"` / `"minor"` /
/// `"none"`) plus an optional override color.
#[derive(Clone, Debug, PartialEq)]
pub struct FontReference {
    /// `@idx` value. Defaults to `"minor"` when absent.
    pub idx: String,
    /// Optional override color resolved from the `fontRef`'s color child.
    pub color: Option<ResolvedColor>,
}

/// Parses a `<p:style>` XML body and resolves its references.
///
/// Returns `Ok(None)` when the supplied `fmt_scheme` is `None` (no theme
/// available for resolution) or the style element is empty.
///
/// # Errors
///
/// Returns [`XmlError`] when the input is not well-formed XML.
pub fn resolve_shape_style(
    xml: &str,
    fmt_scheme: Option<&FormatScheme>,
    resolver: &ColorResolver,
) -> Result<Option<ResolvedStyleReference>, XmlError> {
    let raw: RawStyle = parse_xml(xml)?;
    Ok(build_resolved_style(&raw, fmt_scheme, resolver))
}

pub(crate) fn build_resolved_style(
    raw: &RawStyle,
    fmt_scheme: Option<&FormatScheme>,
    resolver: &ColorResolver,
) -> Option<ResolvedStyleReference> {
    let fmt = fmt_scheme?;
    if raw.fill_ref.is_none()
        && raw.ln_ref.is_none()
        && raw.effect_ref.is_none()
        && raw.font_ref.is_none()
    {
        return None;
    }

    let fill = raw
        .fill_ref
        .as_ref()
        .and_then(|r| resolve_fill_ref(r, fmt, resolver));
    let outline = raw
        .ln_ref
        .as_ref()
        .and_then(|r| resolve_line_ref(r, fmt, resolver));
    let effects = raw
        .effect_ref
        .as_ref()
        .and_then(|r| resolve_effect_ref(r, fmt));
    let font_ref = raw.font_ref.as_ref().map(|r| FontReference {
        idx: r.idx.clone().unwrap_or_else(|| "minor".to_owned()),
        color: r.color.to_color_ref().map(|cr| resolver.resolve(&cr)),
    });

    Some(ResolvedStyleReference {
        fill,
        outline,
        effects,
        font_ref,
    })
}

fn resolve_fill_ref(
    ref_node: &RawStyleRef,
    fmt: &FormatScheme,
    resolver: &ColorResolver,
) -> Option<Fill> {
    let idx = parse_idx(ref_node.idx.as_deref());
    if idx == 0 {
        return None;
    }
    // OOXML convention: idx >= 1000 selects from bgFillStyleLst
    // (subtracting 1001 to land on the 0-based array index); otherwise
    // fillStyleLst with idx-1.
    let (list, array_idx) = if idx >= 1000 {
        (&fmt.bg_fill_styles, idx - 1001)
    } else {
        (&fmt.fill_styles, idx - 1)
    };
    let template = list.get(usize::try_from(array_idx).ok()?)?.clone();

    let override_color = ref_node
        .color
        .to_color_ref()
        .map(|cr| resolver.resolve(&cr));
    match (override_color, template) {
        (Some(c), Fill::Solid(_)) => Some(Fill::Solid(SolidFill { color: c })),
        (Some(c), Fill::Gradient(mut g)) => {
            for stop in &mut g.stops {
                stop.color = c;
            }
            Some(Fill::Gradient(g))
        }
        (_, other) => Some(other),
    }
}

fn resolve_line_ref(
    ref_node: &RawStyleRef,
    fmt: &FormatScheme,
    resolver: &ColorResolver,
) -> Option<Outline> {
    let idx = parse_idx(ref_node.idx.as_deref());
    if idx == 0 {
        return None;
    }
    let array_idx = usize::try_from(idx - 1).ok()?;
    let mut template = fmt.ln_styles.get(array_idx)?.clone();

    let override_color = ref_node
        .color
        .to_color_ref()
        .map(|cr| resolver.resolve(&cr));
    if let Some(c) = override_color {
        template.fill = Some(OutlineFill::Solid(SolidFill { color: c }));
    }
    Some(template)
}

fn resolve_effect_ref(ref_node: &RawStyleRef, fmt: &FormatScheme) -> Option<EffectList> {
    let idx = parse_idx(ref_node.idx.as_deref());
    if idx == 0 {
        return None;
    }
    let array_idx = usize::try_from(idx - 1).ok()?;
    fmt.effect_styles.get(array_idx)?.clone()
}

fn parse_idx(s: Option<&str>) -> i64 {
    s.and_then(|v| v.parse::<i64>().ok()).unwrap_or(0)
}

// --- raw XML shapes (post namespace strip) ---

// Every field ends in `_ref` because the OOXML element names are
// `fillRef` / `lnRef` / `effectRef` / `fontRef`.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawStyle {
    #[serde(rename = "fillRef")]
    pub fill_ref: Option<RawStyleRef>,
    #[serde(rename = "lnRef")]
    pub ln_ref: Option<RawStyleRef>,
    #[serde(rename = "effectRef")]
    pub effect_ref: Option<RawStyleRef>,
    #[serde(rename = "fontRef")]
    pub font_ref: Option<RawFontRef>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawStyleRef {
    #[serde(rename = "@idx")]
    pub idx: Option<String>,
    #[serde(flatten)]
    pub color: RawColorChoice,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawFontRef {
    #[serde(rename = "@idx")]
    pub idx: Option<String>,
    #[serde(flatten)]
    pub color: RawColorChoice,
}
