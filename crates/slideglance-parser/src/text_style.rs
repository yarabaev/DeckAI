//! `<a:lstStyle>` / `<p:defaultTextStyle>` / `<p:txStyles>` parser.
//!
//! Mirrors. Shared between
//! presentation-parser (for `<p:defaultTextStyle>`) and slide-master-parser
//! (for `<p:txStyles>` plus inheritance via `<a:lstStyle>`).

use serde::Deserialize;
use slideglance_color::ColorResolver;
use slideglance_model::{
    AutoNumScheme, BulletType, DefaultParagraphLevelProperties, DefaultRunProperties,
    DefaultTextStyle, FontScheme, ParagraphAlignment,
};
use slideglance_utils::{Emu, HundredthPt};

use crate::raw_color::RawColorChoice;
use crate::xml::{parse_xml, XmlError};

/// Parses an `<a:lstStyle>` (or any block with `defPPr` / `lvl1pPr` …
/// `lvl9pPr` children — `<p:defaultTextStyle>`, `<p:titleStyle>`, etc.).
///
/// Returns `Ok(None)` when both `defPPr` and every `lvlNpPr` are absent or
/// produce no recognized properties.
///
/// # Errors
///
/// Returns [`XmlError`] when the input is not well-formed XML.
pub fn parse_list_style(
    xml: &str,
    resolver: Option<&ColorResolver>,
) -> Result<Option<DefaultTextStyle>, XmlError> {
    let raw: RawListStyle = parse_xml(xml)?;
    Ok(build_list_style(&raw, resolver))
}

pub(crate) fn build_list_style(
    raw: &RawListStyle,
    resolver: Option<&ColorResolver>,
) -> Option<DefaultTextStyle> {
    let default_paragraph = raw
        .def_p_pr
        .as_ref()
        .and_then(|n| build_paragraph_level_properties(n, resolver));
    let levels: Vec<Option<DefaultParagraphLevelProperties>> = [
        raw.lvl1.as_ref(),
        raw.lvl2.as_ref(),
        raw.lvl3.as_ref(),
        raw.lvl4.as_ref(),
        raw.lvl5.as_ref(),
        raw.lvl6.as_ref(),
        raw.lvl7.as_ref(),
        raw.lvl8.as_ref(),
        raw.lvl9.as_ref(),
    ]
    .into_iter()
    .map(|n| n.and_then(|node| build_paragraph_level_properties(node, resolver)))
    .collect();

    if default_paragraph.is_none() && levels.iter().all(Option::is_none) {
        return None;
    }
    Some(DefaultTextStyle {
        default_paragraph,
        levels,
    })
}

pub(crate) fn build_paragraph_level_properties(
    node: &RawParagraphLevel,
    resolver: Option<&ColorResolver>,
) -> Option<DefaultParagraphLevelProperties> {
    let alignment = node.algn.as_deref().and_then(parse_alignment);
    let margin_left = node.mar_l.as_deref().and_then(parse_int).map(Emu::new);
    let indent = node.indent.as_deref().and_then(parse_int).map(Emu::new);
    let bullet = build_bullet(node);
    let bullet_font = node
        .bu_font
        .as_ref()
        .and_then(|n| n.typeface.clone())
        .filter(|s| !s.is_empty());
    let bullet_color = resolver.and_then(|r| {
        node.bu_clr
            .as_ref()
            .and_then(|c| c.to_color_ref().map(|cr| r.resolve(&cr)))
    });
    let bullet_size_pct = node
        .bu_sz_pct
        .as_ref()
        .and_then(|n| n.val.as_deref())
        .and_then(|s| s.parse::<f64>().ok());
    let default_run_properties = node
        .def_r_pr
        .as_ref()
        .and_then(|n| build_default_run_properties(n, resolver));

    let line_spacing = node
        .ln_spc
        .as_ref()
        .and_then(|n| n.spc_pct.as_ref())
        .and_then(|n| n.val.as_deref())
        .and_then(|v| v.parse::<f64>().ok());
    let space_before = crate::text_body::parse_spacing_node_pub(node.spc_bef.as_ref());
    let space_after = crate::text_body::parse_spacing_node_pub(node.spc_aft.as_ref());

    if alignment.is_none()
        && margin_left.is_none()
        && indent.is_none()
        && bullet.is_none()
        && bullet_font.is_none()
        && bullet_color.is_none()
        && bullet_size_pct.is_none()
        && line_spacing.is_none()
        && space_before.is_none()
        && space_after.is_none()
        && default_run_properties.is_none()
    {
        return None;
    }
    Some(DefaultParagraphLevelProperties {
        alignment,
        margin_left,
        indent,
        bullet,
        bullet_font,
        bullet_color,
        bullet_size_pct,
        line_spacing,
        space_before,
        space_after,
        default_run_properties,
    })
}

