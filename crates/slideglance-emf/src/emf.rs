//! EMF (Enhanced Metafile) raster-record extraction.
//!
//! Iterates the EMF record stream and pulls the DIB out of the first
//! bitmap-bearing record we recognise. Each record begins with a
//! `(Type, Size)` pair so we can walk the stream with no dependency
//! on Windows GDI semantics.
//!
//! See [MS-EMF] §2.3 for record types; §2.2.9 for `BitmapBuffer`.

use crate::{build_bmp_file, read_u32_le};

/// Reasons extraction can fail. The renderer treats all of these as
/// "fall back to placeholder" — they are not user-visible diagnostics
/// but they make the test surface easier to assert on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmfRasterError {
    /// Input is shorter than the 88-byte `EMR_HEADER` minimum.
    TooShort,
    /// Leading record is not `EMR_HEADER` (Type=1) with the ` EMF`
    /// signature at byte 40.
    NotAnEmf,
    /// Walked the entire stream without finding a recognised raster
    /// record.
    NoBitmapRecord,
    /// A raster record's DIB header / bits offsets pointed outside
    /// the record bounds.
    MalformedDib,
    /// Record table claimed a record larger than the remaining buffer.
    TruncatedRecord,
}

/// Walks `bytes` (a complete EMF file body) and returns a self-
/// contained BMP for the first bitmap-bearing record encountered.
///
/// Two extraction strategies, in priority order:
/// 1. Legacy GDI raster records (`EMR_STRETCHDIBITS` and friends) — the
///    DIB is built from the record payload directly.
/// 2. EMF+ wrapper (`EMR_COMMENT` records with the ` EMF+` identifier
///    spec'd in [MS-EMFPLUS]) — accumulate the EMF+ stream across all
///    comment records, then walk EMF+ records to find an `EmfPlusObject`
///    of type `Image` with bitmap data. Modern `PowerPoint` exports
///    almost always wrap their bitmaps this way.
pub fn extract_raster_from_emf(bytes: &[u8]) -> Result<Vec<u8>, EmfRasterError> {
    if bytes.len() < 44 {
        return Err(EmfRasterError::TooShort);
    }
    if read_u32_le(bytes, 0) != Some(0x0000_0001) || &bytes[40..44] != b" EMF" {
        return Err(EmfRasterError::NotAnEmf);
    }

    // First record is `EMR_HEADER`; its Size field tells us where
    // record #2 starts.
    let header_size = read_u32_le(bytes, 4).ok_or(EmfRasterError::TooShort)? as usize;
    if header_size < 8 || header_size > bytes.len() {
        return Err(EmfRasterError::TruncatedRecord);
    }

    let mut cursor = header_size;
    let mut emfplus_stream: Vec<u8> = Vec::new();
    while cursor + 8 <= bytes.len() {
        let record_type = read_u32_le(bytes, cursor).ok_or(EmfRasterError::TruncatedRecord)?;
        let record_size =
            read_u32_le(bytes, cursor + 4).ok_or(EmfRasterError::TruncatedRecord)? as usize;
        if record_size < 8 || cursor + record_size > bytes.len() {
            return Err(EmfRasterError::TruncatedRecord);
        }
        let record = &bytes[cursor..cursor + record_size];

        if record_type == EMR_EOF {
            break;
        }

        if let Some(dib_offset) = raster_record_dib_offset(record_type) {
            if let Some(bmp) = try_extract_dib(record, dib_offset) {
                return Ok(bmp);
            }
        }

        // EMR_COMMENT: collect EMF+ payload for after-the-fact scanning.
        // Each EMR_COMMENT carries an arbitrarily-sized chunk of the
        // EMF+ stream and `PowerPoint` exports often split a single
        // `EmfPlusObject` (the bitmap) across several comment records,
        // so we concatenate before parsing.
        if record_type == EMR_COMMENT && record.len() >= 16 {
            // EMR_COMMENT layout: Type(4) Size(4) DataSize(4) CommentIdentifier(4)
            // followed by CommentRecordParm. For EMF+, the identifier is
            // ` EMF+` (LE bytes 0x2B 0x46 0x4D 0x45) and the rest of the
            // comment data (DataSize - 4 bytes) is EMF+ stream content.
            if &record[12..16] == b"EMF+" {
                let data_size = read_u32_le(record, 8).unwrap_or(0) as usize;
                if data_size >= 4 && 8 + data_size <= record.len() {
                    let payload_end = 8 + data_size;
                    emfplus_stream.extend_from_slice(&record[16..payload_end]);
                }
            }
        }

        cursor += record_size;
    }

    if !emfplus_stream.is_empty() {
        if let Some(bmp) = try_extract_emfplus_bitmap(&emfplus_stream) {
            return Ok(bmp);
        }
    }

    Err(EmfRasterError::NoBitmapRecord)
}

