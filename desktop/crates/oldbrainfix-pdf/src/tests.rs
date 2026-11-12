use std::io::Write;

use super::*;
use crate::object::find;

fn flate(data: &[u8]) -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(data).unwrap();
    z.finish().unwrap()
}

/// A stream object's body: its dictionary with `extra` and `data`.
fn stream(extra: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< /Length {} {extra} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// A PDF of `objects`, numbered from 1, the first its catalog.
fn pdf(objects: &[Vec<u8>], trailer: &str) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R {trailer} >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn b(text: &str) -> Vec<u8> {
    text.as_bytes().to_vec()
}

/// One page with the font `font` as /F1 and `content`, deflated.
fn one_page(font: &str, content: &[u8]) -> Vec<u8> {
    pdf(
        &[
            b("<< /Type /Catalog /Pages 2 0 R >>"),
            b("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            b(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
            ),
            b(font),
            stream("/Filter /FlateDecode", &flate(content)),
        ],
        "",
    )
}

const HELVETICA: &str =
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>";

fn texts(bytes: &[u8]) -> Vec<String> {
    read(bytes).unwrap().into_iter().map(|p| p.text).collect()
}

#[test]
fn a_page_gives_its_lines_and_words() {
    let content = b"BT /F1 12 Tf 72 720 Td (Gr\\374\\337e,) Tj ( Welt) Tj 0 -14 Td \
        [(Hal) -20 (lo) -300 (Ann)] TJ 0 -14 Td (A) Tj 50 0 Td (B) Tj ET \
        BT 1 0 0 1 72 600 Tm (Total: 12,50 \\200) Tj T* ET";
    let bytes = one_page(HELVETICA, content);
    assert_eq!(page_count(&bytes), Some(1));
    assert_eq!(
        texts(&bytes),
        vec!["Grüße, Welt\nHallo Ann\nA B\nTotal: 12,50 €".to_string()]
    );
}

#[test]
fn a_composite_font_reads_through_its_tounicode_map() {
    let cmap = b"/CIDInit /ProcSet findresource begin begincmap\n\
        1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
        2 beginbfchar <0003> <0020> <0010> <00660069> endbfchar\n\
        1 beginbfrange <0024> <0026> <0041> endbfrange\nendcmap end";
    let bytes = pdf(
        &[
            b("<< /Type /Catalog /Pages 2 0 R >>"),
            b("<< /Type /Pages /Kids [3 0 R] /Count 1 /Resources << /Font << /F1 4 0 R >> >> >>"),
            b("<< /Type /Page /Parent 2 0 R /Contents 5 0 R >>"),
            b(
                "<< /Type /Font /Subtype /Type0 /Encoding /Identity-H /ToUnicode 6 0 R /DescendantFonts [7 0 R] >>",
            ),
            stream("", b"BT /F1 10 Tf 0 0 Td <00240025002600030010> Tj ET"),
            stream("", cmap),
            b("<< /Type /Font /Subtype /CIDFontType2 /DW 1000 /W [36 [600 600] 38 40 500] >>"),
        ],
        "",
    );
    assert_eq!(texts(&bytes), vec!["ABC fi".to_string()]);
}

#[test]
fn differences_name_their_glyphs() {
    let font = "<< /Type /Font /Subtype /Type1 /Encoding << /BaseEncoding /WinAnsiEncoding \
        /Differences [65 /adieresis /germandbls 200 /Euro /uni2013] >> >>";
    let bytes = one_page(font, b"BT /F1 12 Tf (AB\\310\\311x) Tj ET");
    assert_eq!(texts(&bytes), vec!["äß€–x".to_string()]);
}

#[test]
fn objects_in_object_streams_and_a_cross_reference_stream_are_read() {
    let parts = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
    ];
    let mut header = String::new();
    let mut body = String::new();
    for (i, part) in parts.iter().enumerate() {
        header.push_str(&format!("{} {} ", i + 1, body.len()));
        body.push_str(part);
        body.push(' ');
    }
    let objects = format!("{header}{body}");
    let first = header.len();
    let mut out = b"%PDF-1.5\n".to_vec();
    out.extend_from_slice(b"4 0 obj\n");
    out.extend_from_slice(&stream(
        "/Filter /FlateDecode",
        &flate(b"BT /F1 9 Tf (packed) Tj ET"),
    ));
    out.extend_from_slice(b"\nendobj\n5 0 obj\n");
    out.extend_from_slice(HELVETICA.as_bytes());
    out.extend_from_slice(b"\nendobj\n6 0 obj\n");
    out.extend_from_slice(&stream(
        &format!("/Type /ObjStm /N 3 /First {first} /Filter /FlateDecode"),
        &flate(objects.as_bytes()),
    ));
    out.extend_from_slice(b"\nendobj\n7 0 obj\n");
    out.extend_from_slice(&stream("/Type /XRef /Root 1 0 R /Size 8 /W [1 2 1]", b""));
    out.extend_from_slice(b"\nendobj\nstartxref\n0\n%%EOF\n");
    assert_eq!(texts(&out), vec!["packed".to_string()]);
}

