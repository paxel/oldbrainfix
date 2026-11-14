import 'dart:convert';
import 'dart:typed_data';

import 'package:archive/archive.dart';

import 'imports.dart';

/// What an attached file is, as far as its text goes (ADR 0047).
enum FileKind {
  pdf,

  /// A Word document, `.docx`.
  docx,

  /// An Excel workbook, `.xlsx`.
  xlsx,

  /// An OpenDocument text, `.odt`.
  odt,

  /// An OpenDocument spreadsheet, `.ods`.
  ods,

  /// Plain text or Markdown.
  text,

  /// Any other file: kept without text.
  other;

  /// Whether a text is read out of a file of this kind.
  bool get hasText => this != other;
}

/// What reading a file gives.
sealed class FileReading {
  const FileReading();
}

/// Its text, ready to keep; empty when none was found.
final class TextRead extends FileReading {
  final String text;

  const TextRead(this.text);

  @override
  bool operator ==(Object other) => other is TextRead && other.text == text;

  @override
  int get hashCode => text.hashCode;
}

/// A PDF: the phone's platform reads it (the desktop's `pdf` crate on
/// the phone too), its scanned pages through the OCR, and the page texts
/// are joined by [FileTexts.joinPages].
final class PdfToRead extends FileReading {
  const PdfToRead();
}

/// No text is read out of a file of this kind.
final class NoTextRead extends FileReading {
  const NoTextRead();
}

/// The text of an attached file (ADR 0047), as the desktop core's
/// `file_text.rs` reads it, character for character: what a file is by
/// its name and media type, who reads its text (Q645), and the text of
/// `.docx`, `.xlsx`, `.odt` and `.ods` files (zip and XML, text only) and
/// of plain text and Markdown. Any other file is kept without text.
///
/// Nothing here throws: a file that does not read as its kind says gives
/// an empty text, which records that nothing was found.
abstract final class FileTexts {
  /// The most pages of a PDF the phone reads itself (Q645); a longer one
  /// is left to a computer.
  static const phonePages = 10;

  /// The largest file of another kind with text the phone reads itself.
  static const phoneBytes = 10 << 20;

  /// The most an Attachment Text holds, in characters; the rest of a
  /// longer one is cut off.
  static const maxTextChars = 1000000;

  /// The most bytes a part of a zip is read to.
  static const _maxPartBytes = 32 << 20;

  /// The most sheets of a workbook read.
  static const _maxSheets = 256;

  /// The most times a repeated cell or row of an OpenDocument
  /// spreadsheet is written out.
  static const _maxRepeat = 100;

  /// The ending of the file [name] after its last dot, in lower case
  /// of ASCII letters.
  static String _ending(String name) {
    final dot = name.lastIndexOf('.');
    return dot < 0 ? '' : _asciiLower(name.substring(dot + 1));
  }

  static String _asciiLower(String text) => String.fromCharCodes([
    for (final c in text.codeUnits) c >= 0x41 && c <= 0x5a ? c + 0x20 : c,
  ]);

  /// The kind of the file [name] with the media type [mediaType]: by the
  /// ending of its name in any case of ASCII letters, else by its media
  /// type.
  static FileKind kindOf(String name, String mediaType) => switch (_ending(
    name,
  )) {
    'pdf' => FileKind.pdf,
    'docx' => FileKind.docx,
    'xlsx' => FileKind.xlsx,
    'odt' => FileKind.odt,
    'ods' => FileKind.ods,
    'txt' || 'text' || 'md' || 'markdown' || 'csv' || 'tsv' => FileKind.text,
    _ => switch (mediaType) {
      'application/pdf' => FileKind.pdf,
      'application/vnd.openxmlformats-officedocument.wordprocessingml.document' =>
        FileKind.docx,
      'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' =>
        FileKind.xlsx,
      'application/vnd.oasis.opendocument.text' => FileKind.odt,
      'application/vnd.oasis.opendocument.spreadsheet' => FileKind.ods,
      'text/plain' ||
      'text/markdown' ||
      'text/csv' ||
      'text/tab-separated-values' => FileKind.text,
      _ => FileKind.other,
    },
  };

