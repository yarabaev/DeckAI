//! Extract `<p:embeddedFontLst>` binaries and emit them as a
//! self-contained SVG `<style>@font-face</style>` block.
//!
//! `PowerPoint` decks may embed the actual TTF / OTF font binaries used
//! to author the slide. When the SVG carries those binaries inline as
//! `data:font/...;base64,…` the `@font-face src:`, **every** SVG viewer
//! — browser, librsvg, resvg with system-fonts off — renders the slide
//! with the deck's authored font. This is the single most reliable
//! way to keep the rendered output close to `PowerPoint`'s view.
//!
//! ## Why a `<style>` element inside SVG?
//!
//! `<style>` inside SVG is part of the spec; browsers honour it for
//! the elements in the same document. resvg parses it via usvg ≥ 0.32.
//! For viewers that don't (e.g. older librsvg), the `<style>` is just
//! ignored and the rest of the font-family fallback chain takes over —
//! no regression.
//!
//! ## Obfuscation
//!
//! `PowerPoint` optionally XOR-obfuscates embedded fonts (the file name
//! ends in `.fntdata` and a 32-byte mask derived from the relationship
//! GUID is XOR'd over the first 32 bytes of the binary). Modern decks
//! emit unobfuscated TTF directly; older ones use obfuscation. We
//! support both — see [`deobfuscate`].

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Write as _,
    sync::Arc,
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use slideglance_model::EmbeddedFont;
use slideglance_parser::{
    parse_relationships, resolve_relationship_target, ArchiveError, PptxArchive, XmlError,
};
use xxhash_rust::xxh3::xxh3_64;

const PRES_PATH: &str = "ppt/presentation.xml";
const PRES_RELS_PATH: &str = "ppt/_rels/presentation.xml.rels";

/// Failures specific to embedded-font extraction. Most errors here are
/// silently logged in the convert pipeline rather than surfaced —
/// missing fonts shouldn't fail an otherwise-valid conversion.
#[derive(Debug)]
pub enum EmbeddedFontError {
    /// Underlying ZIP archive failure.
    Archive(ArchiveError),
    /// `presentation.xml.rels` parse failure.
    Xml(XmlError),
}

impl std::fmt::Display for EmbeddedFontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Archive(e) => write!(f, "embedded font archive error: {e}"),
            Self::Xml(e) => write!(f, "embedded font xml error: {e}"),
        }
    }
}

impl From<ArchiveError> for EmbeddedFontError {
    fn from(e: ArchiveError) -> Self {
        Self::Archive(e)
    }
}

impl From<XmlError> for EmbeddedFontError {
    fn from(e: XmlError) -> Self {
        Self::Xml(e)
    }
}

/// Output format for inlined embedded font binaries.
///
/// Selects how `EmbeddedFontFace::bytes` are serialized into the SVG
/// `@font-face src:` data URL. `Auto` picks WOFF2 when the `woff2`
/// cargo feature is enabled at build time, falling back to TTF / OTF
/// (the original deck bytes) otherwise. `Ttf` and `Woff2` force the
/// respective format regardless of feature flags.
///
/// The actual conversion is performed by D2 (`subset_font_bytes` +
/// WOFF2 encoder); D0 only provides the configuration carrier.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum EmbedFormat {
    /// Pick the best available format at build time.
    #[default]
    Auto,
    /// Force raw TTF / OTF bytes (no transformation).
    Ttf,
    /// Force WOFF2. Requires the `woff2` cargo feature; selecting it
    /// without the feature returns an error from the embed pipeline.
    Woff2,
}

/// Accumulates Unicode codepoints seen per typeface while walking slide runs.
///
/// Used to subset embedded font binaries to only the glyphs actually referenced
/// in the deck, reducing payload size before base64-encoding into SVG.
#[derive(Clone, Debug, Default)]
pub struct UsedCodepoints {
    /// Maps lowercase typeface name → set of codepoints (as `u32`).
    inner: HashMap<String, HashSet<u32>>,
}

impl UsedCodepoints {
    /// Record all codepoints from `text` as used by `typeface`.
    pub fn add_text_for(&mut self, typeface: &str, text: &str) {
        if typeface.is_empty() || text.is_empty() {
            return;
        }
        let key = typeface.to_lowercase();
        let entry = self.inner.entry(key).or_default();
        for ch in text.chars() {
            entry.insert(ch as u32);
        }
    }

    /// Retrieve the codepoint set for `typeface` (case-insensitive lookup).
    /// Returns `None` when no text was recorded for this face.
    #[must_use]
    pub fn get(&self, typeface: &str) -> Option<&HashSet<u32>> {
        self.inner.get(&typeface.to_lowercase())
    }

    /// Returns `true` when no codepoints have been recorded.
    #[allow(dead_code)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// Subset errors returned by [`subset_font_bytes`] and [`convert_to_woff2`].
#[derive(Debug)]
#[allow(dead_code)]
pub enum FontProcessError {
    /// Subsetting failed.
    Subset(String),
    /// WOFF2 compression failed.
    Woff2(String),
}

impl std::fmt::Display for FontProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Subset(msg) => write!(f, "font subset error: {msg}"),
            Self::Woff2(msg) => write!(f, "WOFF2 encode error: {msg}"),
        }
    }
}

impl std::error::Error for FontProcessError {}

/// Subset `font_data` (TTF/OTF) to only the glyphs needed for `codepoints`.
///
/// Returns the original bytes unchanged when `codepoints` is empty (no text
/// referenced this face) — this avoids subsetting overhead and potential
/// cmap truncation for fonts that aren't used in a slide.
///
/// Uses `fontcull::subset_font_data_unicode` which retains the `cmap` table
/// required by browsers for `@font-face` rendering.
///
/// # Errors
/// Returns [`FontProcessError::Subset`] on subsetting failure.
pub fn subset_font_bytes(
    font_data: &[u8],
    codepoints: &HashSet<u32>,
) -> Result<Vec<u8>, FontProcessError> {
    if codepoints.is_empty() {
        return Ok(font_data.to_vec());
    }
    let unicodes: Vec<u32> = codepoints.iter().copied().collect();
    fontcull::subset_font_data_unicode(font_data, &unicodes, &[])
        .map_err(|e| FontProcessError::Subset(e.to_string()))
}

/// Compress `ttf_data` to WOFF2 using `fontcull`'s built-in brotli encoder.
///
/// Only available when the `woff2` cargo feature is enabled. This feature
/// activates `fontcull/woff2` which pulls in `woofwoof` (brotli).
///
/// # Errors
/// Returns [`FontProcessError::Woff2`] on compression failure.
#[cfg(feature = "woff2")]
pub fn compress_font_to_woff2(ttf_data: &[u8]) -> Result<Vec<u8>, FontProcessError> {
    fontcull::compress_to_woff2(ttf_data).map_err(|e| FontProcessError::Woff2(e.to_string()))
}

/// Resolve the output MIME type for an embedded face given the chosen format.
///
/// - `EmbedFormat::Auto` → WOFF2 when `woff2` feature enabled, else TTF
/// - `EmbedFormat::Ttf` → always TTF
/// - `EmbedFormat::Woff2` → always WOFF2
#[must_use]
pub fn resolve_embed_mime(format: EmbedFormat) -> &'static str {
    match format {
        EmbedFormat::Woff2 => "font/woff2",
        EmbedFormat::Ttf => "font/ttf",
        EmbedFormat::Auto => {
            #[cfg(feature = "woff2")]
            {
                "font/woff2"
            }
            #[cfg(not(feature = "woff2"))]
            {
                "font/ttf"
            }
        }
    }
}

