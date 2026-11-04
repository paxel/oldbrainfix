import 'package:flutter/foundation.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'agenda_reader.dart';
import 'agenda_rows.dart';
import 'app_lock.dart';
import 'l10n.dart';
import 'missed_summary_card.dart';
import 'session_view.dart';
import 'schedule_text.dart';
import 'unit_system.dart';

/// One row of the home-screen Agenda widget, in the words it shows: an
/// Occurrence, a Line of its Checklist or a pick of its Session under
/// it, a Shopping List's or a Checklist's heading and its Lines under
/// it, a Frog, a Missed Occurrence, or a heading or a line of text. A
/// group of the Agenda (#208) is its heading's row with its entries'
/// rows after it, indented (#209).
class WidgetRow {
  /// An Occurrence; its box checks it off as Done.
  static const occurrence = 'occurrence';

  /// A Session's Occurrence, its picks' heading; its box only shows
  /// whether it is Done, and a tap on it does nothing (#209).
  static const session = 'session';

  /// A pick of a Session's Occurrence today, the Tally [noteId]; its box
  /// shows whether it has an entry, and a tap on it opens its entry form
  /// where this Device may ([checks]) (#209).
  static const pick = 'pick';

  /// A Line of a Checklist Routine's Occurrence; its box checks it for
  /// that Occurrence.
  static const line = 'line';

  /// A Shopping List's row: with [lineId] an open Line under the list's
  /// heading, whose box checks it, else the heading, with no box: the
  /// list's name, or its count when it shows no Lines.
  static const shopping = 'shopping';

  /// A Checklist without a Routine, its Lines' heading, with no box: its
  /// name, or how many Lines are open when it shows none.
  static const checklist = 'checklist';

  /// An open Line of a Checklist without a Routine, under its heading;
  /// its box checks it (#209).
  static const checklistLine = 'checklistLine';

  /// A Frog of today's "Eat a frog"; its box checks it off.
  static const frog = 'frog';

  /// A Missed Occurrence of the Missed Summary; its box checks it off,
  /// late.
  static const missed = 'missed';

  /// A heading, "Missed" or "Eat a frog".
  static const heading = 'heading';

  /// A line of text without a box: a Session's pick, how many a Session
  /// picks, a Routine of the Missed Summary or how many more it Missed.
  static const text = 'text';

  /// Where a day's row sorts among the calendar's appointments, which
  /// the widget reads itself and puts in (UI laws, Agenda): among the
  /// all-day ones, the Lists, the timed ones by [at], or the Frogs.
  static const allDay = 'allDay';
  static const lists = 'lists';
  static const timed = 'timed';
  static const frogs = 'frogs';

  final String kind;
  final String title;

  /// Its time, or "All day"; empty for a row without one.
  final String time;
  final bool done;

  /// Whether this Device may check it here, where it has a box.
  final bool checks;

  /// How far it is indented: 1 under its heading, 2 for a Line under
  /// another Line.
  final int indent;

  /// The Routine and the [Occurrence.key] an Occurrence, a Missed one or
  /// a Line is checked for.
  final String? routineId;
  final String? occurrenceKey;

  /// The Line of a Checklist or a Shopping List its box checks.
  final String? lineId;

  /// The Frog its box checks off.
  final String? frogId;

  /// The Tally a [pick] opens the entry form of.
  final String? noteId;

  /// Where it sorts among the appointments: [allDay], [lists], [timed]
  /// or [frogs]; null outside a day.
  final String? block;

  /// The minute of the day of a [timed] row.
  final int? at;

  const WidgetRow({
    required this.kind,
    required this.title,
    this.time = '',
    this.done = false,
    this.checks = false,
    this.indent = 0,
    this.routineId,
    this.occurrenceKey,
    this.lineId,
    this.frogId,
    this.noteId,
    this.block,
    this.at,
  });

  Map<String, Object?> toMap() => {
    'kind': kind,
    'title': title,
    if (time.isNotEmpty) 'time': time,
    'done': done,
    if (checks) 'checks': true,
    if (indent > 0) 'indent': indent,
    'routine': ?routineId,
    'occurrence': ?occurrenceKey,
    'line': ?lineId,
    'frog': ?frogId,
    'note': ?noteId,
    'block': ?block,
    'at': ?at,
  };
}

/// What the Agenda read of the log as it loaded, all the widget's
/// snapshot needs of it (#204): its days, today's Frogs, the Missed
/// Summary and the Unsorted count, on [wallClock].
class AgendaRead {
  final WallClock wallClock;

  /// The days the Agenda shows, without the calendar's appointments,
  /// which the widget reads itself.
  final AgendaDays days;
  final FrogsToday frogs;
  final MissedSummary missed;
  final int unsorted;

