//! Raw XML deserialization shapes for color-bearing OOXML elements
//! (`<a:srgbClr>` / `<a:schemeClr>` / `<a:sysClr>`) plus the
//! `to_color_ref` extractor that lifts them into a [`slideglance_color::ColorRef`]
//! consumable by [`slideglance_color::ColorResolver`].
//!
//! These types are crate-private — every outward-facing parser builds
//! [`slideglance_model`] structures, not raw XML.

use serde::Deserialize;
use slideglance_color::{ColorRef, ColorTransform, PerMille, Rgb};

/// Container that captures any of the three color-choice variants OOXML
/// allows wherever a color child is permitted (`<a:gs>`, `<a:fgClr>`,
/// `<a:outerShdw>`, …). Exactly one of the three should be populated for a
/// well-formed document.
//
// All fields end in `_clr` because the OOXML element names are `srgbClr`
// / `schemeClr` / `sysClr` — keeping the suffix preserves the source-XML
// mapping the deserializer relies on.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct RawColorChoice {
    #[serde(rename = "srgbClr")]
    pub srgb_clr: Option<RawSrgbClr>,
    #[serde(rename = "schemeClr")]
    pub scheme_clr: Option<RawSchemeClr>,
    #[serde(rename = "sysClr")]
    pub sys_clr: Option<RawSysClr>,
    #[serde(rename = "prstClr")]
    pub prst_clr: Option<RawPrstClrChoice>,
}

impl RawColorChoice {
    /// Lifts the populated variant into a [`ColorRef`].
    pub fn to_color_ref(&self) -> Option<ColorRef> {
        if let Some(s) = &self.srgb_clr {
            return s.to_color_ref();
        }
        if let Some(s) = &self.scheme_clr {
            return Some(s.to_color_ref());
        }
        if let Some(s) = &self.sys_clr {
            return Some(s.to_color_ref());
        }
        if let Some(s) = &self.prst_clr {
            return Some(s.to_color_ref());
        }
        None
    }
}

/// `<a:srgbClr val="RRGGBB">` with optional inline transforms.
//
// `lumMod` / `lumOff` / `tint` / `shade` / `alpha` are inlined here (rather
// than nested via `#[serde(flatten)]`) because quick-xml's serde integration
// does not reliably flatten nested struct fields when they share the same
// element-vs-attribute namespace as the parent.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawSrgbClr {
    #[serde(rename = "@val")]
    pub val: String,
    #[serde(rename = "lumMod")]
    pub lum_mod: Option<RawValAttr>,
    #[serde(rename = "lumOff")]
    pub lum_off: Option<RawValAttr>,
    #[serde(rename = "satMod")]
    pub sat_mod: Option<RawValAttr>,
    #[serde(rename = "satOff")]
    pub sat_off: Option<RawValAttr>,
    pub tint: Option<RawValAttr>,
    pub shade: Option<RawValAttr>,
    pub alpha: Option<RawValAttr>,
}

impl RawSrgbClr {
    pub fn to_color_ref(&self) -> Option<ColorRef> {
        let rgb = Rgb::from_hex(&self.val).ok()?;
        Some(ColorRef::Srgb {
            rgb,
            transform: build_transform_full(
                self.lum_mod.as_ref(),
                self.lum_off.as_ref(),
                self.sat_mod.as_ref(),
                self.sat_off.as_ref(),
                self.tint.as_ref(),
                self.shade.as_ref(),
                self.alpha.as_ref(),
            ),
        })
    }
}

/// `<a:schemeClr val="accent1|tx1|...">` with optional inline transforms.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawSchemeClr {
    #[serde(rename = "@val")]
    pub val: String,
    #[serde(rename = "lumMod")]
    pub lum_mod: Option<RawValAttr>,
    #[serde(rename = "lumOff")]
    pub lum_off: Option<RawValAttr>,
    #[serde(rename = "satMod")]
    pub sat_mod: Option<RawValAttr>,
    #[serde(rename = "satOff")]
    pub sat_off: Option<RawValAttr>,
    pub tint: Option<RawValAttr>,
    pub shade: Option<RawValAttr>,
    pub alpha: Option<RawValAttr>,
}

impl RawSchemeClr {
    pub fn to_color_ref(&self) -> ColorRef {
        ColorRef::Scheme {
            name: self.val.clone(),
            transform: build_transform_full(
                self.lum_mod.as_ref(),
                self.lum_off.as_ref(),
                self.sat_mod.as_ref(),
                self.sat_off.as_ref(),
                self.tint.as_ref(),
                self.shade.as_ref(),
                self.alpha.as_ref(),
            ),
        }
    }
}