pub(crate) fn build_default_run_properties(
    node: &RawDefRPr,
    resolver: Option<&ColorResolver>,
) -> Option<DefaultRunProperties> {
    let font_size = node
        .sz
        .as_deref()
        .and_then(parse_int)
        .map(|raw| HundredthPt::new(raw).to_points());
    let font_family = node
        .latin
        .as_ref()
        .and_then(|n| n.typeface.clone())
        .filter(|s| !s.is_empty());
    let font_family_ea = node
        .ea
        .as_ref()
        .and_then(|n| n.typeface.clone())
        .filter(|s| !s.is_empty());
    let font_family_cs = node
        .cs
        .as_ref()
        .and_then(|n| n.typeface.clone())
        .filter(|s| !s.is_empty());
    let bold = node.b.as_deref().map(parse_bool_attr);
    let italic = node.i.as_deref().map(parse_bool_attr);
    let underline = node.u.as_deref().map(|v| v != "none");
    let strikethrough = node.strike.as_deref().map(|v| v != "noStrike");
    let color = resolver.and_then(|r| {
        node.solid_fill
            .as_ref()
            .and_then(|fill| fill.color.to_color_ref().map(|cr| r.resolve(&cr)))
    });

    if font_size.is_none()
        && font_family.is_none()
        && font_family_ea.is_none()
        && font_family_cs.is_none()
        && bold.is_none()
        && italic.is_none()
        && underline.is_none()
        && strikethrough.is_none()
        && color.is_none()
    {
        return None;
    }
    Some(DefaultRunProperties {
        font_size,
        font_family,
        font_family_ea,
        font_family_cs,
        bold,
        italic,
        underline,
        strikethrough,
        color,
        highlight: None,
    })
}

fn build_bullet(node: &RawParagraphLevel) -> Option<BulletType> {
    if node.bu_none.is_some() {
        return Some(BulletType::None);
    }
    if let Some(ch) = &node.bu_char {
        return Some(BulletType::Char {
            char: ch.char.clone().unwrap_or_else(|| "\u{2022}".to_owned()),
        });
    }
    if let Some(num) = &node.bu_auto_num {
        let scheme = num
            .ty
            .as_deref()
            .and_then(parse_auto_num_scheme)
            .unwrap_or(AutoNumScheme::ArabicPeriod);
        let start_at = num
            .start_at
            .as_deref()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);
        return Some(BulletType::AutoNum { scheme, start_at });
    }
    None
}

fn parse_alignment(s: &str) -> Option<ParagraphAlignment> {
    match s {
        "l" => Some(ParagraphAlignment::L),
        "ctr" => Some(ParagraphAlignment::Ctr),
        "r" => Some(ParagraphAlignment::R),
        "just" => Some(ParagraphAlignment::Just),
        _ => None,
    }
}

fn parse_auto_num_scheme(s: &str) -> Option<AutoNumScheme> {
    match s {
        "arabicPeriod" => Some(AutoNumScheme::ArabicPeriod),
        "arabicParenR" => Some(AutoNumScheme::ArabicParenR),
        "romanUcPeriod" => Some(AutoNumScheme::RomanUcPeriod),
        "romanLcPeriod" => Some(AutoNumScheme::RomanLcPeriod),
        "alphaUcPeriod" => Some(AutoNumScheme::AlphaUcPeriod),
        "alphaLcPeriod" => Some(AutoNumScheme::AlphaLcPeriod),
        "alphaLcParenR" => Some(AutoNumScheme::AlphaLcParenR),
        "alphaUcParenR" => Some(AutoNumScheme::AlphaUcParenR),
        "arabicPlain" => Some(AutoNumScheme::ArabicPlain),
        "circleNumDbPlain" => Some(AutoNumScheme::CircleNumDbPlain),
        "circleNumWdBlackPlain" => Some(AutoNumScheme::CircleNumWdBlackPlain),
        "circleNumWdWhitePlain" => Some(AutoNumScheme::CircleNumWdWhitePlain),
        _ => None,
    }
}

fn parse_int(s: &str) -> Option<i64> {
    s.parse::<i64>().ok()
}

fn parse_bool_attr(s: &str) -> bool {
    s == "1" || s == "true"
}