  const AgendaRead({
    required this.wallClock,
    required this.days,
    required this.frogs,
    required this.missed,
    required this.unsorted,
  });

  /// What [load] read, on [wallClock] (#207).
  AgendaRead.of(AgendaLoad load, this.wallClock)
    : days = load.days,
      frogs = load.frogs,
      missed = load.missed,
      unsorted = load.unsorted;
}

/// What the home-screen Agenda widget shows, handed to the platform
/// whenever the alarms are synced: the Missed Summary, and the rows of
/// the days the Agenda shows in Agenda order as [AgendaDays] reads them
/// for the Agenda, each with its heading (#194), so the widget shows
/// them and moves on to the new day at midnight without the app; the
/// Unsorted count; the calendars whose
/// appointments the widget reads itself, as nothing of an appointment is
/// kept; and the texts, in the reader's language.
class AgendaSnapshot {
  /// The version of its structure (ADR 0034): 2 since the widget shows
  /// every kind of row the Agenda does (#153), 3 since it shows the
  /// Agenda's groups (#209): a Session's heading and picks, and a
  /// Checklist's and a Shopping List's heading with their Lines, 4 since
  /// it holds the days the Agenda shows, each with its heading (#194).
  static const version = 4;

  /// The days, `YYYY-MM-DD`, with their rows: today and the days after
  /// it the Agenda shows.
  final Map<String, List<WidgetRow>> days;

  /// Each of [days]' own heading, its weekday and date; the widget calls
  /// the day it shows as today [today] and the next [tomorrow], as they
  /// are then.
  final Map<String, String> headings;

  /// The Missed Summary's rows, with its heading; none while it is empty.
  final List<WidgetRow> missed;
  final int unsorted;

  /// The calendars whose appointments the Agenda shows; none while that
  /// is off.
  final List<String> calendars;

  /// The headings "Today" and "Tomorrow".
  final String today;
  final String tomorrow;

  /// Shown when today has nothing due.
  final String empty;

  /// Shown when a later day has nothing due, "Nothing planned".
  final String planned;

  /// "x Unsorted", with the number in.
  final String unsortedText;

  /// Whether this Device has "Fat fingers" on (#224): the widget's text
  /// grows with the app's.
  final bool fatFingers;

  /// What the checkbox says to a screen reader.
  final String done;

  /// An appointment's texts: "All day", the title of one without,
  /// "from {time}" and "until {time}", the widget putting in the time.
  final String allDay;
  final String noTitle;
  final String since;
  final String until;

  const AgendaSnapshot({
    required this.days,
    required this.headings,
    required this.missed,
    required this.unsorted,
    required this.calendars,
    required this.today,
    required this.tomorrow,
    required this.empty,
    required this.planned,
    required this.unsortedText,
    required this.done,
    required this.allDay,
    required this.noTitle,
    required this.since,
    required this.until,
    this.fatFingers = false,
  });

  /// How many times a snapshot read the log itself ([AgendaSnapshot.of]),
  /// for the tests that count them: not while the Agenda's read is
  /// current (#204).
  @visibleForTesting
  static var reads = 0;

  /// The snapshot of [store] now, with days as [wallClock] shows them.
  /// It writes nothing: a Session's or the Frogs' pick not made yet
  /// shows as the Agenda will make it.
  factory AgendaSnapshot.of(
    Store store,
    WallClock wallClock,
    AppLocalizations t,
    String locale,
  ) => AgendaSnapshot.from(
    store,
    AgendaRead.of(
      readLog(store, wallClock, deviceUnitSystem(store)),
      wallClock,
    ),
    t,
    locale,
  );

  /// The log as a snapshot of its own reads it, writing nothing, counted
  /// in [reads].
  static AgendaLoad readLog(
    Store store,
    WallClock wallClock,
    UnitSystem system,
  ) {
    reads++;
    return readAgenda(store, wallClock, system);
  }

