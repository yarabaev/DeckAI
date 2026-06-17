//! `ttf-parser`-backed glyph metrics and family-name extraction.
//!
//! Replaces the `opentype.js` runtime from
//! /
//! . Provides the same surface needed by
//! the renderer: scalar metrics (`units_per_em`, ascender / descender,
//! cap- and x-height), glyph index lookup via `cmap`, horizontal
//! advance, and the OS/2-derived `font-family` name string.
//!
//! ## Self-borrow
//!
//! [`ttf_parser::Face`] borrows from its source `&[u8]`, so a `Face`
//! cannot be stored alongside the `Vec<u8>` it parses without a
//! self-referential structure. We sidestep the problem by re-parsing
//! on each call — `Face::parse` only walks the table directory and is
//! cheap. [`FontFace`] owns the bytes and exposes scalar / per-glyph
//! helpers that internally re-parse.
//!
//! Higher layers (e.g. [`crate::OpentypeTextMeasurer`]) keep a
//! `FontFace` per loaded family and call its measurement helpers.

use std::fmt;

use ttf_parser::{Face, FaceParsingError, GlyphId, Tag};

/// Errors produced while constructing a [`FontFace`].
#[derive(Debug)]
pub enum FontError {
    /// `ttf-parser` rejected the bytes (truncated / unsupported tables).
    Parse(FaceParsingError),
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(err) => write!(f, "failed to parse font: {err}"),
        }
    }
}

impl std::error::Error for FontError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(_) => None,
        }
    }
}

impl From<FaceParsingError> for FontError {
    fn from(value: FaceParsingError) -> Self {
        Self::Parse(value)
    }
}

/// Owned font bytes + face index.
///
/// Owns the raw font data (`Vec<u8>`) so it can be moved between
/// threads, cached, and serialized to disk. Construct with
/// [`FontFace::from_bytes`] which validates that the data parses;
/// subsequent metric / glyph queries re-parse internally.
///
/// `face_index` is 0 for plain TTF / OTF; TTC containers use higher
/// indices. Use [`crate::ttc`] to enumerate TTC faces.
///
/// Variable font axes (e.g. `wght`, `wdth`) are stored via
/// [`FontFace::set_variation`] and applied when [`FontFace::face`]
/// reconstructs the parsed face.
#[derive(Clone, Debug)]
pub struct FontFace {
    data: Vec<u8>,
    face_index: u32,
    /// Variation axis settings: `(tag, value)` pairs accumulated via
    /// [`Self::set_variation`]. Empty for non-variable faces.
    variations: Vec<(Tag, f32)>,
}

impl FontFace {
    /// Parses `data` as a TTF / OTF face at `face_index` and validates
    /// it. Returns the owning wrapper on success.
    pub fn from_bytes(data: Vec<u8>, face_index: u32) -> Result<Self, FontError> {
        Face::parse(&data, face_index)?;
        Ok(Self {
            data,
            face_index,
            variations: Vec::new(),
        })
    }

    /// Borrows the underlying bytes (e.g. for serialization).
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Face index passed to [`Self::from_bytes`].
    #[must_use]
    pub fn face_index(&self) -> u32 {
        self.face_index
    }

