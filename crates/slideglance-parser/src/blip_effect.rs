//! `<a:blip>` image-effect parser (grayscale / biLevel / blur / lum /
//! duotone / clrChange).
//!
//! Mirrors.

use serde::Deserialize;
use slideglance_color::{ColorRef, ColorResolver, ColorTransform, ResolvedColor};
use slideglance_model::{
    BiLevelEffect, BlipEffects, BlurEffect, ClrChangeEffect, DuotoneEffect, LumEffect,
};
use slideglance_utils::Emu;

use crate::raw_color::{RawColorChoice, RawSchemeClr, RawSrgbClr, RawSysClr};
use crate::xml::{parse_xml, XmlError};

/// Parses a `<a:blip>` body into [`BlipEffects`].
///
/// Returns `Ok(None)` when no recognized image effects are present.
///
/// # Errors
///
/// Returns [`XmlError`] when the input is not well-formed XML.
pub fn parse_blip_effects(
    xml: &str,
    resolver: &ColorResolver,
) -> Result<Option<BlipEffects>, XmlError> {
    let raw: RawBlip = parse_xml(xml)?;
    Ok(build_blip_effects(&raw, resolver))
}

pub(crate) fn build_blip_effects(raw: &RawBlip, resolver: &ColorResolver) -> Option<BlipEffects> {
    let grayscale = raw.grayscl.is_some();
    let bi_level = raw.bi_level.as_ref().map(build_bi_level);
    let blur = raw.blur.as_ref().map(build_blur);
    let lum = raw.lum.as_ref().map(build_lum);
    let duotone = raw
        .duotone
        .as_ref()
        .and_then(|d| build_duotone(d, resolver));
    let clr_change = raw
        .clr_change
        .as_ref()
        .and_then(|c| build_clr_change(c, resolver));

    if !grayscale
        && bi_level.is_none()
        && blur.is_none()
        && lum.is_none()
        && duotone.is_none()
        && clr_change.is_none()
    {
        return None;
    }
    Some(BlipEffects {
        grayscale,
        bi_level,
        blur,
        lum,
        duotone,
        clr_change,
    })
}

fn build_bi_level(node: &RawBiLevel) -> BiLevelEffect {
    let raw_threshold = node
        .thresh
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(50_000);
    BiLevelEffect {
        // OOXML default is 50000 → 0.5; never NaN.
        threshold: raw_threshold as f64 / 100_000.0,
    }
}

fn build_blur(node: &RawBlur) -> BlurEffect {
    BlurEffect {
        radius: Emu::new(node.rad.unwrap_or(0)),
        // OOXML default for `grow` is "1" — only literal "0" disables growth.
        grow: node.grow.as_deref() != Some("0"),
    }
}

fn build_lum(node: &RawLum) -> LumEffect {
    let bright = node
        .bright
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    let contrast = node
        .contrast
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    LumEffect {
        brightness: bright as f64 / 100_000.0,
        contrast: contrast as f64 / 100_000.0,
    }
}

fn build_duotone(node: &RawDuotone, resolver: &ColorResolver) -> Option<DuotoneEffect> {
    let mut colors = Vec::with_capacity(2);
    for raw in &node.colors {
        if let Some(c) = raw.resolve(resolver) {
            colors.push(c);
        }
    }
    if colors.len() < 2 {
        return None;
    }
    Some(DuotoneEffect {
        color1: colors[0],
        color2: colors[1],
    })
}

fn build_clr_change(node: &RawClrChange, resolver: &ColorResolver) -> Option<ClrChangeEffect> {
    let from = node.clr_from.as_ref()?.to_color_ref()?;
    let to = node.clr_to.as_ref()?.to_color_ref()?;
    Some(ClrChangeEffect {
        clr_from: resolver.resolve(&from),
        clr_to: resolver.resolve(&to),
    })
}

