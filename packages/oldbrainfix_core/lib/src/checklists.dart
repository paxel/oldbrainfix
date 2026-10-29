import 'notes.dart';
import 'sent_items.dart';
import 'shared_areas.dart';
import 'store.dart';
import 'tallies.dart';
import 'text_format.dart';

/// Field keys of a Checklist (ADR 0030). Which list a Note is lies on
/// the Note; its Lines are entities of their own beside it, each with
/// its text, its place, whether it is under the Line above and whether
/// it is checked.
abstract final class ChecklistKeys {
  /// On the Note: the list it is and the id of that making, as
  /// `checklist:<making id>`; absent, or null, for a plain Note.
  static const kind = 'kind';

  /// The list of a Checklist in [kind].
  static const checklist = 'checklist';

  /// The value of [kind] for the making [making] of a Checklist.
  static String kindOf(String making) => '$checklist:$making';

  static const type = r'$type';
  static const typeLine = 'line';

  /// The id prefix of a Line.
  static const linePrefix = 'line:';

  /// The Note the Line belongs to.
  static const note = 'note';

  /// The making of the list the Line belongs to: only the Lines of the
  /// making the Note's [kind] names show.
  static const made = 'made';

  /// The Line's text, as [StoredText.encode] writes it.
  static const text = 'text';

  /// Its place among the Lines: a key [LineOrder] makes.
  static const order = 'order';

  /// `true` while it is under the Line above; absent or null when not.
  static const indent = 'indent';

  /// When it was checked; absent or null while it is not.
  static const checked = 'checked';

  /// When it was deleted; absent or null while it is not.
  static const trashed = 'trashed';

  /// What marks a checked Line in a plain Note's text.
  static const checkMark = '✓ ';

  /// What a Line of a group starts with in a plain Note's text.
  static const groupIndent = '  ';
}

/// The list a Note's `kind` names and the id of its making; null for a
/// value that names none.
({String list, String making})? parseListKind(String? value) {
  if (value == null) return null;
  final colon = value.indexOf(':');
  if (colon <= 0 || colon == value.length - 1) return null;
  final making = value.substring(colon + 1);
  if (making.contains(':')) return null;
  return (list: value.substring(0, colon), making: making);
}

/// The keys that place Lines (ADR 0030): digits of base 62 in ASCII
/// order, compared as text, never ending in `0`, so a key fits between
/// any two without touching another Line.
abstract final class LineOrder {
  static const digits =
      '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';

  static final _valid = RegExp(r'^[0-9A-Za-z]*[1-9A-Za-z]$');

  /// Whether [key] is one this order reads: any other sorts after them.
  static bool isValid(String? key) => key != null && _valid.hasMatch(key);

  /// A key after the valid key [a], or the first key for an empty one:
  /// the first digit that can grow grows, so keys stay short.
  static String after(String a) {
    for (var i = 0; i < a.length; i++) {
      final d = digits.indexOf(a[i]);
      if (d < 0) break;
      if (d < digits.length - 1) return '${a.substring(0, i)}${digits[d + 1]}';
    }
    return '$a${digits[digits.length ~/ 2]}';
  }

  /// A key between [a] and [b]: [a] valid or empty, [b] valid, [a]
  /// before [b].
  static String between(String a, String b) {
    var n = 0;
    while (n < b.length && (n < a.length ? a[n] : '0') == b[n]) {
      n++;
    }
    if (n >= b.length) return after(a);
    final prefix = b.substring(0, n);
    final da = n < a.length ? digits.indexOf(a[n]) : 0;
    final db = digits.indexOf(b[n]);
    if (da < 0 || db <= da) return after(a);
    if (db - da > 1) return '$prefix${digits[(da + db) ~/ 2]}';
    if (b.length > n + 1) return b.substring(0, n + 1);
    final rest = n < a.length ? a.substring(n + 1) : '';
    return '$prefix${digits[da]}${after(rest)}';
  }

