// Same rationale as in `mapping.rs` — OOXML / Adobe / Google product
// names are mixed-case proper nouns rather than code identifiers, and
// wrapping every one in backticks would litter the rustdoc tables.
#![allow(clippy::doc_markdown)]

//! Fallback font metrics keyed by OSS family name.
//!
//! [`get_font_metrics`] resolves a PPTX typeface by **first** running it
//! through [`crate::mapping::get_mapped_font`] (the single source of
//! truth for "PPTX name → OSS replacement") and **then** looking up the
//! resulting OSS family name in this module's metric table. This makes
//! mapping.rs the only place that defines name → OSS-name resolution;
//! adding a new key here only requires an entry in mapping.rs (or this
//! file as the OSS endpoint), never both.
//!
//! ## Coverage
//!
//! | OSS family | source | per-character widths |
//! |-----------|--------|----------------------|
//! | Carlito (Calibri-compatible) | extracted | full ASCII + Latin-1 |
//! | Arimo (Arial / Helvetica-compatible) | extracted from Liberation Sans (Arimo's metric twin) | full ASCII + Latin-1 |
//! | Tinos (Times-compatible) | extracted from Liberation Serif (Tinos's metric twin) | full ASCII + Latin-1 |
//! | Noto Sans JP | extracted | full ASCII + Latin-1 |
//! | Noto Serif JP | scalar approximation | none — uses cjk_width / default_width |
//! | Noto Sans KR  | scalar approximation | none |
//! | Noto Serif KR | scalar approximation | none |
//! | Noto Sans SC  | scalar approximation | none |
//! | Noto Serif SC | scalar approximation | none |
//! | Noto Sans TC  | scalar approximation | none |
//! | Noto Serif TC | scalar approximation | none |
//!
//! The KR / SC / TC entries use scalar-only approximations (matching
//! the Noto Sans JP unitsPerEm / ascender / descender / cjk_width and a
//! representative Latin default_width) until the offline extraction
//! pipeline produces real per-character widths. CJK glyphs measure
//! correctly via `cjk_width` (full-width = unitsPerEm); Latin glyphs in
//! a CJK-named run get a representative width.
//!
//! Cousine (Courier-compatible, monospace) ships scalar metrics plus a
//! single representative width — every ASCII glyph in a monospace face
//! shares the same advance, so a single `default_width` is exact for
//! `\\u{20}..\\u{7e}` and a `cjk_width` covers full-width CJK glyphs in
//! mixed runs. Caladea (Cambria-compatible) ships scalar metrics and a
//! representative `default_width` derived from PT Serif's average
//! advance; per-character widths land alongside the offline-extraction
//! pipeline upgrade.
//!
//! ## CJK Script Equality
//!
//! Every CJK script (Jpan / Hang / Hans / Hant) × style (sans / serif)
//! has an entry, mirroring the 8 cells declared in `mapping.rs`. JP
//! has the richest data (per-character widths) only because the TS
//! reference happened to extract it first; the other seven cells use
//! scalar approximations until extraction catches up. *Behavior* is
//! symmetric — every CJK script falls through the same lookup → scalar
//! approximation → measurer-heuristic chain.

use std::sync::OnceLock;

use crate::mapping::{default_font_mapping, get_mapped_font, FontMapping};

/// Per-font metric data, scalar + optional per-character widths.
///
/// Scalar fields are always populated. `widths` is per-character and
/// may be empty for entries that only have approximated scalar data
/// (the seven Noto CJK entries other than JP) — those fall back to
/// `cjk_width` for CJK code points and `default_width` for everything
/// else.
pub struct FontMetrics {
    /// Em square in font units.
    pub units_per_em: u16,
    /// Ascender height in font units (positive).
    pub ascender: i16,
    /// Descender depth in font units (negative).
    pub descender: i16,
    /// Width applied to characters with no entry in `widths` and which
    /// are not CJK.
    pub default_width: u16,
    /// Width applied to CJK characters that have no explicit width entry.
    pub cjk_width: u16,
    /// OSS font family name — *exactly* matches the corresponding
    /// `mapping.rs` mapping target. Adding the OSS family name to a
    /// SVG `font-family` chain therefore reflects what the renderer's
    /// resolver would have chosen anyway.
    pub fallback_name: &'static str,
    widths: &'static [(char, u16)],
}

impl FontMetrics {
    /// Returns the width of `c` in font units, if the table has an entry.
    ///
    /// `widths` is a sorted slice; lookup uses `binary_search_by_key`
    /// for O(log n) access without heap allocation.
    #[must_use]
    pub fn width_of(&self, c: char) -> Option<u16> {
        self.widths
            .binary_search_by_key(&c, |&(k, _)| k)
            .ok()
            .map(|i| self.widths[i].1)
    }

    /// Number of width entries — exposed for tests and parity audits.
    #[must_use]
    pub fn widths_len(&self) -> usize {
        self.widths.len()
    }

    /// Raw sorted slice — exposed for sort-order validation in tests.
    #[must_use]
    pub fn widths_slice(&self) -> &'static [(char, u16)] {
        self.widths
    }
}

// ---------------------------------------------------------------------------
// Static instances — lazy-built once on first call.
// ---------------------------------------------------------------------------

static CARLITO: OnceLock<FontMetrics> = OnceLock::new();
static ARIMO: OnceLock<FontMetrics> = OnceLock::new();
static TINOS: OnceLock<FontMetrics> = OnceLock::new();
static COUSINE: OnceLock<FontMetrics> = OnceLock::new();
static CALADEA: OnceLock<FontMetrics> = OnceLock::new();
static NOTO_SANS_JP: OnceLock<FontMetrics> = OnceLock::new();
static NOTO_SERIF_JP: OnceLock<FontMetrics> = OnceLock::new();
static NOTO_SANS_KR: OnceLock<FontMetrics> = OnceLock::new();
static NOTO_SERIF_KR: OnceLock<FontMetrics> = OnceLock::new();
static NOTO_SANS_SC: OnceLock<FontMetrics> = OnceLock::new();
static NOTO_SERIF_SC: OnceLock<FontMetrics> = OnceLock::new();
static NOTO_SANS_TC: OnceLock<FontMetrics> = OnceLock::new();
static NOTO_SERIF_TC: OnceLock<FontMetrics> = OnceLock::new();

static CACHED_MAPPING: OnceLock<FontMapping> = OnceLock::new();

fn cached_mapping() -> &'static FontMapping {
    CACHED_MAPPING.get_or_init(default_font_mapping)
}

fn carlito() -> &'static FontMetrics {
    CARLITO.get_or_init(build_carlito)
}

fn arimo() -> &'static FontMetrics {
    ARIMO.get_or_init(build_arimo)
}

fn tinos() -> &'static FontMetrics {
    TINOS.get_or_init(build_tinos)
}

