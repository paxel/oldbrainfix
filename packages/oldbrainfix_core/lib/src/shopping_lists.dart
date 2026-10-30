import 'checklists.dart';
import 'entry.dart';
import 'notes.dart';
import 'schedule.dart';
import 'shared_areas.dart';
import 'store.dart';
import 'tallies.dart';
import 'text_format.dart';

/// Field keys of a Shopping List (ADR 0030). The kind lies on the Note
/// as a Checklist's does, its Lines are a Checklist's Lines with no
/// indent, and each Line holds its reappear time.
abstract final class ShoppingKeys {
  /// The list of a Shopping List in the Note's `kind`.
  static const shopping = 'shopping';

  /// The value of the Note's `kind` for the making [making] of a
  /// Shopping List.
  static String kindOf(String making) => '$shopping:$making';

  /// On a Line: how long it stays struck once checked, a [Reappear]'s
  /// code.
  static const reappear = 'reappear';
}

/// How long a checked Line of a Shopping List stays struck through
/// before it comes back (CONTEXT.md).
enum Reappear {
  /// Gone once checked.
  once('once', 0),
  week1('1w', 1),
  week2('2w', 2),
  week3('3w', 3),
  week4('4w', 4);

  const Reappear(this.code, this.weeks);

  /// How it is stored on the Line.
  final String code;

  /// The weeks it stays struck; none for [once].
  final int weeks;

  /// What a new Line gets.
  static const standard = Reappear.week1;

  /// The reappear time [code] names; [standard] for none, or a code not
  /// known here.
  static Reappear read(String? code) {
    for (final r in values) {
      if (r.code == code) return r;
    }
    return standard;
  }
}

/// How the Agenda of a Device shows Shopping Lists (#123), a setting of
/// that Device.
enum ShoppingAgenda {
  /// Each Line that came back today on its own row, checkable there.
  lines,

  /// One row per Shopping List on a day when Lines came back.
  back,

  /// One row per Shopping List on every day it has open Lines.
  open,

  /// Not at all.
  off;

  /// What a Device shows when nothing was chosen.
  static const standard = back;

  /// The way [code] names; [standard] for none, or a code not known here.
  static ShoppingAgenda read(String? code) =>
      values.asNameMap()[code] ?? standard;
}

/// A Shopping List on today's Agenda with the Lines its row stands for:
/// those that came back today, or for [ShoppingAgenda.open] those open.
typedef ShoppingAgendaList = ({ShoppingList list, List<ShoppingLine> lines});

/// One line of a Shopping List (CONTEXT.md): a Line with its reappear
/// time, struck through for a while once checked.
class ShoppingLine {
  /// Entity id, `line:<id>`.
  final String id;
  final String noteId;
  final String text;
  final Reappear reappear;

  /// Whether it shows struck through: checked, and its day to come back
  /// not here yet.
  final bool struck;

  /// When it was checked last, as its own entry says; null when it was
  /// never checked or was brought back since.
  final DateTime? checkedAt;

  /// The day it comes back, at its start, while it is struck; null
  /// while it is open.
  final Day? backOn;

  /// The day it came back by itself, at its start, while it is open
  /// since then: on that day it "came back today". Null while it is
  /// struck, and for a Line never checked or brought back by hand or by
  /// adding its name.
  final Day? cameBackOn;

  /// Its place as stored ([LineOrder]).
  final String? order;

  const ShoppingLine({
    required this.id,
    required this.noteId,
    required this.text,
    required this.reappear,
    required this.struck,
    required this.checkedAt,
    required this.backOn,
    required this.cameBackOn,
    required this.order,
  });
}

/// A Note whose Lines come back a while after they are checked
/// (CONTEXT.md).
class ShoppingList {
  final String noteId;

  /// The id of its making, as the Note's `kind` names it.
  final String making;

  /// Every Line that shows: the open ones first, the one that came last
  /// at the top, then the struck ones, the one checked last at the end.
  final List<ShoppingLine> lines;

  /// The Lines checked Once: gone, but a name added again brings them
  /// back.
  final List<ShoppingLine> _gone;

  const ShoppingList(
    this.noteId,
    this.making,
    this.lines, [
    this._gone = const [],
  ]);

  /// The Line [id] that shows, or null.
  ShoppingLine? line(String id) {
    for (final line in lines) {
      if (line.id == id) return line;
    }
    return null;
  }

  List<ShoppingLine> get open => [
    for (final l in lines)
      if (!l.struck) l,
  ];

