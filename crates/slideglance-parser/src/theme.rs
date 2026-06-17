//! `ppt/theme/themeN.xml` parser — color scheme, font scheme, format scheme.
//!
//! Mirrors.

use serde::Deserialize;
use slideglance_color::{ColorMap, ColorResolver, ColorScheme, Rgb};
use slideglance_model::{Fill, FontScheme, FormatScheme, Outline, Theme};

use crate::effect::{build_effect_list, RawEffectLst};
use crate::fill::{build_fill, build_outline, RawFillContainer, RawOutline};
use crate::xml::{parse_xml, XmlError};

/// Default font when the theme's `<a:latin>` is missing — matches TS.
const DEFAULT_FONT_TYPEFACE: &str = "Calibri";

/// Default theme color scheme — matches TS the spec fallback values.
const fn default_color_scheme() -> ColorScheme {
    ColorScheme {
        dk1: Rgb::new(0x00, 0x00, 0x00),
        lt1: Rgb::new(0xFF, 0xFF, 0xFF),
        dk2: Rgb::new(0x44, 0x54, 0x6A),
        lt2: Rgb::new(0xE7, 0xE6, 0xE6),
        accent1: Rgb::new(0x44, 0x72, 0xC4),
        accent2: Rgb::new(0xED, 0x7D, 0x31),
        accent3: Rgb::new(0xA5, 0xA5, 0xA5),
        accent4: Rgb::new(0xFF, 0xC0, 0x00),
        accent5: Rgb::new(0x5B, 0x9B, 0xD5),
        accent6: Rgb::new(0x70, 0xAD, 0x47),
        hlink: Rgb::new(0x05, 0x63, 0xC1),
        fol_hlink: Rgb::new(0x95, 0x4F, 0x72),
    }
}

fn default_font_scheme() -> FontScheme {
    FontScheme {
        major_font: DEFAULT_FONT_TYPEFACE.to_owned(),
        minor_font: DEFAULT_FONT_TYPEFACE.to_owned(),
        ..FontScheme::default()
    }
}

/// Parses a theme XML body into a [`Theme`].
///
/// `color_map` is always `None` because the document-level `<p:clrMap>`
/// lives on the slide master, not on the theme part itself; the slide-master
/// parser (later sub-phase) populates it. `fmt_scheme` is filled in when
/// `<a:fmtScheme>` is present.
///
/// # Errors
///
/// Returns [`XmlError`] when the XML is malformed.
pub fn parse_theme(xml: &str) -> Result<Theme, XmlError> {
    let raw: RawTheme = parse_xml(xml)?;
    let Some(theme_elements) = raw.theme_elements else {
        return Ok(Theme {
            color_scheme: default_color_scheme(),
            font_scheme: default_font_scheme(),
            fmt_scheme: None,
            color_map: None,
        });
    };

    let color_scheme = theme_elements
        .clr_scheme
        .as_ref()
        .map_or_else(default_color_scheme, parse_color_scheme);
    let font_scheme = theme_elements
        .font_scheme
        .as_ref()
        .map_or_else(default_font_scheme, parse_font_scheme);
    let fmt_scheme = theme_elements
        .fmt_scheme
        .as_ref()
        .and_then(|node| build_fmt_scheme(node, &color_scheme));

    Ok(Theme {
        color_scheme,
        font_scheme,
        fmt_scheme,
        color_map: None,
    })
}

fn build_fmt_scheme(node: &RawFmtScheme, scheme: &ColorScheme) -> Option<FormatScheme> {
    let resolver = ColorResolver::new(*scheme, ColorMap::default());

    let fill_styles = node
        .fill_style_lst
        .as_ref()
        .map(|lst| build_fill_list(lst, &resolver))
        .unwrap_or_default();
    let bg_fill_styles = node
        .bg_fill_style_lst
        .as_ref()
        .map(|lst| build_fill_list(lst, &resolver))
        .unwrap_or_default();
    let ln_styles = node
        .ln_style_lst
        .as_ref()
        .map(|lst| build_outline_list(lst, &resolver))
        .unwrap_or_default();
    let effect_styles = node
        .effect_style_lst
        .as_ref()
        .map(|lst| build_effect_style_list(lst, &resolver))
        .unwrap_or_default();

    if fill_styles.is_empty()
        && bg_fill_styles.is_empty()
        && ln_styles.is_empty()
        && effect_styles.is_empty()
    {
        return None;
    }
    Some(FormatScheme {
        fill_styles,
        ln_styles,
        effect_styles,
        bg_fill_styles,
    })
}