/// `EMR_EOF` record terminator — defined here rather than alongside the
/// raster types because it's the loop exit signal, not a candidate.
const EMR_EOF: u32 = 0x0000_000E;
/// `EMR_COMMENT` carries application-private data, including the EMF+
/// stream emitted by .NET / modern `PowerPoint` exports. Its identifier
/// at offset 12 distinguishes EMF+ (` EMF+`) from public/private others.
const EMR_COMMENT: u32 = 0x0000_0046;

/// EMF raster record types and the offset (from record start) where
/// their `offBmiSrc` DWORD lives. Each raster record packs four
/// consecutive DWORDs at this offset:
/// `offBmiSrc / cbBmiSrc / offBitsSrc / cbBitsSrc`.
///
/// Offsets verified against [MS-EMF] §2.3.1 (Bitmap records).
const EMR_BITBLT: u32 = 0x0000_004C;
const EMR_STRETCHBLT: u32 = 0x0000_004D;
const EMR_MASKBLT: u32 = 0x0000_004E;
const EMR_PLGBLT: u32 = 0x0000_004F;
const EMR_SETDIBITSTODEVICE: u32 = 0x0000_0050;
const EMR_STRETCHDIBITS: u32 = 0x0000_0051;
const EMR_ALPHABLEND: u32 = 0x0000_0072;
const EMR_TRANSPARENTBLT: u32 = 0x0000_0074;

#[allow(clippy::match_same_arms)]
fn raster_record_dib_offset(record_type: u32) -> Option<usize> {
    // `offBmiSrc` offset within the record. The DIB header starts at
    // record[offBmiSrc..offBmiSrc + cbBmiSrc]; bits at
    // record[offBitsSrc..offBitsSrc + cbBitsSrc].
    match record_type {
        // EMR_STRETCHDIBITS layout (80 bytes header):
        // Type, Size, Bounds (16), xDest, yDest, xSrc, ySrc,
        // cxSrc, cySrc, offBmiSrc, cbBmiSrc, offBitsSrc, cbBitsSrc,
        // UsageSrc, BitBltRasterOperation, cxDest, cyDest.
        EMR_STRETCHDIBITS => Some(48),
        // EMR_SETDIBITSTODEVICE layout (76 bytes header):
        // Type, Size, Bounds (16), xDest, yDest, xSrc, ySrc,
        // cxSrc, cySrc, offBmiSrc, cbBmiSrc, offBitsSrc, cbBitsSrc,
        // UsageSrc, iStartScan, cScans.
        EMR_SETDIBITSTODEVICE => Some(48),
        // EMR_BITBLT layout (100 bytes header):
        // Type, Size, Bounds (16), xDest, yDest, cxDest, cyDest,
        // BitBltRasterOperation, xSrc, ySrc, XformSrc (24),
        // BkColorSrc, UsageSrc, offBmiSrc, cbBmiSrc, offBitsSrc,
        // cbBitsSrc.
        EMR_BITBLT => Some(84),
        // EMR_STRETCHBLT layout (108 bytes header): same as BITBLT
        // plus cxSrc/cySrc trailing.
        EMR_STRETCHBLT => Some(84),
        // EMR_TRANSPARENTBLT layout same prefix as STRETCHBLT.
        EMR_TRANSPARENTBLT => Some(84),
        // EMR_ALPHABLEND layout: BlendFunction (4) then same as STRETCHBLT
        // tail. offBmiSrc lands at 88.
        EMR_ALPHABLEND => Some(88),
        // EMR_MASKBLT carries two DIBs (source + mask). Source DIB
        // offsets land at 84 — same as BITBLT's source side. We grab
        // the source bitmap and ignore the mask for placeholder
        // rendering parity.
        EMR_MASKBLT => Some(84),
        // EMR_PLGBLT carries an XFORM-mapped source DIB. offBmiSrc
        // at 116 after the 3-point parallelogram + XFORM block.
        EMR_PLGBLT => Some(116),
        _ => None,
    }
}

