import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'appointments.dart';
import 'frogs_screen.dart';
import 'note_card.dart';
import 'session_view.dart';
import 'unit_system.dart';

/// An Agenda row with where its Note is filed and whether it shows
/// Masked, read once per change; for a Session its Session, and today
/// its picks; for a Routine spawned from a Checklist its Lines as checked
/// for this Occurrence (#121).
typedef PlacedRow = ({
  AgendaRow row,
  Area? area,
  bool masked,
  Session? session,
  ShownPicks? picks,
  ShownLines? lines,
});

/// The Lines of a row's Checklist as checked for its Occurrence, and
/// whether this Device may check them.
typedef ShownLines = ({Checklist list, bool checks});

/// Today's picks of a Session, with whether each shows Masked and opens
/// its entry form, and whether they swap.
typedef ShownPicks = ({
  SessionOccurrence today,
  List<bool> masked,
  List<bool> opens,
  bool swaps,
});

/// A Shopping List's group on today's Agenda (#123, #186, #208): its
/// open [lines] under its name, then those checked today, by id in
/// [struck], which stay struck for the day unless this Device hides
/// what is checked (#276); or none when it shows as one row with its
/// [count] of open Lines, as chosen for it or as a Masked one does; with
/// where its Note is filed, whether it shows Masked and whether this
/// Device may check its Lines.
typedef ShoppingRow = ({
  Note note,
  Area? area,
  bool masked,
  List<ShoppingLine> lines,
  Set<String> struck,
  int count,
  bool checks,
});

/// A Checklist's group on today's Agenda (#143, #208), shaped as a
/// Shopping List's: the [lines] under its name, or none when it shows as
/// its heading alone with how many of the [list]'s Lines are open, as a
/// Masked one does; with where its Note is filed, whether it shows Masked
/// and whether this Device may check its Lines.
typedef ChecklistRow = ({
  Note note,
  Area? area,
  bool masked,
  Checklist list,
  List<Line> lines,
  bool checks,
});

/// The Lines a Checklist shows under its heading on the Agenda (#208): a
/// Checklist Routine's Occurrence every Line, checked ones struck
/// through, or only its open ones where this Device [hidesChecked]
/// (#217); a Checklist without a Routine its open ones, and those checked
/// [today] struck for the day (#276), a Line with a group while a Line of
/// it shows. The one place that decides it, for the Agenda and its widget
/// alike.
List<Line> agendaLines(
  Checklist list, {
  required bool hidesChecked,
  ({Day day, WallClock wallClock})? today,
}) {
  if (list.on != null && !hidesChecked) return list.lines;
  final shown = {
    for (final line in list.lines)
      if (!line.checked ||
          (!hidesChecked &&
              today != null &&
              line.checkedOn(today.day, today.wallClock)))
        line.id,
  };
  // A group's Line shows over the Lines of its group that show.
  shown.addAll([
    for (final line in list.lines)
      if (line.parent case final parent? when shown.contains(line.id)) parent,
  ]);
  return [
    for (final line in list.lines)
      if (shown.contains(line.id)) line,
  ];
}

/// Whether [placed] shows as Done: checked off, or Done from the entries
/// of every pick or from every Line checked.
bool shownDone(PlacedRow placed) =>
    placed.row.done != null ||
    (placed.picks?.today.complete ?? false) ||
    (placed.lines?.list.allChecked ?? false);

/// The picks of [picks] that show on the Agenda with their index: every
/// one, or only those without an entry where this Device [hidesChecked]
/// (#217).
List<(int, SessionPick)> agendaPicks(
  ShownPicks picks, {
  required bool hidesChecked,
}) => [
  for (final (i, pick) in picks.today.picks.indexed)
    if (!hidesChecked || !pick.done) (i, pick),
];

/// One row of a day on the Agenda: an Occurrence, a part of a calendar
/// appointment, a Shopping List's or a Checklist's.
typedef DayEntry = ({
  PlacedRow? row,
  AppointmentPiece? piece,
  ShoppingRow? shopping,
  ChecklistRow? checklist,
});

/// A Frog of the "Eat a frog" section with its Note and Area, and
/// whether it shows Masked.
typedef ShownFrog = ({PlacedFrog placed, bool masked});

