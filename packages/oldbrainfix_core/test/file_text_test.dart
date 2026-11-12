import 'dart:convert';
import 'dart:typed_data';

import 'package:archive/archive.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// A zip of [parts], each deflated as office programs write them.
Uint8List zipOf(Map<String, String> parts) {
  final archive = Archive();
  for (final MapEntry(:key, :value) in parts.entries) {
    archive.add(
      ArchiveFile.bytes(key, utf8.encode(value))
        ..compression = CompressionType.deflate
        ..lastModTime = 0,
    );
  }
  return ZipEncoder().encodeBytes(archive);
}

String textOf(FileKind kind, List<int> bytes) =>
    switch (FileTexts.read(kind, Uint8List.fromList(bytes))) {
      TextRead(:final text) => text,
      final other => fail('$other'),
    };

void main() {
  test('a file is known by its ending, else by its media type', () {
    expect(FileTexts.kindOf('Brief.PDF', ''), FileKind.pdf);
    expect(FileTexts.kindOf('a.docx', 'application/zip'), FileKind.docx);
    expect(FileTexts.kindOf('notes.md', ''), FileKind.text);
    expect(FileTexts.kindOf('scan', 'application/pdf'), FileKind.pdf);
    expect(FileTexts.kindOf('plan.dwg', 'application/pdf'), FileKind.pdf);
    expect(FileTexts.kindOf('plan.dwg', ''), FileKind.other);
    expect(FileTexts.kindOf('a.zip', 'application/zip'), FileKind.other);
    expect(FileTexts.mediaTypeOf('Brief.Pdf'), 'application/pdf');
    expect(FileTexts.mediaTypeOf('x'), 'application/octet-stream');
  });

  test('a phone reads up to ten pages and leaves the rest to a computer', () {
    expect(FileTexts.computerReads(FileKind.pdf, 1 << 30, 10), isFalse);
    expect(FileTexts.computerReads(FileKind.pdf, 100, 11), isTrue);
    expect(FileTexts.computerReads(FileKind.pdf, 100, null), isTrue);
    expect(
      FileTexts.computerReads(FileKind.docx, FileTexts.phoneBytes, null),
      isFalse,
    );
    expect(
      FileTexts.computerReads(FileKind.text, FileTexts.phoneBytes + 1, null),
      isTrue,
    );
    expect(FileTexts.computerReads(FileKind.other, 1 << 30, null), isFalse);
  });

  test('a Word document gives its paragraphs without deleted text', () {
    const xml =
        '<?xml version="1.0"?><w:document xmlns:w="x"><w:body>'
        '<w:p><w:r><w:t>Dear Ms Weber,</w:t></w:r></w:p>'
        '<w:p><w:r><w:t xml:space="preserve">Total: </w:t><w:tab/>'
        '<w:t>12 &amp; 50 &#8364;</w:t></w:r>'
        '<w:del><w:r><w:delText>gone</w:delText></w:r></w:del>'
        '<w:r><w:br/><w:t>next</w:t></w:r></w:p>'
        '<w:p/><w:p><w:r><w:instrText>PAGE</w:instrText>'
        '<w:t><![CDATA[a<b]]></w:t></w:r></w:p>'
        '</w:body></w:document>';
    expect(
      textOf(FileKind.docx, zipOf({'word/document.xml': xml})),
      'Dear Ms Weber,\nTotal: \t12 & 50 €\nnext\n\na<b',
    );
  });

  test('a workbook gives its sheets in order with their rows', () {
    final bytes = zipOf({
      'xl/workbook.xml':
          '<workbook xmlns:r="r"><sheets>'
          '<sheet name="Costs" sheetId="2" r:id="rId2"/>'
          '<sheet name="Empty" r:id="rId3"/>'
          '<sheet name="Notes" sheetId="1" r:id="rId1"/></sheets></workbook>',
      'xl/_rels/workbook.xml.rels':
          '<Relationships>'
          '<Relationship Id="rId1" Target="worksheets/sheet1.xml"/>'
          '<Relationship Id="rId2" Target="/xl/worksheets/sheet2.xml"/>'
          '<Relationship Id="rId3" Target="worksheets/sheet3.xml"/>'
          '</Relationships>',
      'xl/sharedStrings.xml':
          '<sst><si><t>Rent</t></si><si><r><t>Pow</t></r><r><t>er</t></r>'
          '<rPh><t>ignored</t></rPh></si><si/></sst>',
      'xl/worksheets/sheet1.xml':
          '<worksheet><sheetData><row><c t="str"><v>x</v></c></row>'
          '</sheetData></worksheet>',
      'xl/worksheets/sheet2.xml':
          '<worksheet><sheetData>'
          '<row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1"><v>950.5</v></c>'
          '</row><row r="2"><c t="s"><v>1</v></c><c/><c t="b"><v>1</v></c>'
          '<c/></row><row r="3"><c t="inlineStr"><is><t>a\nb</t></is></c>'
          '</row><row r="4"/></sheetData></worksheet>',
      'xl/worksheets/sheet3.xml': '<worksheet><sheetData/></worksheet>',
    });
    expect(
      textOf(FileKind.xlsx, bytes),
      'Costs\nRent\t950.5\nPower\t\tTRUE\na b\n\nNotes\nx',
    );
  });

  test('an OpenDocument text gives its paragraphs with their spaces', () {
    const xml =
        '<office:document-content><office:automatic-styles><style:style/>'
        '</office:automatic-styles><office:body><office:text>'
        '<text:tracked-changes><text:changed-region><text:deletion>'
        '<text:p>old</text:p></text:deletion></text:changed-region>'
        '</text:tracked-changes>'
        '<text:h text:outline-level="1">Lease</text:h>'
        '<text:p>A<text:s text:c="3"/>B<text:tab/>C<text:line-break/>D '
        '<text:span>E</text:span></text:p>'
        '<text:p/><text:p>F<text:s/>G</text:p></office:text></office:body>'
        '</office:document-content>';
    expect(
      textOf(FileKind.odt, zipOf({'mimetype': 'x', 'content.xml': xml})),
      'Lease\nA   B\tC\nD E\n\nF G',
    );
  });

  test('an OpenDocument spreadsheet repeats cells and rows as far as they '
      'hold values', () {
    const xml =
        '<office:body><office:spreadsheet><table:table table:name="Plan">'
        '<table:table-row><table:table-cell><text:p>Mon</text:p>'
        '</table:table-cell><table:table-cell '
        'table:number-columns-repeated="2"><text:p>x</text:p>'
        '</table:table-cell><table:table-cell '
        'table:number-columns-repeated="1000"/></table:table-row>'
        '<table:table-row table:number-rows-repeated="1048000">'
        '<table:table-cell table:number-columns-repeated="1024"/>'
        '</table:table-row><table:table-row table:number-rows-repeated="2">'
        '<table:table-cell/><table:table-cell><text:p>a</text:p>'
        '<text:p>b</text:p></table:table-cell></table:table-row>'
        '</table:table><table:table table:name="None"/></office:spreadsheet>'
        '</office:body>';
    expect(
      textOf(FileKind.ods, zipOf({'content.xml': xml})),
      'Plan\nMon\tx\tx\n\ta b\n\ta b',
    );
  });

  test('a text file reads as an Import reads it', () {
    expect(
      textOf(FileKind.text, [
        0xef, 0xbb, 0xbf, ...utf8.encode('# Title\r\n\r\n\r\n\r\nline  \r'),
        0x07, ...utf8.encode('end\n'), //
      ]),
      '# Title\n\nline\nend',
    );
    expect(
      textOf(FileKind.text, [
        ...utf8.encode('Gr'),
        0xfc,
        0xdf,
        0x65,
        0x20,
        0x80,
      ]),
      'Grüße €',
    );
    expect(textOf(FileKind.text, [0x61, 0, 0x62]), '');
  });

  test('what does not read as its kind gives an empty text', () {
    expect(textOf(FileKind.docx, utf8.encode('not a zip')), '');
    expect(textOf(FileKind.xlsx, zipOf({'other': 'x'})), '');
    expect(textOf(FileKind.odt, zipOf({'content.xml': '<a><b>open'})), '');
    expect(
      FileTexts.read(FileKind.pdf, Uint8List.fromList(utf8.encode('%PDF'))),
      isA<PdfToRead>(),
    );
    expect(FileTexts.read(FileKind.other, Uint8List(1)), isA<NoTextRead>());
  });

  test('references that name no character stay as written', () {
    expect(
      textOf(
        FileKind.docx,
        zipOf({
          'word/document.xml':
              '<w:p><w:t>&#0; &#xD800; &#x110000; &nbsp; &#65;&#x42;&lt;'
              '&;&amp</w:t></w:p>',
        }),
      ),
      '&#0; &#xD800; &#x110000; &nbsp; AB<&;&amp',
    );
  });

  test('a text is tidied and cut after a million characters', () {
    expect(FileTexts.tidy('  a \t\n\n\n\n b\u0085\u001bc \n'), 'a\n\n bc');
    final long = 'ä' * (FileTexts.maxTextChars + 5);
    expect(FileTexts.tidy(long).runes, hasLength(FileTexts.maxTextChars));
    expect(
      FileTexts.joinPages(['Page 1 ', ' \n', '\nPage 3']),
      'Page 1\n\nPage 3',
    );
  });
}
