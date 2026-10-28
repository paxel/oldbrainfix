import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'alarm_sync.dart';
import 'folder_platform.dart';
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

/// This Device's paired host as automatic sync runs it: where it
/// answers, and the way to stop it. The app wraps the core's
/// [PairedHost], tests a fake.
abstract interface class PairedHosting {
  int get port;
  Future<void> stop();
}

/// How the paired host is started; the app passes [startPairedHosting],
/// tests a fake.
typedef StartPairedHost = Future<PairedHosting> Function(
  Store store, {
  required String name,
  required void Function(SyncSession session) onSession,
});

/// The core's [PairedHost] on every IPv4 address of this Device.
Future<PairedHosting> startPairedHosting(
  Store store, {
  required String name,
  required void Function(SyncSession session) onSession,
}) async => _PairedHosting(
  await PairedHost.start(store, name: name, onSession: onSession),
);

class _PairedHosting implements PairedHosting {
  final PairedHost _host;

  _PairedHosting(this._host);

  @override
  int get port => _host.port;

  @override
  Future<void> stop() => _host.stop();
}

/// How long a sync the periodic work started hosts and looks for paired
/// Devices (ADR 0003): long enough for another phone's run to meet it,
/// well within the ten minutes Android gives the work.
const hostWindow = Duration(minutes: 2);

/// How often, within [hostWindow], the paired Devices not synced yet are
/// tried again.
const retryEvery = Duration(seconds: 15);

/// Automatic sync while the app is open (ADR 0003): with every paired
/// Device that syncs on its own and through the shared folder, at once
/// when the app shows, when the periodic work comes due meanwhile and
/// when a Device shows up on the network, 5 s after the last change made
/// here, and every minute while it stays open ([AutoSyncSchedule]). One
/// sync at a time; failures stay quiet, the next try comes anyway, except
/// one that needs an update of the app here or there: that one is
/// announced once (ADR 0019). While some paired Device syncs on its own,
/// this Device hosts for them too ([host]) and says so on the network,
/// while the app shows and, when the periodic work comes due while the
/// app is out of sight, for [hostWindow]. Tells its listeners when a sync brought
/// Conflicts or needs an update, or a background sync did while the app
/// was closed, for the announcement at the top.
class AutoSync extends ChangeNotifier {
  final Store store;
  final SyncPlatform platform;
  final PairedJoin join;

  /// The shared folder, when one is chosen.
  final FolderPlatform folders;

  /// The shared folder's place among the Devices of the schedule.
  static const folderId = '\u0000folder';

  /// The name this Device goes by in a sync.
  final Future<String> Function() deviceName;

  /// The wall clock the schedule runs on.
  final DateTime Function() clock;

  /// Where a Backup is written before it goes into its folder; without
  /// one, the weekly Backup waits for the background work.
  final String? workDir;

  /// Starts the paired host; without one, this Device only joins.
  final StartPairedHost? host;

  final _schedule = AutoSyncSchedule();

  /// Where discovery last heard each Device.
  final _found = <String, FoundHost>{};
  final _subscriptions = <StreamSubscription<void>>[];
  Timer? _timer;
  var _running = false;
  var _open = false;
  var _disposed = false;

  /// The paired host while it runs, and the changes to it, one at a time.
  PairedHosting? _hosting;
  Future<void> _hostQueue = Future.value();

  /// Whether the periodic work's window runs while the app is out of
  /// sight, and the Devices synced within it.
  var _window = false;
  final _synced = <String>{};

  /// This Device's own entries as of the last look: only a change of
  /// its own sends a sync, not the entries a sync brought in.
  var _own = 0;

  /// Conflicts brought since the announcement last took them.
  var _conflicts = 0;

  /// Syncs that need an update, not announced yet ([UpdateNotices]).
  final _updates = <(String, UpdateNeeded)>[];

