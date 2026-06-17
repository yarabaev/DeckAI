//! PPTX (ZIP) archive reader.
//!
//! Mirrors the original spec :
//! XML/`.rels`/Content-Types entries are eagerly extracted into a string map,
//! while `ppt/media/*` entries are kept zipped and decompressed on demand
//! through a per-archive cache.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{Cursor, Read};

use zip::result::ZipError;
use zip::ZipArchive;

const MEDIA_PREFIX: &str = "ppt/media/";
const CONTENT_TYPES_ENTRY: &str = "[Content_Types].xml";

/// Maximum bytes pre-allocated for a single ZIP entry (64 MiB).
/// The actual entry may be larger — `read_to_end` grows the buffer.
/// This only caps the initial allocation hint to prevent OOM from
/// attacker-controlled `uncompressed_size` fields.
const MAX_ENTRY_PREALLOC: usize = 64 * 1024 * 1024;

/// Maximum total decompressed bytes across ALL entries (XML + `read_bytes`) in
/// one archive. Prevents ZIP bomb attacks where many entries sum to enormous
/// sizes. Tracked in `PptxArchive::bytes_read_total`.
const MAX_TOTAL_BYTES: usize = 512 * 1024 * 1024;

/// A read-open `.pptx` archive.
///
/// Construction (via [`PptxArchive::open`]) eagerly decompresses every
/// `*.xml`, `*.rels`, and `[Content_Types].xml` entry as UTF-8. Media entries
/// (`ppt/media/*`) are left zipped and exposed through [`PptxArchive::media`],
/// which decompresses the requested entry on first access and caches the
/// resulting bytes.
pub struct PptxArchive {
    xml_files: HashMap<String, String>,
    media_paths: HashSet<String>,
    archive: ZipArchive<Cursor<Vec<u8>>>,
    media_cache: HashMap<String, Vec<u8>>,
    /// Cumulative decompressed bytes read across `open()` + every `read_bytes()`
    /// call. Enforces `MAX_TOTAL_BYTES` across all entry types.
    bytes_read_total: usize,
}

// Custom Debug to avoid dumping the entire raw archive bytes (Cursor<Vec<u8>>)
// or the full xml/media payloads — only counts and totals are useful in logs.
impl fmt::Debug for PptxArchive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PptxArchive")
            .field("xml_file_count", &self.xml_files.len())
            .field("media_path_count", &self.media_paths.len())
            .field("media_cache_count", &self.media_cache.len())
            .field("bytes_read_total", &self.bytes_read_total)
            .finish()
    }
}

impl PptxArchive {
    /// Opens a PPTX archive from a byte buffer.
    ///
    /// # Errors
    ///
    /// Returns [`ArchiveError::Zip`] if the buffer is not a valid ZIP archive,
    /// [`ArchiveError::Io`] on I/O failures during decompression, or
    /// [`ArchiveError::NotUtf8`] if an entry expected to contain XML is not
    /// valid UTF-8.
    pub fn open(bytes: impl Into<Vec<u8>>) -> Result<Self, ArchiveError> {
        let bytes = bytes.into();
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).map_err(ArchiveError::Zip)?;

        let mut xml_files = HashMap::new();
        let mut media_paths = HashSet::new();

        let names: Vec<String> = archive.file_names().map(str::to_owned).collect();
        let mut total_decompressed: usize = 0;
        for name in names {
            if name.ends_with('/') {
                continue;
            }
            if name.starts_with(MEDIA_PREFIX) {
                media_paths.insert(name);
                continue;
            }
            if !is_textual_entry(&name) {
                continue;
            }
            let mut entry = archive.by_name(&name).map_err(ArchiveError::Zip)?;
            let hint = (entry.size() as usize).min(MAX_ENTRY_PREALLOC);
            let mut buf = Vec::with_capacity(hint);
            entry.read_to_end(&mut buf).map_err(ArchiveError::Io)?;
            total_decompressed = total_decompressed.saturating_add(buf.len());
            if total_decompressed > MAX_TOTAL_BYTES {
                return Err(ArchiveError::TotalSizeLimitExceeded {
                    limit: MAX_TOTAL_BYTES,
                    observed: total_decompressed,
                });
            }
            let text = String::from_utf8(buf).map_err(|e| ArchiveError::NotUtf8 {
                path: name.clone(),
                source: e,
            })?;
            xml_files.insert(name, text);
        }

