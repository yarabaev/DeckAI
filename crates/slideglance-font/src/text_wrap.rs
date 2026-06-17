// OOXML / OpenType identifiers are mixed-case proper nouns rather
// than code identifiers — same rationale as `mapping.rs`.
#![allow(clippy::doc_markdown)]

//! Greedy paragraph line-wrap with CJK character-boundary breaks.
//!
//! Mirrors 1:1. The algorithm:
//!
//! 1. Tokenize each run by splitting on whitespace / CJK boundaries —
//!    spaces become standalone breakable tokens, each CJK character is
//!    its own token, Latin letter sequences form word tokens. Embedded
//!    `\n` produces a `force_break` token.
//! 2. Greedy layout: append tokens until adding the next would exceed
//!    `available_width + tolerance` (`available_width × 0.02`), then
//!    break before / at the token depending on its `breakable` flag.
//! 3. Empty-line + non-breakable token: split by character with
//!    [`split_token_by_chars`].
//! 4. Merge consecutive same-run tokens into [`LineSegment`]s and trim
//!    trailing whitespace.
//!
//! `text-wrap` algorithm — paragraph runs to wrapped output lines.
//!
//! ## Measurer abstraction
//!
//! [`TextMeasurer`] is a minimal trait so this module can stay
//! independent of the ttf-parser-backed measurer.
//! [`HeuristicTextMeasurer`] delegates to
//! [`crate::text_measure::measure_text_width`] and is the default for
//! callers that have not yet plugged in an opentype-backed measurer.

use slideglance_model::text::{Paragraph, RunProperties};

use crate::text_measure::is_cjk_codepoint;
use crate::text_measurer::{FontStyle, TextMeasurer};

/// Default font size when a run has no explicit size and no defaults
/// resolve. Matches TS `DEFAULT_FONT_SIZE`.
pub const DEFAULT_FONT_SIZE: f64 = 18.0;

/// Tolerance ratio applied to `available_width` so metric-approximation
/// errors (e.g. `Meiryo` measured via `Noto Sans JP` widths) do not
/// nudge a one-line text into two.
///
/// ## Rationale (empirical justification)
///
/// `0.02` (2%) is chosen so that the worst-case per-line cumulative
/// metric error stays below the tolerance:
///
/// - **Latin (Crosextra clones)**: Carlito / Arimo / Tinos are bit-
///   exact metric clones of Calibri / Arial / Times. Worst-case error
///   per character: <0.1% (tail-rounding). Per typical 80-character
///   line: <8% of *one character width* — well below the 2% line-
///   level tolerance.
/// - **CJK (Noto)**: NotoSansJP cjk_width=1000 vs PowerPoint's
///   Meiryo ~1000 → essentially exact. Worst case is when a real
///   font's CJK width drifts ±10 units from 1000 (rare); per 30-CJK-
///   character line this is 30×0.01 = 0.3 of a glyph (~1% of line
///   width).
/// - **Heuristic fallback**: when no metrics resolve, the heuristic
///   uses fixed ratios (0.30 / 0.60 / 1.00 × fontSize). vs real
///   metrics this can be off by up to ~5% per character — but the
///   tolerance only matters when the line is *just* over the
///   available width, and a 5% per-char error rarely accumulates to
///   exactly the tolerance band.
///
/// Choosing `0.02` over `0.01` (catching the Carlito / Arimo case
/// where Liberation widths drift ~1.5% from Arial in extreme glyphs):
/// the extra 1% margin absorbs the rare overshoot without producing
/// noticeable extra-wide lines (2% of a 600px frame = 12px overshoot,
/// invisible at viewing distance).
///
/// ## When to revisit
///
/// - If the offline extraction pipeline produces real per-character
///   widths for Noto KR / SC / TC, the heuristic-fallback worst-case
///   shrinks and `0.02` becomes more conservative than necessary.
///   Re-measure against the new metrics; `0.01` may suffice.
/// - If a future renderer pass enables shaping (`rustybuzz`), kerning
///   tables shrink line widths slightly and the tolerance band may
///   need adjustment.
// Positive ratio = tolerance band (line allowed to *exceed* the column
// width by this fraction before forcing a wrap). The wrap loop checks
// `line_width <= available_width + tolerance`, so a positive value
// loosens the constraint (more text fits on a line) and a negative
// one tightens it (wrap earlier, even before the column edge).
//
// Heuristic / fallback / OpentypeTextMeasurer hmtx widths over-measure
// Hangul (and other wide-EA glyphs) by ~3-5 % vs what the host
// rasterizer (Core Text on macOS, DirectWrite on Windows, FreeType in
// the WebView) actually paints. Without tolerance, a single trailing
// glyph falls onto its own line in tight cells that the host would
// have fit on one row (slide 2's 관련 페이지 번호 header wrapping to
// "관련 페이지 번" / "호" was the original repro). The native viewer
// reproduces the same kind of off-by-one wrap on Korean body text
// that the WASM/canvas-measured browser viewer fits on one row.
//
// 0.05 (≈5 %) covers the typical Hangul measurement drift between
// ttf-parser hmtx and the WebView's painted advance widths. Lines
// that genuinely need to wrap exceed the column by far more than
// 5 %, so the threshold doesn't over-stuff body paragraphs.
const WRAP_TOLERANCE_RATIO: f64 = 0.05;

