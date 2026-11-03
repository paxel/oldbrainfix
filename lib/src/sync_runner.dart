import 'dart:async';
import 'dart:isolate';

import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'auto_sync.dart';
import 'folder_platform.dart';

/// Where the automatic sync's two jobs run (#225): a round through the
/// shared folder and a join of a paired Device. The app runs them in an
/// isolate of their own ([IsolateSyncRunner]), tests on the store they
/// read ([InlineSyncRunner]).
abstract interface class SyncRunner {
  /// One round of [store] through the shared folder kept as [setting];
  /// null when it could not be reached, as [folderRound].
  Future<FolderSyncResult?> folder(Store store, String setting);

  /// One sync of [store] with the paired [device] where it answers, as
  /// [PairedJoin].
  Future<SyncResult> join(
    Store store,
    PairedDevice device, {
    required String host,
    required int port,
    required String name,
  });
}

/// Runs the jobs on the store the app reads, in its isolate.
class InlineSyncRunner implements SyncRunner {
  final FolderPlatform folders;
  final PairedJoin joining;

  const InlineSyncRunner({required this.folders, required this.joining});

  @override
  Future<FolderSyncResult?> folder(Store store, String setting) =>
      folderRound(store, folders.open(setting));

  @override
  Future<SyncResult> join(
    Store store,
    PairedDevice device, {
    required String host,
    required int port,
    required String name,
  }) => joining(store, device, host: host, port: port, name: name);
}

/// What a job in an isolate of its own brought back: its result
/// ([IsolatedValue]) or what it threw ([IsolatedError]), and whether it
/// changed the log or the Profiles.
sealed class IsolatedRun<T> {
  final bool log;
  final bool profile;

  const IsolatedRun({required this.log, required this.profile});

  /// The job's result, or what it threw thrown again.
  T get result => switch (this) {
    IsolatedValue<T>(:final value) => value,
    IsolatedError<T>(:final error) => throw error,
  };
}

/// A job in an isolate that brought back [value].
final class IsolatedValue<T> extends IsolatedRun<T> {
  final T value;

  const IsolatedValue(this.value, {required super.log, required super.profile});
}

/// A job in an isolate that threw [error].
final class IsolatedError<T> extends IsolatedRun<T> {
  final Object error;

  const IsolatedError(this.error, {required super.log, required super.profile});
}

/// Runs the jobs in an isolate of their own, each on a connection of its
/// own to the database at [path] (SQLite WAL), so the app keeps drawing
/// while entries merge (#225). The store the app reads hears of what
/// changed afterwards ([Store.changedElsewhere]), as after a sync of its
/// own. [open] makes the shared folder from its setting and [joining]
/// syncs with a paired Device, both inside the isolate: top-level
/// functions, which cross into it. With [token], the isolate reaches the
/// platform's channels, such as the folder's on Android.
class IsolateSyncRunner implements SyncRunner {
  final String path;
  final SyncFolder Function(String setting) open;
  final PairedJoin joining;
  final RootIsolateToken? token;

  const IsolateSyncRunner({
    required this.path,
    required this.open,
    this.joining = quickPairedSync,
    this.token,
  });

  @override
  Future<FolderSyncResult?> folder(Store store, String setting) =>
      _folder(store, path, token, open, setting);

  @override
  Future<SyncResult> join(
    Store store,
    PairedDevice device, {
    required String host,
    required int port,
    required String name,
  }) => _join(store, path, token, joining, device, host, port, name);
}

// Static, so the closures that cross into the isolate hold only what
// they are handed.

Future<FolderSyncResult?> _folder(
  Store store,
  String path,
  RootIsolateToken? token,
  SyncFolder Function(String setting) open,
  String setting,
) => _isolated(
  store,
  () => _inIsolate<FolderSyncResult?>(
    path,
    token,
    (s) => folderRound(s, open(setting)),
  ),
);

Future<SyncResult> _join(
  Store store,
  String path,
  RootIsolateToken? token,
  PairedJoin joining,
  PairedDevice device,
  String host,
  int port,
  String name,
) => _isolated(
  store,
  () => _inIsolate(
    path,
    token,
    (s) => joining(s, device, host: host, port: port, name: name),
  ),
);

/// Runs [ran] in an isolate, tells [store] what it changed, and hands
/// back its result or throws what it threw.
Future<T> _isolated<T>(
  Store store,
  Future<IsolatedRun<T>> Function() ran,
) async {
  final out = await Isolate.run(ran);
  if (out.log || out.profile) store.changedElsewhere(profile: out.profile);
  return out.result;
}

/// Runs [work] in an isolate of its own on a store of its own on [path]
/// (SQLite WAL), as the sync does (#225), and hands back what it brought
/// and changed; the caller tells the store it reads. [work] crosses into
/// the isolate: a top-level function, or a closure holding only what
/// may.
Future<IsolatedRun<T>> runIsolated<T>(
  String path,
  FutureOr<T> Function(Store store) work, {
  RootIsolateToken? token,
}) => Isolate.run(() => _inIsolate(path, token, work));

/// Inside the isolate: [work] on a store of its own on [path], closed
/// again afterwards.
Future<IsolatedRun<T>> _inIsolate<T>(
  String path,
  RootIsolateToken? token,
  FutureOr<T> Function(Store store) work,
) async {
  if (token != null) BackgroundIsolateBinaryMessenger.ensureInitialized(token);
  final store = Store.open(path);
  var log = false;
  var profile = false;
  store
    ..addListener(() => log = true)
    ..addProfileListener(() => profile = true);
  try {
    final value = await work(store);
    return IsolatedValue(value, log: log, profile: profile);
  } on Object catch (e) {
    return IsolatedError(e, log: log, profile: profile);
  } finally {
    store.close();
  }
}