        Ok(Self {
            xml_files,
            media_paths,
            archive,
            media_cache: HashMap::new(),
            bytes_read_total: total_decompressed,
        })
    }

    /// Returns the textual XML body for `path`, or `None` if no such entry
    /// exists in the archive.
    #[must_use]
    pub fn xml(&self, path: &str) -> Option<&str> {
        self.xml_files.get(path).map(String::as_str)
    }

    /// Returns the full eager XML/rels file map.
    #[must_use]
    pub fn xml_files(&self) -> &HashMap<String, String> {
        &self.xml_files
    }

    /// Returns every known `ppt/media/*` entry path.
    #[must_use]
    pub fn media_paths(&self) -> &HashSet<String> {
        &self.media_paths
    }

    /// Returns the bytes of `path` if it is a known media entry. The first
    /// call decompresses the entry; subsequent calls return the cached bytes.
    ///
    /// Returns `Ok(None)` if `path` is not a known media entry.
    ///
    /// # Errors
    ///
    /// [`ArchiveError::Zip`] / [`ArchiveError::Io`] on decompression failure.
    pub fn media(&mut self, path: &str) -> Result<Option<&[u8]>, ArchiveError> {
        if !self.media_paths.contains(path) {
            return Ok(None);
        }
        if !self.media_cache.contains_key(path) {
            let mut entry = self.archive.by_name(path).map_err(ArchiveError::Zip)?;
            // Cap the initial allocation hint — media entries are large by
            // design (images), so only the pre-alloc is capped, not the read.
            let hint = (entry.size() as usize).min(MAX_ENTRY_PREALLOC);
            let mut buf = Vec::with_capacity(hint);
            entry.read_to_end(&mut buf).map_err(ArchiveError::Io)?;
            self.media_cache.insert(path.to_owned(), buf);
        }
        Ok(self.media_cache.get(path).map(Vec::as_slice))
    }

    /// Returns the raw decompressed bytes of any archive entry — used
    /// for non-XML, non-media files such as embedded font binaries
    /// (`ppt/fonts/font1.fntdata` / `…obfuscatedFont.bin`). Returns
    /// `Ok(None)` when the entry is absent.
    ///
    /// Bytes are returned freshly each call (no cache); the typical
    /// caller reads each font once during SVG generation.
    ///
    /// # Errors
    ///
    /// [`ArchiveError::Zip`] / [`ArchiveError::Io`] on decompression
    /// failure.
    pub fn read_bytes(&mut self, path: &str) -> Result<Option<Vec<u8>>, ArchiveError> {
        match self.archive.by_name(path) {
            Ok(mut entry) => {
                let hint = (entry.size() as usize).min(MAX_ENTRY_PREALLOC);
                let mut buf = Vec::with_capacity(hint);
                entry.read_to_end(&mut buf).map_err(ArchiveError::Io)?;
                self.bytes_read_total = self.bytes_read_total.saturating_add(buf.len());
                if self.bytes_read_total > MAX_TOTAL_BYTES {
                    return Err(ArchiveError::TotalSizeLimitExceeded {
                        limit: MAX_TOTAL_BYTES,
                        observed: self.bytes_read_total,
                    });
                }
                Ok(Some(buf))
            }
            Err(ZipError::FileNotFound) => Ok(None),
            Err(e) => Err(ArchiveError::Zip(e)),
        }
    }
}

// PPTX (OPC) paths are normative-lowercase per ECMA-376 §9.1.2.3, so a
// case-sensitive suffix check is correct.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn is_textual_entry(name: &str) -> bool {
    name == CONTENT_TYPES_ENTRY || name.ends_with(".xml") || name.ends_with(".rels")
}

/// Failure modes when opening or reading from a [`PptxArchive`].
#[derive(Debug)]
pub enum ArchiveError {
    /// Underlying ZIP parser error.
    Zip(ZipError),
    /// I/O failure while decompressing an entry.
    Io(std::io::Error),
    /// An entry expected to contain text was not valid UTF-8.
    NotUtf8 {
        /// The offending entry path.
        path: String,
        /// The UTF-8 conversion error.
        source: std::string::FromUtf8Error,
    },
    /// Archive total decompressed size exceeds the safety limit.
    TotalSizeLimitExceeded {
        /// The configured limit in bytes.
        limit: usize,
        /// The observed total so far when the limit was hit.
        observed: usize,
    },
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zip(e) => write!(f, "zip archive error: {e}"),
            Self::Io(e) => write!(f, "io error reading archive: {e}"),
            Self::NotUtf8 { path, source } => {
                write!(f, "entry {path} is not valid UTF-8: {source}")
            }
            Self::TotalSizeLimitExceeded { limit, observed } => write!(
                f,
                "archive total decompressed size {observed} exceeded limit {limit}"
            ),
        }
    }
}

