//! CJK code-point detection and per-script segmentation.
//!
//! This module re-exports the canonical implementations from `slideglance-font`
//! and retains backward-compat helpers (`is_cjk_codepoint`, `ScriptSegment`)
//! used by the renderer until the migration to `TextPart` is complete.
//!
//! T7 (CJK Equality) routes each segment to the correct per-script fallback;
//! T11 (IP-13) classifies PUA segments. Both build on `TextPart` / `Script`.

pub use slideglance_font::is_cjk_codepoint;
pub use slideglance_font::{split_by_script, TextPart};
// classify_script and Script re-exports kept for downstream/test use
#[allow(unused_imports)]
pub use slideglance_font::{classify_script, Script};

/// Backward-compat shim: a two-way Latin / EA segment.
///
/// New code should use [`TextPart`] + [`Script`] instead.
/// This type bridges renderer callers that haven't been ported yet.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptSegment {
    /// The text fragment.
    pub text: String,
    /// `true` when every code point in [`Self::text`] is East-Asian.
    pub is_ea: bool,
}

impl From<TextPart> for ScriptSegment {
    fn from(part: TextPart) -> Self {
        Self {
            is_ea: part.script.is_ea(),
            text: part.text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_is_not_cjk() {
        for cp in [0x20, u32::from(b'a'), u32::from(b'Z'), 0x7E] {
            assert!(!is_cjk_codepoint(cp));
        }
    }

    #[test]
    fn cjk_unified_is_cjk() {
        assert!(is_cjk_codepoint('日' as u32));
        assert!(is_cjk_codepoint('本' as u32));
        assert!(is_cjk_codepoint('語' as u32));
    }

    #[test]
    fn hangul_is_cjk() {
        assert!(is_cjk_codepoint('한' as u32));
        assert!(is_cjk_codepoint('국' as u32));
        assert!(is_cjk_codepoint('어' as u32));
    }

    #[test]
    fn fullwidth_punctuation_is_cjk() {
        assert!(is_cjk_codepoint('，' as u32));
        assert!(is_cjk_codepoint('。' as u32));
    }

    #[test]
    fn empty_input_yields_empty() {
        assert!(split_by_script("").is_empty());
    }

    #[test]
    fn pure_latin_yields_one_segment() {
        let parts = split_by_script("hello world");
        assert_eq!(parts.len(), 1);
        assert!(!parts[0].script.is_ea());
        assert_eq!(parts[0].text, "hello world");
    }

    #[test]
    fn pure_cjk_yields_one_segment() {
        let parts = split_by_script("안녕하세요");
        assert_eq!(parts.len(), 1);
        assert!(parts[0].script.is_ea());
    }

    #[test]
    fn mixed_text_alternates() {
        let parts = split_by_script("Hello 한국어 World");
        // "Hello " (latin) | "한국어" (ea) | " World" (latin)
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].text, "Hello ");
        assert!(!parts[0].script.is_ea());
        assert_eq!(parts[1].text, "한국어");
        assert!(parts[1].script.is_ea());
        assert_eq!(parts[2].text, " World");
        assert!(!parts[2].script.is_ea());
    }

    #[test]
    fn script_segment_from_text_part_latin() {
        let part = TextPart {
            text: "hello".to_string(),
            script: Script::Latin,
        };
        let seg = ScriptSegment::from(part);
        assert!(!seg.is_ea);
        assert_eq!(seg.text, "hello");
    }

    #[test]
    fn script_segment_from_text_part_korean() {
        let part = TextPart {
            text: "한국".to_string(),
            script: Script::Korean,
        };
        let seg = ScriptSegment::from(part);
        assert!(seg.is_ea);
    }
}