  AutoSync({
    required this.store,
    required this.platform,
    required this.deviceName,
    this.join = quickPairedSync,
    this.folders = const NoFolders(),
    this.clock = DateTime.now,
    this.workDir,
    this.host,
  }) {
    _own = _ownSeq();
    store.addListener(_changed);
    _subscriptions
      ..add(platform.found.listen(_hostFound))
      ..add(platform.wanted.listen((_) => _wanted()));
  }

  int _ownSeq() => store.versionVector()[store.deviceId] ?? 0;

  List<String> get _devices => [
    for (final device in store.autoSyncDevices()) device.id,
    if (store.syncFolder != null) folderId,
  ];

  /// Folder syncs one after the other, never two at once.
  Future<void> _folderQueue = Future.value();

  /// Syncs through the shared folder now, after one still running; null
  /// without a folder or when it could not be reached.
  Future<FolderSyncResult?> syncFolder() {
    final setting = store.syncFolder;
    if (setting == null || _disposed) return Future.value();
    final run = _folderQueue.then(
      (_) => folderRound(store, folders.open(setting)),
    );
    _folderQueue = run.then((_) {}, onError: (Object _) {});
    return run.then((result) {
      if (result == null || _disposed) return result;
      _takeUpdates();
      _schedule.succeeded(folderId, clock());
      if (result.conflicts.isNotEmpty) {
        _conflicts += result.conflicts.length;
        notifyListeners();
      }
      return result;
    });
  }

  /// The Backup being written, while one is.
  Future<void>? _backingUp;

  /// Writes the weekly Backup when it is due (ADR 0016): the background
  /// work does not run while the app shows. One at a time: while one is
  /// written, that one is waited for. A failure is kept for the Backups
  /// page and tried again next time.
  Future<void> backUpWhenDue() {
    final work = workDir;
    if (work == null || _disposed) return Future.value();
    if (_backingUp case final running?) return running;
    final run = openAutoBackup(folders)
        .then(
          (auto) =>
              backUpIfDue(store, folders.openBackups, work, autoBackup: auto),
        )
        .then(
          (_) {},
          // Kept as the Backups page's status.
          onError: (Object _) {},
        );
    _backingUp = run;
    return run.whenComplete(() {
      if (identical(_backingUp, run)) _backingUp = null;
    });
  }

  /// The app shows: discovery runs, every Device is synced at once, a
  /// Backup due is written, and what a background sync brought is
  /// announced.
  void shown() {
    if (_disposed) return;
    _open = true;
    devicesChanged();
    unawaited(backUpWhenDue());
    unawaited(platform.startDiscovery());
    if (store.conflictsArrived) {
      store.conflictsArrived = false;
      _conflicts++;
      notifyListeners();
    }
    _takeUpdates();
    _wantAll();
  }

  /// The app leaves: what changed goes out now, then everything rests
  /// until the periodic work or the next [shown].
  void hidden() {
    if (_disposed) return;
    _open = false;
    _timer?.cancel();
    unawaited(platform.stopDiscovery());
    _host();
    _schedule.flush(clock());
    unawaited(_run());
  }

  /// The paired Devices or their switches changed, or the Backups': the
  /// periodic work runs while one syncs on its own or Backups are
  /// written weekly, and needs no network for a folder.
  void devicesChanged() {
    if (_disposed) return;
    final backups = store.backupsScheduled;
    unawaited(
      platform.setBackground(
        _devices.isNotEmpty || backups,
        folder: store.syncFolder != null || backups,
      ),
    );
    _host();
    _arm();
  }

  /// Starts or stops the paired host as wanted now: while the app shows
  /// or the periodic work's window runs, some paired Device syncs on its
  /// own, and this Device is on a local network. A host that does not
  /// start is tried again on the next occasion.
  void _host() {
    if (host == null) return;
    _hostQueue = _hostQueue
        .then((_) => _applyHost())
        .then((_) {}, onError: (Object _) {});
  }

