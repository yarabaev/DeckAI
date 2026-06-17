//! TrueType Collection (TTC) byte-level helpers.
//!
//! Mirrors. A TTC bundles
//! multiple TTF / OTF faces sharing some tables; opentype.js cannot
//! parse them directly so the spec extracts each face into a
//! standalone TTF buffer before handing it to `opentype.parse`.
//!
//! Rust's `ttf-parser` crate handles TTC natively via the `face_index`
//! parameter on [`crate::FontFace::from_bytes`], so explicit extraction
//! is rarely needed for measurement / shaping. We still provide
//! [`extract_ttc_faces`] for two reasons:
//!
//! 1. Embedding a single face into SVG output (path-mode rendering)
//!    requires standalone TTF bytes — the renderer can't ship a TTC
//!    inside an `<svg>`.
//! 2. Caching individual faces to disk by-name (so different runtime
//!    invocations don't re-extract every time).
//!
//! [`parse_font_data`] is the convenience entry point: takes raw bytes
//! from a PPTX `ppt/fonts/font*.fntdata` blob and returns one or more
//! [`FontFace`]s, auto-detecting TTC.

use std::sync::Arc;

use crate::opentype::{FontError, FontFace};

/// `b"ttcf"` — TTC magic number written at offset 0.
const TTC_TAG: [u8; 4] = *b"ttcf";

/// Returns `true` when `data` is a TTC container (starts with `ttcf`).
#[must_use]
pub fn is_ttc(data: &[u8]) -> bool {
    data.len() >= 4 && data[..4] == TTC_TAG
}

/// Number of faces in `data` if it is a valid TTC, else `None`.
///
/// Validates the TTC has at least the 12-byte header and the per-face
/// offset table, but does not validate each face's table records.
#[must_use]
pub fn ttc_face_count(data: &[u8]) -> Option<u32> {
    if !is_ttc(data) || data.len() < 12 {
        return None;
    }
    let num_fonts = read_u32(data, 8)?;
    let header_end = 12usize.checked_add((num_fonts as usize).checked_mul(4)?)?;
    if data.len() < header_end {
        return None;
    }
    Some(num_fonts)
}

/// Extracts every face in `data` as a standalone TTF/OTF byte vector.
///
/// Returns an empty vector when `data` is not a TTC, has zero faces, or
/// its header is truncated. Individual faces with corrupt table-record
/// offsets are skipped; the rest are still extracted (mirrors the TS
/// per-font try / catch).
#[must_use]
pub fn extract_ttc_faces(data: &[u8]) -> Vec<Vec<u8>> {
    let Some(num_fonts) = ttc_face_count(data) else {
        return Vec::new();
    };
    if num_fonts == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(num_fonts as usize);
    for i in 0..num_fonts {
        let Some(font_offset) = read_u32(data, 12 + (i as usize) * 4) else {
            continue;
        };
        if let Some(extracted) = extract_single_face(data, font_offset as usize) {
            out.push(extracted);
        }
    }
    out
}

/// Extracts the first face in a TTC (the most common need —
/// rendering / measurement only ever uses one variant of a CJK pan-
/// language TTC, see the TS comment on memory consumption).
#[must_use]
pub fn extract_first_ttc_face(data: &[u8]) -> Option<Vec<u8>> {
    let mut faces = extract_ttc_faces(data);
    if faces.is_empty() {
        None
    } else {
        Some(faces.swap_remove(0))
    }
}