  /// The media type of the file [name] by the ending of its name, for a
  /// file no app named one for; `application/octet-stream` when not
  /// known.
  static String mediaTypeOf(String name) => switch (_ending(name)) {
    'pdf' => 'application/pdf',
    'docx' =>
      'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    'xlsx' =>
      'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    'odt' => 'application/vnd.oasis.opendocument.text',
    'ods' => 'application/vnd.oasis.opendocument.spreadsheet',
    'txt' || 'text' => 'text/plain',
    'md' || 'markdown' => 'text/markdown',
    'csv' => 'text/csv',
    'tsv' => 'text/tab-separated-values',
    'jpg' || 'jpeg' => 'image/jpeg',
    'png' => 'image/png',
    'zip' => 'application/zip',
    _ => 'application/octet-stream',
  };

  /// Whether only a computer reads the text of a file of [kind], [size]
  /// bytes and [pages] pages (Q645): a PDF of more than [phonePages] pages
  /// or whose pages were not counted, any other file with text of more
  /// than [phoneBytes]. The attaching Device writes it down, so every
  /// phone leaves the file alone until a computer has read it.
  static bool computerReads(FileKind kind, int size, int? pages) =>
      switch (kind) {
        FileKind.other => false,
        FileKind.pdf => pages == null || pages > phonePages,
        _ => size > phoneBytes,
      };

  /// The text of the file [bytes] of [kind].
  static FileReading read(FileKind kind, Uint8List bytes) => switch (kind) {
    FileKind.pdf => const PdfToRead(),
    FileKind.other => const NoTextRead(),
    FileKind.text => TextRead(tidy(decodeTextFile(bytes) ?? '')),
    FileKind.docx => TextRead(tidy(_zipped(bytes, _docx) ?? '')),
    FileKind.xlsx => TextRead(tidy(_zipped(bytes, _xlsx) ?? '')),
    FileKind.odt => TextRead(tidy(_zipped(bytes, _odt) ?? '')),
    FileKind.ods => TextRead(tidy(_zipped(bytes, _ods) ?? '')),
  };

  /// The texts of a PDF's [pages], in their order, as one Attachment
  /// Text: a blank line between them, blank pages left out.
  static String joinPages(List<String> pages) => tidy(
    [
      for (final page in pages)
        if (tidy(page) case final kept when kept.isNotEmpty) kept,
    ].join('\n\n'),
  );

  /// [text] as an Attachment Text keeps it: line breaks as `\n`, no
  /// control characters but tabs and line breaks, no spaces or tabs at
  /// the ends of lines, at most one blank line in a row, no blank lines
  /// or blanks at its ends, and at most [maxTextChars] characters.
  static String tidy(String text) {
    final unified = text.replaceAll('\r\n', '\n').replaceAll('\r', '\n');
    final out = StringBuffer();
    var breaks = 0;
    var empty = true;
    for (final raw in unified.split('\n')) {
      final line = _trimEnd(
        String.fromCharCodes([
          for (final c in raw.runes)
            if (c == 0x09 || !_isControl(c)) c,
        ]),
        const [0x20, 0x09],
      );
      if (line.isEmpty) {
        breaks++;
        continue;
      }
      if (!empty) out.write(breaks > 1 ? '\n\n' : '\n');
      breaks = 1;
      empty = false;
      out.write(line);
    }
    final trimmed = _trimStart(out.toString(), const [0x20, 0x09]);
    final runes = trimmed.runes;
    if (runes.length <= maxTextChars) return trimmed;
    return _trimEnd(String.fromCharCodes(runes.take(maxTextChars)), const [
      0x20,
      0x09,
      0x0a,
    ]);
  }

  static bool _isControl(int c) => c < 0x20 || (c >= 0x7f && c <= 0x9f);

  static String _trimEnd(String text, List<int> blanks) {
    var end = text.length;
    while (end > 0 && blanks.contains(text.codeUnitAt(end - 1))) {
      end--;
    }
    return text.substring(0, end);
  }