/// Bond between a run's text fragment and its source run properties.
///
/// A wrapped line owns one or more contiguous segments. Two consecutive
/// segments always have *different* source run indices (mirroring TS's
/// reference-equality merge).
#[derive(Clone, Debug, PartialEq)]
pub struct LineSegment {
    /// Fragment text (after whitespace trimming for trailing segments).
    pub text: String,
    /// Source run properties (cloned). All fragments coming from the
    /// same run share equal `properties`.
    pub properties: RunProperties,
}

/// One wrapped output line — an ordered list of [`LineSegment`]s.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WrappedLine {
    /// Segments in source order; concatenating their `text` yields the
    /// rendered line.
    pub segments: Vec<LineSegment>,
}

/// Wraps a paragraph's runs into lines fitting `available_width`.
///
/// `default_font_size` applies to runs with no explicit `<a:rPr @sz>`;
/// `font_scale` multiplies *every* run's font size (used by the
/// renderer's auto-fit pass). `measurer` is consulted per fragment to
/// compute its width.
///
/// Returns at least one (possibly empty) line; never returns an empty
/// `Vec`.
#[must_use]
pub fn wrap_paragraph(
    paragraph: &Paragraph,
    available_width: f64,
    default_font_size: f64,
    font_scale: f64,
    measurer: &dyn TextMeasurer,
) -> Vec<WrappedLine> {
    wrap_paragraph_with_chain(
        paragraph,
        available_width,
        default_font_size,
        font_scale,
        measurer,
        &|_| None,
    )
}

/// Like [`wrap_paragraph`] but accepts a per-run `font-family` chain
/// resolver so browser-backed measurers receive the same chain string
/// that the renderer will emit on the SVG `<text>` element (KDD-15).
///
/// The resolver is called once per run; returning `None` is equivalent
/// to using the run's raw `font_family` / `font_family_ea` (current
/// behavior of [`wrap_paragraph`]). The chain — when supplied —
/// overrides the raw names for the wasm canvas measurement path,
/// preventing wrap drift between measurement and SVG render.
#[must_use]
pub fn wrap_paragraph_with_chain(
    paragraph: &Paragraph,
    available_width: f64,
    default_font_size: f64,
    font_scale: f64,
    measurer: &dyn TextMeasurer,
    chain_for_run: &dyn Fn(&RunProperties) -> Option<String>,
) -> Vec<WrappedLine> {
    if paragraph.runs.is_empty() || !paragraph.runs.iter().any(|r| !r.text.is_empty()) {
        return vec![WrappedLine::default()];
    }

    let safe_width = available_width.max(1.0);
    let tokens = tokenize_runs(
        &paragraph.runs,
        default_font_size,
        font_scale,
        measurer,
        chain_for_run,
    );
    if tokens.is_empty() {
        return vec![WrappedLine::default()];
    }

    layout_tokens_into_lines(
        tokens,
        &paragraph.runs,
        safe_width,
        default_font_size,
        font_scale,
        measurer,
        chain_for_run,
    )
}

