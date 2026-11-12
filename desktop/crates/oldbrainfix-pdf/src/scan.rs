//! A scanned page's picture as a file the OCR reads: a JPEG as it lies
//! in the PDF, or gray or colour pixels as a PNG. Pictures in other
//! encodings (JPEG 2000, JBIG2, CCITT fax) are not read here.

use std::io::Write;

use crate::file::{File, MAX_DECODED};
use crate::object::{Dict, Obj};

/// The fewest pixels a picture needs to be taken for a page's scan.
pub const MIN_PIXELS: u64 = 200 * 200;

/// The picture `number` as a JPEG or a PNG; `None` when it is in an
/// encoding not read here.
pub fn picture(file: &File, number: u32) -> Option<Vec<u8>> {
    let object = file.get(number)?;
    let Obj::Stream(dict, _) = object.as_ref() else {
        return None;
    };
    let (filters, _) = file.filters(dict);
    if matches!(
        filters.last().map(Vec::as_slice),
        Some(b"DCTDecode" | b"DCT")
    ) {
        return file.decoded_until(&object, filters.len() - 1);
    }
    if filters.iter().any(|f| f != b"FlateDecode" && f != b"Fl") {
        return None;
    }
    let pixels = file.decoded(&object)?;
    raw_png(file, dict, &pixels)
}

/// The decoded `pixels` of the picture `dict` as a PNG: 8-bit gray, RGB
/// or CMYK, or 1-bit gray, as gray or RGB.
fn raw_png(file: &File, dict: &Dict, pixels: &[u8]) -> Option<Vec<u8>> {
    let width = usize::try_from(file.number_in(dict, b"Width")?).ok()?;
    let height = usize::try_from(file.number_in(dict, b"Height")?).ok()?;
    let bytes = (width as u64)
        .checked_mul(height as u64)
        .and_then(|p| p.checked_mul(4));
    if width == 0 || height == 0 || bytes.is_none_or(|b| b > MAX_DECODED) {
        return None;
    }
    let mask = dict
        .get(b"ImageMask")
        .and_then(|m| file.resolve(m))
        .is_some_and(|m| *m == Obj::Bool(true));
    let bits = if mask {
        1
    } else {
        file.number_in(dict, b"BitsPerComponent").unwrap_or(8)
    };
    let components = if mask { 1 } else { components(file, dict)? };
    let inverted = dict
        .get(b"Decode")
        .and_then(|d| file.resolve(d))
        .and_then(|d| {
            d.as_array()
                .and_then(|a| a.first())
                .and_then(Obj::as_number)
        })
        .is_some_and(|first| first > 0.5);
    let (channels, rows): (usize, Vec<Vec<u8>>) = match (bits, components) {
        (8, 1 | 3) => {
            let row = width * components;
            let rows = pixels
                .chunks(row)
                .take(height)
                .map(<[u8]>::to_vec)
                .collect();
            (components, rows)
        }
        (8, 4) => {
            let rows = pixels
                .chunks(width * 4)
                .take(height)
                .map(|r| {
                    r.chunks(4)
                        .flat_map(|p| match p {
                            [c, m, y, k] => {
                                let rgb = |v: u8| {
                                    ((255 - u16::from(v)) * (255 - u16::from(*k)) / 255) as u8
                                };
                                [rgb(*c), rgb(*m), rgb(*y)]
                            }
                            _ => [255, 255, 255],
                        })
                        .collect()
                })
                .collect();
            (3, rows)
        }
        (1, 1) => {
            let row = width.div_ceil(8);
            let rows = pixels
                .chunks(row)
                .take(height)
                .map(|r| {
                    (0..width)
                        .map(|x| {
                            // An image mask paints black where its bit
                            // is 0, as gray does.
                            let bit = r.get(x / 8).is_some_and(|b| b >> (7 - x % 8) & 1 == 1);
                            if bit { 255 } else { 0 }
                        })
                        .collect()
                })
                .collect();
            (1, rows)
        }
        _ => return None,
    };
    if rows.len() < height || rows.iter().any(|r| r.len() < width * channels) {
        return None;
    }
    let rows = if inverted {
        rows.into_iter()
            .map(|r| r.into_iter().map(|v| 255 - v).collect())
            .collect()
    } else {
        rows
    };
    png(width, height, channels, &rows)
}

/// How many components a colour of the picture `dict` has: gray, RGB
/// or CMYK, by name or by an ICC profile's count.
fn components(file: &File, dict: &Dict) -> Option<usize> {
    let space = file.resolve(dict.get(b"ColorSpace")?)?;
    match space.as_ref() {
        Obj::Name(n) => match n.as_slice() {
            b"DeviceGray" | b"CalGray" | b"G" => Some(1),
            b"DeviceRGB" | b"CalRGB" | b"RGB" => Some(3),
            b"DeviceCMYK" | b"CMYK" => Some(4),
            _ => None,
        },
        Obj::Array(items) => match items.first().and_then(Obj::as_name) {
            Some(b"ICCBased") => {
                let profile = file.resolve(items.get(1)?)?;
                let n = file.number_in(profile.as_dict()?, b"N")?;
                matches!(n, 1 | 3 | 4).then_some(n as usize)
            }
            Some(b"CalGray") => Some(1),
            Some(b"CalRGB") => Some(3),
            _ => None,
        },
        _ => None,
    }
}

/// `rows` of `channels` (1 gray, 3 RGB) bytes a pixel as a PNG.
pub fn png(width: usize, height: usize, channels: usize, rows: &[Vec<u8>]) -> Option<Vec<u8>> {
    let mut raw = Vec::with_capacity(height * (width * channels + 1));
    for row in rows.iter().take(height) {
        raw.push(0);
        raw.extend_from_slice(row.get(..width * channels)?);
    }
    let mut zlib = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    zlib.write_all(&raw).ok()?;
    let data = zlib.finish().ok()?;
    let mut header = Vec::new();
    header.extend_from_slice(&u32::try_from(width).ok()?.to_be_bytes());
    header.extend_from_slice(&u32::try_from(height).ok()?.to_be_bytes());
    header.extend_from_slice(&[8, if channels == 1 { 0 } else { 2 }, 0, 0, 0]);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    for (kind, body) in [
        (b"IHDR", header.as_slice()),
        (b"IDAT", &data),
        (b"IEND", &[]),
    ] {
        out.extend_from_slice(&u32::try_from(body.len()).ok()?.to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        let mut crc = crc32fast::Hasher::new();
        crc.update(kind);
        crc.update(body);
        out.extend_from_slice(&crc.finalize().to_be_bytes());
    }
    Some(out)
}