/// EMF+ record types (subset). Defined inline rather than alongside the
/// GDI raster types so the discriminants don't collide visually.
mod emfplus {
    #[allow(dead_code)]
    pub const HEADER: u16 = 0x4001;
    pub const EOF: u16 = 0x4002;
    pub const OBJECT: u16 = 0x4008;
    /// `Continued` flag in the upper 16 bits of an `EmfPlusObject` Flags
    /// field. When set, the object body continues in the next record.
    pub const FLAG_CONTINUED: u16 = 0x8000;
    /// `ObjectType.Image` (sec 2.1.2.6).
    pub const OBJECT_TYPE_IMAGE: u16 = 5;
    /// `ImageDataType` values (sec 2.1.1.15).
    pub const IMAGE_TYPE_BITMAP: u32 = 1;
    /// `BitmapDataType.Pixel` — raw pixel bytes follow at `BitmapData` (sec 2.1.1.2).
    pub const BITMAP_DATA_TYPE_PIXEL: u32 = 0;
    /// `BitmapDataType.Compressed` — the `BitmapData` payload is a
    /// self-contained PNG / JPEG / GIF stream.
    pub const BITMAP_DATA_TYPE_COMPRESSED: u32 = 1;
}

/// Walk a (possibly cross-record-concatenated) EMF+ stream and return
/// a self-contained BMP / PNG byte buffer for the first
/// `EmfPlusObject` whose payload is an `Image` of `Bitmap` data type.
///
/// Returns `None` when no image is found, the metadata is malformed,
/// or the pixel format is unsupported. The caller treats `None` as
/// "fall back to placeholder" — same contract as the legacy GDI path.
fn try_extract_emfplus_bitmap(stream: &[u8]) -> Option<Vec<u8>> {
    let mut p = 0usize;
    // Object id → accumulated body across continuation records.
    let mut pending: Option<(u8, Vec<u8>)> = None;
    while p + 12 <= stream.len() {
        let rt = u16::from_le_bytes([stream[p], stream[p + 1]]);
        let flags = u16::from_le_bytes([stream[p + 2], stream[p + 3]]);
        let size = read_u32_le(stream, p + 4)? as usize;
        let data_size = read_u32_le(stream, p + 8)? as usize;
        if size < 12 || p + size > stream.len() || 12 + data_size > size {
            return None;
        }
        if rt == emfplus::EOF {
            break;
        }
        if rt == emfplus::OBJECT {
            // Flags low byte is ObjectId (8 bits), high bits encode
            // ObjectType (low nibble of upper byte) plus the Continued
            // flag (top bit).
            let object_id = (flags & 0x00FF) as u8;
            let object_type = (flags >> 8) & 0x7F;
            let continued = flags & emfplus::FLAG_CONTINUED != 0;
            let body = &stream[p + 12..p + 12 + data_size];

            if object_type == emfplus::OBJECT_TYPE_IMAGE {
                // The first record carrying ObjectId starts a fresh
                // accumulator; continuation records append.
                let buf = match &mut pending {
                    Some((id, buf)) if *id == object_id => buf,
                    _ => {
                        pending = Some((object_id, Vec::with_capacity(data_size)));
                        // SAFETY: just inserted Some.
                        &mut pending.as_mut().expect("just-inserted pending").1
                    }
                };
                buf.extend_from_slice(body);

                if !continued {
                    let buf_clone;
                    let assembled: &[u8] = {
                        let (_, b) = pending.as_ref().expect("buf present");
                        buf_clone = b.clone();
                        &buf_clone
                    };
                    if let Some(image) = decode_emfplus_image(assembled) {
                        return Some(image);
                    }
                    pending = None;
                }
            }
        }
        p += size;
    }
    None
}