  static String _trimStart(String text, List<int> blanks) {
    var start = 0;
    while (start < text.length && blanks.contains(text.codeUnitAt(start))) {
      start++;
    }
    return text.substring(start);
  }

  /// The text [read] makes of the zip [bytes]; null for a zip that does
  /// not open.
  static String? _zipped(Uint8List bytes, String Function(_Parts) read) {
    final Archive archive;
    try {
      archive = ZipDecoder().decodeBytes(bytes);
    } on Object {
      // Whatever the decoder chokes on: no zip, or a damaged one.
      return null;
    }
    return read(_Parts(archive));
  }

  /// The text of a Word document: each paragraph on a line, tabs and line
  /// breaks as written, deleted text and field codes left out.
  static String _docx(_Parts parts) {
    final xml = parts.text('word/document.xml');
    if (xml == null) return '';
    final out = StringBuffer();
    var inText = 0;
    for (final event in _Xml(xml).events()) {
      switch (event) {
        case _Start(:final tag):
          switch (_local(tag.name)) {
            case 't' when !tag.empty:
              inText++;
            case 'tab' when tag.empty:
              out.write('\t');
            case 'br' || 'cr' || 'p' when tag.empty:
              out.write('\n');
          }
        case _End(:final name):
          switch (_local(name)) {
            case 't':
              if (inText > 0) inText--;
            case 'p':
              out.write('\n');
          }
        case _Text(:final text):
          if (inText > 0) out.write(text);
      }
    }
    return out.toString();
  }

  /// The text of an Excel workbook: each sheet under its name, in the
  /// workbook's order, each row on a line with its cells' values between
  /// tabs, a blank line between sheets.
  static String _xlsx(_Parts parts) {
    final shared = switch (parts.text('xl/sharedStrings.xml')) {
      final xml? => _sharedStrings(xml),
      null => const <String>[],
    };
    final targets = switch (parts.text('xl/_rels/workbook.xml.rels')) {
      final xml? => _relationships(xml),
      null => const <(String, String)>[],
    };
    final workbook = parts.text('xl/workbook.xml');
    if (workbook == null) return '';
    final sheets = <(String, String)>[];
    for (final event in _Xml(workbook).events()) {
      if (event case _Start(:final tag) when _local(tag.name) == 'sheet') {
        final name = tag.attribute('name') ?? '';
        final id = tag.attribute('id') ?? '';
        for (final (i, target) in targets) {
          if (i == id) {
            sheets.add((name, _partPath(target)));
            break;
          }
        }
      }
    }
    final out = <String>[];
    for (final (name, path) in sheets.take(_maxSheets)) {
      final xml = parts.text(path);
      if (xml == null) continue;
      final rows = _sheetRows(xml, shared);
      if (rows.isNotEmpty) out.add('$name\n${rows.join('\n')}');
    }
    return out.join('\n\n');
  }

  /// The path in the zip of the workbook's relationship target [target].
  static String _partPath(String target) {
    if (target.startsWith('/')) return target.substring(1);
    var rest = target;
    while (rest.startsWith('./')) {
      rest = rest.substring(2);
    }
    return 'xl/$rest';
  }

  /// The shared strings of a workbook, in their order: each one's text
  /// runs, its phonetic guides left out.
  static List<String> _sharedStrings(String xml) {
    final out = <String>[];
    StringBuffer? current;
    var inText = 0;
    var inGuide = 0;
    for (final event in _Xml(xml).events()) {
      switch (event) {
        case _Start(:final tag):
          switch (_local(tag.name)) {
            case 'si' when tag.empty:
              out.add('');
            case 'si':
              current = StringBuffer();
            case 't' when !tag.empty:
              inText++;
            case 'rPh' when !tag.empty:
              inGuide++;
          }
        case _End(:final name):
          switch (_local(name)) {
            case 'si':
              out.add(current?.toString() ?? '');
              current = null;
            case 't':
              if (inText > 0) inText--;
            case 'rPh':
              if (inGuide > 0) inGuide--;
          }
        case _Text(:final text):
          if (inText > 0 && inGuide == 0) current?.write(text);
      }
    }
    return out;
  }

