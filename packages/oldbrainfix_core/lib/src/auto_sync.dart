import 'pairing.dart';
import 'store.dart';

/// Automatic sync (ADR 0003): paired Devices find each other on the same
/// network and sync on their own, so nobody has to remember to.

extension AutoSyncDevices on Store {
  static const _conflictsKey = 'sync:conflictsArrived';

  /// The paired Devices this one syncs with on its own: those that host
  /// (their certificate's whole fingerprint is pinned) with the switch
  /// on.
  List<PairedDevice> autoSyncDevices() => [
    for (final device in pairedDevices())
      if (device.fingerprint.length == 64 && autoSyncOn(device.id)) device,
  ];

  /// Whether a sync nobody watched, in the background, brought
  /// Conflicts that were not announced yet.
  bool get conflictsArrived => localSetting(_conflictsKey) != null;

  set conflictsArrived(bool arrived) => arrived
      ? setLocalSetting(_conflictsKey, '1')
      : removeLocalSetting(_conflictsKey);
}

/// When the open app syncs with which paired Device: at once on start,
/// on coming back and when a Device shows up on the network; [debounce]
/// after the last change made here; and every [every] while it stays
/// open, so a Done on another Device arrives soon. A Device that shows
/// up again within [settle] of a sync with it is left alone: discovery
/// repeats itself. Pure; the time is handed in.
class AutoSyncSchedule {
  static const debounce = Duration(seconds: 5);
  static const settle = Duration(minutes: 1);
  static const every = Duration(minutes: 1);

  /// When each Device was asked for.
  final _asked = <String, DateTime>{};

  /// When a sync with each Device was last started, and last succeeded.
  final _tried = <String, DateTime>{};
  final _synced = <String, DateTime>{};

  /// When the changes made here are due to go out to every Device.
  DateTime? _changed;

  /// The app started or came back: every one of [devices] now.
  void wantAll(Iterable<String> devices, DateTime now) {
    for (final id in devices) {
      _asked[id] = now;
    }
  }

  /// Something changed here: every Device [debounce] after the last
  /// change.
  void changed(DateTime now) => _changed = now.add(debounce);

  /// The app is leaving: what changed goes out now, not after the wait.
  void flush(DateTime now) {
    final changed = _changed;
    if (changed != null && changed.isAfter(now)) _changed = now;
  }

  /// [device] showed up on the network.
  void found(String device, DateTime now) {
    final last = _synced[device];
    if (last == null || !now.isBefore(last.add(settle))) _asked[device] = now;
  }

  /// When [device] is due next, or null when nothing asks for it.
  DateTime? dueAt(String device) {
    DateTime? due;
    for (final at in [_asked[device], _changed, _tried[device]?.add(every)]) {
      if (at != null && (due == null || at.isBefore(due))) due = at;
    }
    return due;
  }

  /// The earliest time one of [devices] is due, for the timer.
  DateTime? next(Iterable<String> devices) {
    DateTime? next;
    for (final id in devices) {
      final due = dueAt(id);
      if (due != null && (next == null || due.isBefore(next))) next = due;
    }
    return next;
  }

  /// The ones of [devices] due by [now], each marked as started now.
  List<String> take(Iterable<String> devices, DateTime now) {
    final due = [
      for (final id in devices)
        if (dueAt(id) case final at? when !at.isAfter(now)) id,
    ];
    for (final id in due) {
      _asked.remove(id);
      _tried[id] = now;
    }
    final changed = _changed;
    if (changed != null && !changed.isAfter(now)) _changed = null;
    return due;
  }

  /// The sync with [device] went through.
  void succeeded(String device, DateTime now) => _synced[device] = now;
}
