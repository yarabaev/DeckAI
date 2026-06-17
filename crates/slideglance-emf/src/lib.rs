//! EMF / WMF raster-embed extraction.
//!
//! `PowerPoint` frequently wraps a single bitmap (a screenshot, an
//! exported chart, a pasted picture) inside an EMF or WMF metafile.
//! The metafile contains exactly one bitmap-bearing record
//! (`EMR_STRETCHDIBITS`, `EMR_BITBLT`, `EMR_STRETCHBLT`,
//! `EMR_SETDIBITSTODEVICE`, `EMR_TRANSPARENTBLT` for EMF; the
//! `META_DIBBITBLT` / `META_DIBSTRETCHBLT` / `META_STRETCHDIB`
//! family for WMF) whose Device-Independent-Bitmap payload can be
//! reassembled into a standard `.bmp` file.
//!
//! True vector EMF / WMF (path / line / draw commands) is not handled
//! — the caller should fall back to a placeholder rect when this
//! crate returns `None`. The strategy intentionally trades off a
//! small subset of capability for zero external dependencies and
//! pure-Rust deterministic output.
//!
//! ## Output format
//!
//! [`extract_raster`] returns a `Vec<u8>` containing a complete BMP
//! file (`BITMAPFILEHEADER` + `BITMAPINFOHEADER` + palette + pixels).
//! BMP is a universally-supported raster format — `resvg`, all major
//! browsers, and image-decoding crates accept it directly via the
//! `image/bmp` MIME type, so the renderer can inline it as a
//! `data:image/bmp;base64,...` URL exactly like the PNG / JPEG paths.

#![warn(missing_docs)]

mod emf;
mod wmf;

pub use emf::{extract_raster_from_emf, EmfRasterError};
pub use wmf::{extract_raster_from_wmf, WmfRasterError};

/// Detected metafile flavour. Set by [`detect_metafile_kind`] from
/// the leading bytes; the renderer dispatches on it before invoking
/// [`extract_raster_from_emf`] / [`extract_raster_from_wmf`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetafileKind {
    /// Enhanced Metafile (32-bit). First record is `EMR_HEADER` and
    /// the bytes at offset 40 spell ` EMF` (4 bytes incl. leading
    /// space) per [MS-EMF] section 2.3.4.2.
    Emf,
    /// Aldus Placeable Metafile — a 22-byte header (magic `D7CDC69A`)
    /// preceding a standard Windows-3 WMF body.
    WmfPlaceable,
    /// Standard (non-placeable) Windows Metafile. Header is `META_HEADER`
    /// (18 bytes) starting with `Type` (1=memory, 2=disk) followed by
    /// `HeaderSize` (9 = 18 bytes / 2).
    WmfStandard,
}

/// Sniffs the first ~32 bytes of `bytes` to decide whether it's EMF,
/// placeable WMF, or standard WMF. Returns `None` when no metafile
/// signature matches.
#[must_use]
pub fn detect_metafile_kind(bytes: &[u8]) -> Option<MetafileKind> {
    // Aldus Placeable Metafile: 22-byte header starting with the
    // 4-byte magic `D7 CD C6 9A` (little-endian DWORD `0x9AC6CDD7`).
    if bytes.len() >= 22
        && bytes[0] == 0xD7
        && bytes[1] == 0xCD
        && bytes[2] == 0xC6
        && bytes[3] == 0x9A
    {
        return Some(MetafileKind::WmfPlaceable);
    }

    // EMF: leading record is `EMR_HEADER` (Type = 0x00000001) and at
    // record offset 40 the 4-byte signature ` EMF` appears.
    if bytes.len() >= 44 && read_u32_le(bytes, 0) == Some(0x0000_0001) && &bytes[40..44] == b" EMF"
    {
        return Some(MetafileKind::Emf);
    }

    // Standard WMF: Type ∈ {1,2}, HeaderSize = 9 (18 bytes / 2),
    // Version = 0x0300 (Windows 3.0+ format).
    if bytes.len() >= 18 {
        let type_u16 = read_u16_le(bytes, 0).unwrap_or(0);
        let header_size = read_u16_le(bytes, 2).unwrap_or(0);
        let version = read_u16_le(bytes, 4).unwrap_or(0);
        if (type_u16 == 1 || type_u16 == 2) && header_size == 9 && version == 0x0300 {
            return Some(MetafileKind::WmfStandard);
        }
    }

    None
}

