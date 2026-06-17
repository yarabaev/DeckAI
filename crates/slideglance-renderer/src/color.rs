//! Color formatting helpers shared across element renderers.
//!
//! The spec reads `color.hex` directly off the parsed model where
//! every color is already a `#RRGGBB` string. Our `ResolvedColor` carries an
//! `Rgb` triple plus alpha, so we centralize the hex conversion here. We
//! emit uppercase hex (`#FF0000`) to match the spec's serialized
//! form — string parity matters for tests that compare output byte-for-byte.

use slideglance_color::ResolvedColor;

/// Format a [`ResolvedColor`]'s RGB component as `#RRGGBB` (uppercase).
#[must_use]
pub fn color_hex(c: &ResolvedColor) -> String {
    c.rgb.to_hex_upper()
}

/// Format an alpha value for inclusion as an SVG `*-opacity` attribute value.
/// We trim integer-valued alphas (`1` rather than `1.0`) and otherwise use
/// `f64`'s default formatter — same shape as JavaScript's `Number.toString`.
#[must_use]
pub fn alpha_str(alpha: f64) -> String {
    if alpha.fract() == 0.0 && alpha.is_finite() {
        let i = alpha as i64;
        i.to_string()
    } else {
        format!("{alpha}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_color::Rgb;

    #[test]
    fn hex_uppercase() {
        let c = ResolvedColor::new(Rgb::new(0xFF, 0, 0), 1.0);
        assert_eq!(color_hex(&c), "#FF0000");
    }

    #[test]
    fn alpha_one_renders_as_one() {
        assert_eq!(alpha_str(1.0), "1");
    }

    #[test]
    fn alpha_fractional_renders_with_dot() {
        assert_eq!(alpha_str(0.5), "0.5");
        assert_eq!(alpha_str(0.25), "0.25");
    }
}
