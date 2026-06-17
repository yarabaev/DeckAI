//! XML escape helpers used across the renderer.
//!
//! The TypeScript reference uses two escape variants:
//!
//! 1. **Attribute escape** — for values placed inside `attr="..."`: escape
//!    `&`, `"`, `<`, `>` (the last one only out of caution; not strictly
//! required by the XML spec inside a double-quoted value, but the TS
//!    reference does it on `slide.layoutName` and image alt text).
//! 2. **Text-node escape** — for character data between tags: escape `&`,
//!    `<`, `>`. Quote characters are valid as-is.
//!
//! These helpers exist as small `String` returners (rather than streaming into
//! a writer) because the renderer assembles SVG via `String` concatenation
//! exactly the way the TypeScript port does — every escape happens inside a
//! `parts.push(...)` call site. Streaming would be a different design and is
//! not what we are porting here.

/// Escape a string for use inside an SVG attribute value (quoted with `"`).
///
/// Mirrors the inline escape in
/// (`escapeXmlAttr` /
/// `addAriaLabel`).
#[must_use]
pub fn escape_xml_attr(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            other => out.push(other),
        }
    }
    out
}

/// Escape a string for use as an SVG text node (between tags).
#[must_use]
pub fn escape_xml_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            other => out.push(other),
        }
    }
    out
}

/// Emit ` data-sp-id="{n}"` when `sp_id` is `Some`. Returns `""` otherwise.
///
/// The leading space lets callers concatenate the result directly after `<g`
/// (or any other tag name) without conditional spacing logic in their format
/// strings — mirroring the established `build_object_name_attr` pattern.
///
/// `sp_id` carries the slide-scoped `<p:cNvPr @id>` value parsed from the
/// source PPTX so downstream consumers (selection overlays, accessibility
/// trees, automation scripts) can address each shape stably.
#[must_use]
pub fn build_sp_id_attr(sp_id: Option<u32>) -> String {
    match sp_id {
        Some(n) => format!(" data-sp-id=\"{n}\""),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attr_escapes_amp_quote_lt_gt() {
        assert_eq!(
            escape_xml_attr("a & b \"c\" <d> >e<"),
            "a &amp; b &quot;c&quot; &lt;d&gt; &gt;e&lt;"
        );
    }

    #[test]
    fn attr_passes_through_safe_chars() {
        assert_eq!(escape_xml_attr("Slide 1"), "Slide 1");
        assert_eq!(escape_xml_attr(""), "");
        // Apostrophe is not escaped — the renderer only emits double-quoted
        // attribute values.
        assert_eq!(escape_xml_attr("it's"), "it's");
    }

    #[test]
    fn text_escapes_amp_lt_gt_only() {
        assert_eq!(escape_xml_text("a & b <c> d"), "a &amp; b &lt;c&gt; d");
    }

    #[test]
    fn text_keeps_quotes_unescaped() {
        // Quotes are valid in CDATA; the spec does not escape them
        // inside `<text>` / `<tspan>` content.
        assert_eq!(escape_xml_text("\"hello\""), "\"hello\"");
    }

    #[test]
    fn handles_unicode() {
        // Non-ASCII characters round-trip unchanged.
        assert_eq!(escape_xml_text("한국어 & 日本語"), "한국어 &amp; 日本語");
        assert_eq!(escape_xml_attr("한국어 \"日\""), "한국어 &quot;日&quot;");
    }

    #[test]
    fn sp_id_attr_some_and_none() {
        assert_eq!(build_sp_id_attr(Some(42)), " data-sp-id=\"42\"");
        assert_eq!(build_sp_id_attr(None), "");
    }

    #[test]
    fn sp_id_attr_handles_zero_and_max() {
        // PowerPoint sometimes emits id="0" for placeholders; u32::MAX is the
        // upper bound the parser tolerates.
        assert_eq!(build_sp_id_attr(Some(0)), " data-sp-id=\"0\"");
        assert_eq!(
            build_sp_id_attr(Some(u32::MAX)),
            format!(" data-sp-id=\"{}\"", u32::MAX)
        );
    }
}
