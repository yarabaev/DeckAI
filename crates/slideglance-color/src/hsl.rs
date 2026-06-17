//! HSL color space conversions per W3C CSS Color Module Level 3 §4.2.4.
//!
//! All channels are in `[0, 1]`. The algorithms here mirror the TypeScript
//! reference (the spec) operation-for-operation so that round-trip
//! conversions are bit-identical between the two implementations on the same
//! IEEE-754 inputs.

use crate::rgb::Rgb;

/// HSL triple in the range `[0, 1]` for each channel.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Hsl {
    /// Hue, normalized to `[0, 1]`.
    pub h: f64,
    /// Saturation, in `[0, 1]`.
    pub s: f64,
    /// Lightness, in `[0, 1]`.
    pub l: f64,
}

impl Hsl {
    /// Constructs an HSL triple.
    #[inline]
    #[must_use]
    pub const fn new(h: f64, s: f64, l: f64) -> Self {
        Self { h, s, l }
    }

    /// Converts an `Rgb` to `Hsl`.
    // Single-character bindings r/g/b/h/s/l/min/max match the mathematical
    // notation in W3C CSS Color Module Level 3 §4.2.4.
    #[allow(clippy::many_single_char_names)]
    #[must_use]
    pub fn from_rgb(rgb: Rgb) -> Self {
        let r = f64::from(rgb.r) / 255.0;
        let g = f64::from(rgb.g) / 255.0;
        let b = f64::from(rgb.b) / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = max.midpoint(min);

        if max == min {
            return Self { h: 0.0, s: 0.0, l };
        }

        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };

        let h = if max == r {
            ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
        } else if max == g {
            ((b - r) / d + 2.0) / 6.0
        } else {
            ((r - g) / d + 4.0) / 6.0
        };

        Self { h, s, l }
    }

    /// Converts this `Hsl` back to `Rgb`.
    #[allow(clippy::many_single_char_names)]
    #[must_use]
    pub fn to_rgb(self) -> Rgb {
        let Self { h, s, l } = self;

        if s == 0.0 {
            let v = round_to_u8(l * 255.0);
            return Rgb::new(v, v, v);
        }

        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;

        let r = round_to_u8(hue_to_rgb(p, q, h + 1.0 / 3.0) * 255.0);
        let g = round_to_u8(hue_to_rgb(p, q, h) * 255.0);
        let b = round_to_u8(hue_to_rgb(p, q, h - 1.0 / 3.0) * 255.0);

        Rgb::new(r, g, b)
    }
}

fn hue_to_rgb(p: f64, q: f64, t: f64) -> f64 {
    let mut tt = t;
    if tt < 0.0 {
        tt += 1.0;
    }
    if tt > 1.0 {
        tt -= 1.0;
    }
    if tt < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * tt;
    }
    if tt < 1.0 / 2.0 {
        return q;
    }
    if tt < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - tt) * 6.0;
    }
    p
}

fn round_to_u8(v: f64) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_pure_red() {
        let rgb = Rgb::new(0xFF, 0x00, 0x00);
        assert_eq!(Hsl::from_rgb(rgb).to_rgb(), rgb);
    }

    #[test]
    fn roundtrips_pure_green() {
        let rgb = Rgb::new(0x00, 0xFF, 0x00);
        assert_eq!(Hsl::from_rgb(rgb).to_rgb(), rgb);
    }

    #[test]
    fn roundtrips_pure_blue() {
        let rgb = Rgb::new(0x00, 0x00, 0xFF);
        assert_eq!(Hsl::from_rgb(rgb).to_rgb(), rgb);
    }

    #[test]
    fn roundtrips_white_and_black() {
        for c in [Rgb::new(0xFF, 0xFF, 0xFF), Rgb::new(0x00, 0x00, 0x00)] {
            assert_eq!(Hsl::from_rgb(c).to_rgb(), c);
        }
    }

    #[test]
    fn gray_has_zero_saturation() {
        let hsl = Hsl::from_rgb(Rgb::new(0x80, 0x80, 0x80));
        assert_eq!(hsl.s, 0.0);
        assert_eq!(hsl.h, 0.0);
    }
}
