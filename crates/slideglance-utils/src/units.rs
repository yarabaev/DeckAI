//! Newtype wrappers for OOXML length units.
//!
//! These types are `#[repr(transparent)]` zero-cost wrappers; the only
//! difference from a raw integer/float at runtime is that conversions across
//! unit boundaries must go through explicit methods, preventing accidental
//! arithmetic on values of different units.

use crate::constants::{DEFAULT_DPI, EMU_PER_INCH, EMU_PER_POINT};

/// English Metric Units. The native coordinate unit of OOXML.
///
/// 1 inch = 914,400 EMU. A 16:9 slide is 9,144,000 × 5,143,500 EMU.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Emu(pub i64);

impl Emu {
    /// Wraps a raw `i64` as an `Emu` value.
    #[inline]
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// Constructs an `Emu` from an `f64`. Truncates toward zero.
    #[inline]
    #[must_use]
    pub fn from_f64(value: f64) -> Self {
        Self(value as i64)
    }

    /// Returns the underlying raw EMU count.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> i64 {
        self.0
    }

    /// Converts this `Emu` to pixels at the default 96 DPI.
    #[inline]
    #[must_use]
    pub fn to_pixels(self) -> f64 {
        self.to_pixels_at(DEFAULT_DPI)
    }

    /// Converts this `Emu` to pixels at the given DPI.
    #[inline]
    #[must_use]
    pub fn to_pixels_at(self, dpi: u32) -> f64 {
        (self.0 as f64 / EMU_PER_INCH as f64) * f64::from(dpi)
    }

    /// Converts this `Emu` to points (1 pt = 12,700 EMU).
    #[inline]
    #[must_use]
    pub fn to_points(self) -> Pt {
        Pt(self.0 as f64 / EMU_PER_POINT as f64)
    }
}

/// Point — 1/72 of an inch. Used for font sizes and OOXML spacing values
/// after `HundredthPt` is divided by 100.
#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pt(pub f64);

impl Pt {
    /// Wraps a raw `f64` as a `Pt` value.
    #[inline]
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    /// Returns the underlying raw point count.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> f64 {
        self.0
    }

    /// Converts this `Pt` to `Emu` (1 pt = 12,700 EMU).
    #[inline]
    #[must_use]
    pub fn to_emu(self) -> Emu {
        Emu((self.0 * EMU_PER_POINT as f64) as i64)
    }

    /// Converts this `Pt` to pixels at the given DPI (1 pt = 1/72 inch).
    #[inline]
    #[must_use]
    pub fn to_pixels_at(self, dpi: u32) -> f64 {
        self.0 * f64::from(dpi) / 72.0
    }
}

/// 1/100 of a point. Used by ECMA-376 paragraph spacing properties such as
/// `spcPts` and `lnSpc`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HundredthPt(pub i64);

impl HundredthPt {
    /// Wraps a raw `i64` as a `HundredthPt`.
    #[inline]
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// Returns the underlying raw count.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> i64 {
        self.0
    }

    /// Converts this `HundredthPt` to `Pt`.
    #[inline]
    #[must_use]
    pub fn to_points(self) -> Pt {
        Pt(self.0 as f64 / 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // emuToPixels parity with the spec.
    #[test]
    fn emu_to_pixels_one_inch_at_96_dpi() {
        assert!((Emu::new(914_400).to_pixels() - 96.0).abs() < f64::EPSILON);
    }

    #[test]
    fn emu_to_pixels_zero() {
        assert_eq!(Emu::new(0).to_pixels(), 0.0);
    }

    #[test]
    fn emu_to_pixels_standard_slide_width() {
        assert!((Emu::new(9_144_000).to_pixels() - 960.0).abs() < f64::EPSILON);
    }

    #[test]
    fn emu_to_pixels_standard_slide_height() {
        // 5,143,500 EMU = 540 px (rounded; actual is 540.0)
        assert!((Emu::new(5_143_500).to_pixels() - 540.0).abs() < 0.5);
    }

    #[test]
    fn emu_to_pixels_supports_custom_dpi() {
        assert!((Emu::new(914_400).to_pixels_at(72) - 72.0).abs() < f64::EPSILON);
    }

    #[test]
    fn emu_to_points_one_point() {
        assert!((Emu::new(12_700).to_points().raw() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn pt_to_emu_round_trip() {
        let pt = Pt::new(28.0);
        assert_eq!(pt.to_emu().raw(), 28 * EMU_PER_POINT);
    }

    #[test]
    fn pt_to_pixels_at_96_dpi() {
        // 12 pt = 16 px at 96 DPI (12 * 96 / 72)
        assert!((Pt::new(12.0).to_pixels_at(96) - 16.0).abs() < f64::EPSILON);
    }

    #[test]
    fn hundredth_pt_to_pt() {
        assert!((HundredthPt::new(2800).to_points().raw() - 28.0).abs() < f64::EPSILON);
    }

    #[test]
    fn hundredth_pt_zero() {
        assert!((HundredthPt::new(0).to_points().raw() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn raw_accessors_are_const() {
        const E: i64 = Emu::new(123).raw();
        const H: i64 = HundredthPt::new(456).raw();
        assert_eq!(E, 123);
        assert_eq!(H, 456);
    }
}