/// Deduplication cache for encoded font blobs.
///
/// Identical font binaries (same bytes, different typeface names) produce the
/// same base64 string. This cache stores `Arc<String>` so callers share the
/// allocation rather than cloning multi-kilobyte strings per face.
///
/// Cache key: xxHash3-64 of the raw font bytes. Collision probability over
/// a typical deck (< 100 faces) is negligible.
pub struct FontBytesCache {
    /// Maps xxHash3 digest → shared base64 string.
    map: BTreeMap<u64, Arc<String>>,
}

impl FontBytesCache {
    /// Create an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Look up or insert `bytes`. Returns the shared base64 encoding.
    pub fn get_or_insert(&mut self, bytes: &[u8]) -> Arc<String> {
        let key = xxh3_64(bytes);
        if let Some(existing) = self.map.get(&key) {
            return Arc::clone(existing);
        }
        let encoded = Arc::new(STANDARD.encode(bytes));
        self.map.insert(key, Arc::clone(&encoded));
        encoded
    }
}

impl Default for FontBytesCache {
    fn default() -> Self {
        Self::new()
    }
}

/// One extracted embedded face. Holds the typeface name, the variant
/// (regular / bold / italic / bold-italic), and the binary bytes ready
/// for base64 encoding.
///
/// ## bytes vs `bytes_for_svg`
///
/// `bytes` always contains **TTF / OTF** — the original deobfuscated deck
/// binary. It is safe to pass to `FontFace::from_bytes` / `ttf_parser`
/// which cannot parse WOFF2.
///
/// `bytes_for_svg` contains the **SVG-optimised** encoding chosen for
/// `@font-face`: WOFF2 when the `woff2` feature is active, else equal to
/// `bytes`. Only `render_embedded_font_defs` reads this field.
///
/// Keeping the two fields separate prevents the WOFF2-silent-drop regression
/// where WOFF2 bytes in `bytes` cause every embedded face to be silently
/// dropped from `build_embedded_buffer_resolver` and
/// `build_auto_opentype_measurer` (both use `FontFace::from_bytes` which
/// only accepts TTF/OTF, not WOFF2).
#[derive(Clone, Debug)]
pub struct EmbeddedFontFace {
    /// Font family name as declared in `<p:embeddedFont @typeface>`.
    pub typeface: String,
    /// CSS `font-weight` attribute that should be paired with this face.
    pub weight: &'static str,
    /// CSS `font-style` attribute that should be paired with this face.
    pub style: &'static str,
    /// Raw TTF / OTF bytes — always parseable by `ttf_parser`.
    /// De-obfuscated (and subsetted) but never WOFF2-encoded.
    /// Used by `FontFace::from_bytes`, `all_face_family_names`, and
    /// `build_embedded_buffer_resolver`.
    pub bytes: Vec<u8>,
    /// SVG-optimised bytes for the `@font-face src:` data URL. Equals
    /// `bytes` when no WOFF2 encoding was applied; contains WOFF2 when the
    /// `woff2` feature is active. Only `render_embedded_font_defs` reads
    /// this field.
    pub bytes_for_svg: Vec<u8>,
    /// Sniffed MIME type for `bytes_for_svg` (`font/ttf` / `font/woff2`).
    pub mime: &'static str,
    /// Pre-computed base64 encoding of `bytes_for_svg`, shared via
    /// `Arc<String>` so identical font binaries share one allocation.
    /// `None` until `build_embedded_face_list` populates it via
    /// [`FontBytesCache`].
    pub base64_cache: Option<Arc<String>>,
    /// Additional family-name aliases the font file's `name` table exposes
    /// (localized names, typographic family, PostScript name). Populated by
    /// `build_embedded_face_list` via `all_face_family_names`. Used by
    /// `render_embedded_font_defs` to emit one `@font-face` per alias so
    /// text-mode browsers match runs that reference localized family names.
    pub aliases: Vec<String>,
}

/// Embedded font face whose payload is `MicroType` Express (MTX)
/// compressed.
///
/// The Rust pipeline can't decompress MTX (the algorithm is a
/// proprietary Agfa Monotype variant of LZ77 + adaptive Huffman), so
/// these faces are NOT registered as TTF parseable bytes — they're
/// surfaced separately so a JS-side decompressor (`mtx-decompressor`
/// in the chrome-extension viewer) can decode them at runtime and
/// register the resulting TTF as a `FontFace`.
///
/// The `payload` field is the inner font-data bytes the EOT header
/// pointed at, with the optional XOR obfuscation already undone. It
/// can be passed straight to `mtx-decompressor`'s
/// `decompressEotFont(payload, /*compressed=*/true, /*encrypted=*/false)`.
#[derive(Clone, Debug)]
pub struct CompressedEmbeddedFont {
    /// Family name as declared in `<p:embeddedFont @typeface>`.
    pub typeface: String,
    /// Inner font-data bytes (post-EOT-header, post-XOR-deobfuscation).
    /// Still MTX-compressed — caller must decompress.
    pub payload: Vec<u8>,
    /// CSS `font-weight` (`"normal"` / `"bold"` / numeric).
    pub weight: &'static str,
    /// CSS `font-style` (`"normal"` / `"italic"`).
    pub style: &'static str,
}

/// Caller-supplied font binary that should be inlined into every
/// rendered SVG as a `@font-face`. Use this when the deck does **not**
/// embed its fonts (most don't) and you want to guarantee that the
/// rendered SVG looks identical regardless of the host's installed
/// fonts.
///
/// Typical usage: pass the actual TTF / OTF byte buffer for every
/// typeface the deck declares (e.g. `Noto Sans JP`, `Yu Gothic`).
/// The SVG carries the data URL inline so any viewer that honours
/// `<style>` inside SVG (browsers, recent resvg/usvg) renders with
/// exactly that font.
#[derive(Clone, Debug)]
pub struct AdditionalFont {
    /// CSS `font-family` value the face will be registered as. Should
    /// match the typeface name the deck uses (Latin spelling — the
    /// browser does CSS-string matching, so spelling matters).
    pub typeface: String,
    /// CSS `font-weight` (`"normal"`, `"bold"`, or a numeric weight as
    /// a string like `"300"`). Defaults to `"normal"` via
    /// [`AdditionalFont::regular`].
    pub weight: String,
    /// CSS `font-style` (`"normal"` or `"italic"`).
    pub style: String,
    /// Raw font file bytes.
    pub bytes: Vec<u8>,
}

impl AdditionalFont {
    /// Wrap a font binary as the regular (normal weight, normal style)
    /// face for `typeface`.
    #[must_use]
    pub fn regular(typeface: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            typeface: typeface.into(),
            weight: "normal".to_string(),
            style: "normal".to_string(),
            bytes,
        }
    }

    /// Wrap a font binary as a specific weight / style variant.
    #[must_use]
    pub fn variant(
        typeface: impl Into<String>,
        weight: impl Into<String>,
        style: impl Into<String>,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            typeface: typeface.into(),
            weight: weight.into(),
            style: style.into(),
            bytes,
        }
    }
}

/// Convert a caller-supplied [`AdditionalFont`] list into the same
/// face shape `extract_embedded_faces` produces. The two collections
/// then share the `@font-face` rendering path.
#[must_use]
pub fn additional_to_faces(additional: &[AdditionalFont]) -> Vec<EmbeddedFontFace> {
    additional
        .iter()
        .map(|f| {
            let mime = sniff_mime(&f.typeface, &f.bytes);
            EmbeddedFontFace {
                typeface: f.typeface.clone(),
                // Leak to `&'static` is unnecessary: clone via Box::leak
                // would create unbounded growth. Instead pre-coerce the
                // most common values to `'static` strings.
                weight: leak_static_weight(&f.weight),
                style: leak_static_style(&f.style),
                bytes: f.bytes.clone(),
                bytes_for_svg: f.bytes.clone(),
                mime,
                base64_cache: None,
                aliases: Vec::new(),
            }
        })
        .collect()
}