/// Decode an `EmfPlusImage` body (sec 2.2.2.18). Returns BMP/PNG bytes
/// suitable for inlining into SVG, or `None` if the format isn't a
/// supported bitmap variant.
#[allow(clippy::too_many_lines)]
fn decode_emfplus_image(body: &[u8]) -> Option<Vec<u8>> {
    if body.len() < 8 {
        return None;
    }
    // ImageData header: Version(4) + Type(4)
    let image_type = read_u32_le(body, 4)?;
    if image_type != emfplus::IMAGE_TYPE_BITMAP {
        return None;
    }
    // Bitmap object header (sec 2.2.2.2): Width Height Stride PixelFormat Type
    if body.len() < 8 + 20 {
        return None;
    }
    #[allow(clippy::cast_possible_wrap)]
    let width = read_u32_le(body, 8)? as i32;
    #[allow(clippy::cast_possible_wrap)]
    let height = read_u32_le(body, 12)? as i32;
    #[allow(clippy::cast_possible_wrap)]
    let stride = read_u32_le(body, 16)? as i32;
    let pixel_format = read_u32_le(body, 20)?;
    let bmp_type = read_u32_le(body, 24)?;
    let bitmap_data = body.get(28..)?;

    match bmp_type {
        emfplus::BITMAP_DATA_TYPE_COMPRESSED => {
            // Compressed format: bitmap data is already a complete
            // PNG / JPEG / GIF stream. PNG sources go through verbatim
            // so RGBA alpha is preserved (the BMP intermediate
            // collapses to 24-bit RGB and would clobber transparent
            // icon backgrounds). For other formats we re-encode as
            // PNG via the `image` crate to keep the surrounding
            // pipeline format-uniform.
            if bitmap_data.starts_with(b"\x89PNG\r\n\x1a\n") {
                return Some(bitmap_data.to_vec());
            }
            let img = image::load_from_memory(bitmap_data).ok()?;
            let mut png = Vec::with_capacity(bitmap_data.len() * 2);
            img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                .ok()?;
            Some(png)
        }
        emfplus::BITMAP_DATA_TYPE_PIXEL => {
            // Raw pixel data. Convert to BMP so the BMP→PNG re-encode
            // path in `lib.rs::bmp_to_png` can finish the conversion.
            // Stride is in bytes per scanline (signed: negative means
            // top-down). Width and Height are unsigned in spec but
            // `PowerPoint` exports use positive values.
            let stride_abs = stride.unsigned_abs() as usize;
            let height_abs = height.unsigned_abs() as usize;
            let width_abs = width.unsigned_abs() as usize;
            if width_abs == 0 || height_abs == 0 || stride_abs == 0 {
                return None;
            }
            // PixelFormat encoding (sec 2.1.1.20). Common values:
            //   0x0021_0008 = 8bppIndexed     (not supported here)
            //   0x0022_0010 = 16bppRGB555
            //   0x0024_0010 = 16bppRGB565
            //   0x0021_8200 = 16bppGrayScale
            //   0x0028_1018 = 24bppRGB
            //   0x0026_2020 = 32bppARGB     (high byte 0x26: PixelFormatAlpha bit set)
            //   0x000E_2020 = 32bppRGB      (high byte 0x0E: no alpha — X channel is padding)
            //   0x000F_2020 = 32bppPARGB    (high byte 0x0F: premultiplied alpha)
            // For the BMP path we only need to map RGB / ARGB families.
            let bpp = (pixel_format & 0xFF00) >> 8;
            if bpp != 24 && bpp != 32 {
                return None;
            }
            let expected_bytes = stride_abs.checked_mul(height_abs)?;
            if bitmap_data.len() < expected_bytes {
                return None;
            }
            // For 32bpp pixel data we have to decide whether the X /
            // alpha byte is meaningful. `PowerPoint` mislabels EMF
            // payloads: pictures whose actual bitmap holds an alpha
            // channel (slide 96 / 100 / 61's icon illustrations) are
            // tagged 0x000E_xxxx ("32bppRGB — no alpha") even though
            // every transparent corner pixel sits at A=0. Detect real
            // alpha by sampling the data: if every X byte is 0x00 or
            // every X byte is 0xFF, the channel carries no information
            // and we collapse to opaque 24bpp BGR. Otherwise we keep
            // 32bpp + BI_BITFIELDS with an explicit alpha mask so the
            // BMP decoder does not throw the channel away.
            let alpha_state = if bpp == 32 {
                detect_alpha_state(bitmap_data, width_abs, height_abs, stride_abs)
            } else {
                AlphaState::None
            };
            let (final_bpp, compression, packed_bits) = match alpha_state {
                AlphaState::Variable => {
                    // 32bpp BI_BITFIELDS — caller injects the masks.
                    (
                        32u32,
                        3u32,
                        std::borrow::Cow::Borrowed(&bitmap_data[..expected_bytes]),
                    )
                }
                AlphaState::AllOpaque | AlphaState::AllTransparent | AlphaState::None
                    if bpp == 32 =>
                {
                    // X channel is uniform → meaningless. Repack as
                    // packed 24bpp BGR so downstream sees an opaque
                    // image (instead of a fully-transparent rectangle).
                    let new_stride = (width_abs * 3 + 3) & !3;
                    let mut packed = vec![0u8; new_stride * height_abs];
                    for row in 0..height_abs {
                        let src = &bitmap_data[row * stride_abs..row * stride_abs + width_abs * 4];
                        let dst = &mut packed[row * new_stride..row * new_stride + width_abs * 3];
                        for col in 0..width_abs {
                            dst[col * 3] = src[col * 4];
                            dst[col * 3 + 1] = src[col * 4 + 1];
                            dst[col * 3 + 2] = src[col * 4 + 2];
                        }
                    }
                    (24u32, 0u32, std::borrow::Cow::Owned(packed))
                }
                _ => (
                    bpp,
                    0u32,
                    std::borrow::Cow::Borrowed(&bitmap_data[..expected_bytes]),
                ),
            };
            // BITMAPV4HEADER (108 bytes) is required so we can declare
            // the four colour masks — the standard BITMAPINFOHEADER (40)
            // alpha-mask field is only consulted when biCompression is
            // BI_BITFIELDS, but the masks themselves live at offset 40
            // and beyond. Using V4 keeps the structure self-contained.
            let header_size = if compression == 3 { 108u32 } else { 40u32 };
            let mut dib = vec![0u8; header_size as usize];
            dib[0..4].copy_from_slice(&header_size.to_le_bytes()); // biSize
                                                                   // biHeight sign: top-down stride → negative biHeight.
            let bmp_height = if stride > 0 { -height } else { height };
            dib[4..8].copy_from_slice(&width.to_le_bytes());
            dib[8..12].copy_from_slice(&bmp_height.to_le_bytes());
            dib[12..14].copy_from_slice(&1u16.to_le_bytes()); // biPlanes
            dib[14..16].copy_from_slice(&(final_bpp as u16).to_le_bytes());
            dib[16..20].copy_from_slice(&compression.to_le_bytes()); // biCompression
            let len_u32 = u32::try_from(packed_bits.len()).ok()?;
            dib[20..24].copy_from_slice(&len_u32.to_le_bytes());
            if compression == 3 {
                // V4 colour masks for BGRA layout (little-endian byte
                // order: pixel bytes are B G R A). The 32-bit mask values
                // pick which bits each channel occupies.
                dib[40..44].copy_from_slice(&0x00FF_0000u32.to_le_bytes()); // bV4RedMask
                dib[44..48].copy_from_slice(&0x0000_FF00u32.to_le_bytes()); // bV4GreenMask
                dib[48..52].copy_from_slice(&0x0000_00FFu32.to_le_bytes()); // bV4BlueMask
                dib[52..56].copy_from_slice(&0xFF00_0000u32.to_le_bytes()); // bV4AlphaMask
                dib[56..60].copy_from_slice(&0x7352_4742u32.to_le_bytes()); // sRGB color space
            }
            build_bmp_file(&dib, &packed_bits)
        }
        _ => None,
    }
}

