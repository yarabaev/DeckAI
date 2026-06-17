//! Optional host-system font discovery.
//!
//! Compiled in only when the `system-fonts` cargo feature is enabled.
//! Wraps [`fontdb::Database::load_system_fonts`] and lifts every face
//! the OS reports into a [`BufferFontResolver`] keyed by family name —
//! the same shape the deterministic byte-buffer path uses, so the
//! renderer's resolver chain treats system-discovered faces and
//! caller-supplied buffers uniformly.
//!
//! ## Why this is opt-in
//!
//! Loading host-installed fonts breaks the project's bit-equivalence
//! guarantee — same PPTX bytes will render differently on machines
//! with different fonts installed. Enabling this feature is an
//! intentional trade-off, typically for interactive tools where the
//! user prefers locally available faces over the OSS fallback chain.
//! Library callers and CI pipelines should leave it off (the default).

use std::sync::Arc;

use crate::font_resolver::BufferFontResolver;
use crate::opentype::FontFace;

/// Read every host-installed font face into a `Vec<Vec<u8>>` of raw
/// OpenType blobs — suitable as `measurement_fonts` for
/// `slideglance::PptxDocument::parse`. Faces with unreadable backing
/// storage (database-only entries, missing files) are skipped.
///
/// **Cost:** reads every system-installed font from disk (potentially
/// hundreds on macOS / Windows). On a fresh process this can be
/// hundreds of MB of I/O plus identical-size memory growth. Prefer
/// [`load_system_font_bytes_for_families`] when the caller already
/// knows which families are needed (e.g. extracted from a PPTX) —
/// that variant short-circuits on fontdb's metadata-only index and
/// only reads files for matching faces.
///
/// **Determinism:** disabled. Same caveats as [`load_system_fonts`].
#[must_use]
pub fn load_system_font_bytes() -> Vec<Vec<u8>> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    let mut out: Vec<Vec<u8>> = Vec::new();
    for face_info in db.faces() {
        match &face_info.source {
            fontdb::Source::Binary(arc) => {
                let bytes_ref: &[u8] = (**arc).as_ref();
                out.push(bytes_ref.to_vec());
            }
            fontdb::Source::File(path) => {
                if let Ok(bytes) = std::fs::read(path) {
                    out.push(bytes);
                }
            }
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }
    out
}

/// Read only the host-system font faces whose family name matches one
/// of `families` (case-insensitive, exact-string match). Fast path for
/// `slideglance::PptxDocument::parse`'s `measurement_fonts` argument:
/// extract the typeface names a deck actually references and pass
/// them here instead of slurping the entire OS font collection.
///
/// fontdb is built from a metadata-only scan so unmatched faces never
/// touch disk; only faces with a hit on `families` get
/// [`std::fs::read`] called on their backing file. On a typical PPTX
/// referencing a handful of families the I/O is bounded to a few MB.
///
/// One source file is read at most once even when it backs several
/// matched faces (TTC bundles, Variable Fonts) — the returned `Vec`
/// has unique buffer identity per source path.
///
/// **Determinism:** disabled. Two machines with different installed
/// fonts will produce different byte buffers.
#[must_use]
pub fn load_system_font_bytes_for_families(families: &[&str]) -> Vec<Vec<u8>> {
    if families.is_empty() {
        return Vec::new();
    }
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let needles: std::collections::HashSet<String> = families
        .iter()
        .map(|f| f.trim().to_lowercase())
        .filter(|f| !f.is_empty())
        .collect();
    if needles.is_empty() {
        return Vec::new();
    }

    let mut seen_paths: std::collections::HashSet<std::path::PathBuf> =
        std::collections::HashSet::new();
    let mut out: Vec<Vec<u8>> = Vec::new();
    for face_info in db.faces() {
        let matches = face_info
            .families
            .iter()
            .any(|(name, _)| needles.contains(&name.to_lowercase()))
            || face_info
                .post_script_name
                .to_lowercase()
                .split('-')
                .next()
                .is_some_and(|head| needles.contains(head));
        if !matches {
            continue;
        }
        match &face_info.source {
            fontdb::Source::Binary(arc) => {
                let bytes_ref: &[u8] = (**arc).as_ref();
                out.push(bytes_ref.to_vec());
            }
            fontdb::Source::File(path) => {
                if !seen_paths.insert(path.clone()) {
                    continue;
                }
                if let Ok(bytes) = std::fs::read(path) {
                    out.push(bytes);
                }
            }
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }
    out
}

/// Build a [`BufferFontResolver`] populated with every font face
/// `fontdb::Database::load_system_fonts` reports on the host.
///
/// Faces with no readable bytes (database-only metadata entries) are
/// skipped silently; faces whose bytes fail to parse as TrueType /
/// OpenType are skipped as well. Returns the resolver populated with
/// whatever faces were successfully loaded — the count is reported via
/// [`BufferFontResolver::len`] for callers that want to log it.
///
/// **Determinism:** disabled. The same call on different machines
/// yields different resolvers. See module docs.
#[must_use]
pub fn load_system_fonts() -> BufferFontResolver {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    let mut resolver = BufferFontResolver::new();
    for face_info in db.faces() {
        let bytes: Vec<u8> = match &face_info.source {
            fontdb::Source::Binary(arc) => {
                // Binary sources expose `Arc<dyn AsRef<[u8]> + Send + Sync>`.
                let bytes_ref: &[u8] = (**arc).as_ref();
                bytes_ref.to_vec()
            }
            fontdb::Source::File(path) => match std::fs::read(path) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            },
            // SharedFile is gated on the `memmap` cargo feature in
            // fontdb. We don't enable it (`fs` only); skip the variant
            // here so the match stays exhaustive across feature combos.
            #[allow(unreachable_patterns)]
            _ => continue,
        };
        let Ok(face) = FontFace::from_bytes(bytes, face_info.index) else {
            continue;
        };
        let family = face_info
            .families
            .first()
            .map(|(name, _)| name.clone())
            .or_else(|| face.family_name());
        let Some(family) = family else {
            continue;
        };
        resolver.insert_arc(family, Arc::new(face));
    }
    resolver
}
