//! Effect types: outer/inner shadow, glow, soft edge, and image (blip)
//! effects (grayscale, biLevel, blur, lum, duotone, color change).

use slideglance_color::ResolvedColor;
use slideglance_utils::Emu;

/// `<a:effectLst>` — fixed-shape list of the supported shape effects.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EffectList {
    /// `<a:outerShdw>`.
    pub outer_shadow: Option<OuterShadow>,
    /// `<a:innerShdw>`.
    pub inner_shadow: Option<InnerShadow>,
    /// `<a:glow>`.
    pub glow: Option<Glow>,
    /// `<a:softEdge>`.
    pub soft_edge: Option<SoftEdge>,
}

/// `<a:outerShdw>`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OuterShadow {
    /// Blur radius.
    pub blur_radius: Emu,
    /// Distance from shape edge.
    pub distance: Emu,
    /// Direction in degrees (0 = right, 90 = down).
    pub direction: f64,
    /// Shadow color.
    pub color: ResolvedColor,
    /// `@algn` value (`tl`, `t`, `tr`, `l`, `ctr`, `r`, `bl`, `b`, `br`).
    pub alignment: String,
    /// `@rotWithShape`.
    pub rotate_with_shape: bool,
}

/// `<a:innerShdw>`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InnerShadow {
    /// Blur radius.
    pub blur_radius: Emu,
    /// Distance from shape edge.
    pub distance: Emu,
    /// Direction in degrees.
    pub direction: f64,
    /// Shadow color.
    pub color: ResolvedColor,
}

/// `<a:glow>`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Glow {
    /// Glow radius.
    pub radius: Emu,
    /// Glow color.
    pub color: ResolvedColor,
}

/// `<a:softEdge>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SoftEdge {
    /// Soft-edge radius.
    pub radius: Emu,
}

/// Image (blip) effects applied inside an `<a:blipFill>`.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlipEffects {
    /// `<a:grayscl/>`.
    pub grayscale: bool,
    /// `<a:biLevel>`.
    pub bi_level: Option<BiLevelEffect>,
    /// `<a:blur>`.
    pub blur: Option<BlurEffect>,
    /// `<a:lum>` brightness/contrast.
    pub lum: Option<LumEffect>,
    /// `<a:duotone>` two-color recolor.
    pub duotone: Option<DuotoneEffect>,
    /// `<a:clrChange>` color replacement.
    pub clr_change: Option<ClrChangeEffect>,
}

/// `<a:biLevel>` — threshold on luminance.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BiLevelEffect {
    /// Threshold in `[0, 1]`.
    pub threshold: f64,
}

/// `<a:blur>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlurEffect {
    /// Blur radius.
    pub radius: Emu,
    /// `@grow` — when true, the blurred image extends past the container.
    pub grow: bool,
}

/// `<a:lum>` — brightness/contrast adjustment.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LumEffect {
    /// Brightness in `[-1, 1]`.
    pub brightness: f64,
    /// Contrast in `[-1, 1]`.
    pub contrast: f64,
}

/// `<a:duotone>` — two-color image recoloring.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DuotoneEffect {
    /// First duotone color.
    pub color1: ResolvedColor,
    /// Second duotone color.
    pub color2: ResolvedColor,
}

/// `<a:clrChange>` — single-color replacement (from -> to).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClrChangeEffect {
    /// Source color to replace.
    pub clr_from: ResolvedColor,
    /// Replacement color.
    pub clr_to: ResolvedColor,
}
