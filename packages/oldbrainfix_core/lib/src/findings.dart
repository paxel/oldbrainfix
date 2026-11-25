/// Where a search finds what it finds (obf-1047): the folding both
/// search and the find bar match by, and the places a query is found in
/// a Note's texts as they show, the same in both cores (`findings.rs`).
///
/// A text and a query are matched folded ([Findings.fold]): letter by
/// letter in lower case, by a table both cores carry rather than by
/// their platforms' Unicode, which differ and change with an update, a
/// letter with accents or an umlaut as its letter without them, "ß" as "ss", "æ" as "ae", "œ" as
/// "oe", and the combining accents (U+0300 to U+036F) dropped. So
/// "muller" finds "Müller" and "strasse" "Straße"; "mueller" does not
/// find "Müller", and nothing allows for a typo.
///
/// A formatted text is matched as it reads without its marks, as search
/// reads it ([TextFormats.visible]), line by line: "Müller" written
/// `**Mül**ler` is found, and a place covers the marks inside it.
library;

import 'dart:math' as math;

import 'markdown.dart';
import 'notes.dart';
import 'text_format.dart';

/// Which of the texts of a Note or of one of its Sections a [Finding]
/// lies in.
enum FindingPart {
  /// The text of the Note or the Section.
  text,

  /// The Transcript of a voice recording.
  transcript,

  /// The Photo Text of a photo.
  photoText,

  /// The name of an attached file.
  fileName,

  /// The Attachment Text of a file.
  attachmentText,
}

/// The cell of a table a [Finding] begins in: the [table] it is among
/// the tables of its text, the top one 0, which begins at [tableStart]
/// of the text in UTF-16 code units; its [row], the header row 0, as
/// [MarkdownBlock.row] counts them; and its [column], the first 0.
class FindingCell {
  final int table;
  final int tableStart;
  final int row;
  final int column;

  const FindingCell({
    required this.table,
    required this.tableStart,
    required this.row,
    required this.column,
  });
}

/// A place a query is found in a text as it shows: in its [part], of the
/// recording, photo or file whose hash is [key] (empty for a text), of
/// the Section [section] or, null, of the Note itself. It lies from
/// [start] to [end] of that text as it shows (a Note's or a Section's
/// [Note.shownText], a Transcript, Photo Text, Attachment Text or file
/// name as it is), marks inside it included, in UTF-16 code units, on
/// its [line] counted from 0 as [Markdown.lineAt] counts them; in a
/// table's [cell], null elsewhere. Its [context] is that line as it
/// reads, or a cut of it around the place marked with "…", the place
/// lying in it from [contextStart] to [contextEnd].
class Finding {
  final FindingPart part;
  final String key;
  final String? section;
  final int start;
  final int end;
  final int line;
  final FindingCell? cell;
  final String context;
  final int contextStart;
  final int contextEnd;

  const Finding({
    required this.part,
    required this.key,
    required this.section,
    required this.start,
    required this.end,
    required this.line,
    required this.cell,
    required this.context,
    required this.contextStart,
    required this.contextEnd,
  });
}

/// Folding for search and the places of a query in a Note's texts.
abstract final class Findings {
  /// How many characters a [Finding.context] holds at most, its "…"
  /// not counted, unless the place is longer.
  static const contextLength = 80;

  /// How many characters a cut [Finding.context] shows before the place
  /// at most.
  static const contextBefore = 20;

  /// [text] folded for matching: each character in lower case, the first
  /// of its lower case by [_lowerRuns], then a letter with accents
  /// or an umlaut as its letter without them, "ß" as "ss", "æ" as "ae",
  /// "œ" as "oe", and the combining accents dropped.
  static String fold(String text) {
    var ascii = true;
    for (var i = 0; i < text.length; i++) {
      if (text.codeUnitAt(i) >= 0x80) {
        ascii = false;
        break;
      }
    }
    if (ascii) return text.toLowerCase();
    final out = StringBuffer();
    for (final rune in text.runes) {
      if (rune < 0x80) {
        out.writeCharCode(rune >= 0x41 && rune <= 0x5A ? rune + 0x20 : rune);
      } else {
        out.write(_folded(rune));
      }
    }
    return out.toString();
  }