impl std::error::Error for ArchiveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Zip(e) => Some(e),
            Self::Io(e) => Some(e),
            Self::NotUtf8 { source, .. } => Some(source),
            Self::TotalSizeLimitExceeded { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal ZIP where the central directory's `uncompressed_size`
    /// field is patched to `u32::MAX` (`0xFFFF_FFFF` ~= 4 GiB) while the actual
    /// entry contains only a tiny XML document. This exercises the pre-alloc
    /// cap: without the cap, `Vec::with_capacity(u32::MAX)` would attempt a
    /// 4 GiB allocation.
    ///
    /// On macOS with overcommit, the allocation may succeed (returning a
    /// virtual address without physical pages), but the test completes without
    /// panic regardless — which is the key invariant we're checking.
    fn make_zip_bomb_bytes() -> Vec<u8> {
        use std::io::{Cursor, Write};
        use zip::write::{FileOptions, ZipWriter};
        use zip::CompressionMethod;

        let mut buf = Cursor::new(Vec::new());
        let mut zw = ZipWriter::new(&mut buf);
        let opts: FileOptions<'_, ()> =
            FileOptions::default().compression_method(CompressionMethod::Stored);
        zw.start_file("[Content_Types].xml", opts).unwrap();
        let xml = br#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#;
        zw.write_all(xml).unwrap();
        zw.finish().unwrap();
        let mut bytes = buf.into_inner();

        // Patch central directory: locate the CD signature 0x02014B50 and
        // overwrite the uncompressed_size field at offset +24 (little-endian u32).
        if let Some(pos) = bytes.windows(4).position(|w| w == b"PK\x01\x02") {
            let size_offset = pos + 24;
            if size_offset + 4 <= bytes.len() {
                bytes[size_offset..size_offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            }
        }
        bytes
    }

    /// Build a ZIP with N binary entries under `ppt/fonts/` whose combined
    /// decompressed size just exceeds `MAX_TOTAL_BYTES` (512 MiB). Each entry
    /// contains `chunk_size` bytes of zeros stored uncompressed.
    fn make_font_bomb_archive(n_entries: usize, chunk_size: usize) -> Vec<u8> {
        use std::io::Write;
        use zip::write::{FileOptions, ZipWriter};
        use zip::CompressionMethod;

        let mut buf = std::io::Cursor::new(Vec::new());
        let mut zw = ZipWriter::new(&mut buf);
        let opts: FileOptions<'_, ()> =
            FileOptions::default().compression_method(CompressionMethod::Stored);

        // Minimal Content_Types so open() succeeds.
        zw.start_file("[Content_Types].xml", opts).unwrap();
        let xml = br#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#;
        zw.write_all(xml).unwrap();

        let chunk = vec![0u8; chunk_size];
        for i in 0..n_entries {
            zw.start_file(format!("ppt/fonts/font{i}.fntdata"), opts)
                .unwrap();
            zw.write_all(&chunk).unwrap();
        }
        zw.finish().unwrap();
        buf.into_inner()
    }

    #[test]
    fn read_bytes_cumulative_cap_enforced() {
        // 6 entries × 100 MiB = 600 MiB > MAX_TOTAL_BYTES (512 MiB).
        // Each read_bytes call should accumulate into bytes_read_total and
        // return TotalSizeLimitExceeded before all entries are consumed.
        const CHUNK: usize = 100 * 1024 * 1024; // 100 MiB per font entry
        let bytes = make_font_bomb_archive(6, CHUNK);
        let mut archive = PptxArchive::open(bytes).expect("archive opens");

        let mut total_read: usize = 0;
        let mut hit_limit = false;
        for i in 0..6usize {
            let path = format!("ppt/fonts/font{i}.fntdata");
            match archive.read_bytes(&path) {
                Ok(Some(data)) => total_read += data.len(),
                Ok(None) => {}
                Err(ArchiveError::TotalSizeLimitExceeded { .. }) => {
                    hit_limit = true;
                    break;
                }
                Err(e) => panic!("unexpected error: {e}"),
            }
        }
        assert!(
            hit_limit,
            "expected TotalSizeLimitExceeded after {total_read} bytes"
        );
    }

    #[test]
    fn read_bytes_under_limit_succeeds() {
        // 3 entries × 10 MiB = 30 MiB — well under the 512 MiB cap.
        const CHUNK: usize = 10 * 1024 * 1024;
        let bytes = make_font_bomb_archive(3, CHUNK);
        let mut archive = PptxArchive::open(bytes).expect("archive opens");
        for i in 0..3usize {
            let path = format!("ppt/fonts/font{i}.fntdata");
            let result = archive.read_bytes(&path);
            assert!(result.is_ok(), "expected ok for entry {i}: {result:?}");
        }
    }

    #[test]
    fn zip_bomb_claimed_size_does_not_panic() {
        // A crafted ZIP claiming enormous uncompressed size must return an
        // error gracefully or succeed (if the actual data fits), but must
        // never panic or abort due to an attempted multi-GiB allocation.
        let bytes = make_zip_bomb_bytes();
        let result = PptxArchive::open(bytes);
        match result {
            Ok(_) | Err(_) => {} // either outcome is acceptable
        }
    }

    #[test]
    fn zip_bomb_claimed_size_does_not_allocate_gigabytes() {
        // The pre-allocation cap must prevent Vec::with_capacity from
        // requesting more than MAX_ENTRY_PREALLOC bytes.
        let bytes = make_zip_bomb_bytes();
        let _result = PptxArchive::open(bytes);
        // Test completing without OOM or panic is the assertion.
    }
}