/// Top-level extraction entry. Detects the metafile flavour and
/// dispatches to the appropriate extractor, then returns PNG bytes.
/// Returns `None` when the input isn't a recognized metafile,
/// contains no bitmap-bearing record, or fails the BMP→PNG round-trip.
///
/// The returned bytes are a self-contained PNG. PNG was chosen over
/// raw BMP because `resvg` (and the wider SVG ecosystem) does not
/// inline `data:image/bmp` URLs reliably, while every SVG renderer
/// supports `data:image/png`.
///
/// EMF+ "compressed" payloads (PNG / JPEG already embedded in the
/// metafile, the most common case from `PowerPoint` exports) bypass the
/// BMP round-trip entirely so RGBA alpha is preserved end-to-end. The
/// previous round-trip through `image::ImageFormat::Bmp` defaulted to
/// 24-bit RGB and stripped alpha, which collapsed transparent icon
/// backgrounds into solid white squares that overpainted neighbouring
/// text (slide 96 / 100's title clipping was the canary).
#[must_use]
pub fn extract_raster(bytes: &[u8]) -> Option<Vec<u8>> {
    let bmp = extract_raster_as_bmp(bytes)?;
    if is_png_bytes(&bmp) {
        return Some(bmp);
    }
    bmp_to_png(&bmp)
}

fn is_png_bytes(b: &[u8]) -> bool {
    b.len() >= 8 && b.starts_with(b"\x89PNG\r\n\x1a\n")
}

/// Same dispatch as [`extract_raster`] but returns the intermediate
/// BMP bytes. Useful for tests / tooling that wants to inspect the
/// raw extracted DIB without paying for a PNG re-encode.
#[must_use]
pub fn extract_raster_as_bmp(bytes: &[u8]) -> Option<Vec<u8>> {
    match detect_metafile_kind(bytes)? {
        MetafileKind::Emf => extract_raster_from_emf(bytes).ok(),
        MetafileKind::WmfPlaceable => {
            // Skip the 22-byte placeable header, then parse the
            // standard WMF body.
            extract_raster_from_wmf(bytes.get(22..)?).ok()
        }
        MetafileKind::WmfStandard => extract_raster_from_wmf(bytes).ok(),
    }
}

/// Decode a BMP byte stream and re-encode it as PNG using the
/// `image` crate. Returns `None` when the BMP is malformed (the
/// caller treats this as "fall back to placeholder" — the `image`
/// crate's BMP decoder accepts the EMF-extracted DIBs we generate
/// in practice).
fn bmp_to_png(bmp: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory_with_format(bmp, image::ImageFormat::Bmp).ok()?;
    let mut out = Vec::with_capacity(bmp.len() / 2);
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .ok()?;
    Some(out)
}

// -- Little-endian byte readers ------------------------------------------

#[inline]
pub(crate) fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([slice[0], slice[1]]))
}

#[inline]
pub(crate) fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// Build a complete BMP file from a `BITMAPINFOHEADER` + pixel buffer.
///
/// The DIB header is the bytes following the EMF/WMF record's
/// `offBmiSrc` field; the bits buffer is the bytes at `offBitsSrc`.
/// This helper prepends the 14-byte `BITMAPFILEHEADER` so the result
/// is a self-contained, decodable BMP.
pub(crate) fn build_bmp_file(dib_header: &[u8], dib_bits: &[u8]) -> Option<Vec<u8>> {
    // Need at least the 4-byte biSize prefix to know the actual DIB
    // header length (`BITMAPCOREHEADER` is 12, `BITMAPINFOHEADER` 40,
    // `BITMAPV4HEADER` 108, `BITMAPV5HEADER` 124).
    if dib_header.len() < 4 {
        return None;
    }
    let dib_size = read_u32_le(dib_header, 0)? as usize;
    if dib_size < 12 || dib_size > dib_header.len() {
        return None;
    }

    // Determine palette length (4 bytes / RGBQUAD entry) when the
    // bitmap is paletted (1 / 4 / 8 bpp). For BITMAPINFOHEADER+:
    // biClrUsed at offset 32, biBitCount at offset 14.
    // For BITMAPCOREHEADER (size 12) palette uses RGBTRIPLE (3 bytes).
    let palette_len = compute_palette_len(dib_header)?;
    let total_dib = dib_size + palette_len;
    let file_size = 14usize + total_dib + dib_bits.len();
    let bits_offset = 14usize + total_dib;

    let mut out = Vec::with_capacity(file_size);
    // BITMAPFILEHEADER (14 bytes):
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&u32::try_from(file_size).ok()?.to_le_bytes());
    out.extend_from_slice(&[0u8; 4]); // reserved1 + reserved2
    out.extend_from_slice(&u32::try_from(bits_offset).ok()?.to_le_bytes());
    // DIB header + palette (header in `dib_header[..]` already includes
    // any palette appended by EMF when cbBmiSrc > biSize):
    out.extend_from_slice(dib_header);
    // Pixel bits:
    out.extend_from_slice(dib_bits);
    Some(out)
}