  List<ShoppingLine> get struck => [
    for (final l in lines)
      if (l.struck) l,
  ];

  /// The Lines that came back by themselves at the start of [day].
  List<ShoppingLine> cameBack(Day day) => [
    for (final l in lines)
      if (l.cameBackOn == day) l,
  ];

  int get count => lines.length;
  int get openCount => lines.where((l) => !l.struck).length;
  int get struckCount => lines.where((l) => l.struck).length;
}

/// Making Notes Shopping Lists and back, and adding, checking and
/// changing their Lines (ADR 0030). What is struck and what came back
/// follows from the moment each Line was checked and the wall clock, so
/// nothing is written when a Line comes back.
class ShoppingLists {
  final Store store;

  ShoppingLists(this.store);

  /// The Shopping List of the Note [noteId] as it shows now, with days
  /// as [wallClock] shows them, or null when it is none: a plain Note, a
  /// Checklist, a Tally (the Tally wins), or no Note.
  ShoppingList? byNote(String noteId, WallClock wallClock) =>
      ofNotes([noteId], wallClock)[noteId];

  /// Whether the Note [noteId] is a Shopping List.
  bool isShoppingList(String noteId) => making(noteId) != null;

  /// The Shopping Lists of the Notes [noteIds] that are one, read in one
  /// pass over the Lines: for a list that shows many Notes in full.
  Map<String, ShoppingList> ofNotes(
    Iterable<String> noteIds,
    WallClock wallClock,
  ) {
    final makings = <String, String>{};
    for (final id in noteIds) {
      if (making(id) case final making?) makings[id] = making;
    }
    if (makings.isEmpty) return const {};
    final today = Day.today(store.clock(), wallClock);
    final read = <String, List<_Read>>{};
    for (final id in store.entitiesOfType(ChecklistKeys.typeLine)) {
      final latest = <String, Entry>{};
      for (final e in store.history(id)) {
        final held = latest[e.field];
        if (held == null || Store.latestWins(e, held) > 0) latest[e.field] = e;
      }
      final note = latest[ChecklistKeys.note]?.value;
      if (note == null || latest[ChecklistKeys.trashed]?.value != null) {
        continue;
      }
      final making = makings[note];
      if (making == null || latest[ChecklistKeys.made]?.value != making) {
        continue;
      }
      (read[note] ??= []).add(_Read(id, note, latest, today, wallClock));
    }
    return {
      for (final MapEntry(key: note, value: making) in makings.entries)
        note: _list(note, making, read[note] ?? []),
    };
  }

  /// The Shopping Lists on today's Agenda by [wallClock] when it shows
  /// them the way [way]: each list not in the trash nor merged with the
  /// Lines that came back today, or for [ShoppingAgenda.open] those
  /// open, where there are any; oldest capture first. For
  /// [ShoppingAgenda.lines] each of those Lines is a row of its own.
  List<ShoppingAgendaList> agenda(ShoppingAgenda way, WallClock wallClock) {
    if (way == ShoppingAgenda.off) return const [];
    final notes = Notes(store);
    final shown = <Note>[];
    for (final id in store.entitiesOfType(NoteKeys.typeNote)) {
      if (making(id) == null) continue;
      if (notes.byId(id) case final note?
          when !note.isTrashed && !note.isMerged) {
        shown.add(note);
      }
    }
    shown.sort((a, b) {
      final byCapture = a.captured.compareTo(b.captured);
      return byCapture != 0 ? byCapture : a.id.compareTo(b.id);
    });
    final lists = ofNotes([for (final n in shown) n.id], wallClock);
    final today = Day.today(store.clock(), wallClock);
    return [
      for (final note in shown)
        if (lists[note.id] case final list?)
          if (way == ShoppingAgenda.open ? list.open : list.cameBack(today)
              case final lines when lines.isNotEmpty)
            (list: list, lines: lines),
    ];
  }

  /// The Notes that are a Shopping List this Device may add Lines to:
  /// each one not in the trash nor merged where it holds a role, oldest
  /// capture first. What a capture can be sorted into (#124).
  List<Note> addable() {
    final notes = Notes(store);
    final found = <Note>[];
    for (final id in store.entitiesOfType(NoteKeys.typeNote)) {
      if (making(id) == null || !mayCheck(id)) continue;
      if (notes.byId(id) case final note?
          when !note.isTrashed && !note.isMerged) {
        found.add(note);
      }
    }
    found.sort((a, b) {
      final byCapture = a.captured.compareTo(b.captured);
      return byCapture != 0 ? byCapture : a.id.compareTo(b.id);
    });
    return found;
  }

