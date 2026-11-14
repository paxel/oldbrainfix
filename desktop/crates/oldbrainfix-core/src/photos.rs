//! A stored photo never says where it was taken; the Note's own Capture
//! Context does that. The Dart core's `stripJpegMetadata`, byte for byte,
//! so every Device that strips the same file keeps the same bytes under
//! the same hash.

/// `jpeg` without its APP1..APP15 and COM segments (Exif with its GPS
/// position, XMP, ICC, comments). The picture data is untouched, no
/// re-encoding. Bytes that are not a JPEG, or a cut-off one, come back
/// as they are.
pub fn strip_jpeg_metadata(jpeg: &[u8]) -> Vec<u8> {
    let [0xFF, 0xD8, _, _, ..] = jpeg else {
        return jpeg.to_vec();
    };
    let mut out = vec![0xFF, 0xD8];
    let mut i = 2;
    let mut changed = false;
    while let Some(&[0xFF, marker, high, low]) = jpeg.get(i..i + 4) {
        if marker == 0xD9 || marker == 0xDA {
            // EOI: nothing follows; SOS: the scan runs to the end.
            break;
        }
        if marker == 0xFF {
            i += 1; // fill byte
            continue;
        }
        if (0xD0..=0xD7).contains(&marker) {
            out.extend_from_slice(&[0xFF, marker]); // RSTn carries no length
            i += 2;
            continue;
        }
        let length = (usize::from(high) << 8) | usize::from(low);
        let end = i + 2 + length;
        let Some(segment) = jpeg.get(i..end) else {
            return jpeg.to_vec(); // truncated: leave it alone
        };
        if (0xE1..=0xEF).contains(&marker) || marker == 0xFE {
            changed = true;
        } else {
            out.extend_from_slice(segment);
        }
        i = end;
    }
    if !changed {
        return jpeg.to_vec();
    }
    // `i` never passes the end: each step lands within the bytes.
    out.extend_from_slice(jpeg.get(i..).unwrap_or_default());
    out
}

#[cfg(test)]
mod tests;
