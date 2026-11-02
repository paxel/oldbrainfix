import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'l10n.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';
import 'tally_view.dart';

/// What the compact view of a card says under the Note's name of its
/// Tally or of the Session it runs (#189), read once per list: a Tally's
/// fields shown, each with its total today and its target ("Pages 30 / 50
/// pages"); else for a Note that runs a Session its next run not Done and
/// how many runs were Done and Missed this week, weeks from [weekStart].
/// Nothing for a Note that is neither.
class CardGlances {
  final Store store;
  final AppLocalizations t;
  final String locale;

  /// The decimal sign of the app's language.
  final String decimal;

  /// The day weeks start on, `DateTime.monday` to `DateTime.sunday`.
  final int weekStart;

  CardGlances(
    this.store, {
    required this.t,
    required this.locale,
    required this.decimal,
    required this.weekStart,
  });

  /// The glances as [context] shows them: in its language, weeks from the
  /// day its locale starts them on.
  factory CardGlances.inContext(BuildContext context, Store store) {
    final first = MaterialLocalizations.of(context).firstDayOfWeekIndex;
    return CardGlances(
      store,
      t: context.t,
      locale: Localizations.localeOf(context).toString(),
      decimal: decimalSign(context),
      weekStart: first == 0 ? DateTime.sunday : first,
    );
  }

  late final _tallies = Tallies(store);
  late final _routines = Routines(store);
  late final _sessions = Sessions(store);
  late final _system = deviceUnitSystem(store);

  /// The Sessions not in the trash, by the Note they show under.
  late final Map<String, List<Routine>> _sessionsByNote = () {
    final byNote = <String, List<Routine>>{};
    for (final routine in _routines.all()) {
      if (!_sessions.isSession(routine.id)) continue;
      final note = _routines.note(routine)?.id;
      if (note != null) byNote.putIfAbsent(note, () => []).add(routine);
    }
    return byNote;
  }();

  /// The lines of the Note [noteId].
  List<String> of(String noteId) {
    if (_tallies.byNote(noteId) case final tally?) {
      return [for (final field in tally.shown) _fieldLine(noteId, field)];
    }
    final sessions = _sessionsByNote[noteId];
    if (sessions == null || sessions.isEmpty) return const [];
    // #191 makes the Session a kind of its own; until then its first.
    final routine = sessions.first;
    final history = _routines.history(
      routine,
      deviceWallClock,
      weekStart: weekStart,
    );
    Occurrence? next;
    for (final occurrence in routine.schedule.next(history.today, 64)) {
      if (!routine.done.containsKey(occurrence.key)) {
        next = occurrence;
        break;
      }
    }
    return [
      if (next != null)
        t.sessionCardNext(
          '${dayText(locale, next.day)} ${occurrenceTime(t, locale, next)}',
        ),
      t.sessionCardWeek(history.thisWeek.done, history.thisWeek.missed),
    ];
  }

  /// [field]'s total today against its target.
  String _fieldLine(String noteId, TallyField field) {
    final today = Day.today(store.clock(), deviceWallClock);
    final bars = _tallies.chart(
      noteId,
      field.id,
      TallyPeriod.day,
      deviceWallClock,
    );
    final last = bars.isEmpty ? null : bars.last;
    final total = last != null && last.start == today ? last.value ?? 0 : 0.0;
    final target = field.target;
    return target == null
        ? t.tallyCardTotalOnly(
            field.name,
            shownValue(total, field.unit, _system, decimal),
          )
        : t.tallyCardTotal(
            field.name,
            typedValue(total, field.unit, _system, decimal),
            shownValue(target, field.unit, _system, decimal),
          );
  }
}
