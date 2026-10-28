import 'dart:async';

import 'package:oldbrainfix/src/agenda_widget.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// An [AlarmPlatform] that records what was armed and answers as told.
class FakeAlarms implements AlarmPlatform {
  /// What was armed last.
  List<ArmedAlarm> armed = [];
  AlarmLabels? labels;

  /// How often the plan was handed over.
  int arms = 0;

  /// Taps not acknowledged yet.
  final pending = <AlarmAction>[];

  /// The seq acknowledged last.
  int? acknowledged;

  Map<AlarmPermission, bool> granted = {
    for (final p in AlarmPermission.values) p: true,
  };

  /// Whether asking grants a permission.
  bool grantOnRequest = true;
  final requested = <AlarmPermission>[];

  final _actions = StreamController<void>.broadcast();

  /// A notification is tapped while the app runs.
  void tap(AlarmAction action) {
    pending.add(action);
    _actions.add(null);
  }

  /// The ids armed last, in order.
  List<String> get ids => [for (final a in armed) a.alarm.id];

  @override
  bool get available => true;

  /// The Occurrences whose notifications were taken away last, as
  /// `routine@occurrence`.
  List<String> done = [];

  /// The Nudge armed last; null when none was.
  ArmedNudge? nudge;

  /// The Changes Nudge armed last; null when none was.
  ArmedChangesNudge? changes;

  /// The sound armed last; null for the app's own.
  String? sound;

  @override
  Future<void> arm(
    List<ArmedAlarm> alarms,
    AlarmLabels labels, {
    List<(String, Occurrence)> done = const [],
    ArmedNudge? nudge,
    ArmedChangesNudge? changes,
    String? sound,
  }) async {
    armed = alarms;
    this.sound = sound;
    this.nudge = nudge;
    this.changes = changes;
    this.labels = labels;
    this.done = [for (final (r, o) in done) '$r@${o.key}'];
    arms++;
  }

  @override
  Future<List<AlarmAction>> pendingActions() async => List.of(pending);

  @override
  Future<void> acknowledge(int seq) async {
    pending.removeWhere((a) => a.seq <= seq);
    acknowledged = seq;
  }

  @override
  Stream<void> get actionsArrived => _actions.stream;

  /// What rings full screen now.
  Ringing? ringingNow;

  /// The answers from the full-screen alarm: null for Done, else the
  /// snooze.
  final answers = <Duration?>[];

  /// A full-screen alarm starts ringing, or stops with null.
  void ring(Ringing? ringing) {
    ringingNow = ringing;
    _actions.add(null);
  }

  @override
  Future<Ringing?> ringing() async => ringingNow;

  /// Answered, it stops ringing and is queued like a tap.
  @override
  Future<void> answerRing(Ringing ring, {Duration? snooze}) async {
    answers.add(snooze);
    ringingNow = null;
    final seq = (pending.isEmpty ? acknowledged ?? 0 : pending.last.seq) + 1;
    final now = DateTime.now().toUtc();
    pending.add(
      snooze == null
          ? DoneTapped(seq, ring.routineId, ring.occurrenceKey, now)
          : SnoozeTapped(
              seq,
              ring.routineId,
              ring.occurrenceKey,
              now,
              now.add(snooze),
            ),
    );
  }

  var _nudgeTapped = false;

  /// The Nudge's notification is tapped while the app runs.
  void tapNudge() {
    _nudgeTapped = true;
    _actions.add(null);
  }

  @override
  Future<bool> nudgeOpened() async {
    final tapped = _nudgeTapped;
    _nudgeTapped = false;
    return tapped;
  }

  var _changesTapped = false;

  /// The Changes Nudge's notification is tapped while the app runs.
  void tapChanges() {
    _changesTapped = true;
    _actions.add(null);
  }

  @override
  Future<bool> changesOpened() async {
    final tapped = _changesTapped;
    _changesTapped = false;
    return tapped;
  }

  /// The Agenda widget's snapshots, in the order they were handed over.
  final agendas = <AgendaSnapshot>[];

  @override
  Future<void> showAgenda(AgendaSnapshot snapshot) async =>
      agendas.add(snapshot);

  var _unsortedTapped = false;

  /// The Agenda widget's Unsorted count is tapped while the app runs.
  void tapUnsorted() {
    _unsortedTapped = true;
    _actions.add(null);
  }

  @override
  Future<bool> unsortedOpened() async {
    final tapped = _unsortedTapped;
    _unsortedTapped = false;
    return tapped;
  }

  @override
  Future<Map<AlarmPermission, bool>> permissions() async => Map.of(granted);

  @override
  Future<void> request(AlarmPermission permission) async {
    requested.add(permission);
    if (grantOnRequest) granted[permission] = true;
  }

  /// What the sound picker answers; null leaves it without a choice.
  PickedSound? soundPicked;

  /// The sounds the picker was opened on, null for the app's own.
  final pickedFrom = <String?>[];

  @override
  Future<PickedSound?> pickSound(String? current, String title) async {
    pickedFrom.add(current);
    return soundPicked;
  }
}