  /// The places [query], trimmed, is found in [shown], a text as it
  /// shows in [format], top first; none for a query that is blank or
  /// folds to nothing. Each is of [part], [key] and [section], as the
  /// caller says which text [shown] is.
  static List<Finding> inText(
    String shown,
    String format,
    String query, {
    FindingPart part = FindingPart.text,
    String key = '',
    String? section,
  }) {
    final wanted = fold(query.trim());
    if (wanted.isEmpty) return [];
    final lines = format == TextFormats.markdown
        ? _formattedLines(shown)
        : _plainLines(shown);
    return [
      for (final line in lines)
        for (final (first, last) in _matches(line, wanted))
          _finding(line, first, last, part, key, section),
    ];
  }

  /// The places [query] is found in [note]: in its text, its Transcripts
  /// (its recordings' order, those removed after them), its Photo Texts
  /// (likewise), its files' names and their Attachment Texts (the files'
  /// order, those removed after them), then the same of each of
  /// [sections], in their order, as [inText] finds them. A text left
  /// over in no order goes last, by its hash.
  static List<Finding> ofNote(
    Note note,
    String query, {
    List<Section> sections = const [],
  }) => [
    ...inText(note.shownText, note.textFormat, query),
    ..._ofMedia(
      query,
      null,
      recordings: [...note.recordings, ...note.removedRecordings],
      transcripts: note.transcripts,
      photos: [...note.photos, ...note.removedPhotos],
      photoTexts: note.photoTexts,
      attachments: note.attachments,
      removedAttachments: note.removedAttachments,
      attachmentTexts: note.attachmentTexts,
    ),
    for (final s in sections) ...[
      ...inText(s.shownText, s.textFormat, query, section: s.id),
      ..._ofMedia(
        query,
        s.id,
        recordings: [...s.recordings, ...s.removedRecordings],
        transcripts: s.transcripts,
        photos: [...s.photos, ...s.removedPhotos],
        photoTexts: s.photoTexts,
        attachments: s.attachments,
        removedAttachments: s.removedAttachments,
        attachmentTexts: s.attachmentTexts,
      ),
    ],
  ];

  static List<Finding> _ofMedia(
    String query,
    String? section, {
    required List<String> recordings,
    required Map<String, String> transcripts,
    required List<String> photos,
    required Map<String, String> photoTexts,
    required List<Attachment> attachments,
    required List<Attachment> removedAttachments,
    required Map<String, String> attachmentTexts,
  }) {
    List<Finding> find(FindingPart part, String key, String text) => inText(
      text,
      TextFormats.plain,
      query,
      part: part,
      key: key,
      section: section,
    );
    final files = [
      for (final file in [...attachments, ...removedAttachments]) file.hash,
    ];
    return [
      for (final hash in _ordered(recordings, transcripts))
        ...find(FindingPart.transcript, hash, transcripts[hash] ?? ''),
      for (final hash in _ordered(photos, photoTexts))
        ...find(FindingPart.photoText, hash, photoTexts[hash] ?? ''),
      for (final file in attachments)
        ...find(FindingPart.fileName, file.hash, file.name),
      for (final hash in _ordered(files, attachmentTexts))
        ...find(FindingPart.attachmentText, hash, attachmentTexts[hash] ?? ''),
    ];
  }

  /// The keys of [texts] in the order of [order], then those it lacks by
  /// their order as text.
  static List<String> _ordered(List<String> order, Map<String, String> texts) {
    final seen = <String>{};
    return [
      for (final key in order)
        if (texts.containsKey(key) && seen.add(key)) key,
      ...[
        for (final key in texts.keys)
          if (!seen.contains(key)) key,
      ]..sort(),
    ];
  }

  /// The runs of [line] that [wanted], folded and not empty, is found
  /// in, as the first and the last of its characters, left to right and
  /// none inside another: a match on half of a character's folding takes
  /// the whole character, the accents dropped right after its end are
  /// its own, and two that would take the same character are one.
  static List<(int, int)> _matches(_Line line, String wanted) {
    final folded = StringBuffer();
    final owner = <int>[];
    final empty = <bool>[];
    for (final (i, rune) in line.runes.indexed) {
      final f = _folded(rune);
      folded.write(f);
      for (var k = 0; k < f.length; k++) {
        owner.add(i);
      }
      empty.add(f.isEmpty);
    }
    final text = folded.toString();
    final found = <(int, int)>[];
    var at = 0;
    while (true) {
      final m = text.indexOf(wanted, at);
      if (m < 0) break;
      at = m + wanted.length;
      final first = owner[m];
      var last = owner[at - 1];
      while (last + 1 < empty.length && empty[last + 1]) {
        last++;
      }
      if (found.isNotEmpty && first <= found.last.$2) {
        found.last = (found.last.$1, math.max(found.last.$2, last));
      } else {
        found.add((first, last));
      }
    }
    return found;
  }