  /// The key for a place after [a] and before [b], either null for no
  /// Line there.
  static String place(String? a, String? b) => switch ((a, b)) {
    (null, null) => after(''),
    (null, final b?) => between('', b),
    (final a?, null) => after(a),
    (final a?, final b?) => between(a, b),
  };
}

/// One line of a Checklist that is checked off (CONTEXT.md).
class Line {
  /// Entity id, `line:<id>`.
  final String id;
  final String noteId;
  final String text;

  /// Whether it shows checked: a Line with a group when every Line of
  /// the group is, any other when it was checked last.
  final bool checked;

  /// When it was checked, as its own entry says; null while it is not.
  /// A Line with a group may show checked without one.
  final DateTime? checkedAt;

  /// The Line it shows under, or null for one at the top.
  final String? parent;

  /// Its place as stored ([LineOrder]).
  final String? order;

  const Line({
    required this.id,
    required this.noteId,
    required this.text,
    required this.checked,
    required this.checkedAt,
    required this.parent,
    required this.order,
  });

  /// Whether it shows under a parent, in its group.
  bool get inGroup => parent != null;
}

/// A Note whose Lines are checked off (CONTEXT.md).
class Checklist {
  final String noteId;

  /// The id of its making, as the Note's `kind` names it.
  final String making;

  /// Every Line, in the order they show: each parent followed by its
  /// group.
  final List<Line> lines;

  const Checklist(this.noteId, this.making, this.lines);

  /// The Line [id], or null.
  Line? line(String id) {
    for (final line in lines) {
      if (line.id == id) return line;
    }
    return null;
  }

  /// The group of the Line [id]: the Lines under it, in order.
  List<Line> groupOf(String id) => [
    for (final line in lines)
      if (line.parent == id) line,
  ];

  /// How many Lines show checked, and how many there are.
  int get checkedCount => lines.where((l) => l.checked).length;
  int get count => lines.length;

  bool get allChecked => lines.every((l) => l.checked);
}

/// Making Notes Checklists and back, and adding, changing, moving and
/// checking their Lines (ADR 0030).
class Checklists {
  final Store store;

  Checklists(this.store);

  /// The Checklist of the Note [noteId], or null when it is none: a
  /// plain Note, a Tally (the Tally wins), or no Note.
  Checklist? byNote(String noteId) => ofNotes([noteId])[noteId];

  /// Whether the Note [noteId] is a Checklist.
  bool isChecklist(String noteId) => byNote(noteId) != null;

  /// The Checklists of the Notes [noteIds] that are one, read in one
  /// pass over the Lines: for a list that shows many Notes in full.
  Map<String, Checklist> ofNotes(Iterable<String> noteIds) {
    final makings = <String, String>{};
    for (final id in noteIds) {
      if (_making(id) case final making?) makings[id] = making;
    }
    if (makings.isEmpty) return const {};
    final raw = <String, List<_Raw>>{};
    for (final id in store.entitiesOfType(ChecklistKeys.typeLine)) {
      final fields = store.currentFields(id);
      final note = fields[ChecklistKeys.note];
      if (note == null || fields[ChecklistKeys.trashed] != null) continue;
      final making = makings[note];
      if (making == null || fields[ChecklistKeys.made] != making) continue;
      (raw[note] ??= []).add(_Raw(id, fields));
    }
    return {
      for (final MapEntry(key: note, value: making) in makings.entries)
        note: Checklist(note, making, _shown(note, raw[note] ?? [])),
    };
  }

  /// Whether this Device may make the Note [noteId] a Checklist: a Note
  /// in the lists, of a text known here, neither a Tally nor a list
  /// already, which it may change.
  bool mayMake(String noteId) {
    final note = Notes(store).byId(noteId);
    return note != null &&
        !note.isTrashed &&
        !note.isMerged &&
        note.textEditable &&
        parseListKind(store.current(noteId, ChecklistKeys.kind)) == null &&
        !Tallies(store).isTally(noteId) &&
        store.mayChange(noteId);
  }

