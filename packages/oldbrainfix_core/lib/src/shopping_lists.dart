import 'checklists.dart';
import 'device_settings.dart';
import 'entry.dart';
import 'notes.dart';
import 'profiles.dart';
import 'schedule.dart';
import 'shared_areas.dart';
import 'sessions.dart';
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

  /// On the user's own `agenda:<note id>` entity
  /// ([ChecklistKeys.agendaPrefix]), beside a Checklist's field: how the
  /// Shopping List shows on their Agenda (#186), a [ShoppingOnAgenda]'s
  /// code.
  static const agenda = 'shopping';

  /// On a voice capture merged into a Shopping List (#311): the Note id
  /// of the list it still owes the Lines of its Transcripts; cleared
  /// once they are added, so they are added once.
  static const owesLines = 'owesLines';
}

/// The unit of a [Reappear] time (#195), the letter its code ends in.
enum ReappearUnit {
  day('d'),
  week('w'),
  month('m'),
  year('y');

  const ReappearUnit(this.letter);

  /// How its code ends.
  final String letter;
}

/// How long a checked Line of a Shopping List stays struck through
/// before it comes back (CONTEXT.md): none, so it stays struck until a
/// time is set; Once, gone once checked; or a number of days, weeks,
/// months or years (#195).
final class Reappear {
  /// The number of [unit]s; 0 for none and Once.
  final int count;

  /// Null for none and Once.
  final ReappearUnit? unit;

  final bool _once;

  const Reappear._(this.count, this.unit, this._once);

  /// [count] [unit]s, [count] from 1 to [maxCount].
  factory Reappear.after(int count, ReappearUnit unit) {
    RangeError.checkValueInInterval(count, 1, maxCount, 'count');
    return Reappear._(count, unit, false);
  }

  /// None: a checked Line stays struck until a time is set. What a new
  /// Line gets.
  static const unset = Reappear._(0, null, false);

  /// Gone once checked.
  static const once = Reappear._(0, null, true);

  /// What a Line without a reappear time, or with one not known here,
  /// reads as: one week, as every version before #195 read it.
  static const fallback = Reappear._(1, ReappearUnit.week, false);

  /// The largest number a time holds, so its code stays short.
  static const maxCount = 99;

  bool get isOnce => _once;

  /// Whether it is none.
  bool get isUnset => unit == null && !_once;

  /// How it is stored on the Line: `none`, `once`, or the number and the
  /// unit's letter, such as `4w` or `3m`.
  String get code => switch (unit) {
    final unit? => '$count${unit.letter}',
    null => _once ? 'once' : 'none',
  };

  /// A number from 1 to 99 without a leading zero, then a unit's letter.
  static final _timed = RegExp(r'^([1-9][0-9]?)([dwmy])$');

  /// The reappear time [code] names; [fallback] for none, or a code not
  /// known here.
  static Reappear read(String? code) {
    if (code == 'none') return unset;
    if (code == 'once') return once;
    final match = _timed.firstMatch(code ?? '');
    if (match == null) return fallback;
    final count = int.tryParse(match[1] ?? '');
    final letter = match[2];
    if (count == null || letter == null) return fallback;
    final unit = ReappearUnit.values.firstWhere((u) => u.letter == letter);
    return Reappear._(count, unit, false);
  }

  /// The day a Line checked on [checked] comes back on: as many days or
  /// weeks on, or the same day of the month as many months or years on,
  /// the month's last day where it has no such day. Null for none and
  /// Once, which never come back by themselves.
  Day? backFrom(Day checked) => switch (unit) {
    null => null,
    ReappearUnit.day => checked.addDays(count),
    ReappearUnit.week => checked.addDays(7 * count),
    ReappearUnit.month => _monthsOn(checked, count),
    ReappearUnit.year => _monthsOn(checked, 12 * count),
  };

  static Day _monthsOn(Day day, int months) {
    final first = Day(day.year, day.month + months, 1);
    final last = Day(first.year, first.month + 1, 0).day;
    return Day(first.year, first.month, day.day < last ? day.day : last);
  }

  @override
  bool operator ==(Object other) =>
      other is Reappear &&
      other.count == count &&
      other.unit == unit &&
      other._once == _once;

  @override
  int get hashCode => Object.hash(count, unit, _once);