#[test]
fn pages_come_in_order_with_inherited_resources_and_a_loop_is_left() {
    let bytes = pdf(
        &[
            b("<< /Type /Catalog /Pages 2 0 R >>"),
            b(
                "<< /Type /Pages /Kids [3 0 R 4 0 R 2 0 R] /Resources << /Font << /F1 6 0 R >> >> >>",
            ),
            b("<< /Type /Pages /Kids [5 0 R] >>"),
            b("<< /Type /Page /Contents [7 0 R 8 0 R] >>"),
            b("<< /Type /Page /Contents 9 0 R >>"),
            b(HELVETICA),
            stream("", b"BT /F1 12 Tf (two,) Tj"),
            stream("", b" ( continued) Tj ET"),
            stream("", b"BT /F1 12 Tf (one) Tj ET"),
        ],
        "",
    );
    assert_eq!(page_count(&bytes), Some(2));
    assert_eq!(
        texts(&bytes),
        vec!["one".to_string(), "two, continued".to_string()]
    );
}

#[test]
fn a_form_draws_its_text() {
    let bytes = pdf(
        &[
            b("<< /Type /Catalog /Pages 2 0 R >>"),
            b("<< /Type /Pages /Kids [3 0 R] >>"),
            b("<< /Type /Page /Resources << /XObject << /X1 5 0 R >> >> /Contents 4 0 R >>"),
            stream("", b"q 1 0 0 1 10 10 cm /X1 Do Q"),
            stream(
                "/Type /XObject /Subtype /Form /Resources << /Font << /F1 6 0 R >> >>",
                b"BT /F1 8 Tf (in a form) Tj ET",
            ),
            b(HELVETICA),
        ],
        "",
    );
    assert_eq!(texts(&bytes), vec!["in a form".to_string()]);
}

#[test]
fn a_scanned_page_hands_on_its_picture() {
    let jpeg = b"\xff\xd8\xff\xe0 a jpeg \xff\xd9";
    let gray: Vec<u8> = (0..200 * 200).map(|i| (i % 256) as u8).collect();
    let bytes = pdf(
        &[
            b("<< /Type /Catalog /Pages 2 0 R >>"),
            b("<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] >>"),
            b(
                "<< /Type /Page /Resources << /XObject << /Im 6 0 R /Small 8 0 R >> >> /Contents 9 0 R >>",
            ),
            b("<< /Type /Page /Resources << /XObject << /Im 7 0 R >> >> /Contents 10 0 R >>"),
            b("<< /Type /Page /Resources << /XObject << /Im 8 0 R >> >> /Contents 10 0 R >>"),
            stream(
                "/Type /XObject /Subtype /Image /Width 300 /Height 300 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode",
                jpeg,
            ),
            stream(
                "/Type /XObject /Subtype /Image /Width 200 /Height 200 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode",
                &flate(&gray),
            ),
            stream(
                "/Type /XObject /Subtype /Image /Width 10 /Height 10 /ColorSpace /DeviceGray /BitsPerComponent 8",
                &[0; 100],
            ),
            stream("", b"q 595 0 0 842 0 0 cm /Small Do /Im Do Q"),
            stream("", b"q 595 0 0 842 0 0 cm /Im Do Q"),
        ],
        "",
    );
    let pages = read(&bytes).unwrap();
    assert_eq!(pages[0].scan.as_deref(), Some(&jpeg[..]));
    let png = image::load_from_memory(pages[1].scan.as_ref().unwrap()).unwrap();
    assert_eq!((png.width(), png.height()), (200, 200));
    assert_eq!(png.to_luma8().as_raw(), &gray);
    // Too small to be a scan.
    assert_eq!(pages[2].scan, None);
    assert!(pages.iter().all(|p| p.text.is_empty()));
}

#[test]
fn a_page_with_text_hands_on_no_picture() {
    let bytes = pdf(
        &[
            b("<< /Type /Catalog /Pages 2 0 R >>"),
            b("<< /Type /Pages /Kids [3 0 R] >>"),
            b(
                "<< /Type /Page /Resources << /Font << /F1 4 0 R >> /XObject << /Im 5 0 R >> >> /Contents 6 0 R >>",
            ),
            b(HELVETICA),
            stream(
                "/Subtype /Image /Width 300 /Height 300 /Filter /DCTDecode",
                b"\xff\xd8",
            ),
            stream("", b"/Im Do BT /F1 9 Tf 3 Tr (OCR layer) Tj ET"),
        ],
        "",
    );
    assert_eq!(
        read(&bytes).unwrap(),
        vec![Page {
            text: "OCR layer".into(),
            scan: None
        }]
    );
}

#[test]
fn an_encrypted_file_is_counted_but_not_read() {
    let mut bytes = one_page(HELVETICA, b"BT /F1 12 Tf (secret) Tj ET");
    let trailer = find(&bytes, b"trailer", 0).unwrap();
    bytes.splice(
        trailer + 10..trailer + 10,
        b"/Encrypt 9 0 R ".iter().copied(),
    );
    assert_eq!(page_count(&bytes), Some(1));
    assert_eq!(read(&bytes), None);
}