  /// The relationships of a workbook: each id with its target.
  static List<(String, String)> _relationships(String xml) => [
    for (final event in _Xml(xml).events())
      if (event case _Start(:final tag) when _local(tag.name) == 'Relationship')
        if ((tag.attribute('Id'), tag.attribute('Target')) case (
          final String id,
          final String target,
        ))
          (id, target),
  ];

  /// The rows of a worksheet with a value, each its cells' values between
  /// tabs, the empty ones at the end left out.
  static List<String> _sheetRows(String xml, List<String> shared) {
    final rows = <String>[];
    final row = <String>[];
    (String, StringBuffer)? cell;
    var inValue = 0;
    var inText = 0;
    final inline = StringBuffer();
    for (final event in _Xml(xml).events()) {
      switch (event) {
        case _Start(:final tag):
          switch (_local(tag.name)) {
            case 'row' when tag.empty:
              break;
            case 'row':
              row.clear();
            case 'c' when tag.empty:
              row.add('');
            case 'c':
              cell = (tag.attribute('t') ?? '', StringBuffer());
              inline.clear();
            case 'v' when !tag.empty:
              inValue++;
            case 't' when !tag.empty:
              inText++;
          }
        case _End(:final name):
          switch (_local(name)) {
            case 'row':
              while (row.isNotEmpty && row.last.isEmpty) {
                row.removeLast();
              }
              if (row.isNotEmpty) rows.add(row.join('\t'));
              row.clear();
            case 'c':
              final (kind, value) = cell ?? ('', StringBuffer());
              row.add(
                _cellValue(kind, value.toString(), inline.toString(), shared),
              );
              cell = null;
            case 'v':
              if (inValue > 0) inValue--;
            case 't':
              if (inText > 0) inText--;
          }
        case _Text(:final text):
          if (cell != null) {
            if (inValue > 0) {
              cell.$2.write(text);
            } else if (inText > 0) {
              inline.write(text);
            }
          }
      }
    }
    return rows;
  }

  /// What a cell of the type [kind] with [value] and its inline text
  /// [inline] shows: a shared string by its index, a truth value as TRUE
  /// or FALSE, anything else as written. Tabs and line breaks in it
  /// become spaces, so it stays in its row.
  static String _cellValue(
    String kind,
    String value,
    String inline,
    List<String> shared,
  ) {
    final shown = switch (kind) {
      's' => switch (_whole(value)) {
        final i? when i < shared.length => shared[i],
        _ => '',
      },
      'inlineStr' => inline,
      'b' => switch (value.trim()) {
        '1' => 'TRUE',
        '0' => 'FALSE',
        final other => other,
      },
      _ => value,
    };
    return _oneLine(shown);
  }

  /// [text] with its tabs and line breaks as spaces, its ends trimmed.
  static String _oneLine(String text) {
    final spaced = text.replaceAll(RegExp('[\t\n\r]'), ' ');
    var start = 0;
    var end = spaced.length;
    while (start < end && spaced.codeUnitAt(start) == 0x20) {
      start++;
    }
    while (end > start && spaced.codeUnitAt(end - 1) == 0x20) {
      end--;
    }
    return spaced.substring(start, end);
  }

  /// The text of an OpenDocument text: each paragraph and heading on a
  /// line, its spaces, tabs and line breaks as written, deleted text left
  /// out.
  static String _odt(_Parts parts) {
    final xml = parts.text('content.xml');
    if (xml == null) return '';
    final out = StringBuffer();
    var inBody = 0;
    var inParagraph = 0;
    var inDeleted = 0;
    for (final event in _Xml(xml).events()) {
      switch (event) {
        case _Start(:final tag):
          final name = _local(tag.name);
          if (tag.empty) {
            if (inBody > 0 && inParagraph > 0 && inDeleted == 0) {
              _inlineMark(tag, out);
            }
            if ((name == 'p' || name == 'h') && inBody > 0 && inDeleted == 0) {
              out.write('\n');
            }
            continue;
          }
          switch (name) {
            case 'body':
              inBody++;
            case 'p' || 'h':
              inParagraph++;
            case 'tracked-changes':
              inDeleted++;
          }
        case _End(:final name):
          switch (_local(name)) {
            case 'body':
              if (inBody > 0) inBody--;
            case 'p' || 'h':
              if (inParagraph > 0) inParagraph--;
              if (inBody > 0 && inDeleted == 0) out.write('\n');
            case 'tracked-changes':
              if (inDeleted > 0) inDeleted--;
          }
        case _Text(:final text):
          if (inBody > 0 && inParagraph > 0 && inDeleted == 0) {
            out.write(text);
          }
      }
    }
    return out.toString();
  }