  Future<void> _applyHost() async {
    final start = host;
    final want =
        start != null &&
        !_disposed &&
        (_open || _window) &&
        store.autoSyncWanted &&
        await platform.onLocalNetwork();
    final running = _hosting;
    if (running != null) {
      if (want) return;
      _hosting = null;
      await platform.unadvertise();
      await running.stop();
      return;
    }
    if (!want) return;
    final started = await start(
      store,
      name: await deviceName(),
      onSession: _hostSession,
    );
    if (_disposed) {
      await started.stop();
      return;
    }
    _hosting = started;
    await platform.advertise(device: store.deviceId, port: started.port);
  }

  /// A paired Device synced with this one's host.
  void _hostSession(SyncSession session) {
    if (_disposed) return;
    _synced.add(session.peer);
    _schedule.succeeded(session.peer, clock());
    if (session.conflicts.isNotEmpty) {
      _conflicts += session.conflicts.length;
      notifyListeners();
    }
  }

  /// The periodic work came due. While the app shows, every Device is
  /// synced now; out of sight, this Device also hosts and looks for
  /// [hostWindow], or until every Device that syncs on its own synced,
  /// the work keeping the app running meanwhile. Then the work hears it
  /// is over.
  void _wanted() {
    if (_disposed) return;
    if (_open || _window) {
      _wantAll();
      unawaited(platform.finished());
      return;
    }
    unawaited(_hostWindow());
  }

  Future<void> _hostWindow() async {
    _window = true;
    _synced.clear();
    _host();
    unawaited(platform.startDiscovery());
    _wantAll();
    final until = clock().add(hostWindow);
    var retry = clock().add(retryEvery);
    while (!_disposed && !_open && clock().isBefore(until) && !_allSynced()) {
      await Future<void>.delayed(const Duration(seconds: 1));
      if (_disposed || !clock().isAfter(retry)) continue;
      retry = clock().add(retryEvery);
      _schedule.wantAll(
        _devices.where((d) => d != folderId && !_synced.contains(d)),
        clock(),
      );
      unawaited(_run());
    }
    _window = false;
    if (_disposed) return;
    if (!_open) unawaited(platform.stopDiscovery());
    _host();
    unawaited(platform.finished());
  }

  /// Whether every paired Device that syncs on its own synced within the
  /// window, either way round.
  bool _allSynced() => store.pairedDevices().every(
    (d) => !store.autoSyncOn(d.id) || _synced.contains(d.id),
  );

  /// How many Conflicts syncs brought since last asked; taking them
  /// clears them.
  int takeConflicts() {
    final count = _conflicts;
    _conflicts = 0;
    return count;
  }

  /// The syncs that need an update since last asked, by the Device synced
  /// with or the shared folder; taking them clears them.
  List<(String source, UpdateNeeded where)> takeUpdates() {
    final updates = List.of(_updates);
    _updates.clear();
    return updates;
  }

  /// Takes the update notices not announced yet, and tells the listeners
  /// when there are any.
  void _takeUpdates() {
    if (_disposed) return;
    final fresh = store.takeUpdateNotices();
    if (fresh.isEmpty) return;
    _updates.addAll(fresh);
    notifyListeners();
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
        final due = _schedule.take(_devices, clock());
        if (due.isEmpty) break;
        if (due.contains(folderId)) {
          try {
            await syncFolder();
          } on Object {
            // Not now: tried again on the next occasion.
          }
        }
        if (!devices.any((d) => due.contains(d.id))) continue;
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
            _synced.add(device.id);
            if (result.session.conflicts.isNotEmpty && !_disposed) {
              _conflicts += result.session.conflicts.length;
              notifyListeners();
            }
          } on SyncException catch (e) {
            // An update is needed: announced once, tried again all the same.
            _noteUpdate(store, device.id, e);
            _takeUpdates();
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
    if (_hosting case final running?) {
      _hosting = null;
      unawaited(platform.unadvertise());
      unawaited(running.stop());
    }
    store.removeListener(_changed);
    for (final subscription in _subscriptions) {
      unawaited(subscription.cancel());
    }
    super.dispose();
  }
}