  /// The making of the Shopping List the Note [noteId] is now, or null
  /// when it is none: no Note, another list, or a Tally.
  String? making(String noteId) {
    final fields = store.currentFields(noteId);
    if (fields[NoteKeys.type] != NoteKeys.typeNote) return null;
    final kind = parseListKind(fields[ChecklistKeys.kind]);
    if (kind == null || kind.list != ShoppingKeys.shopping) return null;
    if (Tallies(store).isTally(noteId)) return null;
    return kind.making;
  }

  /// Whether the Line of [fields] is gone: checked, and Once.
  static bool isGone(Map<String, String?> fields) =>
      fields[ChecklistKeys.checked] != null &&
      Reappear.read(fields[ShoppingKeys.reappear]) == Reappear.once;

  /// What a name is compared as when it is added: trimmed, in lower
  /// case.
  static String nameKey(String text) => text.trim().toLowerCase();

  /// Whether this Device may make the Note [noteId] a Shopping List: a
  /// Note in the lists, of a text known here, neither a Tally nor a list
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

  /// Makes the Note [noteId] a Shopping List: its first line stays its
  /// text, every other line not blank becomes a Line, indented or not,
  /// struck when it starts with "✓ ". Every Line comes in at one moment,
  /// so they show in the order of the text. Takes it out of Unsorted, as
  /// a Checklist does.
  void make(String noteId) {
    final note = Notes(store).byId(noteId);
    if (note == null || !mayMake(noteId)) {
      throw StateError('$noteId may not be made a Shopping List here');
    }
    final rows = splitLines(note.text);
    final title = rows.first.trim();
    final parsed = [for (final row in rows.skip(1)) ?parseLine(row)];
    final making = Store.newId();
    store.batch(() {
      store.append(noteId, ChecklistKeys.kind, ShoppingKeys.kindOf(making));
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
      final now = store.clock().toUtc();
      var key = '';
      for (final line in parsed) {
        key = LineOrder.after(key);
        _writeLine(
          noteId,
          making,
          line.text,
          key,
          checked: line.checked,
          date: now,
        );
      }
    });
  }