  static Finding _finding(
    _Line line,
    int first,
    int last,
    FindingPart part,
    String key,
    String? section,
  ) {
    final runes = line.runes;
    final n = runes.length;
    final a = first;
    final b = last + 1;
    var lo = 0;
    var hi = n;
    while (lo < a && _blank(runes[lo])) {
      lo++;
    }
    while (hi > b && _blank(runes[hi - 1])) {
      hi--;
    }
    var from = lo;
    var to = hi;
    if (hi - lo > contextLength) {
      from = math.max(lo, a - contextBefore);
      to = math.min(hi, math.max(b, from + contextLength));
      if (to - from < contextLength) from = math.max(lo, to - contextLength);
      // Whole words at a cut.
      if (from > lo && !_blank(runes[from - 1])) {
        for (var i = from; i < a; i++) {
          if (_blank(runes[i])) {
            from = i + 1;
            break;
          }
        }
      }
      if (to < hi && !_blank(runes[to])) {
        for (var i = to - 1; i >= b; i--) {
          if (_blank(runes[i])) {
            to = i;
            break;
          }
        }
      }
      while (from < a && _blank(runes[from])) {
        from++;
      }
      while (to > b && _blank(runes[to - 1])) {
        to--;
      }
    }
    final before = String.fromCharCodes(runes.sublist(from, a));
    final hit = String.fromCharCodes(runes.sublist(a, b));
    final after = String.fromCharCodes(runes.sublist(b, to));
    final open = from > lo ? '…' : '';
    final close = to < hi ? '…' : '';
    final table = line.table;
    final start = line.from[a];
    return Finding(
      part: part,
      key: key,
      section: section,
      start: start,
      end: math.max(start, line.to[last]),
      line: line.line,
      cell: table == null
          ? null
          : FindingCell(
              table: table,
              tableStart: line.tableStart,
              row: line.row,
              column: line.columns[a],
            ),
      context: '$open$before$hit$after$close',
      contextStart: open.length + before.length,
      contextEnd: open.length + before.length + hit.length,
    );
  }

  static bool _blank(int rune) => rune == 0x20 || rune == 0x09;

  /// The lines of a text that is not formatted, each character as it is.
  static List<_Line> _plainLines(String text) {
    final lines = <_Line>[];
    var line = _Line(0);
    var i = 0;
    while (i < text.length) {
      final c = text.codeUnitAt(i);
      if (c == 0x0A || c == 0x0D) {
        lines.add(line);
        line = _Line(lines.length);
        i += c == 0x0D && i + 1 < text.length && text.codeUnitAt(i + 1) == 0x0A
            ? 2
            : 1;
        continue;
      }
      final rune = _runeAt(text, i);
      final w = rune > 0xFFFF ? 2 : 1;
      line.add(rune, i, i + w, 0);
      i += w;
    }
    lines.add(line);
    return lines;
  }

  /// The lines of a formatted text as they read without their marks,
  /// [MarkdownBlock.visible], each character with where it is written:
  /// a table's row its cells that show something, a space between them,
  /// that space lying between the two cells; no line for a table's
  /// delimiter row, the line marking its first column as headings, or a
  /// code block's fence.
  static List<_Line> _formattedLines(String text) {
    final lines = <_Line>[];
    final tables = <int, int>{};
    for (final block in Markdown.parse(text)) {
      final table = block.table;
      if (table != null) {
        final row = block.row;
        if (row == null) continue;
        final line = _Line(
          block.line,
          table: tables.putIfAbsent(table.start, () => tables.length),
          tableStart: table.start,
          row: row,
        );
        int? after;
        for (final (column, cell) in table.rows[row].cells.indexed) {
          if (cell.visible.isEmpty) continue;
          if (after != null) line.add(0x20, after, cell.start, column);
          for (final span in cell.spans) {
            _span(text, span, line, column);
          }
          after = cell.end;
        }
        lines.add(line);
        continue;
      }
      if (block.code?.isFence(block.line) ?? false) continue;
      final line = _Line(block.line);
      for (final span in block.spans) {
        _span(text, span, line, 0);
      }
      lines.add(line);
    }
    return lines;
  }