  @override
  String toString() => 'Reappear($code)';
}

/// How the Agenda of a Device showed Shopping Lists (#123), a setting of
/// that Device until each Shopping List had its own choice (#186). It is
/// read only for the default of [ShoppingOnAgenda] on this Device.
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

/// How a Shopping List shows on today's Agenda (#186), the name its code
/// knows the shared [ListOnAgenda] by.
typedef ShoppingOnAgenda = ListOnAgenda;

/// How a Shopping List (#186) or a Checklist (#219) shows on today's
/// Agenda, each person's own choice for each list, kept as an Override is
/// (ADR 0030).
enum ListOnAgenda {
  /// Never.
  off,

  /// One row with the number of its open Lines, such as "Groceries: 4
  /// open" or "Before the trip: 3 of 7 open".
  count,

  /// Each open Line on a row of its own, checkable there.
  lines;

  /// How it is stored.
  String get code => name;

  /// The choice [code] names; null for none known here.
  static ListOnAgenda? parse(String? code) => values.asNameMap()[code];

  /// What a list nobody chose for shows as on a Device whose Agenda
  /// showed Shopping Lists the way [old] before: off for "Not at all",
  /// each open Line for "Each Line that came back today", else one row
  /// with the count.
  static ListOnAgenda fromDevice(ShoppingAgenda old) => switch (old) {
    ShoppingAgenda.off => off,
    ShoppingAgenda.lines => lines,
    ShoppingAgenda.back || ShoppingAgenda.open => count,
  };
}

/// A Shopping List on today's Agenda (#186), how it shows there and its
/// open Lines: one row with their number for [ShoppingOnAgenda.count],
/// one row each for [ShoppingOnAgenda.lines].
typedef ShoppingAgendaRow = ({
  ShoppingList list,
  ShoppingOnAgenda shows,
  List<ShoppingLine> lines,
});

/// How far off the day a struck Line comes back lies (#328), as the apps
/// show it faint beside the Line: a number of days, weeks, months or
/// years.
final class BackIn {
  final int count;
  final ReappearUnit unit;

  const BackIn(this.count, this.unit);

  /// [days] from 1 on as they read: up to 13 days as days, 1 being
  /// tomorrow; then up to 59 as weeks, under a year as months of 30 days
  /// and from 365 on as years, each the nearest whole number, half up.
  factory BackIn.days(int days) => switch (days) {
    < 14 => BackIn(days, ReappearUnit.day),
    < 60 => BackIn((days + 3) ~/ 7, ReappearUnit.week),
    < 365 => BackIn((days + 15) ~/ 30, ReappearUnit.month),
    _ => BackIn((days + 182) ~/ 365, ReappearUnit.year),
  };

  @override
  bool operator ==(Object other) =>
      other is BackIn && other.count == count && other.unit == unit;

  @override
  int get hashCode => Object.hash(count, unit);

  /// The number and the unit's letter, as a reappear time's code: `3w`.
  @override
  String toString() => '$count${unit.letter}';
}

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
  /// while it is open, and while it is struck without a reappear time.
  final Day? backOn;

  /// How far off [backOn] lies from the day it was read on (#328); null
  /// where [backOn] is.
  final BackIn? backIn;

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
    required this.backIn,
    required this.cameBackOn,
    required this.order,
  });

  /// Whether it is checked, struck or gone, since a moment on [day] by
  /// [wallClock].
  bool checkedOn(Day day, WallClock wallClock) => switch (checkedAt) {
    final at? => Day.of(wallClock(at)) == day,
    null => false,
  };
}

/// A Note whose Lines come back a while after they are checked
/// (CONTEXT.md).
class ShoppingList {
  final String noteId;

  /// The id of its making, as the Note's `kind` names it.
  final String making;

  /// Every Line that shows: the open ones first, in the order they were
  /// placed in, then the struck ones, the soonest back first (#328), the
  /// same day the one checked first first, those without a reappear time
  /// last, by their checks too.
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

  /// The Lines checked Once: gone, shown nowhere.
  List<ShoppingLine> get gone => List.unmodifiable(_gone);

  List<ShoppingLine> get struck => [
    for (final l in lines)
      if (l.struck) l,
  ];