  /// Turns the Shopping List of [noteId] back into a plain Note: every
  /// Line that shows goes into its text, one per line, under its text,
  /// a struck one marked "✓ ". The Lines and their checks stay in the
  /// log.
  void toPlain(String noteId, WallClock wallClock) {
    final (note, list) = _requireList(noteId, wallClock);
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

  /// The text the Shopping List [list] of a Note titled [title] goes back
  /// to.
  static String plainText(String title, ShoppingList list) => [
    if (title.isNotEmpty) title,
    for (final line in list.lines)
      '${line.struck ? ChecklistKeys.checkMark : ''}${line.text}',
  ].join('\n');

  /// Whether this Device may add Lines to the Shopping List of [noteId]
  /// and check them: anywhere it holds a role.
  bool mayCheck(String noteId) => store.mayWriteIn(noteId);

  /// Whether this Device may rename and delete the Line [lineId] and set
  /// its reappear time: as for a Checklist's Line.
  bool mayEdit(String lineId) => Checklists(store).mayEdit(lineId);

  /// Adds [text] to the Shopping List of [noteId], at the top. A name it
  /// has already, in another case or with spaces around it, adds
  /// nothing: a struck or gone Line of that name comes back now, at the
  /// top, an open one stays. Anyone with a role in its Area may. Returns
  /// the id of the Line added, brought back or found.
  String add(String noteId, String text, WallClock wallClock) {
    final (_, list) = _requireList(noteId, wallClock);
    if (!mayCheck(noteId)) {
      throw StateError('no role in the Shared Area of $noteId');
    }
    final named = lineText(text);
    final wanted = nameKey(named);
    for (final line in list.lines) {
      if (nameKey(line.text) != wanted) continue;
      if (line.struck) store.append(line.id, ChecklistKeys.checked, null);
      return line.id;
    }
    for (final line in list._gone) {
      if (nameKey(line.text) != wanted) continue;
      store.append(line.id, ChecklistKeys.checked, null);
      return line.id;
    }
    String? last;
    for (final line in [...list.lines, ...list._gone]) {
      final key = line.order;
      if (key != null &&
          LineOrder.isValid(key) &&
          (last == null || key.compareTo(last) > 0)) {
        last = key;
      }
    }
    final key = LineOrder.place(last, null);
    return store.batch(() => _writeLine(noteId, list.making, named, key));
  }

  /// Adds every line of [text] that is not blank to the Shopping List of
  /// [noteId] as [add] does, all in one batch, the first line ending at
  /// the top: what a capture sorted into the list becomes (#124). Returns
  /// the ids of the Lines added, brought back or found, in the order of
  /// the lines; none for a blank [text].
  List<String> addLines(String noteId, String text, WallClock wallClock) {
    final rows = [
      for (final row in splitLines(text))
        if (row.trim().isNotEmpty) row,
    ];
    if (rows.isEmpty) return const [];
    return store.batch(() {
      final ids = [
        for (final row in rows.reversed) add(noteId, row, wallClock),
      ];
      return ids.reversed.toList();
    });
  }

  /// Renames the Line [lineId].
  void rename(String lineId, String text) {
    final fields = _requireEditable(lineId);
    final named = lineText(text);
    if (named == StoredText.read(fields[ChecklistKeys.text]).text) return;
    store.append(lineId, ChecklistKeys.text, StoredText.plain(named).encode());
  }

  /// Sets how long the Line [lineId] stays struck once checked.
  void setReappear(String lineId, Reappear reappear) {
    final fields = _requireEditable(lineId);
    if (Reappear.read(fields[ShoppingKeys.reappear]) == reappear) return;
    store.append(lineId, ShoppingKeys.reappear, reappear.code);
  }

  /// Deletes the Line [lineId]. It can be restored.
  void delete(String lineId) {
    _requireEditable(lineId);
    store.append(
      lineId,
      ChecklistKeys.trashed,
      store.clock().toUtc().toIso8601String(),
    );
  }

  /// Brings the deleted Line [lineId] back.
  void restore(String lineId) {
    final fields = store.currentFields(lineId);
    final note = fields[ChecklistKeys.note];
    if (fields[ChecklistKeys.type] != ChecklistKeys.typeLine ||
        note == null ||
        fields[ChecklistKeys.trashed] == null ||
        fields[ChecklistKeys.made] != making(note)) {
      throw ArgumentError.value(lineId, 'lineId', 'not a deleted Line');
    }
    if (!mayEdit(lineId)) throw StateError('may not change $lineId');
    store.append(lineId, ChecklistKeys.trashed, null);
  }

  /// Checks the Line [lineId]: it is struck through and goes to the end
  /// until the start of the same weekday as many weeks on as its
  /// reappear time says, by [wallClock]; a Line Once is gone. A struck
  /// one stays as it is.
  void check(String lineId, WallClock wallClock) {
    final (list, line) = _requireShown(lineId, wallClock);
    if (!mayCheck(list.noteId)) {
      throw StateError('no role in the Shared Area of ${list.noteId}');
    }
    if (line.struck) return;
    store.append(
      lineId,
      ChecklistKeys.checked,
      store.clock().toUtc().toIso8601String(),
    );
  }

  /// Brings the struck Line [lineId] back now, at the top. An open one
  /// stays as it is.
  void uncheck(String lineId, WallClock wallClock) {
    final (list, line) = _requireShown(lineId, wallClock);
    if (!mayCheck(list.noteId)) {
      throw StateError('no role in the Shared Area of ${list.noteId}');
    }
    if (!line.struck) return;
    store.append(lineId, ChecklistKeys.checked, null);
  }

  /// [lines] as they show, the gone ones kept apart.
  static ShoppingList _list(String noteId, String making, List<_Read> lines) {
    int byOrder(_Read a, _Read b) {
      final av = LineOrder.isValid(a.line.order);
      final bv = LineOrder.isValid(b.line.order);
      if (av != bv) return av ? -1 : 1;
      final byKey = (a.line.order ?? '').compareTo(b.line.order ?? '');
      return byKey != 0 ? byKey : a.line.id.compareTo(b.line.id);
    }

    final open = [
      for (final r in lines)
        if (!r.gone && !r.line.struck) r,
    ];
    final struck = [
      for (final r in lines)
        if (!r.gone && r.line.struck) r,
    ];
    final gone = [
      for (final r in lines)
        if (r.gone) r,
    ];
    open.sort((a, b) {
      final byAt = b.at.compareTo(a.at);
      return byAt != 0 ? byAt : byOrder(a, b);
    });
    int byCheck(_Read a, _Read b) {
      final byAt = a.at.compareTo(b.at);
      return byAt != 0 ? byAt : byOrder(a, b);
    }

    struck.sort(byCheck);
    gone.sort(byCheck);
    return ShoppingList(
      noteId,
      making,
      [
        for (final r in [...open, ...struck]) r.line,
      ],
      [for (final r in gone) r.line],
    );
  }

  /// Writes a new Line, inside a batch, with the reappear time new Lines
  /// get; returns its id.
  String _writeLine(
    String noteId,
    String making,
    String text,
    String order, {
    bool checked = false,
    DateTime? date,
  }) {
    final now = (date ?? store.clock()).toUtc();
    final id = writeLine(
      store,
      noteId,
      making,
      text: text,
      order: order,
      checked: checked,
      date: now,
    );
    store.append(id, ShoppingKeys.reappear, Reappear.standard.code, date: now);
    return id;
  }

  /// The Note [noteId], not in the trash, and its Shopping List.
  (Note, ShoppingList) _requireList(String noteId, WallClock wallClock) {
    final note = Notes(store).byId(noteId);
    final list = byNote(noteId, wallClock);
    if (note == null || note.isTrashed || list == null) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Shopping List');
    }
    return (note, list);
  }