#[test]
fn what_is_no_pdf_or_damaged_reads_as_far_as_it_goes() {
    assert_eq!(read(b"PK\x03\x04 a zip"), None);
    assert_eq!(page_count(b""), None);
    let bytes = one_page(HELVETICA, b"BT /F1 12 Tf (kept) Tj ET");
    for cut in [10, bytes.len() / 3, bytes.len() / 2, bytes.len() - 30] {
        let _ = read(&bytes[..cut]);
    }
    // A stream whose length is wrong, or refers to itself.
    let bytes = pdf(
        &[
            b("<< /Type /Catalog /Pages 2 0 R >>"),
            b("<< /Type /Pages /Kids [3 0 R] >>"),
            b("<< /Type /Page /Resources << /Font << /F1 4 0 R >> >> /Contents [5 0 R 6 0 R] >>"),
            b(HELVETICA),
            b("<< /Length 9999 >>\nstream\nBT /F1 12 Tf (long) Tj ET\nendstream"),
            b("<< /Length 6 0 R >>\nstream\nBT /F1 12 Tf 0 -20 Td (self) Tj ET\nendstream"),
        ],
        "",
    );
    assert_eq!(texts(&bytes), vec!["long\nself".to_string()]);
}

#[test]
fn bytes_inside_a_stream_are_never_taken_for_an_object() {
    let bytes = pdf(
        &[
            b("<< /Type /Catalog /Pages 2 0 R >>"),
            b("<< /Type /Pages /Kids [3 0 R] >>"),
            b("<< /Type /Page /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"),
            b(HELVETICA),
            stream(
                "",
                b"BT /F1 12 Tf (real) Tj ET % 3 0 obj << /Type /Page /Contents 9 0 R >> endobj",
            ),
        ],
        "",
    );
    assert_eq!(texts(&bytes), vec!["real".to_string()]);
}

#[test]
fn an_inline_image_is_stepped_over() {
    let bytes = one_page(
        HELVETICA,
        b"BI /W 2 /H 2 /BPC 8 /CS /G ID \x00(Tj)\xff EI BT /F1 12 Tf (after) Tj ET",
    );
    assert_eq!(texts(&bytes), vec!["after".to_string()]);
}

#[test]
fn a_picture_too_large_to_hold_is_no_scan() {
    let bytes = pdf(
        &[
            b("<< /Type /Catalog /Pages 2 0 R >>"),
            b("<< /Type /Pages /Kids [3 0 R] >>"),
            b("<< /Type /Page /Resources << /XObject << /Im 4 0 R >> >> /Contents 5 0 R >>"),
            stream(
                "/Subtype /Image /Width 4611686018427387904 /Height 1 /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Filter /FlateDecode",
                &flate(&[0; 64]),
            ),
            stream("", b"/Im Do"),
        ],
        "",
    );
    assert_eq!(
        read(&bytes),
        Some(vec![Page {
            text: String::new(),
            scan: None
        }])
    );
}

#[test]
fn one_stream_named_again_and_again_is_read_up_to_a_limit() {
    let spaces = flate(&vec![b' '; 1 << 20]);
    let refs = "5 0 R ".repeat(100);
    let bytes = pdf(
        &[
            b("<< /Type /Catalog /Pages 2 0 R >>"),
            b("<< /Type /Pages /Kids [3 0 R] >>"),
            b(&format!("<< /Type /Page /Contents [{refs}] >>")),
            b("<< >>"),
            stream("/Filter /FlateDecode", &spaces),
        ],
        "",
    );
    let pdf = Pdf::open(&bytes).unwrap();
    let page = &pdf.pages[0];
    assert!(pdf.contents(page).len() as u64 <= file::MAX_DECODED);
    assert_eq!(pdf.page(0).map(|p| p.text), Some(String::new()));
}

#[test]
fn names_and_numbers_referred_to_are_followed() {
    let jpeg = b"\xff\xd8\xff\xe0 referred \xff\xd9";
    let bytes = pdf(
        &[
            b("<< /Type 6 0 R /Pages 2 0 R >>"),
            b("<< /Type /Pages /Kids [3 0 R] >>"),
            b("<< /Type /Page /Resources << /XObject << /Im0 4 0 R >> >> /Contents 5 0 R >>"),
            stream(
                "/Subtype 7 0 R /Width 8 0 R /Height 8 0 R /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter 9 0 R",
                jpeg,
            ),
            stream("", b"q 595 0 0 842 0 0 cm /Im0 Do Q"),
            b("/Catalog"),
            b("/Image"),
            b("400"),
            b("/DCTDecode"),
        ],
        "",
    );
    let pages = read(&bytes).unwrap();
    assert_eq!(pages[0].scan.as_deref(), Some(&jpeg[..]));
}