fn cousine() -> &'static FontMetrics {
    COUSINE.get_or_init(build_cousine)
}

fn caladea() -> &'static FontMetrics {
    CALADEA.get_or_init(build_caladea)
}

fn noto_sans_jp() -> &'static FontMetrics {
    NOTO_SANS_JP.get_or_init(build_noto_sans_jp)
}

fn noto_serif_jp() -> &'static FontMetrics {
    NOTO_SERIF_JP.get_or_init(|| build_noto_cjk_approx("Noto Serif JP"))
}

fn noto_sans_kr() -> &'static FontMetrics {
    NOTO_SANS_KR.get_or_init(|| build_noto_cjk_approx("Noto Sans KR"))
}

fn noto_serif_kr() -> &'static FontMetrics {
    NOTO_SERIF_KR.get_or_init(|| build_noto_cjk_approx("Noto Serif KR"))
}

fn noto_sans_sc() -> &'static FontMetrics {
    NOTO_SANS_SC.get_or_init(|| build_noto_cjk_approx("Noto Sans SC"))
}

fn noto_serif_sc() -> &'static FontMetrics {
    NOTO_SERIF_SC.get_or_init(|| build_noto_cjk_approx("Noto Serif SC"))
}

fn noto_sans_tc() -> &'static FontMetrics {
    NOTO_SANS_TC.get_or_init(|| build_noto_cjk_approx("Noto Sans TC"))
}

fn noto_serif_tc() -> &'static FontMetrics {
    NOTO_SERIF_TC.get_or_init(|| build_noto_cjk_approx("Noto Serif TC"))
}

// ---------------------------------------------------------------------------
// Public lookup
// ---------------------------------------------------------------------------

/// Resolves a PPTX font name to its fallback metrics, or `None` when
/// the name has no registered OSS endpoint.
///
/// Resolution order:
///
/// 1. Direct match against an OSS family name (`Carlito`, `Arimo`,
///    `Noto Sans JP`, …) — useful for callers that already resolved
///    through [`crate::mapping::get_mapped_font`].
/// 2. Run the input through [`crate::mapping::get_mapped_font`] and
///    look up the resulting OSS family name.
///
/// Step 2 ensures every PPTX name registered in `mapping.rs` is
/// covered automatically — adding a new mapping entry there makes its
/// metrics available here without further plumbing.
#[must_use]
pub fn get_font_metrics(font_family: &str) -> Option<&'static FontMetrics> {
    if font_family.is_empty() {
        return None;
    }
    if let Some(m) = metrics_for_oss_name(font_family) {
        return Some(m);
    }
    let mapped = get_mapped_font(font_family, cached_mapping())?;
    metrics_for_oss_name(&mapped)
}

/// Returns the OSS family name that the renderer should put on the
/// SVG `font-family` fallback list for `font_family`, or `None` when
/// the input has no registered OSS endpoint.
///
/// The result is identical to
/// `get_font_metrics(font_family).map(|m| m.fallback_name)` — by
/// invariant the metric's `fallback_name` matches `mapping.rs`'s
/// resolution target.
#[must_use]
pub fn get_metrics_fallback_font(font_family: &str) -> Option<&'static str> {
    get_font_metrics(font_family).map(|m| m.fallback_name)
}