/// Result of scanning the X / alpha byte of a 32-bit pixel buffer.
enum AlphaState {
    /// At least one X byte differs from the others — real alpha.
    Variable,
    /// Every X byte equals 0xFF — opaque, X is padding.
    AllOpaque,
    /// Every X byte equals 0x00 — would render fully transparent if
    /// passed through; treat as padding instead.
    AllTransparent,
    /// Not 32bpp data.
    None,
}

fn detect_alpha_state(
    bitmap_data: &[u8],
    width: usize,
    height: usize,
    stride: usize,
) -> AlphaState {
    if width == 0 || height == 0 {
        return AlphaState::None;
    }
    let mut saw_zero = false;
    let mut saw_full = false;
    let mut saw_other = false;
    for row in 0..height {
        let start = row * stride;
        if start + width * 4 > bitmap_data.len() {
            break;
        }
        for col in 0..width {
            let a = bitmap_data[start + col * 4 + 3];
            match a {
                0 => saw_zero = true,
                0xFF => saw_full = true,
                _ => {
                    saw_other = true;
                    break;
                }
            }
        }
        if saw_other {
            break;
        }
    }
    if saw_other || (saw_zero && saw_full) {
        AlphaState::Variable
    } else if saw_full {
        AlphaState::AllOpaque
    } else if saw_zero {
        AlphaState::AllTransparent
    } else {
        AlphaState::None
    }
}

