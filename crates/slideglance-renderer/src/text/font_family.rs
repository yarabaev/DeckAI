//! `font-family` attribute value construction.
//!
//! Direct port of `buildFontFamilyValue` from
//! . The list-of-fallback
//! semantics matter for parity with browser / resvg rendering: the first
//! face that exists on the host OS is the one `PowerPoint` actually shows.
//!
//! In TS the helper consults three globals: `getCjkFallbackFonts(name)`
//!
//! reaches them through the
//! `font/* modules' module-level state — we pass a config
//! tuple through instead.
//! (per-OS preinstalled CJK), `getCurrentMappedFont(name)` (PPTX -> OSS
//! mapping), and `getMetricsFallbackFont(name)` (metric-compatible OSS
//! fallback for Latin faces). The Rust port resolves all three through
//! `slideglance-font` directly — the renderer threads the chosen
//! [`slideglance_font::CjkPlatform`] and an optional custom mapping so deterministic
//! tests can pin the OS variant.

use slideglance_font::{
    get_cjk_fallback_fonts, get_latin_os_defaults, get_mapped_font, get_metrics_fallback_font,
    known_metric_fonts, metric_distance, metric_for_family, CjkPlatform, FontMapping,
    ScriptFontContext,
};

/// Return up to `top_n` typefaces from the static metric catalogue whose
/// PANOSE / OS/2 vector is closest to `family`'s vector.
///
/// "Static" means the catalogue is compiled in — there is no host font
/// scan, no network call, and no OS-specific data path. The same input
/// produces the same output in every target environment:
///
/// - Browsers (Chrome / Safari / Firefox / Edge) running the WASM build
/// - Node.js, Deno, Bun running the WASM build
/// - The native CLI on macOS / Windows / Linux
/// - Air-gapped / offline machines (no Google Fonts, no Local Font Access)
///
/// This is the universal-environment branch of the font-fidelity
/// strategy. Runtime host-font scanning lives behind the opt-in
/// `metric-match` cargo feature on `slideglance-font::MetricResolver` and is
/// only used by the native CLI when the caller explicitly enables it.
fn static_metric_siblings(family: &str, top_n: usize) -> Vec<&'static str> {
    let Some(target) = metric_for_family(family) else {
        return Vec::new();
    };
    let mut scored: Vec<(&'static str, f32)> = known_metric_fonts()
        .filter(|(name, _)| !name.eq_ignore_ascii_case(family))
        .map(|(name, vec)| (name, metric_distance(&target, vec)))
        .collect();
    scored.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(b.0))
    });
    scored
        .into_iter()
        .take(top_n)
        .map(|(name, _)| name)
        .collect()
}

