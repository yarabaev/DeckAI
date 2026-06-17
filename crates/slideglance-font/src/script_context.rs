//! Theme script-font context for CJK fallback during text rendering.
//!
//! Holds the script-keyed entries from
//! [`slideglance_model::FontScheme::major_script_fonts`] /
//! [`slideglance_model::FontScheme::minor_script_fonts`] in a form the renderer
//! can consult per shaped run. The renderer asks "for this CJK
//! codepoint, which theme font should I prefer?" and gets back the
//! name(s) declared in the slide master's theme.
//!
//! ## Reference parity vs CJK Script Equality
//!
//! keeps **only** the
//! Japanese (`Jpan`) major / minor fonts in module-level mutable state
//! and exposes `getJpanFallbackFont()`. Korean / Chinese theme fallbacks
//! are silently dropped.
//!
//! Per the project rule "CJK Script Equality" (`CLAUDE.md`), this Rust
//! port stores **every** script the theme XML declares — Jpan, Hang,
//! Hans, Hant, plus any other ISO 15924 code (Arab, Thai, …) — and
//! exposes symmetric `jpan_fallback` / `hang_fallback` / `hans_fallback`
//! / `hant_fallback` helpers along with a generic
//! [`ScriptFontContext::fallback_font`] for arbitrary script codes.
//!
//! No global mutable state: callers thread a `ScriptFontContext` through
//! render calls. The TS module-level pattern conflicts with Rust's
//! threading guarantees and the renderer's planned per-conversion
//! options anyway.

use std::collections::BTreeMap;

/// Canonical ISO 15924 codes for the four CJK scripts the project's CJK
/// Script Equality rule requires equal treatment for.
///
/// Iteration order: Japanese, Korean, Simplified Chinese, Traditional
/// Chinese. Used by [`ScriptFontContext::cjk_fallbacks`] for deterministic
/// snapshot output.
pub const CJK_SCRIPT_CODES: [&str; 4] = ["Hang", "Hans", "Hant", "Jpan"];

/// Per-render theme script-font context.
///
/// Constructed from a parsed `<a:fontScheme>`. Holds the major / minor
/// script-keyed font names; [`ScriptFontContext::fallback_font`] prefers
/// the major font and falls back to the minor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptFontContext {
    major: BTreeMap<String, String>,
    minor: BTreeMap<String, String>,
}

impl ScriptFontContext {
    /// Builds a context from explicit major / minor script-font tables.
    ///
    /// Keys are ISO 15924 script codes (`"Jpan"`, `"Hang"`, `"Hans"`,
    /// `"Hant"`, `"Arab"`, …). Values are the typeface strings from the
    /// theme XML.
    #[must_use]
    pub fn new(major: BTreeMap<String, String>, minor: BTreeMap<String, String>) -> Self {
        Self { major, minor }
    }

    /// An empty context — all lookups return `None`.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Returns the major-font typeface for ISO 15924 script `code`.
    #[must_use]
    pub fn major_font(&self, code: &str) -> Option<&str> {
        self.major.get(code).map(String::as_str)
    }

    /// Returns the minor-font typeface for ISO 15924 script `code`.
    #[must_use]
    pub fn minor_font(&self, code: &str) -> Option<&str> {
        self.minor.get(code).map(String::as_str)
    }

    /// Returns the renderer's fallback typeface for ISO 15924 script
    /// `code`: the major font if present, otherwise the minor font.
    ///
    /// Mirrors the TS `getJpanFallbackFont` resolution order
    /// (`jpanMajorFont ?? jpanMinorFont`) generalized to every script.
    #[must_use]
    pub fn fallback_font(&self, code: &str) -> Option<&str> {
        self.major_font(code).or_else(|| self.minor_font(code))
    }

    /// Japanese (Jpan) fallback. TS-equivalent to `getJpanFallbackFont`.
    #[must_use]
    pub fn jpan_fallback(&self) -> Option<&str> {
        self.fallback_font("Jpan")
    }