/// Parses `data` as either TTF or TTC.
///
/// On a plain TTF / OTF: returns one [`FontFace`].
/// On a TTC: returns one [`FontFace`] per contained face that
/// `ttf-parser` accepts.
///
/// Unlike [`extract_ttc_faces`] which produces raw bytes, this calls
/// [`FontFace::from_bytes`] with the TTC's natively-parsed bytes,
/// using the `face_index` parameter — no re-extraction needed for
/// pure metric / shaping work.
///
/// Accepts any type convertible to `Arc<[u8]>` (`Vec<u8>`, `&[u8]`,
/// `Arc<[u8]>`) so callers that already hold a shared buffer avoid
/// an extra allocation.
pub fn parse_font_data(data: impl Into<Arc<[u8]>>) -> Result<Vec<FontFace>, FontError> {
    let arc: Arc<[u8]> = data.into();
    let vec: Vec<u8> = arc.to_vec();
    if !is_ttc(&vec) {
        return Ok(vec![FontFace::from_bytes(vec, 0)?]);
    }
    let count = ttc_face_count(&vec).unwrap_or(0);
    let mut faces = Vec::with_capacity(count as usize);
    for i in 0..count {
        // ttf-parser handles TTC indexing natively. Skip faces that
        // fail to parse; the rest are usable.
        if let Ok(face) = FontFace::from_bytes(vec.clone(), i) {
            faces.push(face);
        }
    }
    if faces.is_empty() {
        // None of the contained faces parsed — surface the first error.
        FontFace::from_bytes(vec, 0)?;
    }
    Ok(faces)
}

// ---------------------------------------------------------------------------
// Internal — single face byte-level extraction (TS extractSingleFont port).
// ---------------------------------------------------------------------------

fn extract_single_face(data: &[u8], font_offset: usize) -> Option<Vec<u8>> {
    if font_offset.checked_add(12)? > data.len() {
        return None;
    }
    let sf_version = read_u32(data, font_offset)?;
    let num_tables = read_u16(data, font_offset + 4)?;
    if num_tables == 0 {
        return None;
    }

    let table_records_start = font_offset + 12;
    let table_records_end = table_records_start.checked_add((num_tables as usize) * 16)?;
    if table_records_end > data.len() {
        return None;
    }

    // Read each table record + bounds-check the referenced table data.
    let mut tables: Vec<TableRecord> = Vec::with_capacity(num_tables as usize);
    for i in 0..num_tables {
        let rec_offset = table_records_start + (i as usize) * 16;
        let table_offset = read_u32(data, rec_offset + 8)? as usize;
        let table_length = read_u32(data, rec_offset + 12)? as usize;

        if table_offset > data.len() || table_length > data.len() - table_offset {
            return None;
        }

        tables.push(TableRecord {
            tag: read_u32(data, rec_offset)?,
            check_sum: read_u32(data, rec_offset + 4)?,
            offset: table_offset,
            length: table_length,
        });
    }

    // Compute output buffer size.
    let header_size = 12 + (num_tables as usize) * 16;
    let mut data_size = 0usize;
    for table in &tables {
        data_size = data_size.checked_add(align4(table.length))?;
    }
    let total = header_size.checked_add(data_size)?;

    let mut out = vec![0u8; total];

    // Offset table header.
    write_u32(&mut out, 0, sf_version);
    write_u16(&mut out, 4, num_tables);
    let (search_range, entry_selector, range_shift) = calc_offset_table_fields(num_tables);
    write_u16(&mut out, 6, search_range);
    write_u16(&mut out, 8, entry_selector);
    write_u16(&mut out, 10, range_shift);

    // Table records + data, with offsets pointing into the new buffer.
    let mut current_data_offset = header_size;
    for (i, table) in tables.iter().enumerate() {
        let rec_offset = 12 + i * 16;
        write_u32(&mut out, rec_offset, table.tag);
        write_u32(&mut out, rec_offset + 4, table.check_sum);
        // Offsets cannot overflow u32 because we capped lengths via
        // bounds checks above. The TTF spec uses 32-bit offsets.
        write_u32(&mut out, rec_offset + 8, current_data_offset as u32);
        write_u32(&mut out, rec_offset + 12, table.length as u32);

        out[current_data_offset..current_data_offset + table.length]
            .copy_from_slice(&data[table.offset..table.offset + table.length]);

        current_data_offset += align4(table.length);
    }

    Some(out)
}

#[derive(Clone, Copy)]
struct TableRecord {
    tag: u32,
    check_sum: u32,
    offset: usize,
    length: usize,
}

fn align4(n: usize) -> usize {
    (n + 3) & !3
}