  /// The fields of the Line [lineId] that shows in a Shopping List whose
  /// Note is not in the trash.
  Map<String, String?> _requireLine(String lineId) {
    final fields = store.currentFields(lineId);
    final note = fields[ChecklistKeys.note];
    final making = note == null ? null : this.making(note);
    final trashed =
        note == null || (Notes(store).byId(note)?.isTrashed ?? true);
    if (fields[ChecklistKeys.type] != ChecklistKeys.typeLine ||
        making == null ||
        fields[ChecklistKeys.made] != making ||
        fields[ChecklistKeys.trashed] != null ||
        trashed ||
        isGone(fields)) {
      throw ArgumentError.value(lineId, 'lineId', 'not a Shopping List Line');
    }
    return fields;
  }

  Map<String, String?> _requireEditable(String lineId) {
    final fields = _requireLine(lineId);
    if (!mayEdit(lineId)) throw StateError('may not change $lineId');
    return fields;
  }

  /// The Shopping List the shown Line [lineId] lies in, and the Line.
  (ShoppingList, ShoppingLine) _requireShown(
    String lineId,
    WallClock wallClock,
  ) {
    final note = _requireLine(lineId)[ChecklistKeys.note];
    final list = note == null ? null : byNote(note, wallClock);
    final line = list?.line(lineId);
    if (list == null || line == null) {
      throw ArgumentError.value(lineId, 'lineId', 'not a Shopping List Line');
    }
    return (list, line);
  }
}

/// A Line of a Shopping List as read from its latest entries, with the
/// moment it sorts by: a struck or gone one by its check, an open one by
/// when it came to the top, on the wall clock.
class _Read {
  final ShoppingLine line;
  final bool gone;
  final DateTime at;

  _Read._(this.line, this.gone, this.at);

  factory _Read(
    String id,
    String noteId,
    Map<String, Entry> latest,
    Day today,
    WallClock wallClock,
  ) {
    final reappear = Reappear.read(latest[ShoppingKeys.reappear]?.value);
    final check = latest[ChecklistKeys.checked];
    final value = check?.value;
    Day? back;
    Day? came;
    var struck = false;
    var gone = false;
    final DateTime at;
    if (check != null && value != null) {
      gone = reappear == Reappear.once;
      final day = Day.of(wallClock(check.date)).addDays(7 * reappear.weeks);
      struck = !gone && today < day;
      if (struck) back = day;
      if (!gone && !struck) came = day;
      at = gone || struck
          ? check.date
          : DateTime.utc(day.year, day.month, day.day);
    } else {
      final came = check?.date ?? latest[ChecklistKeys.type]?.date;
      at = came == null ? DateTime.utc(0) : _wall(wallClock(came));
    }
    return _Read._(
      ShoppingLine(
        id: id,
        noteId: noteId,
        text: StoredText.read(latest[ChecklistKeys.text]?.value).text,
        reappear: reappear,
        struck: struck,
        checkedAt: DateTime.tryParse(value ?? '')?.toUtc(),
        backOn: back,
        cameBackOn: came,
        order: latest[ChecklistKeys.order]?.value,
      ),
      gone,
      at,
    );
  }

  /// The wall-clock time [wall] shows, its zone ignored, as a value that
  /// compares with a day's start.
  static DateTime _wall(DateTime wall) => DateTime.utc(
    wall.year,
    wall.month,
    wall.day,
    wall.hour,
    wall.minute,
    wall.second,
    wall.millisecond,
    wall.microsecond,
  );
}
