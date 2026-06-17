// Same rationale as `mapping.rs` / `cjk_fallback.rs` — OS / OOXML font
// names are mixed-case proper nouns, not code identifiers.
#![allow(clippy::doc_markdown)]

//! Per-OS Latin sans-serif fallback chains.
//!
//! Inserted into the SVG `font-family` chain by
//! `slideglance_renderer::build_font_family_chain` after the authored
//! typeface, its OSS metric-compatible siblings, and its CJK platform
//! fallbacks — but **before** the deck's theme `<a:font script="…">`
//! CJK fonts.
//!
//! ## Why this layer exists
//!
//! Without this insert, a Latin run on a deck whose theme declares a
//! Korean / Japanese / Chinese script font (e.g. `<a:font script="Hang"
//! typeface="맑은 고딕"/>`) walks past:
//!
//! 1. the requested Latin face (e.g. `Open Sauce Bold`) — missing
//! 2. its mapped OSS replacement (`Carlito`-style) — missing
//! 3. its metric-compatible siblings — possibly missing
//!
//! …straight into the theme's CJK script font (e.g. `맑은 고딕`).
//! Browsers happily render Latin glyphs from a CJK font, but those
//! glyphs are typically the CJK font's bare-bones Latin subset (often
//! a thin / wide / off-weight rendering that looks visibly wrong next
//! to the rest of the deck).
//!
//! On macOS the situation is worse: CoreText silently aliases unknown
//! Korean / Chinese family names (`맑은 고딕` → `Apple SD Gothic Neo`,
//! `微软雅黑` → `PingFang SC`). `document.fonts.check()` reports the
//! requested CJK family as installed even on machines that have never
//! seen Office, so the [`crate::cjk_fallback`] entries that come
//! *after* the script font in the chain never get evaluated. Latin
//! runs then render with CJK-font Latin glyphs and the status-bar
//! indicator misleadingly reports "rendered as 맑은 고딕".
//!
//! Inserting OS-default Latin sans-serif families *before* the script
//! fonts side-steps both issues:
//!
//! - On every supported OS the chain reaches a known-good Latin
//!   sans-serif face that is essentially guaranteed to be installed.
//! - The browser's per-glyph fallback semantics still let CJK glyphs
//!   inside a Latin run drop through to the script-font entries
//!   below, so CJK characters embedded in a Latin paragraph stay
//!   correctly typeset.
//!
//! ## Per-OS lists
//!
//! - **macOS** — `Helvetica Neue`, `Helvetica`, `Arial`. All three
//!   ship with every modern macOS release.
//! - **Windows** — `Segoe UI`, `Arial`, `Tahoma`. `Segoe UI` is the
//!   modern system UI face, `Arial` covers older builds, `Tahoma`
//!   covers reduced-install scenarios.
//! - **Linux** — `Liberation Sans`, `DejaVu Sans`, `Arial`. Most
//!   desktop distros ship Liberation or DejaVu; `Arial` is included
//!   for Wine / msttcorefonts users.
//! - **Other** — `Arial`, `Liberation Sans`. Conservative pair so
//!   the chain still has *something* on unidentified targets.

use crate::cjk_fallback::CjkPlatform;

/// Returns the list of OS-default Latin sans-serif typefaces for
/// `platform`, in priority order.
///
/// The returned slice has `'static` lifetime — callers iterate it
/// without taking ownership. Consumed by
/// `slideglance_renderer::build_font_family_chain`.
#[must_use]
pub fn get_latin_os_defaults(platform: CjkPlatform) -> &'static [&'static str] {
    match platform {
        CjkPlatform::MacOs => MACOS_LATIN_DEFAULTS,
        CjkPlatform::Windows => WINDOWS_LATIN_DEFAULTS,
        CjkPlatform::Linux => LINUX_LATIN_DEFAULTS,
        CjkPlatform::Other => OTHER_LATIN_DEFAULTS,
    }
}

const MACOS_LATIN_DEFAULTS: &[&str] = &["Helvetica Neue", "Helvetica", "Arial"];

const WINDOWS_LATIN_DEFAULTS: &[&str] = &["Segoe UI", "Arial", "Tahoma"];

const LINUX_LATIN_DEFAULTS: &[&str] = &["Liberation Sans", "DejaVu Sans", "Arial"];

const OTHER_LATIN_DEFAULTS: &[&str] = &["Arial", "Liberation Sans"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_chain_starts_with_helvetica_neue() {
        assert_eq!(
            get_latin_os_defaults(CjkPlatform::MacOs)[0],
            "Helvetica Neue"
        );
    }

    #[test]
    fn windows_chain_starts_with_segoe_ui() {
        assert_eq!(get_latin_os_defaults(CjkPlatform::Windows)[0], "Segoe UI");
    }

    #[test]
    fn linux_chain_starts_with_liberation_sans() {
        assert_eq!(
            get_latin_os_defaults(CjkPlatform::Linux)[0],
            "Liberation Sans"
        );
    }

    #[test]
    fn other_chain_is_non_empty() {
        // Ensures every platform variant has at least one fallback so
        // the chain never collapses to just the CSS generic.
        assert!(!get_latin_os_defaults(CjkPlatform::Other).is_empty());
    }

    #[test]
    fn no_chain_is_empty() {
        for p in [
            CjkPlatform::MacOs,
            CjkPlatform::Windows,
            CjkPlatform::Linux,
            CjkPlatform::Other,
        ] {
            assert!(!get_latin_os_defaults(p).is_empty(), "platform {p:?}");
        }
    }
}