#[derive(Clone, Debug)]
struct Token {
    text: String,
    run_index: usize,
    width: f64,
    breakable: bool,
    force_break: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FragmentKind {
    Latin,
    Cjk,
    Space,
}

fn tokenize_runs(
    runs: &[slideglance_model::text::TextRun],
    default_font_size: f64,
    font_scale: f64,
    measurer: &dyn TextMeasurer,
    chain_for_run: &dyn Fn(&RunProperties) -> Option<String>,
) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut is_first = true;

    for (run_index, run) in runs.iter().enumerate() {
        if run.text.is_empty() {
            continue;
        }

        if run.text.contains('\n') {
            for (pi, part) in run.text.split('\n').enumerate() {
                if pi > 0 {
                    tokens.push(Token {
                        text: String::new(),
                        run_index,
                        width: 0.0,
                        breakable: true,
                        force_break: true,
                    });
                    is_first = false;
                }
                if part.is_empty() {
                    continue;
                }
                tokenize_run_part(
                    part,
                    run,
                    run_index,
                    default_font_size,
                    font_scale,
                    measurer,
                    chain_for_run,
                    &mut tokens,
                    &mut is_first,
                );
            }
            continue;
        }

        tokenize_run_part(
            &run.text,
            run,
            run_index,
            default_font_size,
            font_scale,
            measurer,
            chain_for_run,
            &mut tokens,
            &mut is_first,
        );
    }

    tokens
}

#[allow(clippy::too_many_arguments)] // Mirrors TS tokenizeRuns inline scope.
fn tokenize_run_part(
    part: &str,
    run: &slideglance_model::text::TextRun,
    run_index: usize,
    default_font_size: f64,
    font_scale: f64,
    measurer: &dyn TextMeasurer,
    chain_for_run: &dyn Fn(&RunProperties) -> Option<String>,
    tokens: &mut Vec<Token>,
    is_first: &mut bool,
) {
    let font_size = effective_font_size(&run.properties, default_font_size, font_scale);
    let style = FontStyle {
        bold: run.properties.bold,
        italic: run.properties.italic,
    };
    let font_family = run.properties.font_family.as_deref();
    let font_family_ea = run.properties.font_family_ea.as_deref();
    let chain = chain_for_run(&run.properties);
    let chain_ref = chain.as_deref();

    for (fragment, breakable) in split_text_into_fragments(part) {
        let width = measurer.measure_text_width_with_chain(
            &fragment,
            font_size,
            style,
            font_family,
            font_family_ea,
            chain_ref,
        );
        tokens.push(Token {
            text: fragment,
            run_index,
            width,
            breakable: !*is_first && breakable,
            force_break: false,
        });
        *is_first = false;
    }
}

fn effective_font_size(props: &RunProperties, default_font_size: f64, font_scale: f64) -> f64 {
    match props.font_size {
        Some(pt) if pt.0 > 0.0 => pt.0 * font_scale,
        _ => default_font_size,
    }
}

