import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'alarm_sync.dart';
import 'announcements.dart';
import 'l10n.dart';
import 'routine_editor_screen.dart';

/// Snooze and Not today wherever the app answers an Occurrence (#269):
/// the Agenda row's ⋮ and the Note editor's Due line, as the
/// notification does.
enum OccurrenceAnswer {
  snooze10(10),
  snooze30(30),
  snooze60(60),
  skip(null),
  tomorrow(null);

  /// How long a snooze waits; null for Not today's choices.
  final int? minutes;

  const OccurrenceAnswer(this.minutes);
}

/// Hands the app's [AlarmSync] to the pages that answer an Occurrence,
/// so a snooze made there is armed at once.
class AlarmSyncScope extends InheritedWidget {
  final AlarmSync alarms;

  const AlarmSyncScope({super.key, required this.alarms, required super.child});

  /// The app's [AlarmSync], if there is an app; read once, so it does
  /// not listen.
  static AlarmSync? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<AlarmSyncScope>()?.alarms;

  @override
  bool updateShouldNotify(AlarmSyncScope oldWidget) =>
      alarms != oldWidget.alarms;
}

/// What can be answered for [occurrence] of [routine] now, read when its
/// menu opens: Snooze while it is due, started and not yet Missed, as a
/// snooze rings from now on; Not today where this Device may check it
/// off and it is neither Done nor skipped, Tomorrow only for an
/// Occurrence of today or later.
({bool snooze, bool notToday, bool tomorrow}) occurrenceAnswers(
  Store store,
  Routine routine,
  Occurrence occurrence,
) {
  final open =
      routine.doneAt(occurrence) == null &&
      routine.skippedAt(occurrence) == null;
  final now = store.clock();
  final snooze =
      open &&
      openOccurrences(
        routines: [routine],
        now: now,
        wallClock: deviceWallClock,
        allDayFrom: store.allDayFrom,
      ).any((o) => !o.missed && o.occurrence == occurrence);
  final notToday = open && store.mayCheckOff(routine.id);
  return (
    snooze: snooze,
    notToday: notToday,
    tomorrow: notToday && !(occurrence.day < Day.today(now, deviceWallClock)),
  );
}

/// The menu entries of Snooze, with the snooze choices, and of Not
/// today, with Skip and Tomorrow, each under its heading, as many as
/// [snooze], [notToday] and [tomorrow] allow.
List<PopupMenuEntry<T>> occurrenceAnswerItems<T>(
  AppLocalizations t,
  T Function(OccurrenceAnswer answer) value, {
  required bool snooze,
  required bool notToday,
  required bool tomorrow,
}) {
  PopupMenuItem<T> item(OccurrenceAnswer answer, String label) => PopupMenuItem(
    key: ValueKey('answer-${answer.name}'),
    value: value(answer),
    child: Text(label),
  );
  return [
    if (snooze) ...[
      PopupMenuItem(enabled: false, child: Text(t.snooze)),
      item(OccurrenceAnswer.snooze10, t.snooze10),
      item(OccurrenceAnswer.snooze30, t.snooze30),
      item(OccurrenceAnswer.snooze60, t.snooze60),
    ],
    if (snooze && notToday) const PopupMenuDivider(),
    if (notToday) ...[
      PopupMenuItem(enabled: false, child: Text(t.notToday)),
      item(OccurrenceAnswer.skip, t.skip),
      if (tomorrow) item(OccurrenceAnswer.tomorrow, t.notTodayTomorrow),
    ],
  ];
}

/// Answers [occurrence] of the Routine [routineId] with [answer]: a
/// snooze rings again then, armed at once through [alarms]; Skip and
/// Tomorrow go at once and offer at the top to undo them, which takes
/// the skip back (#268).
void answerOccurrence(
  BuildContext context,
  Routines routines,
  AlarmSync? alarms,
  String routineId,
  Occurrence occurrence,
  OccurrenceAnswer answer,
) {
  final store = routines.store;
  if (answer.minutes case final minutes?) {
    final until = store.clock().add(Duration(minutes: minutes));
    if (alarms != null) {
      alarms.snooze(routineId, occurrence, until);
    } else {
      Alarms(store).applySnooze(routineId, occurrence.key, until);
    }
    return;
  }
  final tomorrow = answer == OccurrenceAnswer.tomorrow;
  if (tomorrow) {
    routines.tomorrow(routineId, occurrence);
  } else {
    routines.skip(routineId, occurrence);
  }
  final t = context.t;
  AnnouncementsScope.of(context)
      ?.offerUndo(tomorrow ? t.undoTomorrow : t.undoSkipped, () {
        // A sync or another answer may have got there first.
        final routine = routines.byId(routineId);
        if (routine == null || routine.skippedAt(occurrence) == null) return;
        if (!store.mayCheckOff(routineId)) return;
        routines.unskip(routineId, occurrence);
      });
}

/// A button of the Note editor's Due line (#269), Snooze or Not today,
/// whose tap opens its choices where the finger is, as its notification
/// replaces its buttons with them.
class AnswerButton extends StatefulWidget {
  final String label;

  /// The choices it opens, as [occurrenceAnswerItems] makes them.
  final List<PopupMenuEntry<OccurrenceAnswer>> Function() items;
  final ValueChanged<OccurrenceAnswer> onAnswer;

  const AnswerButton({
    super.key,
    required this.label,
    required this.items,
    required this.onAnswer,
  });

  @override
  State<AnswerButton> createState() => _AnswerButtonState();
}

class _AnswerButtonState extends State<AnswerButton> {
  /// Where the finger last went down on the button.
  Offset? _finger;

  Future<void> _open() async {
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    final box = context.findRenderObject() as RenderBox?;
    if (overlay == null || box == null) return;
    // Opened without a finger, as by a screen reader: at the button.
    final at = _finger ?? box.localToGlobal(box.size.center(Offset.zero));
    _finger = null;
    final picked = await showMenu<OccurrenceAnswer>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: widget.items(),
    );
    if (!mounted || picked == null) return;
    widget.onAnswer(picked);
  }

  @override
  Widget build(BuildContext context) => Listener(
    onPointerDown: (event) => _finger = event.position,
    child: OutlinedButton(onPressed: _open, child: Text(widget.label)),
  );
}