/// The rows of the Agenda's days, in the Agenda's order, as both the
/// Agenda and the home-screen Agenda widget show them (#153), so the
/// two cannot drift apart: today's with its Shopping Lists and
/// Checklists, tomorrow's and the days after, and today's Frogs.
class AgendaDays {
  /// How many days the Agenda shows, today included, and the widget with
  /// it (#194).
  static const shown = 7;

  final Day today;
  final Day tomorrow;
  final List<DayEntry> todayRows;
  final List<DayEntry> tomorrowRows;

  /// The days after tomorrow, each with its rows.
  final List<({Day day, List<DayEntry> rows})> later;

  /// Today's Frogs as they show.
  final List<ShownFrog> frogs;

  /// The units a Session's targets show in.
  final UnitSystem system;

  /// Whether this Device hides what is checked instead of striking it
  /// through (#217): the rows shown as Done are left out already; the
  /// checked Lines and the picks with an entry go with [agendaLines] and
  /// [agendaPicks].
  final bool hidesChecked;

  /// Whether the "Eat a frog" section of [frogs] shows today: while it is
  /// [FrogsToday.visible], unless this Device [hidesChecked] and today's
  /// Frog is eaten (#217).
  bool showsFrogs(FrogsToday frogs) =>
      frogs.visible && !(hidesChecked && frogs.eaten);

  /// What the days' rows are made of before the appointments go in, so
  /// a new calendar read goes in without reading the log again (#205).
  final _DayParts _parts;

  AgendaDays._(
    _DayParts parts,
    List<Appointment> appointments, {
    required this.frogs,
    required this.system,
    required this.hidesChecked,
  }) : _parts = parts,
       today = parts.today.day,
       tomorrow = parts.tomorrow.day,
       todayRows = parts.today.entries(appointments, parts.wallClock),
       tomorrowRows = parts.tomorrow.entries(appointments, parts.wallClock),
       later = [
         for (final day in parts.later)
           (day: day.day, rows: day.entries(appointments, parts.wallClock)),
       ];

  /// These days with the calendar's [appointments] in place of those
  /// they were read with; the log is not read again.
  AgendaDays withAppointments(List<Appointment> appointments) => AgendaDays._(
    _parts,
    appointments,
    frogs: frogs,
    system: system,
    hidesChecked: hidesChecked,
  );

  /// Reads the [days] of the Agenda from [store] on [wallClock], with
  /// today's Session Occurrences [sessions], today's "Eat a frog" section
  /// [frogsToday] and the calendar's [appointments]; a Session's targets
  /// in [system], else in this Device's ([deviceUnitSystem]), which an
  /// isolate of its own cannot ask the platform for (#207). It writes
  /// nothing: the Agenda hands in the picks it writes as it shows, the
  /// widget's snapshot those it reads.
  factory AgendaDays.read({
    required Store store,
    required Places places,
    required WallClock wallClock,
    required int days,
    required List<SessionOccurrence> sessions,
    required FrogsToday frogsToday,
    List<Appointment> appointments = const [],
    UnitSystem? system,
  }) {
    final hidesChecked = store.agendaHidesChecked;
    final reader = _Reader(store, places, wallClock, sessions);
    // What shows as Done leaves where this Device hides it (#217), and
    // so does what is struck as skipped (#269).
    List<PlacedRow> placed(List<AgendaRow> rows) => [
      for (final row in rows)
        if (reader.place(row) case final placed
            when !hidesChecked ||
                !(shownDone(placed) || placed.row.skipped != null))
          placed,
    ];
    final agenda = Routines(store).agenda(wallClock, days: days);
    reader.listNotes = reader.checklists
        .ofNotes({
          for (final rows in [
            agenda.todayRows,
            agenda.tomorrowRows,
            ...agenda.laterRows,
          ])
            for (final row in rows)
              if (row.note case final note?) note.id,
        })
        .keys
        .toSet();
    final frogs = Frogs(store);
    return AgendaDays._(
      (
        wallClock: wallClock,
        today: _DayPart(
          agenda.today,
          placed(agenda.todayRows),
          shopping: reader.shoppingRows(hidesChecked: hidesChecked),
          checklists: reader.checklistRows(hidesChecked: hidesChecked),
        ),
        tomorrow: _DayPart(agenda.tomorrow, placed(agenda.tomorrowRows)),
        later: [
          for (final (i, rows) in agenda.laterRows.indexed)
            _DayPart(agenda.later(i), placed(rows)),
        ],
      ),
      appointments,
      frogs: [
        for (final frog in frogsToday.shown)
          if (placeFrog(frogs, places, frog) case final placed?)
            (placed: placed, masked: reader.masksNote(placed.note)),
      ],
      system: system ?? deviceUnitSystem(store),
      hidesChecked: hidesChecked,
    );
  }
}