  /// The Lines checked on [day] by [wallClock] that are still checked,
  /// struck or gone, the one checked first first: what stays struck on
  /// the Agenda for the day (#276).
  List<ShoppingLine> checkedOn(Day day, WallClock wallClock) {
    final checked = [
      for (final l in [...lines, ..._gone])
        if (l.checkedAt case final at? when l.checkedOn(day, wallClock))
          (line: l, at: at),
    ];
    checked.sort((a, b) {
      final byAt = a.at.compareTo(b.at);
      return byAt != 0 ? byAt : a.line.id.compareTo(b.line.id);
    });
    return [for (final c in checked) c.line];
  }

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
  /// pass over the Lines: for a list that shows many Notes in full. The
  /// Notes and the Lines are read in a few queries however many there
  /// are (obf-1069).
  Map<String, ShoppingList> ofNotes(
    Iterable<String> noteIds,
    WallClock wallClock,
  ) {
    final makings = makingsOf(noteIds);
    if (makings.isEmpty) return const {};
    final today = Day.today(store.clock(), wallClock);
    final read = <String, List<_Read>>{};
    final ever = store.everOfType(ChecklistKeys.typeLine);
    final histories = store.historiesOf(ever);
    for (final id in ever) {
      final latest = <String, Entry>{};
      for (final e in histories[id] ?? const <Entry>[]) {
        final held = latest[e.field];
        if (held == null || Store.latestWins(e, held) > 0) latest[e.field] = e;
      }
      // As [Store.entitiesOfType] lists them.
      if (latest[ChecklistKeys.type]?.value != ChecklistKeys.typeLine) {
        continue;
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

  /// The Shopping Lists on today's Agenda by [wallClock] (#186): each
  /// list not in the trash nor merged that has an open Line and that
  /// this person does not keep [ShoppingOnAgenda.off], with how it shows
  /// ([onAgenda]) and its open Lines; oldest capture first. A list whose
  /// Lines are all struck or gone shows whatever the choice. With
  /// [checkedToday] the Lines checked today and still checked follow the
  /// open ones, struck for the day ([ShoppingList.checkedOn]), and a list
  /// with only those shows too (#276). Reading writes nothing.
  List<ShoppingAgendaRow> agendaRows(
    WallClock wallClock, {
    bool checkedToday = false,
  }) {
    final notes = Notes(store);
    final standard = ShoppingOnAgenda.fromDevice(store.shoppingAgenda);
    final own = <String, bool>{};
    final shown = <(Note, ShoppingOnAgenda)>[];
    for (final id in store.entitiesOfType(NoteKeys.typeNote)) {
      if (making(id) == null) continue;
      final shows = _onAgenda(id, standard, own);
      if (shows == ShoppingOnAgenda.off) continue;
      if (notes.byId(id) case final note?
          when !note.isTrashed && !note.isMerged) {
        shown.add((note, shows));
      }
    }
    shown.sort((a, b) {
      final byCapture = a.$1.captured.compareTo(b.$1.captured);
      return byCapture != 0 ? byCapture : a.$1.id.compareTo(b.$1.id);
    });
    final lists = ofNotes([for (final (n, _) in shown) n.id], wallClock);
    // The clock is read only when asked for.
    final today = checkedToday ? Day.today(store.clock(), wallClock) : null;
    return [
      for (final (note, shows) in shown)
        if (lists[note.id] case final list?)
          if ([
                ...list.open,
                if (today != null) ...list.checkedOn(today, wallClock),
              ]
              case final lines when lines.isNotEmpty)
            (list: list, shows: shows, lines: lines),
    ];
  }

  /// How the Shopping List of [noteId] shows on today's Agenda for this
  /// person: as one of the user's own Devices chose last; for a list
  /// nobody chose for, as this Device showed Shopping Lists before
  /// (#186, [ShoppingOnAgenda.fromDevice]). What another person's Device
  /// wrote counts for nothing, so in a Shared Area each person decides
  /// for their own Agenda; a value no app writes reads as
  /// [ShoppingOnAgenda.count], so it hides no list.
  ShoppingOnAgenda onAgenda(String noteId) =>
      _onAgenda(noteId, ShoppingOnAgenda.fromDevice(store.shoppingAgenda), {});

  ShoppingOnAgenda _onAgenda(
    String noteId,
    ShoppingOnAgenda standard,
    Map<String, bool> own,
  ) {
    final code = _chosen(noteId, own);
    if (code == null) return standard;
    return ShoppingOnAgenda.parse(code) ?? ShoppingOnAgenda.count;
  }

  String? _chosen(String noteId, Map<String, bool> own) => store.currentByOwn(
    '${ChecklistKeys.agendaPrefix}$noteId',
    ShoppingKeys.agenda,
    own,
  );

  /// Whether "Show on the Agenda" applies to the Note [noteId]: a
  /// Shopping List not in the trash nor merged. One with a Routine or
  /// Reminder shows too, as its Lines keep their own checks.
  bool mayShowOnAgenda(String noteId) {
    final note = Notes(store).byId(noteId);
    return note != null &&
        !note.isTrashed &&
        !note.isMerged &&
        making(noteId) != null;
  }

  /// Sets how the Shopping List of [noteId] shows on the Agenda for this
  /// person alone, kept like an Override: on the user's own Devices
  /// only, whatever their role in its Area. The choice is written even
  /// when the list already shows that way by this Device's default, so
  /// every own Device shows it alike; the same choice again writes
  /// nothing.
  void setOnAgenda(String noteId, ShoppingOnAgenda shows) {
    if (making(noteId) == null) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Shopping List');
    }
    if (_chosen(noteId, {}) == shows.code) return;
    store.append(
      '${ChecklistKeys.agendaPrefix}$noteId',
      ShoppingKeys.agenda,
      shows.code,
    );
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
  String? making(String noteId) =>
      _makingOf(noteId, store.currentFields(noteId));

  /// The making of each of [noteIds] that is a Shopping List now, as
  /// [making] tells it, in their order: their fields read in one query
  /// rather than one per Note (obf-1069).
  Map<String, String> makingsOf(Iterable<String> noteIds) {
    final read = store.currentFieldsOf(noteIds);
    final makings = <String, String>{};
    for (final id in noteIds) {
      if (_makingOf(id, read[id] ?? const {}) case final making?) {
        makings[id] = making;
      }
    }
    return makings;
  }

  /// The making of the Shopping List the Note [noteId] of the current
  /// [fields] is ([making]).
  String? _makingOf(String noteId, Map<String, String?> fields) {
    if (fields[NoteKeys.type] != NoteKeys.typeNote) return null;
    final kind = parseListKind(fields[ChecklistKeys.kind]);
    if (kind == null || kind.list != ShoppingKeys.shopping) return null;
    if (Tallies(store).isTally(noteId)) return null;
    return kind.making;
  }

  /// Whether the Line of [fields] is gone: checked, and Once.
  static bool isGone(Map<String, String?> fields) =>
      fields[ChecklistKeys.checked] != null &&
      Reappear.read(fields[ShoppingKeys.reappear]).isOnce;

  /// What a name is compared as when it is added: trimmed, in lower
  /// case.
  static String nameKey(String text) => text.trim().toLowerCase();

  /// Whether this Device may make the Note [noteId] a Shopping List: a
  /// Note in the lists, of a text known here and not code, neither a
  /// Tally nor a list already, which it may change.
  bool mayMake(String noteId) {
    final note = Notes(store).byId(noteId);
    return note != null &&
        !note.isTrashed &&
        !note.isMerged &&
        note.textEditable &&
        note.textFormat != TextFormats.code &&
        parseListKind(store.current(noteId, ChecklistKeys.kind)) == null &&
        !Tallies(store).isTally(noteId) &&
        !Sessions(store).isSessionNote(noteId) &&
        store.mayChange(noteId);
  }

  /// Makes the Note [noteId] a Shopping List named by its first line, as
  /// a new list is from the name typed for it: its first line stays its
  /// text, every other line not blank becomes a Line, indented or not,
  /// struck when it starts with "✓ ", in a formatted text without the
  /// mark of an item or a quote ([parseLine]). Every Line comes in at one moment,
  /// so they show in the order of the text. Takes it out of Unsorted, as
  /// a Checklist does. The text keeps its format. Typed text is made a
  /// Shopping List by [makeFromText].
  void make(String noteId) => _make(noteId, fromText: false);

  /// Makes the typed text of the Note [noteId] a Shopping List (#309), as
  /// [make] does but with every line a Line unless the first looks like
  /// a heading ([listOfText]): only then is it the Name, else the list
  /// has none.
  void makeFromText(String noteId) => _make(noteId, fromText: true);

  void _make(String noteId, {required bool fromText}) {
    final note = Notes(store).byId(noteId);
    if (note == null || !mayMake(noteId)) {
      throw StateError('$noteId may not be made a Shopping List here');
    }
    final markdown = note.textFormat == TextFormats.markdown;
    final (name: title, lines: parsed) = fromText
        ? listOfText(note.text, markdown: markdown)
        : namedList(note.text, markdown: markdown);
    final making = Store.newId();
    store.batch(() {
      store.append(noteId, ChecklistKeys.kind, ShoppingKeys.kindOf(making));
      if (title != note.text) {
        store.append(
          noteId,
          NoteKeys.text,
          StoredText.of(note.textFormat, title).encode(),
        );
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
  /// a struck one marked "✓ ", in the format the text is in. The Lines
  /// and their checks stay in the log.
  void toPlain(String noteId, WallClock wallClock) {
    final (note, list) = _requireList(noteId, wallClock);
    if (!store.mayChange(noteId)) throw StateError('may not change $noteId');
    if (!note.textEditable) {
      throw StateError('the text of $noteId is in a format not known here');
    }
    final text = plainText(note.text, list);
    store.batch(() {
      store.append(
        noteId,
        NoteKeys.text,
        StoredText.of(note.textFormat, text).encode(),
      );
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

  /// Whether this Device may switch the list the Note [noteId] is, a
  /// Checklist to a Shopping List or the other way ([switchList]): a
  /// Note in the lists that is one of the two, which it may change.
  bool maySwitch(String noteId) {
    final note = Notes(store).byId(noteId);
    return note != null &&
        !note.isTrashed &&
        !note.isMerged &&
        (making(noteId) != null || Checklists(store).isChecklist(noteId)) &&
        store.mayChange(noteId);
  }

  /// Switches the list the Note [noteId] is (#188): a Checklist becomes a
  /// Shopping List and a Shopping List a Checklist, in a new making. The
  /// Lines that show carry over, as Lines of the new making, in the order
  /// they show, at one moment: a checked Line checked, a struck one
  /// checked, each at the top with the reappear time a new Line gets on a
  /// Shopping List. A Checklist's Lines show their own checks here, not
  /// those of an Occurrence. The text is not touched; the Lines of the
  /// old making stay in the log.
  void switchList(String noteId, WallClock wallClock) {
    if (!maySwitch(noteId)) {
      throw StateError('the list of $noteId may not be switched here');
    }
    final shopping = byNote(noteId, wallClock);
    final List<({String text, bool checked})> carried;
    if (shopping != null) {
      carried = [
        for (final l in shopping.lines) (text: l.text, checked: l.struck),
      ];
    } else {
      carried = [
        // maySwitch made it a Checklist; none would carry no Lines.
        for (final l
            in Checklists(store).byNote(noteId)?.lines ?? const <Line>[])
          (text: l.text, checked: l.checked),
      ];
    }
    final making = Store.newId();
    store.batch(() {
      store.append(
        noteId,
        ChecklistKeys.kind,
        shopping != null
            ? ChecklistKeys.kindOf(making)
            : ShoppingKeys.kindOf(making),
      );
      final now = store.clock().toUtc();
      var key = '';
      for (final line in carried) {
        key = LineOrder.after(key);
        if (shopping != null) {
          writeLine(
            store,
            noteId,
            making,
            text: line.text,
            order: key,
            checked: line.checked,
            date: now,
          );
        } else {
          _writeLine(
            noteId,
            making,
            line.text,
            key,
            checked: line.checked,
            date: now,
          );
        }
      }
    });
  }

  /// Whether this Device may add Lines to the Shopping List of [noteId]
  /// and check them: anywhere it holds a role.
  bool mayCheck(String noteId) => store.mayWriteIn(noteId);

  /// Whether this Device may rename, delete and move the Line [lineId]
  /// and set its reappear time: as for a Checklist's Line.
  bool mayEdit(String lineId) => Checklists(store).mayEdit(lineId);

  /// Adds [text] to the Shopping List of [noteId] at [at], an index in
  /// the order its open Lines show, or after every other. A name it has
  /// already, in another case or with spaces around it, adds nothing: a
  /// struck or gone Line of that name comes back now, in its place, an
  /// open one stays. Anyone with a role in its Area may. Returns the id
  /// of the Line added, brought back or found.
  String add(String noteId, String text, WallClock wallClock, {int? at}) {
    if (at != null && at < 0) throw RangeError.value(at, 'at');
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
    final key = _keyAt(list, at);
    return store.batch(() => _writeLine(noteId, list.making, named, key));
  }

  /// The key for a new Line at the place [at] of the open Lines of
  /// [list], or after every key with none or one past them: between the
  /// last valid key above it and the first valid key of any Line after
  /// that one, a struck or gone Line too.
  static String _keyAt(ShoppingList list, int? at) {
    final open = list.open;
    String? a;
    if (at == null || at >= open.length) {
      for (final line in [...list.lines, ...list._gone]) {
        final key = line.order;
        if (key != null &&
            LineOrder.isValid(key) &&
            (a == null || key.compareTo(a) > 0)) {
          a = key;
        }
      }
      return LineOrder.place(a, null);
    }
    for (var i = at - 1; i >= 0; i--) {
      if (LineOrder.isValid(open[i].order)) {
        a = open[i].order;
        break;
      }
    }
    String? b;
    for (final line in [...list.lines, ...list._gone]) {
      final key = line.order;
      if (key != null &&
          LineOrder.isValid(key) &&
          (a == null || key.compareTo(a) > 0) &&
          (b == null || key.compareTo(b) < 0)) {
        b = key;
      }
    }
    return LineOrder.place(a, b);
  }

  /// Adds every line of [text] that is not blank to the Shopping List of
  /// [noteId] as [add] does, all in one batch, in the order of the lines
  /// after every other: what a capture sorted into the list becomes
  /// (#124). Returns the ids of the Lines added, brought back or found,
  /// in the order of the lines; none for a blank [text].
  List<String> addLines(String noteId, String text, WallClock wallClock) {
    final rows = [
      for (final row in splitLines(text))
        if (row.trim().isNotEmpty) row,
    ];
    if (rows.isEmpty) return const [];
    return store.batch(
      () => [for (final row in rows) add(noteId, row, wallClock)],
    );
  }

  /// The items said in the Transcript [text] (#311), in the order said:
  /// it is split at line breaks, at `;`, `!`, `?` and `…`, at a comma or
  /// a full stop but one between two digits ("1,5", "1.5"), and at the
  /// words "und" and "and" in any case, whole words only. Whisper writes
  /// a pause as such a mark, and the app keeps no time of its pauses.
  /// Each item is its words with one space between them; empty ones are
  /// dropped, a repeated one is kept.
  static List<String> spokenItems(String text) {
    final runes = text.runes.toList();
    final items = <String>[];
    final piece = <int>[];
    bool digit(int at) =>
        at >= 0 && at < runes.length && runes[at] >= 0x30 && runes[at] <= 0x39;
    void flush() {
      final words = <String>[];
      void item() {
        if (words.isNotEmpty) items.add(words.join(' '));
        words.clear();
      }

      final word = StringBuffer();
      void endWord() {
        if (word.isEmpty) return;
        final said = word.toString();
        word.clear();
        if (_joins.contains(said.toLowerCase())) {
          item();
        } else {
          words.add(said);
        }
      }

      for (final rune in piece) {
        if (_isSpace(rune)) {
          endWord();
        } else {
          word.writeCharCode(rune);
        }
      }
      endWord();
      item();
      piece.clear();
    }

    for (final (at, rune) in runes.indexed) {
      final ends = switch (rune) {
        0x0a || 0x0d || 0x3b || 0x21 || 0x3f || 0x2026 => true,
        0x2c || 0x2e => !(digit(at - 1) && digit(at + 1)),
        _ => false,
      };
      if (ends) {
        flush();
      } else {
        piece.add(rune);
      }
    }
    flush();
    return items;
  }

  /// The words that join two items said ([spokenItems]), in lower case.
  static const _joins = {'und', 'and'};

  /// Unicode's White_Space and the byte order mark.
  static bool _isSpace(int rune) =>
      (rune >= 0x09 && rune <= 0x0d) ||
      rune == 0x20 ||
      rune == 0x85 ||
      rune == 0xa0 ||
      rune == 0x1680 ||
      (rune >= 0x2000 && rune <= 0x200a) ||
      rune == 0x2028 ||
      rune == 0x2029 ||
      rune == 0x202f ||
      rune == 0x205f ||
      rune == 0x3000 ||
      rune == 0xfeff;

  /// Adds the voice capture [noteId] to the Shopping List [listId]
  /// (#311), all in one batch: each line of its text that is not blank
  /// becomes a Line as [addLines] makes it, and the Note, with its text
  /// and recordings, goes where the list lies and is merged into it, so
  /// its recordings stay on the list as a Section, to be listened to. It
  /// owes the list the items said in its Transcripts ([spokenItems]),
  /// added at once when every recording has one, else once the last one
  /// comes ([addOwedLines]). Refused as [add] and [Notes.merge] refuse.
  /// Returns the ids of the Lines added, brought back or found now.
  List<String> addVoiceCapture(
    String listId,
    String noteId,
    WallClock wallClock,
  ) {
    final (into, _) = _requireList(listId, wallClock);
    if (!mayCheck(listId)) {
      throw StateError('no role in the Shared Area of $listId');
    }
    final notes = Notes(store);
    final note = notes.byId(noteId);
    if (note == null || note.isTrashed || note.isMerged) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Note in the lists');
    }
    return store.batch(() {
      final typed = addLines(listId, note.shownText, wallClock);
      final place = into.place;
      if (note.place != place) {
        if (place == null) {
          notes.unfile(noteId);
        } else {
          notes.file(noteId, place);
        }
      }
      notes.merge(noteId, listId);
      store.append(noteId, ShoppingKeys.owesLines, listId);
      return [...typed, ...addOwedLines(noteId, wallClock)];
    });
  }

  /// The Note id of the Shopping List the voice capture [noteId] still
  /// owes the items said in its Transcripts (#311); null when it owes
  /// none.
  String? owedTo(String noteId) =>
      store.current(noteId, ShoppingKeys.owesLines);

  /// Adds the items said in the Transcripts of the voice capture
  /// [noteId] to the Shopping List it owes them ([owedTo]), once every
  /// recording has a Transcript or a removed one, recording by recording
  /// in their order, each as [add] adds it, and clears what it owes in
  /// the same batch, so they are added once (#311). While a recording
  /// waits for its Transcript nothing is written. A Note no longer merged
  /// into that list, in the trash, or a list that can no longer be added
  /// to owes nothing any more: that is cleared, and no Line is added.
  /// Returns the ids of the Lines added, brought back or found.
  List<String> addOwedLines(String noteId, WallClock wallClock) {
    final listId = owedTo(noteId);
    if (listId == null) return const [];
    final note = Notes(store).byId(noteId);
    if (note == null) return const [];
    void settled() => store.append(noteId, ShoppingKeys.owesLines, null);
    if (note.isTrashed || note.mergedInto != listId) {
      settled();
      return const [];
    }
    if (note.recordings.any(
      (hash) =>
          !note.transcripts.containsKey(hash) &&
          !note.removedTranscripts.contains(hash),
    )) {
      return const [];
    }
    try {
      return store.batch(() {
        final added = [
          for (final hash in note.recordings)
            for (final item in spokenItems(note.transcripts[hash] ?? ''))
              add(listId, item, wallClock),
        ];
        settled();
        return added;
      });
    } on StateError {
      // No role there any more: nothing is owed.
    } on ArgumentError {
      // No Shopping List any more: the same.
    }
    settled();
    return const [];
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

  /// Moves the open Line [lineId] to just before the open Line [before],
  /// or after the open Lines (#283): its key goes where [add] puts a new
  /// Line there, so a struck Line keeps its place among the open ones for
  /// when it comes back. A struck Line does not move. Where it is
  /// already, nothing is written.
  void move(String lineId, WallClock wallClock, {String? before}) {
    _requireEditable(lineId);
    final (list, line) = _requireShown(lineId, wallClock);
    if (line.struck) {
      throw ArgumentError.value(lineId, 'lineId', 'a struck Line');
    }
    final from = list.open.indexWhere((l) => l.id == lineId);
    final rest = ShoppingList(list.noteId, list.making, [
      for (final l in list.lines)
        if (l.id != lineId) l,
    ], list._gone);
    final others = rest.open;
    int? at;
    if (before != null) {
      if (before == lineId) return;
      at = others.indexWhere((l) => l.id == before);
      if (at < 0) {
        throw ArgumentError.value(before, 'before', 'not an open Line');
      }
    }
    if ((at ?? others.length) == from) return;
    store.append(lineId, ChecklistKeys.order, _keyAt(rest, at));
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

  /// The deleted Lines of the Shopping List of [noteId] that [restore]
  /// brings back, the last deleted first; none when it is no Shopping
  /// List.
  List<DeletedLine> deletedLines(String noteId) =>
      deletedLinesOf(store, noteId, making(noteId));

  /// Checks the Line [lineId]: it is struck through and goes to the end
  /// until the start of the day its reappear time brings it back on
  /// ([Reappear.backFrom]), by [wallClock], or until a time is set for one
  /// without; a Line Once is gone. A struck one stays as it is.
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

  /// Brings the struck or gone Line [lineId] back now, in its place, as
  /// adding its name does. An open one stays as it is.
  void uncheck(String lineId, WallClock wallClock) {
    final (list, line) = _requireShown(lineId, wallClock, gone: true);
    if (!mayCheck(list.noteId)) {
      throw StateError('no role in the Shared Area of ${list.noteId}');
    }
    if (!line.struck && !list._gone.contains(line)) return;
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
    // In the order of their keys; one without a valid key, which no app
    // writes, after them, the one that came last first.
    open.sort((a, b) {
      final valid = LineOrder.isValid(a.line.order);
      if (valid != LineOrder.isValid(b.line.order)) return valid ? -1 : 1;
      final byAt = valid ? 0 : b.at.compareTo(a.at);
      return byAt != 0 ? byAt : byOrder(a, b);
    });
    int byCheck(_Read a, _Read b) {
      final byAt = a.at.compareTo(b.at);
      return byAt != 0 ? byAt : byOrder(a, b);
    }

    // The soonest back first, those that never come back by themselves
    // after them (#328).
    struck.sort((a, b) {
      final (ab, bb) = (a.line.backOn, b.line.backOn);
      final byBack = switch ((ab, bb)) {
        (final ab?, final bb?) => ab.compareTo(bb),
        (null, null) => 0,
        (null, _) => 1,
        (_, null) => -1,
      };
      return byBack != 0 ? byBack : byCheck(a, b);
    });
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
    store.append(id, ShoppingKeys.reappear, Reappear.unset.code, date: now);
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
  /// Note is not in the trash, or with [gone] one gone there.
  Map<String, String?> _requireLine(String lineId, {bool gone = false}) {
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
        (!gone && isGone(fields))) {
      throw ArgumentError.value(lineId, 'lineId', 'not a Shopping List Line');
    }
    return fields;
  }

  Map<String, String?> _requireEditable(String lineId) {
    final fields = _requireLine(lineId);
    if (!mayEdit(lineId)) throw StateError('may not change $lineId');
    return fields;
  }

  /// The Shopping List the shown Line [lineId] lies in, and the Line;
  /// with [gone] a gone one too.
  (ShoppingList, ShoppingLine) _requireShown(
    String lineId,
    WallClock wallClock, {
    bool gone = false,
  }) {
    final note = _requireLine(lineId, gone: gone)[ChecklistKeys.note];
    final list = note == null ? null : byNote(note, wallClock);
    final line = switch (list) {
      final list? =>
        list.line(lineId) ??
            (gone ? list._gone.where((l) => l.id == lineId).firstOrNull : null),
      null => null,
    };
    if (list == null || line == null) {
      throw ArgumentError.value(lineId, 'lineId', 'not a Shopping List Line');
    }
    return (list, line);
  }
}

/// A Line of a Shopping List as read from its latest entries, with the
/// moment it sorts by: a struck one, after its day back (#328), and a
/// gone one by its check, an open one
/// without a valid key by when it came back or was added, on the wall
/// clock.
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
      gone = reappear.isOnce;
      final day = reappear.backFrom(Day.of(wallClock(check.date)));
      // Without a time it stays struck until one is set.
      struck = !gone && (day == null || today < day);
      if (struck) back = day;
      if (!gone && !struck) came = day;
      at = gone || struck || day == null
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
        backIn: back == null ? null : BackIn.days(today.daysUntil(back)),
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
