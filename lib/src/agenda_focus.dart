import 'package:flutter/foundation.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'agenda_rows.dart';
import 'alarm_platform.dart';
import 'missed_summary_card.dart';

/// Where the Agenda shows an Occurrence whose notification was tapped
/// (obf-1039): marked among today's rows, in the Missed Summary or on a
/// coming day; or, listed nowhere, that it was Done or skipped
/// meanwhile, or neither.
enum AgendaFound { today, missed, comingDay, done, skipped, nowhere }

/// The Occurrence the Agenda is to show marked, and what is told once
/// it found where.
typedef AgendaFocusAsk = ({
  OpenedOccurrence occurrence,
  ValueChanged<AgendaFound> found,
});

/// The way an Occurrence's notification tap reaches the Agenda
/// (obf-1039): asked for here, taken by the Agenda once its rows are
/// read, also when it comes up only after the tap.
class AgendaFocus extends ChangeNotifier {
  AgendaFocusAsk? _asked;

  /// Asks the Agenda to show [occurrence] marked; [found] hears where.
  void show(OpenedOccurrence occurrence, ValueChanged<AgendaFound> found) {
    _asked = (occurrence: occurrence, found: found);
    notifyListeners();
  }

  /// What was asked for and not taken yet; taking it clears it.
  AgendaFocusAsk? take() {
    final asked = _asked;
    _asked = null;
    return asked;
  }
}

/// Where the Agenda lists [occurrence]: among [today]'s rows, then in
/// the [missed] Summary as far as its card shows, then on [tomorrow] or
/// a [later] day. Listed nowhere, [routine] as the log has it now says
/// whether it was Done or skipped.
AgendaFound agendaFound(
  OpenedOccurrence occurrence, {
  required List<DayEntry> today,
  required MissedSummary missed,
  required List<DayEntry> tomorrow,
  required List<List<DayEntry>> later,
  required Routine? routine,
}) {
  final (:routineId, :occurrenceKey) = occurrence;
  bool lists(List<DayEntry> rows) => rows.any((entry) {
    final row = entry.row?.row;
    return row != null &&
        row.routine.id == routineId &&
        row.occurrence.key == occurrenceKey;
  });
  if (lists(today)) return AgendaFound.today;
  if (missedShows(missed, occurrence)) return AgendaFound.missed;
  if (lists(tomorrow) || later.any(lists)) return AgendaFound.comingDay;
  if (routine?.done.containsKey(occurrenceKey) ?? false) {
    return AgendaFound.done;
  }
  if (routine?.skipped.containsKey(occurrenceKey) ?? false) {
    return AgendaFound.skipped;
  }
  return AgendaFound.nowhere;
}

/// Whether the Missed Summary's card shows [occurrence] to check off.
bool missedShows(MissedSummary missed, OpenedOccurrence occurrence) =>
    missed.routines.any(
      (r) =>
          r.routine.id == occurrence.routineId &&
          r.missed
              .take(MissedSummaryCard.shown)
              .any((m) => m.occurrence.key == occurrence.occurrenceKey),
    );
