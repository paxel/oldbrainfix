import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'alarm_sync.dart';
import 'sync_platform.dart';

/// How an automatic sync with a paired Device is run; the app passes
/// [quickPairedSync], tests a fake.
typedef PairedJoin = Future<SyncResult> Function(
  Store store,
  PairedDevice device, {
  required String host,
  required int port,
  required String name,
});

/// The core's [pairedSync], giving up soon on a Device that does not
/// answer: it is tried again later anyway.
Future<SyncResult> quickPairedSync(
  Store store,
  PairedDevice device, {
  required String host,
  required int port,
  required String name,
}) => pairedSync(
  store,
  device,
  host: host,
  port: port,
  name: name,
  timeout: const Duration(seconds: 5),
);

/// Automatic sync while the app is open (ADR 0003): with every paired
/// Device that syncs on its own, at once when the app shows, when the
/// periodic work comes due meanwhile and when a Device shows up on the
/// network, 5 s after the last change made here, and every minute while
/// it stays open ([AutoSyncSchedule]). One sync at a time; failures stay
/// quiet, the next try comes anyway. Tells its listeners when a sync
/// brought Conflicts, or a background sync did while the app was
/// closed, for the announcement at the top.
class AutoSync extends ChangeNotifier {
  final Store store;
  final SyncPlatform platform;
  final PairedJoin join;

  /// The name this Device goes by in a sync.
  final Future<String> Function() deviceName;

  /// The wall clock the schedule runs on.
  final DateTime Function() clock;

  final _schedule = AutoSyncSchedule();

  /// Where discovery last heard each Device.
  final _found = <String, FoundHost>{};
  final _subscriptions = <StreamSubscription<void>>[];
  Timer? _timer;
  var _running = false;
  var _open = false;
  var _disposed = false;

  /// This Device's own entries as of the last look: only a change of
  /// its own sends a sync, not the entries a sync brought in.
  var _own = 0;

  /// Conflicts brought since the announcement last took them.
  var _conflicts = 0;

  AutoSync({
    required this.store,
    required this.platform,
    required this.deviceName,
    this.join = quickPairedSync,
    this.clock = DateTime.now,
  }) {
    _own = _ownSeq();
    store.addListener(_changed);
    _subscriptions
      ..add(platform.found.listen(_hostFound))
      ..add(platform.wanted.listen((_) => _wantAll()));
  }

  int _ownSeq() => store.versionVector()[store.deviceId] ?? 0;

  List<String> get _devices => [
    for (final device in store.autoSyncDevices()) device.id,
  ];

  /// The app shows: discovery runs, every Device is synced at once, and
  /// what a background sync brought is announced.
  void shown() {
    if (_disposed) return;
    _open = true;
    devicesChanged();
    unawaited(platform.startDiscovery());
    if (store.conflictsArrived) {
      store.conflictsArrived = false;
      _conflicts++;
      notifyListeners();
    }
    _wantAll();
  }

  /// The app leaves: what changed goes out now, then everything rests
  /// until the periodic work or the next [shown].
  void hidden() {
    if (_disposed) return;
    _open = false;
    _timer?.cancel();
    unawaited(platform.stopDiscovery());
    _schedule.flush(clock());
    unawaited(_run());
  }

  /// The paired Devices or their switches changed: the periodic work runs
  /// while one syncs on its own.
  void devicesChanged() {
    if (_disposed) return;
    unawaited(platform.setBackground(_devices.isNotEmpty));
    _arm();
  }

  /// How many Conflicts syncs brought since last asked; taking them
  /// clears them.
  int takeConflicts() {
    final count = _conflicts;
    _conflicts = 0;
    return count;
  }

  void _wantAll() {
    if (_disposed) return;
    _schedule.wantAll(_devices, clock());
    unawaited(_run());
  }

  void _hostFound(FoundHost host) {
    if (_disposed || store.pairedDevice(host.device) == null) return;
    _found[host.device] = host;
    _schedule.found(host.device, clock());
    unawaited(_run());
  }

  void _changed() {
    if (_disposed) return;
    final own = _ownSeq();
    if (own <= _own) return;
    _own = own;
    _schedule.changed(clock());
    _arm();
  }

  /// Sets the timer for the next Device due, while the app shows.
  void _arm() {
    _timer?.cancel();
    if (!_open || _disposed || _running) return;
    final next = _schedule.next(_devices);
    if (next == null) return;
    final wait = next.difference(clock());
    _timer = Timer(wait.isNegative ? Duration.zero : wait, () {
      unawaited(_run());
    });
  }

  Future<void> _run() async {
    if (_running || _disposed) return;
    _running = true;
    try {
      while (!_disposed) {
        final devices = store.autoSyncDevices();
        final due = _schedule.take([for (final d in devices) d.id], clock());
        if (due.isEmpty) break;
        final name = await deviceName();
        for (final device in devices) {
          if (_disposed) return;
          if (!due.contains(device.id)) continue;
          final found = _found[device.id];
          final host = found?.host ?? device.host;
          final port = found?.port ?? device.port;
          if (host == null || port == null) continue;
          try {
            final result = await join(
              store,
              device,
              host: host,
              port: port,
              name: name,
            );
            _schedule.succeeded(device.id, clock());
            if (result.session.conflicts.isNotEmpty && !_disposed) {
              _conflicts += result.session.conflicts.length;
              notifyListeners();
            }
          } on Object {
            // Not there, or not now: tried again on the next occasion.
          }
        }
      }
    } finally {
      _running = false;
      _arm();
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _timer?.cancel();
    store.removeListener(_changed);
    for (final subscription in _subscriptions) {
      unawaited(subscription.cancel());
    }
    super.dispose();
  }
}

/// One background sync, as the platform's periodic work runs it while
/// the app is closed: what was tapped on notifications meanwhile is
/// applied first, so a Done here goes out with it; every paired Device
/// that syncs on its own is looked for for [look] and synced where it
/// answers; Conflicts it brought are kept to be announced when the app
/// opens; and the alarms are armed anew, so a Done on another Device
/// silences the alarm here. [alarms] has its texts already.
Future<void> backgroundSync({
  required Store store,
  required SyncPlatform platform,
  required AlarmSync alarms,
  required String name,
  PairedJoin join = quickPairedSync,
  Duration look = const Duration(seconds: 8),
}) async {
  await alarms.refresh();
  final devices = store.autoSyncDevices();
  if (devices.isEmpty) return;
  final found = {for (final f in await platform.discover(look)) f.device: f};
  for (final device in devices) {
    final host = found[device.id]?.host ?? device.host;
    final port = found[device.id]?.port ?? device.port;
    if (host == null || port == null) continue;
    try {
      final result = await join(
        store,
        device,
        host: host,
        port: port,
        name: name,
      );
      if (result.session.conflicts.isNotEmpty) store.conflictsArrived = true;
    } on Object {
      // Not there now: the next run tries again.
    }
  }
  await alarms.refresh();
}

/// Hands the app's [AutoSync] down to the Sync page.
class AutoSyncScope extends InheritedWidget {
  final AutoSync sync;

  const AutoSyncScope({super.key, required this.sync, required super.child});

  /// The app's [AutoSync], or null where there is none; read once, so it
  /// does not listen.
  static AutoSync? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<AutoSyncScope>()?.sync;

  @override
  bool updateShouldNotify(AutoSyncScope oldWidget) => sync != oldWidget.sync;
}
