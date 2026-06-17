//! Theme definitions: font scheme + format scheme. Color scheme/map types are
//! re-exported from [`slideglance_color`] (see crate root).

use std::collections::BTreeMap;

use crate::effect::EffectList;
use crate::fill::Fill;
use crate::line::Outline;
use slideglance_color::{ColorMap, ColorScheme};

/// Theme block from `theme1.xml` (`<a:theme>`).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Theme {
    /// Theme color slots (`<a:clrScheme>`).
    pub color_scheme: ColorScheme,
    /// Theme font scheme (`<a:fontScheme>`).
    pub font_scheme: FontScheme,
    /// Optional format scheme (`<a:fmtScheme>`).
    pub fmt_scheme: Option<FormatScheme>,
    /// Document-level color map (`<p:clrMap>`). Lives on `theme1` only via
    /// the slide master that references the theme; included here for
    /// downstream convenience.
    pub color_map: Option<ColorMap>,
}

/// `<a:fontScheme>` — major/minor (Latin/East-Asian/Complex) font names plus
/// every `<a:font script="...">` script-based fallback collected into
/// `*_script_fonts`.
///
/// **CJK equality**: every script in the source XML is collected equally —
/// `Jpan`, `Hang`, `Hans`, `Hant`, and any other ISO 15924 codes. The TS
/// reference (the spec) extracts only `Jpan`; per the project rule
/// "CJK Script Equality" in `CLAUDE.md`, we intentionally diverge here so
/// Korean and Chinese presentations get the same fidelity as Japanese.
//
// `major_font_jpan` / `minor_font_jpan` are kept as convenience accessors
// (lookups into `*_script_fonts`) for wire-compatibility with code that may
// have been written against the TS shape; new code should read
// `*_script_fonts` directly.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FontScheme {
    /// `<a:majorFont><a:latin>` typeface.
    pub major_font: String,
    /// `<a:minorFont><a:latin>` typeface.
    pub minor_font: String,
    /// `<a:majorFont><a:ea>` typeface.
    pub major_font_ea: Option<String>,
    /// `<a:minorFont><a:ea>` typeface.
    pub minor_font_ea: Option<String>,
    /// `<a:majorFont><a:cs>` typeface.
    pub major_font_cs: Option<String>,
    /// `<a:minorFont><a:cs>` typeface.
    pub minor_font_cs: Option<String>,
    /// Script-keyed major-font fallbacks (`<a:majorFont><a:font script="...">`).
    /// Keys are ISO 15924 script codes (`"Jpan"`, `"Hang"`, `"Hans"`, `"Hant"`,
    /// `"Arab"`, `"Thai"`, …). All scripts present in the source XML are
    /// stored.
    pub major_script_fonts: BTreeMap<String, String>,
    /// Script-keyed minor-font fallbacks (`<a:minorFont><a:font script="...">`).
    pub minor_script_fonts: BTreeMap<String, String>,
}

impl FontScheme {
    /// Returns the major-font typeface for ISO 15924 script `code`, if any.
    #[must_use]
    pub fn major_script_font(&self, code: &str) -> Option<&str> {
        self.major_script_fonts.get(code).map(String::as_str)
    }

    /// Returns the minor-font typeface for ISO 15924 script `code`, if any.
    #[must_use]
    pub fn minor_script_font(&self, code: &str) -> Option<&str> {
        self.minor_script_fonts.get(code).map(String::as_str)
    }
}

/// `<a:fmtScheme>` — the four reusable shape style libraries (fill / line /
/// effect / background fill).
//
// All four field names end in "_styles" because the OOXML elements are
// `fillStyleLst`, `lnStyleLst`, `effectStyleLst`, `bgFillStyleLst` — we keep
// the suffix to mirror the source-XML mapping.
#[allow(clippy::struct_field_names)]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FormatScheme {
    /// `<a:fillStyleLst>` — usually 3 styles (subtle / moderate / intense).
    pub fill_styles: Vec<Fill>,
    /// `<a:lnStyleLst>` — outline styles.
    pub ln_styles: Vec<Outline>,
    /// `<a:effectStyleLst>` — effect lists. `None` represents an
    /// `<a:effectStyle/>` with no effects (TS uses `null`).
    pub effect_styles: Vec<Option<EffectList>>,
    /// `<a:bgFillStyleLst>` — background fill styles.
    pub bg_fill_styles: Vec<Fill>,
}