fn leak_static_weight(s: &str) -> &'static str {
    // Coerce to a small set of `'static` strings so the helper
    // signature can stay simple. Empty / unknown values fall back to
    // the CSS default `"normal"` — callers needing exotic weights
    // should register the font under a separate typeface name.
    match s {
        "bold" => "bold",
        "100" => "100",
        "200" => "200",
        "300" => "300",
        "400" => "400",
        "500" => "500",
        "600" => "600",
        "700" => "700",
        "800" => "800",
        "900" => "900",
        _ => "normal",
    }
}

fn leak_static_style(s: &str) -> &'static str {
    match s {
        "italic" => "italic",
        "oblique" => "oblique",
        _ => "normal",
    }
}

/// Walk every `<p:embeddedFont>` entry and split each face into one of
/// two buckets: ready-to-use TTF/OTF binaries (returned to callers via
/// the `Vec<EmbeddedFontFace>` collection), and `MicroType` Express
/// compressed payloads that need a JS-side decompressor before they
/// can be turned into `FontFaces`.
///
/// Returns an empty vector when the deck has no embedded fonts. Faces
/// whose relationship target is missing or unreadable are skipped
/// silently.
///
/// # Errors
///
/// [`EmbeddedFontError::Archive`] on ZIP failure,
/// [`EmbeddedFontError::Xml`] on `presentation.xml.rels` parse failure.
pub fn extract_embedded_fonts_split(
    archive: &mut PptxArchive,
    embedded_fonts: &[EmbeddedFont],
) -> Result<(Vec<EmbeddedFontFace>, Vec<CompressedEmbeddedFont>), EmbeddedFontError> {
    if embedded_fonts.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let pres_rels = archive
        .xml(PRES_RELS_PATH)
        .map(parse_relationships)
        .transpose()?
        .unwrap_or_default();
    if pres_rels.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut ready = Vec::new();
    let mut compressed = Vec::new();
    for font in embedded_fonts {
        for variant in font_variants(font) {
            let Some(r_id) = variant.r_id else { continue };
            let Some(rel) = pres_rels.get(r_id) else {
                continue;
            };
            let target = resolve_relationship_target(PRES_PATH, &rel.target);
            let Some(bytes) = archive.read_bytes(&target)? else {
                continue;
            };
            let cleaned = deobfuscate(bytes.clone(), &target);
            // Try the fast path first: parse the EOT header and check
            // whether the inner payload starts with a valid sfntVersion.
            // When the payload is uncompressed TTF/OTF the wrapper-strip
            // succeeds and we register the bytes as a ready face. When
            // it returns None the inner stream is MTX-compressed — keep
            // the raw payload so a JS-side decompressor can rescue it
            // (the chrome-extension viewer wires this up via
            // `mtx-decompressor`).
            match strip_eot_wrapper(cleaned.clone()) {
                Some(stripped) => {
                    let mime = sniff_mime(&target, &stripped);
                    ready.push(EmbeddedFontFace {
                        typeface: font.typeface.clone(),
                        weight: variant.weight,
                        style: variant.style,
                        bytes_for_svg: stripped.clone(),
                        bytes: stripped,
                        mime,
                        base64_cache: None,
                        aliases: Vec::new(),
                    });
                }
                None => {
                    if let Some(payload) = extract_eot_payload(&cleaned) {
                        compressed.push(CompressedEmbeddedFont {
                            typeface: font.typeface.clone(),
                            payload,
                            weight: variant.weight,
                            style: variant.style,
                        });
                    }
                }
            }
        }
    }
    Ok((ready, compressed))
}

/// Backwards-compatible wrapper that yields only the ready-to-use
/// TTF/OTF faces, mirroring the original `extract_embedded_faces`
/// behaviour. New code should call [`extract_embedded_fonts_split`]
/// directly so MTX-compressed faces aren't silently dropped.
///
/// # Errors
///
/// Same as [`extract_embedded_fonts_split`].
pub fn extract_embedded_faces(
    archive: &mut PptxArchive,
    embedded_fonts: &[EmbeddedFont],
) -> Result<Vec<EmbeddedFontFace>, EmbeddedFontError> {
    let (ready, _compressed) = extract_embedded_fonts_split(archive, embedded_fonts)?;
    Ok(ready)
}

/// Pulls just the inner font-data bytes out of an EOT wrapper without
/// validating their format. Returns `None` only when the EOT header
/// itself is malformed (size fields lying about the buffer length,
/// magic mismatch).
///
/// Callers that need to distinguish "uncompressed TTF" from
/// "MTX-compressed payload" should still go through
/// [`strip_eot_wrapper`] for the validating path.
fn extract_eot_payload(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < EOT_OFFSET_FLAGS + 4 {
        return None;
    }
    let eot_size = u32::from_le_bytes(
        bytes[EOT_OFFSET_EOT_SIZE..EOT_OFFSET_EOT_SIZE + 4]
            .try_into()
            .ok()?,
    ) as usize;
    let font_data_size = u32::from_le_bytes(
        bytes[EOT_OFFSET_FONT_DATA_SIZE..EOT_OFFSET_FONT_DATA_SIZE + 4]
            .try_into()
            .ok()?,
    ) as usize;
    let flags = u32::from_le_bytes(
        bytes[EOT_OFFSET_FLAGS..EOT_OFFSET_FLAGS + 4]
            .try_into()
            .ok()?,
    );
    if eot_size > bytes.len() || font_data_size > eot_size {
        return None;
    }
    let header_size = eot_size - font_data_size;
    if header_size + font_data_size > bytes.len() {
        return None;
    }
    let mut payload = bytes[header_size..header_size + font_data_size].to_vec();
    if flags & EOT_FLAG_XOR_ENCRYPT_DATA != 0 {
        for byte in &mut payload {
            *byte ^= EOT_XOR_KEY;
        }
    }
    Some(payload)
}

/// Build an SVG `<defs><style>…</style></defs>` block declaring one
/// `@font-face` per supplied face. Returns an empty string when
/// `faces` is empty (no `<defs>` is emitted).
///
/// The block is intended to be inlined into the SVG document right
/// after the `<svg …>` opening tag — the `slide::render_slide_to_svg`
/// caller already concatenates a `<defs>` block in that position.
///
/// Uses `face.base64_cache` when available (pre-computed by
/// `build_embedded_face_list` via [`FontBytesCache`]), falling back to
/// encoding `face.bytes_for_svg` on the fly only when the cache field is absent.
///
/// Emits one `@font-face` for the primary typeface name, plus one for each
/// alias in `face.aliases` (localized names, typographic family, PostScript
/// name). All declarations share the same `base64` payload so the per-alias
/// overhead is only the small CSS declaration text. This ensures text-mode
/// browsers resolve runs that reference localized family names to the same
/// embedded face rather than falling back to OS fonts.
#[must_use]
pub fn render_embedded_font_defs(faces: &[EmbeddedFontFace]) -> String {
    if faces.is_empty() {
        return String::new();
    }
    let mut css = String::with_capacity(faces.len() * 4096);
    for face in faces {
        let b64_owned: String;
        let b64: &str = if let Some(cached) = &face.base64_cache {
            cached.as_str()
        } else {
            b64_owned = STANDARD.encode(&face.bytes_for_svg);
            &b64_owned
        };
        let format = mime_to_format(face.mime);
        // Emit one @font-face for the primary name …
        let _ = write!(
            css,
            "@font-face{{font-family:'{name}';font-weight:{weight};font-style:{style};src:url(data:{mime};base64,{b64}) format('{format}');}}",
            name = escape_css_string(&face.typeface),
            weight = face.weight,
            style = face.style,
            mime = face.mime,
        );
        // … and one for each localized / typographic alias (Fix 7).
        for alias in &face.aliases {
            if alias == &face.typeface {
                continue;
            }
            let _ = write!(
                css,
                "@font-face{{font-family:'{name}';font-weight:{weight};font-style:{style};src:url(data:{mime};base64,{b64}) format('{format}');}}",
                name = escape_css_string(alias),
                weight = face.weight,
                style = face.style,
                mime = face.mime,
            );
        }
    }
    format!("<defs><style type=\"text/css\">{css}</style></defs>")
}