/// OSS family name → metrics dispatch. Single source of truth for
/// the OSS endpoint side; expand here whenever a new OSS face gets
/// metric data extracted.
fn metrics_for_oss_name(name: &str) -> Option<&'static FontMetrics> {
    match name {
        "Carlito" => Some(carlito()),
        "Arimo" => Some(arimo()),
        "Tinos" => Some(tinos()),
        "Cousine" => Some(cousine()),
        "Caladea" => Some(caladea()),
        "Noto Sans JP" => Some(noto_sans_jp()),
        "Noto Serif JP" => Some(noto_serif_jp()),
        "Noto Sans KR" => Some(noto_sans_kr()),
        "Noto Serif KR" => Some(noto_serif_kr()),
        "Noto Sans SC" => Some(noto_sans_sc()),
        "Noto Serif SC" => Some(noto_serif_sc()),
        "Noto Sans TC" => Some(noto_sans_tc()),
        "Noto Serif TC" => Some(noto_serif_tc()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Carlito (Calibri-compatible)
// ---------------------------------------------------------------------------

// Sorted by char codepoint — required by binary_search_by_key in width_of().
#[allow(clippy::too_many_lines)]
static CARLITO_WIDTHS: &[(char, u16)] = &[
    (' ', 463),
    ('!', 667),
    ('"', 821),
    ('#', 1020),
    ('$', 1038),
    ('%', 1464),
    ('&', 1397),
    ('\'', 452),
    ('(', 621),
    (')', 621),
    ('*', 1020),
    ('+', 1020),
    (',', 511),
    ('-', 627),
    ('.', 517),
    ('/', 791),
    ('0', 1038),
    ('1', 1038),
    ('2', 1038),
    ('3', 1038),
    ('4', 1038),
    ('5', 1038),
    ('6', 1038),
    ('7', 1038),
    ('8', 1038),
    ('9', 1038),
    (':', 548),
    (';', 548),
    ('<', 1020),
    ('=', 1020),
    ('>', 1020),
    ('?', 949),
    ('@', 1831),
    ('A', 1185),
    ('B', 1114),
    ('C', 1092),
    ('D', 1260),
    ('E', 1000),
    ('F', 941),
    ('G', 1292),
    ('H', 1276),
    ('I', 516),
    ('J', 653),
    ('K', 1064),
    ('L', 861),
    ('M', 1751),
    ('N', 1322),
    ('O', 1356),
    ('P', 1058),
    ('Q', 1378),
    ('R', 1112),
    ('S', 941),
    ('T', 998),
    ('U', 1314),
    ('V', 1162),
    ('W', 1822),
    ('X', 1063),
    ('Y', 998),
    ('Z', 959),
    ('[', 628),
    ('\\', 791),
    (']', 628),
    ('^', 1020),
    ('_', 1020),
    ('`', 596),
    ('a', 981),
    ('b', 1076),
    ('c', 866),
    ('d', 1076),
    ('e', 1019),
    ('f', 625),
    ('g', 964),
    ('h', 1076),
    ('i', 470),
    ('j', 490),
    ('k', 931),
    ('l', 470),
    ('m', 1636),
    ('n', 1076),
    ('o', 1080),
    ('p', 1076),
    ('q', 1076),
    ('r', 714),
    ('s', 801),
    ('t', 686),
    ('u', 1076),
    ('v', 925),
    ('w', 1464),
    ('x', 887),
    ('y', 927),
    ('z', 809),
    ('{', 644),
    ('|', 943),
    ('}', 644),
    ('~', 1020),
    ('\u{00a0}', 463),
    ('\u{00a1}', 667),
    ('\u{00a2}', 1020),
    ('\u{00a3}', 1038),
    ('\u{00a4}', 1020),
    ('\u{00a5}', 1038),
    ('\u{00a6}', 1020),
    ('\u{00a7}', 1020),
    ('\u{00a8}', 804),
    ('\u{00a9}', 1709),
    ('\u{00aa}', 824),
    ('\u{00ab}', 1049),
    ('\u{00ac}', 1020),
    ('\u{00ae}', 1038),
    ('\u{00af}', 807),
    ('\u{00b0}', 694),
    ('\u{00b1}', 1020),
    ('\u{00b2}', 688),
    ('\u{00b3}', 685),
    ('\u{00b4}', 598),
    ('\u{00b5}', 1126),
    ('\u{00b6}', 1200),
    ('\u{00b7}', 517),
    ('\u{00b8}', 629),
    ('\u{00b9}', 504),
    ('\u{00ba}', 865),
    ('\u{00bb}', 1049),
    ('\u{00bc}', 1303),
    ('\u{00bd}', 1375),
    ('\u{00be}', 1383),
    ('\u{00bf}', 949),
    ('\u{00c0}', 1185),
    ('\u{00c1}', 1185),
    ('\u{00c2}', 1185),
    ('\u{00c3}', 1185),
    ('\u{00c4}', 1185),
    ('\u{00c5}', 1185),
    ('\u{00c6}', 1563),
    ('\u{00c7}', 1092),
    ('\u{00c8}', 1000),
    ('\u{00c9}', 1000),
    ('\u{00ca}', 1000),
    ('\u{00cb}', 1000),
    ('\u{00cc}', 516),
    ('\u{00cd}', 516),
    ('\u{00ce}', 516),
    ('\u{00cf}', 516),
    ('\u{00d0}', 1279),
    ('\u{00d1}', 1322),
    ('\u{00d2}', 1356),
    ('\u{00d3}', 1356),
    ('\u{00d4}', 1356),
    ('\u{00d5}', 1356),
    ('\u{00d6}', 1356),
    ('\u{00d7}', 1020),
    ('\u{00d8}', 1359),
    ('\u{00d9}', 1314),
    ('\u{00da}', 1314),
    ('\u{00db}', 1314),
    ('\u{00dc}', 1314),
    ('\u{00dd}', 998),
    ('\u{00de}', 1058),
    ('\u{00df}', 1080),
    ('\u{00e0}', 981),
    ('\u{00e1}', 981),
    ('\u{00e2}', 981),
    ('\u{00e3}', 981),
    ('\u{00e4}', 981),
    ('\u{00e5}', 981),
    ('\u{00e6}', 1583),
    ('\u{00e7}', 866),
    ('\u{00e8}', 1019),
    ('\u{00e9}', 1019),
    ('\u{00ea}', 1019),
    ('\u{00eb}', 1019),
    ('\u{00ec}', 470),
    ('\u{00ed}', 470),
    ('\u{00ee}', 470),
    ('\u{00ef}', 470),
    ('\u{00f0}', 1076),
    ('\u{00f1}', 1076),
    ('\u{00f2}', 1080),
    ('\u{00f3}', 1080),
    ('\u{00f4}', 1080),
    ('\u{00f5}', 1080),
    ('\u{00f6}', 1080),
    ('\u{00f7}', 1020),
    ('\u{00f8}', 1084),
    ('\u{00f9}', 1076),
    ('\u{00fa}', 1076),
    ('\u{00fb}', 1076),
    ('\u{00fc}', 1076),
    ('\u{00fd}', 927),
    ('\u{00fe}', 1076),
    ('\u{00ff}', 927),
];

fn build_carlito() -> FontMetrics {
    FontMetrics {
        units_per_em: 2048,
        ascender: 1950,
        descender: -550,
        default_width: 991,
        cjk_width: 2048,
        fallback_name: "Carlito",
        widths: CARLITO_WIDTHS,
    }
}

// ---------------------------------------------------------------------------
// Arimo (Arial / Helvetica-compatible).
//
// Width data is extracted from Liberation Sans, which is metric-
// compatible with Arimo (both are independently engineered metric
// clones of Arial — they share the same glyph advance widths to
// within rounding). Mapping.rs sends `Arial`/`Helvetica` to `Arimo`,
// so labelling the fallback_name as `Arimo` keeps the renderer's SVG
// `font-family` chain consistent with the resolver's choice.
// ---------------------------------------------------------------------------

// Sorted by char codepoint — required by binary_search_by_key in width_of().
#[allow(clippy::too_many_lines)]
static ARIMO_WIDTHS: &[(char, u16)] = &[
    (' ', 569),
    ('!', 569),
    ('"', 727),
    ('#', 1139),
    ('$', 1139),
    ('%', 1821),
    ('&', 1366),
    ('\'', 391),
    ('(', 682),
    (')', 682),
    ('*', 797),
    ('+', 1196),
    (',', 569),
    ('-', 682),
    ('.', 569),
    ('/', 569),
    ('0', 1139),
    ('1', 1139),
    ('2', 1139),
    ('3', 1139),
    ('4', 1139),
    ('5', 1139),
    ('6', 1139),
    ('7', 1139),
    ('8', 1139),
    ('9', 1139),
    (':', 569),
    (';', 569),
    ('<', 1196),
    ('=', 1196),
    ('>', 1196),
    ('?', 1139),
    ('@', 2079),
    ('A', 1366),
    ('B', 1366),
    ('C', 1479),
    ('D', 1479),
    ('E', 1366),
    ('F', 1251),
    ('G', 1593),
    ('H', 1479),
    ('I', 569),
    ('J', 1024),
    ('K', 1366),
    ('L', 1139),
    ('M', 1706),
    ('N', 1479),
    ('O', 1593),
    ('P', 1366),
    ('Q', 1593),
    ('R', 1479),
    ('S', 1366),
    ('T', 1251),
    ('U', 1479),
    ('V', 1366),
    ('W', 1933),
    ('X', 1366),
    ('Y', 1366),
    ('Z', 1251),
    ('[', 569),
    ('\\', 569),
    (']', 569),
    ('^', 961),
    ('_', 1139),
    ('`', 682),
    ('a', 1139),
    ('b', 1139),
    ('c', 1024),
    ('d', 1139),
    ('e', 1139),
    ('f', 569),
    ('g', 1139),
    ('h', 1139),
    ('i', 455),
    ('j', 455),
    ('k', 1024),
    ('l', 455),
    ('m', 1706),
    ('n', 1139),
    ('o', 1139),
    ('p', 1139),
    ('q', 1139),
    ('r', 682),
    ('s', 1024),
    ('t', 569),
    ('u', 1139),
    ('v', 1024),
    ('w', 1479),
    ('x', 1024),
    ('y', 1024),
    ('z', 1024),
    ('{', 684),
    ('|', 532),
    ('}', 684),
    ('~', 1196),
    ('\u{00a0}', 569),
    ('\u{00a1}', 682),
    ('\u{00a2}', 1139),
    ('\u{00a3}', 1139),
    ('\u{00a4}', 1139),
    ('\u{00a5}', 1139),
    ('\u{00a6}', 532),
    ('\u{00a7}', 1139),
    ('\u{00a8}', 682),
    ('\u{00a9}', 1509),
    ('\u{00aa}', 758),
    ('\u{00ab}', 1139),
    ('\u{00ac}', 1196),
    ('\u{00ad}', 682),
    ('\u{00ae}', 1509),
    ('\u{00af}', 1131),
    ('\u{00b0}', 819),
    ('\u{00b1}', 1124),
    ('\u{00b2}', 682),
    ('\u{00b3}', 682),
    ('\u{00b4}', 682),
    ('\u{00b5}', 1180),
    ('\u{00b6}', 1100),
    ('\u{00b7}', 682),
    ('\u{00b8}', 682),
    ('\u{00b9}', 682),
    ('\u{00ba}', 748),
    ('\u{00bb}', 1139),
    ('\u{00bc}', 1708),
    ('\u{00bd}', 1708),
    ('\u{00be}', 1708),
    ('\u{00bf}', 1251),
    ('\u{00c0}', 1366),
    ('\u{00c1}', 1366),
    ('\u{00c2}', 1366),
    ('\u{00c3}', 1366),
    ('\u{00c4}', 1366),
    ('\u{00c5}', 1366),
    ('\u{00c6}', 2048),
    ('\u{00c7}', 1479),
    ('\u{00c8}', 1366),
    ('\u{00c9}', 1366),
    ('\u{00ca}', 1366),
    ('\u{00cb}', 1366),
    ('\u{00cc}', 569),
    ('\u{00cd}', 569),
    ('\u{00ce}', 569),
    ('\u{00cf}', 569),
    ('\u{00d0}', 1479),
    ('\u{00d1}', 1479),
    ('\u{00d2}', 1593),
    ('\u{00d3}', 1593),
    ('\u{00d4}', 1593),
    ('\u{00d5}', 1593),
    ('\u{00d6}', 1593),
    ('\u{00d7}', 1196),
    ('\u{00d8}', 1593),
    ('\u{00d9}', 1479),
    ('\u{00da}', 1479),
    ('\u{00db}', 1479),
    ('\u{00dc}', 1479),
    ('\u{00dd}', 1366),
    ('\u{00de}', 1366),
    ('\u{00df}', 1251),
    ('\u{00e0}', 1139),
    ('\u{00e1}', 1139),
    ('\u{00e2}', 1139),
    ('\u{00e3}', 1139),
    ('\u{00e4}', 1139),
    ('\u{00e5}', 1139),
    ('\u{00e6}', 1821),
    ('\u{00e7}', 1024),
    ('\u{00e8}', 1139),
    ('\u{00e9}', 1139),
    ('\u{00ea}', 1139),
    ('\u{00eb}', 1139),
    ('\u{00ec}', 569),
    ('\u{00ed}', 569),
    ('\u{00ee}', 569),
    ('\u{00ef}', 569),
    ('\u{00f0}', 1139),
    ('\u{00f1}', 1139),
    ('\u{00f2}', 1139),
    ('\u{00f3}', 1139),
    ('\u{00f4}', 1139),
    ('\u{00f5}', 1139),
    ('\u{00f6}', 1139),
    ('\u{00f7}', 1124),
    ('\u{00f8}', 1251),
    ('\u{00f9}', 1139),
    ('\u{00fa}', 1139),
    ('\u{00fb}', 1139),
    ('\u{00fc}', 1139),
    ('\u{00fd}', 1024),
    ('\u{00fe}', 1139),
    ('\u{00ff}', 1024),
];

fn build_arimo() -> FontMetrics {
    FontMetrics {
        units_per_em: 2048,
        ascender: 1854,
        descender: -434,
        default_width: 1114,
        cjk_width: 2048,
        fallback_name: "Arimo",
        widths: ARIMO_WIDTHS,
    }
}

// ---------------------------------------------------------------------------
// Tinos (Times New Roman-compatible).
//
// Same arrangement as Arimo: width data is from Liberation Serif,
// metric-compatible with Tinos. Mapping.rs sends `Times` /
// `Times New Roman` to `Tinos`.
// ---------------------------------------------------------------------------

// Sorted by char codepoint — required by binary_search_by_key in width_of().
#[allow(clippy::too_many_lines)]
static TINOS_WIDTHS: &[(char, u16)] = &[
    (' ', 512),
    ('!', 682),
    ('"', 836),
    ('#', 1024),
    ('$', 1024),
    ('%', 1706),
    ('&', 1593),
    ('\'', 369),
    ('(', 682),
    (')', 682),
    ('*', 1024),
    ('+', 1155),
    (',', 512),
    ('-', 682),
    ('.', 512),
    ('/', 569),
    ('0', 1024),
    ('1', 1024),
    ('2', 1024),
    ('3', 1024),
    ('4', 1024),
    ('5', 1024),
    ('6', 1024),
    ('7', 1024),
    ('8', 1024),
    ('9', 1024),
    (':', 569),
    (';', 569),
    ('<', 1155),
    ('=', 1155),
    ('>', 1155),
    ('?', 909),
    ('@', 1886),
    ('A', 1479),
    ('B', 1366),
    ('C', 1366),
    ('D', 1479),
    ('E', 1251),
    ('F', 1139),
    ('G', 1479),
    ('H', 1479),
    ('I', 682),
    ('J', 797),
    ('K', 1479),
    ('L', 1251),
    ('M', 1821),
    ('N', 1479),
    ('O', 1479),
    ('P', 1139),
    ('Q', 1479),
    ('R', 1366),
    ('S', 1139),
    ('T', 1251),
    ('U', 1479),
    ('V', 1479),
    ('W', 1933),
    ('X', 1479),
    ('Y', 1479),
    ('Z', 1251),
    ('[', 682),
    ('\\', 569),
    (']', 682),
    ('^', 961),
    ('_', 1024),
    ('`', 682),
    ('a', 909),
    ('b', 1024),
    ('c', 909),
    ('d', 1024),
    ('e', 909),
    ('f', 682),
    ('g', 1024),
    ('h', 1024),
    ('i', 569),
    ('j', 569),
    ('k', 1024),
    ('l', 569),
    ('m', 1593),
    ('n', 1024),
    ('o', 1024),
    ('p', 1024),
    ('q', 1024),
    ('r', 682),
    ('s', 797),
    ('t', 569),
    ('u', 1024),
    ('v', 1024),
    ('w', 1479),
    ('x', 1024),
    ('y', 1024),
    ('z', 909),
    ('{', 983),
    ('|', 410),
    ('}', 983),
    ('~', 1108),
    ('\u{00a0}', 512),
    ('\u{00a1}', 682),
    ('\u{00a2}', 1024),
    ('\u{00a3}', 1024),
    ('\u{00a4}', 1024),
    ('\u{00a5}', 1024),
    ('\u{00a6}', 410),
    ('\u{00a7}', 1024),
    ('\u{00a8}', 682),
    ('\u{00a9}', 1556),
    ('\u{00aa}', 565),
    ('\u{00ab}', 1024),
    ('\u{00ac}', 1155),
    ('\u{00ad}', 682),
    ('\u{00ae}', 1556),
    ('\u{00af}', 1024),
    ('\u{00b0}', 819),
    ('\u{00b1}', 1124),
    ('\u{00b2}', 614),
    ('\u{00b3}', 614),
    ('\u{00b4}', 682),
    ('\u{00b5}', 1180),
    ('\u{00b6}', 928),
    ('\u{00b7}', 682),
    ('\u{00b8}', 682),
    ('\u{00b9}', 614),
    ('\u{00ba}', 635),
    ('\u{00bb}', 1024),
    ('\u{00bc}', 1536),
    ('\u{00bd}', 1536),
    ('\u{00be}', 1536),
    ('\u{00bf}', 909),
    ('\u{00c0}', 1479),
    ('\u{00c1}', 1479),
    ('\u{00c2}', 1479),
    ('\u{00c3}', 1479),
    ('\u{00c4}', 1479),
    ('\u{00c5}', 1479),
    ('\u{00c6}', 1821),
    ('\u{00c7}', 1366),
    ('\u{00c8}', 1251),
    ('\u{00c9}', 1251),
    ('\u{00ca}', 1251),
    ('\u{00cb}', 1251),
    ('\u{00cc}', 682),
    ('\u{00cd}', 682),
    ('\u{00ce}', 682),
    ('\u{00cf}', 682),
    ('\u{00d0}', 1479),
    ('\u{00d1}', 1479),
    ('\u{00d2}', 1479),
    ('\u{00d3}', 1479),
    ('\u{00d4}', 1479),
    ('\u{00d5}', 1479),
    ('\u{00d6}', 1479),
    ('\u{00d7}', 1155),
    ('\u{00d8}', 1479),
    ('\u{00d9}', 1479),
    ('\u{00da}', 1479),
    ('\u{00db}', 1479),
    ('\u{00dc}', 1479),
    ('\u{00dd}', 1479),
    ('\u{00de}', 1139),
    ('\u{00df}', 1024),
    ('\u{00e0}', 909),
    ('\u{00e1}', 909),
    ('\u{00e2}', 909),
    ('\u{00e3}', 909),
    ('\u{00e4}', 909),
    ('\u{00e5}', 909),
    ('\u{00e6}', 1366),
    ('\u{00e7}', 909),
    ('\u{00e8}', 909),
    ('\u{00e9}', 909),
    ('\u{00ea}', 909),
    ('\u{00eb}', 909),
    ('\u{00ec}', 569),
    ('\u{00ed}', 569),
    ('\u{00ee}', 569),
    ('\u{00ef}', 569),
    ('\u{00f0}', 1024),
    ('\u{00f1}', 1024),
    ('\u{00f2}', 1024),
    ('\u{00f3}', 1024),
    ('\u{00f4}', 1024),
    ('\u{00f5}', 1024),
    ('\u{00f6}', 1024),
    ('\u{00f7}', 1124),
    ('\u{00f8}', 1024),
    ('\u{00f9}', 1024),
    ('\u{00fa}', 1024),
    ('\u{00fb}', 1024),
    ('\u{00fc}', 1024),
    ('\u{00fd}', 1024),
    ('\u{00fe}', 1024),
    ('\u{00ff}', 1024),
];

fn build_tinos() -> FontMetrics {
    FontMetrics {
        units_per_em: 2048,
        ascender: 1825,
        descender: -443,
        default_width: 1056,
        cjk_width: 2048,
        fallback_name: "Tinos",
        widths: TINOS_WIDTHS,
    }
}

// ---------------------------------------------------------------------------
// Cousine (Courier-compatible, monospace).
//
// Every glyph in a monospace face shares the same advance, so a single
// `default_width` is exact for ASCII / Latin-1 — no per-character map
// is needed. CJK glyphs in mixed-script runs measure via `cjk_width`.
// Scalar values match Liberation Mono's hhea (Cousine and Liberation
// Mono are independently engineered metric clones of Courier New).
// ---------------------------------------------------------------------------

fn build_cousine() -> FontMetrics {
    FontMetrics {
        units_per_em: 2048,
        ascender: 1705,
        descender: -615,
        // Courier New / Cousine ASCII advance = 1229 in font units
        // (= 600/1000 of the em square in PostScript-style metrics).
        default_width: 1229,
        cjk_width: 2048,
        fallback_name: "Cousine",
        widths: &[],
    }
}

// ---------------------------------------------------------------------------
// Caladea (Cambria-compatible).
//
// Caladea is a Lato-family serif released as a Cambria metric clone
// for LibreOffice. We ship scalar metrics matching the upstream hhea
// table; per-character widths land alongside the offline-extraction
// pipeline upgrade. Until then, `default_width` uses Caladea's average
// Latin advance and CJK glyphs measure via `cjk_width`.
// ---------------------------------------------------------------------------

fn build_caladea() -> FontMetrics {
    FontMetrics {
        units_per_em: 2048,
        ascender: 1900,
        descender: -481,
        // Average advance of A-Z + a-z in Caladea (~520 / 1000 em).
        default_width: 1066,
        cjk_width: 2048,
        fallback_name: "Caladea",
        widths: &[],
    }
}

// ---------------------------------------------------------------------------
// Noto Sans JP
// ---------------------------------------------------------------------------

// Sorted by char codepoint — required by binary_search_by_key in width_of().
#[allow(clippy::too_many_lines)]
static NOTO_SANS_JP_WIDTHS: &[(char, u16)] = &[
    (' ', 224),
    ('!', 323),
    ('"', 474),
    ('#', 555),
    ('$', 555),
    ('%', 921),
    ('&', 680),
    ('\'', 278),
    ('(', 338),
    (')', 338),
    ('*', 467),
    ('+', 555),
    (',', 278),
    ('-', 347),
    ('.', 278),
    ('/', 392),
    ('0', 555),
    ('1', 555),
    ('2', 555),
    ('3', 555),
    ('4', 555),
    ('5', 555),
    ('6', 555),
    ('7', 555),
    ('8', 555),
    ('9', 555),
    (':', 278),
    (';', 278),
    ('<', 555),
    ('=', 555),
    ('>', 555),
    ('?', 474),
    ('@', 946),
    ('A', 608),
    ('B', 657),
    ('C', 638),
    ('D', 688),
    ('E', 589),
    ('F', 552),
    ('G', 689),
    ('H', 728),
    ('I', 293),
    ('J', 535),
    ('K', 646),
    ('L', 543),
    ('M', 812),
    ('N', 723),
    ('O', 742),
    ('P', 633),
    ('Q', 742),
    ('R', 635),
    ('S', 596),
    ('T', 599),
    ('U', 721),
    ('V', 575),
    ('W', 878),
    ('X', 573),
    ('Y', 531),
    ('Z', 603),
    ('[', 338),
    ('\\', 392),
    (']', 338),
    ('^', 555),
    ('_', 559),
    ('`', 606),
    ('a', 563),
    ('b', 618),
    ('c', 510),
    ('d', 620),
    ('e', 554),
    ('f', 325),
    ('g', 564),
    ('h', 607),
    ('i', 275),
    ('j', 275),
    ('k', 552),
    ('l', 284),
    ('m', 926),
    ('n', 610),
    ('o', 606),
    ('p', 620),
    ('q', 620),
    ('r', 388),
    ('s', 468),
    ('t', 377),
    ('u', 607),
    ('v', 521),
    ('w', 802),
    ('x', 498),
    ('y', 521),
    ('z', 475),
    ('{', 338),
    ('|', 270),
    ('}', 338),
    ('~', 555),
    ('\u{00a0}', 224),
    ('\u{00a1}', 323),
    ('\u{00a2}', 555),
    ('\u{00a3}', 555),
    ('\u{00a4}', 555),
    ('\u{00a5}', 555),
    ('\u{00a6}', 270),
    ('\u{00a7}', 1000),
    ('\u{00a8}', 606),
    ('\u{00a9}', 832),
    ('\u{00aa}', 386),
    ('\u{00ab}', 479),
    ('\u{00ac}', 555),
    ('\u{00ad}', 347),
    ('\u{00ae}', 473),
    ('\u{00af}', 606),
    ('\u{00b0}', 370),
    ('\u{00b1}', 1000),
    ('\u{00b2}', 411),
    ('\u{00b3}', 411),
    ('\u{00b4}', 606),
    ('\u{00b5}', 628),
    ('\u{00b6}', 1000),
    ('\u{00b7}', 561),
    ('\u{00b8}', 606),
    ('\u{00b9}', 411),
    ('\u{00ba}', 407),
    ('\u{00bb}', 479),
    ('\u{00bc}', 873),
    ('\u{00bd}', 903),
    ('\u{00be}', 889),
    ('\u{00bf}', 474),
    ('\u{00c0}', 608),
    ('\u{00c1}', 608),
    ('\u{00c2}', 608),
    ('\u{00c3}', 608),
    ('\u{00c4}', 608),
    ('\u{00c5}', 608),
    ('\u{00c6}', 918),
    ('\u{00c7}', 638),
    ('\u{00c8}', 589),
    ('\u{00c9}', 589),
    ('\u{00ca}', 589),
    ('\u{00cb}', 589),
    ('\u{00cc}', 293),
    ('\u{00cd}', 293),
    ('\u{00ce}', 293),
    ('\u{00cf}', 293),
    ('\u{00d0}', 712),
    ('\u{00d1}', 723),
    ('\u{00d2}', 742),
    ('\u{00d3}', 742),
    ('\u{00d4}', 742),
    ('\u{00d5}', 742),
    ('\u{00d6}', 742),
    ('\u{00d7}', 1000),
    ('\u{00d8}', 742),
    ('\u{00d9}', 721),
    ('\u{00da}', 721),
    ('\u{00db}', 721),
    ('\u{00dc}', 721),
    ('\u{00dd}', 531),
    ('\u{00de}', 652),
    ('\u{00df}', 643),
    ('\u{00e0}', 563),
    ('\u{00e1}', 563),
    ('\u{00e2}', 563),
    ('\u{00e3}', 563),
    ('\u{00e4}', 563),
    ('\u{00e5}', 563),
    ('\u{00e6}', 877),
    ('\u{00e7}', 510),
    ('\u{00e8}', 554),
    ('\u{00e9}', 554),
    ('\u{00ea}', 554),
    ('\u{00eb}', 554),
    ('\u{00ec}', 275),
    ('\u{00ed}', 275),
    ('\u{00ee}', 275),
    ('\u{00ef}', 275),
    ('\u{00f0}', 608),
    ('\u{00f1}', 610),
    ('\u{00f2}', 606),
    ('\u{00f3}', 606),
    ('\u{00f4}', 606),
    ('\u{00f5}', 606),
    ('\u{00f6}', 606),
    ('\u{00f7}', 1000),
    ('\u{00f8}', 606),
    ('\u{00f9}', 607),
    ('\u{00fa}', 607),
    ('\u{00fb}', 607),
    ('\u{00fc}', 607),
    ('\u{00fd}', 521),
    ('\u{00fe}', 620),
    ('\u{00ff}', 521),
];

fn build_noto_sans_jp() -> FontMetrics {
    FontMetrics {
        units_per_em: 1000,
        ascender: 1160,
        descender: -288,
        default_width: 563,
        cjk_width: 1000,
        fallback_name: "Noto Sans JP",
        widths: NOTO_SANS_JP_WIDTHS,
    }
}

// ---------------------------------------------------------------------------
// Noto CJK approximations (KR / SC / TC, sans + serif, plus Japanese
// serif).
//
// Scalar values mirror Adobe Source Han / Google Noto CJK family
// defaults — every Noto CJK face shipped by Google uses
// unitsPerEm=1000, ascender=1160, descender=-288. CJK glyphs are full-
// width (advance = unitsPerEm = 1000); Latin glyphs in a CJK face
// average ~563 advance (matches Noto Sans JP's measured default).
//
// The widths HashMap is intentionally empty: per-character data
// requires the offline extraction pipeline. Until then,
// `width_of(c)` returns None for every char and the measurer falls
// back to `cjk_width` (correct for full-width CJK) or `default_width`
// (representative for Latin in CJK faces).
// ---------------------------------------------------------------------------

fn build_noto_cjk_approx(fallback_name: &'static str) -> FontMetrics {
    FontMetrics {
        units_per_em: 1000,
        ascender: 1160,
        descender: -288,
        default_width: 563,
        cjk_width: 1000,
        fallback_name,
        widths: &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- get_font_metrics: PPTX names route through mapping ------------------

    #[test]
    fn calibri_resolves_to_carlito_metrics() {
        let m = get_font_metrics("Calibri").expect("Calibri must resolve");
        assert_eq!(m.units_per_em, 2048);
        assert_eq!(m.width_of('A'), Some(1185));
        assert_eq!(m.fallback_name, "Carlito");
    }

    #[test]
    fn arial_resolves_to_arimo_metrics() {
        let m = get_font_metrics("Arial").expect("Arial must resolve");
        assert_eq!(m.units_per_em, 2048);
        assert_eq!(m.width_of('A'), Some(1366));
        assert_eq!(m.fallback_name, "Arimo");
    }

    #[test]
    fn helvetica_resolves_to_arimo_metrics() {
        let m = get_font_metrics("Helvetica").expect("Helvetica must resolve");
        assert_eq!(m.width_of('A'), Some(1366));
        assert_eq!(m.fallback_name, "Arimo");
    }

    #[test]
    fn times_new_roman_resolves_to_tinos_metrics() {
        let m = get_font_metrics("Times New Roman").expect("must resolve");
        assert_eq!(m.width_of('A'), Some(1479));
        assert_eq!(m.fallback_name, "Tinos");
    }

    #[test]
    fn meiryo_resolves_to_noto_sans_jp() {
        let m = get_font_metrics("Meiryo").expect("Meiryo must resolve");
        assert_eq!(m.units_per_em, 1000);
        assert_eq!(m.cjk_width, 1000);
        assert_eq!(m.fallback_name, "Noto Sans JP");
    }

    #[test]
    fn yu_gothic_resolves_to_noto_sans_jp() {
        let m = get_font_metrics("Yu Gothic").expect("Yu Gothic must resolve");
        assert_eq!(m.units_per_em, 1000);
        assert_eq!(m.fallback_name, "Noto Sans JP");
    }

    #[test]
    fn yu_mincho_resolves_to_noto_serif_jp() {
        let m = get_font_metrics("Yu Mincho").expect("Yu Mincho must resolve");
        assert_eq!(m.fallback_name, "Noto Serif JP");
        assert_eq!(m.cjk_width, 1000);
    }

    // -- CJK Script Equality: KR / SC / TC sans+serif all resolve ------------

    #[test]
    fn malgun_gothic_resolves_to_noto_sans_kr() {
        let m = get_font_metrics("Malgun Gothic").expect("Malgun Gothic must resolve");
        assert_eq!(m.fallback_name, "Noto Sans KR");
        assert_eq!(m.cjk_width, 1000);
    }

    #[test]
    fn batang_resolves_to_noto_serif_kr() {
        let m = get_font_metrics("Batang").expect("Batang must resolve");
        assert_eq!(m.fallback_name, "Noto Serif KR");
    }

    #[test]
    fn microsoft_yahei_resolves_to_noto_sans_sc() {
        let m = get_font_metrics("Microsoft YaHei").expect("Microsoft YaHei must resolve");
        assert_eq!(m.fallback_name, "Noto Sans SC");
        assert_eq!(m.cjk_width, 1000);
    }

    #[test]
    fn simsun_resolves_to_noto_serif_sc() {
        let m = get_font_metrics("SimSun").expect("SimSun must resolve");
        assert_eq!(m.fallback_name, "Noto Serif SC");
    }

    #[test]
    fn microsoft_jhenghei_resolves_to_noto_sans_tc() {
        let m = get_font_metrics("Microsoft JhengHei").expect("Microsoft JhengHei must resolve");
        assert_eq!(m.fallback_name, "Noto Sans TC");
    }

    #[test]
    fn pmingliu_resolves_to_noto_serif_tc() {
        let m = get_font_metrics("PMingLiU").expect("PMingLiU must resolve");
        assert_eq!(m.fallback_name, "Noto Serif TC");
    }

    #[test]
    fn cjk_script_equality_all_eight_cells_resolve() {
        // Every (script × style) cell of mapping.rs has a metric
        // entry. Sample one representative key per cell.
        let cells: &[(&str, &str)] = &[
            ("Yu Gothic", "Noto Sans JP"),
            ("Yu Mincho", "Noto Serif JP"),
            ("Malgun Gothic", "Noto Sans KR"),
            ("Batang", "Noto Serif KR"),
            ("Microsoft YaHei", "Noto Sans SC"),
            ("SimSun", "Noto Serif SC"),
            ("Microsoft JhengHei", "Noto Sans TC"),
            ("PMingLiU", "Noto Serif TC"),
        ];
        for (key, expected_target) in cells {
            let m = get_font_metrics(key)
                .unwrap_or_else(|| panic!("{key} must resolve to {expected_target}"));
            assert_eq!(m.fallback_name, *expected_target);
        }
    }

    // -- Direct OSS-name lookup (skipping the mapping) -----------------------

    #[test]
    fn direct_oss_name_lookup() {
        // Callers that already resolved through mapping.rs can pass
        // the OSS family name directly.
        for oss in [
            "Carlito",
            "Arimo",
            "Tinos",
            "Cousine",
            "Caladea",
            "Noto Sans JP",
            "Noto Serif JP",
            "Noto Sans KR",
            "Noto Serif KR",
            "Noto Sans SC",
            "Noto Serif SC",
            "Noto Sans TC",
            "Noto Serif TC",
        ] {
            let m = get_font_metrics(oss).unwrap_or_else(|| panic!("OSS name {oss}"));
            assert_eq!(m.fallback_name, oss);
        }
    }

    // -- Lookup edge cases ---------------------------------------------------

    #[test]
    fn case_insensitive_for_latin() {
        // mapping.rs handles case folding for Latin entries.
        let upper = get_font_metrics("Calibri").expect("Calibri");
        let lower = get_font_metrics("calibri").expect("calibri");
        assert_eq!(upper.width_of('A'), lower.width_of('A'));
    }

    #[test]
    fn unknown_font_returns_none() {
        assert!(get_font_metrics("NonExistentFont").is_none());
    }

    #[test]
    fn empty_returns_none() {
        assert!(get_font_metrics("").is_none());
    }

    #[test]
    fn courier_resolves_to_cousine_metrics() {
        let m = get_font_metrics("Courier New").expect("Courier New must resolve");
        assert_eq!(m.fallback_name, "Cousine");
        assert_eq!(m.units_per_em, 2048);
        assert_eq!(m.cjk_width, 2048);
    }

    #[test]
    fn cambria_resolves_to_caladea_metrics() {
        let m = get_font_metrics("Cambria").expect("Cambria must resolve");
        assert_eq!(m.fallback_name, "Caladea");
        assert_eq!(m.units_per_em, 2048);
    }

    #[test]
    fn cousine_caladea_have_empty_widths_table() {
        // Monospace + scalar-only entries: per-character maps stay
        // empty; ASCII glyphs measure via default_width and CJK via
        // cjk_width.
        for name in ["Cousine", "Caladea"] {
            let m = get_font_metrics(name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(m.widths_len(), 0, "{name} should have empty widths table");
        }
    }

    #[test]
    fn ascender_positive_descender_negative_each_font() {
        for name in [
            "Calibri",
            "Arial",
            "Times New Roman",
            "Meiryo",
            "Yu Mincho",
            "Malgun Gothic",
            "Batang",
            "Microsoft YaHei",
            "SimSun",
            "Microsoft JhengHei",
            "PMingLiU",
        ] {
            let m = get_font_metrics(name).unwrap_or_else(|| panic!("{name}"));
            assert!(m.ascender > 0, "{name} ascender");
            assert!(m.descender < 0, "{name} descender");
        }
    }

    // -- get_metrics_fallback_font (matches mapping.rs target) --------------

    #[test]
    fn fallback_font_matches_mapping_target() {
        // Invariant: get_metrics_fallback_font(name) ==
        // get_mapped_font(name, default_mapping) for every name with
        // both a mapping and a metric entry.
        use crate::mapping::{default_font_mapping, get_mapped_font};
        let mapping = default_font_mapping();
        for name in [
            "Calibri",
            "Arial",
            "Helvetica",
            "Times New Roman",
            "Meiryo",
            "Yu Gothic",
            "Yu Mincho",
            "Malgun Gothic",
            "맑은 고딕",
            "Batang",
            "Microsoft YaHei",
            "SimSun",
            "Microsoft JhengHei",
            "PMingLiU",
        ] {
            let metric_name =
                get_metrics_fallback_font(name).unwrap_or_else(|| panic!("metrics for {name}"));
            let mapping_target =
                get_mapped_font(name, &mapping).unwrap_or_else(|| panic!("mapping for {name}"));
            assert_eq!(
                metric_name, mapping_target,
                "fallback_name / mapping target diverged for {name}"
            );
        }
    }

    #[test]
    fn fallback_font_calibri_to_carlito() {
        assert_eq!(get_metrics_fallback_font("Calibri"), Some("Carlito"));
    }

    #[test]
    fn fallback_font_arial_to_arimo() {
        assert_eq!(get_metrics_fallback_font("Arial"), Some("Arimo"));
    }

    #[test]
    fn fallback_font_times_to_tinos() {
        assert_eq!(get_metrics_fallback_font("Times New Roman"), Some("Tinos"));
    }

    #[test]
    fn fallback_font_japanese_to_noto_sans_jp() {
        assert_eq!(get_metrics_fallback_font("Meiryo"), Some("Noto Sans JP"));
        assert_eq!(get_metrics_fallback_font("Yu Gothic"), Some("Noto Sans JP"));
        assert_eq!(
            get_metrics_fallback_font("Noto Sans JP"),
            Some("Noto Sans JP")
        );
    }

    #[test]
    fn fallback_font_case_insensitive_latin() {
        assert_eq!(get_metrics_fallback_font("calibri"), Some("Carlito"));
    }

    #[test]
    fn fallback_font_unknown_none() {
        assert_eq!(get_metrics_fallback_font("NonExistentFont"), None);
    }

    #[test]
    fn fallback_font_empty_none() {
        assert_eq!(get_metrics_fallback_font(""), None);
    }

    // -- Per-font soft-hyphen distinction (sanity check on width data) ------

    #[test]
    fn carlito_has_no_soft_hyphen_entry() {
        let m = get_font_metrics("Calibri").unwrap();
        assert!(m.width_of('\u{00ad}').is_none());
    }

    #[test]
    fn arimo_has_soft_hyphen_682() {
        // Width data from Liberation Sans, which Arimo is metric-
        // compatible with.
        let m = get_font_metrics("Arial").unwrap();
        assert_eq!(m.width_of('\u{00ad}'), Some(682));
    }

    #[test]
    fn noto_sans_jp_soft_hyphen_347() {
        let m = get_font_metrics("Meiryo").unwrap();
        assert_eq!(m.width_of('\u{00ad}'), Some(347));
    }

    // -- CJK approximation entries have empty per-character tables ----------

    #[test]
    fn noto_kr_sc_tc_have_empty_widths_table() {
        // Approximated entries: per-character widths are intentionally
        // empty; CJK glyphs measure via cjk_width=1000, Latin glyphs
        // via default_width=563.
        for name in [
            "Noto Sans KR",
            "Noto Serif KR",
            "Noto Sans SC",
            "Noto Serif SC",
            "Noto Sans TC",
            "Noto Serif TC",
            "Noto Serif JP",
        ] {
            let m = get_font_metrics(name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(m.widths_len(), 0, "{name} should have empty widths table");
            assert_eq!(m.cjk_width, 1000);
            assert_eq!(m.default_width, 563);
        }
    }

    // -- T14 (IP-4): widths must be a sorted slice, not HashMap ---------------

    #[test]
    fn widths_slice_is_sorted_by_char_carlito() {
        // binary_search_by_key requires the slice to be sorted by key.
        let m = get_font_metrics("Calibri").expect("Calibri");
        let slice = m.widths_slice();
        for w in slice.windows(2) {
            assert!(
                w[0].0 <= w[1].0,
                "Carlito widths not sorted: {:?} > {:?}",
                w[0].0,
                w[1].0
            );
        }
    }

    #[test]
    fn widths_slice_is_sorted_by_char_arimo() {
        let m = get_font_metrics("Arial").expect("Arial");
        let slice = m.widths_slice();
        for w in slice.windows(2) {
            assert!(
                w[0].0 <= w[1].0,
                "Arimo widths not sorted: {:?} > {:?}",
                w[0].0,
                w[1].0
            );
        }
    }

    #[test]
    fn widths_slice_is_sorted_by_char_tinos() {
        let m = get_font_metrics("Times New Roman").expect("Times New Roman");
        let slice = m.widths_slice();
        for w in slice.windows(2) {
            assert!(
                w[0].0 <= w[1].0,
                "Tinos widths not sorted: {:?} > {:?}",
                w[0].0,
                w[1].0
            );
        }
    }

    #[test]
    fn widths_slice_is_sorted_by_char_noto_sans_jp() {
        let m = get_font_metrics("Noto Sans JP").expect("Noto Sans JP");
        let slice = m.widths_slice();
        for w in slice.windows(2) {
            assert!(
                w[0].0 <= w[1].0,
                "Noto Sans JP widths not sorted: {:?} > {:?}",
                w[0].0,
                w[1].0
            );
        }
    }

    #[test]
    fn width_of_uses_binary_search() {
        // Verify that width_of returns correct results for known entries
        // and None for entries not in the table — same semantics as
        // the old HashMap.get, but now via binary_search_by_key.
        let m = get_font_metrics("Calibri").expect("Calibri");
        assert_eq!(m.width_of('A'), Some(1185));
        assert_eq!(m.width_of('\u{00ff}'), Some(927));
        // Character beyond Latin-1 range — not in Carlito table.
        assert_eq!(m.width_of('\u{0400}'), None);
    }
}