/// The days' parts of an [AgendaDays], and the wall clock that places
/// the appointments on them.
typedef _DayParts = ({
  WallClock wallClock,
  _DayPart today,
  _DayPart tomorrow,
  List<_DayPart> later,
});

/// One day's rows as read from the log: its Occurrences [rows], and
/// today's Shopping Lists [shopping] and Checklists [checklists].
class _DayPart {
  final Day day;
  final List<PlacedRow> rows;
  final List<ShoppingRow> shopping;
  final List<ChecklistRow> checklists;

  const _DayPart(
    this.day,
    this.rows, {
    this.shopping = const [],
    this.checklists = const [],
  });

  /// The rows of [day] with the [appointments] on it: its all-day
  /// Occurrences and all-day appointments, then the rows of the Shopping
  /// Lists and of the Checklists, then the timed ones of both by time, an
  /// Occurrence before an appointment at the same time.
  List<DayEntry> entries(List<Appointment> appointments, WallClock wallClock) {
    final pieces = appointmentsOn(day, appointments, wallClock);
    final timedRows = [
      for (final placed in rows)
        if (!placed.row.occurrence.allDay) placed,
    ];
    final timedPieces = [
      for (final piece in pieces)
        if (!piece.allDay) piece,
    ];
    final timed = <DayEntry>[];
    var r = 0;
    var p = 0;
    while (r < timedRows.length || p < timedPieces.length) {
      final rowAt = r < timedRows.length
          ? timedRows[r].row.occurrence.time
          : null;
      final pieceAt = p < timedPieces.length ? timedPieces[p].at : null;
      if (rowAt != null && (pieceAt == null || rowAt.compareTo(pieceAt) <= 0)) {
        timed.add((
          row: timedRows[r++],
          piece: null,
          shopping: null,
          checklist: null,
        ));
      } else {
        timed.add((
          row: null,
          piece: timedPieces[p++],
          shopping: null,
          checklist: null,
        ));
      }
    }
    return [
      for (final placed in rows)
        if (placed.row.occurrence.allDay)
          (row: placed, piece: null, shopping: null, checklist: null),
      for (final piece in pieces)
        if (piece.allDay)
          (row: null, piece: piece, shopping: null, checklist: null),
      for (final row in shopping)
        (row: null, piece: null, shopping: row, checklist: null),
      for (final row in checklists)
        (row: null, piece: null, shopping: null, checklist: row),
      ...timed,
    ];
  }
}

/// What [AgendaDays.read] reads its rows with.
class _Reader {
  final Store store;
  final Places places;
  final WallClock wallClock;
  final Notes notes;
  final AreaLocks locks;
  final Sessions sessions;
  final Checklists checklists;
  final ShoppingLists shopping;

  /// Today's Occurrences of Sessions with their picks, by Session id and
  /// Occurrence key.
  final Map<String, SessionOccurrence> sessionsToday;

  /// The Notes of the rows that are Checklists, read in one pass so only
  /// their rows read their Lines for the Occurrence.
  Set<String> listNotes = const {};

  /// Today on [wallClock], read once.
  late final Day today = Day.today(store.clock(), wallClock);

  _Reader(
    this.store,
    this.places,
    this.wallClock,
    List<SessionOccurrence> today,
  ) : notes = Notes(store),
      locks = AreaLocks(store),
      sessions = Sessions(store),
      checklists = Checklists(store),
      shopping = ShoppingLists(store),
      sessionsToday = {
        for (final occurrence in today)
          _sessionKey(occurrence.session.id, occurrence.occurrence): occurrence,
      };