fn try_extract_dib(record: &[u8], dib_offset: usize) -> Option<Vec<u8>> {
    if record.len() < dib_offset + 16 {
        return None;
    }
    let off_bmi = read_u32_le(record, dib_offset)? as usize;
    let cb_bmi = read_u32_le(record, dib_offset + 4)? as usize;
    let off_bits = read_u32_le(record, dib_offset + 8)? as usize;
    let cb_bits = read_u32_le(record, dib_offset + 12)? as usize;

    if cb_bmi == 0 || cb_bits == 0 {
        return None;
    }
    let bmi_end = off_bmi.checked_add(cb_bmi)?;
    let bits_end = off_bits.checked_add(cb_bits)?;
    if bmi_end > record.len() || bits_end > record.len() {
        return None;
    }

    let dib_header = &record[off_bmi..bmi_end];
    let dib_bits = &record[off_bits..bits_end];
    build_bmp_file(dib_header, dib_bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimum-sized EMF carrying a single `EMR_STRETCHDIBITS` with a
    /// 1×1 24-bit bitmap. Used to exercise the parser end-to-end.
    fn synthetic_stretchdibits_emf() -> Vec<u8> {
        // -- EMR_HEADER (88 bytes) -------------------------------------
        let mut emf = Vec::new();
        // Type = 0x00000001
        emf.extend_from_slice(&1u32.to_le_bytes());
        // Size = 88 (placeholder - we'll overwrite below)
        emf.extend_from_slice(&88u32.to_le_bytes());
        // Bounds RECTL (16 bytes) + Frame (16) = 32 bytes
        emf.extend_from_slice(&[0u8; 32]);
        // Signature " EMF"
        emf.extend_from_slice(b" EMF");
        // Version = 0x00010000
        emf.extend_from_slice(&0x0001_0000u32.to_le_bytes());
        // Bytes (file size, computed later — placeholder)
        emf.extend_from_slice(&0u32.to_le_bytes());
        // Records (count, set later)
        emf.extend_from_slice(&2u32.to_le_bytes());
        // Handles
        emf.extend_from_slice(&0u16.to_le_bytes());
        // Reserved
        emf.extend_from_slice(&0u16.to_le_bytes());
        // nDescription
        emf.extend_from_slice(&0u32.to_le_bytes());
        // offDescription
        emf.extend_from_slice(&0u32.to_le_bytes());
        // nPalEntries
        emf.extend_from_slice(&0u32.to_le_bytes());
        // SizeL.cx, SizeL.cy
        emf.extend_from_slice(&100i32.to_le_bytes());
        emf.extend_from_slice(&100i32.to_le_bytes());
        // SizeMillimeters.cx,.cy
        emf.extend_from_slice(&26i32.to_le_bytes());
        emf.extend_from_slice(&26i32.to_le_bytes());
        assert_eq!(emf.len(), 88);

        // -- EMR_STRETCHDIBITS (80 hdr + 40 BITMAPINFOHEADER + 4 bits = 124) -
        let dib_off = emf.len();
        let record_size = 80 + 40 + 4;
        emf.extend_from_slice(&0x0000_0051u32.to_le_bytes()); // Type
        emf.extend_from_slice(&(record_size as u32).to_le_bytes()); // Size
        emf.extend_from_slice(&[0u8; 16]); // Bounds
        emf.extend_from_slice(&[0u8; 24]); // xDest, yDest, xSrc, ySrc, cxSrc, cySrc
                                           // offBmiSrc = 80, cbBmiSrc = 40, offBitsSrc = 120, cbBitsSrc = 4
        emf.extend_from_slice(&80u32.to_le_bytes());
        emf.extend_from_slice(&40u32.to_le_bytes());
        emf.extend_from_slice(&120u32.to_le_bytes());
        emf.extend_from_slice(&4u32.to_le_bytes());
        // UsageSrc, BitBltRasterOperation, cxDest, cyDest
        emf.extend_from_slice(&[0u8; 16]);
        // BITMAPINFOHEADER (40 bytes): 1×1, 24bpp.
        emf.extend_from_slice(&40u32.to_le_bytes()); // biSize
        emf.extend_from_slice(&1i32.to_le_bytes()); // biWidth
        emf.extend_from_slice(&1i32.to_le_bytes()); // biHeight
        emf.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
        emf.extend_from_slice(&24u16.to_le_bytes()); // biBitCount
        emf.extend_from_slice(&[0u8; 24]); // biCompression.. biClrImportant
                                           // 1 RGB pixel padded to 4-byte row.
        emf.extend_from_slice(&[0xFF, 0x00, 0x00, 0x00]);
        assert_eq!(emf.len() - dib_off, record_size);

        // -- EMR_EOF (20 bytes minimum) --------------------------------
        emf.extend_from_slice(&0x0000_000Eu32.to_le_bytes()); // Type
        emf.extend_from_slice(&20u32.to_le_bytes()); // Size
        emf.extend_from_slice(&[0u8; 12]);

        emf
    }

    #[test]
    fn extracts_bmp_from_synthetic_stretchdibits() {
        let emf = synthetic_stretchdibits_emf();
        let bmp = extract_raster_from_emf(&emf).expect("extract OK");
        assert_eq!(&bmp[..2], b"BM");
        // BITMAPFILEHEADER (14) + BITMAPINFOHEADER (40) + 4 pixel bytes
        assert_eq!(bmp.len(), 14 + 40 + 4);
    }

    #[test]
    fn rejects_too_short_input() {
        assert_eq!(
            extract_raster_from_emf(&[0u8; 10]),
            Err(EmfRasterError::TooShort)
        );
    }

    #[test]
    fn rejects_non_emf_input() {
        let mut bytes = vec![0u8; 50];
        bytes[40] = b'P';
        bytes[41] = b'N';
        bytes[42] = b'G';
        bytes[43] = b'!';
        // First record type is still 0 — not 1 — so this trips the
        // signature check first.
        assert_eq!(
            extract_raster_from_emf(&bytes),
            Err(EmfRasterError::NotAnEmf)
        );
    }
}
