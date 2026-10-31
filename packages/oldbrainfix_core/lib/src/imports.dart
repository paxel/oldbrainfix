import 'dart:convert';
import 'dart:typed_data';

import 'notes.dart';
import 'places.dart';
import 'store.dart';
import 'text_boxes.dart';
import 'text_format.dart';

/// One file handed to an Import: its name, when it was last changed
/// (null when the app cannot tell) and its bytes. The apps pick the
/// folder and read its files; the core does the rest (ADR 0036).
class ImportFile {
  /// The file's name; a path's folders before the last `/` or `\` are
  /// left out.
  final String name;
  final DateTime? modified;
  final Uint8List bytes;

  const ImportFile(this.name, this.modified, this.bytes);
}

/// One Note an Import makes: its [text] in its [format], when it was
/// [captured], and the [name] of what it came from, which orders Notes
/// of the same date.
class ImportedNote {
  final String name;
  final String text;
  final String format;
  final DateTime captured;

  const ImportedNote({
    required this.name,
    required this.text,
    required this.format,
    required this.captured,
  });
}

/// What an Import made, for the apps to announce.
class ImportSummary {
  /// The new Area the Notes landed in; null when there was nothing to
  /// import and no Area was made.
  final String? areaId;

  /// How many Notes were made.
  final int notes;

  /// Files skipped as no text: another kind of file, a hidden one, one
  /// holding a zero byte, or one with nothing in it or its name.
  final int notText;

  /// Files skipped as too large ([Imports.maxFileBytes],
  /// [Imports.maxTextBytes], [Imports.maxTotalBytes]).
  final int tooLarge;

  const ImportSummary({
    required this.areaId,
    required this.notes,
    required this.notText,
    required this.tooLarge,
  });

  /// How many files were skipped.
  int get skipped => notText + tooLarge;
}

/// Imports: Notes made from the files of a folder, into a new Area named
/// after it, each dated by its file, none of them Nudged (ADR 0036).
class Imports {
  /// The largest file read; the apps need not read a larger one.
  static const maxFileBytes = 1 << 20;

  /// The largest text kept, as stored with its format: a Backup reads an
  /// entry of up to 1 MiB, its value escaped once more inside it.
  static const maxTextBytes = 256 << 10;

  /// The most text one Import keeps, as stored; the newest files are
  /// kept first.
  static const maxTotalBytes = 32 << 20;

  final Store store;

  Imports(this.store);

  /// Makes one Note of each Markdown (`.md`, `.markdown`) and text
  /// (`.txt`) file of [files] in a new Area named [areaName], as [notes]
  /// writes them; any other file is skipped and counted. A Markdown
  /// file's Note is formatted, its Markdown as written; a text file's is
  /// plain. A Note is dated by its file's modification time, and by now
  /// when that is unknown or later.
  ImportSummary folder(String areaName, List<ImportFile> files) {
    final name = _areaName(areaName);
    final now = store.clock().toUtc();
    final made = <ImportedNote>[];
    var notText = 0;
    var tooLarge = 0;
    for (final file in files) {
      final base = _base(file.name);
      final format = _format(base);
      if (format == null) {
        notText++;
        continue;
      }
      if (file.bytes.length > maxFileBytes) {
        tooLarge++;
        continue;
      }
      final read = _decode(file.bytes);
      if (read == null) {
        notText++;
        continue;
      }
      final text = _titled(
        base.substring(0, base.lastIndexOf('.')),
        read,
        format,
      );
      if (text.isEmpty) {
        notText++;
        continue;
      }
      var at = file.modified?.toUtc();
      if (at == null || !at.isAfter(_epoch) || at.isAfter(now)) at = now;
      made.add(
        ImportedNote(name: base, text: text, format: format, captured: at),
      );
    }
    return notes(name, made, notText: notText, tooLarge: tooLarge);
  }

  /// Writes [notes] into a new Area named [areaName], oldest first,
  /// filed there so none of them is Unsorted or Nudged, each captured
  /// and dated when it says. An Area of that name already there gets a
  /// new one beside it, named with a number after it. A text larger
  /// than [maxTextBytes], and the oldest beyond [maxTotalBytes], are
  /// skipped as too large, counted on top of [notText] and [tooLarge];
  /// with nothing left, no Area is made.
  ImportSummary notes(
    String areaName,
    List<ImportedNote> notes, {
    int notText = 0,
    int tooLarge = 0,
  }) {
    final name = _areaName(areaName);
    for (final note in notes) {
      if (!TextFormats.known.contains(note.format)) {
        throw ArgumentError.value(note.format, 'format', 'not known here');
      }
    }
    final order = [
      for (final (i, note) in notes.indexed)
        (i: i, note: note, at: note.captured.toUtc()),
    ];
    order.sort((a, b) {
      final byDate = a.at.compareTo(b.at);
      if (byDate != 0) return byDate;
      final byName = _byCodePoints(a.note.name, b.note.name);
      return byName != 0 ? byName : a.i.compareTo(b.i);
    });
    final kept = <({ImportedNote note, DateTime at})>[];
    var large = tooLarge;
    var total = 0;
    for (final o in order.reversed) {
      final size = utf8
          .encode(StoredText.of(o.note.format, o.note.text).encode())
          .length;
      if (size > maxTextBytes || total + size > maxTotalBytes) {
        large++;
        continue;
      }
      total += size;
      kept.add((note: o.note, at: o.at));
    }
    if (kept.isEmpty) {
      return ImportSummary(
        areaId: null,
        notes: 0,
        notText: notText,
        tooLarge: large,
      );
    }
    final areaId = store.batch(() {
      final area = Places(store).addArea(_unused(name));
      for (final (:note, :at) in kept.reversed) {
        final id = 'note:${Store.newId()}';
        store.append(id, NoteKeys.type, NoteKeys.typeNote, date: at);
        store.append(id, NoteKeys.captured, at.toIso8601String(), date: at);
        final boxed = TextBoxes.convert(
          note.text,
          format: note.format,
          newId: Store.newId,
        );
        store.append(
          id,
          NoteKeys.text,
          StoredText.of(note.format, boxed).encode(),
          date: at,
        );
        store.append(id, NoteKeys.place, area.id, date: at);
      }
      return area.id;
    });
    return ImportSummary(
      areaId: areaId,
      notes: kept.length,
      notText: notText,
      tooLarge: large,
    );
  }