fn build_fill_list(lst: &RawFillStyleLst, resolver: &ColorResolver) -> Vec<Fill> {
    lst.entries
        .iter()
        .filter_map(|entry| build_fill(&entry.to_container(), resolver, None))
        .collect()
}

fn build_outline_list(lst: &RawLnStyleLst, resolver: &ColorResolver) -> Vec<Outline> {
    lst.ln
        .iter()
        .filter_map(|raw| build_outline(raw, resolver))
        .collect()
}

fn build_effect_style_list(
    lst: &RawEffectStyleLst,
    resolver: &ColorResolver,
) -> Vec<Option<slideglance_model::EffectList>> {
    lst.effect_style
        .iter()
        .map(|style| {
            style
                .effect_lst
                .as_ref()
                .and_then(|raw| build_effect_list(raw, resolver))
        })
        .collect()
}

fn parse_color_scheme(node: &RawClrScheme) -> ColorScheme {
    ColorScheme {
        dk1: extract_color(node.dk1.as_ref()),
        lt1: extract_color(node.lt1.as_ref()),
        dk2: extract_color(node.dk2.as_ref()),
        lt2: extract_color(node.lt2.as_ref()),
        accent1: extract_color(node.accent1.as_ref()),
        accent2: extract_color(node.accent2.as_ref()),
        accent3: extract_color(node.accent3.as_ref()),
        accent4: extract_color(node.accent4.as_ref()),
        accent5: extract_color(node.accent5.as_ref()),
        accent6: extract_color(node.accent6.as_ref()),
        hlink: extract_color(node.hlink.as_ref()),
        fol_hlink: extract_color(node.fol_hlink.as_ref()),
    }
}

fn extract_color(slot: Option<&RawColorSlot>) -> Rgb {
    let Some(slot) = slot else {
        return Rgb::new(0, 0, 0);
    };
    if let Some(srgb) = slot.srgb_clr.as_ref() {
        return parse_hex_or_black(srgb.val.as_deref());
    }
    if let Some(sys) = slot.sys_clr.as_ref() {
        return parse_hex_or_black(sys.last_clr.as_deref());
    }
    Rgb::new(0, 0, 0)
}

fn parse_hex_or_black(hex: Option<&str>) -> Rgb {
    hex.and_then(|s| Rgb::from_hex(s).ok())
        .unwrap_or(Rgb::new(0, 0, 0))
}

fn parse_font_scheme(node: &RawFontScheme) -> FontScheme {
    let major = node.major_font.as_ref();
    let minor = node.minor_font.as_ref();
    FontScheme {
        major_font: latin_or_default(major),
        minor_font: latin_or_default(minor),
        major_font_ea: resolve_ea_font(major),
        minor_font_ea: resolve_ea_font(minor),
        major_font_cs: major
            .and_then(|n| n.cs.as_ref())
            .and_then(|t| t.typeface.clone()),
        minor_font_cs: minor
            .and_then(|n| n.cs.as_ref())
            .and_then(|t| t.typeface.clone()),
        major_script_fonts: collect_script_fonts(major),
        minor_script_fonts: collect_script_fonts(minor),
    }
}