fn calc_offset_table_fields(num_tables: u16) -> (u16, u16, u16) {
    let mut search_range: u16 = 16;
    let mut entry_selector: u16 = 0;
    while search_range.saturating_mul(2) <= num_tables.saturating_mul(16) {
        search_range = search_range.saturating_mul(2);
        entry_selector = entry_selector.saturating_add(1);
    }
    let range_shift = num_tables.saturating_mul(16).saturating_sub(search_range);
    (search_range, entry_selector, range_shift)
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let bytes = data.get(offset..offset + 2)?;
    Some(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn write_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn write_u16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a synthetic minimal TTF whose bytes round-trip through
    /// our extractor cleanly. Not a *valid* font (ttf-parser will
    /// reject it) — only the byte structure (header + table records +
    /// table data) is correct, so we can verify byte-level parity.
    ///
    /// 1 table with a 4-byte body. Total size = 12 + 16 + 4 = 32.
    fn synthetic_ttf(family_marker: u8) -> Vec<u8> {
        let mut out = vec![0u8; 32];
        // sfntVersion (TrueType).
        out[..4].copy_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        // numTables = 1.
        out[4..6].copy_from_slice(&1u16.to_be_bytes());
        // searchRange = 16, entrySelector = 0, rangeShift = 0 (numTables=1).
        out[6..8].copy_from_slice(&16u16.to_be_bytes());
        out[8..10].copy_from_slice(&0u16.to_be_bytes());
        out[10..12].copy_from_slice(&0u16.to_be_bytes());
        // Table record: tag = b"test", checksum = 0, offset = 28, length = 4.
        out[12..16].copy_from_slice(b"test");
        out[16..20].copy_from_slice(&0u32.to_be_bytes());
        out[20..24].copy_from_slice(&28u32.to_be_bytes());
        out[24..28].copy_from_slice(&4u32.to_be_bytes());
        // Table data — 4 bytes whose first byte distinguishes faces.
        out[28..32].copy_from_slice(&[family_marker, 0, 0, 0]);
        out
    }

    /// Mirrors.
    fn build_ttc_from_ttfs(ttfs: &[Vec<u8>]) -> Vec<u8> {
        let mut font_infos: Vec<(u32, Vec<TableRecord>)> = Vec::new();
        for buf in ttfs {
            let sf_version = read_u32(buf, 0).unwrap();
            let num_tables = read_u16(buf, 4).unwrap();
            let mut tables = Vec::new();
            for i in 0..num_tables {
                let rec_offset = 12 + (i as usize) * 16;
                tables.push(TableRecord {
                    tag: read_u32(buf, rec_offset).unwrap(),
                    check_sum: read_u32(buf, rec_offset + 4).unwrap(),
                    offset: read_u32(buf, rec_offset + 8).unwrap() as usize,
                    length: read_u32(buf, rec_offset + 12).unwrap() as usize,
                });
            }
            font_infos.push((sf_version, tables));
        }

        let ttc_header_size = 12 + ttfs.len() * 4;
        let font_header_sizes: Vec<usize> =
            font_infos.iter().map(|(_, t)| 12 + t.len() * 16).collect();

        let mut font_offsets = Vec::with_capacity(ttfs.len());
        let mut pos = ttc_header_size;
        for size in &font_header_sizes {
            font_offsets.push(pos);
            pos += size;
        }

        let data_start = pos;
        let mut new_table_offsets: Vec<Vec<usize>> = Vec::with_capacity(ttfs.len());
        let mut data_pos = data_start;
        for (_, tables) in &font_infos {
            let mut offsets = Vec::with_capacity(tables.len());
            for table in tables {
                offsets.push(data_pos);
                data_pos += align4(table.length);
            }
            new_table_offsets.push(offsets);
        }

        let total_size = data_pos;
        let mut out = vec![0u8; total_size];
        out[..4].copy_from_slice(&TTC_TAG);
        write_u16(&mut out, 4, 1); // major version
        write_u16(&mut out, 6, 0); // minor version
        write_u32(&mut out, 8, ttfs.len() as u32);
        for (i, off) in font_offsets.iter().enumerate() {
            write_u32(&mut out, 12 + i * 4, *off as u32);
        }

        for (fi, (sf_version, tables)) in font_infos.iter().enumerate() {
            let base = font_offsets[fi];
            write_u32(&mut out, base, *sf_version);
            write_u16(&mut out, base + 4, tables.len() as u16);
            let (sr, es, rs) = calc_offset_table_fields(tables.len() as u16);
            write_u16(&mut out, base + 6, sr);
            write_u16(&mut out, base + 8, es);
            write_u16(&mut out, base + 10, rs);

            for (ti, table) in tables.iter().enumerate() {
                let rec_offset = base + 12 + ti * 16;
                write_u32(&mut out, rec_offset, table.tag);
                write_u32(&mut out, rec_offset + 4, table.check_sum);
                write_u32(&mut out, rec_offset + 8, new_table_offsets[fi][ti] as u32);
                write_u32(&mut out, rec_offset + 12, table.length as u32);
            }
        }

        // Copy table data.
        let mut write_pos = data_start;
        for (fi, (_, tables)) in font_infos.iter().enumerate() {
            for table in tables {
                let src = &ttfs[fi][table.offset..table.offset + table.length];
                out[write_pos..write_pos + table.length].copy_from_slice(src);
                write_pos += align4(table.length);
            }
        }

        out
    }

    // -- is_ttc --------------------------------------------------------------

    #[test]
    fn is_ttc_recognizes_magic() {
        let mut buf = vec![0u8; 12];
        buf[..4].copy_from_slice(&TTC_TAG);
        assert!(is_ttc(&buf));
    }

    #[test]
    fn is_ttc_rejects_ttf() {
        let ttf = synthetic_ttf(1);
        assert!(!is_ttc(&ttf));
    }

    #[test]
    fn is_ttc_rejects_empty() {
        assert!(!is_ttc(&[]));
    }

    #[test]
    fn is_ttc_rejects_too_short() {
        assert!(!is_ttc(&[0x74, 0x74, 0x63]));
    }

    // -- ttc_face_count ------------------------------------------------------

    #[test]
    fn ttc_face_count_reads_header() {
        let ttc = build_ttc_from_ttfs(&[synthetic_ttf(1), synthetic_ttf(2)]);
        assert_eq!(ttc_face_count(&ttc), Some(2));
    }

    #[test]
    fn ttc_face_count_none_for_non_ttc() {
        let ttf = synthetic_ttf(1);
        assert_eq!(ttc_face_count(&ttf), None);
    }

    // -- extract_ttc_faces ---------------------------------------------------

    #[test]
    fn extracts_individual_ttf_buffers() {
        let ttc = build_ttc_from_ttfs(&[synthetic_ttf(0xAA), synthetic_ttf(0xBB)]);
        let extracted = extract_ttc_faces(&ttc);
        assert_eq!(extracted.len(), 2);
        // Roundtrip distinguishability: each extracted buffer's data
        // table preserves its source family marker byte.
        let table0_offset = read_u32(&extracted[0], 12 + 8).unwrap() as usize;
        let table1_offset = read_u32(&extracted[1], 12 + 8).unwrap() as usize;
        assert_eq!(extracted[0][table0_offset], 0xAA);
        assert_eq!(extracted[1][table1_offset], 0xBB);
    }

    #[test]
    fn non_ttc_returns_empty() {
        assert!(extract_ttc_faces(&synthetic_ttf(1)).is_empty());
    }

    #[test]
    fn empty_buffer_returns_empty() {
        assert!(extract_ttc_faces(&[]).is_empty());
    }

    #[test]
    fn zero_face_ttc_returns_empty() {
        let mut buf = vec![0u8; 12];
        buf[..4].copy_from_slice(&TTC_TAG);
        write_u16(&mut buf, 4, 1); // major version
        write_u16(&mut buf, 6, 0); // minor version
        write_u32(&mut buf, 8, 0); // numFonts = 0
        assert!(extract_ttc_faces(&buf).is_empty());
    }

    #[test]
    fn single_face_ttc_extracts_one() {
        let ttc = build_ttc_from_ttfs(&[synthetic_ttf(0xCC)]);
        let extracted = extract_ttc_faces(&ttc);
        assert_eq!(extracted.len(), 1);
    }

    #[test]
    fn out_of_range_table_offset_skipped() {
        let mut ttc = build_ttc_from_ttfs(&[synthetic_ttf(0xAA)]);
        let font_offset = read_u32(&ttc, 12).unwrap() as usize;
        let first_table_record_offset = font_offset + 12;
        // Corrupt the first table's offset to be wildly out of range.
        write_u32(&mut ttc, first_table_record_offset + 8, 0xFFFF_FFFF);
        let extracted = extract_ttc_faces(&ttc);
        assert!(extracted.is_empty());
    }

    #[test]
    fn corrupt_second_face_only_returns_first() {
        let mut ttc = build_ttc_from_ttfs(&[synthetic_ttf(0xAA), synthetic_ttf(0xBB)]);
        let font2_offset = read_u32(&ttc, 16).unwrap() as usize;
        let first_table_record_offset = font2_offset + 12;
        write_u32(&mut ttc, first_table_record_offset + 8, 0xFFFF_FFFF);
        let extracted = extract_ttc_faces(&ttc);
        assert_eq!(extracted.len(), 1);
    }

    // -- extract_first_ttc_face ---------------------------------------------

    #[test]
    fn extract_first_ttc_face_returns_only_first() {
        let ttc = build_ttc_from_ttfs(&[synthetic_ttf(0xAA), synthetic_ttf(0xBB)]);
        let face = extract_first_ttc_face(&ttc).expect("first face");
        let table0_offset = read_u32(&face, 12 + 8).unwrap() as usize;
        assert_eq!(face[table0_offset], 0xAA);
    }

    #[test]
    fn extract_first_ttc_face_none_for_non_ttc() {
        let ttf = synthetic_ttf(1);
        assert!(extract_first_ttc_face(&ttf).is_none());
    }

    // -- parse_font_data error path -----------------------------------------

    #[test]
    fn parse_font_data_rejects_invalid_bytes() {
        let result = parse_font_data(b"not a font".to_vec());
        assert!(result.is_err());
    }

    #[test]
    fn parse_font_data_rejects_ttc_with_invalid_faces() {
        // Synthetic TTC contains structurally-correct but content-invalid
        // faces; ttf-parser rejects every face → parse_font_data errors.
        let ttc = build_ttc_from_ttfs(&[synthetic_ttf(0xAA)]);
        let result = parse_font_data(ttc);
        assert!(result.is_err(), "expected error for synthetic TTC");
    }

    // -- align4 / calc_offset_table_fields ----------------------------------

    #[test]
    fn align4_basic() {
        assert_eq!(align4(0), 0);
        assert_eq!(align4(1), 4);
        assert_eq!(align4(4), 4);
        assert_eq!(align4(5), 8);
        assert_eq!(align4(7), 8);
        assert_eq!(align4(8), 8);
    }

    #[test]
    fn calc_offset_table_fields_one_table() {
        let (sr, es, rs) = calc_offset_table_fields(1);
        // 16 * 1 = 16, search_range starts at 16, no doubling possible.
        assert_eq!((sr, es, rs), (16, 0, 0));
    }

    #[test]
    fn calc_offset_table_fields_eight_tables() {
        let (sr, es, rs) = calc_offset_table_fields(8);
        // numTables*16 = 128. Loop condition is `<=`, so search_range
        // doubles 16→32→64→128 (last iteration: 64*2=128 ≤ 128). es=3,
        // rs = 128 - 128 = 0. Matches TS exactly.
        assert_eq!((sr, es, rs), (128, 3, 0));
    }

    #[test]
    fn calc_offset_table_fields_three_tables() {
        let (sr, es, rs) = calc_offset_table_fields(3);
        // numTables*16 = 48. 16→32 (32*2=64 > 48 → stop). es=1,
        // rs = 48 - 32 = 16.
        assert_eq!((sr, es, rs), (32, 1, 16));
    }
}