    /// Korean (Hang) fallback. Added per CJK Script Equality (TS omits
    /// this).
    #[must_use]
    pub fn hang_fallback(&self) -> Option<&str> {
        self.fallback_font("Hang")
    }

    /// Simplified Chinese (Hans) fallback. Added per CJK Script Equality
    /// (TS omits this).
    #[must_use]
    pub fn hans_fallback(&self) -> Option<&str> {
        self.fallback_font("Hans")
    }

    /// Traditional Chinese (Hant) fallback. Added per CJK Script Equality
    /// (TS omits this).
    #[must_use]
    pub fn hant_fallback(&self) -> Option<&str> {
        self.fallback_font("Hant")
    }

    /// Iterator over `(script_code, fallback_font)` for every CJK script
    /// in [`CJK_SCRIPT_CODES`] order whose entry is present.
    ///
    /// Skips scripts with no major / minor font.
    pub fn cjk_fallbacks(&self) -> impl Iterator<Item = (&'static str, &str)> + '_ {
        CJK_SCRIPT_CODES
            .iter()
            .filter_map(|code| self.fallback_font(code).map(|f| (*code, f)))
    }

    /// Returns `true` if the context has no major or minor entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.major.is_empty() && self.minor.is_empty()
    }

    /// Major-font script entries. Use [`ScriptFontContext::major_font`]
    /// for single-key lookup; this borrow exists for callers that need
    /// to iterate or merge.
    #[must_use]
    pub fn major_table(&self) -> &BTreeMap<String, String> {
        &self.major
    }

