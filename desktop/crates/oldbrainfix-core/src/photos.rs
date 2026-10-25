//! A stored photo never says where it was taken; the Note's own Capture
//! Context does that. The Dart core's `stripJpegMetadata`, byte for byte,
//! so every Device that strips the same file keeps the same bytes under
//! the same hash.

/// `jpeg` without its APP1..APP15 and COM segments (Exif with its GPS
/// position, XMP, ICC, comments). The picture data is untouched, no
/// re-encoding. Bytes that are not a JPEG, or a cut-off one, come back
/// as they are.
pub fn strip_jpeg_metadata(jpeg: &[u8]) -> Vec<u8> {
    if jpeg.len() < 4 || jpeg[0] != 0xFF || jpeg[1] != 0xD8 {
        return jpeg.to_vec();
    }
    let mut out = vec![0xFF, 0xD8];
    let mut i = 2;
    let mut changed = false;
    while i + 4 <= jpeg.len() && jpeg[i] == 0xFF {
        let marker = jpeg[i + 1];
        if marker == 0xD9 || marker == 0xDA {
            // EOI: nothing follows; SOS: the scan runs to the end.
            break;
        }
        if marker == 0xFF {
            i += 1; // fill byte
            continue;
        }
        if (0xD0..=0xD7).contains(&marker) {
            out.extend_from_slice(&jpeg[i..i + 2]); // RSTn carries no length
            i += 2;
            continue;
        }
        let length = (usize::from(jpeg[i + 2]) << 8) | usize::from(jpeg[i + 3]);
        let end = i + 2 + length;
        if end > jpeg.len() {
            return jpeg.to_vec(); // truncated: leave it alone
        }
        if (0xE1..=0xEF).contains(&marker) || marker == 0xFE {
            changed = true;
        } else {
            out.extend_from_slice(&jpeg[i..end]);
        }
        i = end;
    }
    if !changed {
        return jpeg.to_vec();
    }
    out.extend_from_slice(&jpeg[i..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A segment with `marker` and `body`.
    fn segment(marker: u8, body: &[u8]) -> Vec<u8> {
        let length = (body.len() + 2) as u16;
        let mut out = vec![0xFF, marker];
        out.extend_from_slice(&length.to_be_bytes());
        out.extend_from_slice(body);
        out
    }

    fn jpeg(segments: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![0xFF, 0xD8];
        for s in segments {
            out.extend_from_slice(s);
        }
        out.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x02, 1, 2, 3, 0xFF, 0xD9]);
        out
    }

    #[test]
    fn exif_xmp_and_comments_go_and_the_picture_stays() {
        let app0 = segment(0xE0, b"JFIF\0");
        let dqt = segment(0xDB, &[0; 5]);
        let with = jpeg(&[
            app0.clone(),
            segment(0xE1, b"Exif\0\0GPS"),
            segment(0xFE, b"a comment"),
            segment(0xE2, b"ICC"),
            dqt.clone(),
        ]);
        assert_eq!(strip_jpeg_metadata(&with), jpeg(&[app0, dqt]));
    }

    #[test]
    fn what_is_no_jpeg_or_cut_off_or_clean_comes_back_as_it_is() {
        assert_eq!(strip_jpeg_metadata(b"PNG...."), b"PNG....");
        let clean = jpeg(&[segment(0xE0, b"JFIF\0")]);
        assert_eq!(strip_jpeg_metadata(&clean), clean);
        let mut cut = vec![0xFF, 0xD8];
        cut.extend_from_slice(&segment(0xE1, b"Exif")[..5]);
        cut.extend_from_slice(&[0, 0]);
        assert_eq!(strip_jpeg_metadata(&cut), cut);
    }
}
