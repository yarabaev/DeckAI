//! Theme color scheme types per ECMA-376 §20.1.6.

use core::fmt;
use core::str::FromStr;

use crate::rgb::Rgb;
use crate::transforms::ColorTransform;

/// Identifier of a slot in a [`ColorScheme`].
///
/// These are the 12 fixed slots ECMA-376 defines on `<a:clrScheme>`:
/// `dk1`, `lt1`, `dk2`, `lt2`, `accent1`–`accent6`, `hlink`, `folHlink`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SchemeColorKey {
    /// Dark 1 (typically used for primary text).
    Dk1,
    /// Light 1 (typically used for primary background).
    Lt1,
    /// Dark 2.
    Dk2,
    /// Light 2.
    Lt2,
    /// Accent 1.
    Accent1,
    /// Accent 2.
    Accent2,
    /// Accent 3.
    Accent3,
    /// Accent 4.
    Accent4,
    /// Accent 5.
    Accent5,
    /// Accent 6.
    Accent6,
    /// Hyperlink color.
    Hlink,
    /// Followed hyperlink color.
    FolHlink,
}

impl SchemeColorKey {
    /// Returns the canonical OOXML attribute string for this key
    /// (e.g. `"accent1"`, `"folHlink"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dk1 => "dk1",
            Self::Lt1 => "lt1",
            Self::Dk2 => "dk2",
            Self::Lt2 => "lt2",
            Self::Accent1 => "accent1",
            Self::Accent2 => "accent2",
            Self::Accent3 => "accent3",
            Self::Accent4 => "accent4",
            Self::Accent5 => "accent5",
            Self::Accent6 => "accent6",
            Self::Hlink => "hlink",
            Self::FolHlink => "folHlink",
        }
    }
}

impl FromStr for SchemeColorKey {
    type Err = UnknownSchemeName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dk1" => Ok(Self::Dk1),
            "lt1" => Ok(Self::Lt1),
            "dk2" => Ok(Self::Dk2),
            "lt2" => Ok(Self::Lt2),
            "accent1" => Ok(Self::Accent1),
            "accent2" => Ok(Self::Accent2),
            "accent3" => Ok(Self::Accent3),
            "accent4" => Ok(Self::Accent4),
            "accent5" => Ok(Self::Accent5),
            "accent6" => Ok(Self::Accent6),
            "hlink" => Ok(Self::Hlink),
            "folHlink" => Ok(Self::FolHlink),
            _ => Err(UnknownSchemeName(s.to_owned())),
        }
    }
}

/// Returned by [`SchemeColorKey::from_str`] when the name is not one of the
/// 12 canonical slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSchemeName(
    /// The unrecognized name as supplied.
    pub String,
);

impl fmt::Display for UnknownSchemeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown scheme color name: {}", self.0)
    }
}

impl std::error::Error for UnknownSchemeName {}

/// The 12 theme colors defined on `<a:clrScheme>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColorScheme {
    /// `dk1` slot.
    pub dk1: Rgb,
    /// `lt1` slot.
    pub lt1: Rgb,
    /// `dk2` slot.
    pub dk2: Rgb,
    /// `lt2` slot.
    pub lt2: Rgb,
    /// `accent1` slot.
    pub accent1: Rgb,
    /// `accent2` slot.
    pub accent2: Rgb,
    /// `accent3` slot.
    pub accent3: Rgb,
    /// `accent4` slot.
    pub accent4: Rgb,
    /// `accent5` slot.
    pub accent5: Rgb,
    /// `accent6` slot.
    pub accent6: Rgb,
    /// `hlink` slot.
    pub hlink: Rgb,
    /// `folHlink` slot.
    pub fol_hlink: Rgb,
}

impl ColorScheme {
    /// Returns the color stored at the given slot.
    #[must_use]
    pub fn get(&self, key: SchemeColorKey) -> Rgb {
        match key {
            SchemeColorKey::Dk1 => self.dk1,
            SchemeColorKey::Lt1 => self.lt1,
            SchemeColorKey::Dk2 => self.dk2,
            SchemeColorKey::Lt2 => self.lt2,
            SchemeColorKey::Accent1 => self.accent1,
            SchemeColorKey::Accent2 => self.accent2,
            SchemeColorKey::Accent3 => self.accent3,
            SchemeColorKey::Accent4 => self.accent4,
            SchemeColorKey::Accent5 => self.accent5,
            SchemeColorKey::Accent6 => self.accent6,
            SchemeColorKey::Hlink => self.hlink,
            SchemeColorKey::FolHlink => self.fol_hlink,
        }
    }
}