#[derive(Clone, Copy)]
struct Variant<'a> {
    r_id: Option<&'a str>,
    weight: &'static str,
    style: &'static str,
}

fn font_variants(font: &EmbeddedFont) -> [Variant<'_>; 4] {
    [
        Variant {
            r_id: font.regular_r_id.as_deref(),
            weight: "normal",
            style: "normal",
        },
        Variant {
            r_id: font.bold_r_id.as_deref(),
            weight: "bold",
            style: "normal",
        },
        Variant {
            r_id: font.italic_r_id.as_deref(),
            weight: "normal",
            style: "italic",
        },
        Variant {
            r_id: font.bold_italic_r_id.as_deref(),
            weight: "bold",
            style: "italic",
        },
    ]
}

/// `PowerPoint` optionally XOR-obfuscates `.fntdata` payloads. The
/// obfuscation key is derived from the GUID encoded in the file name
/// (32 hex chars between braces). The first 32 bytes of the binary
/// are XOR'd with two reversed 16-byte halves of the key. Files whose
/// name doesn't carry a GUID (e.g. plain `font1.ttf`) pass through
/// untouched.
///
/// # Algorithm (interpretation A — ECMA-376 §14.2.5)
///
/// GUID = 16 bytes parsed from the 32 hex digits. Key = GUID repeated
/// twice (32 bytes total). XOR pattern:
///   bytes[0..16]  ^= key[31-i] for i in 0..16  (= reversed second half)
///   bytes[16..32] ^= key[15-i] for i in 0..16  (= reversed first half)
///
/// UNVERIFIED — must validate against {GUID}.fntdata fixture + `LibreOffice`
/// cross-check before final freeze. See OD-1 in
/// .plans/02-planning-font-pipeline/domain-2-embedded-plan.md.
#[must_use]
pub fn deobfuscate(mut bytes: Vec<u8>, target_path: &str) -> Vec<u8> {
    let Some(key) = extract_guid_key(target_path) else {
        return bytes;
    };
    let len = bytes.len();
    if len < 16 {
        return bytes;
    }
    // key is [u8; 32]: GUID (16 bytes) repeated twice.
    // First 16 bytes of payload XOR with reversed second half of key (key[31-i]).
    for i in 0..16 {
        bytes[i] ^= key[31 - i];
    }
    // Second 16 bytes of payload (when present) XOR with reversed first half (key[15-i]).
    if len >= 32 {
        for i in 0..16 {
            bytes[16 + i] ^= key[15 - i];
        }
    }
    bytes
}

/// Pull a 32-byte key (GUID bytes repeated twice) out of a `{guid}`
/// segment in `path`. Returns `None` when no `{…}` brace pair is
/// present or the contents aren't 32 hex digits.
fn extract_guid_key(path: &str) -> Option<[u8; 32]> {
    let lbrace = path.rfind('{')?;
    let rbrace = path[lbrace..].find('}')? + lbrace;
    let inside = &path[lbrace + 1..rbrace];
    let hex: String = inside.chars().filter(char::is_ascii_hexdigit).collect();
    if hex.len() != 32 {
        return None;
    }
    let mut guid = [0_u8; 16];
    for i in 0..16 {
        guid[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    // Interpretation A: double the 16-byte GUID to a 32-byte key.
    let mut out = [0_u8; 32];
    out[..16].copy_from_slice(&guid);
    out[16..].copy_from_slice(&guid);
    Some(out)
}

/// EOT (Embedded OpenType) `MagicNumber` field value. Always 0x504C
/// (`'P'`, `'L'` little-endian) at offset 34 — the only reliable
/// indicator that a binary is EOT-wrapped rather than raw TTF / OTF.
const EOT_MAGIC_NUMBER: u16 = 0x504C;

/// EOT header field offsets, fixed across every spec version we
/// realistically encounter (1.0 / 2.0 / 2.0.1 / 2.0.2). Modern
/// `PowerPoint` exports use 2.0.2.
const EOT_OFFSET_EOT_SIZE: usize = 0;
const EOT_OFFSET_FONT_DATA_SIZE: usize = 4;
const EOT_OFFSET_FLAGS: usize = 12;
const EOT_OFFSET_MAGIC: usize = 34;

/// `Flags` field bits we act on. The full bit allocation is documented
/// in the EOT spec, but only XOR-encryption matters for the actual
/// font payload — every other flag is metadata.
#[allow(dead_code)] // kept for documentation parity with the EOT spec
const EOT_FLAG_TT_COMPRESSED: u32 = 0x0000_0004;
const EOT_FLAG_XOR_ENCRYPT_DATA: u32 = 0x1000_0000;

/// Fixed XOR key used by EOT's `XORENCRYPTDATA` mode. The MS spec
/// defines this as the literal byte `'P'` (0x50).
const EOT_XOR_KEY: u8 = 0x50;

/// Verify a buffer begins with one of the recognised TrueType /
/// OpenType / collection magic numbers. Used to decide whether an
/// EOT payload is usable raw font data or actually MTX-compressed
/// (in which case our extractor can't recover it without a heavy
/// codec). We treat OTF, TTF, Apple `true`, Type 1 `typ1`, and
/// `ttcf` as valid; anything else fails fast.
fn has_valid_font_magic(bytes: &[u8]) -> bool {
    if bytes.len() < 4 {
        return false;
    }
    matches!(
        &bytes[..4],
        [0x00, 0x01, 0x00, 0x00]    // TrueType
            | b"OTTO"               // CFF OpenType
            | b"true"               // Apple TrueType
            | b"typ1"               // Apple Type 1
            | b"ttcf" // TrueType Collection
    )
}

/// Strip the EOT (Embedded OpenType) wrapper that `PowerPoint` puts
/// around every `.fntdata` binary, returning the inner TTF / OTF
/// payload that browsers and resvg can actually decode.
///
/// Layout (little-endian throughout):
///
/// ```text
/// 0..4    EOTSize         total file length (header + payload)
/// 4..8    FontDataSize    payload length
/// 8..12   Version         0x00010000 / 0x00020001 / 0x00020002
/// 12..16  Flags           bitfield: TTCOMPRESSED (0x4), XORENCRYPT (0x10000000), …
/// 16..26  PANOSE          (10 bytes — irrelevant here)
/// 26..34  Charset / Italic / Weight / fsType
/// 34..36  MagicNumber     0x504C — sentinel, never changes
/// 36..    variable-length name strings + payload at EOTSize-FontDataSize
/// ```
///
/// Returns:
///
/// - `Some(bytes)` when no EOT magic is present — the input is
///   already raw TTF / OTF and the downstream MIME sniffer will
///   take it from here.
/// - `Some(payload)` when the EOT was successfully unwrapped and
///   the payload's first 4 bytes match a recognised sfntVersion
///   (`0x00010000`, `OTTO`, `true`, `typ1`, `ttcf`). The
///   `TTCOMPRESSED` flag is **intentionally ignored** — real
///   `PowerPoint` exports often set the flag on uncompressed
///   payloads, so we trust the magic-number probe rather than the
///   flag bit.
/// - `None` when the post-strip payload doesn't start with a
///   recognised font magic. This means the data is genuinely
///   compressed (`MicroType` Express) or corrupt; the caller drops
///   the face entirely so the SVG never emits a `@font-face` that
///   the browser would log a noisy decode-failure warning for.
///
/// `XORENCRYPTDATA` is honoured before validation — payloads with
/// the flag are XOR-decrypted with the fixed key 0x50 first, then
/// the magic check decides whether the result is usable.
fn strip_eot_wrapper(bytes: Vec<u8>) -> Option<Vec<u8>> {
    if bytes.len() < EOT_OFFSET_MAGIC + 2 {
        return Some(bytes);
    }
    // Magic check first — every other field is meaningless if this
    // doesn't match, and the input is presumably already raw TTF.
    let magic = u16::from_le_bytes([bytes[EOT_OFFSET_MAGIC], bytes[EOT_OFFSET_MAGIC + 1]]);
    if magic != EOT_MAGIC_NUMBER {
        return Some(bytes);
    }

    let eot_size = u32::from_le_bytes([
        bytes[EOT_OFFSET_EOT_SIZE],
        bytes[EOT_OFFSET_EOT_SIZE + 1],
        bytes[EOT_OFFSET_EOT_SIZE + 2],
        bytes[EOT_OFFSET_EOT_SIZE + 3],
    ]) as usize;
    let font_data_size = u32::from_le_bytes([
        bytes[EOT_OFFSET_FONT_DATA_SIZE],
        bytes[EOT_OFFSET_FONT_DATA_SIZE + 1],
        bytes[EOT_OFFSET_FONT_DATA_SIZE + 2],
        bytes[EOT_OFFSET_FONT_DATA_SIZE + 3],
    ]) as usize;
    let flags = u32::from_le_bytes([
        bytes[EOT_OFFSET_FLAGS],
        bytes[EOT_OFFSET_FLAGS + 1],
        bytes[EOT_OFFSET_FLAGS + 2],
        bytes[EOT_OFFSET_FLAGS + 3],
    ]);

    if font_data_size == 0 || eot_size < font_data_size || eot_size > bytes.len() {
        // Malformed wrapper — the bytes claim to be EOT (magic
        // matches) but the size fields are inconsistent. Refuse to
        // emit anything; treating these as raw TTF is just as
        // broken and would still trigger a browser warning.
        return None;
    }

    let header_size = eot_size - font_data_size;
    if header_size + font_data_size > bytes.len() {
        return None;
    }

    let mut payload = bytes[header_size..header_size + font_data_size].to_vec();

    if flags & EOT_FLAG_XOR_ENCRYPT_DATA != 0 {
        for byte in &mut payload {
            *byte ^= EOT_XOR_KEY;
        }
    }

    // The `TTCOMPRESSED` flag in PPTX exports is unreliable — some
    // writers set it even when the payload is plain TTF. Trust the
    // magic-number probe instead: if the bytes after the header
    // start with a known sfntVersion the EOT was wrapping
    // uncompressed data; otherwise it's genuine MicroType Express
    // compression that we don't decode (and shouldn't ship to the
    // browser as a broken `@font-face`).
    if has_valid_font_magic(&payload) {
        Some(payload)
    } else {
        None
    }
}

fn sniff_mime(path: &str, bytes: &[u8]) -> &'static str {
    // Check the trailing extension via case-insensitive equality on
    // the last segment after `.`. Avoids the `case_sensitive_file_
    // extension_comparisons` lint while keeping the logic explicit.
    let ext = path.rsplit('.').next().map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("woff2") => return "font/woff2",
        Some("woff") => return "font/woff",
        Some("otf") => return "font/otf",
        _ => {}
    }
    // OTF magic = "OTTO" (4F54544F); TTF magic = 0x00010000 or 'true' / 'typ1'.
    if bytes.len() >= 4 && &bytes[..4] == b"OTTO" {
        return "font/otf";
    }
    "font/ttf"
}