/// Collects every `<a:font script="..." typeface="..."/>` entry under a
/// `<a:majorFont>` / `<a:minorFont>` node, keyed by the ISO 15924 script
/// code. Mandated by the project rule "CJK Script Equality" — no script is
/// privileged over another (the spec extracts only `Jpan`, which is
/// the bug this generalization fixes).
fn collect_script_fonts(
    node: Option<&RawFontFamily>,
) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    let Some(family) = node else { return out };
    for entry in &family.font {
        let (Some(script), Some(typeface)) = (entry.script.as_deref(), entry.typeface.as_deref())
        else {
            continue;
        };
        if script.is_empty() || typeface.is_empty() {
            continue;
        }
        out.insert(script.to_owned(), typeface.to_owned());
    }
    out
}

fn latin_or_default(node: Option<&RawFontFamily>) -> String {
    node.and_then(|n| n.latin.as_ref())
        .and_then(|t| t.typeface.clone())
        .unwrap_or_else(|| DEFAULT_FONT_TYPEFACE.to_owned())
}

/// `<a:ea>` typeface, falling back to any `<a:font script="...">` entry on
/// the same family (CJK script codes prioritized in document order).
///
/// The spec falls back only to `script="Jpan"`; per the project rule
/// "CJK Script Equality" in `CLAUDE.md`, we treat every CJK script
/// (`Jpan`, `Hang`, `Hans`, `Hant`) equally. The first CJK script found in
/// document order wins — callers that need a content-language-specific font
/// should look up `*_script_fonts[<code>]` directly.
fn resolve_ea_font(node: Option<&RawFontFamily>) -> Option<String> {
    // All four CJK scripts treated equally per CJK Script Equality rule in CLAUDE.md.
    // This matches slideglance_font::CJK_SCRIPT_CODES; kept local to avoid a
    // circular dependency (slideglance-parser must not depend on slideglance-font).
    const CJK_SCRIPTS: &[&str] = &["Jpan", "Hang", "Hans", "Hant"];
    let ea = node
        .and_then(|n| n.ea.as_ref())
        .and_then(|t| t.typeface.clone());
    if ea.as_deref().is_some_and(|s| !s.is_empty()) {
        return ea;
    }
    let family = node?;
    for entry in &family.font {
        let (Some(script), Some(typeface)) = (entry.script.as_deref(), entry.typeface.as_deref())
        else {
            continue;
        };
        if CJK_SCRIPTS.contains(&script) && !typeface.is_empty() {
            return Some(typeface.to_owned());
        }
    }
    None
}

// --- raw XML shapes (post namespace strip) ---

#[derive(Debug, Default, Deserialize)]
struct RawTheme {
    #[serde(rename = "themeElements")]
    theme_elements: Option<RawThemeElements>,
}

// All fields end in `_scheme` because the OOXML element names are
// `clrScheme` / `fontScheme` / `fmtScheme` — keeping the suffix preserves
// the source-XML mapping the deserializer relies on.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Default, Deserialize)]
struct RawThemeElements {
    #[serde(rename = "clrScheme")]
    clr_scheme: Option<RawClrScheme>,
    #[serde(rename = "fontScheme")]
    font_scheme: Option<RawFontScheme>,
    #[serde(rename = "fmtScheme")]
    fmt_scheme: Option<RawFmtScheme>,
}

// Same OOXML naming reasoning: every field ends in `*StyleLst`.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Default, Deserialize)]
struct RawFmtScheme {
    #[serde(rename = "fillStyleLst")]
    fill_style_lst: Option<RawFillStyleLst>,
    #[serde(rename = "lnStyleLst")]
    ln_style_lst: Option<RawLnStyleLst>,
    #[serde(rename = "effectStyleLst")]
    effect_style_lst: Option<RawEffectStyleLst>,
    #[serde(rename = "bgFillStyleLst")]
    bg_fill_style_lst: Option<RawFillStyleLst>,
}

/// `<a:fillStyleLst>` / `<a:bgFillStyleLst>` — a sequence of mixed fill
/// children (`solidFill` / `gradFill` / `pattFill` / `blipFill` / `noFill`)
/// in source order.
#[derive(Debug, Default, Deserialize)]
struct RawFillStyleLst {
    #[serde(rename = "$value", default)]
    entries: Vec<RawFillStyleEntry>,
}