    /// Re-parses and returns a [`Face`] borrowing from `self.data`.
    ///
    /// When variation axes have been registered via [`Self::set_variation`],
    /// each axis is applied to the returned face so metric queries reflect
    /// the requested instance (e.g. a bold weight axis rather than a
    /// synthetic `BOLD_FACTOR` heuristic).
    ///
    /// Cheap for non-variable faces (table-directory walk only).
    pub fn face(&self) -> Face<'_> {
        // Safe to unwrap: `from_bytes` already validated parsing succeeds,
        // and the bytes are immutable. A panic here would indicate the
        // bytes were corrupted in place, which our API does not allow.
        let mut face = Face::parse(&self.data, self.face_index)
            .expect("FontFace bytes already validated by from_bytes");
        for &(tag, value) in &self.variations {
            face.set_variation(tag, value);
        }
        face
    }

    /// Returns `true` when the font contains the `fvar` table
    /// (OpenType Font Variations).
    ///
    /// Variable fonts expose continuous axes (`wght`, `wdth`, …) that
    /// let callers request an exact weight or width instance without
    /// needing a separate font file per style.
    #[must_use]
    pub fn is_variable(&self) -> bool {
        // `fvar` presence is the canonical "this is a variable font" check.
        self.face()
            .raw_face()
            .table(Tag::from_bytes(b"fvar"))
            .is_some()
    }

    /// Registers or updates a variation axis value.
    ///
    /// `tag` is the 4-byte OpenType axis tag (e.g. `Tag::from_bytes(b"wght")`).
    /// `value` is the axis value in the axis's design space (e.g. `700.0`
    /// for bold weight).
    ///
    /// The value is stored and applied inside [`Self::face`] on every
    /// re-parse. Calling with the same tag twice replaces the first entry.
    ///
    /// Has no visible effect on non-variable faces — `ttf-parser`
    /// silently ignores `set_variation` calls when no `fvar` table is
    /// present.
    pub fn set_variation(&mut self, tag: Tag, value: f32) {
        if let Some(existing) = self.variations.iter_mut().find(|(t, _)| *t == tag) {
            existing.1 = value;
        } else {
            self.variations.push((tag, value));
        }
    }

    /// Returns an owned clone with `(tag, value)` set as a variation
    /// axis. Convenience wrapper for callers holding `Arc<FontFace>`
    /// who need to apply a single axis without mutating the shared face.
    ///
    /// Has no effect on non-variable fonts (the variation is stored but
    /// `ttf-parser` ignores axes when no `fvar` table is present).
    #[must_use]
    pub fn with_variation(&self, tag: Tag, value: f32) -> Self {
        let mut clone = self.clone();
        clone.set_variation(tag, value);
        clone
    }

    /// `head.unitsPerEm` — the font's design grid resolution.
    #[must_use]
    pub fn units_per_em(&self) -> u16 {
        self.face().units_per_em()
    }

    /// `hhea.ascender` (or `OS/2.sTypoAscender` per the face).
    #[must_use]
    pub fn ascender(&self) -> i16 {
        self.face().ascender()
    }

    /// `hhea.descender` — negative.
    #[must_use]
    pub fn descender(&self) -> i16 {
        self.face().descender()
    }

    /// `OS/2.sCapHeight` if the table is present, else `None`.
    #[must_use]
    pub fn cap_height(&self) -> Option<i16> {
        self.face().capital_height()
    }

    /// `OS/2.sxHeight` if present, else `None`.
    #[must_use]
    pub fn x_height(&self) -> Option<i16> {
        self.face().x_height()
    }

    /// `OS/2.usWeightClass` (CSS weights: 400 regular, 700 bold) when
    /// the OS/2 table is present.
    #[must_use]
    pub fn weight(&self) -> u16 {
        self.face().weight().to_number()
    }

    /// `cmap` lookup. Returns `None` for codepoints with no glyph
    /// (renderer falls back to fallback metrics / heuristic).
    #[must_use]
    pub fn glyph_index(&self, c: char) -> Option<GlyphId> {
        self.face().glyph_index(c)
    }

    /// `hmtx` horizontal advance in font units. `None` if the glyph id
    /// is out of range.
    #[must_use]
    pub fn glyph_hor_advance(&self, glyph: GlyphId) -> Option<u16> {
        self.face().glyph_hor_advance(glyph)
    }

    /// Convenience: cmap → hmtx, returning the advance in font units
    /// for a single character.
    ///
    /// Returns `None` if the glyph is missing (`.notdef`).
    #[must_use]
    pub fn char_hor_advance(&self, c: char) -> Option<u16> {
        let face = self.face();
        let gid = face.glyph_index(c)?;
        face.glyph_hor_advance(gid)
    }

    /// Sum of horizontal advances for every char in `text` (font units).
    ///
    /// Missing glyphs contribute `0` — callers that care about missing
    /// glyph contribution should fall back to fallback metrics or use a
    /// per-char approach.
    #[must_use]
    pub fn measure_text_units(&self, text: &str) -> u32 {
        let face = self.face();
        let mut sum = 0_u32;
        for ch in text.chars() {
            if let Some(gid) = face.glyph_index(ch) {
                if let Some(adv) = face.glyph_hor_advance(gid) {
                    sum = sum.saturating_add(u32::from(adv));
                }
            }
        }
        sum
    }

    /// Pixel width of `text` at `font_size_pt`. Convenience over
    /// [`Self::measure_text_units`] using the `96 / 72` CSS ratio.
    ///
    /// Note this returns *advance sum* — bold widening (e.g. via
    /// synthetic bold) is **not** applied here; callers handle that.
    #[must_use]
    pub fn measure_text_pixels(&self, text: &str, font_size_pt: f64) -> f64 {
        let units = f64::from(self.measure_text_units(text));
        let upem = f64::from(self.units_per_em());
        if upem == 0.0 {
            return 0.0;
        }
        let base_size_px = font_size_pt * (96.0 / 72.0);
        units / upem * base_size_px
    }

    /// Reads the OS/2 / `name` table family name (preferred-family
    /// id 16, fallback id 1). Returns the first English (Mac or
    /// Windows) record's value when present.
    ///
    /// Used by the system font scanner to index loaded faces by
    /// family name.
    #[must_use]
    pub fn family_name(&self) -> Option<String> {
        let face = self.face();
        let names = face.names();
        // Preferred family (id 16) wins when available; otherwise plain
        // family (id 1). Walk records in order, prefer Windows English
        // (platform 3, language 0x0409), fall back to Mac English
        // (platform 1, language 0).
        let mut best: Option<(u8, String)> = None;
        for i in 0..names.len() {
            let Some(rec) = names.get(i) else { continue };
            if rec.name_id != 16 && rec.name_id != 1 {
                continue;
            }
            let Some(text) = rec.to_string() else {
                continue;
            };
            // Score: prefer name_id 16 (preferred family) over 1; within the
            // same id, prefer Windows English (platform 3, language 0x0409)
            // over Mac English. Higher score wins.
            let score: u8 = match (rec.name_id, rec.platform_id, rec.language_id) {
                (16, ttf_parser::PlatformId::Windows, 0x0409) => 8,
                (16, ttf_parser::PlatformId::Macintosh, 0) => 7,
                (16, _, _) => 6,
                (1, ttf_parser::PlatformId::Windows, 0x0409) => 4,
                (1, ttf_parser::PlatformId::Macintosh, 0) => 3,
                (1, _, _) => 2,
                _ => 0,
            };
            match &best {
                Some((cur_score, _)) if *cur_score >= score => {}
                _ => best = Some((score, text)),
            }
        }
        best.map(|(_, name)| name)
    }

    /// True if the font contains the named OpenType layout table (e.g.
    /// `Tag::from_bytes(b"GSUB")`). Useful for shaping decisions.
    #[must_use]
    pub fn has_table(&self, tag: Tag) -> bool {
        self.face().raw_face().table(tag).is_some()
    }

    /// Returns the kern pair adjustment (in font units) for a pair of glyphs
    /// from the TrueType `kern` table, or `0` when the pair has no entry or
    /// the face has no `kern` table.
    ///
    /// Only Format 0 (horizontal, un-crossed) subtables are queried — this
    /// covers the vast majority of Latin fonts. GPOS kerning (Calibri, Noto
    /// Sans, etc.) requires a shaping pass and is handled in the DD-2
    /// (rustybuzz) work item.
    #[must_use]
    pub fn kern_pair_adjustment(&self, left: GlyphId, right: GlyphId) -> i16 {
        let face = self.face();
        let Some(kern) = face.tables().kern else {
            return 0;
        };
        for subtable in kern.subtables {
            if subtable.horizontal && !subtable.variable && !subtable.has_cross_stream {
                if let Some(adj) = subtable.glyphs_kerning(left, right) {
                    return adj;
                }
            }
        }
        0
    }

    /// All distinct family names this face exposes through its `name`
    /// table — every platform / encoding / language record for
    /// `name_id` 1 (Family), 4 (Full name), and 16 (Typographic
    /// family). De-duplicated, original-case preserved.
    ///
    /// PPTX `<a:latin typeface="…"/>` carries whatever string the
    /// authoring app showed at write time; that may be the English
    /// `Family` (id 1, lang 0x0409), the localized id 1 (e.g. Korean
    /// 0x0412 → "프리젠테이션 7 Bold"), the typographic id 16, or the
    /// PostScript id 6. Returning every variant lets the caller wire
    /// the font into `fontdb` / browser font lookup under each label
    /// without baking a per-font alias table into source.
    #[must_use]
    pub fn all_family_names(&self) -> Vec<String> {
        let face = self.face();
        let names = face.names();
        let mut out: Vec<String> = Vec::new();
        for i in 0..names.len() {
            let Some(rec) = names.get(i) else { continue };
            // Family-bearing IDs only — skip subfamily / version / etc.
            if !matches!(rec.name_id, 1 | 4 | 6 | 16 | 21) {
                continue;
            }
            let Some(s) = rec.to_string() else { continue };
            let s = s.trim();
            if s.is_empty() {
                continue;
            }
            if !out.iter().any(|x| x == s) {
                out.push(s.to_string());
            }
        }
        out
    }
}

