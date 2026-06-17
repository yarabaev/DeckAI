//! Number formatting helpers.
//!
//! The spec relies on JavaScript's default `Number.prototype.toString`
//! when interpolating values into SVG path data: integer-valued floats emit
//! without a decimal point, fractional values emit with the shortest round-
//! trip representation. Rust's `{}` for `f64` is similar but always emits a
//! decimal for `5.0` (`"5"` vs `"5"` — actually the Rust default produces
//! `"5"` for an integral f64 only in some versions; we normalize explicitly
//! to keep behavior stable across compiler versions).

#[inline]
#[must_use]
pub(crate) fn n(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < 1.0e16 {
        // Integer-valued: print without decimal, collapse `-0` to `0`.
        let i = value as i64;
        if i == 0 {
            "0".to_string()
        } else {
            i.to_string()
        }
    } else {
        format!("{value}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_floats_have_no_decimal() {
        assert_eq!(n(0.0), "0");
        assert_eq!(n(-0.0), "0");
        assert_eq!(n(96.0), "96");
        assert_eq!(n(-1.0), "-1");
    }

    #[test]
    fn fractional_floats_keep_precision() {
        assert_eq!(n(0.5), "0.5");
        assert_eq!(n(-1.25), "-1.25");
    }

    #[test]
    fn nan_and_inf_use_default_formatter() {
        // We don't expect these in OOXML data, but make sure the helper
        // does not panic on them.
        assert_eq!(n(f64::NAN), "NaN");
        assert!(n(f64::INFINITY).contains("inf"));
    }
}