  /// Writes what the empty OpenDocument element [tag] inside a paragraph
  /// stands for: spaces, a tab or a line break.
  static void _inlineMark(_Tag tag, StringBuffer out) {
    switch (_local(tag.name)) {
      case 's':
        final count = switch (tag.attribute('c')) {
          final c? => _whole(c) ?? 1,
          null => 1,
        };
        out.write(' ' * (count > _maxRepeat ? _maxRepeat : count));
      case 'tab':
        out.write('\t');
      case 'line-break':
        out.write('\n');
    }
  }

  /// The text of an OpenDocument spreadsheet: each sheet under its name,
  /// each row with a value on a line with its cells' texts between tabs, a
  /// blank line between sheets.
  static String _ods(_Parts parts) {
    final xml = parts.text('content.xml');
    if (xml == null) return '';
    final sheets = <String>[];
    (String, List<String>)? sheet;
    (int, List<String>)? row;
    (int, StringBuffer)? cell;
    var inParagraph = 0;
    var paragraphs = 0;
    void endCell() {
      final (held, into) = (cell, row);
      cell = null;
      if ((held, into) case ((final repeat, final text)?, (_, final cells)?)) {
        final kept = _oneLine(text.toString());
        for (var i = 0; i < repeat; i++) {
          cells.add(kept);
        }
      }
    }

    void endRow() {
      final (held, into) = (row, sheet);
      row = null;
      if (held == null) return;
      final (repeat, cells) = held;
      while (cells.isNotEmpty && cells.last.isEmpty) {
        cells.removeLast();
      }
      if (into case (_, final rows)? when cells.isNotEmpty) {
        final line = cells.join('\t');
        for (var i = 0; i < repeat; i++) {
          rows.add(line);
        }
      }
    }

    for (final event in _Xml(xml).events()) {
      switch (event) {
        case _Start(:final tag):
          final name = _local(tag.name);
          switch (name) {
            case 'table' when !tag.empty:
              sheet = (tag.attribute('name') ?? '', <String>[]);
            case 'table-row':
              row = (_repeated(tag, 'number-rows-repeated'), <String>[]);
              if (tag.empty) endRow();
            case 'table-cell' || 'covered-table-cell':
              cell = (
                _repeated(tag, 'number-columns-repeated'),
                StringBuffer(),
              );
              paragraphs = 0;
              if (tag.empty) endCell();
            case 'p' || 'h' when cell != null:
              if (paragraphs > 0) cell?.$2.write(' ');
              paragraphs++;
              if (!tag.empty) inParagraph++;
            case _ when inParagraph > 0 && tag.empty:
              if (cell case (_, final text)?) _inlineMark(tag, text);
          }
        case _End(:final name):
          switch (_local(name)) {
            case 'table':
              if (sheet case (final name, final rows)? when rows.isNotEmpty) {
                sheets.add('$name\n${rows.join('\n')}');
              }
              sheet = null;
            case 'table-row':
              endRow();
            case 'table-cell' || 'covered-table-cell':
              endCell();
            case 'p' || 'h':
              if (inParagraph > 0) inParagraph--;
          }
        case _Text(:final text):
          if (inParagraph > 0) cell?.$2.write(text);
      }
    }
    return sheets.join('\n\n');
  }

  /// How often [tag] is repeated by its attribute [attribute], at least
  /// once and at most [_maxRepeat] times.
  static int _repeated(_Tag tag, String attribute) {
    final n = switch (tag.attribute(attribute)) {
      final a? => _whole(a) ?? 1,
      null => 1,
    };
    return n < 1 ? 1 : (n > _maxRepeat ? _maxRepeat : n);
  }

