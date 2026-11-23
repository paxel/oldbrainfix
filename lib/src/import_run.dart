import 'dart:async';
import 'dart:isolate';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'folder_platform.dart';
import 'sync_runner.dart';

/// One Import as a job (obf-1061): it reads what was picked through
/// [folders] and writes its Notes into [store], telling [progress] how
/// far the writing is. It crosses into an isolate: a closure made by a
/// top-level function, holding only the strings it was handed.
typedef ImportWork = Future<ImportSummary> Function(
  Store store,
  FolderPlatform folders,
  ImportProgress progress,
);

/// Where an Import runs (obf-1061): the app runs it in an isolate of its
/// own ([IsolateImportRunner]), so it stays usable and draws on while
/// thousands of entries are written; tests on the store they read
/// ([InlineImportRunner]).
abstract interface class ImportRunner {
  /// [work] on [store], or on a connection of its own to its database,
  /// [progress] told here as it goes.
  Future<ImportSummary> run(
    Store store,
    ImportWork work,
    ImportProgress progress,
  );
}

/// Runs an Import on the store the app reads, in its isolate, reading
/// through [folders].
class InlineImportRunner implements ImportRunner {
  final FolderPlatform folders;

  const InlineImportRunner(this.folders);

  @override
  Future<ImportSummary> run(
    Store store,
    ImportWork work,
    ImportProgress progress,
  ) => work(store, folders, progress);
}

/// Runs an Import in an isolate of its own on a connection of its own to
/// the database at [path] (SQLite WAL), as the sync does (#225), reading
/// through [folders] there: a platform without state, such as
/// [AndroidFolders], whose channel the isolate reaches with [token]. The
/// store the app reads hears of the new Notes afterwards
/// ([runIsolated]); how far it is comes back as it goes.
class IsolateImportRunner implements ImportRunner {
  final String path;
  final FolderPlatform folders;
  final RootIsolateToken? token;

  const IsolateImportRunner({
    required this.path,
    required this.folders,
    this.token,
  });

  @override
  Future<ImportSummary> run(
    Store store,
    ImportWork work,
    ImportProgress progress,
  ) async {
    final told = ReceivePort();
    told.listen((message) {
      if (message case (final int written, final int of)) {
        progress(written, of);
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
Future<ImportSummary> _isolated(
  Store store,
  String path,
  FolderPlatform folders,
  RootIsolateToken? token,
  ImportWork work,
  SendPort told,
) => runIsolated(
  store,
  path,
  (s) => work(s, folders, (written, of) => told.send((written, of))),
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
  final ImportRunner runner;

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