// Variants intentionally hold their full payload (rather than `Box`-ing the
// large ones) because fmtScheme fill lists are short — typically 3 entries —
// and the consumer transforms each into a `Fill` immediately, so the
// per-enum size cost is irrelevant in practice and pattern-match ergonomics
// matter more than uniform variant sizes.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Deserialize)]
enum RawFillStyleEntry {
    #[serde(rename = "noFill")]
    NoFill,
    #[serde(rename = "solidFill")]
    Solid(crate::fill::RawSolidFill),
    #[serde(rename = "gradFill")]
    Gradient(crate::fill::RawGradFill),
    #[serde(rename = "pattFill")]
    Pattern(crate::fill::RawPattFill),
    #[serde(rename = "blipFill")]
    Blip(crate::fill::RawBlipFill),
}

impl RawFillStyleEntry {
    /// Lifts the variant into a [`RawFillContainer`] so the shared
    /// [`build_fill`] helper can process it.
    fn to_container(&self) -> RawFillContainer {
        let mut c = RawFillContainer::default();
        match self {
            Self::NoFill => c.no_fill = Some(crate::fill::EmptyMarker {}),
            Self::Solid(s) => c.solid_fill = Some(s.clone()),
            Self::Gradient(g) => c.grad_fill = Some(g.clone()),
            Self::Pattern(p) => c.patt_fill = Some(p.clone()),
            Self::Blip(b) => c.blip_fill = Some(b.clone()),
        }
        c
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawLnStyleLst {
    #[serde(default)]
    ln: Vec<RawOutline>,
}

#[derive(Debug, Default, Deserialize)]
struct RawEffectStyleLst {
    #[serde(default, rename = "effectStyle")]
    effect_style: Vec<RawEffectStyle>,
}

#[derive(Debug, Default, Deserialize)]
struct RawEffectStyle {
    #[serde(rename = "effectLst")]
    effect_lst: Option<RawEffectLst>,
}

#[derive(Debug, Default, Deserialize)]
struct RawClrScheme {
    dk1: Option<RawColorSlot>,
    lt1: Option<RawColorSlot>,
    dk2: Option<RawColorSlot>,
    lt2: Option<RawColorSlot>,
    accent1: Option<RawColorSlot>,
    accent2: Option<RawColorSlot>,
    accent3: Option<RawColorSlot>,
    accent4: Option<RawColorSlot>,
    accent5: Option<RawColorSlot>,
    accent6: Option<RawColorSlot>,
    hlink: Option<RawColorSlot>,
    #[serde(rename = "folHlink")]
    fol_hlink: Option<RawColorSlot>,
}

#[derive(Debug, Default, Deserialize)]
struct RawColorSlot {
    #[serde(rename = "srgbClr")]
    srgb_clr: Option<RawSrgbClr>,
    #[serde(rename = "sysClr")]
    sys_clr: Option<RawSysClr>,
}

#[derive(Debug, Default, Deserialize)]
struct RawSrgbClr {
    #[serde(rename = "@val")]
    val: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawSysClr {
    #[serde(rename = "@lastClr")]
    last_clr: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawFontScheme {
    #[serde(rename = "majorFont")]
    major_font: Option<RawFontFamily>,
    #[serde(rename = "minorFont")]
    minor_font: Option<RawFontFamily>,
}

#[derive(Debug, Default, Deserialize)]
struct RawFontFamily {
    latin: Option<RawTypeface>,
    ea: Option<RawTypeface>,
    cs: Option<RawTypeface>,
    #[serde(default)]
    font: Vec<RawScriptFont>,
}

#[derive(Debug, Default, Deserialize)]
struct RawTypeface {
    #[serde(rename = "@typeface")]
    typeface: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawScriptFont {
    #[serde(rename = "@script")]
    script: Option<String>,
    #[serde(rename = "@typeface")]
    typeface: Option<String>,
}