  /// The whole number [text] writes in decimal digits alone, blanks around
  /// them trimmed, at most nine of them.
  static int? _whole(String text) {
    final digits = text.trim();
    if (digits.isEmpty ||
        digits.length > 9 ||
        digits.codeUnits.any((c) => c < 0x30 || c > 0x39)) {
      return null;
    }
    return int.parse(digits);
  }

  /// The name of an element or attribute without its namespace prefix.
  static String _local(String name) {
    final colon = name.lastIndexOf(':');
    return colon < 0 ? name : name.substring(colon + 1);
  }
}

/// The parts of a zip, read by name.
class _Parts {
  final Archive archive;

  _Parts(this.archive);

  /// The part [name] as UTF-8 text; null when it is not there, too large
  /// or not UTF-8.
  String? text(String name) {
    final part = archive.findFile(name);
    if (part == null || part.size > FileTexts._maxPartBytes) return null;
    try {
      final bytes = part.readBytes();
      if (bytes == null || bytes.length > FileTexts._maxPartBytes) {
        return null;
      }
      return _utf8(bytes);
    } on Object {
      // A damaged part reads as absent.
      return null;
    }
  }

  /// [bytes] as strict UTF-8, one byte order mark before them dropped
  /// and any other kept, as the desktop core reads them.
  static String? _utf8(Uint8List bytes) {
    bool bom(int at) =>
        bytes.length >= at + 3 &&
        bytes[at] == 0xef &&
        bytes[at + 1] == 0xbb &&
        bytes[at + 2] == 0xbf;
    final start = bom(0) ? 3 : 0;
    try {
      // Drops a second byte order mark, which the desktop keeps.
      final text = utf8.decode(Uint8List.sublistView(bytes, start));
      return bom(start) ? '﻿$text' : text;
    } on FormatException {
      return null;
    }
  }
}

/// A start tag of an XML element.
class _Tag {
  final String name;

  /// The raw attributes, between the name and the end of the tag.
  final String attributes;

  /// Whether it closes itself (`<a/>`); no end follows.
  final bool empty;

  _Tag(this.name, this.attributes, this.empty);

  /// The value of the attribute [wanted], by its name without its prefix,
  /// its references resolved.
  String? attribute(String wanted) {
    var rest = attributes;
    while (true) {
      rest = _trimStartXml(rest);
      final equals = rest.indexOf('=');
      if (equals < 0) return null;
      final name = _trimEndXml(rest.substring(0, equals));
      rest = _trimStartXml(rest.substring(equals + 1));
      if (rest.isEmpty) return null;
      final quote = rest[0];
      if (quote != '"' && quote != "'") return null;
      final valueEnd = rest.indexOf(quote, 1);
      if (valueEnd < 0) return null;
      final value = rest.substring(1, valueEnd);
      if (FileTexts._local(name) == wanted) return _unescape(value);
      rest = rest.substring(valueEnd + 1);
    }
  }
}

sealed class _Event {}

class _Start extends _Event {
  final _Tag tag;
  _Start(this.tag);
}

class _End extends _Event {
  final String name;
  _End(this.name);
}

class _Text extends _Event {
  final String text;
  _Text(this.text);
}

bool _isXmlSpace(int c) => c == 0x20 || c == 0x09 || c == 0x0a || c == 0x0d;

String _trimStartXml(String text) {
  var start = 0;
  while (start < text.length && _isXmlSpace(text.codeUnitAt(start))) {
    start++;
  }
  return text.substring(start);
}

String _trimEndXml(String text) {
  var end = text.length;
  while (end > 0 && _isXmlSpace(text.codeUnitAt(end - 1))) {
    end--;
  }
  return text.substring(0, end);
}

/// A small reader of XML, the same in both cores: start tags, end tags
/// and text with its references resolved; comments, processing
/// instructions and declarations skipped, CDATA as text. It stops at the
/// first tag that does not end.
class _Xml {
  final String xml;

  _Xml(this.xml);

