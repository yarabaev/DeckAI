//! WMF (Windows Metafile, pre-Win32) raster-record extraction.
//!
//! WMF is the 16-bit predecessor of EMF. Records carry their size in
//! 16-bit `WORD` units (the field name in the spec is `RecordSize`)
//! and the record type as a 16-bit `Function` code. The DIB-bearing
//! records we care about are:
//!
//! - `META_STRETCHDIB` (`0x0F43`)
//! - `META_DIBBITBLT` (`0x0940`)
//! - `META_DIBSTRETCHBLT` (`0x0B41`)
//!
//! Each carries a contiguous Device-Independent-Bitmap blob
//! (`BITMAPINFOHEADER` + optional palette + pixel bits), which we
//! re-wrap as a BMP file via [`super::build_bmp_file`].
//!
//! See [MS-WMF] §2.3.1 for the record types and §2.3.5 for `META_HEADER`.

use crate::{compute_palette_len_pub, read_u16_le, read_u32_le};

/// Reasons WMF extraction can fail. Mirror of [`super::EmfRasterError`]
/// but with WMF-specific tags so the dispatcher can preserve the
/// error origin in tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WmfRasterError {
    /// Input is shorter than the 18-byte `META_HEADER` minimum.
    TooShort,
    /// `META_HEADER` fields don't match the standard WMF magic.
    NotAWmf,
    /// Record claimed a size that walked past the buffer end.
    TruncatedRecord,
    /// Walked the entire stream without finding a recognised raster
    /// record.
    NoBitmapRecord,
    /// A raster record's DIB header / bits offsets pointed outside
    /// the record bounds.
    MalformedDib,
}

const META_DIBBITBLT: u16 = 0x0940;
const META_DIBSTRETCHBLT: u16 = 0x0B41;
const META_STRETCHDIB: u16 = 0x0F43;
const META_EOF: u16 = 0x0000;

/// Walks `bytes` (a complete WMF file body, post-placeable-header)
/// and returns a self-contained BMP for the first DIB-bearing record
/// encountered.
pub fn extract_raster_from_wmf(bytes: &[u8]) -> Result<Vec<u8>, WmfRasterError> {
    if bytes.len() < 18 {
        return Err(WmfRasterError::TooShort);
    }
    let type_u16 = read_u16_le(bytes, 0).unwrap_or(0);
    let header_size_words = read_u16_le(bytes, 2).unwrap_or(0);
    let version = read_u16_le(bytes, 4).unwrap_or(0);
    if (type_u16 != 1 && type_u16 != 2) || header_size_words != 9 || version != 0x0300 {
        return Err(WmfRasterError::NotAWmf);
    }

    let mut cursor = (header_size_words as usize) * 2;
    while cursor + 6 <= bytes.len() {
        // RecordSize is in 16-bit WORD units, so multiply by 2 for bytes.
        let record_words = read_u32_le(bytes, cursor).ok_or(WmfRasterError::TruncatedRecord)?;
        let record_size = (record_words as usize)
            .checked_mul(2)
            .ok_or(WmfRasterError::TruncatedRecord)?;
        if record_size < 6 || cursor + record_size > bytes.len() {
            return Err(WmfRasterError::TruncatedRecord);
        }
        let function = read_u16_le(bytes, cursor + 4).ok_or(WmfRasterError::TruncatedRecord)?;
        if function == META_EOF {
            break;
        }

        // Function low-order 8 bits encode the parameter count; the
        // high-order bits encode the actual record category. For
        // matching DIB records the full 16-bit value is what's spec'd.
        let record = &bytes[cursor..cursor + record_size];
        let dib_offset = match function {
            META_STRETCHDIB => Some(28),
            META_DIBSTRETCHBLT => Some(26),
            META_DIBBITBLT => Some(22),
            _ => None,
        };

        if let Some(off) = dib_offset {
            if let Some(bmp) = try_extract_contiguous_dib(record, off) {
                return Ok(bmp);
            }
        }

        cursor += record_size;
    }

    Err(WmfRasterError::NoBitmapRecord)
}

