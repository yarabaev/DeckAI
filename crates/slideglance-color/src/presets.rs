//! `<a:prstClr>` named-color preset table.
//!
//! Mirrors
//! exactly — only the eight base values the spec recognizes
//! (`black` / `white` / `red` / `green` / `blue` / `yellow` / `cyan` /
//! `magenta`). Unknown names return `None`, matching the spec behavior of
//! returning `null` and having callers skip the color.
//!
//! The full ECMA-376 §20.1.10.49 enumeration carries ~140 entries; we
//! deliberately keep the smaller set per project rule "spec parity
//! — no silent divergence". Add new entries only when the spec
//! adds them.

use crate::rgb::Rgb;

/// Resolves an OOXML preset color name to an [`Rgb`] value.
///
/// Returns `None` for any name not present in the spec's
/// `PRESET_COLORS` table.
#[must_use]
pub fn resolve_preset(name: &str) -> Option<Rgb> {
    match name {
        "black" => Some(Rgb::new(0x00, 0x00, 0x00)),
        "white" => Some(Rgb::new(0xFF, 0xFF, 0xFF)),
        "red" => Some(Rgb::new(0xFF, 0x00, 0x00)),
        "green" => Some(Rgb::new(0x00, 0x80, 0x00)),
        "blue" => Some(Rgb::new(0x00, 0x00, 0xFF)),
        "yellow" => Some(Rgb::new(0xFF, 0xFF, 0x00)),
        "cyan" => Some(Rgb::new(0x00, 0xFF, 0xFF)),
        "magenta" => Some(Rgb::new(0xFF, 0x00, 0xFF)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_names_resolve() {
        assert_eq!(resolve_preset("black"), Some(Rgb::new(0, 0, 0)));
        assert_eq!(resolve_preset("white"), Some(Rgb::new(0xFF, 0xFF, 0xFF)));
        assert_eq!(resolve_preset("red"), Some(Rgb::new(0xFF, 0, 0)));
        assert_eq!(resolve_preset("green"), Some(Rgb::new(0, 0x80, 0)));
        assert_eq!(resolve_preset("blue"), Some(Rgb::new(0, 0, 0xFF)));
        assert_eq!(resolve_preset("yellow"), Some(Rgb::new(0xFF, 0xFF, 0)));
        assert_eq!(resolve_preset("cyan"), Some(Rgb::new(0, 0xFF, 0xFF)));
        assert_eq!(resolve_preset("magenta"), Some(Rgb::new(0xFF, 0, 0xFF)));
    }

    #[test]
    fn unknown_names_return_none() {
        // The spec's table has no entry for these — they intentionally
        // resolve to None so callers (e.g. duotone parser) skip the color.
        assert_eq!(resolve_preset("aliceBlue"), None);
        assert_eq!(resolve_preset("BLACK"), None); // case-sensitive
        assert_eq!(resolve_preset(""), None);
    }
}