  static String _sessionKey(String id, Occurrence occurrence) =>
      '$id@${occurrence.key}';

  /// Today's groups of the Shopping Lists, as this person chose for each
  /// list (#186): its open Lines under it, then those checked today,
  /// struck for the day unless this Device [hidesChecked] (#276); or only
  /// the number of open ones, never 0. A list of a Masked Area tells
  /// nothing of its Lines.
  List<ShoppingRow> shoppingRows({required bool hidesChecked}) {
    final rows = <ShoppingRow>[];
    for (final (:list, :shows, :lines) in shopping.agendaRows(
      wallClock,
      checkedToday: !hidesChecked,
    )) {
      final note = notes.byId(list.noteId);
      if (note == null) continue;
      final masked = masksNote(note);
      final listed = shows == ListOnAgenda.lines && !masked;
      if (!listed && list.openCount == 0) continue;
      final open = {for (final line in list.open) line.id};
      rows.add((
        note: note,
        area: placeNote(places, note).area,
        masked: masked,
        lines: listed ? lines : const <ShoppingLine>[],
        struck: {
          if (listed)
            for (final line in lines)
              if (!open.contains(line.id)) line.id,
        },
        count: list.openCount,
        checks: shopping.mayCheck(note.id),
      ));
    }
    return rows;
  }

  /// Today's rows of the Checklists without a Routine that show on this
  /// person's Agenda, as this person chose for each list (#219): its open
  /// Lines under it, then those checked today struck for the day unless
  /// this Device [hidesChecked] (#276), or only their count, never with
  /// none open; oldest capture first. One of a Masked Area tells nothing
  /// of it.
  List<ChecklistRow> checklistRows({required bool hidesChecked}) => [
    for (final (:list, :shows) in checklists.agendaRows(
      checkedToday: hidesChecked ? null : wallClock,
    ))
      if (notes.byId(list.noteId) case final note?)
        if (checklistRow(
              note,
              list,
              masked: masksNote(note),
              count: shows == ListOnAgenda.count,
              hidesChecked: hidesChecked,
            )
            case final row when row.lines.isNotEmpty || list.openCount > 0)
          row,
  ];

  /// The group of the Checklist [list] of [note]: every Line it shows
  /// under its name, or, [masked] or shown as its [count], its heading
  /// alone with the count.
  ChecklistRow checklistRow(
    Note note,
    Checklist list, {
    required bool masked,
    bool count = false,
    required bool hidesChecked,
  }) => (
    note: note,
    area: placeNote(places, note).area,
    masked: masked,
    list: list,
    // Its checked Lines show only on the day they were checked (#143,
    // #276).
    lines: masked || count
        ? const <Line>[]
        : agendaLines(
            list,
            hidesChecked: hidesChecked,
            today: (day: today, wallClock: wallClock),
          ),
    checks: checklists.mayCheck(note.id),
  );

  PlacedRow place(AgendaRow row) {
    final id = row.routine.id;
    final today = sessionsToday[_sessionKey(id, row.occurrence)];
    return (
      row: row,
      area: switch (row.note) {
        final note? => placeNote(places, note).area,
        null => null,
      },
      masked: locks.masks(row.routine),
      session: today?.session ?? sessions.byId(row.routine.id),
      picks: today == null
          ? null
          : (
              today: today,
              masked: [
                for (final pick in today.picks)
                  switch (pick.note) {
                    final note? => masksNote(note),
                    null => false,
                  },
              ],
              opens: [
                for (final pick in today.picks) opensPick(store, id, pick),
              ],
              swaps: today.canSwap && store.mayCheckOff(id),
            ),
      lines: switch (listNotes.contains(row.note?.id)
          ? checklists.forOccurrence((
              routineId: id,
              occurrence: row.occurrence,
            ))
          : null) {
        final list? when list.count > 0 => (
          list: list,
          checks: store.mayCheckOff(id),
        ),
        _ => null,
      },
    );
  }

  /// Whether [note] shows Masked here: it is filed in a locked Area set
  /// to Masked, as [AreaLocks.masks] says of a Routine.
  bool masksNote(Note note) {
    final area = locks.lockedAreaOf(note);
    return area != null && store.isAreaMasked(area);
  }
}