    /// Minor-font script entries.
    #[must_use]
    pub fn minor_table(&self) -> &BTreeMap<String, String> {
        &self.minor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ScriptFontContext {
        let mut major = BTreeMap::new();
        major.insert("Jpan".to_string(), "Yu Gothic".to_string());
        major.insert("Hang".to_string(), "맑은 고딕".to_string());
        major.insert("Hans".to_string(), "Microsoft YaHei".to_string());
        major.insert("Hant".to_string(), "Microsoft JhengHei".to_string());
        let mut minor = BTreeMap::new();
        minor.insert("Jpan".to_string(), "Meiryo".to_string());
        minor.insert("Hans".to_string(), "SimSun".to_string());
        ScriptFontContext::new(major, minor)
    }

    // -- Generic lookup -------------------------------------------------------

    #[test]
    fn empty_context_returns_none_everywhere() {
        let ctx = ScriptFontContext::empty();
        assert!(ctx.is_empty());
        assert_eq!(ctx.fallback_font("Jpan"), None);
        assert_eq!(ctx.fallback_font("Hang"), None);
        assert_eq!(ctx.fallback_font("Hans"), None);
        assert_eq!(ctx.fallback_font("Hant"), None);
    }

    #[test]
    fn major_font_lookup() {
        let ctx = fixture();
        assert_eq!(ctx.major_font("Jpan"), Some("Yu Gothic"));
        assert_eq!(ctx.major_font("Hang"), Some("맑은 고딕"));
        assert_eq!(ctx.major_font("Unknown"), None);
    }

    #[test]
    fn minor_font_lookup() {
        let ctx = fixture();
        assert_eq!(ctx.minor_font("Jpan"), Some("Meiryo"));
        assert_eq!(ctx.minor_font("Hans"), Some("SimSun"));
        // Hang has no minor entry in fixture.
        assert_eq!(ctx.minor_font("Hang"), None);
    }

    #[test]
    fn fallback_prefers_major_then_minor() {
        let ctx = fixture();
        // Has both major and minor — major wins.
        assert_eq!(ctx.fallback_font("Jpan"), Some("Yu Gothic"));
        // Hang has only major — major used.
        assert_eq!(ctx.fallback_font("Hang"), Some("맑은 고딕"));
        // Major-only minor missing case (Hant has only major).
        assert_eq!(ctx.fallback_font("Hant"), Some("Microsoft JhengHei"));
    }

    #[test]
    fn fallback_uses_minor_when_major_missing() {
        let major = BTreeMap::new();
        let mut minor = BTreeMap::new();
        minor.insert("Jpan".to_string(), "Meiryo".to_string());
        let ctx = ScriptFontContext::new(major, minor);
        assert_eq!(ctx.fallback_font("Jpan"), Some("Meiryo"));
    }

    // -- CJK Script Equality helpers ------------------------------------------

    #[test]
    fn cjk_fallback_helpers_match_generic_lookup() {
        let ctx = fixture();
        assert_eq!(ctx.jpan_fallback(), ctx.fallback_font("Jpan"));
        assert_eq!(ctx.hang_fallback(), ctx.fallback_font("Hang"));
        assert_eq!(ctx.hans_fallback(), ctx.fallback_font("Hans"));
        assert_eq!(ctx.hant_fallback(), ctx.fallback_font("Hant"));
    }

    #[test]
    fn cjk_fallbacks_iterates_in_canonical_order() {
        let ctx = fixture();
        let collected: Vec<(&str, &str)> = ctx.cjk_fallbacks().collect();
        // ISO 15924 lexicographic: Hang < Hans < Hant < Jpan
        assert_eq!(
            collected,
            vec![
                ("Hang", "맑은 고딕"),
                ("Hans", "Microsoft YaHei"),
                ("Hant", "Microsoft JhengHei"),
                ("Jpan", "Yu Gothic"),
            ]
        );
    }

    #[test]
    fn cjk_fallbacks_skips_missing_scripts() {
        let mut major = BTreeMap::new();
        major.insert("Jpan".to_string(), "Yu Gothic".to_string());
        major.insert("Hans".to_string(), "Microsoft YaHei".to_string());
        let ctx = ScriptFontContext::new(major, BTreeMap::new());
        let collected: Vec<(&str, &str)> = ctx.cjk_fallbacks().collect();
        // Lexicographic: Hans before Jpan.
        assert_eq!(
            collected,
            vec![("Hans", "Microsoft YaHei"), ("Jpan", "Yu Gothic")]
        );
    }

    // -- CJK Script Equality verification --------------------------------------

    #[test]
    fn cjk_equality_all_four_scripts_have_dedicated_helpers() {
        // Surface-level verification: each CJK script has a named helper
        // that is implementation-equivalent to fallback_font(<code>). This
        // is the structural assertion behind the project rule.
        let ctx = fixture();
        let helpers: [Option<&str>; 4] = [
            ctx.jpan_fallback(),
            ctx.hang_fallback(),
            ctx.hans_fallback(),
            ctx.hant_fallback(),
        ];
        let generic: [Option<&str>; 4] = [
            ctx.fallback_font("Jpan"),
            ctx.fallback_font("Hang"),
            ctx.fallback_font("Hans"),
            ctx.fallback_font("Hant"),
        ];
        assert_eq!(helpers, generic);
    }

    #[test]
    fn cjk_script_codes_constant_lists_all_four_scripts_in_order() {
        // ISO 15924 lexicographic order (KDD-14).
        assert_eq!(CJK_SCRIPT_CODES, ["Hang", "Hans", "Hant", "Jpan"]);
    }

    // -- Non-CJK scripts work too (Arab / Thai / etc.) ------------------------

    #[test]
    fn arbitrary_script_codes_are_supported() {
        let mut major = BTreeMap::new();
        major.insert("Arab".to_string(), "Arial".to_string());
        major.insert("Thai".to_string(), "Cordia New".to_string());
        let ctx = ScriptFontContext::new(major, BTreeMap::new());
        assert_eq!(ctx.fallback_font("Arab"), Some("Arial"));
        assert_eq!(ctx.fallback_font("Thai"), Some("Cordia New"));
    }

    // -- Tables exposed for merging / iteration -------------------------------

    #[test]
    fn major_and_minor_tables_are_exposed() {
        let ctx = fixture();
        assert_eq!(ctx.major_table().len(), 4);
        assert_eq!(ctx.minor_table().len(), 2);
    }
}