/// `<a:sysClr val="windowText" lastClr="000000">` with optional inline
/// transforms. `lastClr` is the implementation-specific resolved value;
/// when absent we fall back to black (the OOXML default).
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawSysClr {
    #[serde(rename = "@val")]
    #[allow(dead_code)] // Reserved for future system-color name dispatch.
    pub val: Option<String>,
    #[serde(rename = "@lastClr")]
    pub last_clr: Option<String>,
    #[serde(rename = "lumMod")]
    pub lum_mod: Option<RawValAttr>,
    #[serde(rename = "lumOff")]
    pub lum_off: Option<RawValAttr>,
    #[serde(rename = "satMod")]
    pub sat_mod: Option<RawValAttr>,
    #[serde(rename = "satOff")]
    pub sat_off: Option<RawValAttr>,
    pub tint: Option<RawValAttr>,
    pub shade: Option<RawValAttr>,
    pub alpha: Option<RawValAttr>,
}

impl RawSysClr {
    pub fn to_color_ref(&self) -> ColorRef {
        let last = self
            .last_clr
            .as_deref()
            .and_then(|h| Rgb::from_hex(h).ok())
            .unwrap_or(Rgb::new(0, 0, 0));
        ColorRef::System {
            last,
            transform: build_transform_full(
                self.lum_mod.as_ref(),
                self.lum_off.as_ref(),
                self.sat_mod.as_ref(),
                self.sat_off.as_ref(),
                self.tint.as_ref(),
                self.shade.as_ref(),
                self.alpha.as_ref(),
            ),
        }
    }
}

/// `<a:prstClr val="black|white|red|...">` with optional inline transforms.
/// Resolved through [`slideglance_color::resolve_preset`] in the central resolver;
/// unknown names fall back to black there. Wrapper named with the `Choice`
/// suffix to avoid colliding with the existing `RawPrstClr` in
/// `crate::blip_effect` which exposes a different (Option-returning) API.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawPrstClrChoice {
    #[serde(rename = "@val")]
    pub val: String,
    #[serde(rename = "lumMod")]
    pub lum_mod: Option<RawValAttr>,
    #[serde(rename = "lumOff")]
    pub lum_off: Option<RawValAttr>,
    #[serde(rename = "satMod")]
    pub sat_mod: Option<RawValAttr>,
    #[serde(rename = "satOff")]
    pub sat_off: Option<RawValAttr>,
    pub tint: Option<RawValAttr>,
    pub shade: Option<RawValAttr>,
    pub alpha: Option<RawValAttr>,
}

impl RawPrstClrChoice {
    pub fn to_color_ref(&self) -> ColorRef {
        ColorRef::Preset {
            name: self.val.clone(),
            transform: build_transform_full(
                self.lum_mod.as_ref(),
                self.lum_off.as_ref(),
                self.sat_mod.as_ref(),
                self.sat_off.as_ref(),
                self.tint.as_ref(),
                self.shade.as_ref(),
                self.alpha.as_ref(),
            ),
        }
    }
}

/// `<a:* val="...">` value-only attribute carrier (lumMod / lumOff / tint
/// / shade / alpha / pos / etc.).
//
// `val` is `String` because quick-xml's serde de surfaces XML attribute
// values as strings for deeply-nested children even when the field is typed
// as a number — top-level attributes parse straight into integers but
// `Option<RawValAttr>` carrying an `i32`/`i64` field fails with
// "invalid type: string ...". We accept the string and parse on demand.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawValAttr {
    #[serde(rename = "@val")]
    pub val: String,
}

impl RawValAttr {
    /// Returns the parsed numeric value for use as a `PerMille` amount.
    /// Returns `0` if the input is malformed (matches the spec's
    /// `Number(...)` coercion of `NaN` -> `0`).
    pub fn as_per_mille_i32(&self) -> i32 {
        self.val.parse::<i64>().ok().map_or(0, |v| {
            let saturated = if v < 0 { i32::MIN } else { i32::MAX };
            i32::try_from(v).unwrap_or(saturated)
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn build_transform_full(
    lum_mod: Option<&RawValAttr>,
    lum_off: Option<&RawValAttr>,
    sat_mod: Option<&RawValAttr>,
    sat_off: Option<&RawValAttr>,
    tint: Option<&RawValAttr>,
    shade: Option<&RawValAttr>,
    alpha: Option<&RawValAttr>,
) -> ColorTransform {
    ColorTransform {
        lum_mod: lum_mod.map(|v| PerMille::new(v.as_per_mille_i32())),
        lum_off: lum_off.map(|v| PerMille::new(v.as_per_mille_i32())),
        sat_mod: sat_mod.map(|v| PerMille::new(v.as_per_mille_i32())),
        sat_off: sat_off.map(|v| PerMille::new(v.as_per_mille_i32())),
        tint: tint.map(|v| PerMille::new(v.as_per_mille_i32())),
        shade: shade.map(|v| PerMille::new(v.as_per_mille_i32())),
        alpha: alpha.map(|v| PerMille::new(v.as_per_mille_i32())),
    }
}