  /// The snapshot of what [read] holds, as the Agenda read it (#204): no
  /// read of the log again but the words.
  factory AgendaSnapshot.from(
    Store store,
    AgendaRead agenda,
    AppLocalizations t,
    String locale,
  ) {
    final frogs = agenda.frogs;
    final read = agenda.days;
    final words = _Words(
      t,
      locale,
      store,
      read.system,
      NumberFormat.decimalPattern(locale).symbols.DECIMAL_SEP,
      read.hidesChecked,
    );
    final unsorted = agenda.unsorted;
    return AgendaSnapshot(
      days: {
        '${read.today}': [
          ...words.day(read.today, read.todayRows),
          if (read.showsFrogs(frogs)) ...words.frogs(read.frogs),
        ],
        '${read.tomorrow}': words.day(read.tomorrow, read.tomorrowRows),
        for (final (:day, :rows) in read.later) '$day': words.day(day, rows),
      },
      headings: {
        for (final day in [
          read.today,
          read.tomorrow,
          for (final (:day, rows: _) in read.later) day,
        ])
          '$day': t.agendaDay(day.wall),
      },
      missed: words.missed(agenda.missed),
      unsorted: unsorted,
      calendars: store.agendaCalendarOn
          ? ([...store.agendaCalendars]..sort())
          : const [],
      today: t.agendaToday,
      tomorrow: t.agendaTomorrow,
      empty: t.widgetNothingToday,
      planned: t.nothingPlanned,
      unsortedText: t.unsortedBanner(unsorted),
      done: t.markDone,
      allDay: t.allDay,
      noTitle: t.appointmentNoTitle,
      since: t.appointmentSince('{time}'),
      until: t.appointmentUntil('{time}'),
      fatFingers: store.fatFingers,
    );
  }

  Map<String, Object?> toMap() => {
    'v': version,
    'days': {
      for (final MapEntry(:key, :value) in days.entries)
        key: [for (final row in value) row.toMap()],
    },
    'headings': headings,
    'missed': [for (final row in missed) row.toMap()],
    'unsorted': unsorted,
    'calendars': calendars,
    if (fatFingers) 'fatFingers': true,
    'labels': {
      'today': today,
      'tomorrow': tomorrow,
      'empty': empty,
      'planned': planned,
      'unsorted': unsortedText,
      'done': done,
      'allDay': allDay,
      'noTitle': noTitle,
      'since': since,
      'until': until,
    },
  };
}

/// Puts the Agenda's rows into the widget's words, as the Agenda shows
/// them: a row from a locked Area set to Masked shows its masked text,
/// the home screen being as public as the lock screen, and names
/// nothing under it.
class _Words {
  final AppLocalizations t;
  final String locale;
  final Store store;
  final UnitSystem system;
  final String decimal;

  /// Whether this Device hides what is checked, as the Agenda read it
  /// (#217).
  final bool hidesChecked;

  _Words(
    this.t,
    this.locale,
    this.store,
    this.system,
    this.decimal,
    this.hidesChecked,
  );

  /// The first line of [note]'s text, "Photo" for a photo Note without
  /// words.
  String _firstLine(Note note) =>
      noteText(t, note).trim().split('\n').first.trim();

  /// The rows of [day] in Agenda order.
  List<WidgetRow> day(Day day, List<DayEntry> entries) => [
    for (final entry in entries)
      if (entry.row case final placed?)
        ..._occurrence(day, placed)
      else if (entry.shopping case final shopping?)
        ..._shopping(shopping)
      else if (entry.checklist case final checklist?)
        ..._checklist(checklist),
  ];

  List<WidgetRow> _occurrence(Day day, PlacedRow placed) {
    final (:row, :masked, :session, :picks, :lines, area: _) = placed;
    final time = row.occurrence.time;
    final block = time == null ? WidgetRow.allDay : WidgetRow.timed;
    final at = time == null ? null : time.hour * 60 + time.minute;
    final note = row.note;
    // Done from the entries of every pick, or from every Line checked,
    // stays checked.
    final complete =
        (picks?.today.complete ?? false) || (lines?.list.allChecked ?? false);
    return [
      WidgetRow(
        kind: session == null ? WidgetRow.occurrence : WidgetRow.session,
        title: masked
            ? maskedText(t, locale, row.routine, row.occurrence)
            : note == null
            ? t.noteGone
            : _firstLine(note),
        time: occurrenceTime(t, locale, row.occurrence),
        done: row.done != null || complete,
        // A Session's box only shows Done: its picks make it (#208).
        checks: !complete && session == null,
        routineId: row.routine.id,
        occurrenceKey: row.occurrence.key,
        block: block,
        at: at,
      ),
      if (!masked && session != null)
        if (picks == null)
          WidgetRow(
            kind: WidgetRow.text,
            title: t.sessionPicks(session.countOn(day)),
            indent: 1,
            block: block,
            at: at,
          )
        else if (picks.today.picks.isEmpty)
          WidgetRow(
            kind: WidgetRow.text,
            title: t.sessionNothingToPick,
            indent: 1,
            block: block,
            at: at,
          )
        else
          for (final (i, pick) in agendaPicks(
            picks,
            hidesChecked: hidesChecked,
          ))
            WidgetRow(
              kind: WidgetRow.pick,
              title: picks.masked[i]
                  ? t.maskedTally
                  : pickText(t, pick, system, decimal),
              done: pick.done,
              checks: picks.opens[i],
              indent: 1,
              routineId: row.routine.id,
              occurrenceKey: row.occurrence.key,
              noteId: pick.noteId,
              block: block,
              at: at,
            )
      else if (!masked && lines != null)
        for (final line in agendaLines(lines.list, hidesChecked: hidesChecked))
          WidgetRow(
            kind: WidgetRow.line,
            title: line.text,
            done: line.checked,
            checks: lines.checks,
            indent: line.inGroup ? 2 : 1,
            routineId: row.routine.id,
            occurrenceKey: row.occurrence.key,
            lineId: line.id,
            block: block,
            at: at,
          ),
    ];
  }

