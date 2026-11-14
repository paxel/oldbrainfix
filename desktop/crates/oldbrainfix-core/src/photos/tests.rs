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