/// Convenience: walk every TTC sub-face in `data` and return the
/// union of `all_family_names` across all faces. Handy for
/// system-font scanners that want a flat alias list per file.
#[must_use]
pub fn all_face_family_names(data: &[u8]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let face_count = ttf_parser::fonts_in_collection(data).unwrap_or(1);
    for idx in 0..face_count {
        let Ok(face) = FontFace::from_bytes(data.to_vec(), idx) else {
            continue;
        };
        for n in face.all_family_names() {
            if !out.iter().any(|x| x == &n) {
                out.push(n);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Smoke / error-path tests only here. Real-font integration tests
    // live in `tests/opentype_integration.rs` once the embedded-font
    // fixture extraction path lands.

    #[test]
    fn invalid_bytes_fail_to_parse() {
        let result = FontFace::from_bytes(b"not a font".to_vec(), 0);
        assert!(matches!(result, Err(FontError::Parse(_))));
    }

    #[test]
    fn empty_bytes_fail_to_parse() {
        let result = FontFace::from_bytes(Vec::new(), 0);
        assert!(matches!(result, Err(FontError::Parse(_))));
    }

    #[test]
    fn font_error_display() {
        let err = FontFace::from_bytes(b"not a font".to_vec(), 0).unwrap_err();
        let s = format!("{err}");
        assert!(s.contains("failed to parse font"), "{s}");
    }

    // T3 (KDD-13): variable font axis API surface tests.
    // These tests use DejaVuSans.ttf (non-variable) to verify the API
    // compiles and behaves correctly on static faces. Variable font
    // axis application tests require a real variable font binary.

    #[ignore = "needs testing/fixtures/fonts/DejaVuSans.ttf (drop fixture into testing/fixtures/ to enable)"]
    #[test]
    fn set_variation_stores_tag_and_value() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testing/fixtures/fonts/DejaVuSans.ttf"
        ))
        .expect("DejaVuSans.ttf");
        let mut face = FontFace::from_bytes(data, 0).unwrap();
        let wght = Tag::from_bytes(b"wght");
        face.set_variation(wght, 700.0);
        // Variations stored: one entry.
        assert_eq!(face.variations.len(), 1);
        assert_eq!(face.variations[0].0, wght);
        assert!((face.variations[0].1 - 700.0).abs() < 1e-6);
    }

    #[ignore = "needs testing/fixtures/fonts/DejaVuSans.ttf (drop fixture into testing/fixtures/ to enable)"]
    #[test]
    fn set_variation_replaces_existing_tag() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testing/fixtures/fonts/DejaVuSans.ttf"
        ))
        .expect("DejaVuSans.ttf");
        let mut face = FontFace::from_bytes(data, 0).unwrap();
        let wght = Tag::from_bytes(b"wght");
        face.set_variation(wght, 400.0);
        face.set_variation(wght, 700.0);
        // Still only one entry for the same tag.
        assert_eq!(face.variations.len(), 1);
        assert!((face.variations[0].1 - 700.0).abs() < 1e-6);
    }

    #[ignore = "needs testing/fixtures/fonts/DejaVuSans.ttf (drop fixture into testing/fixtures/ to enable)"]
    #[test]
    fn set_variation_multiple_distinct_tags() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testing/fixtures/fonts/DejaVuSans.ttf"
        ))
        .expect("DejaVuSans.ttf");
        let mut face = FontFace::from_bytes(data, 0).unwrap();
        face.set_variation(Tag::from_bytes(b"wght"), 700.0);
        face.set_variation(Tag::from_bytes(b"wdth"), 75.0);
        assert_eq!(face.variations.len(), 2);
    }

    #[ignore = "needs testing/fixtures/fonts/DejaVuSans.ttf (drop fixture into testing/fixtures/ to enable)"]
    #[test]
    fn is_variable_false_for_static_font() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testing/fixtures/fonts/DejaVuSans.ttf"
        ))
        .expect("DejaVuSans.ttf");
        let face = FontFace::from_bytes(data, 0).unwrap();
        assert!(!face.is_variable(), "DejaVuSans is a static font");
    }

    #[ignore = "needs testing/fixtures/fonts/DejaVuSans.ttf (drop fixture into testing/fixtures/ to enable)"]
    #[test]
    fn face_applies_stored_variations() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testing/fixtures/fonts/DejaVuSans.ttf"
        ))
        .expect("DejaVuSans.ttf");
        let mut ff = FontFace::from_bytes(data, 0).unwrap();
        // set_variation on a non-variable face is a no-op in ttf-parser
        // but must not panic or corrupt the face.
        ff.set_variation(Tag::from_bytes(b"wght"), 700.0);
        let _face = ff.face(); // must not panic
    }

    // T10 (IP-1): kern_pair_adjustment API surface
    #[ignore = "needs testing/fixtures/fonts/DejaVuSans.ttf (drop fixture into testing/fixtures/ to enable)"]
    #[test]
    fn kern_pair_adjustment_av_is_negative_in_dejavu() {
        // DejaVuSans has a kern table — A-V should be a negative kern
        // (tighten spacing). The exact value is -131 font units (verified
        // empirically), but the test only asserts it is non-zero and negative
        // so it stays valid if the font is regenerated.
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testing/fixtures/fonts/DejaVuSans.ttf"
        ))
        .expect("DejaVuSans.ttf");
        let face = FontFace::from_bytes(data, 0).unwrap();
        let a = face.glyph_index('A');
        let v = face.glyph_index('V');
        if let (Some(ga), Some(gv)) = (a, v) {
            let adj = face.kern_pair_adjustment(ga, gv);
            assert!(adj < 0, "A-V kern should be negative (tighten), got {adj}");
        }
    }

    #[ignore = "needs testing/fixtures/fonts/DejaVuSans.ttf (drop fixture into testing/fixtures/ to enable)"]
    #[test]
    fn kern_pair_adjustment_returns_zero_for_unknown_pair() {
        // Two glyphs that are unlikely to have a kern pair.
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testing/fixtures/fonts/DejaVuSans.ttf"
        ))
        .expect("DejaVuSans.ttf");
        let face = FontFace::from_bytes(data, 0).unwrap();
        // GlyphId 0 is the .notdef glyph — it should have no kern pair.
        let adj = face.kern_pair_adjustment(GlyphId(0), GlyphId(0));
        assert_eq!(adj, 0, ".notdef/.notdef pair should have no kern");
    }
}