  String _listTitle(Note note, String fallback) => switch (_firstLine(note)) {
    '' => fallback,
    final title => title,
  };

  /// A Shopping List's group, as the Agenda's (#208): its name as the
  /// heading, without a box, and its open Lines under it, each with its
  /// box; or the heading alone with how many are open.
  List<WidgetRow> _shopping(ShoppingRow shopping) {
    final (:note, :masked, :lines, :count, :checks, area: _) = shopping;
    final title = _listTitle(note, t.shoppingHeading);
    final shown = masked ? t.maskedShoppingList : title;
    return [
      WidgetRow(
        kind: WidgetRow.shopping,
        title: lines.isEmpty ? t.shoppingRowOpen(shown, count) : title,
        block: WidgetRow.lists,
      ),
      for (final line in lines)
        WidgetRow(
          kind: WidgetRow.shopping,
          title: line.text,
          checks: checks,
          indent: 1,
          lineId: line.id,
          block: WidgetRow.lists,
        ),
    ];
  }

  /// A Checklist's group, as the Agenda's (#208): its name as the
  /// heading, without a box, and the Lines it shows under it, each with
  /// its box; or, Masked, the heading alone with how many are open.
  List<WidgetRow> _checklist(ChecklistRow row) {
    final (:note, :masked, :list, :lines, :checks, area: _) = row;
    final title = _listTitle(note, t.checklistHeading);
    return [
      WidgetRow(
        kind: WidgetRow.checklist,
        title: lines.isEmpty
            ? t.checklistRowOpen(
                masked ? t.maskedChecklist : title,
                list.openCount,
                list.count,
              )
            : title,
        block: WidgetRow.lists,
      ),
      for (final line in lines)
        WidgetRow(
          kind: WidgetRow.checklistLine,
          title: line.text,
          done: line.checked,
          checks: checks,
          indent: line.inGroup ? 2 : 1,
          lineId: line.id,
          block: WidgetRow.lists,
        ),
    ];
  }

  /// Today's "Eat a frog" section, under what is due.
  List<WidgetRow> frogs(List<ShownFrog> frogs) => [
    WidgetRow(
      kind: WidgetRow.heading,
      title: t.eatAFrog,
      block: WidgetRow.frogs,
    ),
    for (final (:placed, :masked) in frogs)
      WidgetRow(
        kind: WidgetRow.frog,
        title: masked ? t.maskedFrog : _firstLine(placed.note),
        done: placed.frog.isDone,
        checks: store.mayCheckOff(placed.frog.id),
        frogId: placed.frog.id,
        block: WidgetRow.frogs,
      ),
  ];

  /// The Missed Summary as its card shows it: each Routine with how often
  /// it Missed, and its latest Missed Occurrences to check off late.
  List<WidgetRow> missed(MissedSummary summary) {
    if (summary.isEmpty) return const [];
    final locks = AreaLocks(store);
    return [
      WidgetRow(kind: WidgetRow.heading, title: t.missedTitle),
      for (final missed in summary.routines) ...[
        WidgetRow(
          kind: WidgetRow.text,
          title:
              '${switch (missed.note) {
                _ when locks.masks(missed.routine) => maskedText(t, locale, missed.routine),
                final note? => _firstLine(note),
                null => t.noteGone,
              }}'
              ' · ${t.missedTimes(missed.count)}',
        ),
        for (final (:occurrence, missedAt: _) in missed.missed.take(
          MissedSummaryCard.shown,
        ))
          WidgetRow(
            kind: WidgetRow.missed,
            title: dayText(locale, occurrence.day),
            time: occurrenceTime(t, locale, occurrence),
            checks: true,
            routineId: missed.routine.id,
            occurrenceKey: occurrence.key,
          ),
        if (missed.count > MissedSummaryCard.shown)
          WidgetRow(
            kind: WidgetRow.text,
            title: t.missedMore(missed.count - MissedSummaryCard.shown),
            indent: 1,
          ),
      ],
    ];
  }
}