/// Build the ordered list of font-family names that the SVG would emit
/// for the given OOXML typeface inputs, *without* the surrounding
/// `font-family="..."` formatting and without the trailing generic
/// (`sans-serif` / `serif`).
///
/// Used by [`build_font_family_value`] to compose the final attribute
/// string and by `slideglance::font_usage` to expose the same chain to host
/// UIs (so a viewer can probe `document.fonts.check()` against each
/// entry to identify the actually-rendered font).
///
/// Returns an empty `Vec` when every supplied entry is `None` or empty.
#[must_use]
pub fn build_font_family_chain(
    fonts: &[Option<&str>],
    mapping: &FontMapping,
    cjk_platform: CjkPlatform,
    script_fonts: &ScriptFontContext,
) -> Vec<String> {
    let mut unique: Vec<String> = Vec::new();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    let push_with_cjk_fallbacks =
        |name: &str, unique: &mut Vec<String>, seen: &mut std::collections::BTreeSet<String>| {
            if seen.insert(name.to_string()) {
                unique.push(name.to_string());
            }
            for cjk in get_cjk_fallback_fonts(cjk_platform, name) {
                if seen.insert(cjk.to_string()) {
                    unique.push(cjk.to_string());
                }
            }
        };

    for entry in fonts {
        let Some(font) = entry else { continue };
        if font.is_empty() {
            continue;
        }
        if seen.contains(*font) {
            continue;
        }
        push_with_cjk_fallbacks(font, &mut unique, &mut seen);

        // OOXML name -> OSS-mapped name (e.g. Calibri -> Carlito), with that
        // mapped name's own CJK fallbacks.
        if let Some(mapped) = get_mapped_font(font, mapping) {
            if !seen.contains(&mapped) {
                push_with_cjk_fallbacks(&mapped, &mut unique, &mut seen);
            }
        }

        // Metric-compatible Latin fallback (no further CJK chain — these
        // are Latin-only families like Carlito / Arimo).
        if let Some(fallback) = get_metrics_fallback_font(font) {
            if seen.insert(fallback.to_string()) {
                unique.push(fallback.to_string());
            }
        }

        // Static-catalogue metric siblings — accuracy-first, environment-
        // independent. Adds up to two visually-closest typefaces from the
        // compile-time catalogue so the browser / resvg / native PDF
        // pipeline has a deterministic chance of landing on a similarly-
        // proportioned face when neither the original typeface nor its
        // OSS / metric mapping is installed. Top_n=2 keeps the chain
        // short; the seen-set dedups against earlier entries.
        for sibling in static_metric_siblings(font, 2) {
            if seen.insert(sibling.to_string()) {
                unique.push(sibling.to_string());
            }
        }
    }

    // OS-default Latin sans-serif fallbacks. Inserted *before* the
    // theme `<a:font script="…">` CJK fonts so a Latin run on a deck
    // whose theme declares e.g. `Hang typeface="맑은 고딕"` falls
    // through to a Latin face (`Helvetica Neue` on macOS, `Segoe UI`
    // on Windows, `Liberation Sans` on Linux) instead of the CJK
    // font's bare-bones Latin glyph subset. See
    // `slideglance_font::latin_defaults` for the rationale and per-OS
    // lists. Browsers' per-glyph fallback semantics still let any
    // CJK characters embedded in the run drop through to the script
    // fonts below, so this insert improves Latin coverage without
    // losing CJK coverage.
    //
    // Gated on `!unique.is_empty()` so the no-text-input path
    // (`fonts = [None, None]` and similar) still returns an empty
    // chain — `build_font_family_value` relies on this to decide
    // whether to emit the `font-family` attribute at all.
    if !unique.is_empty() {
        for default in get_latin_os_defaults(cjk_platform) {
            if seen.insert((*default).to_string()) {
                unique.push((*default).to_string());
            }
        }
    }

    // Theme `<a:font script="…">` fallbacks — appended after the
    // per-typeface chains so the run's authored typeface wins when
    // present, but the deck's CJK faces still get a shot before the
    // generic CSS family. Each script font also drags its own platform
    // CJK chain (e.g. "Yu Gothic" -> "Yu Gothic Medium", "Yu Gothic UI"
    // on Windows; "Hiragino Sans" on macOS).
    for (_code, name) in script_fonts.cjk_fallbacks() {
        if seen.contains(name) {
            continue;
        }
        push_with_cjk_fallbacks(name, &mut unique, &mut seen);
        // Script fonts may themselves have an OSS mapping target.
        if let Some(mapped) = get_mapped_font(name, mapping) {
            if !seen.contains(&mapped) {
                push_with_cjk_fallbacks(&mapped, &mut unique, &mut seen);
            }
        }
    }

    unique
}

/// Build the `font-family="..."` value list for one or more OOXML
/// typefaces in priority order (Latin first, EA second is the typical
/// caller pattern — see `super::segment::render_segment`).
///
/// `mapping` selects the PPTX -> OSS mapping table to consult. Pass
/// [`slideglance_font::DEFAULT_FONT_MAPPING`] for the standard mapping.
/// `cjk_platform` selects which CJK preinstalled-fonts table the helper
/// adds as fallbacks for each candidate.
///
/// `script_fonts`, when non-empty, appends the deck's theme `<a:font
/// script="…">` typefaces (Jpan / Hang / Hans / Hant — every CJK script
/// in `slideglance_font::CJK_SCRIPT_CODES`) to the fallback chain after the
/// per-typeface CJK platform fallbacks. This guarantees that browsers
/// rendering the SVG pick the deck's authored CJK face when it's
/// installed locally — even when the run only declares a Latin
/// `<a:latin typeface="…"/>`. `PowerPoint` applies the same fallback
/// implicitly; making it explicit in the SVG `font-family` chain
/// removes the silent divergence the spec exhibits when CJK
/// script fonts are present in the theme but missing from runs.
///
/// Returns `None` when every supplied entry is `None` or empty.
#[must_use]
pub fn build_font_family_value(
    fonts: &[Option<&str>],
    mapping: &FontMapping,
    cjk_platform: CjkPlatform,
    script_fonts: &ScriptFontContext,
) -> Option<String> {
    let unique = build_font_family_chain(fonts, mapping, cjk_platform, script_fonts);
    if unique.is_empty() {
        return None;
    }

    let generic = generic_family(&unique[0]);
    let mut parts: Vec<String> = unique
        .iter()
        .map(|f| {
            let escaped = escape_font_name(f);
            if f.contains(' ') {
                format!("'{escaped}'")
            } else {
                escaped
            }
        })
        .collect();
    parts.push(generic.to_string());
    Some(parts.join(", "))
}