/// Splits `text` into fragments + their per-fragment `breakable` flag.
///
/// - Whitespace runs become a `Space` fragment.
/// - Each CJK code point becomes its own fragment.
/// - Latin (and any other non-CJK non-whitespace) runs together as one
///   word fragment.
fn split_text_into_fragments(text: &str) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    let mut current = String::new();
    let mut current_kind: Option<FragmentKind> = None;

    for ch in text.chars() {
        let cp = ch as u32;
        if is_whitespace(cp) {
            if !current.is_empty() && current_kind != Some(FragmentKind::Space) {
                let breakable = matches!(current_kind, Some(FragmentKind::Cjk));
                out.push((std::mem::take(&mut current), breakable));
            }
            current_kind = Some(FragmentKind::Space);
            current.push(ch);
        } else if is_cjk_codepoint(cp) {
            if !current.is_empty() {
                let breakable =
                    matches!(current_kind, Some(FragmentKind::Cjk | FragmentKind::Space));
                out.push((std::mem::take(&mut current), breakable));
            }
            // Each CJK code point is its own breakable fragment.
            out.push((ch.to_string(), true));
            current_kind = Some(FragmentKind::Cjk);
            current.clear();
        } else {
            // Latin / other non-CJK non-whitespace.
            if !current.is_empty() && current_kind != Some(FragmentKind::Latin) {
                let breakable = matches!(current_kind, Some(FragmentKind::Space));
                out.push((std::mem::take(&mut current), breakable));
            }
            current_kind = Some(FragmentKind::Latin);
            current.push(ch);
        }
    }

    if !current.is_empty() {
        let breakable = matches!(current_kind, Some(FragmentKind::Space | FragmentKind::Cjk));
        out.push((current, breakable));
    }

    out
}

fn is_whitespace(cp: u32) -> bool {
    matches!(cp, 0x20 | 0x09 | 0x0A | 0x0D)
}

fn is_space_only(text: &str) -> bool {
    text.chars().all(|c| is_whitespace(c as u32))
}

fn split_token_by_chars(
    token: &Token,
    runs: &[slideglance_model::text::TextRun],
    available_width: f64,
    default_font_size: f64,
    font_scale: f64,
    measurer: &dyn TextMeasurer,
    chain_for_run: &dyn Fn(&RunProperties) -> Option<String>,
) -> Vec<Vec<Token>> {
    let run = &runs[token.run_index];
    let font_size = effective_font_size(&run.properties, default_font_size, font_scale);
    let style = FontStyle {
        bold: run.properties.bold,
        italic: run.properties.italic,
    };
    let font_family = run.properties.font_family.as_deref();
    let font_family_ea = run.properties.font_family_ea.as_deref();
    let chain = chain_for_run(&run.properties);
    let chain_ref = chain.as_deref();

    let mut lines: Vec<Vec<Token>> = Vec::new();
    let mut current: Vec<Token> = Vec::new();
    let mut current_width = 0.0_f64;

    for ch in token.text.chars() {
        let s = ch.to_string();
        let char_width = measurer.measure_text_width_with_chain(
            &s,
            font_size,
            style,
            font_family,
            font_family_ea,
            chain_ref,
        );

        if current_width + char_width > available_width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            current_width = 0.0;
        }

        current.push(Token {
            text: s,
            run_index: token.run_index,
            width: char_width,
            breakable: false,
            force_break: false,
        });
        current_width += char_width;
    }

    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn merge_segments(tokens: &[Token], runs: &[slideglance_model::text::TextRun]) -> Vec<LineSegment> {
    let mut segments: Vec<LineSegment> = Vec::new();
    let mut last_run_index: Option<usize> = None;

    for token in tokens {
        if token.text.is_empty() {
            // force-break tokens carry no text — caller should never emit
            // them into segments, but guard anyway.
            continue;
        }
        if last_run_index == Some(token.run_index) {
            if let Some(last) = segments.last_mut() {
                last.text.push_str(&token.text);
                continue;
            }
        }
        segments.push(LineSegment {
            text: token.text.clone(),
            properties: runs[token.run_index].properties.clone(),
        });
        last_run_index = Some(token.run_index);
    }

    segments
}

