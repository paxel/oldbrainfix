import 'dart:async';
import 'dart:isolate';

import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'auto_sync.dart';
import 'folder_platform.dart';

/// Where the automatic sync's jobs run (#225): a round through the
/// shared folder, a join of a paired Device and a paired Device's join
/// of this one's host. The app runs them in an
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

  /// [request], which a paired Device sent this Device's paired host,
  /// answered by [store] going by [name], its `/sync` merging what it
  /// brought, as [LanServing.servePaired].
  Future<(HostResponse, SyncSession?)> host(
    Store store,
    HostRequest request, {
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

  @override
  Future<(HostResponse, SyncSession?)> host(
    Store store,
    HostRequest request, {
    required String name,
  }) async => store.servePaired(request, name: name);
}

/// What a job in an isolate of its own brought back: its result
/// ([_IsolatedValue]) or what it threw ([_IsolatedError]), and whether it
/// changed the log or the Profiles.
sealed class _IsolatedRun<T> {
  final bool log;
  final bool profile;

  const _IsolatedRun({required this.log, required this.profile});

  /// The job's result, or what it threw thrown again with the trace it
  /// was thrown at in the isolate.
  T get result => switch (this) {
    _IsolatedValue<T>(:final value) => value,
    _IsolatedError<T>(:final error, :final trace) => Error.throwWithStackTrace(
      error,
      trace,
    ),
  };
}

/// A job in an isolate that brought back [value].
final class _IsolatedValue<T> extends _IsolatedRun<T> {
  final T value;

  const _IsolatedValue(
    this.value, {
    required super.log,
    required super.profile,
  });
}

/// A job in an isolate that threw [error] at [trace].
final class _IsolatedError<T> extends _IsolatedRun<T> {
  final Object error;
  final StackTrace trace;

  const _IsolatedError(
    this.error,
    this.trace, {
    required super.log,
    required super.profile,
  });
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

  @override
  Future<(HostResponse, SyncSession?)> host(
    Store store,
    HostRequest request, {
    required String name,
  }) => _host(store, path, token, request, name);
}

// Static, so the closures that cross into the isolate hold only what
// they are handed.

Future<FolderSyncResult?> _folder(
  Store store,
  String path,
  RootIsolateToken? token,
  SyncFolder Function(String setting) open,
  String setting,
) => runIsolated(
  store,
  path,
  (s) => folderRound(s, open(setting)),
  token: token,
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
) => runIsolated(
  store,
  path,
  (s) => joining(s, device, host: host, port: port, name: name),
  token: token,
);

/// The request crosses as its parts: its answer waits here.
Future<(HostResponse, SyncSession?)> _host(
  Store store,
  String path,
  RootIsolateToken? token,
  HostRequest request,
  String name,
) {
  final HostRequest(:method, path: at, :headers, :body, :from, :challenged) =
      request;
  return runIsolated(
    store,
    path,
    (s) => s.servePaired(
      HostRequest(
        method: method,
        path: at,
        headers: headers,
        body: body,
        from: from,
        challenged: challenged,
      ),
      name: name,
    ),
    token: token,
  );
}

/// Runs [work] in an isolate of its own on a store of its own on [path]
/// (SQLite WAL), as the sync does (#225), and hands back its result or
/// throws what it threw. Whatever it wrote there, [store], the one the
/// app reads, hears of afterwards ([Store.changedElsewhere]), as after
/// a change of its own: here and nowhere else, so no caller can forget
/// it, and also when the job threw. When the isolate itself failed,
/// nobody knows what it wrote, and [store] hears of everything. [work]
/// crosses into the isolate: a top-level function, or a closure holding
/// only what may.
Future<T> runIsolated<T>(
  Store store,
  String path,
  FutureOr<T> Function(Store store) work, {
  RootIsolateToken? token,
}) async {
  final _IsolatedRun<T> out;
  try {
    out = await Isolate.run(() => _inIsolate(path, token, work));
  } on Object {
    store.changedElsewhere(profile: true);
    rethrow;
  }
  if (out.log || out.profile) store.changedElsewhere(profile: out.profile);
  return out.result;
}

/// Inside the isolate: [work] on a store of its own on [path], closed
/// again afterwards.
Future<_IsolatedRun<T>> _inIsolate<T>(
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
    return _IsolatedValue(value, log: log, profile: profile);
  } on Object catch (e, trace) {
    return _IsolatedError(e, trace, log: log, profile: profile);
  } finally {
    store.close();
  }
}
