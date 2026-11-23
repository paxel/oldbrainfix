import 'dart:async';
import 'dart:isolate';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'folder_platform.dart';
import 'sync_runner.dart';

/// How far a long job is: [done] of [of] (obf-1061, obf-1093).
typedef JobProgress = void Function(int done, int of);

/// A long job, such as an Import (obf-1061) or deleting an Area
/// (obf-1093): it reads what it needs through [folders] and writes into
/// [store], telling [progress] how far it is. It crosses into an
/// isolate: a closure made by a top-level function, holding only the
/// strings it was handed.
typedef LongWork<T> = Future<T> Function(
  Store store,
  FolderPlatform folders,
  JobProgress progress,
);

/// One Import as a job (obf-1061), telling how far the writing is.
typedef ImportWork = LongWork<ImportSummary>;

/// Where a long job runs (obf-1061): the app runs it in an isolate of
/// its own ([IsolateJobRunner]), so it stays usable and draws on while
/// thousands of entries are written; tests on the store they read
/// ([InlineJobRunner]).
abstract interface class JobRunner {
  /// [work] on [store], or on a connection of its own to its database,
  /// [progress] told here as it goes.
  Future<T> run<T>(Store store, LongWork<T> work, JobProgress progress);
}

/// Runs a long job on the store the app reads, in its isolate, reading
/// through [folders].
class InlineJobRunner implements JobRunner {
  final FolderPlatform folders;

  const InlineJobRunner(this.folders);

  @override
  Future<T> run<T>(Store store, LongWork<T> work, JobProgress progress) =>
      work(store, folders, progress);
}

/// Runs a long job in an isolate of its own on a connection of its own
/// to the database at [path] (SQLite WAL), as the sync does (#225),
/// reading through [folders] there: a platform without state, such as
/// [AndroidFolders], whose channel the isolate reaches with [token]. The
/// store the app reads hears of what it wrote afterwards
/// ([runIsolated]); how far it is comes back as it goes.
class IsolateJobRunner implements JobRunner {
  final String path;
  final FolderPlatform folders;
  final RootIsolateToken? token;

  const IsolateJobRunner({
    required this.path,
    required this.folders,
    this.token,
  });

  @override
  Future<T> run<T>(Store store, LongWork<T> work, JobProgress progress) async {
    final told = ReceivePort();
    told.listen((message) {
      if (message case (final int done, final int of)) {
        progress(done, of);
      }
    });
    try {
      return await _isolated(store, path, folders, token, work, told.sendPort);
    } finally {
      told.close();
    }
  }
}

// Static, so the closure that crosses into the isolate holds only what
// it is handed, not what tells the page here.
Future<T> _isolated<T>(
  Store store,
  String path,
  FolderPlatform folders,
  RootIsolateToken? token,
  LongWork<T> work,
  SendPort told,
) => runIsolated(
  store,
  path,
  (s) => work(s, folders, (done, of) => told.send((done, of))),
  token: token,
);

/// The Imports of Settings: a folder of files, or Google Keep's export.
enum ImportKind { folder, keep }

/// How an Import ended, as its page shows it: [text], and the new [area]
/// to open when Notes came in.
class ImportEnd {
  final String text;
  final Area? area;

  const ImportEnd(this.text, {this.area});
}

/// One Import of [kind] as its page shows it (obf-1061): reading while
/// [of] is null, then [written] of [of] Notes in, then its [end].
class ImportJob extends ChangeNotifier {
  final ImportKind kind;

  ImportJob(this.kind);

  var _written = 0;
  int? _of;
  ImportEnd? _end;

  int get written => _written;
  int? get of => _of;
  ImportEnd? get end => _end;

  void _progress(int written, int of) {
    _written = written;
    _of = of;
    notifyListeners();
  }

  /// The Import ended as [end] says.
  void ended(ImportEnd end) {
    _end = end;
    notifyListeners();
  }
}

/// The app's Imports, one at a time, run by [runner] on [store];
/// [running] holds the one that runs, which the page bar's line shows
/// (UI laws, "The sync line").
class ImportJobs {
  final Store store;
  final JobRunner runner;

  /// The Import running now; null while none is.
  final running = ValueNotifier<ImportJob?>(null);
  var _disposed = false;

  ImportJobs({required this.store, required this.runner});

  /// Runs [work] as [job], its count told to it as it goes.
  Future<ImportSummary> run(ImportJob job, ImportWork work) async {
    running.value = job;
    try {
      return await runner.run(store, work, job._progress);
    } finally {
      if (!_disposed && identical(running.value, job)) running.value = null;
    }
  }

  void dispose() {
    _disposed = true;
    running.dispose();
  }
}

/// Hands the app's [ImportJobs] down to Settings and to the page bar's
/// line.
class ImportJobsScope extends InheritedWidget {
  final ImportJobs jobs;

  const ImportJobsScope({super.key, required this.jobs, required super.child});

  /// The app's [ImportJobs], or null where there are none; read once, so
  /// it does not listen.
  static ImportJobs? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<ImportJobsScope>()?.jobs;

  @override
  bool updateShouldNotify(ImportJobsScope oldWidget) => jobs != oldWidget.jobs;
}