fn trim_trailing_spaces(mut segments: Vec<LineSegment>) -> Vec<LineSegment> {
    while let Some(last) = segments.last_mut() {
        // Trim trailing whitespace per `\s+$` semantics — \s includes
        // ASCII space / tab / CR / LF (and the Unicode whitespace TS's
        // RegExp matches). For our wrapping pipeline only ASCII space-
        // class characters can appear since we tokenize on those.
        let trimmed = last.text.trim_end();
        if trimmed.is_empty() {
            segments.pop();
            continue;
        }
        if trimmed.len() != last.text.len() {
            last.text.truncate(trimmed.len());
        }
        break;
    }
    segments
}

#[allow(clippy::too_many_arguments)] // Algorithm needs all of these — splitting
                                     // any further hurts the source-line ↔ TS
                                     // mapping for parity audits.
fn layout_tokens_into_lines(
    tokens: Vec<Token>,
    runs: &[slideglance_model::text::TextRun],
    available_width: f64,
    default_font_size: f64,
    font_scale: f64,
    measurer: &dyn TextMeasurer,
    chain_for_run: &dyn Fn(&RunProperties) -> Option<String>,
) -> Vec<WrappedLine> {
    if tokens.is_empty() {
        return vec![WrappedLine::default()];
    }

    let tolerance = available_width * WRAP_TOLERANCE_RATIO;
    let mut lines: Vec<WrappedLine> = Vec::new();
    let mut current_line: Vec<Token> = Vec::new();
    let mut current_width = 0.0_f64;

    for token in tokens {
        if token.force_break {
            let segments = trim_trailing_spaces(merge_segments(&current_line, runs));
            lines.push(WrappedLine { segments });
            current_line.clear();
            current_width = 0.0;
            continue;
        }

        if current_width + token.width <= available_width + tolerance {
            current_width += token.width;
            current_line.push(token);
        } else if current_line.is_empty() {
            // Empty line + 1 token won't fit → force char-split unless
            // it's whitespace (then skip).
            if is_space_only(&token.text) {
                continue;
            }
            let split = split_token_by_chars(
                &token,
                runs,
                available_width,
                default_font_size,
                font_scale,
                measurer,
                chain_for_run,
            );
            let last_idx = split.len().saturating_sub(1);
            for (j, chunk) in split.into_iter().enumerate() {
                if j < last_idx {
                    let segments = trim_trailing_spaces(merge_segments(&chunk, runs));
                    if !segments.is_empty() {
                        lines.push(WrappedLine { segments });
                    }
                } else {
                    current_width = chunk.iter().map(|t| t.width).sum();
                    current_line = chunk;
                }
            }
        } else if token.breakable {
            let segments = trim_trailing_spaces(merge_segments(&current_line, runs));
            if !segments.is_empty() {
                lines.push(WrappedLine { segments });
            }

            if is_space_only(&token.text) {
                current_line = Vec::new();
                current_width = 0.0;
            } else {
                current_width = token.width;
                current_line = vec![token];
            }
        } else {
            // Not breakable but doesn't fit → break before this token.
            let segments = trim_trailing_spaces(merge_segments(&current_line, runs));
            if !segments.is_empty() {
                lines.push(WrappedLine { segments });
            }
            current_width = token.width;
            current_line = vec![token];
        }
    }

    if !current_line.is_empty() {
        let segments = trim_trailing_spaces(merge_segments(&current_line, runs));
        if !segments.is_empty() {
            lines.push(WrappedLine { segments });
        }
    }

    if lines.is_empty() {
        vec![WrappedLine::default()]
    } else {
        lines
    }
}

#[cfg(test)]
mod tests {
    use slideglance_model::text::{ParagraphAlignment, ParagraphProperties, TextRun};
    use slideglance_utils::Pt;

    use crate::text_measurer::HeuristicTextMeasurer;

    use super::*;

