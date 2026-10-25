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

/// Automatic sync while the app is open (ADR 0003): with every paired
/// Device that syncs on its own and through the shared folder, at once
/// when the app shows, when the periodic work comes due meanwhile and
/// when a Device shows up on the network, 5 s after the last change made
/// here, and every minute while it stays open ([AutoSyncSchedule]). One
/// sync at a time; failures stay quiet, the next try comes anyway. Tells
/// its listeners when a sync brought Conflicts, or a background sync did
/// while the app was closed, for the announcement at the top.
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
    this.folders = const NoFolders(),
    this.clock = DateTime.now,
    this.workDir,
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
/// applied first, so a Done here goes out with it; on a local network
/// every paired Device that syncs on its own is looked for for [look]
/// and synced where it answers; then the shared folder, wherever the
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
  String? workDir,
}) async {
  await alarms.refresh();
  final devices = store.autoSyncDevices();
  if (devices.isNotEmpty && await platform.onLocalNetwork()) {
    await _lanInBackground(store, platform, devices, name, join, look);
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
  List<PairedDevice> devices,
  String name,
  PairedJoin join,
  Duration look,
) async {
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