pub(crate) fn compute_palette_len_pub(dib_header: &[u8]) -> Option<usize> {
    compute_palette_len(dib_header)
}

fn compute_palette_len(dib_header: &[u8]) -> Option<usize> {
    let dib_size = read_u32_le(dib_header, 0)? as usize;
    if dib_size == 12 {
        // BITMAPCOREHEADER. Palette = 2^biBitCount entries × 3 bytes.
        // biBitCount at offset 10 (u16 within 12-byte header).
        let bpp = read_u16_le(dib_header, 10)? as usize;
        if bpp <= 8 {
            return Some(3 * (1usize << bpp));
        }
        return Some(0);
    }
    // BITMAPINFOHEADER family — palette uses RGBQUAD (4 bytes).
    // biBitCount at offset 14 (u16); biClrUsed at offset 32 (u32).
    let bpp = read_u16_le(dib_header, 14)? as usize;
    if bpp == 0 || bpp > 8 {
        return Some(0);
    }
    let used = read_u32_le(dib_header, 32)? as usize;
    let count = if used == 0 { 1usize << bpp } else { used };
    Some(4 * count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_none_for_empty() {
        assert_eq!(detect_metafile_kind(&[]), None);
    }

    #[test]
    fn detect_returns_none_for_random_bytes() {
        assert_eq!(detect_metafile_kind(b"hello world"), None);
    }

    #[test]
    fn detect_recognises_placeable_wmf_magic() {
        let mut bytes = vec![0u8; 22];
        bytes[0] = 0xD7;
        bytes[1] = 0xCD;
        bytes[2] = 0xC6;
        bytes[3] = 0x9A;
        assert_eq!(
            detect_metafile_kind(&bytes),
            Some(MetafileKind::WmfPlaceable)
        );
    }

    #[test]
    fn detect_recognises_emf_signature() {
        let mut bytes = vec![0u8; 44];
        bytes[0] = 0x01; // Type = EMR_HEADER
        bytes[40] = b' ';
        bytes[41] = b'E';
        bytes[42] = b'M';
        bytes[43] = b'F';
        assert_eq!(detect_metafile_kind(&bytes), Some(MetafileKind::Emf));
    }

    #[test]
    fn detect_recognises_standard_wmf_header() {
        // Type=1, HeaderSize=9, Version=0x0300
        let mut bytes = vec![0u8; 18];
        bytes[0] = 0x01; // Type
        bytes[2] = 0x09; // HeaderSize lo
        bytes[4] = 0x00; // Version lo
        bytes[5] = 0x03; // Version hi
        assert_eq!(
            detect_metafile_kind(&bytes),
            Some(MetafileKind::WmfStandard)
        );
    }

    #[test]
    fn build_bmp_file_assembles_header_and_bits() {
        // Minimal BITMAPINFOHEADER (40 bytes), 1×1 24bpp pixel.
        let mut dib = vec![0u8; 40];
        dib[0] = 40; // biSize = 40
        dib[4] = 1; // biWidth = 1
        dib[8] = 1; // biHeight = 1
        dib[12] = 1; // biPlanes = 1
        dib[14] = 24; // biBitCount = 24
        let bits = vec![0xFFu8, 0x00, 0x00, 0x00]; // 1 BGR pixel padded to DWORD
        let bmp = build_bmp_file(&dib, &bits).expect("BMP build");
        assert_eq!(&bmp[..2], b"BM");
        // file size = 14 + 40 + 4 = 58
        assert_eq!(read_u32_le(&bmp, 2), Some(58));
        // bits offset = 14 + 40 = 54
        assert_eq!(read_u32_le(&bmp, 10), Some(54));
        // Trailing bytes should be the bits we passed in.
        assert_eq!(&bmp[54..], &bits[..]);
    }
}
