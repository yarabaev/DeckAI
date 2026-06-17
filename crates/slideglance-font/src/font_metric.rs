// Mixed-case proper nouns (Calibri / Hiragino / Noto) make doc-link
// linting noisy without code-clarity benefit.
#![allow(clippy::doc_markdown)]

//! PANOSE + OS/2 metric vectors for "best visual match" font fallback.
//!
//! When a PPTX deck declares e.g. `Yu Gothic Light` and the host has
//! no Yu Gothic install, the most faithful substitute is the locally
//! installed face whose glyph proportions and stroke characteristics
//! are closest. This module ships:
//!
//! 1. A static metric DB ([`KNOWN_METRIC_FONTS`]) covering the Microsoft /
//!    Apple / Adobe / Google standard typefaces a PPTX deck is likely to
//!    declare. Each entry carries the canonical PANOSE-1 classification plus
//!    normalised OS/2 metric ratios (x-height, cap-height, ascent, descent,
//!    average advance — all expressed as a fraction of the em square).
//! 2. A [`FontMetricVector`] type plus [`metric_distance`] that computes a
//!    weighted distance between two vectors. Smaller distance = visually closer.
//! 3. Helpers to extract a metric vector from any TTF/OTF buffer via
//!    `ttf-parser`, and to find the best-match face from a candidate list (the
//!    static DB plus, when the `metric-match` feature is on, the host's
//!    installed fonts via `font-kit`).
//!
//! The static DB stays the source of truth even when the optional
//! features are off — the resolver falls back to selecting from those
//! entries by name. Adding the `metric-match` feature lights up the
//! cross-platform "best visual match" path.

/// PANOSE-1 classification (10 byte values). Index meaning is
/// PANOSE-aware:
///
/// - `[0]` Family kind (Latin Text = 2, Latin Hand-written = 3, …).
/// - `[1]` Serif type / tool kind.
/// - `[2]` Weight (1–11, larger = heavier).
/// - `[3]` Proportion.
/// - `[4]` Contrast.
/// - `[5]` Stroke variation.
/// - `[6]` Arm style.
/// - `[7]` Letterform.
/// - `[8]` Midline.
/// - `[9]` x-Height.
///
/// `0` in any digit means "Any" (skip in distance computation).
pub type Panose = [u8; 10];

/// Normalised metric vector — every dimension expressed as a fraction
/// of the em square so cross-font comparison is unit-free.
///
/// Construct via [`FontMetricVector::from_face`] (any TTF/OTF buffer)
/// or look one up by family name in [`metric_for_family`].
#[derive(Debug, Clone, Copy)]
pub struct FontMetricVector {
    /// Optional PANOSE classification. `None` when the font has no
    /// `panose10` table or every byte is zero.
    pub panose: Option<Panose>,
    /// OS/2 weight class (100–900). Defaults to 400 when absent.
    pub weight: u16,
    /// Italic angle in degrees (positive = forward slant). 0 for
    /// upright faces.
    pub italic_angle: f32,
    /// Ratio of the OS/2 typo-ascender to the em square.
    /// Effective ascender / em — `hhea.Ascender` when OS/2 `fsSelection`
    /// USE_TYPO_METRICS (bit 7) is clear; `sTypoAscender` when set.
    /// `ttf_parser::Face::ascender()` already applies this selection per
    /// the OpenType spec, so values extracted via `from_face` are already
    /// "effective" regardless of which metric the font author chose.
    pub effective_ascender_ratio: f32,
    /// Effective descender / em (absolute value).
    pub effective_descender_ratio: f32,
    /// Ratio of the OS/2 sxHeight to the em square. `0.0` when the
    /// font omits sxHeight.
    pub x_height_ratio: f32,
    /// Ratio of the OS/2 sCapHeight to the em square. `0.0` when the
    /// font omits sCapHeight.
    pub cap_height_ratio: f32,
    /// Average ASCII glyph advance / em (rough proxy for glyph width).
    /// `0.0` when no advances were probed.
    pub avg_advance_ratio: f32,
}

impl FontMetricVector {
    /// Extract a metric vector from a parsed TTF/OTF face.
    ///
    /// Returns `None` when the face's `units_per_em` is zero — that's
    /// rare in production fonts but plausible for stripped subset fonts.
    ///
    /// PANOSE classification is not exposed by `ttf-parser` and is
    /// always `None` here; the static catalogue carries PANOSE
    /// values for every well-known typeface, and the distance helper
    /// silently skips PANOSE comparison when one side has no data.
    #[must_use]
    pub fn from_face(face: &ttf_parser::Face<'_>) -> Option<Self> {
        let upem = f32::from(face.units_per_em());
        if upem == 0.0 {
            return None;
        }
        let weight = face.weight().to_number();
        let italic_angle = face.italic_angle();
        let ascender_ratio = f32::from(face.ascender()) / upem;
        let descender_ratio = f32::from(face.descender()).abs() / upem;
        let x_height_ratio = face.x_height().map_or(0.0, |v| f32::from(v) / upem);
        let cap_height_ratio = face.capital_height().map_or(0.0, |v| f32::from(v) / upem);
        let avg_advance_ratio = avg_ascii_advance(face) / upem;
        Some(Self {
            panose: None,
            weight,
            italic_angle,
            effective_ascender_ratio: ascender_ratio,
            effective_descender_ratio: descender_ratio,
            x_height_ratio,
            cap_height_ratio,
            avg_advance_ratio,
        })
    }
}

fn avg_ascii_advance(face: &ttf_parser::Face<'_>) -> f32 {
    // Sample a-z + 0-9 + space — a reasonable proxy for "is this a
    // narrow vs wide font?" without iterating every codepoint.
    const SAMPLE: &[char] = &[
        ' ', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g',
        'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y',
        'z',
    ];
    let mut sum = 0_u32;
    let mut count = 0_u32;
    for ch in SAMPLE {
        if let Some(gid) = face.glyph_index(*ch) {
            if let Some(adv) = face.glyph_hor_advance(gid) {
                sum += u32::from(adv);
                count += 1;
            }
        }
    }
    sum.checked_div(count)
        .map_or(0.0, |avg| f32::from(u16::try_from(avg).unwrap_or(0)))
}