  /// Makes the Note [noteId] a Checklist: its first line stays its text,
  /// every other line not blank becomes a Line, under the line above
  /// when it starts with a space or a tab, checked when it starts with
  /// "✓ ". Takes it out of Unsorted, as a Tally does.
  void make(String noteId) {
    final note = Notes(store).byId(noteId);
    if (note == null || !mayMake(noteId)) {
      throw StateError('$noteId may not be made a Checklist here');
    }
    final rows = splitLines(note.text);
    final title = rows.first.trim();
    final parsed = [for (final row in rows.skip(1)) ?parseLine(row)];
    final making = Store.newId();
    store.batch(() {
      store.append(noteId, ChecklistKeys.kind, ChecklistKeys.kindOf(making));
      if (title != note.text) {
        store.append(noteId, NoteKeys.text, StoredText.plain(title).encode());
      }
      if (!note.spawned) {
        store.append(
          noteId,
          NoteKeys.spawned,
          store.clock().toUtc().toIso8601String(),
        );
      }
      var key = '';
      for (final (i, line) in parsed.indexed) {
        key = LineOrder.after(key);
        writeLine(
          store,
          noteId,
          making,
          text: line.text,
          order: key,
          indent: i > 0 && line.indent,
          checked: line.checked,
        );
      }
    });
  }

  /// Turns the Checklist of [noteId] back into a plain Note: every Line
  /// goes into its text, one per line, under its text, a checked one
  /// marked "✓ ", one of a group indented. The Lines and their checks
  /// stay in the log.
  void toPlain(String noteId) {
    final (note, list) = _requireList(noteId);
    if (!store.mayChange(noteId)) throw StateError('may not change $noteId');
    if (!note.textEditable) {
      throw StateError('the text of $noteId is in a format not known here');
    }
    final text = plainText(note.text, list);
    store.batch(() {
      store.append(noteId, NoteKeys.text, StoredText.plain(text).encode());
      store.append(noteId, ChecklistKeys.kind, null);
    });
  }

  /// The text the Checklist [list] of a Note titled [title] goes back
  /// to.
  static String plainText(String title, Checklist list) => [
    if (title.isNotEmpty) title,
    for (final line in list.lines)
      '${line.inGroup ? ChecklistKeys.groupIndent : ''}'
          '${line.checked ? ChecklistKeys.checkMark : ''}${line.text}',
  ].join('\n');

  /// Whether this Device may add Lines to the Checklist of [noteId] and
  /// check them: anywhere it holds a role.
  bool mayCheck(String noteId) => store.mayWriteIn(noteId);

  /// Whether this Device may rename, delete, move and indent the Line
  /// [lineId]: in no Shared Area or one it manages, or as the author of
  /// the Line or of its Note.
  bool mayEdit(String lineId) {
    if (store.mayChange(lineId)) return true;
    final note = store.current(lineId, ChecklistKeys.note);
    return note != null && store.mayChange(note);
  }

  /// Adds a Line holding [text] to the Checklist of [noteId] at [at], an
  /// index in the order the Lines show, or at the end; under the Line
  /// above with [indent]. Anyone with a role in its Area may. Returns
  /// its id.
  String add(String noteId, String text, {int? at, bool indent = false}) {
    final (_, list) = _requireList(noteId);
    if (!mayCheck(noteId)) {
      throw StateError('no role in the Shared Area of $noteId');
    }
    final named = _text(text);
    final lines = list.lines;
    final pos = switch (at) {
      null => lines.length,
      final at when at < 0 => throw RangeError.value(at, 'at'),
      final at => at > lines.length ? lines.length : at,
    };
    final key = _keyAt(lines, pos);
    return store.batch(
      () => writeLine(
        store,
        noteId,
        list.making,
        text: named,
        order: key,
        indent: indent && pos > 0,
      ),
    );
  }