  /// [name], or with " (2)", " (3)" and so on after it when an Area is
  /// called so already.
  String _unused(String name) {
    final taken = {for (final area in Places(store).areas()) area.name};
    if (!taken.contains(name)) return name;
    for (var n = 2; ; n++) {
      final candidate = '$name ($n)';
      if (!taken.contains(candidate)) return candidate;
    }
  }

  static String _areaName(String name) {
    final trimmed = name.trim();
    if (trimmed.isEmpty) {
      throw ArgumentError.value(name, 'areaName', 'must not be empty');
    }
    return trimmed;
  }

  /// Modification times up to this one are taken as unknown, as a
  /// platform says 0 for one it does not know.
  static final _epoch = DateTime.utc(1970);

  /// The part of [name] after its last `/` or `\`.
  static String _base(String name) {
    final slash = name.lastIndexOf('/');
    final backslash = name.lastIndexOf(r'\');
    return name.substring((slash > backslash ? slash : backslash) + 1);
  }

  /// The format of the file [base] becomes a Note in, by its extension
  /// in any case of ASCII letters; null for a hidden file and any other
  /// kind.
  static String? _format(String base) {
    if (base.startsWith('.')) return null;
    final dot = base.lastIndexOf('.');
    if (dot < 0) return null;
    return switch (_asciiLower(base.substring(dot + 1))) {
      'md' || 'markdown' => TextFormats.markdown,
      'txt' => TextFormats.plain,
      _ => null,
    };
  }

  static String _asciiLower(String text) => String.fromCharCodes([
    for (final c in text.codeUnits) c >= 0x41 && c <= 0x5a ? c + 0x20 : c,
  ]);

  /// The text of [bytes]: UTF-8 without the byte order mark before it,
  /// else Windows-1252, the encoding of older text files on Windows,
  /// which reads every byte; line breaks as `\n`. Null for bytes holding
  /// a zero, which no text file does.
  static String? _decode(Uint8List bytes) {
    String text;
    try {
      // Drops a byte order mark at the start.
      text = utf8.decode(bytes);
    } on FormatException {
      text = String.fromCharCodes([
        for (final b in bytes) b >= 0x80 && b < 0xa0 ? _cp1252[b - 0x80] : b,
      ]);
    }
    if (text.contains('\u0000')) return null;
    return text.replaceAll('\r\n', '\n').replaceAll('\r', '\n');
  }

  /// What Windows-1252 has at 0x80 to 0x9F; the five bytes it leaves
  /// open read as Latin-1 does.
  static const _cp1252 = [
    0x20ac, 0x0081, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, //
    0x02c6, 0x2030, 0x0160, 0x2039, 0x0152, 0x008d, 0x017d, 0x008f, //
    0x0090, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, //
    0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0x009d, 0x017e, 0x0178, //
  ];

  /// [text] without its blank lines at the start and blanks at the end,
  /// under the file's name [stem] as its first line, a heading in
  /// Markdown, unless its first line with words already says it (the
  /// Note's title, in any case).
  static String _titled(String stem, String text, String format) {
    final trimmed = text.trimRight();
    final lead = trimmed.length - trimmed.trimLeft().length;
    final body = trimmed.substring(trimmed.lastIndexOf('\n', lead) + 1);
    final name = stem.replaceAll('\r', ' ').replaceAll('\n', ' ').trim();
    if (name.isEmpty) return body;
    final visible = TextFormats.visible(format, body);
    for (final line in visible.split('\n')) {
      final words = line.trim();
      if (words.isEmpty) continue;
      if (words.toLowerCase() == name.toLowerCase()) return body;
      break;
    }
    final title = format == TextFormats.markdown
        ? '# ${TextFormats.literal(format, name)}'
        : name;
    return body.isEmpty ? title : '$title\n$body';
  }

  /// [a] against [b] by code points, as the Rust core compares strings.
  static int _byCodePoints(String a, String b) {
    final x = a.runes.toList();
    final y = b.runes.toList();
    for (var i = 0; i < x.length && i < y.length; i++) {
      if (x[i] != y[i]) return x[i].compareTo(y[i]);
    }
    return x.length.compareTo(y.length);
  }
}