  /// The characters [span] shows, each with where it is written.
  static void _span(String text, MarkdownSpan span, _Line line, int column) {
    switch (span.kind) {
      case MarkdownSpanKind.text when span.contentStart == span.start:
        var at = span.start;
        for (final rune in span.text.runes) {
          final w = rune > 0xFFFF ? 2 : 1;
          line.add(rune, at, at + w, column);
          at += w;
        }
      case MarkdownSpanKind.text:
        // One escaped character: the backslash is its own.
        for (final rune in span.text.runes) {
          line.add(rune, span.start, span.end, column);
        }
      case MarkdownSpanKind.code:
        // Its characters as written, but for a space dropped at both ends
        // and a `\|` in a cell showing `|`.
        var at = span.contentStart;
        for (final rune in span.text.runes) {
          final w = rune > 0xFFFF ? 2 : 1;
          var q = at;
          var escaped = false;
          while (q < span.contentEnd) {
            if (_runeAt(text, q) == rune) break;
            if (text.codeUnitAt(q) == 0x5C &&
                q + 1 < span.contentEnd &&
                _runeAt(text, q + 1) == rune) {
              escaped = true;
              break;
            }
            q++;
          }
          if (q >= span.contentEnd) {
            line.add(rune, span.start, span.end, column);
            continue;
          }
          final end = q + w + (escaped ? 1 : 0);
          line.add(rune, q, end, column);
          at = end;
        }
      case _:
        for (final child in span.children) {
          _span(text, child, line, column);
        }
    }
  }

  static int _runeAt(String text, int i) {
    final c = text.codeUnitAt(i);
    if (c >= 0xD800 && c < 0xDC00 && i + 1 < text.length) {
      final d = text.codeUnitAt(i + 1);
      if (d >= 0xDC00 && d < 0xE000) {
        return 0x10000 + ((c - 0xD800) << 10) + (d - 0xDC00);
      }
    }
    return c;
  }

  /// What [rune] folds to, kept once worked out.
  static String _folded(int rune) {
    if (rune < 0x80) {
      return rune >= 0x41 && rune <= 0x5A
          ? String.fromCharCode(rune + 0x20)
          : String.fromCharCode(rune);
    }
    return _foldedOnce[rune] ??= _foldOf(rune);
  }

  static final _foldedOnce = <int, String>{};

  static String _foldOf(int rune) {
    final lower = _lower[rune] ?? rune;
    if (lower >= 0x300 && lower <= 0x36F) return '';
    if (_wider[lower] case final wide?) return wide;
    return String.fromCharCode(_bases[lower] ?? lower);
  }

  /// The letters that fold to two.
  static const _wider = {
    0xDF: 'ss', // ß
    0x1E9E: 'ss', // ẞ
    0xE6: 'ae', // æ
    0xC6: 'ae', // Æ
    0x153: 'oe', // œ
    0x152: 'oe', // Œ
  };

  /// The lower case of each character that has one, by [_lowerRuns].
  static final Map<int, int> _lower = {
    for (final (start, end, step, delta) in _lowerRuns)
      for (var c = start; c <= end; c += step) c: c + delta,
  };

  /// The letter each letter with accents folds to, by [_foldRuns].
  static final Map<int, int> _bases = {
    for (final (start, letters) in _foldRuns)
      for (final (i, letter) in letters.runes.indexed)
        if (letter != 0x2E) start + i: letter,
  };
}