// --- raw XML shapes ---

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawBlip {
    /// `<a:blip @r:embed>` — relationship id of the embedded media. After
    /// namespace strip the attribute reads as `embed`.
    #[serde(rename = "@embed")]
    pub embed: Option<String>,
    /// `<a:blip @r:link>` — relationship id of an externally linked media.
    /// Currently unused (we only render embedded media), but accepted so
    /// fixtures with linked images parse without error.
    #[serde(rename = "@link")]
    #[allow(dead_code)]
    pub link: Option<String>,
    /// `<a:alphaModFix amt="..."/>` — fix the image alpha to a specific
    /// percentage. `amt` is `ST_PositiveFixedPercentage` (0–100000 → 0–100%).
    /// Missing element or missing `amt` defaults to 100% (fully opaque).
    #[serde(rename = "alphaModFix")]
    pub alpha_mod_fix: Option<RawAlphaModFix>,
    /// `<a:grayscl/>` — empty marker element.
    pub grayscl: Option<EmptyMarker>,
    #[serde(rename = "biLevel")]
    pub bi_level: Option<RawBiLevel>,
    pub blur: Option<RawBlur>,
    pub lum: Option<RawLum>,
    pub duotone: Option<RawDuotone>,
    #[serde(rename = "clrChange")]
    pub clr_change: Option<RawClrChange>,
    /// `<a:extLst>` carrier — picks up Office's
    /// `<asvg:svgBlip r:embed="rIdN"/>` extension that points at the
    /// SVG version of the picture (Office 2016+ "icons" feature).
    #[serde(rename = "extLst", default)]
    pub ext_lst: Option<RawBlipExtLst>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct RawBlipExtLst {
    #[serde(rename = "ext", default)]
    pub exts: Vec<RawBlipExt>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct RawBlipExt {
    #[serde(rename = "svgBlip", default)]
    pub svg_blip: Option<RawSvgBlip>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct RawSvgBlip {
    #[serde(rename = "@embed", default)]
    pub embed: Option<String>,
}

impl RawBlip {
    /// Returns the SVG-extension `r:embed` if present.
    pub(crate) fn svg_embed(&self) -> Option<&str> {
        self.ext_lst
            .as_ref()?
            .exts
            .iter()
            .find_map(|e| e.svg_blip.as_ref()?.embed.as_deref())
    }

    /// Resolves the effective alpha multiplier from `<a:alphaModFix>`.
    /// Returns 1.0 when the element is absent or `amt` is missing —
    /// matching `PowerPoint`'s default of fully-opaque blip rendering.
    /// `amt` is clamped to `[0, 100000]` to guard against malformed inputs.
    pub(crate) fn alpha(&self) -> f64 {
        let Some(node) = self.alpha_mod_fix.as_ref() else {
            return 1.0;
        };
        let Some(raw) = node.amt.as_deref().and_then(|s| s.parse::<i64>().ok()) else {
            return 1.0;
        };
        (raw.clamp(0, 100_000) as f64) / 100_000.0
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct RawAlphaModFix {
    #[serde(rename = "@amt")]
    pub amt: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct EmptyMarker {}

#[derive(Debug, Deserialize)]
pub(crate) struct RawBiLevel {
    #[serde(rename = "@thresh")]
    pub thresh: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawBlur {
    #[serde(rename = "@rad")]
    pub rad: Option<i64>,
    #[serde(rename = "@grow")]
    pub grow: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawLum {
    #[serde(rename = "@bright")]
    pub bright: Option<String>,
    #[serde(rename = "@contrast")]
    pub contrast: Option<String>,
}

/// `<a:duotone>` — accepts an ordered sequence of color children
/// (`prstClr` / `srgbClr` / `schemeClr` / `sysClr`). The two source-order
/// children become `color1` / `color2`.
#[derive(Debug, Deserialize)]
pub(crate) struct RawDuotone {
    #[serde(rename = "$value", default)]
    pub colors: Vec<RawDuotoneColor>,
}

#[derive(Debug, Deserialize)]
pub(crate) enum RawDuotoneColor {
    #[serde(rename = "prstClr")]
    Preset(RawPrstClr),
    #[serde(rename = "srgbClr")]
    Srgb(RawSrgbClr),
    #[serde(rename = "schemeClr")]
    Scheme(RawSchemeClr),
    #[serde(rename = "sysClr")]
    System(RawSysClr),
}

impl RawDuotoneColor {
    fn resolve(&self, resolver: &ColorResolver) -> Option<ResolvedColor> {
        match self {
            Self::Preset(p) => p.to_resolved_color(),
            Self::Srgb(s) => s.to_color_ref().map(|r| resolver.resolve(&r)),
            Self::Scheme(s) => Some(resolver.resolve(&s.to_color_ref())),
            Self::System(s) => Some(resolver.resolve(&s.to_color_ref())),
        }
    }
}

/// `<a:prstClr val="black|white|red|...">` — a small fixed map of named
/// colors. The spec exposes only the eight base values; we share
/// the table with [`slideglance_color::resolve_preset`] so the resolver path and
/// this parser-local fallback agree.
#[derive(Debug, Deserialize)]
pub(crate) struct RawPrstClr {
    #[serde(rename = "@val")]
    pub val: String,
}

impl RawPrstClr {
    fn to_resolved_color(&self) -> Option<ResolvedColor> {
        let rgb = slideglance_color::resolve_preset(&self.val)?;
        Some(ResolvedColor::opaque(rgb))
    }
}

/// `<a:clrChange><a:clrFrom>...</a:clrFrom><a:clrTo>...</a:clrTo></a:clrChange>`.
#[derive(Debug, Deserialize)]
pub(crate) struct RawClrChange {
    #[serde(rename = "clrFrom")]
    pub clr_from: Option<RawColorChoice>,
    #[serde(rename = "clrTo")]
    pub clr_to: Option<RawColorChoice>,
}

// `ColorTransform` is referenced only via the pub-use chain elsewhere;
// silence the unused-import lint here.
#[allow(dead_code)]
type _UnusedTransform = ColorTransform;
#[allow(dead_code)]
type _UnusedColorRef = ColorRef;