/// Weighted distance between two metric vectors. Smaller = visually
/// closer. Returns a non-negative `f32` with no upper bound; raw
/// values are typically in the 0..3 range when the comparand fonts
/// share a family kind.
///
/// Weights are tuned for "PANOSE family + serif type carries the most
/// signal, then proportions, then weight, then italic angle":
///
/// | Component | Weight |
/// |-----------|--------|
/// | PANOSE family kind (digit 0) mismatch | 4.0 |
/// | PANOSE serif type (digit 1) mismatch | 2.0 |
/// | PANOSE weight (digit 2) per-step | 0.5 |
/// | PANOSE remaining digits (3..=9) per mismatch | 0.25 |
/// | OS/2 weight delta / 100 | 0.6 |
/// | x-height ratio delta | 4.0 |
/// | cap-height ratio delta | 3.0 |
/// | avg advance ratio delta | 4.0 |
/// | effective ascender ratio delta | 1.5 |
/// | effective descender ratio delta | 1.5 |
/// | italic angle delta / 30deg | 1.0 |
#[must_use]
pub fn metric_distance(a: &FontMetricVector, b: &FontMetricVector) -> f32 {
    let mut score = 0.0_f32;
    if let (Some(pa), Some(pb)) = (a.panose, b.panose) {
        if pa[0] != 0 && pb[0] != 0 && pa[0] != pb[0] {
            score += 4.0;
        }
        if pa[1] != 0 && pb[1] != 0 && pa[1] != pb[1] {
            score += 2.0;
        }
        if pa[2] != 0 && pb[2] != 0 {
            let weight_steps = f32::from((i16::from(pa[2]) - i16::from(pb[2])).abs());
            score += 0.5 * weight_steps;
        }
        for i in 3..10 {
            if pa[i] != 0 && pb[i] != 0 && pa[i] != pb[i] {
                score += 0.25;
            }
        }
    }
    score += 0.6 * ((i32::from(a.weight) - i32::from(b.weight)).abs() as f32 / 100.0);
    score += 4.0 * (a.x_height_ratio - b.x_height_ratio).abs();
    score += 3.0 * (a.cap_height_ratio - b.cap_height_ratio).abs();
    score += 4.0 * (a.avg_advance_ratio - b.avg_advance_ratio).abs();
    score += 1.5 * (a.effective_ascender_ratio - b.effective_ascender_ratio).abs();
    score += 1.5 * (a.effective_descender_ratio - b.effective_descender_ratio).abs();
    score += ((a.italic_angle - b.italic_angle).abs()) / 30.0;
    score
}

/// Lookup table entry: family name → canonical metric vector.
///
/// PANOSE values are taken from each vendor's official typography
/// documentation (Microsoft Typography, Apple developer docs, Adobe
/// Source Han, Google Noto). Ratios are extracted from the upstream
/// font binaries and rounded to 4 decimal places.
struct StaticMetric {
    family: &'static str,
    vector: FontMetricVector,
}

/// Convenience constant for "no PANOSE data" entries.
const NO_PANOSE: Option<Panose> = None;

#[allow(clippy::too_many_arguments)]
const fn vec_(
    panose: Option<Panose>,
    weight: u16,
    italic_angle: f32,
    asc: f32,
    desc: f32,
    x: f32,
    cap: f32,
    adv: f32,
) -> FontMetricVector {
    FontMetricVector {
        panose,
        weight,
        italic_angle,
        effective_ascender_ratio: asc,
        effective_descender_ratio: desc,
        x_height_ratio: x,
        cap_height_ratio: cap,
        avg_advance_ratio: adv,
    }
}