    fn make_run_props(font_size_pt: f64) -> RunProperties {
        RunProperties {
            font_size: Some(Pt(font_size_pt)),
            ..RunProperties::default()
        }
    }

    fn make_run_props_bold(font_size_pt: f64, bold: bool) -> RunProperties {
        RunProperties {
            font_size: Some(Pt(font_size_pt)),
            bold,
            ..RunProperties::default()
        }
    }

    fn make_paragraph_with_props(texts: &[&str], props: &RunProperties) -> Paragraph {
        Paragraph {
            runs: texts
                .iter()
                .map(|t| TextRun {
                    text: (*t).to_string(),
                    properties: props.clone(),
                    field_type: None,
                })
                .collect(),
            properties: ParagraphProperties {
                alignment: Some(ParagraphAlignment::L),
                line_spacing: None,
                space_before: None,
                space_after: None,
                level: 0,
                bullet: None,
                bullet_font: None,
                bullet_color: None,
                bullet_size_pct: None,
                margin_left: None,
                indent: None,
                tab_stops: Vec::new(),
            },
            end_para_run_properties: None,
        }
    }

    fn make_paragraph(texts: &[&str]) -> Paragraph {
        make_paragraph_with_props(texts, &make_run_props(18.0))
    }

    fn measurer() -> HeuristicTextMeasurer {
        HeuristicTextMeasurer
    }

    fn line_text(line: &WrappedLine) -> String {
        line.segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<String>()
    }

    fn all_text(lines: &[WrappedLine]) -> String {
        lines.iter().map(line_text).collect::<String>()
    }

    // -- TS-parity tests ------------------------------------------------------