/// One background sync, as the platform's periodic work runs it while
/// the app is closed: what was tapped on notifications meanwhile is
/// applied first, so a Done here goes out with it; on a local network,
/// while some paired Device syncs on its own, this Device hosts for them
/// ([host]) and says so on the network, and every paired Device it can
/// join is looked for for [look] and synced where it answers, again
/// every [retryEvery], for [window] or until every one synced, either
/// way round; then the shared folder, wherever the
/// Device is; Conflicts they brought are kept to be announced when the
/// app opens; and the alarms are armed anew, so a Done on another Device
/// silences the alarm here. A weekly Backup due is written through
/// [workDir] (ADR 0016). [alarms] has its texts already.
Future<void> backgroundSync({
  required Store store,
  required SyncPlatform platform,
  required AlarmSync alarms,
  required String name,
  PairedJoin join = quickPairedSync,
  FolderPlatform folders = const NoFolders(),
  Duration look = const Duration(seconds: 8),
  StartPairedHost? host,
  Duration window = hostWindow,
  DateTime Function() clock = DateTime.now,
  String? workDir,
}) async {
  await alarms.refresh();
  if (store.autoSyncWanted && await platform.onLocalNetwork()) {
    await _lanInBackground(
      store,
      platform,
      name,
      join,
      look,
      host,
      clock().add(window),
      clock,
    );
  }
  if (store.syncFolder case final folder?) {
    try {
      final result = await folderRound(store, folders.open(folder));
      if (result != null && result.conflicts.isNotEmpty) {
        store.conflictsArrived = true;
      }
    } on Object {
      // Not now: the next run tries again.
    }
  }
  if (workDir != null) {
    try {
      await backUpIfDue(
        store,
        folders.openBackups,
        workDir,
        autoBackup: await openAutoBackup(folders),
      );
    } on Object {
      // Kept as the Backups page's status; the next run tries again.
    }
  }
  await alarms.refresh();
}

Future<void> _lanInBackground(
  Store store,
  SyncPlatform platform,
  String name,
  PairedJoin join,
  Duration look,
  StartPairedHost? host,
  DateTime until,
  DateTime Function() clock,
) async {
  final synced = <String>{};
  void took(SyncSession session) {
    if (session.conflicts.isNotEmpty) store.conflictsArrived = true;
  }

  PairedHosting? hosting;
  if (host != null) {
    try {
      hosting = await host(
        store,
        name: name,
        onSession: (session) {
          synced.add(session.peer);
          took(session);
        },
      );
      await platform.advertise(device: store.deviceId, port: hosting.port);
    } on Object {
      // Not hosting this time: joining still works.
    }
  }
  try {
    while (true) {
      final devices = [
        for (final d in store.autoSyncDevices())
          if (!synced.contains(d.id)) d,
      ];
      if (devices.isNotEmpty) {
        final found = {
          for (final f in await platform.discover(look)) f.device: f,
        };
        for (final device in devices) {
          final at = found[device.id]?.host ?? device.host;
          final port = found[device.id]?.port ?? device.port;
          if (at == null || port == null) continue;
          try {
            final result = await join(
              store,
              device,
              host: at,
              port: port,
              name: name,
            );
            synced.add(device.id);
            took(result.session);
          } on SyncException catch (e) {
            // Kept to be announced when the app opens.
            _noteUpdate(store, device.id, e);
          } on Object {
            // Not there now: tried again within the window, or next run.
          }
        }
      }
      final all = store.pairedDevices().every(
        (d) => !store.autoSyncOn(d.id) || synced.contains(d.id),
      );
      if (all || !clock().isBefore(until)) break;
      await Future<void>.delayed(retryEvery);
    }
  } finally {
    if (hosting != null) {
      await platform.unadvertise();
      await hosting.stop();
    }
  }
}

/// Keeps that a sync with [device] needs an update, when [e] says so.
void _noteUpdate(Store store, String device, SyncException e) {
  if (!e.needsUpdate) return;
  store.noteUpdateNeeded(
    device,
    e.failure == SyncFailure.updateHere
        ? UpdateNeeded.here
        : UpdateNeeded.there,
  );
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