/// WMF DIB records carry the `BITMAPINFOHEADER` + palette + bits as
/// a contiguous blob — unlike EMF, there's no separate
/// `offBmiSrc` / `offBitsSrc` split. Compute the palette length from
/// the DIB header to find the boundary, then forward both halves to
/// the shared BMP builder.
fn try_extract_contiguous_dib(record: &[u8], dib_offset: usize) -> Option<Vec<u8>> {
    let blob = record.get(dib_offset..)?;
    if blob.len() < 4 {
        return None;
    }
    let dib_size = read_u32_le(blob, 0)? as usize;
    if dib_size < 12 || dib_size > blob.len() {
        return None;
    }
    let palette_len = compute_palette_len_pub(&blob[..dib_size])?;
    let header_total = dib_size.checked_add(palette_len)?;
    if header_total > blob.len() {
        return None;
    }
    let dib_header = &blob[..header_total];
    let dib_bits = &blob[header_total..];
    crate::build_bmp_file(dib_header, dib_bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_stretchdib_wmf() -> Vec<u8> {
        let mut wmf = Vec::new();
        // -- META_HEADER (18 bytes) ------------------------------------
        wmf.extend_from_slice(&1u16.to_le_bytes()); // Type = memory metafile
        wmf.extend_from_slice(&9u16.to_le_bytes()); // HeaderSize = 9 WORDs
        wmf.extend_from_slice(&0x0300u16.to_le_bytes()); // Version
        wmf.extend_from_slice(&[0u8; 4]); // Size in WORDs
        wmf.extend_from_slice(&0u16.to_le_bytes()); // NumberOfObjects
        wmf.extend_from_slice(&[0u8; 4]); // MaxRecord
        wmf.extend_from_slice(&0u16.to_le_bytes()); // NumberOfMembers
        assert_eq!(wmf.len(), 18);

        // -- META_STRETCHDIB record ------------------------------------
        // Header (28 bytes) + BITMAPINFOHEADER (40) + 4 bits = 72 bytes
        // RecordSize in WORDs = 36.
        let record_words = 36u32;
        let record_size = record_words as usize * 2;
        let start = wmf.len();
        wmf.extend_from_slice(&record_words.to_le_bytes()); // Size (4 bytes, in WORDs)
        wmf.extend_from_slice(&META_STRETCHDIB.to_le_bytes()); // Function
        wmf.extend_from_slice(&0u32.to_le_bytes()); // RasterOperation
        wmf.extend_from_slice(&0u16.to_le_bytes()); // ColorUsage
        wmf.extend_from_slice(&0u16.to_le_bytes()); // SrcHeight
        wmf.extend_from_slice(&0u16.to_le_bytes()); // SrcWidth
        wmf.extend_from_slice(&0u16.to_le_bytes()); // YSrc
        wmf.extend_from_slice(&0u16.to_le_bytes()); // XSrc
        wmf.extend_from_slice(&0u16.to_le_bytes()); // DestHeight
        wmf.extend_from_slice(&0u16.to_le_bytes()); // DestWidth
        wmf.extend_from_slice(&0u16.to_le_bytes()); // YDest
        wmf.extend_from_slice(&0u16.to_le_bytes()); // XDest
                                                    // DIB blob: BITMAPINFOHEADER (40) + 4 pixel bytes
        wmf.extend_from_slice(&40u32.to_le_bytes()); // biSize
        wmf.extend_from_slice(&1i32.to_le_bytes()); // biWidth
        wmf.extend_from_slice(&1i32.to_le_bytes()); // biHeight
        wmf.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
        wmf.extend_from_slice(&24u16.to_le_bytes()); // biBitCount
        wmf.extend_from_slice(&[0u8; 24]); // biCompression.. biClrImportant
        wmf.extend_from_slice(&[0xFF, 0x00, 0x00, 0x00]); // 1 BGR pixel
        assert_eq!(wmf.len() - start, record_size);

        // -- META_EOF (3 WORDs = 6 bytes) ------------------------------
        wmf.extend_from_slice(&3u32.to_le_bytes()); // Size
        wmf.extend_from_slice(&META_EOF.to_le_bytes()); // Function = 0
        wmf
    }

    #[test]
    fn extracts_bmp_from_synthetic_stretchdib() {
        let wmf = synthetic_stretchdib_wmf();
        let bmp = extract_raster_from_wmf(&wmf).expect("extract OK");
        assert_eq!(&bmp[..2], b"BM");
        // 14 + 40 + 4 = 58
        assert_eq!(bmp.len(), 58);
    }

    #[test]
    fn rejects_too_short_input() {
        assert_eq!(
            extract_raster_from_wmf(&[0u8; 5]),
            Err(WmfRasterError::TooShort)
        );
    }

    #[test]
    fn rejects_non_wmf_header() {
        let bytes = vec![0u8; 50];
        assert_eq!(
            extract_raster_from_wmf(&bytes),
            Err(WmfRasterError::NotAWmf)
        );
    }
}