  Iterable<_Event> events() sync* {
    var at = 0;
    while (at < xml.length) {
      if (xml.codeUnitAt(at) != 0x3c) {
        final end = xml.indexOf('<', at);
        final stop = end < 0 ? xml.length : end;
        yield _Text(_unescape(xml.substring(at, stop)));
        at = stop;
        continue;
      }
      if (xml.startsWith('<!--', at)) {
        final end = xml.indexOf('-->', at + 4);
        if (end < 0) return;
        at = end + 3;
        continue;
      }
      if (xml.startsWith('<![CDATA[', at)) {
        final end = xml.indexOf(']]>', at + 9);
        if (end < 0) return;
        yield _Text(xml.substring(at + 9, end));
        at = end + 3;
        continue;
      }
      if (xml.startsWith('<?', at)) {
        final end = xml.indexOf('?>', at + 2);
        if (end < 0) return;
        at = end + 2;
        continue;
      }
      if (xml.startsWith('<!', at)) {
        final end = xml.indexOf('>', at + 2);
        if (end < 0) return;
        at = end + 1;
        continue;
      }
      final end = _tagEnd(xml, at);
      if (end < 0) return;
      var inner = xml.substring(at + 1, end);
      at = end + 1;
      if (inner.startsWith('/')) {
        yield _End(_trimEndXml(_trimStartXml(inner.substring(1))));
        continue;
      }
      final empty = inner.endsWith('/');
      if (empty) inner = inner.substring(0, inner.length - 1);
      var nameEnd = 0;
      while (nameEnd < inner.length &&
          !_isXmlSpace(inner.codeUnitAt(nameEnd))) {
        nameEnd++;
      }
      yield _Start(
        _Tag(inner.substring(0, nameEnd), inner.substring(nameEnd), empty),
      );
    }
  }

  /// Where the tag at [at] ends: its `>` outside quotes; -1 when it does
  /// not end.
  static int _tagEnd(String xml, int at) {
    int? quote;
    for (var i = at; i < xml.length; i++) {
      final c = xml.codeUnitAt(i);
      if (quote != null) {
        if (c == quote) quote = null;
      } else if (c == 0x22 || c == 0x27) {
        quote = c;
      } else if (c == 0x3e) {
        return i;
      }
    }
    return -1;
  }
}

/// [text] with its character and entity references resolved; one that is
/// not known, or names no character, stays as written.
String _unescape(String text) {
  if (!text.contains('&')) return text;
  final out = StringBuffer();
  var rest = text;
  while (true) {
    final at = rest.indexOf('&');
    if (at < 0) break;
    out.write(rest.substring(0, at));
    rest = rest.substring(at);
    final end = rest.indexOf(';');
    final code = end < 0 ? null : _reference(rest.substring(1, end));
    if (code != null) {
      out.writeCharCode(code);
      rest = rest.substring(end + 1);
    } else {
      out.write('&');
      rest = rest.substring(1);
    }
  }
  out.write(rest);
  return out.toString();
}

/// The code point the reference [name] (between `&` and `;`) stands for.
int? _reference(String name) {
  switch (name) {
    case 'lt':
      return 0x3c;
    case 'gt':
      return 0x3e;
    case 'amp':
      return 0x26;
    case 'quot':
      return 0x22;
    case 'apos':
      return 0x27;
  }
  if (!name.startsWith('#')) return null;
  var digits = name.substring(1);
  var radix = 10;
  if (digits.startsWith('x') || digits.startsWith('X')) {
    digits = digits.substring(1);
    radix = 16;
  }
  bool isDigit(int c) =>
      (c >= 0x30 && c <= 0x39) ||
      (radix == 16 && ((c >= 0x41 && c <= 0x46) || (c >= 0x61 && c <= 0x66)));
  if (digits.isEmpty || digits.length > 8 || !digits.codeUnits.every(isDigit)) {
    return null;
  }
  final code = int.parse(digits, radix: radix);
  if (code == 0 || code > 0x10ffff || (code >= 0xd800 && code <= 0xdfff)) {
    return null;
  }
  return code;
}
