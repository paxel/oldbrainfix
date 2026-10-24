import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'l10n.dart';
import 'schedule_text.dart';

/// One Occurrence as the home-screen Agenda widget shows it.
class WidgetRow {
  final String routineId;

  /// The [Occurrence.key], with [routineId] what a Done tapped in the
  /// widget is queued for.
  final String occurrenceKey;
  final String title;

  /// Its time, or "All day".
  final String time;
  final bool done;

  const WidgetRow({
    required this.routineId,
    required this.occurrenceKey,
    required this.title,
    required this.time,
    required this.done,
  });

  Map<String, Object?> toMap() => {
    'routine': routineId,
    'occurrence': occurrenceKey,
    'title': title,
    'time': time,
    'done': done,
  };
}

/// What the home-screen Agenda widget shows, handed to the platform
/// whenever the alarms are synced: today's Occurrences in Agenda order
/// and tomorrow's as well, so the widget moves on to the new day at
/// midnight without the app; the Unsorted count; and the texts, in the
/// reader's language.
class AgendaSnapshot {
  /// The days, `YYYY-MM-DD`, with their rows: today and tomorrow.
  final Map<String, List<WidgetRow>> days;
  final int unsorted;

  /// The heading, "Today".
  final String today;

  /// Shown when the day has no Occurrence.
  final String empty;

  /// "x Unsorted", with the number in.
  final String unsortedText;

  /// "and {count} more", the widget puts in the number.
  final String more;

  /// What the checkbox says to a screen reader.
  final String done;

  const AgendaSnapshot({
    required this.days,
    required this.unsorted,
    required this.today,
    required this.empty,
    required this.unsortedText,
    required this.more,
    required this.done,
  });

  /// The snapshot of [store] now, with days as [wallClock] shows them.
  factory AgendaSnapshot.of(
    Store store,
    WallClock wallClock,
    AppLocalizations t,
    String locale,
  ) {
    final agenda = Routines(store).agenda(wallClock);
    final unsorted = Notes(store).unsorted().length;
    List<WidgetRow> rows(List<AgendaRow> rows) => [
      for (final row in rows)
        WidgetRow(
          routineId: row.routine.id,
          occurrenceKey: row.occurrence.key,
          title: _title(t, row),
          time: occurrenceTime(t, locale, row.occurrence),
          done: row.done != null,
        ),
    ];
    return AgendaSnapshot(
      days: {
        '${agenda.today}': rows(agenda.todayRows),
        '${agenda.tomorrow}': rows(agenda.tomorrowRows),
      },
      unsorted: unsorted,
      today: t.agendaToday,
      empty: t.widgetNothingToday,
      unsortedText: t.unsortedBanner(unsorted),
      more: t.widgetMore('{count}'),
      done: t.markDone,
    );
  }

  /// The row's title: the first line of its Note, "Photo" for a photo
  /// Note without words.
  ///
  /// Masked (#26): an Occurrence from a locked Area set to Masked must
  /// not show its title here either, the home screen being as public as
  /// the lock screen; once Area Locks exist, its masked text goes here.
  static String _title(AppLocalizations t, AgendaRow row) {
    final note = row.note;
    if (note == null) return t.noteGone;
    return noteText(t, note).trim().split('\n').first.trim();
  }

  Map<String, Object?> toMap() => {
    'days': {
      for (final MapEntry(:key, :value) in days.entries)
        key: [for (final row in value) row.toMap()],
    },
    'unsorted': unsorted,
    'labels': {
      'today': today,
      'empty': empty,
      'unsorted': unsortedText,
      'more': more,
      'done': done,
    },
  };
}
