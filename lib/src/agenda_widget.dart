import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'agenda_rows.dart';
import 'app_lock.dart';
import 'l10n.dart';
import 'missed_summary_card.dart';
import 'session_view.dart';
import 'schedule_text.dart';

/// One row of the home-screen Agenda widget, in the words it shows: an
/// Occurrence, a Line of its Checklist or a pick of its Session under
/// it, a Shopping List's or a Checklist's row, a Frog, a Missed
/// Occurrence, or a heading or a line of text.
class WidgetRow {
  /// An Occurrence; its box checks it off as Done.
  static const occurrence = 'occurrence';

  /// A Line of a Checklist Routine's Occurrence; its box checks it for
  /// that Occurrence.
  static const line = 'line';

  /// A Shopping List's row: with [lineId] a Line that came back, whose
  /// box checks it, else the list's count, with no box.
  static const shopping = 'shopping';

  /// A Checklist without a Routine, with how many Lines are open; no box.
  static const checklist = 'checklist';

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

  /// How far it is indented: 1 under its Occurrence, 2 in a group.
  final int indent;

  /// The Routine and the [Occurrence.key] an Occurrence, a Missed one or
  /// a Line is checked for.
  final String? routineId;
  final String? occurrenceKey;

  /// The Line of a Checklist or a Shopping List its box checks.
  final String? lineId;

  /// The Frog its box checks off.
  final String? frogId;

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
    'block': ?block,
    'at': ?at,
  };
}

/// What the home-screen Agenda widget shows, handed to the platform
/// whenever the alarms are synced: the Missed Summary, and today's rows
/// in Agenda order as [AgendaDays] reads them for the Agenda, and
/// tomorrow's as well, so the widget moves on to the new day at
/// midnight without the app; the Unsorted count; the calendars whose
/// appointments the widget reads itself, as nothing of an appointment is
/// kept; and the texts, in the reader's language.
class AgendaSnapshot {
  /// The version of its structure (ADR 0034): 2 since the widget shows
  /// every kind of row the Agenda does (#153).
  static const version = 2;

  /// The days, `YYYY-MM-DD`, with their rows: today and tomorrow.
  final Map<String, List<WidgetRow>> days;

  /// The Missed Summary's rows, with its heading; none while it is empty.
  final List<WidgetRow> missed;
  final int unsorted;

  /// The calendars whose appointments the Agenda shows; none while that
  /// is off.
  final List<String> calendars;

  /// The heading, "Today".
  final String today;

  /// Shown when the day has nothing due.
  final String empty;

  /// "x Unsorted", with the number in.
  final String unsortedText;

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
    required this.missed,
    required this.unsorted,
    required this.calendars,
    required this.today,
    required this.empty,
    required this.unsortedText,
    required this.done,
    required this.allDay,
    required this.noTitle,
    required this.since,
    required this.until,
  });

  /// The snapshot of [store] now, with days as [wallClock] shows them.
  /// It writes nothing: a Session's or the Frogs' pick not made yet
  /// shows as the Agenda will make it.
  factory AgendaSnapshot.of(
    Store store,
    WallClock wallClock,
    AppLocalizations t,
    String locale,
  ) {
    final frogs = Frogs(store).today(wallClock);
    final read = AgendaDays.read(
      store: store,
      places: Places(store),
      wallClock: wallClock,
      days: 2,
      sessions: Sessions(store).today(wallClock),
      frogsToday: frogs,
    );
    final words = _Words(
      t,
      locale,
      store,
      read.system,
      NumberFormat.decimalPattern(locale).symbols.DECIMAL_SEP,
    );
    final unsorted = Notes(store).unsorted().length;
    return AgendaSnapshot(
      days: {
        '${read.today}': [
          ...words.day(read.today, read.todayRows, read.shoppingWay),
          if (frogs.visible) ...words.frogs(read.frogs),
        ],
        '${read.tomorrow}': words.day(
          read.tomorrow,
          read.tomorrowRows,
          read.shoppingWay,
        ),
      },
      missed: words.missed(Alarms(store).missed(wallClock)),
      unsorted: unsorted,
      calendars: store.agendaCalendarOn
          ? ([...store.agendaCalendars]..sort())
          : const [],
      today: t.agendaToday,
      empty: t.widgetNothingToday,
      unsortedText: t.unsortedBanner(unsorted),
      done: t.markDone,
      allDay: t.allDay,
      noTitle: t.appointmentNoTitle,
      since: t.appointmentSince('{time}'),
      until: t.appointmentUntil('{time}'),
    );
  }

  Map<String, Object?> toMap() => {
    'v': version,
    'days': {
      for (final MapEntry(:key, :value) in days.entries)
        key: [for (final row in value) row.toMap()],
    },
    'missed': [for (final row in missed) row.toMap()],
    'unsorted': unsorted,
    'calendars': calendars,
    'labels': {
      'today': today,
      'empty': empty,
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

  _Words(this.t, this.locale, this.store, this.system, this.decimal);

  /// The first line of [note]'s text, "Photo" for a photo Note without
  /// words.
  String _firstLine(Note note) =>
      noteText(t, note).trim().split('\n').first.trim();

  /// The rows of [day] in Agenda order.
  List<WidgetRow> day(Day day, List<DayEntry> entries, ShoppingAgenda way) => [
    for (final entry in entries)
      if (entry.row case final placed?)
        ..._occurrence(day, placed)
      else if (entry.shopping case final shopping?)
        _shopping(shopping, way)
      else if (entry.checklist case final checklist?)
        _checklist(checklist),
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
        kind: WidgetRow.occurrence,
        title: masked
            ? maskedText(t, locale, row.routine, row.occurrence)
            : note == null
            ? t.noteGone
            : _firstLine(note),
        time: occurrenceTime(t, locale, row.occurrence),
        done: row.done != null,
        checks: !complete,
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
        else
          for (final (i, pick) in picks.today.picks.indexed)
            WidgetRow(
              kind: WidgetRow.text,
              title: picks.masked[i]
                  ? t.maskedTally
                  : pickText(t, pick, system, decimal),
              done: pick.done,
              indent: 1,
              block: block,
              at: at,
            )
      else if (!masked && lines != null)
        for (final line in lines.list.lines)
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

  WidgetRow _shopping(ShoppingRow shopping, ShoppingAgenda way) {
    final (:note, :masked, :line, :count, :checks, area: _) = shopping;
    final title = _listTitle(note, t.shoppingHeading);
    final shown = masked ? t.maskedShoppingList : title;
    return WidgetRow(
      kind: WidgetRow.shopping,
      title: switch (line) {
        final line? => '${line.text} · $title',
        null when way == ShoppingAgenda.open => t.shoppingRowOpen(shown, count),
        null => t.shoppingRowBack(shown, count),
      },
      checks: line != null && checks,
      lineId: line?.id,
      block: WidgetRow.lists,
    );
  }

  WidgetRow _checklist(ChecklistRow row) {
    final (:note, :masked, :list, area: _) = row;
    final title = masked
        ? t.maskedChecklist
        : _listTitle(note, t.checklistHeading);
    return WidgetRow(
      kind: WidgetRow.checklist,
      title: t.checklistRowOpen(title, list.openCount, list.count),
      block: WidgetRow.lists,
    );
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