fn mime_to_format(mime: &str) -> &'static str {
    match mime {
        "font/woff2" => "woff2",
        "font/woff" => "woff",
        "font/otf" => "opentype",
        _ => "truetype",
    }
}

/// Escape a font-family name for safe inclusion in a single-quoted CSS
/// string literal inside an SVG `<style>` element.
///
/// Beyond CSS string escaping (`\` and `'`), we also hex-escape characters
/// that could break out of the `<style>` block into raw HTML/SVG context:
/// `<`, `>`, `/`, and `"`. An attacker-controlled typeface of the form
/// `</style><script>alert(1)</script>` would otherwise escape the `<style>`
/// element and execute arbitrary script — same severity as the inline-SVG
/// XSS blocked by KDD-25.
///
/// Hex-escaping uses CSS `\HHHHHH ` notation (six hex digits + space
/// terminator as required by CSS 2.1 §4.1.3). This is universally
/// supported and does not alter the rendered font-family name as seen
/// by the browser's CSS engine.
fn escape_css_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            // Hex-escape characters that could break out of a <style> block.
            '<' | '>' | '/' | '"' => {
                let _ = write!(out, "\\{:06X} ", ch as u32);
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_faces_yields_empty_defs() {
        assert_eq!(render_embedded_font_defs(&[]), "");
    }

    #[test]
    fn embed_format_default_is_auto() {
        assert_eq!(EmbedFormat::default(), EmbedFormat::Auto);
    }

    #[test]
    fn embed_format_variants_exhaustive() {
        // Compile-time confirmation that the enum carries exactly the
        // three variants mandated by IC-4 (Auto / Ttf / Woff2). Any new
        // variant or rename will fail to match here.
        let fmt = EmbedFormat::Auto;
        let _ = match fmt {
            EmbedFormat::Auto => 0,
            EmbedFormat::Ttf => 1,
            EmbedFormat::Woff2 => 2,
        };
    }

    #[test]
    fn face_emits_defs_with_data_url() {
        let face = EmbeddedFontFace {
            typeface: "Test".to_string(),
            weight: "normal",
            style: "normal",
            bytes: vec![0u8; 16],
            bytes_for_svg: vec![0u8; 16],
            mime: "font/ttf",
            base64_cache: None,
            aliases: Vec::new(),
        };
        let svg = render_embedded_font_defs(&[face]);
        assert!(svg.starts_with("<defs><style"));
        assert!(svg.contains("@font-face"));
        assert!(svg.contains("font-family:'Test'"));
        assert!(svg.contains("font-weight:normal"));
        assert!(svg.contains("src:url(data:font/ttf;base64,"));
        assert!(svg.contains("format('truetype')"));
    }

    #[test]
    fn css_escapes_quotes_and_backslash() {
        assert_eq!(escape_css_string("a\\b"), "a\\\\b");
        assert_eq!(escape_css_string("foo'bar"), "foo\\'bar");
    }

    #[test]
    fn css_escape_blocks_style_breakout_xss() {
        // A typeface name crafted to break out of a <style> block must be
        // hex-escaped so the resulting SVG <defs><style> stays well-formed.
        let payload = "</style><script>alert(1)</script>";
        let escaped = escape_css_string(payload);
        // The escaped result must NOT contain the raw < > / characters.
        assert!(!escaped.contains('<'), "< must be hex-escaped");
        assert!(!escaped.contains('>'), "> must be hex-escaped");
        assert!(
            !escaped.contains("</style>"),
            "</style> breakout must be blocked"
        );
        // The escaped string must be valid CSS hex escapes (\\HHHHHH format).
        assert!(escaped.contains("\\00003C"), "< should be \\00003C");
        assert!(escaped.contains("\\00003E"), "> should be \\00003E");
    }

    #[test]
    fn css_escape_blocks_double_quote_xss() {
        let payload = r#"x" onerror="alert(1)"#;
        let escaped = escape_css_string(payload);
        assert!(!escaped.contains('"'), "\" must be hex-escaped");
        assert!(escaped.contains("\\000022"), "\" should be \\000022");
    }

    #[test]
    fn css_escape_normal_typeface_unchanged() {
        // Typical font-family names should pass through without modification.
        assert_eq!(escape_css_string("Noto Sans KR"), "Noto Sans KR");
        assert_eq!(escape_css_string("Yu Gothic"), "Yu Gothic");
        assert_eq!(escape_css_string("맑은 고딕"), "맑은 고딕");
    }

    #[test]
    fn extract_guid_key_parses_braced_path() {
        let path = "ppt/fonts/{12345678-1234-1234-1234-1234567890AB}.fntdata";
        let key = extract_guid_key(path).expect("guid present");
        // First 16 bytes: GUID bytes
        assert_eq!(key[0], 0x12);
        assert_eq!(key[15], 0xAB);
        // Second 16 bytes: GUID repeated
        assert_eq!(key[16], 0x12);
        assert_eq!(key[31], 0xAB);
    }

    #[test]
    fn extract_guid_key_rejects_non_guid() {
        assert!(extract_guid_key("ppt/fonts/font1.ttf").is_none());
        assert!(extract_guid_key("ppt/fonts/{not-a-guid}").is_none());
    }

    #[test]
    fn deobfuscate_no_guid_passthrough() {
        let bytes = vec![0xAB; 64];
        let out = deobfuscate(bytes.clone(), "ppt/fonts/font1.ttf");
        assert_eq!(out, bytes);
    }

    #[test]
    fn deobfuscate_short_buffer_passthrough() {
        let bytes = vec![0u8; 8];
        let out = deobfuscate(bytes.clone(), "{12345678-1234-1234-1234-1234567890AB}");
        assert_eq!(out, bytes);
    }

    #[test]
    fn sniff_mime_detects_otto() {
        assert_eq!(sniff_mime("foo.bin", b"OTTO\x00\x00\x00"), "font/otf");
    }

    /// Build a minimal EOT 2.0.2 header followed by `payload`, with
    /// the header padded out to `header_size` bytes. Used by the EOT
    /// strip tests below — real EOT headers have variable-length
    /// name strings after offset 36 so we can't write a fixed-size
    /// header literal.
    fn build_eot(payload: &[u8], header_size: usize, flags: u32) -> Vec<u8> {
        assert!(header_size >= EOT_OFFSET_MAGIC + 2);
        let total = header_size + payload.len();
        let mut out = vec![0u8; header_size];
        // EOTSize = total file size
        out[0..4].copy_from_slice(&u32::try_from(total).unwrap().to_le_bytes());
        // FontDataSize = payload size
        out[4..8].copy_from_slice(&u32::try_from(payload.len()).unwrap().to_le_bytes());
        // Version = 0x00020002 (EOT 2.0.2)
        out[8..12].copy_from_slice(&0x0002_0002u32.to_le_bytes());
        // Flags
        out[12..16].copy_from_slice(&flags.to_le_bytes());
        // Magic
        out[34..36].copy_from_slice(&EOT_MAGIC_NUMBER.to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn strip_eot_passthrough_when_no_magic() {
        // No 0x504C at offset 34 — caller will treat the bytes as
        // already-raw font data. Pad to >= 36 bytes so the offset
        // check doesn't trivially short-circuit.
        let bytes: Vec<u8> = (0..64u8).collect();
        let out = strip_eot_wrapper(bytes.clone()).expect("non-EOT data passes through");
        assert_eq!(out, bytes);
    }

    #[test]
    fn strip_eot_extracts_payload_from_uncompressed_eot() {
        let payload: Vec<u8> = vec![0x00, 0x01, 0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF];
        let wrapped = build_eot(&payload, 100, 0);
        let out = strip_eot_wrapper(wrapped).expect("valid TTF magic recognised");
        assert_eq!(out, payload);
    }

    #[test]
    fn strip_eot_applies_xor_when_encrypt_flag_set() {
        let payload: Vec<u8> = vec![0x50, 0x51, 0x50, 0x50]; // XOR'd 0x00 0x01 0x00 0x00
        let wrapped = build_eot(&payload, 100, EOT_FLAG_XOR_ENCRYPT_DATA);
        let out = strip_eot_wrapper(wrapped).expect("XOR-decrypt yields valid TTF magic");
        // Each byte XOR'd with 0x50 yields the TTF magic.
        assert_eq!(out, vec![0x00, 0x01, 0x00, 0x00]);
    }

    #[test]
    fn strip_eot_returns_none_when_payload_lacks_font_magic() {
        // Payload starts with garbage — magic-number probe must
        // reject this and signal the caller to drop the face.
        let payload: Vec<u8> = vec![0xAA; 16];
        let wrapped = build_eot(&payload, 100, 0);
        assert!(strip_eot_wrapper(wrapped).is_none());
    }

    #[test]
    fn strip_eot_strips_even_when_compressed_flag_set_if_payload_is_valid_ttf() {
        // Real PowerPoint exports sometimes set TTCOMPRESSED on
        // payloads that are actually plain TTF. The magic-number
        // probe must override the flag and recover the font.
        let payload: Vec<u8> = vec![0x00, 0x01, 0x00, 0x00, 0xCA, 0xFE];
        let wrapped = build_eot(&payload, 100, EOT_FLAG_TT_COMPRESSED);
        let out = strip_eot_wrapper(wrapped).expect("valid TTF magic wins over the flag");
        assert_eq!(out, payload);
    }

    #[test]
    fn strip_eot_returns_none_for_genuinely_compressed_payload() {
        // TTCOMPRESSED + payload that isn't a recognised font magic
        // = real MTX compression. We can't decode it, so signal None
        // to drop the face entirely.
        let payload: Vec<u8> = vec![0x12, 0x34, 0x56, 0x78];
        let wrapped = build_eot(&payload, 100, EOT_FLAG_TT_COMPRESSED);
        assert!(strip_eot_wrapper(wrapped).is_none());
    }

    #[test]
    fn strip_eot_returns_short_buffer_unchanged() {
        // Below the magic offset — can't be EOT, pass through.
        let bytes: Vec<u8> = vec![0; 10];
        let out = strip_eot_wrapper(bytes.clone()).expect("short buffer passes through");
        assert_eq!(out, bytes);
    }

    #[test]
    fn strip_eot_rejects_inconsistent_sizes() {
        // EOTSize < FontDataSize would yield a negative header.
        // The wrapper claims EOT magic so we can't pretend it's raw
        // TTF — drop it.
        let mut bytes = build_eot(&[0; 16], 100, 0);
        bytes[0..4].copy_from_slice(&8u32.to_le_bytes());
        assert!(strip_eot_wrapper(bytes).is_none());
    }

    #[test]
    fn has_valid_font_magic_recognises_known_sfnt_versions() {
        assert!(has_valid_font_magic(&[0x00, 0x01, 0x00, 0x00]));
        assert!(has_valid_font_magic(b"OTTO\x00"));
        assert!(has_valid_font_magic(b"true\x00"));
        assert!(has_valid_font_magic(b"typ1\x00"));
        assert!(has_valid_font_magic(b"ttcf\x00"));
        assert!(!has_valid_font_magic(&[0xDE, 0xAD, 0xBE, 0xEF]));
        assert!(!has_valid_font_magic(&[0x00; 3]));
    }

    #[test]
    fn sniff_mime_defaults_to_ttf() {
        assert_eq!(sniff_mime("foo.fntdata", b"\x00\x01\x00\x00"), "font/ttf");
    }

    #[test]
    fn sniff_mime_extension_overrides_for_woff() {
        assert_eq!(sniff_mime("foo.woff2", b""), "font/woff2");
        assert_eq!(sniff_mime("foo.woff", b""), "font/woff");
    }

    #[test]
    fn mime_to_format_round_trip() {
        assert_eq!(mime_to_format("font/ttf"), "truetype");
        assert_eq!(mime_to_format("font/otf"), "opentype");
        assert_eq!(mime_to_format("font/woff"), "woff");
        assert_eq!(mime_to_format("font/woff2"), "woff2");
    }

    // --- Step B: UsedCodepoints, FontBytesCache, subset/WOFF2 wrappers ---

    #[test]
    fn used_codepoints_add_and_get() {
        let mut uc = UsedCodepoints::default();
        uc.add_text_for("Calibri", "Hello");
        let cp = uc
            .get("calibri")
            .expect("case-insensitive lookup must work");
        assert!(cp.contains(&('H' as u32)));
        assert!(cp.contains(&('e' as u32)));
        assert!(!cp.contains(&('z' as u32)));
    }

    #[test]
    fn used_codepoints_case_insensitive_key() {
        let mut uc = UsedCodepoints::default();
        uc.add_text_for("Calibri", "A");
        uc.add_text_for("CALIBRI", "B");
        // Both insertions go to the same bucket.
        let cp = uc.get("calibri").expect("must exist");
        assert!(cp.contains(&('A' as u32)));
        assert!(cp.contains(&('B' as u32)));
    }

    #[test]
    fn used_codepoints_empty_typeface_or_text_is_noop() {
        let mut uc = UsedCodepoints::default();
        uc.add_text_for("", "text");
        uc.add_text_for("Calibri", "");
        assert!(uc.is_empty());
    }

    #[test]
    fn used_codepoints_get_missing_returns_none() {
        let uc = UsedCodepoints::default();
        assert!(uc.get("Calibri").is_none());
    }

    #[test]
    fn font_bytes_cache_dedup_same_bytes() {
        let mut cache = FontBytesCache::new();
        let bytes = b"fakefontdata" as &[u8];
        let r1 = cache.get_or_insert(bytes);
        let r2 = cache.get_or_insert(bytes);
        // Same Arc pointer — no extra allocation for duplicate bytes.
        assert!(Arc::ptr_eq(&r1, &r2));
    }

    #[test]
    fn font_bytes_cache_different_bytes_different_arcs() {
        let mut cache = FontBytesCache::new();
        let a = cache.get_or_insert(b"aaaa");
        let b = cache.get_or_insert(b"bbbb");
        assert!(!Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn font_bytes_cache_encodes_base64() {
        let mut cache = FontBytesCache::new();
        let raw = b"hello";
        let encoded = cache.get_or_insert(raw);
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        assert_eq!(encoded.as_str(), STANDARD.encode(raw));
    }

    #[test]
    fn resolve_embed_mime_ttf() {
        assert_eq!(resolve_embed_mime(EmbedFormat::Ttf), "font/ttf");
    }

    #[test]
    fn resolve_embed_mime_woff2() {
        assert_eq!(resolve_embed_mime(EmbedFormat::Woff2), "font/woff2");
    }

    #[test]
    fn resolve_embed_mime_auto_without_woff2_feature() {
        // When `woff2` feature is off, Auto → TTF.
        // When `woff2` feature is on, Auto → WOFF2.
        let mime = resolve_embed_mime(EmbedFormat::Auto);
        assert!(mime == "font/ttf" || mime == "font/woff2");
    }

    #[test]
    fn subset_font_bytes_empty_codepoints_passthrough() {
        let fake_font = vec![0x00u8, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let codepoints = HashSet::new();
        let result =
            subset_font_bytes(&fake_font, &codepoints).expect("empty codepoints → passthrough");
        assert_eq!(result, fake_font);
    }

    /// B.5 prereq: verify fontcull retains cmap after subsetting.
    /// Requires a real TTF fixture — gated with #[ignore] until one is added.
    #[test]
    #[ignore = "B.5-cmap: needs testing/fixtures/subset-test.ttf — add a minimal real TTF to un-ignore"]
    fn subset_font_bytes_retains_cmap() {
        use std::fs;
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testing/fixtures/subset-test.ttf"
        );
        let font_data = fs::read(fixture).expect("fixture must exist");
        let mut codepoints = HashSet::new();
        codepoints.insert('a' as u32);
        codepoints.insert('b' as u32);
        codepoints.insert('c' as u32);
        let subsetted = subset_font_bytes(&font_data, &codepoints).expect("subset must succeed");
        // Verify cmap table is present by checking the ttf-parser can parse the output.
        let face = ttf_parser::Face::parse(&subsetted, 0)
            .expect("subsetted font must be parseable by ttf-parser");
        // Ensure the kept codepoints have glyphs in the subsetted font.
        for cp in ['a', 'b', 'c'] {
            assert!(
                face.glyph_index(cp).is_some(),
                "codepoint {cp:?} must have a glyph after subsetting"
            );
        }
    }

    // --- Step A: XOR boundary tests ---

    /// B.8: Cache key (xxHash3) and declaration key (typeface×weight×style) must be independent.
    /// Same bytes under different typeface names → same hash, different @font-face declarations.
    #[test]
    fn font_bytes_cache_key_independent_of_declaration_key() {
        let mut cache = FontBytesCache::new();
        let bytes = b"samefontdata";
        // Insert the same byte payload twice (simulating two different faces sharing a binary).
        let b64_a = cache.get_or_insert(bytes);
        let b64_b = cache.get_or_insert(bytes);
        // Cache key is hash-based → same Arc pointer (dedup worked).
        assert!(
            Arc::ptr_eq(&b64_a, &b64_b),
            "identical bytes must share one Arc"
        );
        // The declaration key (typeface×weight×style) is separate from the cache key.
        // Here we just assert the base64 payload is correct for both.
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        assert_eq!(b64_a.as_str(), STANDARD.encode(bytes));
    }

    /// B.11: resvg WOFF2 acceptance stub — OD-3.
    /// Gated on having a real WOFF2 font + a PPTX that uses it.
    #[test]
    #[ignore = "OD-3: resvg WOFF2 acceptance must be verified empirically with a real WOFF2 @font-face — add fixture to un-ignore"]
    fn resvg_accepts_woff2_font_face() {
        // Stub: when this test is un-ignored, build an SVG with a WOFF2
        // @font-face, pass it to slideglance_png::svg_to_png, and assert the PNG
        // is non-empty without error.
    }

    // --- Step A: XOR boundary tests ---
    #[test]
    #[ignore = "OD-1: requires {GUID}.fntdata fixture + LibreOffice cross-check"]
    fn deobfuscate_known_guid_vector() {
        let obfuscated: Vec<u8> = vec![]; // replace with real fixture bytes
        let path = "{12345678-1234-1234-1234-1234567890AB}.fntdata";
        let result = deobfuscate(obfuscated.clone(), path);
        assert!(
            result.starts_with(&[0x00, 0x01, 0x00, 0x00]) || result.starts_with(b"OTTO"),
            "deobfuscated bytes must start with valid font magic"
        );
    }

    /// A.2: len < 16 must pass through unchanged.
    #[test]
    fn deobfuscate_len_less_than_16_passthrough() {
        let path = "{12345678-1234-1234-1234-1234567890AB}.fntdata";
        for len in [0usize, 1, 7, 15] {
            let bytes: Vec<u8> = (0..len as u8).collect();
            let out = deobfuscate(bytes.clone(), path);
            assert_eq!(out, bytes, "len={len} must pass through unchanged");
        }
    }

    /// A.3: len == 24 — only first 16 bytes XOR'd; bytes 16..24 unchanged.
    #[test]
    fn deobfuscate_len_24_xors_first_16_only() {
        // GUID = 00...0001 => guid[15] = 0x01, all others 0x00
        // key[0..16] = guid = [0x00;15, 0x01], key[16..32] = guid repeated
        // First 16 bytes: bytes[i] ^= key[31-i]
        //   i=0: bytes[0] ^= key[31] = 0x01 => 0xFF ^ 0x01 = 0xFE
        //   i=1..15: bytes[i] ^= key[30..16] = 0x00 => unchanged
        let path = "{00000000-0000-0000-0000-000000000001}.fntdata";
        let bytes = vec![0xFFu8; 24];
        let out = deobfuscate(bytes, path);
        assert_eq!(out[0], 0xFE, "byte 0 must be XOR'd with key[31]=0x01");
        for i in 1..16 {
            assert_eq!(
                out[i], 0xFF,
                "byte {i} must be unchanged (key byte is 0x00)"
            );
        }
        // Bytes 16..24: second-half XOR only applies when len >= 32
        for i in 16..24 {
            assert_eq!(
                out[i], 0xFF,
                "byte {i} beyond first block must be unchanged"
            );
        }
    }

    /// A.4: len == 32 — both XOR halves apply.
    #[test]
    fn deobfuscate_len_32_xors_both_halves() {
        // GUID = 00...0001 => key = [0x00;15, 0x01, 0x00;15, 0x01]
        // First half: bytes[0] ^= key[31]=0x01 => 0xFE; others unchanged
        // Second half: bytes[16] ^= key[15]=0x01 => 0xFE; others unchanged
        let path = "{00000000-0000-0000-0000-000000000001}.fntdata";
        let bytes = vec![0xFFu8; 32];
        let out = deobfuscate(bytes, path);
        assert_eq!(out[0], 0xFE, "byte 0 must be XOR'd with key[31]");
        assert_eq!(out[16], 0xFE, "byte 16 must be XOR'd with key[15]");
        for i in [1usize, 14, 15, 17, 30, 31] {
            assert_eq!(
                out[i], 0xFF,
                "byte {i} with zero key byte must be unchanged"
            );
        }
    }

    /// A.5: XOR with same key twice must return original (involution).
    #[test]
    fn deobfuscate_is_idempotent_involution() {
        let path = "{AABBCCDD-EEFF-0011-2233-445566778899}.fntdata";
        let original: Vec<u8> = (0u8..=255).collect();
        let once = deobfuscate(original.clone(), path);
        let twice = deobfuscate(once, path);
        assert_eq!(twice, original, "deobfuscate must be its own inverse");
    }

    /// A.6: Integration stub — gated on testing/fixtures/embed-font-guid.pptx.
    #[test]
    #[ignore = "OD-1: testing/fixtures/embed-font-guid.pptx not yet present — add a PPTX with an obfuscated embedded font to un-ignore"]
    fn integration_extract_guid_obfuscated_font() {
        use std::fs;
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testing/fixtures/embed-font-guid.pptx"
        );
        let bytes = fs::read(fixture).expect("fixture file must exist");
        // Smoke test: extraction must return at least one face with valid font magic.
        // Full implementation in D5 (IP-21).
        let _ = bytes; // placeholder — replace with real extraction call when D5 adds fixture
    }

    // --- Fix 5: bytes vs bytes_for_svg separation ---

    #[test]
    fn render_uses_bytes_for_svg_not_bytes() {
        // render_embedded_font_defs must use bytes_for_svg, not bytes.
        // Simulate the WOFF2 case: bytes = TTF payload, bytes_for_svg = different content.
        let ttf_payload = vec![0xFFu8; 8];
        let svg_payload = vec![0xAAu8; 8];
        let face = EmbeddedFontFace {
            typeface: "MyFont".to_string(),
            weight: "normal",
            style: "normal",
            bytes: ttf_payload.clone(),
            bytes_for_svg: svg_payload.clone(),
            mime: "font/woff2",
            base64_cache: None,
            aliases: Vec::new(),
        };
        let svg = render_embedded_font_defs(&[face]);
        let expected_b64 = STANDARD.encode(&svg_payload);
        let unexpected_b64 = STANDARD.encode(&ttf_payload);
        assert!(
            svg.contains(&expected_b64),
            "render must use bytes_for_svg payload"
        );
        assert!(
            !svg.contains(&unexpected_b64),
            "render must NOT use bytes (TTF) payload"
        );
    }

    #[test]
    fn render_uses_base64_cache_over_bytes_for_svg() {
        // When base64_cache is set, render must use it (no re-encoding).
        let cached_b64 = Arc::new("CACHEDPAYLOAD".to_string());
        let face = EmbeddedFontFace {
            typeface: "CachedFont".to_string(),
            weight: "normal",
            style: "normal",
            bytes: vec![0x01u8; 4],
            bytes_for_svg: vec![0x02u8; 4],
            mime: "font/ttf",
            base64_cache: Some(Arc::clone(&cached_b64)),
            aliases: Vec::new(),
        };
        let svg = render_embedded_font_defs(&[face]);
        assert!(
            svg.contains("CACHEDPAYLOAD"),
            "render must use base64_cache when set"
        );
    }

    // --- Fix 7: aliases @font-face emission ---

    #[test]
    fn render_emits_alias_font_face_declarations() {
        let face = EmbeddedFontFace {
            typeface: "Freesentation".to_string(),
            weight: "normal",
            style: "normal",
            bytes: vec![0u8; 4],
            bytes_for_svg: vec![0u8; 4],
            mime: "font/ttf",
            base64_cache: None,
            aliases: vec!["프리젠테이션".to_string(), "Freesentation Bold".to_string()],
        };
        let svg = render_embedded_font_defs(&[face]);
        // Primary name must appear.
        assert!(svg.contains("font-family:'Freesentation'"));
        // Each alias must get its own @font-face declaration.
        assert!(
            svg.contains("font-family:'프리젠테이션'"),
            "Korean alias must be declared"
        );
        assert!(
            svg.contains("font-family:'Freesentation Bold'"),
            "Bold alias must be declared"
        );
        // All three should share the same base64 payload.
        let b64 = STANDARD.encode(vec![0u8; 4]);
        assert_eq!(
            svg.matches(&b64).count(),
            3,
            "all 3 declarations must share the same b64"
        );
    }

    #[test]
    fn render_skips_duplicate_alias_equal_to_typeface() {
        let face = EmbeddedFontFace {
            typeface: "Calibri".to_string(),
            weight: "normal",
            style: "normal",
            bytes: vec![0u8; 4],
            bytes_for_svg: vec![0u8; 4],
            mime: "font/ttf",
            base64_cache: None,
            // "Calibri" is the same as typeface — must be skipped.
            aliases: vec!["Calibri".to_string(), "Calibri Body".to_string()],
        };
        let svg = render_embedded_font_defs(&[face]);
        // "Calibri" must appear exactly once (from typeface, not from alias).
        assert_eq!(
            svg.matches("font-family:'Calibri'").count(),
            1,
            "duplicate alias equal to typeface must not produce a second declaration"
        );
        // "Calibri Body" alias should still appear.
        assert!(svg.contains("font-family:'Calibri Body'"));
    }
}