/// The letters with accents and umlauts and the letters they fold to, the
/// same in both cores (`findings.rs`): Latin and Greek letters whose
/// canonical decomposition is a letter and combining accents, and the
/// letters with a stroke ø, đ, ħ, ł and ŧ and the dotless ı, each to a
/// letter in lower case, upper and lower case alike. Each run gives the
/// letter of each character from its start on, `.` for one folding to
/// itself.
const _foldRuns = [
  (
    0xC0,
    'aaaaaa.ceeeeiiii.nooooo.ouuuuy..aaaaaa.ceeeeiiii.nooooo.ouuu'
        'uy.yaaaaaaccccccccddddeeeeeeeeeegggggggghhhhiiiiiiiiii..jjkk'
        '.llllll..llnnnnnn...oooooo..rrrrrrssssssssttttttuuuuuuuuuuuu'
        'wwyyyzzzzzz.................................oo.............u'
        'u............................aaiioouuuuuuuuuu.aaaa....ggkkoo'
        'oo..j...gg..nnaa....aaaaeeeeiiiioooorrrruuuusstt..hh......aa'
        'eeooooooooyy',
  ),
  (
    0x370,
    '......................α.εηι.ο.υωι.........................ιυ'
        'αεηιυ.........................ιυουω',
  ),
  (
    0x1E00,
    'aabbbbbbccddddddddddeeeeeeeeeeffgghhhhhhhhhhiiiikkkkkkllllll'
        'llmmmmmmnnnnnnnnoooooooopppprrrrrrrrssssssssssttttttttuuuuuu'
        'uuuuvvvvwwwwwwwwwwxxxxyyzzzzzzhtwy......aaaaaaaaaaaaaaaaaaaa'
        'aaaaeeeeeeeeeeeeeeeeiiiioooooooooooooooooooooooouuuuuuuuuuuu'
        'uuyyyyyyyy',
  ),
];