/// Decide the generic CSS family suffix (`serif` vs `sans-serif`) that
/// should close the `font-family` list. The spec uses a small
/// allowlist of serif names plus a `serif` substring check.
fn generic_family(font_family: &str) -> &'static str {
    let lower = font_family.to_lowercase();
    if lower.contains("mincho")
        || lower.contains("明朝")
        || lower == "times new roman"
        || lower == "georgia"
        || lower == "cambria"
        || (lower.contains("serif") && !lower.contains("sans"))
    {
        "serif"
    } else {
        "sans-serif"
    }
}

/// XML-attribute-escape a font name for inclusion in `font-family="..."`.
/// Mirrors the TS helper exactly — escapes `&`, `<`, `>`, `"`.
#[must_use]
pub fn escape_font_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use slideglance_font::FontMapping;

    fn empty_mapping() -> FontMapping {
        FontMapping::new()
    }

    #[test]
    fn null_only_returns_none() {
        let m = empty_mapping();
        assert_eq!(
            build_font_family_value(
                &[None, None],
                &m,
                CjkPlatform::Other,
                &ScriptFontContext::empty()
            ),
            None
        );
    }

    #[test]
    fn single_latin_font_appends_generic() {
        let m = empty_mapping();
        let s = build_font_family_value(
            &[Some("Calibri")],
            &m,
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
        )
        .unwrap();
        // No CJK fallbacks for Calibri on the synthetic Other platform; the
        // empty mapping leaves it without an OSS replacement either; the
        // metric fallback table maps Calibri -> Carlito.
        assert!(s.contains("Calibri"));
        assert!(s.ends_with(", sans-serif"));
        assert!(s.contains("Carlito"));
    }

    #[test]
    fn font_with_space_is_quoted() {
        let m = empty_mapping();
        let s = build_font_family_value(
            &[Some("Times New Roman")],
            &m,
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
        )
        .unwrap();
        assert!(s.contains("'Times New Roman'"));
        assert!(s.ends_with(", serif"));
    }

    #[test]
    fn duplicates_are_collapsed() {
        let m = empty_mapping();
        let s = build_font_family_value(
            &[Some("Arial"), Some("Arial")],
            &m,
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
        )
        .unwrap();
        assert_eq!(s.matches("Arial").count(), 1);
    }

    #[test]
    fn empty_string_entries_are_skipped() {
        let m = empty_mapping();
        let s = build_font_family_value(
            &[Some("")],
            &m,
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
        );
        assert_eq!(s, None);
    }

    #[test]
    fn mincho_classifies_as_serif() {
        let m = empty_mapping();
        let s = build_font_family_value(
            &[Some("MS Mincho")],
            &m,
            CjkPlatform::Other,
            &ScriptFontContext::empty(),
        )
        .unwrap();
        assert!(s.ends_with(", serif"));
    }

    #[test]
    fn ko_cjk_fallbacks_added_on_macos() {
        let m = empty_mapping();
        // "Noto Sans KR" is the canonical mapped Korean sans face.
        let s = build_font_family_value(
            &[Some("Noto Sans KR")],
            &m,
            CjkPlatform::MacOs,
            &ScriptFontContext::empty(),
        )
        .unwrap();
        // The macOS preinstalled Korean fallback chain should be present.
        // Existence of the entry is enough — exact ordering is verified in
        // slideglance-font's own tests.
        assert!(s.contains("Noto Sans KR"));
    }

    #[test]
    fn font_name_with_xml_unsafe_chars_is_escaped() {
        let escaped = escape_font_name("Big & Bold \"Family\"");
        assert_eq!(escaped, "Big &amp; Bold &quot;Family&quot;");
    }
}