/// Maps document-level color names (`bg1`, `tx1`, `accent1`, …) to their
/// underlying [`SchemeColorKey`] slots, per `<p:clrMap>`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColorMap {
    /// `bg1` -> typically `Lt1`.
    pub bg1: SchemeColorKey,
    /// `tx1` -> typically `Dk1`.
    pub tx1: SchemeColorKey,
    /// `bg2` -> typically `Lt2`.
    pub bg2: SchemeColorKey,
    /// `tx2` -> typically `Dk2`.
    pub tx2: SchemeColorKey,
    /// `accent1` slot mapping.
    pub accent1: SchemeColorKey,
    /// `accent2` slot mapping.
    pub accent2: SchemeColorKey,
    /// `accent3` slot mapping.
    pub accent3: SchemeColorKey,
    /// `accent4` slot mapping.
    pub accent4: SchemeColorKey,
    /// `accent5` slot mapping.
    pub accent5: SchemeColorKey,
    /// `accent6` slot mapping.
    pub accent6: SchemeColorKey,
    /// `hlink` slot mapping.
    pub hlink: SchemeColorKey,
    /// `folHlink` slot mapping.
    pub fol_hlink: SchemeColorKey,
}

/// The default color mapping per ECMA-376 §20.1.6.2 — `bg1`→`Lt1`,
/// `tx1`→`Dk1`, `bg2`→`Lt2`, `tx2`→`Dk2`, accents and links pass through
/// unchanged. Used by parsers that need a `ColorMap` before they have read
/// the slide-master's `<p:clrMap>` (e.g. theme `fmtScheme` resolution).
impl Default for ColorMap {
    fn default() -> Self {
        Self {
            bg1: SchemeColorKey::Lt1,
            tx1: SchemeColorKey::Dk1,
            bg2: SchemeColorKey::Lt2,
            tx2: SchemeColorKey::Dk2,
            accent1: SchemeColorKey::Accent1,
            accent2: SchemeColorKey::Accent2,
            accent3: SchemeColorKey::Accent3,
            accent4: SchemeColorKey::Accent4,
            accent5: SchemeColorKey::Accent5,
            accent6: SchemeColorKey::Accent6,
            hlink: SchemeColorKey::Hlink,
            fol_hlink: SchemeColorKey::FolHlink,
        }
    }
}

impl ColorMap {
    /// Resolves a document-level color reference name to a scheme slot.
    ///
    /// Lookup order (matching ECMA-376 / the spec):
    /// 1. The 12 `ColorMap` keys (`bg1`, `tx1`, `bg2`, `tx2`, `accent1`–
    ///    `accent6`, `hlink`, `folHlink`).
    /// 2. The 12 [`SchemeColorKey`] names directly.
    /// 3. Fallback: [`SchemeColorKey::Dk1`].
    #[must_use]
    pub fn resolve_name(&self, name: &str) -> SchemeColorKey {
        match name {
            "bg1" => self.bg1,
            "tx1" => self.tx1,
            "bg2" => self.bg2,
            "tx2" => self.tx2,
            "accent1" => self.accent1,
            "accent2" => self.accent2,
            "accent3" => self.accent3,
            "accent4" => self.accent4,
            "accent5" => self.accent5,
            "accent6" => self.accent6,
            "hlink" => self.hlink,
            "folHlink" => self.fol_hlink,
            other => SchemeColorKey::from_str(other).unwrap_or(SchemeColorKey::Dk1),
        }
    }
}

/// An unresolved color reference as it appears inside a fill/text/effect
/// element. Phase 3 (parser) constructs these; [`crate::ColorResolver`]
/// resolves them into [`crate::ResolvedColor`].
#[derive(Clone, Debug, PartialEq)]
pub enum ColorRef {
    /// `<a:srgbClr val="RRGGBB">` — direct sRGB reference.
    Srgb {
        /// The sRGB color carried by the attribute.
        rgb: Rgb,
        /// Inline transforms applied to it.
        transform: ColorTransform,
    },
    /// `<a:schemeClr val="...">` — theme color reference (e.g. `accent1`,
    /// `bg1`). The name is resolved through the document's [`ColorMap`].
    Scheme {
        /// The scheme name (`bg1`, `tx1`, `accent1`, `dk1`, …) as stored in
        /// the source XML.
        name: String,
        /// Inline transforms applied to it.
        transform: ColorTransform,
    },
    /// `<a:sysClr val="..." lastClr="RRGGBB">` — system color whose
    /// implementation-specific value was last serialized as `lastClr`.
    System {
        /// Last serialized concrete color (used as the resolved value).
        last: Rgb,
        /// Inline transforms applied to it.
        transform: ColorTransform,
    },
    /// `<a:prstClr val="black|white|red|...">` — OOXML named preset color.
    /// Resolution uses the small TS-parity preset table
    /// (`crate::presets::resolve_preset`); unknown names fall back to black
    /// in [`ColorResolver::resolve`], matching the resolver's existing
    /// convention for unknown scheme references.
    Preset {
        /// Preset name as stored in the source XML (case-sensitive).
        name: String,
        /// Inline transforms applied to it.
        transform: ColorTransform,
    },
}