/// The lower case of every character that has one, the first character
/// of it where it is more than one ("İ" to "i"), as Unicode 18 has it
/// (Rust 1.100), the same in both cores (`findings.rs`): runs from a
/// character to another, every one or every other, each lowered by the
/// same distance.
const _lowerRuns = [
  (0xC0, 0xD6, 1, 32),
  (0xD8, 0xDE, 1, 32),
  (0x100, 0x12E, 2, 1),
  (0x130, 0x130, 1, -199),
  (0x132, 0x136, 2, 1),
  (0x139, 0x147, 2, 1),
  (0x14A, 0x176, 2, 1),
  (0x178, 0x178, 1, -121),
  (0x179, 0x17D, 2, 1),
  (0x181, 0x181, 1, 210),
  (0x182, 0x184, 2, 1),
  (0x186, 0x186, 1, 206),
  (0x187, 0x187, 1, 1),
  (0x189, 0x18A, 1, 205),
  (0x18B, 0x18B, 1, 1),
  (0x18E, 0x18E, 1, 79),
  (0x18F, 0x18F, 1, 202),
  (0x190, 0x190, 1, 203),
  (0x191, 0x191, 1, 1),
  (0x193, 0x193, 1, 205),
  (0x194, 0x194, 1, 207),
  (0x196, 0x196, 1, 211),
  (0x197, 0x197, 1, 209),
  (0x198, 0x198, 1, 1),
  (0x19C, 0x19C, 1, 211),
  (0x19D, 0x19D, 1, 213),
  (0x19F, 0x19F, 1, 214),
  (0x1A0, 0x1A4, 2, 1),
  (0x1A6, 0x1A6, 1, 218),
  (0x1A7, 0x1A7, 1, 1),
  (0x1A9, 0x1A9, 1, 218),
  (0x1AC, 0x1AC, 1, 1),
  (0x1AE, 0x1AE, 1, 218),
  (0x1AF, 0x1AF, 1, 1),
  (0x1B1, 0x1B2, 1, 217),
  (0x1B3, 0x1B5, 2, 1),
  (0x1B7, 0x1B7, 1, 219),
  (0x1B8, 0x1B8, 1, 1),
  (0x1BC, 0x1BC, 1, 1),
  (0x1C4, 0x1C4, 1, 2),
  (0x1C5, 0x1C5, 1, 1),
  (0x1C7, 0x1C7, 1, 2),
  (0x1C8, 0x1C8, 1, 1),
  (0x1CA, 0x1CA, 1, 2),
  (0x1CB, 0x1DB, 2, 1),
  (0x1DE, 0x1EE, 2, 1),
  (0x1F1, 0x1F1, 1, 2),
  (0x1F2, 0x1F4, 2, 1),
  (0x1F6, 0x1F6, 1, -97),
  (0x1F7, 0x1F7, 1, -56),
  (0x1F8, 0x21E, 2, 1),
  (0x220, 0x220, 1, -130),
  (0x222, 0x232, 2, 1),
  (0x23A, 0x23A, 1, 10795),
  (0x23B, 0x23B, 1, 1),
  (0x23D, 0x23D, 1, -163),
  (0x23E, 0x23E, 1, 10792),
  (0x241, 0x241, 1, 1),
  (0x243, 0x243, 1, -195),
  (0x244, 0x244, 1, 69),
  (0x245, 0x245, 1, 71),
  (0x246, 0x24E, 2, 1),
  (0x370, 0x372, 2, 1),
  (0x376, 0x376, 1, 1),
  (0x37F, 0x37F, 1, 116),
  (0x386, 0x386, 1, 38),
  (0x388, 0x38A, 1, 37),
  (0x38C, 0x38C, 1, 64),
  (0x38E, 0x38F, 1, 63),
  (0x391, 0x3A1, 1, 32),
  (0x3A3, 0x3AB, 1, 32),
  (0x3CF, 0x3CF, 1, 8),
  (0x3D8, 0x3EE, 2, 1),
  (0x3F4, 0x3F4, 1, -60),
  (0x3F7, 0x3F7, 1, 1),
  (0x3F9, 0x3F9, 1, -7),
  (0x3FA, 0x3FA, 1, 1),
  (0x3FD, 0x3FF, 1, -130),
  (0x400, 0x40F, 1, 80),
  (0x410, 0x42F, 1, 32),
  (0x460, 0x480, 2, 1),
  (0x48A, 0x4BE, 2, 1),
  (0x4C0, 0x4C0, 1, 15),
  (0x4C1, 0x4CD, 2, 1),
  (0x4D0, 0x52E, 2, 1),
  (0x531, 0x556, 1, 48),
  (0x10A0, 0x10C5, 1, 7264),
  (0x10C7, 0x10C7, 1, 7264),
  (0x10CD, 0x10CD, 1, 7264),
  (0x13A0, 0x13EF, 1, 38864),
  (0x13F0, 0x13F5, 1, 8),
  (0x1C89, 0x1C89, 1, 1),
  (0x1C90, 0x1CBA, 1, -3008),
  (0x1CBD, 0x1CBF, 1, -3008),
  (0x1E00, 0x1E94, 2, 1),
  (0x1E9E, 0x1E9E, 1, -7615),
  (0x1EA0, 0x1EFE, 2, 1),
  (0x1F08, 0x1F0F, 1, -8),
  (0x1F18, 0x1F1D, 1, -8),
  (0x1F28, 0x1F2F, 1, -8),
  (0x1F38, 0x1F3F, 1, -8),
  (0x1F48, 0x1F4D, 1, -8),
  (0x1F59, 0x1F5F, 2, -8),
  (0x1F68, 0x1F6F, 1, -8),
  (0x1F88, 0x1F8F, 1, -8),
  (0x1F98, 0x1F9F, 1, -8),
  (0x1FA8, 0x1FAF, 1, -8),
  (0x1FB8, 0x1FB9, 1, -8),
  (0x1FBA, 0x1FBB, 1, -74),
  (0x1FBC, 0x1FBC, 1, -9),
  (0x1FC8, 0x1FCB, 1, -86),
  (0x1FCC, 0x1FCC, 1, -9),
  (0x1FD8, 0x1FD9, 1, -8),
  (0x1FDA, 0x1FDB, 1, -100),
  (0x1FE8, 0x1FE9, 1, -8),
  (0x1FEA, 0x1FEB, 1, -112),
  (0x1FEC, 0x1FEC, 1, -7),
  (0x1FF8, 0x1FF9, 1, -128),
  (0x1FFA, 0x1FFB, 1, -126),
  (0x1FFC, 0x1FFC, 1, -9),
  (0x2126, 0x2126, 1, -7517),
  (0x212A, 0x212A, 1, -8383),
  (0x212B, 0x212B, 1, -8262),
  (0x2132, 0x2132, 1, 28),
  (0x2160, 0x216F, 1, 16),
  (0x2183, 0x2183, 1, 1),
  (0x24B6, 0x24CF, 1, 26),
  (0x2C00, 0x2C2F, 1, 48),
  (0x2C60, 0x2C60, 1, 1),
  (0x2C62, 0x2C62, 1, -10743),
  (0x2C63, 0x2C63, 1, -3814),
  (0x2C64, 0x2C64, 1, -10727),
  (0x2C67, 0x2C6B, 2, 1),
  (0x2C6D, 0x2C6D, 1, -10780),
  (0x2C6E, 0x2C6E, 1, -10749),
  (0x2C6F, 0x2C6F, 1, -10783),
  (0x2C70, 0x2C70, 1, -10782),
  (0x2C72, 0x2C72, 1, 1),
  (0x2C75, 0x2C75, 1, 1),
  (0x2C7E, 0x2C7F, 1, -10815),
  (0x2C80, 0x2CE2, 2, 1),
  (0x2CEB, 0x2CED, 2, 1),
  (0x2CF2, 0x2CF2, 1, 1),
  (0xA640, 0xA66C, 2, 1),
  (0xA680, 0xA69A, 2, 1),
  (0xA722, 0xA72E, 2, 1),
  (0xA732, 0xA76E, 2, 1),
  (0xA779, 0xA77B, 2, 1),
  (0xA77D, 0xA77D, 1, -35332),
  (0xA77E, 0xA786, 2, 1),
  (0xA78B, 0xA78B, 1, 1),
  (0xA78D, 0xA78D, 1, -42280),
  (0xA790, 0xA792, 2, 1),
  (0xA796, 0xA7A8, 2, 1),
  (0xA7AA, 0xA7AA, 1, -42308),
  (0xA7AB, 0xA7AB, 1, -42319),
  (0xA7AC, 0xA7AC, 1, -42315),
  (0xA7AD, 0xA7AD, 1, -42305),
  (0xA7AE, 0xA7AE, 1, -42308),
  (0xA7B0, 0xA7B0, 1, -42258),
  (0xA7B1, 0xA7B1, 1, -42282),
  (0xA7B2, 0xA7B2, 1, -42261),
  (0xA7B3, 0xA7B3, 1, 928),
  (0xA7B4, 0xA7C2, 2, 1),
  (0xA7C4, 0xA7C4, 1, -48),
  (0xA7C5, 0xA7C5, 1, -42307),
  (0xA7C6, 0xA7C6, 1, -35384),
  (0xA7C7, 0xA7C9, 2, 1),
  (0xA7CB, 0xA7CB, 1, -42343),
  (0xA7CC, 0xA7DA, 2, 1),
  (0xA7DC, 0xA7DC, 1, -42561),
  (0xA7DD, 0xA7DD, 1, -42342),
  (0xA7E2, 0xA7E2, 1, -42342),
  (0xA7F5, 0xA7F5, 1, 1),
  (0xAB6C, 0xAB6D, 1, -33),
  (0xFF21, 0xFF3A, 1, 32),
  (0x10400, 0x10427, 1, 40),
  (0x104B0, 0x104D3, 1, 40),
  (0x10570, 0x1057A, 1, 39),
  (0x1057C, 0x1058A, 1, 39),
  (0x1058C, 0x10592, 1, 39),
  (0x10594, 0x10595, 1, 39),
  (0x10C80, 0x10CB2, 1, 64),
  (0x10D50, 0x10D65, 1, 32),
  (0x118A0, 0x118BF, 1, 32),
  (0x16E40, 0x16E5F, 1, 32),
  (0x16EA0, 0x16EB8, 1, 27),
  (0x1DF40, 0x1DF40, 1, 1),
  (0x1DF48, 0x1DF4A, 2, 1),
  (0x1DF4D, 0x1DF4D, 1, 1),
  (0x1DF51, 0x1DF51, 1, 1),
  (0x1DF68, 0x1DF6E, 2, 1),
  (0x1DF72, 0x1DF7E, 2, 1),
  (0x1E900, 0x1E921, 1, 34),
];

/// A line of a text as it reads, each of its characters with where it
/// is written, [from] to [to], and the column of a table's row it lies
/// in.
class _Line {
  final int line;
  final int? table;
  final int tableStart;
  final int row;
  final runes = <int>[];
  final from = <int>[];
  final to = <int>[];
  final columns = <int>[];

  _Line(this.line, {this.table, this.tableStart = 0, this.row = 0});

  void add(int rune, int start, int end, int column) {
    runes.add(rune);
    from.add(start);
    to.add(end);
    columns.add(column);
  }
}
