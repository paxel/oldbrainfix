use super::*;
use std::io::Write;

/// A zip of `parts`, each deflated as office programs write them.
pub(crate) fn zip_of(parts: &[(&str, &str)]) -> Vec<u8> {
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, text) in parts {
        out.start_file(*name, options).unwrap();
        out.write_all(text.as_bytes()).unwrap();
    }
    out.finish().unwrap().into_inner()
}

fn text(kind: FileKind, bytes: &[u8]) -> String {
    match read(kind, bytes) {
        FileReading::Text(text) => text,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_file_is_known_by_its_ending_else_by_its_media_type() {
    assert_eq!(kind_of("Brief.PDF", ""), FileKind::Pdf);
    assert_eq!(kind_of("a.docx", "application/zip"), FileKind::Docx);
    assert_eq!(kind_of("notes.md", ""), FileKind::Text);
    assert_eq!(kind_of("scan", "application/pdf"), FileKind::Pdf);
    assert_eq!(kind_of("plan.dwg", "application/pdf"), FileKind::Pdf);
    assert_eq!(kind_of("plan.dwg", ""), FileKind::Other);
    assert_eq!(kind_of("archive.zip", "application/zip"), FileKind::Other);
    assert_eq!(media_type_of("Brief.Pdf"), "application/pdf");
    assert_eq!(media_type_of("x"), "application/octet-stream");
}

#[test]
fn a_phone_reads_up_to_ten_pages_and_leaves_the_rest_to_a_computer() {
    assert!(!computer_reads(FileKind::Pdf, 1 << 30, Some(10)));
    assert!(computer_reads(FileKind::Pdf, 100, Some(11)));
    assert!(computer_reads(FileKind::Pdf, 100, None));
    assert!(!computer_reads(FileKind::Docx, PHONE_BYTES, None));
    assert!(computer_reads(FileKind::Text, PHONE_BYTES + 1, None));
    assert!(!computer_reads(FileKind::Other, 1 << 30, None));
}

#[test]
fn a_word_document_gives_its_paragraphs_without_deleted_text() {
    let xml = r#"<?xml version="1.0"?><w:document xmlns:w="x"><w:body>
            <w:p><w:r><w:t>Dear Ms Weber,</w:t></w:r></w:p>
            <w:p><w:r><w:t xml:space="preserve">Total: </w:t><w:tab/><w:t>12 &amp; 50 &#8364;</w:t></w:r>
            <w:del><w:r><w:delText>gone</w:delText></w:r></w:del><w:r><w:br/><w:t>next</w:t></w:r></w:p>
            <w:p/><w:p><w:r><w:instrText>PAGE</w:instrText><w:t><![CDATA[a<b]]></w:t></w:r></w:p>
            </w:body></w:document>"#;
    let bytes = zip_of(&[("word/document.xml", xml)]);
    assert_eq!(
        text(FileKind::Docx, &bytes),
        "Dear Ms Weber,\nTotal: \t12 & 50 €\nnext\n\na<b"
    );
}

#[test]
fn a_workbook_gives_its_sheets_in_order_with_their_rows() {
    let workbook = r#"<workbook xmlns:r="r"><sheets>
            <sheet name="Costs" sheetId="2" r:id="rId2"/><sheet name="Empty" r:id="rId3"/>
            <sheet name="Notes" sheetId="1" r:id="rId1"/></sheets></workbook>"#;
    let rels = r#"<Relationships><Relationship Id="rId1" Target="worksheets/sheet1.xml"/>
            <Relationship Id="rId2" Target="/xl/worksheets/sheet2.xml"/>
            <Relationship Id="rId3" Target="worksheets/sheet3.xml"/></Relationships>"#;
    let shared = r#"<sst><si><t>Rent</t></si><si><r><t>Pow</t></r><r><t>er</t></r>
            <rPh><t>ignored</t></rPh></si><si/></sst>"#;
    let costs = r#"<worksheet><sheetData>
            <row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1"><v>950.5</v></c></row>
            <row r="2"><c t="s"><v>1</v></c><c/><c t="b"><v>1</v></c><c/></row>
            <row r="3"><c t="inlineStr"><is><t>a
b</t></is></c></row><row r="4"/></sheetData></worksheet>"#;
    let notes =
        r#"<worksheet><sheetData><row><c t="str"><v>x</v></c></row></sheetData></worksheet>"#;
    let empty = r#"<worksheet><sheetData/></worksheet>"#;
    let bytes = zip_of(&[
        ("xl/workbook.xml", workbook),
        ("xl/_rels/workbook.xml.rels", rels),
        ("xl/sharedStrings.xml", shared),
        ("xl/worksheets/sheet1.xml", notes),
        ("xl/worksheets/sheet2.xml", costs),
        ("xl/worksheets/sheet3.xml", empty),
    ]);
    assert_eq!(
        text(FileKind::Xlsx, &bytes),
        "Costs\nRent\t950.5\nPower\t\tTRUE\na b\n\nNotes\nx"
    );
}

#[test]
fn an_opendocument_text_gives_its_paragraphs_with_their_spaces() {
    let xml = r#"<office:document-content><office:automatic-styles><style:style/>
            </office:automatic-styles><office:body><office:text>
            <text:tracked-changes><text:changed-region><text:deletion><text:p>old</text:p>
            </text:deletion></text:changed-region></text:tracked-changes>
            <text:h text:outline-level="1">Lease</text:h>
            <text:p>A<text:s text:c="3"/>B<text:tab/>C<text:line-break/>D <text:span>E</text:span></text:p>
            <text:p/><text:p>F<text:s/>G</text:p></office:text></office:body></office:document-content>"#;
    let bytes = zip_of(&[("mimetype", "x"), ("content.xml", xml)]);
    assert_eq!(text(FileKind::Odt, &bytes), "Lease\nA   B\tC\nD E\n\nF G");
}

#[test]
fn an_opendocument_spreadsheet_repeats_cells_and_rows_as_far_as_they_hold_values() {
    let xml = r#"<office:body><office:spreadsheet><table:table table:name="Plan">
            <table:table-row><table:table-cell><text:p>Mon</text:p></table:table-cell>
            <table:table-cell table:number-columns-repeated="2"><text:p>x</text:p></table:table-cell>
            <table:table-cell table:number-columns-repeated="1000"/></table:table-row>
            <table:table-row table:number-rows-repeated="1048000"><table:table-cell table:number-columns-repeated="1024"/></table:table-row>
            <table:table-row table:number-rows-repeated="2"><table:table-cell/><table:table-cell><text:p>a</text:p><text:p>b</text:p></table:table-cell></table:table-row>
            </table:table><table:table table:name="None"/></office:spreadsheet></office:body>"#;
    let bytes = zip_of(&[("content.xml", xml)]);
    assert_eq!(text(FileKind::Ods, &bytes), "Plan\nMon\tx\tx\n\ta b\n\ta b");
}

#[test]
fn a_text_file_reads_as_an_import_reads_it() {
    assert_eq!(
        text(
            FileKind::Text,
            b"\xef\xbb\xbf# Title\r\n\r\n\r\n\r\nline  \r\x07end\n"
        ),
        "# Title\n\nline\nend"
    );
    assert_eq!(text(FileKind::Text, b"Gr\xfc\xdfe \x80"), "Grüße €");
    assert_eq!(text(FileKind::Text, b"a\0b"), "");
}

#[test]
fn what_does_not_read_as_its_kind_gives_an_empty_text() {
    assert_eq!(text(FileKind::Docx, b"not a zip"), "");
    assert_eq!(text(FileKind::Xlsx, &zip_of(&[("other", "x")])), "");
    assert_eq!(
        text(FileKind::Odt, &zip_of(&[("content.xml", "<a><b>open")])),
        ""
    );
    assert_eq!(read(FileKind::Pdf, b"%PDF-1.4"), FileReading::Pdf);
    assert_eq!(read(FileKind::Other, b"x"), FileReading::Nothing);
}

#[test]
fn references_that_name_no_character_stay_as_written() {
    assert_eq!(
        unescape("&#0; &#xD800; &#x110000; &nbsp; &#65;&#x42;&lt;&;&amp"),
        "&#0; &#xD800; &#x110000; &nbsp; AB<&;&amp"
    );
}

#[test]
fn a_text_is_tidied_and_cut_after_a_million_characters() {
    assert_eq!(tidy("  a \t\n\n\n\n b\u{85}\u{1b}c \n"), "a\n\n bc");
    let long = "ä".repeat(MAX_TEXT_CHARS + 5);
    assert_eq!(tidy(&long).chars().count(), MAX_TEXT_CHARS);
    assert_eq!(
        join_pages(&["Page 1 ".into(), " \n".into(), "\nPage 3".into()]),
        "Page 1\n\nPage 3"
    );
}