/// Resolves the OOXML theme-font tokens (`+mj-lt`, `+mn-ea`, etc.) into the
/// concrete typeface from the supplied [`FontScheme`].
///
/// Returns the input unchanged if it is not one of the six known tokens, or
/// if `font_scheme` is `None`. Mirrors `resolveThemeFont` from the TS
/// reference.
#[must_use]
pub fn resolve_theme_font(
    typeface: Option<&str>,
    font_scheme: Option<&FontScheme>,
) -> Option<String> {
    let (Some(name), Some(scheme)) = (typeface, font_scheme) else {
        return typeface.map(str::to_owned);
    };
    let resolved: Option<String> = match name {
        "+mj-lt" => Some(scheme.major_font.clone()),
        "+mn-lt" => Some(scheme.minor_font.clone()),
        // IP-14: if the dedicated ea field is empty, fall back to the first
        // BTreeMap entry (alphabetically) — typically the dominant CJK script
        // in the theme (Hang, Hans, Jpan, etc.).
        "+mj-ea" => scheme
            .major_font_ea
            .clone()
            .or_else(|| scheme.major_script_fonts.values().next().cloned()),
        "+mn-ea" => scheme
            .minor_font_ea
            .clone()
            .or_else(|| scheme.minor_script_fonts.values().next().cloned()),
        "+mj-cs" => scheme.major_font_cs.clone(),
        "+mn-cs" => scheme.minor_font_cs.clone(),
        other => Some(other.to_owned()),
    };
    resolved
}

// --- raw XML shapes (post namespace strip) ---

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawListStyle {
    #[serde(rename = "defPPr")]
    pub def_p_pr: Option<RawParagraphLevel>,
    #[serde(rename = "lvl1pPr")]
    pub lvl1: Option<RawParagraphLevel>,
    #[serde(rename = "lvl2pPr")]
    pub lvl2: Option<RawParagraphLevel>,
    #[serde(rename = "lvl3pPr")]
    pub lvl3: Option<RawParagraphLevel>,
    #[serde(rename = "lvl4pPr")]
    pub lvl4: Option<RawParagraphLevel>,
    #[serde(rename = "lvl5pPr")]
    pub lvl5: Option<RawParagraphLevel>,
    #[serde(rename = "lvl6pPr")]
    pub lvl6: Option<RawParagraphLevel>,
    #[serde(rename = "lvl7pPr")]
    pub lvl7: Option<RawParagraphLevel>,
    #[serde(rename = "lvl8pPr")]
    pub lvl8: Option<RawParagraphLevel>,
    #[serde(rename = "lvl9pPr")]
    pub lvl9: Option<RawParagraphLevel>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawParagraphLevel {
    #[serde(rename = "@algn")]
    pub algn: Option<String>,
    #[serde(rename = "@marL")]
    pub mar_l: Option<String>,
    #[serde(rename = "@indent")]
    pub indent: Option<String>,
    #[serde(rename = "buNone")]
    pub bu_none: Option<EmptyMarker>,
    #[serde(rename = "buChar")]
    pub bu_char: Option<RawBuChar>,
    #[serde(rename = "buAutoNum")]
    pub bu_auto_num: Option<RawBuAutoNum>,
    #[serde(rename = "buFont")]
    pub bu_font: Option<RawTypeface>,
    #[serde(rename = "buClr")]
    pub bu_clr: Option<RawColorChoice>,
    #[serde(rename = "buSzPct")]
    pub bu_sz_pct: Option<RawValAttrStr>,
    #[serde(rename = "defRPr")]
    pub def_r_pr: Option<RawDefRPr>,
    /// `<a:lnSpc>` line spacing — only consumed by paragraph-level parsing
    /// (text-body), not by list-style defaults.
    #[serde(rename = "lnSpc")]
    pub ln_spc: Option<crate::text_body::RawSpacing>,
    /// `<a:spcBef>` space before — only consumed by paragraph-level parsing.
    #[serde(rename = "spcBef")]
    pub spc_bef: Option<crate::text_body::RawSpacing>,
    /// `<a:spcAft>` space after — only consumed by paragraph-level parsing.
    #[serde(rename = "spcAft")]
    pub spc_aft: Option<crate::text_body::RawSpacing>,
    /// `<a:tabLst>` — only consumed by paragraph-level parsing.
    #[serde(rename = "tabLst")]
    pub tab_lst: Option<crate::text_body::RawTabLst>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct EmptyMarker {}

#[derive(Debug, Deserialize)]
pub(crate) struct RawBuChar {
    #[serde(rename = "@char")]
    pub char: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawBuAutoNum {
    #[serde(rename = "@type")]
    pub ty: Option<String>,
    #[serde(rename = "@startAt")]
    pub start_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawTypeface {
    #[serde(rename = "@typeface")]
    pub typeface: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawValAttrStr {
    #[serde(rename = "@val")]
    pub val: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawDefRPr {
    #[serde(rename = "@sz")]
    pub sz: Option<String>,
    #[serde(rename = "@b")]
    pub b: Option<String>,
    #[serde(rename = "@i")]
    pub i: Option<String>,
    #[serde(rename = "@u")]
    pub u: Option<String>,
    #[serde(rename = "@strike")]
    pub strike: Option<String>,
    pub latin: Option<RawTypeface>,
    pub ea: Option<RawTypeface>,
    pub cs: Option<RawTypeface>,
    #[serde(rename = "solidFill")]
    pub solid_fill: Option<RawDefRPrSolidFill>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawDefRPrSolidFill {
    #[serde(flatten)]
    pub color: RawColorChoice,
}