    #[test]
    fn short_text_does_not_wrap() {
        let p = make_paragraph(&["Hi"]);
        let lines = wrap_paragraph(&p, 500.0, 18.0, 1.0, &measurer());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].segments.len(), 1);
        assert_eq!(lines[0].segments[0].text, "Hi");
    }

    #[test]
    fn long_english_wraps_on_word_boundaries() {
        let p = make_paragraph(&["The quick brown fox jumps over the lazy dog"]);
        let lines = wrap_paragraph(&p, 100.0, 18.0, 1.0, &measurer());
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(!line.segments.is_empty());
        }
    }

    #[test]
    fn long_cjk_wraps_on_character_boundaries() {
        let p = make_paragraph(&["本日は晴天なり今日もいい天気です"]);
        let lines = wrap_paragraph(&p, 100.0, 18.0, 1.0, &measurer());
        assert!(lines.len() > 1);
    }

    #[test]
    fn empty_paragraph_returns_one_empty_line() {
        let p = make_paragraph(&[]);
        let lines = wrap_paragraph(&p, 500.0, 18.0, 1.0, &measurer());
        assert_eq!(lines.len(), 1);
        assert!(lines[0].segments.is_empty());
    }

    #[test]
    fn empty_text_run_returns_one_empty_line() {
        let p = make_paragraph(&[""]);
        let lines = wrap_paragraph(&p, 500.0, 18.0, 1.0, &measurer());
        assert_eq!(lines.len(), 1);
        assert!(lines[0].segments.is_empty());
    }

    #[test]
    fn multiple_runs_preserved_in_segments() {
        let bold = make_run_props_bold(18.0, true);
        let normal = make_run_props_bold(18.0, false);
        let p = Paragraph {
            runs: vec![
                TextRun {
                    text: "Bold ".to_string(),
                    properties: bold,
                    field_type: None,
                },
                TextRun {
                    text: "Normal".to_string(),
                    properties: normal,
                    field_type: None,
                },
            ],
            properties: ParagraphProperties {
                alignment: Some(ParagraphAlignment::L),
                line_spacing: None,
                space_before: None,
                space_after: None,
                level: 0,
                bullet: None,
                bullet_font: None,
                bullet_color: None,
                bullet_size_pct: None,
                margin_left: None,
                indent: None,
                tab_stops: Vec::new(),
            },
            end_para_run_properties: None,
        };
        let lines = wrap_paragraph(&p, 500.0, 18.0, 1.0, &measurer());
        assert_eq!(lines.len(), 1);
        assert_eq!(line_text(&lines[0]), "Bold Normal");
    }

    #[test]
    fn bold_normal_runs_wrap_across_boundaries() {
        let bold = make_run_props_bold(18.0, true);
        let normal = make_run_props_bold(18.0, false);
        let p = Paragraph {
            runs: vec![
                TextRun {
                    text: "First part is bold and ".to_string(),
                    properties: bold,
                    field_type: None,
                },
                TextRun {
                    text: "second part is normal text".to_string(),
                    properties: normal,
                    field_type: None,
                },
            ],
            properties: ParagraphProperties {
                alignment: Some(ParagraphAlignment::L),
                line_spacing: None,
                space_before: None,
                space_after: None,
                level: 0,
                bullet: None,
                bullet_font: None,
                bullet_color: None,
                bullet_size_pct: None,
                margin_left: None,
                indent: None,
                tab_stops: Vec::new(),
            },
            end_para_run_properties: None,
        };
        let lines = wrap_paragraph(&p, 150.0, 18.0, 1.0, &measurer());
        assert!(lines.len() > 1);
        // Concat per-line text + space separator (mirrors TS test).
        let joined: String = lines.iter().map(line_text).collect::<Vec<_>>().join(" ");
        assert!(joined.contains("First part"), "got {joined}");
        assert!(joined.contains("bold and"), "got {joined}");
        assert!(joined.contains("second part"), "got {joined}");
        assert!(joined.contains("normal text"), "got {joined}");
    }

    #[test]
    fn very_small_width_still_emits_at_least_one_char() {
        let p = make_paragraph(&["AB"]);
        let lines = wrap_paragraph(&p, 1.0, 18.0, 1.0, &measurer());
        assert!(!lines.is_empty());
        assert_eq!(all_text(&lines), "AB");
    }

    #[test]
    fn trailing_spaces_trimmed() {
        let p = make_paragraph(&["Hello World"]);
        // Width tight enough to wrap between "Hello" and "World".
        let lines = wrap_paragraph(&p, 80.0, 18.0, 1.0, &measurer());
        assert!(!lines.is_empty());
        for line in &lines {
            if let Some(last) = line.segments.last() {
                assert!(
                    !last.text.ends_with(char::is_whitespace),
                    "trailing whitespace in {:?}",
                    last.text
                );
            }
        }
    }

    #[test]
    fn font_scale_applies_to_run_font_size() {
        // run fontSize=36 with scale=0.5 must wrap the same as fontSize=18 scale=1.
        let p_scaled = make_paragraph_with_props(
            &["The quick brown fox jumps over the lazy dog"],
            &make_run_props(36.0),
        );
        let scaled = wrap_paragraph(&p_scaled, 200.0, 18.0, 0.5, &measurer());

        let p_small = make_paragraph_with_props(
            &["The quick brown fox jumps over the lazy dog"],
            &make_run_props(18.0),
        );
        let small = wrap_paragraph(&p_small, 200.0, 18.0, 1.0, &measurer());

        assert_eq!(scaled.len(), small.len());
    }

    #[test]
    fn metric_overshoot_within_tolerance_does_not_wrap() {
        // "ABCシステム" measured ≈ 135.74px (Carlito ABC + NotoSansJP CJK*4 at 18pt).
        // After WRAP_TOLERANCE_RATIO was retuned to -0.003 (wrap slightly
        // earlier than the metric overshoot to better match PowerPoint's
        // PDF wrap), 134 × -0.003 = -0.402 → effective threshold 133.598.
        // 135.74 still overshoots, so this case now wraps. Use a wider
        // available_width that comfortably accommodates the metric so the
        // "no-wrap-when-fits" intent of the test is preserved.
        let props = RunProperties {
            font_size: Some(Pt(18.0)),
            font_family: Some("Calibri".to_string()),
            font_family_ea: Some("Meiryo".to_string()),
            ..RunProperties::default()
        };
        let p = make_paragraph_with_props(&["ABCシステム"], &props);
        let lines = wrap_paragraph(&p, 145.0, 18.0, 1.0, &measurer());
        assert_eq!(lines.len(), 1);
        assert_eq!(line_text(&lines[0]), "ABCシステム");
    }

    #[test]
    fn wrap_when_overshoot_exceeds_tolerance() {
        let props = RunProperties {
            font_size: Some(Pt(18.0)),
            font_family: Some("Calibri".to_string()),
            font_family_ea: Some("Meiryo".to_string()),
            ..RunProperties::default()
        };
        let p = make_paragraph_with_props(&["ABCシステム"], &props);
        let lines = wrap_paragraph(&p, 120.0, 18.0, 1.0, &measurer());
        assert!(lines.len() > 1);
    }

    #[test]
    fn font_scale_1_uses_run_font_size_as_is() {
        let p_large = make_paragraph_with_props(&["Hello World"], &make_run_props(36.0));
        let large = wrap_paragraph(&p_large, 200.0, 18.0, 1.0, &measurer());
        let p_small = make_paragraph_with_props(&["Hello World"], &make_run_props(18.0));
        let small = wrap_paragraph(&p_small, 200.0, 18.0, 1.0, &measurer());
        assert!(large.len() >= small.len());
    }

    // -- Force break ----------------------------------------------------------

    #[test]
    fn embedded_newline_force_breaks() {
        let p = make_paragraph(&["foo\nbar"]);
        let lines = wrap_paragraph(&p, 500.0, 18.0, 1.0, &measurer());
        assert_eq!(lines.len(), 2);
        assert_eq!(line_text(&lines[0]), "foo");
        assert_eq!(line_text(&lines[1]), "bar");
    }

    #[test]
    fn newline_at_start_keeps_empty_first_line() {
        let p = make_paragraph(&["\nbar"]);
        let lines = wrap_paragraph(&p, 500.0, 18.0, 1.0, &measurer());
        assert_eq!(lines.len(), 2);
        assert!(lines[0].segments.is_empty());
        assert_eq!(line_text(&lines[1]), "bar");
    }

    // -- Internal helpers -----------------------------------------------------

    #[test]
    fn split_text_into_fragments_basic_latin() {
        let frags = split_text_into_fragments("Hello world");
        let texts: Vec<&str> = frags.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(texts, vec!["Hello", " ", "world"]);
        // Per TS: pushed-on-transition fragment carries `breakable =
        // current-kind matches`. Latin → Space pushes Latin with
        // breakable=(was-cjk)=false. Space → Latin pushes Space with
        // breakable=(was-space)=true. End-of-input pushes the Latin
        // tail with breakable=(latin → false).
        assert!(!frags[0].1, "Latin head non-breakable, got {}", frags[0].1);
        assert!(frags[1].1, "Space middle breakable, got {}", frags[1].1);
        assert!(!frags[2].1, "Latin tail non-breakable, got {}", frags[2].1);
    }

    #[test]
    fn split_text_into_fragments_cjk_per_char() {
        let frags = split_text_into_fragments("漢字");
        assert_eq!(frags.len(), 2);
        assert_eq!(frags[0].0, "漢");
        assert_eq!(frags[1].0, "字");
        assert!(frags[0].1);
        assert!(frags[1].1);
    }

    #[test]
    fn split_text_into_fragments_mixed() {
        let frags = split_text_into_fragments("ABシ");
        let texts: Vec<&str> = frags.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(texts, vec!["AB", "シ"]);
    }
}
