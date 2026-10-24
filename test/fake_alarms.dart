import 'dart:async';

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

  @override
  Future<void> arm(
    List<ArmedAlarm> alarms,
    AlarmLabels labels, {
    List<(String, Occurrence)> done = const [],
  }) async {
    armed = alarms;
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

  @override
  Future<Map<AlarmPermission, bool>> permissions() async => Map.of(granted);

  @override
  Future<void> request(AlarmPermission permission) async {
    requested.add(permission);
    if (grantOnRequest) granted[permission] = true;
  }
}