  /// Renames the Line [lineId].
  void rename(String lineId, String text) {
    final (_, line) = _requireEditable(lineId);
    final named = _text(text);
    if (named == line.text) return;
    store.append(lineId, ChecklistKeys.text, StoredText.plain(named).encode());
  }

  /// Deletes the Line [lineId]; its group, as far as this Device may
  /// change it, comes up to the top. It can be restored.
  void delete(String lineId) {
    final (list, _) = _requireEditable(lineId);
    final at = store.clock().toUtc().toIso8601String();
    store.batch(() {
      store.append(lineId, ChecklistKeys.trashed, at);
      for (final line in list.groupOf(lineId)) {
        if (mayEdit(line.id)) store.append(line.id, ChecklistKeys.indent, null);
      }
    });
  }

  /// Brings the deleted Line [lineId] back, where it was.
  void restore(String lineId) {
    final fields = store.currentFields(lineId);
    final note = fields[ChecklistKeys.note];
    if (fields[ChecklistKeys.type] != ChecklistKeys.typeLine ||
        note == null ||
        fields[ChecklistKeys.trashed] == null ||
        fields[ChecklistKeys.made] != _making(note)) {
      throw ArgumentError.value(lineId, 'lineId', 'not a deleted Line');
    }
    if (!mayEdit(lineId)) throw StateError('may not change $lineId');
    store.append(lineId, ChecklistKeys.trashed, null);
  }

  /// Moves the Line [lineId], with its group, to just before the Line
  /// [before], or to the end. A single Line moved before a Line of a
  /// group joins that group; one placed before a Line at the top comes
  /// to the top. A Line with a group never goes into another: placed in
  /// one, it goes after it.
  void move(String lineId, {String? before}) {
    final (list, line) = _requireEditable(lineId);
    final block = [line, ...list.groupOf(lineId)];
    final moving = {for (final l in block) l.id};
    if (before != null && moving.contains(before)) return;
    final rest = [
      for (final l in list.lines)
        if (!moving.contains(l.id)) l,
    ];
    var pos = rest.length;
    if (before != null) {
      pos = rest.indexWhere((l) => l.id == before);
      if (pos < 0) {
        throw ArgumentError.value(before, 'before', 'not a Line of the list');
      }
    }
    final grouped = block.length > 1;
    if (grouped) {
      while (pos < rest.length && rest[pos].inGroup) {
        pos++;
      }
    }
    final indent = !grouped && pos < rest.length && rest[pos].inGroup;
    var (a, b) = _neighbours(rest, pos);
    store.batch(() {
      for (final l in block) {
        final key = LineOrder.place(a, b);
        store.append(l.id, ChecklistKeys.order, key);
        a = key;
      }
      if (!grouped && indent != _indented(lineId)) {
        store.append(lineId, ChecklistKeys.indent, indent ? 'true' : null);
      }
    });
  }

  /// Puts the Line [lineId] under the Line above it, or with [under]
  /// false back at the top, where it is; the Lines under it then follow
  /// it. A Line at the very top has nothing to go under.
  void indent(String lineId, {bool under = true}) {
    final (list, _) = _requireEditable(lineId);
    if (under && list.lines.first.id == lineId) {
      throw ArgumentError.value(lineId, 'lineId', 'no Line above');
    }
    if (under == _indented(lineId)) return;
    store.append(lineId, ChecklistKeys.indent, under ? 'true' : null);
  }

  /// Checks the Line [lineId]: a Line with a group checks its whole
  /// group, and the last Line of a group checked checks its parent.
  /// Every Line that changes gets an entry of its own.
  void check(String lineId) => _setChecked(lineId, true);

  /// Unchecks the Line [lineId]: a Line with a group unchecks its whole
  /// group, and one of a group unchecks its parent.
  void uncheck(String lineId) => _setChecked(lineId, false);

