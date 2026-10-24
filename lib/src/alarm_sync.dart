import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'alarm_platform.dart';
import 'schedule_text.dart';

/// Keeps the platform's alarms in step with the core's alarm plan: on
/// start, on every change to the log, and whenever the app comes back.
/// Before each full sync it applies what was tapped on notifications
/// meanwhile, so a Done reaches the log even when it was tapped while
/// the app was not running. Tells its listeners when taps were applied
/// or the granted permissions changed.
class AlarmSync extends ChangeNotifier {
  /// The permissions whose absence the red banner shows. The full-screen
  /// permission only matters for Escalation (#12); the setup asks for
  /// it, the banner does not nag about it yet.
  static const needed = [
    AlarmPermission.notifications,
    AlarmPermission.exactAlarms,
  ];

  final Store store;
  final AlarmPlatform platform;
  final WallClock wallClock;

  AppLocalizations? _t;
  String _locale = '';
  Map<AlarmPermission, bool> _granted = const {};
  StreamSubscription<void>? _arrivals;
  Future<void> _tail = Future.value();
  var _changeQueued = false;
  var _disposed = false;

  /// What was handed to the platform last, to skip handing it again.
  String? _lastArmed;

  AlarmSync({
    required this.store,
    required this.platform,
    required this.wallClock,
  }) {
    store.addListener(_changed);
    _arrivals = platform.actionsArrived.listen((_) => refresh());
  }

  /// The texts the notifications show; the first ones start the sync.
  void setTexts(AppLocalizations t, String locale) {
    final first = _t == null;
    final changed = t != _t || locale != _locale;
    _t = t;
    _locale = locale;
    if (first) {
      unawaited(refresh());
    } else if (changed) {
      _changed();
    }
  }

  /// Whether [permission] is granted; unknown counts as granted until
  /// the platform has answered.
  bool granted(AlarmPermission permission) => _granted[permission] ?? true;

  /// The [needed] permissions missing, for the red banner; none where
  /// there is no alarm layer.
  List<AlarmPermission> get missing => [
    if (platform.available)
      for (final p in needed)
        if (!granted(p)) p,
  ];

  /// Applies the taps waiting on the platform, reads the permissions and
  /// arms the plan anew.
  Future<void> refresh() => _serial(() async {
    if (_disposed) return;
    final actions = await platform.pendingActions();
    final alarms = Alarms(store);
    for (final action in actions) {
      switch (action) {
        case DoneTapped(:final routineId, :final occurrenceKey, :final at):
          alarms.applyDone(routineId, occurrenceKey, at);
        case SnoozeTapped(:final routineId, :final occurrenceKey, :final until):
          alarms.applySnooze(routineId, occurrenceKey, until);
      }
    }
    final granted = await platform.permissions();
    if (_disposed) return;
    final permissionsChanged = !mapEquals(granted, _granted);
    _granted = granted;
    await _arm(force: true);
    // Acknowledged only once applied and armed: a crash in between
    // applies them again, which changes nothing.
    if (actions.isNotEmpty) {
      await platform.acknowledge(
        actions.map((a) => a.seq).reduce((a, b) => a > b ? a : b),
      );
    }
    if (!_disposed && (actions.isNotEmpty || permissionsChanged)) {
      notifyListeners();
    }
  });

  /// Asks for [permission]; the answer arrives with the next [refresh]
  /// when the app comes back.
  Future<void> request(AlarmPermission permission) async {
    await platform.request(permission);
    await refresh();
  }

  void _changed() {
    if (_changeQueued) return;
    _changeQueued = true;
    scheduleMicrotask(() {
      _changeQueued = false;
      if (!_disposed) unawaited(_serial(_arm));
    });
  }

  Future<void> _arm({bool force = false}) async {
    final t = _t;
    if (t == null || _disposed) return;
    final armed = [
      for (final alarm in Alarms(store).plan(wallClock))
        ArmedAlarm(
          alarm: alarm,
          title: _title(alarm.text) ?? t.noteGone,
          body: occurrenceTime(t, _locale, alarm.occurrence),
        ),
    ];
    final done = Alarms(store).recentlyDone(wallClock);
    final signature = [
      for (final a in armed) '${a.alarm}|${a.title}|${a.body}',
      for (final (routine, occurrence) in done) 'done $routine@$occurrence',
    ].join('\n');
    if (!force && signature == _lastArmed) return;
    _lastArmed = signature;
    await platform.arm(
      armed,
      AlarmLabels(
        channel: t.alarmChannel,
        done: t.markDone,
        snooze: t.snooze,
        snoozeQuestion: t.snoozeQuestion,
        snooze10: t.snooze10,
        snooze30: t.snooze30,
        snooze60: t.snooze60,
      ),
      done: done,
    );
  }

  /// The first line of a Note's text, or null when there is none.
  static String? _title(String? text) {
    final line = text?.trim().split('\n').first.trim();
    return line == null || line.isEmpty ? null : line;
  }

  /// Runs [job] after the jobs before it, one at a time.
  Future<void> _serial(Future<void> Function() job) {
    final next = _tail.then((_) => job());
    _tail = next.then<void>((_) {}, onError: (Object _) {});
    return next;
  }

  @override
  void dispose() {
    _disposed = true;
    store.removeListener(_changed);
    unawaited(_arrivals?.cancel());
    super.dispose();
  }
}
