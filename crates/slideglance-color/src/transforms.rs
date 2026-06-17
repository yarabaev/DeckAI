//! ECMA-376 §20.1.2.3 color transforms.
//!
//! OOXML stores transform amounts as integer per-mille values (1/100000),
//! e.g. `<lumMod val="50000"/>` means 50%. [`PerMille`] preserves this raw
//! integer; [`apply_color_transforms`] applies the transforms in the same
//! order as the TypeScript reference: luminance first (HSL space), then tint,
//! then shade, then alpha.

use crate::hsl::Hsl;
use crate::rgb::{ResolvedColor, Rgb};

/// Per-mille fraction (1/100000), the raw OOXML transform amount unit.
///
/// Example: `PerMille(50000)` represents 50%.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PerMille(pub i32);

impl PerMille {
    /// Wraps a raw integer per-mille value.
    #[inline]
    #[must_use]
    pub const fn new(value: i32) -> Self {
        Self(value)
    }

    /// Converts to a fractional `f64` (e.g. `50000` -> `0.5`).
    #[inline]
    #[must_use]
    pub fn as_f64(self) -> f64 {
        f64::from(self.0) / 100_000.0
    }
}

/// Set of OOXML color transforms applied to a base color.
///
/// Mirrors the elements found inside an `<a:srgbClr>` / `<a:schemeClr>` /
/// `<a:sysClr>` node. Fields are `Option`al because each is independently
/// optional in the source XML.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColorTransform {
    /// `<a:lumMod val="..."/>` — multiplies HSL luminance.
    pub lum_mod: Option<PerMille>,
    /// `<a:lumOff val="..."/>` — adds to HSL luminance.
    pub lum_off: Option<PerMille>,
    /// `<a:satMod val="..."/>` — multiplies HSL saturation.
    ///
    /// Project-level intentional divergence from the spec
    /// (the spec) which silently drops `satMod`. Computed in HSL
    /// space alongside `lumMod` / `lumOff` so a single round-trip
    /// through Hsl/Rgb keeps numerical drift identical to the TS path
    /// when no satMod is set. See `feedback_ts_reference_parity.md` —
    /// CJK-equality-style explicit-divergence pattern; documented in
    /// memory `project_slideglance_color_omissions.md` (now resolved).
    pub sat_mod: Option<PerMille>,
    /// `<a:satOff val="..."/>` — adds to HSL saturation. Same divergence
    /// rationale as [`Self::sat_mod`].
    pub sat_off: Option<PerMille>,
    /// `<a:tint val="..."/>` — RGB blend toward white.
    pub tint: Option<PerMille>,
    /// `<a:shade val="..."/>` — RGB blend toward black.
    pub shade: Option<PerMille>,
    /// `<a:alpha val="..."/>` — opacity (overrides any prior alpha).
    pub alpha: Option<PerMille>,
}

/// Applies a [`ColorTransform`] to a [`ResolvedColor`], producing a new
/// `ResolvedColor` with luminance/saturation/tint/shade adjustments and
/// the alpha set.
///
/// Order matches the TypeScript reference for shared transforms:
/// luminance → tint → shade → alpha. Saturation modifiers (`sat_mod`,
/// `sat_off`) are applied together with luminance in a single HSL round
/// trip — they share an HSL conversion so values stay numerically close
/// to the TS path when only luminance is set.
#[must_use]
pub fn apply_color_transforms(input: ResolvedColor, transform: &ColorTransform) -> ResolvedColor {
    let mut rgb = input.rgb;
    let mut alpha = input.alpha;

    let needs_hsl = transform.lum_mod.is_some()
        || transform.lum_off.is_some()
        || transform.sat_mod.is_some()
        || transform.sat_off.is_some();
    if needs_hsl {
        rgb = apply_hsl_adjustments(
            rgb,
            transform.lum_mod,
            transform.lum_off,
            transform.sat_mod,
            transform.sat_off,
        );
    }

    if let Some(t) = transform.tint {
        rgb = apply_tint(rgb, t.as_f64());
    }

    if let Some(s) = transform.shade {
        rgb = apply_shade(rgb, s.as_f64());
    }

    if let Some(a) = transform.alpha {
        alpha = a.as_f64();
    }

    ResolvedColor { rgb, alpha }
}

fn apply_hsl_adjustments(
    rgb: Rgb,
    lum_mod: Option<PerMille>,
    lum_off: Option<PerMille>,
    sat_mod: Option<PerMille>,
    sat_off: Option<PerMille>,
) -> Rgb {
    let Hsl { h, s, l } = Hsl::from_rgb(rgb);
    let sm = sat_mod.map_or(1.0, PerMille::as_f64);
    let so = sat_off.map_or(0.0, PerMille::as_f64);
    let lm = lum_mod.map_or(1.0, PerMille::as_f64);
    let lo = lum_off.map_or(0.0, PerMille::as_f64);
    let new_s = (s * sm + so).clamp(0.0, 1.0);
    let new_l = (l * lm + lo).clamp(0.0, 1.0);
    Hsl::new(h, new_s, new_l).to_rgb()
}

fn apply_tint(rgb: Rgb, tint: f64) -> Rgb {
    Rgb::new(
        round_clamp_u8(f64::from(rgb.r) + (255.0 - f64::from(rgb.r)) * tint),
        round_clamp_u8(f64::from(rgb.g) + (255.0 - f64::from(rgb.g)) * tint),
        round_clamp_u8(f64::from(rgb.b) + (255.0 - f64::from(rgb.b)) * tint),
    )
}

fn apply_shade(rgb: Rgb, shade: f64) -> Rgb {
    Rgb::new(
        round_clamp_u8(f64::from(rgb.r) * shade),
        round_clamp_u8(f64::from(rgb.g) * shade),
        round_clamp_u8(f64::from(rgb.b) * shade),
    )
}

fn round_clamp_u8(v: f64) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}