  /// Unchecks every Line of the Checklist of [noteId], to use it again.
  void uncheckAll(String noteId) {
    final (_, list) = _requireList(noteId);
    if (!mayCheck(noteId)) {
      throw StateError('no role in the Shared Area of $noteId');
    }
    store.batch(() {
      for (final line in list.lines) {
        if (_checkedOwn(line.id)) {
          store.append(line.id, ChecklistKeys.checked, null);
        }
      }
    });
  }

  void _setChecked(String lineId, bool checked) {
    final (list, line) = _requireLine(lineId);
    if (!mayCheck(list.noteId)) {
      throw StateError('no role in the Shared Area of ${list.noteId}');
    }
    final group = list.groupOf(lineId);
    final changed = <Line>[line, ...group];
    if (line.parent case final parent?) {
      final siblings = list.groupOf(parent);
      final rest = siblings.where((l) => l.id != lineId);
      if (list.line(parent) case final above?
          when !checked || rest.every((l) => l.checked)) {
        changed.add(above);
      }
    }
    final at = store.clock().toUtc().toIso8601String();
    store.batch(() {
      for (final l in changed) {
        if (_checkedOwn(l.id) == checked) continue;
        store.append(l.id, ChecklistKeys.checked, checked ? at : null);
      }
    });
  }

  /// Whether the Line [id] itself was checked last.
  bool _checkedOwn(String id) =>
      store.current(id, ChecklistKeys.checked) != null;

  bool _indented(String id) =>
      store.current(id, ChecklistKeys.indent) == 'true';

  /// The making of the Checklist the Note [noteId] is now, or null when
  /// it is no Checklist: no Note, another list, or a Tally.
  String? _making(String noteId) {
    final fields = store.currentFields(noteId);
    if (fields[NoteKeys.type] != NoteKeys.typeNote) return null;
    final kind = parseListKind(fields[ChecklistKeys.kind]);
    if (kind == null || kind.list != ChecklistKeys.checklist) return null;
    if (Tallies(store).isTally(noteId)) return null;
    return kind.making;
  }

  /// [raw] in the order they show, each with its parent and what shows
  /// checked.
  static List<Line> _shown(String noteId, List<_Raw> raw) {
    raw.sort((a, b) {
      final av = LineOrder.isValid(a.order);
      final bv = LineOrder.isValid(b.order);
      if (av != bv) return av ? -1 : 1;
      final byOrder = (a.order ?? '').compareTo(b.order ?? '');
      return byOrder != 0 ? byOrder : a.id.compareTo(b.id);
    });
    final parents = <String?>[];
    String? top;
    for (final r in raw) {
      if (r.indent && top != null) {
        parents.add(top);
      } else {
        top = r.id;
        parents.add(null);
      }
    }
    final groupChecked = <String, bool>{};
    for (final (i, r) in raw.indexed) {
      if (parents[i] case final parent?) {
        groupChecked[parent] = (groupChecked[parent] ?? true) && r.checked;
      }
    }
    return [
      for (final (i, r) in raw.indexed)
        Line(
          id: r.id,
          noteId: noteId,
          text: r.text,
          checked: groupChecked[r.id] ?? r.checked,
          checkedAt: r.checkedAt,
          parent: parents[i],
          order: r.order,
        ),
    ];
  }

  /// The valid keys around the place [pos] of [lines]: the last one
  /// before it, and the first one after it past that one; null for none.
  static (String?, String?) _neighbours(List<Line> lines, int pos) {
    String? a;
    for (var i = pos - 1; i >= 0; i--) {
      if (LineOrder.isValid(lines[i].order)) {
        a = lines[i].order;
        break;
      }
    }
    String? b;
    for (var i = pos; i < lines.length; i++) {
      final key = lines[i].order;
      if (key != null &&
          LineOrder.isValid(key) &&
          (a == null || key.compareTo(a) > 0)) {
        b = key;
        break;
      }
    }
    return (a, b);
  }

  static String _keyAt(List<Line> lines, int pos) {
    final (a, b) = _neighbours(lines, pos);
    return LineOrder.place(a, b);
  }

