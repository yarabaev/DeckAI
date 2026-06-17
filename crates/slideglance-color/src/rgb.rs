//! sRGB color triple and resolved color (RGB + alpha).

use core::fmt;
use core::num::ParseIntError;

/// 24-bit sRGB color (8 bits per channel).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rgb {
    /// Red channel (0–255).
    pub r: u8,
    /// Green channel (0–255).
    pub g: u8,
    /// Blue channel (0–255).
    pub b: u8,
}

impl Rgb {
    /// Constructs an `Rgb` from raw 8-bit channel values.
    #[inline]
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses a hex color string. Accepts `#RRGGBB`, `RRGGBB`, in any case.
    ///
    /// # Errors
    ///
    /// Returns [`ColorParseError`] if the input is not 6 hex digits (with an
    /// optional `#` prefix).
    pub fn from_hex(hex: &str) -> Result<Self, ColorParseError> {
        let body = hex.strip_prefix('#').unwrap_or(hex);
        if body.len() != 6 {
            return Err(ColorParseError::WrongLength { got: body.len() });
        }
        if !body.is_ascii() {
            return Err(ColorParseError::WrongLength { got: body.len() });
        }
        let r = u8::from_str_radix(&body[0..2], 16).map_err(ColorParseError::InvalidDigit)?;
        let g = u8::from_str_radix(&body[2..4], 16).map_err(ColorParseError::InvalidDigit)?;
        let b = u8::from_str_radix(&body[4..6], 16).map_err(ColorParseError::InvalidDigit)?;
        Ok(Self { r, g, b })
    }

    /// Formats the color as a lowercase hex string `#rrggbb`.
    #[must_use]
    pub fn to_hex_lower(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Formats the color as an uppercase hex string `#RRGGBB`.
    #[must_use]
    pub fn to_hex_upper(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// Failure mode when parsing a hex color string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorParseError {
    /// Body length (after stripping `#`) was not 6 ASCII characters.
    WrongLength {
        /// The length actually observed.
        got: usize,
    },
    /// One of the 3 channel pairs was not valid base-16.
    InvalidDigit(ParseIntError),
}

impl fmt::Display for ColorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongLength { got } => {
                write!(f, "hex color must be 6 digits (got {got})")
            }
            Self::InvalidDigit(e) => write!(f, "invalid hex digit: {e}"),
        }
    }
}

impl std::error::Error for ColorParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidDigit(e) => Some(e),
            Self::WrongLength { .. } => None,
        }
    }
}

/// Final resolved color: an RGB triple plus an alpha value in `[0, 1]`.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResolvedColor {
    /// The sRGB color.
    pub rgb: Rgb,
    /// Opacity, where `1.0` is fully opaque and `0.0` is fully transparent.
    pub alpha: f64,
}

impl ResolvedColor {
    /// Constructs a `ResolvedColor` from an RGB triple and an alpha value.
    #[inline]
    #[must_use]
    pub const fn new(rgb: Rgb, alpha: f64) -> Self {
        Self { rgb, alpha }
    }

    /// Constructs a fully opaque `ResolvedColor` (`alpha = 1.0`).
    #[inline]
    #[must_use]
    pub const fn opaque(rgb: Rgb) -> Self {
        Self { rgb, alpha: 1.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_uppercase_hash_prefixed() {
        assert_eq!(
            Rgb::from_hex("#FF0000").unwrap(),
            Rgb::new(0xFF, 0x00, 0x00)
        );
    }

    #[test]
    fn parses_lowercase_no_prefix() {
        assert_eq!(Rgb::from_hex("00ff00").unwrap(), Rgb::new(0x00, 0xFF, 0x00));
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(matches!(
            Rgb::from_hex("#FFF"),
            Err(ColorParseError::WrongLength { got: 3 })
        ));
    }

    #[test]
    fn rejects_invalid_digit() {
        assert!(matches!(
            Rgb::from_hex("#GG0000"),
            Err(ColorParseError::InvalidDigit(_))
        ));
    }

    #[test]
    fn formats_hex_lower_and_upper() {
        let c = Rgb::new(0xAB, 0xCD, 0xEF);
        assert_eq!(c.to_hex_lower(), "#abcdef");
        assert_eq!(c.to_hex_upper(), "#ABCDEF");
    }
}