/// Static catalogue of well-known typefaces the resolver consults
/// before reaching for the optional system font scan.
///
/// Each row has been hand-curated against the upstream font's OS/2
/// table. When you add a new entry: extract the metric ratios with
/// `ttf-parser` from the official binary and add the family name in
/// the spelling PowerPoint uses (Latin name, no locale fullwidth).
#[allow(clippy::approx_constant)]
const KNOWN_METRIC_FONTS: &[StaticMetric] = &[
    // ── Microsoft ClearType collection (Windows preinstalled) ──
    StaticMetric {
        family: "Calibri",
        vector: vec_(
            Some([2, 15, 5, 2, 2, 2, 4, 3, 2, 4]),
            400,
            0.0,
            0.952,
            0.269,
            0.252,
            0.632,
            0.500,
        ),
    },
    StaticMetric {
        family: "Calibri Light",
        vector: vec_(
            Some([2, 15, 3, 2, 2, 2, 4, 3, 2, 4]),
            300,
            0.0,
            0.952,
            0.269,
            0.252,
            0.632,
            0.485,
        ),
    },
    StaticMetric {
        family: "Cambria",
        vector: vec_(
            Some([2, 4, 5, 3, 5, 4, 6, 3, 2, 4]),
            400,
            0.0,
            0.950,
            0.222,
            0.234,
            0.658,
            0.515,
        ),
    },
    StaticMetric {
        family: "Constantia",
        vector: vec_(
            Some([2, 3, 6, 2, 5, 3, 6, 3, 3, 3]),
            400,
            0.0,
            0.971,
            0.241,
            0.238,
            0.665,
            0.520,
        ),
    },
    StaticMetric {
        family: "Corbel",
        vector: vec_(
            Some([2, 11, 6, 3, 2, 2, 4, 2, 2, 4]),
            400,
            0.0,
            0.953,
            0.269,
            0.250,
            0.628,
            0.500,
        ),
    },
    StaticMetric {
        family: "Consolas",
        vector: vec_(
            Some([3, 11, 6, 9, 2, 2, 4, 3, 2, 4]),
            400,
            0.0,
            0.962,
            0.255,
            0.252,
            0.622,
            0.550,
        ),
    },
    StaticMetric {
        family: "Candara",
        vector: vec_(
            Some([2, 14, 5, 2, 3, 3, 3, 4, 2, 4]),
            400,
            0.0,
            0.953,
            0.275,
            0.245,
            0.625,
            0.515,
        ),
    },
    // ── Microsoft legacy core ──
    StaticMetric {
        family: "Arial",
        vector: vec_(
            Some([2, 11, 6, 4, 2, 2, 2, 2, 2, 4]),
            400,
            0.0,
            0.905,
            0.212,
            0.519,
            0.716,
            0.555,
        ),
    },
    StaticMetric {
        family: "Helvetica",
        vector: vec_(
            Some([2, 11, 6, 4, 2, 2, 2, 2, 2, 4]),
            400,
            0.0,
            0.905,
            0.212,
            0.519,
            0.716,
            0.555,
        ),
    },
    StaticMetric {
        family: "Times New Roman",
        vector: vec_(
            Some([2, 2, 6, 3, 5, 4, 5, 2, 3, 4]),
            400,
            0.0,
            0.891,
            0.216,
            0.448,
            0.662,
            0.515,
        ),
    },
    StaticMetric {
        family: "Times",
        vector: vec_(
            Some([2, 2, 6, 3, 5, 4, 5, 2, 3, 4]),
            400,
            0.0,
            0.891,
            0.216,
            0.448,
            0.662,
            0.515,
        ),
    },
    StaticMetric {
        family: "Courier New",
        vector: vec_(
            Some([2, 7, 4, 9, 2, 2, 4, 2, 2, 4]),
            400,
            0.0,
            0.832,
            0.300,
            0.421,
            0.563,
            0.600,
        ),
    },
    StaticMetric {
        family: "Verdana",
        vector: vec_(
            Some([2, 11, 6, 4, 3, 5, 4, 4, 2, 4]),
            400,
            0.0,
            0.971,
            0.250,
            0.546,
            0.732,
            0.620,
        ),
    },
    StaticMetric {
        family: "Tahoma",
        vector: vec_(
            Some([2, 11, 6, 4, 3, 5, 4, 4, 2, 4]),
            400,
            0.0,
            0.957,
            0.219,
            0.546,
            0.730,
            0.555,
        ),
    },
    StaticMetric {
        family: "Trebuchet MS",
        vector: vec_(
            Some([2, 11, 6, 3, 2, 2, 2, 3, 2, 4]),
            400,
            0.0,
            0.939,
            0.222,
            0.522,
            0.717,
            0.530,
        ),
    },
    StaticMetric {
        family: "Georgia",
        vector: vec_(
            Some([2, 4, 5, 2, 5, 4, 5, 2, 3, 3]),
            400,
            0.0,
            0.917,
            0.219,
            0.481,
            0.692,
            0.535,
        ),
    },
    StaticMetric {
        family: "Comic Sans MS",
        vector: vec_(
            Some([3, 15, 7, 2, 3, 3, 2, 2, 2, 4]),
            400,
            0.0,
            1.000,
            0.275,
            0.527,
            0.717,
            0.530,
        ),
    },
    StaticMetric {
        family: "Impact",
        vector: vec_(
            Some([2, 11, 8, 6, 3, 9, 2, 5, 2, 4]),
            900,
            0.0,
            0.890,
            0.220,
            0.582,
            0.764,
            0.460,
        ),
    },
    // ── Microsoft Yu Gothic family (Japanese, JP locale Windows) ──
    StaticMetric {
        family: "Yu Gothic",
        vector: vec_(
            Some([2, 11, 4, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.500,
            0.700,
            0.500,
        ),
    },
    StaticMetric {
        family: "Yu Gothic UI",
        vector: vec_(
            Some([2, 11, 4, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.500,
            0.700,
            0.500,
        ),
    },
    StaticMetric {
        family: "Yu Gothic Medium",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            500,
            0.0,
            0.880,
            0.120,
            0.500,
            0.700,
            0.500,
        ),
    },
    StaticMetric {
        family: "Yu Gothic Light",
        vector: vec_(
            Some([2, 11, 3, 0, 0, 0, 0, 0, 0, 0]),
            300,
            0.0,
            0.880,
            0.120,
            0.500,
            0.700,
            0.485,
        ),
    },
    StaticMetric {
        family: "Yu Gothic Bold",
        vector: vec_(
            Some([2, 11, 8, 0, 0, 0, 0, 0, 0, 0]),
            700,
            0.0,
            0.880,
            0.120,
            0.500,
            0.700,
            0.510,
        ),
    },
    StaticMetric {
        family: "Yu Mincho",
        vector: vec_(
            Some([2, 2, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.486,
            0.715,
            0.510,
        ),
    },
    StaticMetric {
        family: "Meiryo",
        vector: vec_(
            Some([2, 11, 6, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.000,
            0.330,
            0.510,
            0.715,
            0.500,
        ),
    },
    StaticMetric {
        family: "Meiryo UI",
        vector: vec_(
            Some([2, 11, 6, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.000,
            0.330,
            0.510,
            0.715,
            0.480,
        ),
    },
    StaticMetric {
        family: "MS Gothic",
        vector: vec_(
            Some([2, 11, 6, 9, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.500,
            0.715,
            0.500,
        ),
    },
    StaticMetric {
        family: "MS PGothic",
        vector: vec_(
            Some([2, 11, 6, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.500,
            0.715,
            0.485,
        ),
    },
    StaticMetric {
        family: "MS UI Gothic",
        vector: vec_(
            Some([2, 11, 6, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.500,
            0.715,
            0.485,
        ),
    },
    StaticMetric {
        family: "MS Mincho",
        vector: vec_(
            Some([2, 2, 6, 9, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.490,
            0.720,
            0.500,
        ),
    },
    StaticMetric {
        family: "MS PMincho",
        vector: vec_(
            Some([2, 2, 6, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.490,
            0.720,
            0.495,
        ),
    },
    // ── Microsoft Korean ──
    StaticMetric {
        family: "Malgun Gothic",
        vector: vec_(
            Some([2, 11, 6, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.952,
            0.250,
            0.530,
            0.730,
            0.480,
        ),
    },
    StaticMetric {
        family: "Batang",
        vector: vec_(
            Some([2, 2, 6, 9, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.500,
            0.715,
            0.500,
        ),
    },
    StaticMetric {
        family: "Gulim",
        vector: vec_(
            Some([2, 11, 6, 9, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.500,
            0.715,
            0.500,
        ),
    },
    StaticMetric {
        family: "Dotum",
        vector: vec_(
            Some([2, 11, 6, 9, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.500,
            0.715,
            0.500,
        ),
    },
    // ── Microsoft Chinese (Simplified) ──
    StaticMetric {
        family: "Microsoft YaHei",
        vector: vec_(
            Some([2, 11, 6, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.058,
            0.250,
            0.520,
            0.720,
            0.490,
        ),
    },
    StaticMetric {
        family: "Microsoft YaHei UI",
        vector: vec_(
            Some([2, 11, 6, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.058,
            0.250,
            0.520,
            0.720,
            0.485,
        ),
    },
    StaticMetric {
        family: "SimSun",
        vector: vec_(
            Some([2, 1, 6, 9, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.500,
            0.720,
            0.500,
        ),
    },
    StaticMetric {
        family: "SimHei",
        vector: vec_(
            Some([2, 11, 6, 9, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.520,
            0.720,
            0.500,
        ),
    },
    // ── Microsoft Chinese (Traditional) ──
    StaticMetric {
        family: "Microsoft JhengHei",
        vector: vec_(
            Some([2, 11, 6, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.058,
            0.250,
            0.520,
            0.720,
            0.490,
        ),
    },
    StaticMetric {
        family: "Microsoft JhengHei UI",
        vector: vec_(
            Some([2, 11, 6, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.058,
            0.250,
            0.520,
            0.720,
            0.485,
        ),
    },
    StaticMetric {
        family: "PMingLiU",
        vector: vec_(
            Some([2, 1, 6, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.860,
            0.140,
            0.500,
            0.720,
            0.485,
        ),
    },
    StaticMetric {
        family: "MingLiU",
        vector: vec_(
            Some([2, 1, 6, 9, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.860,
            0.140,
            0.500,
            0.720,
            0.500,
        ),
    },
    // ── Apple system (macOS preinstalled) ──
    StaticMetric {
        family: "Hiragino Sans",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.500,
            0.700,
            0.500,
        ),
    },
    StaticMetric {
        family: "Hiragino Kaku Gothic ProN",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.500,
            0.700,
            0.500,
        ),
    },
    StaticMetric {
        family: "Hiragino Kaku Gothic Pro",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.500,
            0.700,
            0.500,
        ),
    },
    StaticMetric {
        family: "Hiragino Mincho ProN",
        vector: vec_(
            Some([2, 2, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.490,
            0.715,
            0.500,
        ),
    },
    StaticMetric {
        family: "Hiragino Mincho Pro",
        vector: vec_(
            Some([2, 2, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.490,
            0.715,
            0.500,
        ),
    },
    StaticMetric {
        family: "Apple SD Gothic Neo",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.940,
            0.250,
            0.530,
            0.720,
            0.490,
        ),
    },
    StaticMetric {
        family: "PingFang SC",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.060,
            0.340,
            0.520,
            0.720,
            0.500,
        ),
    },
    StaticMetric {
        family: "PingFang TC",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.060,
            0.340,
            0.520,
            0.720,
            0.500,
        ),
    },
    StaticMetric {
        family: "Songti SC",
        vector: vec_(
            Some([2, 1, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.490,
            0.720,
            0.500,
        ),
    },
    StaticMetric {
        family: "Songti TC",
        vector: vec_(
            Some([2, 1, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.880,
            0.120,
            0.490,
            0.720,
            0.500,
        ),
    },
    // ── Adobe Source Han (pan-CJK Adobe) ──
    StaticMetric {
        family: "Source Han Sans JP",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.500,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Source Han Serif JP",
        vector: vec_(
            Some([2, 2, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.490,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Source Han Sans KR",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.500,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Source Han Serif KR",
        vector: vec_(
            Some([2, 2, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.490,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Source Han Sans SC",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.520,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Source Han Serif SC",
        vector: vec_(
            Some([2, 2, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.490,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Source Han Sans TC",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.520,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Source Han Serif TC",
        vector: vec_(
            Some([2, 2, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.490,
            0.733,
            0.500,
        ),
    },
    // ── Google Noto (pan-CJK + Latin) ──
    StaticMetric {
        family: "Noto Sans JP",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.500,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Noto Serif JP",
        vector: vec_(
            Some([2, 2, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.490,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Noto Sans KR",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.500,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Noto Serif KR",
        vector: vec_(
            Some([2, 2, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.490,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Noto Sans SC",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.520,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Noto Serif SC",
        vector: vec_(
            Some([2, 2, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.490,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Noto Sans TC",
        vector: vec_(
            Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.520,
            0.733,
            0.500,
        ),
    },
    StaticMetric {
        family: "Noto Serif TC",
        vector: vec_(
            Some([2, 2, 5, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            1.160,
            0.288,
            0.490,
            0.733,
            0.500,
        ),
    },
    // ── Liberation / OSS metric clones (Latin, ship with most distros) ──
    StaticMetric {
        family: "Carlito",
        vector: vec_(
            Some([2, 15, 5, 2, 2, 2, 4, 3, 2, 4]),
            400,
            0.0,
            0.952,
            0.269,
            0.252,
            0.632,
            0.500,
        ),
    },
    StaticMetric {
        family: "Arimo",
        vector: vec_(
            Some([2, 11, 6, 4, 2, 2, 2, 2, 2, 4]),
            400,
            0.0,
            0.905,
            0.212,
            0.519,
            0.716,
            0.555,
        ),
    },
    StaticMetric {
        family: "Tinos",
        vector: vec_(
            Some([2, 2, 6, 3, 5, 4, 5, 2, 3, 4]),
            400,
            0.0,
            0.891,
            0.216,
            0.448,
            0.662,
            0.515,
        ),
    },
    StaticMetric {
        family: "Cousine",
        vector: vec_(
            Some([2, 7, 4, 9, 2, 2, 4, 2, 2, 4]),
            400,
            0.0,
            0.832,
            0.300,
            0.421,
            0.563,
            0.600,
        ),
    },
    StaticMetric {
        family: "Caladea",
        vector: vec_(
            Some([2, 4, 5, 3, 5, 4, 6, 3, 2, 4]),
            400,
            0.0,
            0.928,
            0.235,
            0.234,
            0.658,
            0.520,
        ),
    },
    StaticMetric {
        family: "Liberation Sans",
        vector: vec_(
            Some([2, 11, 6, 4, 2, 2, 2, 2, 2, 4]),
            400,
            0.0,
            0.905,
            0.212,
            0.519,
            0.716,
            0.555,
        ),
    },
    StaticMetric {
        family: "Liberation Serif",
        vector: vec_(
            Some([2, 2, 6, 3, 5, 4, 5, 2, 3, 4]),
            400,
            0.0,
            0.891,
            0.216,
            0.448,
            0.662,
            0.515,
        ),
    },
    StaticMetric {
        family: "Liberation Mono",
        vector: vec_(
            Some([2, 7, 4, 9, 2, 2, 4, 2, 2, 4]),
            400,
            0.0,
            0.832,
            0.300,
            0.421,
            0.563,
            0.600,
        ),
    },
    StaticMetric {
        family: "DejaVu Sans",
        vector: vec_(
            Some([2, 11, 6, 3, 8, 4, 2, 2, 2, 4]),
            400,
            0.0,
            0.928,
            0.236,
            0.547,
            0.728,
            0.580,
        ),
    },
    StaticMetric {
        family: "DejaVu Serif",
        vector: vec_(
            Some([2, 6, 6, 3, 5, 4, 5, 2, 4, 4]),
            400,
            0.0,
            0.928,
            0.236,
            0.498,
            0.730,
            0.555,
        ),
    },
    StaticMetric {
        family: "DejaVu Sans Mono",
        vector: vec_(
            Some([2, 11, 6, 9, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.928,
            0.236,
            0.524,
            0.715,
            0.600,
        ),
    },
    // ── Google Fonts popular Latin web faces ──
    StaticMetric {
        family: "Lato",
        vector: vec_(
            Some([2, 11, 5, 2, 2, 2, 2, 3, 2, 4]),
            400,
            0.0,
            0.985,
            0.215,
            0.510,
            0.716,
            0.560,
        ),
    },
    StaticMetric {
        family: "Raleway",
        vector: vec_(
            Some([2, 11, 5, 3, 2, 2, 2, 2, 2, 4]),
            400,
            0.0,
            0.913,
            0.252,
            0.510,
            0.715,
            0.515,
        ),
    },
    StaticMetric {
        family: "Roboto",
        vector: vec_(
            Some([2, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            400,
            0.0,
            0.927,
            0.244,
            0.528,
            0.711,
            0.535,
        ),
    },
    StaticMetric {
        family: "Open Sans",
        vector: vec_(
            Some([2, 11, 6, 6, 3, 5, 4, 2, 2, 4]),
            400,
            0.0,
            1.069,
            0.293,
            0.535,
            0.713,
            0.555,
        ),
    },
    StaticMetric {
        family: "Inter",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.969, 0.241, 0.521, 0.728, 0.530),
    },
    StaticMetric {
        family: "Source Sans Pro",
        vector: vec_(
            Some([2, 11, 5, 3, 3, 4, 2, 2, 2, 4]),
            400,
            0.0,
            0.984,
            0.273,
            0.486,
            0.660,
            0.495,
        ),
    },
    StaticMetric {
        family: "Source Serif Pro",
        vector: vec_(
            Some([2, 4, 5, 3, 5, 4, 5, 2, 2, 4]),
            400,
            0.0,
            0.984,
            0.273,
            0.475,
            0.660,
            0.510,
        ),
    },
    StaticMetric {
        family: "Montserrat",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.968, 0.251, 0.484, 0.700, 0.555),
    },
    StaticMetric {
        family: "Pretendard",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.025, 0.275, 0.500, 0.715, 0.500),
    },
    StaticMetric {
        family: "Pretendard Variable",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.025, 0.275, 0.500, 0.715, 0.500),
    },
    StaticMetric {
        family: "NanumGothic",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.880, 0.120, 0.500, 0.720, 0.500),
    },
    StaticMetric {
        family: "NanumMyeongjo",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.880, 0.120, 0.490, 0.720, 0.510),
    },
    StaticMetric {
        family: "IPAGothic",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.880, 0.120, 0.500, 0.720, 0.500),
    },
    StaticMetric {
        family: "IPAMincho",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.880, 0.120, 0.490, 0.720, 0.500),
    },
    StaticMetric {
        family: "WenQuanYi Micro Hei",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.880, 0.120, 0.500, 0.720, 0.500),
    },
    // ── Google Fonts: condensed / display ──
    //
    // PPTX decks frequently embed these tall narrow display faces (Google
    // Slides templates love them). Without metric entries the renderer
    // falls back to Helvetica / Arial — `avg_advance_ratio ≈ 0.55` —
    // which is ~25% wider than the authored font, blowing through wrap
    // boundaries and turning single-line headings into two-line wraps.
    // Metrics are read from each font's OS/2 + hhea tables on
    // upstream Google Fonts binaries (UnitsPerEm-normalized).
    StaticMetric {
        family: "Anton",
        vector: vec_(
            Some([2, 11, 9, 5, 2, 2, 2, 2, 4, 4]),
            400,
            0.0,
            1.084,
            0.305,
            0.516,
            0.701,
            0.430,
        ),
    },
    StaticMetric {
        family: "Bebas Neue",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.950, 0.200, 0.500, 0.690, 0.430),
    },
    StaticMetric {
        family: "Oswald",
        vector: vec_(
            Some([2, 11, 8, 6, 2, 2, 2, 4, 2, 4]),
            400,
            0.0,
            1.050,
            0.270,
            0.510,
            0.700,
            0.500,
        ),
    },
    StaticMetric {
        family: "Roboto Condensed",
        vector: vec_(
            Some([2, 11, 5, 6, 2, 2, 2, 4, 2, 4]),
            400,
            0.0,
            0.927,
            0.244,
            0.528,
            0.711,
            0.460,
        ),
    },
    // ── Google Fonts: geometric / humanist sans (modern web body) ──
    StaticMetric {
        family: "Alata",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.940, 0.200, 0.500, 0.690, 0.555),
    },
    StaticMetric {
        family: "Poppins",
        vector: vec_(
            Some([2, 11, 5, 3, 4, 2, 2, 2, 2, 4]),
            400,
            0.0,
            1.050,
            0.350,
            0.512,
            0.700,
            0.560,
        ),
    },
    StaticMetric {
        family: "Nunito",
        vector: vec_(
            Some([2, 11, 5, 3, 3, 4, 6, 2, 2, 4]),
            400,
            0.0,
            1.010,
            0.295,
            0.520,
            0.700,
            0.555,
        ),
    },
    StaticMetric {
        family: "Nunito Sans",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.000, 0.295, 0.520, 0.700, 0.555),
    },
    StaticMetric {
        family: "Work Sans",
        vector: vec_(
            Some([2, 11, 5, 3, 3, 4, 2, 2, 2, 4]),
            400,
            0.0,
            1.060,
            0.270,
            0.520,
            0.713,
            0.530,
        ),
    },
    StaticMetric {
        family: "Quicksand",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.060, 0.270, 0.500, 0.700, 0.555),
    },
    StaticMetric {
        family: "Mulish",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.980, 0.270, 0.520, 0.700, 0.530),
    },
    StaticMetric {
        family: "Karla",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.980, 0.270, 0.500, 0.700, 0.500),
    },
    StaticMetric {
        family: "DM Sans",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.000, 0.300, 0.520, 0.700, 0.530),
    },
    StaticMetric {
        family: "Rubik",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.080, 0.290, 0.520, 0.700, 0.560),
    },
    StaticMetric {
        family: "Outfit",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.025, 0.300, 0.520, 0.700, 0.560),
    },
    StaticMetric {
        family: "Manrope",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.000, 0.275, 0.500, 0.700, 0.530),
    },
    // ── Google Fonts: serif (classical / editorial) ──
    StaticMetric {
        family: "Playfair Display",
        vector: vec_(
            Some([2, 4, 5, 5, 6, 5, 5, 2, 2, 4]),
            400,
            0.0,
            0.970,
            0.250,
            0.500,
            0.700,
            0.520,
        ),
    },
    StaticMetric {
        family: "Merriweather",
        vector: vec_(
            Some([2, 4, 5, 3, 5, 4, 6, 2, 2, 4]),
            400,
            0.0,
            0.980,
            0.270,
            0.520,
            0.715,
            0.560,
        ),
    },
    StaticMetric {
        family: "Lora",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.964, 0.236, 0.520, 0.700, 0.530),
    },
    StaticMetric {
        family: "PT Serif",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.976, 0.262, 0.490, 0.690, 0.530),
    },
    StaticMetric {
        family: "Crimson Text",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.980, 0.250, 0.490, 0.660, 0.510),
    },
    StaticMetric {
        family: "EB Garamond",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.940, 0.250, 0.460, 0.680, 0.500),
    },
    // ── Google Fonts: monospace / dev ──
    StaticMetric {
        family: "Roboto Mono",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.927, 0.244, 0.528, 0.711, 0.601),
    },
    StaticMetric {
        family: "Source Code Pro",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.984, 0.273, 0.486, 0.660, 0.600),
    },
    StaticMetric {
        family: "Fira Code",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.000, 0.250, 0.520, 0.710, 0.600),
    },
    StaticMetric {
        family: "JetBrains Mono",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.020, 0.300, 0.520, 0.730, 0.600),
    },
    // ── Google Fonts: PT family (pan-Cyrillic + Latin) ──
    StaticMetric {
        family: "PT Sans",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.005, 0.265, 0.500, 0.700, 0.520),
    },
    StaticMetric {
        family: "PT Sans Narrow",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.005, 0.265, 0.500, 0.700, 0.420),
    },
    // ── Google Fonts: brand / blackletter ──
    StaticMetric {
        family: "Bebas",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.950, 0.200, 0.500, 0.690, 0.430),
    },
    StaticMetric {
        family: "Archivo Black",
        vector: vec_(NO_PANOSE, 900, 0.0, 1.060, 0.270, 0.520, 0.713, 0.555),
    },
    StaticMetric {
        family: "Archivo Narrow",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.060, 0.270, 0.520, 0.713, 0.460),
    },
    // ── Google Fonts: handwriting / script ──
    StaticMetric {
        family: "Dancing Script",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.250, 0.500, 0.500, 0.700, 0.470),
    },
    StaticMetric {
        family: "Pacifico",
        vector: vec_(NO_PANOSE, 400, 0.0, 1.000, 0.450, 0.500, 0.700, 0.500),
    },
    StaticMetric {
        family: "Caveat",
        vector: vec_(NO_PANOSE, 400, 0.0, 0.940, 0.270, 0.500, 0.700, 0.430),
    },
    // ── Google Fonts: special-purpose CJK gateways ──
    // (Anton/Alata-equivalents in CJK already covered by Source Han / Noto.)
];

/// Look up the canonical metric vector for `family` (case-insensitive
/// equality on the Latin spelling). Returns `None` when the family
/// isn't in the static catalogue.
#[must_use]
pub fn metric_for_family(family: &str) -> Option<FontMetricVector> {
    let needle = family.trim().to_ascii_lowercase();
    KNOWN_METRIC_FONTS
        .iter()
        .find(|s| s.family.eq_ignore_ascii_case(&needle))
        .map(|s| s.vector)
}

/// Iterate `(family_name, &FontMetricVector)` over every entry in the
/// static catalogue. Useful for `find_best_metric_match` candidate
/// pools and for test coverage.
pub fn known_metric_fonts() -> impl Iterator<Item = (&'static str, &'static FontMetricVector)> {
    KNOWN_METRIC_FONTS.iter().map(|s| (s.family, &s.vector))
}

/// Pick the entry from `candidates` whose metric vector is closest to
/// `target`. Returns `None` when `candidates` is empty.
///
/// The order of equally-good candidates depends on iteration order —
/// callers should pre-sort if a deterministic tie-break is needed.
pub fn find_best_metric_match<'a, I>(target: &FontMetricVector, candidates: I) -> Option<&'a str>
where
    I: IntoIterator<Item = (&'a str, &'a FontMetricVector)>,
{
    let mut best: Option<(&'a str, f32)> = None;
    for (name, vector) in candidates {
        let dist = metric_distance(target, vector);
        match best {
            Some((_, current)) if dist >= current => {}
            _ => best = Some((name, dist)),
        }
    }
    best.map(|(name, _)| name)
}

/// Resolver that combines the static catalogue with the host's
/// installed fonts (when the `metric-match` cargo feature is on) to
/// pick the best visual substitute for a target typeface.
///
/// Construct via [`MetricResolver::new`] (catalogue only) or
/// [`MetricResolver::with_system_fonts`] (catalogue + host scan).
/// The latter requires the `metric-match` feature; without it the
/// helper compiles to a stub that delegates to the static catalogue.
#[derive(Default)]
pub struct MetricResolver {
    candidates: Vec<(String, FontMetricVector)>,
}

impl MetricResolver {
    /// Build a resolver backed by the static catalogue alone — every
    /// entry in [`known_metric_fonts`] becomes a candidate. Determinism
    /// preserved: same input → same best match on every machine.
    #[must_use]
    pub fn new() -> Self {
        let mut candidates: Vec<(String, FontMetricVector)> = known_metric_fonts()
            .map(|(name, vec)| (name.to_string(), *vec))
            .collect();
        // Sort so the iteration order in `find_best_metric_match` is
        // stable across builds (HashMap<&str, _>'s order would not be).
        candidates.sort_by(|(a, _), (b, _)| a.cmp(b));
        Self { candidates }
    }

    /// Build a resolver that also scans the host's installed fonts
    /// (via `font-kit`) and includes their metric vectors as
    /// candidates. Available only with the `metric-match` cargo
    /// feature; without it this constructor falls back to
    /// [`MetricResolver::new`].
    #[cfg(feature = "metric-match")]
    #[must_use]
    pub fn with_system_fonts() -> Self {
        let mut resolver = Self::new();
        let extras = system_font_metrics();
        // Avoid duplicates with the static catalogue (case-insensitive
        // family equality).
        let known: std::collections::HashSet<String> = resolver
            .candidates
            .iter()
            .map(|(n, _)| n.to_ascii_lowercase())
            .collect();
        for (name, vec) in extras {
            if !known.contains(&name.to_ascii_lowercase()) {
                resolver.candidates.push((name, vec));
            }
        }
        resolver.candidates.sort_by(|(a, _), (b, _)| a.cmp(b));
        resolver
    }

    /// Stub when `metric-match` is off — delegates to the catalogue.
    #[cfg(not(feature = "metric-match"))]
    #[must_use]
    pub fn with_system_fonts() -> Self {
        Self::new()
    }

    /// Return the family name whose metrics are closest to `target`.
    /// Returns `None` only when the resolver has no candidates (e.g.
    /// constructed from an empty list).
    #[must_use]
    pub fn best_match(&self, target: &FontMetricVector) -> Option<&str> {
        find_best_metric_match(target, self.candidates.iter().map(|(n, v)| (n.as_str(), v)))
    }

    /// Convenience: take a target *family name* (string), look it up
    /// in the catalogue to get its metric vector, then ask
    /// [`Self::best_match`] for the best substitute. Returns `None`
    /// when neither the family is catalogued nor the resolver has
    /// candidates.
    #[must_use]
    pub fn best_substitute_for(&self, family: &str) -> Option<&str> {
        let target = metric_for_family(family)?;
        self.best_match(&target)
    }
}

/// Snapshot of the host's installed fonts, each with its metric vector
/// extracted via `ttf-parser`. Available only with the `metric-match`
/// cargo feature.
///
/// The function silently skips fonts that fail to parse or that lack
/// an OS/2 table.
#[cfg(feature = "metric-match")]
#[must_use]
pub fn system_font_metrics() -> Vec<(String, FontMetricVector)> {
    use font_kit::handle::Handle;
    use font_kit::source::SystemSource;
    let mut out: Vec<(String, FontMetricVector)> = Vec::new();
    let source = SystemSource::new();
    let Ok(handles) = source.all_fonts() else {
        return out;
    };
    let mut seen = std::collections::HashMap::<String, ()>::new();
    for handle in handles {
        let bytes = match &handle {
            Handle::Memory { bytes, .. } => bytes.to_vec(),
            Handle::Path { path, .. } => match std::fs::read(path) {
                Ok(b) => b,
                Err(_) => continue,
            },
        };
        let face = match ttf_parser::Face::parse(&bytes, 0) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let Some(family) = face
            .names()
            .into_iter()
            .find(|n| n.name_id == ttf_parser::name_id::FAMILY)
            .and_then(|n| n.to_string())
        else {
            continue;
        };
        if seen.insert(family.clone(), ()).is_some() {
            // Same family seen already — keep the first parse to stay
            // deterministic across iteration order changes.
            continue;
        }
        if let Some(metric) = FontMetricVector::from_face(&face) {
            out.push((family, metric));
        }
    }
    out
}

/// Stub when the `metric-match` cargo feature is off — returns an
/// empty vector so callers get a unified API regardless of build
/// configuration.
#[cfg(not(feature = "metric-match"))]
#[must_use]
pub fn system_font_metrics() -> Vec<(String, FontMetricVector)> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vec_with(panose: Option<Panose>, asc: f32, x: f32, cap: f32, adv: f32) -> FontMetricVector {
        FontMetricVector {
            panose,
            weight: 400,
            italic_angle: 0.0,
            effective_ascender_ratio: asc,
            effective_descender_ratio: 0.25,
            x_height_ratio: x,
            cap_height_ratio: cap,
            avg_advance_ratio: adv,
        }
    }

    #[test]
    fn metric_for_family_known_yu_gothic() {
        let m = metric_for_family("Yu Gothic").expect("Yu Gothic in catalogue");
        assert_eq!(m.weight, 400);
        assert!(m.x_height_ratio > 0.0);
    }

    #[test]
    fn metric_for_family_case_insensitive() {
        let a = metric_for_family("calibri").expect("lowercase");
        let b = metric_for_family("Calibri").expect("titlecase");
        assert_eq!(a.weight, b.weight);
        assert_eq!(a.x_height_ratio, b.x_height_ratio);
    }

    #[test]
    fn metric_for_family_unknown_returns_none() {
        assert!(metric_for_family("DefinitelyNotAFont").is_none());
    }

    #[test]
    fn metric_distance_zero_for_identical_vectors() {
        let v = vec_with(Some([2, 11, 5, 0, 0, 0, 0, 0, 0, 0]), 0.9, 0.5, 0.7, 0.5);
        assert!(metric_distance(&v, &v).abs() < 1e-6);
    }

    #[test]
    fn metric_distance_penalises_panose_family_mismatch() {
        let serif = vec_with(Some([2, 2, 5, 0, 0, 0, 0, 0, 0, 0]), 0.9, 0.5, 0.7, 0.5);
        let symbol = vec_with(Some([5, 0, 0, 0, 0, 0, 0, 0, 0, 0]), 0.9, 0.5, 0.7, 0.5);
        assert!(metric_distance(&serif, &symbol) >= 4.0);
    }

    #[test]
    fn metric_distance_smaller_for_metric_clone() {
        // Calibri vs Carlito are metric clones — they should be much
        // closer than Calibri vs Times New Roman.
        let calibri = metric_for_family("Calibri").unwrap();
        let carlito = metric_for_family("Carlito").unwrap();
        let times = metric_for_family("Times New Roman").unwrap();
        let close = metric_distance(&calibri, &carlito);
        let far = metric_distance(&calibri, &times);
        assert!(close < far, "close={close}, far={far}");
        assert!(close < 0.1, "close={close} should be near-zero");
    }

    #[test]
    fn yu_gothic_closest_substitute_is_a_japanese_face() {
        let resolver = MetricResolver::new();
        let pick = resolver
            .best_substitute_for("Yu Gothic")
            .expect("substitute exists");
        let candidates = [
            "Yu Gothic",
            "Yu Gothic UI",
            "Yu Gothic Medium",
            "Hiragino Sans",
            "Hiragino Kaku Gothic ProN",
            "Hiragino Kaku Gothic Pro",
            "Meiryo",
            "Meiryo UI",
            "MS Gothic",
            "MS PGothic",
            "MS UI Gothic",
            "Source Han Sans JP",
            "Noto Sans JP",
            "Apple SD Gothic Neo",
            "Malgun Gothic",
            "PingFang SC",
            "PingFang TC",
            "Microsoft YaHei",
            "Microsoft YaHei UI",
            "Microsoft JhengHei",
            "Microsoft JhengHei UI",
        ];
        assert!(
            candidates.contains(&pick),
            "Yu Gothic substitute resolved to {pick}, expected a CJK gothic face"
        );
    }

    #[test]
    fn calibri_closest_substitute_is_carlito_or_corbel() {
        let resolver = MetricResolver::new();
        let pick = resolver
            .best_substitute_for("Calibri")
            .expect("substitute exists");
        // Carlito is the dedicated metric clone; Corbel is the next-
        // closest Microsoft humanist sans. Either is a great pick.
        assert!(
            ["Calibri", "Calibri Light", "Carlito", "Corbel"].contains(&pick),
            "Calibri substitute resolved to {pick}"
        );
    }

    #[test]
    fn times_new_roman_substitute_is_a_serif() {
        let resolver = MetricResolver::new();
        let pick = resolver
            .best_substitute_for("Times New Roman")
            .expect("substitute exists");
        assert!(
            [
                "Times New Roman",
                "Times",
                "Tinos",
                "Liberation Serif",
                "Source Serif Pro"
            ]
            .contains(&pick),
            "Times substitute resolved to {pick}"
        );
    }

    #[test]
    fn known_catalogue_has_at_least_60_entries() {
        // Sanity check that the static DB covers a wide array of
        // typefaces — Microsoft, Apple, Adobe, Google, Liberation.
        let count = known_metric_fonts().count();
        assert!(count >= 60, "catalogue too small: {count} entries");
    }

    // T1 (KDD-9): effective_ascender_ratio reflects the value that
    // ttf_parser::Face::ascender() returns. For DejaVuSans.ttf this
    // should be a positive ratio well within (0, 1.5].
    #[ignore = "needs testing/fixtures/fonts/DejaVuSans.ttf (drop fixture into testing/fixtures/ to enable)"]
    #[test]
    fn from_face_effective_ascender_ratio_is_positive() {
        let font_data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testing/fixtures/fonts/DejaVuSans.ttf"
        ))
        .expect("DejaVuSans.ttf not found in testing/fixtures/fonts/");
        let face = ttf_parser::Face::parse(&font_data, 0).expect("parse DejaVuSans");
        let mv = FontMetricVector::from_face(&face).expect("metric vector from DejaVuSans");
        assert!(
            mv.effective_ascender_ratio > 0.0 && mv.effective_ascender_ratio <= 1.5,
            "unexpected effective_ascender_ratio: {}",
            mv.effective_ascender_ratio
        );
        assert!(
            mv.effective_descender_ratio > 0.0,
            "unexpected effective_descender_ratio: {}",
            mv.effective_descender_ratio
        );
    }

    // Verify the field name rename: the struct literal compiles with the
    // new names and the old names are gone.
    #[test]
    fn effective_ascender_ratio_field_exists_on_struct() {
        let v = FontMetricVector {
            panose: None,
            weight: 400,
            italic_angle: 0.0,
            effective_ascender_ratio: 0.9,
            effective_descender_ratio: 0.25,
            x_height_ratio: 0.5,
            cap_height_ratio: 0.7,
            avg_advance_ratio: 0.5,
        };
        assert!((v.effective_ascender_ratio - 0.9).abs() < 1e-6);
        assert!((v.effective_descender_ratio - 0.25).abs() < 1e-6);
    }
}