  /// The Note [noteId], not in the trash, and its Checklist.
  (Note, Checklist) _requireList(String noteId) {
    final note = Notes(store).byId(noteId);
    final list = byNote(noteId);
    if (note == null || note.isTrashed || list == null) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Checklist');
    }
    return (note, list);
  }

  /// The Checklist the shown Line [lineId] lies in, its Note not in the
  /// trash, and the Line.
  (Checklist, Line) _requireLine(String lineId) {
    final note = store.current(lineId, ChecklistKeys.note);
    final list = note == null ? null : byNote(note);
    final line = list?.line(lineId);
    final trashed =
        note == null || (Notes(store).byId(note)?.isTrashed ?? true);
    if (list == null || line == null || trashed) {
      throw ArgumentError.value(lineId, 'lineId', 'not a Line of a Checklist');
    }
    return (list, line);
  }

  (Checklist, Line) _requireEditable(String lineId) {
    final found = _requireLine(lineId);
    if (!mayEdit(lineId)) throw StateError('may not change $lineId');
    return found;
  }

  static String _text(String text) {
    final trimmed = text.replaceAll(RegExp('[\r\n]+'), ' ').trim();
    if (trimmed.isEmpty) {
      throw ArgumentError.value(text, 'text', 'must not be empty');
    }
    return trimmed;
  }
}

/// A Line as [Checklists] reads it from its fields.
class _Raw {
  final String id;
  final String text;
  final String? order;
  final bool indent;
  final bool checked;
  final DateTime? checkedAt;

  _Raw(this.id, Map<String, String?> fields)
    : text = StoredText.read(fields[ChecklistKeys.text]).text,
      order = fields[ChecklistKeys.order],
      indent = fields[ChecklistKeys.indent] == 'true',
      checked = fields[ChecklistKeys.checked] != null,
      checkedAt = DateTime.tryParse(fields[ChecklistKeys.checked] ?? '')
          ?.toUtc();
}

/// One line of a text as it becomes a Line.
typedef ParsedLine = ({String text, bool indent, bool checked});

/// The lines of [text]: split at every line break, `\r\n` and a lone
/// `\r` among them.
List<String> splitLines(String text) =>
    text.replaceAll('\r\n', '\n').replaceAll('\r', '\n').split('\n');

/// The Line the text line [row] becomes, or null for a blank one: under
/// the line above when it starts with a space or a tab, checked when its
/// text starts with "✓ ".
ParsedLine? parseLine(String row) {
  var text = row.trim();
  if (text.isEmpty) return null;
  final indent = row.startsWith(' ') || row.startsWith('\t');
  var checked = false;
  if (text.startsWith(ChecklistKeys.checkMark)) {
    final rest = text.substring(ChecklistKeys.checkMark.length).trim();
    if (rest.isNotEmpty) {
      text = rest;
      checked = true;
    }
  }
  return (text: text, indent: indent, checked: checked);
}

/// Writes a new Line of the Note [noteId] in its making [making], inside
/// a batch; returns its id.
String writeLine(
  Store store,
  String noteId,
  String making, {
  required String text,
  required String order,
  bool indent = false,
  bool checked = false,
  String? received,
}) {
  final id = '${ChecklistKeys.linePrefix}${Store.newId()}';
  final now = store.clock().toUtc();
  store.append(id, ChecklistKeys.type, ChecklistKeys.typeLine, date: now);
  store.append(id, ChecklistKeys.note, noteId, date: now);
  store.append(id, ChecklistKeys.made, making, date: now);
  store.append(
    id,
    ChecklistKeys.text,
    StoredText.plain(text).encode(),
    date: now,
  );
  store.append(id, ChecklistKeys.order, order, date: now);
  if (indent) store.append(id, ChecklistKeys.indent, 'true', date: now);
  if (checked) {
    store.append(id, ChecklistKeys.checked, now.toIso8601String(), date: now);
  }
  if (received != null) {
    store.append(id, SentKeys.received, received, date: now);
  }
  return id;
}
