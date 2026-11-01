import 'dart:convert';
import 'dart:typed_data';

import 'checklists.dart';
import 'notes.dart';
import 'places.dart';
import 'store.dart';
import 'text_boxes.dart';
import 'text_format.dart';
import 'text_tags.dart';

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

/// One Line of a Checklist an Import makes: its [text], and when it was
/// [checked], null while it is not.
class ImportedLine {
  final String text;
  final DateTime? checked;

  const ImportedLine(this.text, {this.checked});
}

/// One Note an Import makes: its [text] in its [format], when it was
/// [captured], and the [name] of what it came from, which orders Notes
/// of the same date. With [lines] it is a Checklist of them, an empty
/// one too; it holds the [photos], as their bytes.
class ImportedNote {
  final String name;
  final String text;
  final String format;
  final DateTime captured;
  final List<ImportedLine>? lines;
  final List<Uint8List> photos;

  const ImportedNote({
    required this.name,
    required this.text,
    required this.format,
    required this.captured,
    this.lines,
    this.photos = const [],
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
  /// [Imports.maxTextBytes], [Imports.maxTotalBytes],
  /// [Imports.maxPhotoBytes]).
  final int tooLarge;

  /// How many of the [notes] are Checklists.
  final int checklists;

  /// How many photos the [notes] hold.
  final int photos;

  /// Notes skipped as in the trash of the app they came from.
  final int trashed;

  /// Files a note names that are not among those handed over.
  final int missing;

  /// Files a note names that are no photo, such as a voice recording.
  final int notPhoto;

  const ImportSummary({
    required this.areaId,
    required this.notes,
    required this.notText,
    required this.tooLarge,
    this.checklists = 0,
    this.photos = 0,
    this.trashed = 0,
    this.missing = 0,
    this.notPhoto = 0,
  });

  /// How many files and notes were skipped.
  int get skipped => notText + tooLarge + trashed + missing + notPhoto;
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

  /// The largest photo kept; the apps need not read a larger one.
  static const maxPhotoBytes = 16 << 20;

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
  /// and dated when it says, with its photos and, for a Checklist, its
  /// Lines, checked when they say. An Area of that name already there gets a
  /// new one beside it, named with a number after it. A text larger
  /// than [maxTextBytes], and the oldest beyond [maxTotalBytes], are
  /// skipped as too large, counted on top of [notText] and [tooLarge];
  /// with nothing left, no Area is made.
  ImportSummary notes(
    String areaName,
    List<ImportedNote> notes, {
    int notText = 0,
    int tooLarge = 0,
  }) => _write(
    areaName,
    notes,
    ImportSummary(areaId: null, notes: 0, notText: notText, tooLarge: tooLarge),
  );

  /// [notes] with the files and notes [skipped] before them counted.
  ImportSummary _write(
    String areaName,
    List<ImportedNote> notes,
    ImportSummary skipped,
  ) {
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
    var large = skipped.tooLarge;
    var total = 0;
    for (final o in order.reversed) {
      var size = _size(StoredText.of(o.note.format, o.note.text));
      for (final line in o.note.lines ?? const <ImportedLine>[]) {
        size += _size(StoredText.plain(line.text));
      }
      if (size > maxTextBytes || total + size > maxTotalBytes) {
        large++;
        continue;
      }
      total += size;
      kept.add((note: o.note, at: o.at));
    }
    ImportSummary summary({
      String? areaId,
      int notes = 0,
      int checklists = 0,
      int photos = 0,
    }) => ImportSummary(
      areaId: areaId,
      notes: notes,
      notText: skipped.notText,
      tooLarge: large,
      checklists: checklists,
      photos: photos,
      trashed: skipped.trashed,
      missing: skipped.missing,
      notPhoto: skipped.notPhoto,
    );
    if (kept.isEmpty) return summary();
    final writing = kept.reversed.toList();
    // The bytes are kept before the log names them, each photo once.
    final hashes = [
      for (final (:note, at: _) in writing)
        {for (final bytes in note.photos) store.putPhoto(bytes)}.toList(),
    ];
    final areaId = store.batch(() {
      final area = Places(store).addArea(_unused(name));
      for (final (i, (:note, :at)) in writing.indexed) {
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
        for (final hash in hashes[i]) {
          store.append(id, NoteKeys.photo(hash), NoteKeys.photoAdded, date: at);
        }
        if (note.lines case final lines?) _writeLines(id, lines, at);
      }
      return area.id;
    });
    return summary(
      areaId: areaId,
      notes: kept.length,
      checklists: kept.where((k) => k.note.lines != null).length,
      photos: hashes.fold(0, (n, h) => n + h.length),
    );
  }

  /// Makes the new Note [noteId] a Checklist of [lines], not spawned, as
  /// a Sent Item does, since it is filed already: each Line written at
  /// [at], its check, when it has one, when it says but not before.
  /// Lines with no text are left out. It starts off today's Agenda for
  /// this person, so an Import of many lists does not fill it.
  void _writeLines(String noteId, List<ImportedLine> lines, DateTime at) {
    final making = Store.newId();
    store.append(
      noteId,
      ChecklistKeys.kind,
      ChecklistKeys.kindOf(making),
      date: at,
    );
    var key = '';
    for (final line in lines) {
      if (line.text.trim().isEmpty) continue;
      key = LineOrder.after(key);
      final id = writeLine(
        store,
        noteId,
        making,
        text: lineText(line.text),
        order: key,
        date: at,
      );
      if (line.checked case final checked?) {
        final on = checked.toUtc().isBefore(at) ? at : checked.toUtc();
        store.append(id, ChecklistKeys.checked, on.toIso8601String(), date: on);
      }
    }
    Checklists(store).setShowOnAgenda(noteId, false);
  }

  static int _size(StoredText text) => utf8.encode(text.encode()).length;

  /// The most bytes the import from Google Keep reads of the file at
  /// [path], an entry of a Takeout export: [maxFileBytes] of a note's
  /// `.json`, [maxPhotoBytes] of a photo. Null for any other file, which
  /// it does not read; the apps need not hand it over. A file larger
  /// than that may be handed over as that many bytes and one more,
  /// unread, to be counted as too large.
  static int? keepReads(String path) {
    final base = _base(path);
    if (base.startsWith('.')) return null;
    final extension = _extension(base);
    if (extension == 'json') return maxFileBytes;
    if (_photoKinds.contains(extension)) return maxPhotoBytes;
    return null;
  }

  /// Makes one Note of each note of a Google Keep export in a new Area
  /// named [areaName], as [notes] writes them (ADR 0036). [entries] are
  /// the files of its Takeout, a zip's or a folder's, by their paths:
  /// each note's `.json`, which is all that is read of it, and the
  /// photos they name. When any of them lies in a folder named `Keep`,
  /// only those do. A note in Keep's trash is skipped; an archived one
  /// is made as any other. Its title is its first line, its labels are
  /// tags in a last line, a list is a Checklist with its Lines checked as
  /// in Keep, its photos are its photos. It is captured when Keep says it
  /// was made, and its checks are dated when it was last changed.
  ImportSummary keep(String areaName, List<ImportFile> entries) {
    final name = _areaName(areaName);
    final now = store.clock().toUtc();
    final inKeep = entries.any((e) => _inKeep(e.name));
    final taken = [
      for (final e in entries)
        if (!inKeep || _inKeep(e.name)) e,
    ];
    final byName = <String, ImportFile>{};
    for (final e in taken) {
      byName.putIfAbsent(_base(e.name), () => e);
    }
    final made = <ImportedNote>[];
    var notText = 0;
    var tooLarge = 0;
    var trashed = 0;
    var missing = 0;
    var notPhoto = 0;
    for (final entry in taken) {
      final base = _base(entry.name);
      if (base.startsWith('.') || _extension(base) != 'json') continue;
      if (entry.bytes.length > maxFileBytes) {
        tooLarge++;
        continue;
      }
      final note = _keepNote(entry.bytes);
      if (note == null) {
        notText++;
        continue;
      }
      if (note['isTrashed'] == true) {
        trashed++;
        continue;
      }
      final created = _usec(note['createdTimestampUsec'], now);
      final edited = _usec(note['userEditedTimestampUsec'], now);
      final at = created ?? edited ?? now;
      final checkedAt = edited != null && !edited.isBefore(at) ? edited : at;
      final body = _titled(
        _string(note['title']),
        _string(note['textContent']),
        TextFormats.plain,
      );
      final lines = switch (note['listContent']) {
        final List<Object?> items => [
          for (final item in items) ?_line(item, checkedAt),
        ],
        _ => null,
      };
      final photos = <Uint8List>[];
      if (note['attachments'] case final List<Object?> attachments) {
        for (final attachment in attachments) {
          final path = switch (attachment) {
            {'filePath': final String path} => _base(path),
            _ => '',
          };
          if (!_photoKinds.contains(_extension(path))) {
            path.isEmpty ? missing++ : notPhoto++;
            continue;
          }
          final file = byName[path] ?? byName[_otherJpeg(path)];
          if (file == null) {
            missing++;
          } else if (file.bytes.length > maxPhotoBytes) {
            tooLarge++;
          } else {
            photos.add(file.bytes);
          }
        }
      }
      if (body.isEmpty && (lines?.isEmpty ?? true) && photos.isEmpty) {
        notText++;
        continue;
      }
      final tags = _tags(note['labels'], body);
      final text = tags.isEmpty
          ? body
          : body.isEmpty
          ? tags
          : '$body\n$tags';
      made.add(
        ImportedNote(
          name: base,
          text: text,
          format: TextFormats.plain,
          captured: at,
          lines: lines,
          photos: photos,
        ),
      );
    }
    return _write(
      name,
      made,
      ImportSummary(
        areaId: null,
        notes: 0,
        notText: notText,
        tooLarge: tooLarge,
        trashed: trashed,
        missing: missing,
        notPhoto: notPhoto,
      ),
    );
  }

  /// The extensions of the photos a Keep note may hold, in lower case.
  static const _photoKinds = {'jpg', 'jpeg', 'png', 'webp', 'gif'};

  /// Whether the file at [path] lies in a folder named `Keep`, in any
  /// case of ASCII letters.
  static bool _inKeep(String path) {
    final folders = path.split(RegExp(r'[/\\]'));
    return folders.length > 1 &&
        _asciiLower(folders[folders.length - 2]) == 'keep';
  }

  /// The extension of the file [base] in lower case ASCII letters, empty
  /// for none.
  static String _extension(String base) {
    final dot = base.lastIndexOf('.');
    return dot < 0 ? '' : _asciiLower(base.substring(dot + 1));
  }

  /// [base] with `.jpg` for `.jpeg` and the other way round, as a Keep
  /// export may name a photo by one and hold it by the other; null for
  /// any other file.
  static String? _otherJpeg(String base) {
    final dot = base.lastIndexOf('.');
    return switch (_extension(base)) {
      'jpeg' => '${base.substring(0, dot)}.jpg',
      'jpg' => '${base.substring(0, dot)}.jpeg',
      _ => null,
    };
  }

  /// The note a Keep `.json` holds: an object with Keep's time of making
  /// or of the last change as a whole number; null for anything else.
  static Map<String, Object?>? _keepNote(Uint8List bytes) {
    final Object? json;
    try {
      // Drops a byte order mark at the start.
      json = jsonDecode(utf8.decode(bytes));
    } on FormatException {
      return null;
    }
    if (json is! Map<String, Object?>) return null;
    if (json['createdTimestampUsec'] is! int &&
        json['userEditedTimestampUsec'] is! int) {
      return null;
    }
    return json;
  }

  /// The Line a Keep list's [item] is, checked at [checkedAt] when Keep
  /// says it is; null for one without text.
  static ImportedLine? _line(Object? item, DateTime checkedAt) {
    if (item is! Map<String, Object?>) return null;
    final text = item['text'];
    if (text is! String || text.trim().isEmpty) return null;
    final checked = item['isChecked'] == true ? checkedAt : null;
    return ImportedLine(text, checked: checked);
  }

  /// The moment [value] says in microseconds since 1970, when it is a
  /// whole number after 1970 and not later than [now]; else null.
  static DateTime? _usec(Object? value, DateTime now) =>
      value is int && value > 0 && value <= now.microsecondsSinceEpoch
      ? DateTime.fromMicrosecondsSinceEpoch(value, isUtc: true)
      : null;

  /// [value] with its line breaks as `\n` when it is text; else empty.
  static String _string(Object? value) => value is String
      ? value.replaceAll('\r\n', '\n').replaceAll('\r', '\n')
      : '';

  /// The tags of the Keep [labels] ([TextTags.fromLabel]) not in [body]
  /// yet, each once whatever its case, in their order, apart by spaces.
  static String _tags(Object? labels, String body) {
    if (labels is! List<Object?>) return '';
    final seen = {for (final t in TextTags.find(body)) t.tag};
    final tags = <String>[];
    for (final label in labels) {
      if (label case {'name': final String name}) {
        final tag = TextTags.fromLabel(name);
        if (tag != null && seen.add(tag.toLowerCase())) tags.add(tag);
      }
    }
    return tags.join(' ');
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
