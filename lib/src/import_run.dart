import 'dart:async';
import 'dart:io';
import 'dart:isolate';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'folder_platform.dart';
import 'sync_runner.dart';

/// How far a long job is: [done] of [of] (obf-1061, obf-1093).
typedef JobProgress = void Function(int done, int of);

/// How far one who reads beside the app is, as the page Running shows it
/// (obf-1078): since when it reads without a rest, and how many of how
/// many it took meanwhile, the one being read the next.
typedef ReaderBusy = ({DateTime since, int done, int of});

/// A long job, such as an Import (obf-1061) or deleting an Area
/// (obf-1093): it reads what it needs through [folders] and writes into
/// [store], telling [progress] how far it is, and asks [stop] before
/// each of its steps whether Abort on the page Running was pressed
/// (obf-1078). It crosses into an isolate: a closure made by a top-level
/// function, holding only the strings it was handed.
typedef LongWork<T> = Future<T> Function(
  Store store,
  FolderPlatform folders,
  JobProgress progress,
  Stopping stop,
);

/// The Abort of one long job (obf-1078): a file in the app's temporary
/// folder, named for this job alone, whose being there tells the job to
/// stop before its next step. The job asks it in its own isolate,
/// without waiting, as the core asks before each Note; a port's message
/// would wait behind the work. A file that cannot be written leaves the
/// job running to its end, so nothing is lost either way.
class JobStop {
  /// The file; only its being there counts.
  final String path;

  var _raised = false;

  JobStop._(this.path);

  static var _made = 0;

  /// A stop of its own under the system's temporary folder, the app's
  /// cache on the phone, not raised.
  factory JobStop() => JobStop._(
    '${Directory.systemTemp.path}/obf-stop-$pid-'
    '${DateTime.now().microsecondsSinceEpoch}-${_made++}',
  )..clear();

  /// Whether Abort was pressed for this job.
  bool get raised => _raised;

  /// Tells the job to stop before its next step; where the file cannot
  /// be written, it runs to its end and Abort stays offered.
  void raise() {
    try {
      File(path).writeAsBytesSync(const [], flush: true);
      _raised = true;
    } on FileSystemException {
      // The job runs to its end.
    }
  }

  /// Takes the file away once the job ended, so none is left behind.
  void clear() {
    try {
      File(path).deleteSync();
    } on FileSystemException {
      // Not there.
    }
  }
}

/// Whether the [JobStop] at [path] was raised, asked where the job runs.
bool _stopRaised(String path) => File(path).existsSync();

/// One Import as a job (obf-1061), telling how far the writing is.
typedef ImportWork = LongWork<ImportSummary>;

/// Where a long job runs (obf-1061): the app runs it in an isolate of
/// its own ([IsolateJobRunner]), so it stays usable and draws on while
/// thousands of entries are written; tests on the store they read
/// ([InlineJobRunner]).
abstract interface class JobRunner {
  /// [work] on [store], or on a connection of its own to its database,
  /// [progress] told here as it goes, stopping before its next step once
  /// [stop] is raised.
  Future<T> run<T>(
    Store store,
    LongWork<T> work,
    JobProgress progress,
    JobStop stop,
  );
}

/// Runs a long job on the store the app reads, in its isolate, reading
/// through [folders].
class InlineJobRunner implements JobRunner {
  final FolderPlatform folders;

  const InlineJobRunner(this.folders);

  @override
  Future<T> run<T>(
    Store store,
    LongWork<T> work,
    JobProgress progress,
    JobStop stop,
  ) {
    final path = stop.path;
    return work(store, folders, progress, () => _stopRaised(path));
  }
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
  Future<T> run<T>(
    Store store,
    LongWork<T> work,
    JobProgress progress,
    JobStop stop,
  ) async {
    final told = ReceivePort();
    told.listen((message) {
      if (message case (final int done, final int of)) {
        progress(done, of);
      }
    });
    try {
      return await _isolated(
        store,
        path,
        folders,
        token,
        work,
        told.sendPort,
        stop.path,
      );
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
  String stop,
) => runIsolated(
  store,
  path,
  (s) => work(
    s,
    folders,
    (done, of) => told.send((done, of)),
    () => _stopRaised(stop),
  ),
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
/// [of] is null, then [written] of [of] Notes in, then its [end]. [name]
/// is the folder's or the file's, as the page Running names it
/// (obf-1078).
class ImportJob extends ChangeNotifier {
  final ImportKind kind;
  final String? name;

  ImportJob(this.kind, {this.name});

  var _written = 0;
  int? _of;
  ImportEnd? _end;
  JobStop? _stop;
  ({int done, int of})? _stopped;

  int get written => _written;
  int? get of => _of;
  ImportEnd? get end => _end;

  /// Whether Abort was pressed for it on the page Running (obf-1078).
  bool get aborting => _stop?.raised ?? false;

  /// How many of how many Notes it made, once Abort stopped it before
  /// all of them were in; null otherwise.
  ({int done, int of})? get stopped => _stopped;

  /// Stops it before its next Note; the Notes in stay (obf-1078).
  void abort() {
    final stop = _stop;
    if (stop == null || stop.raised) return;
    stop.raise();
    notifyListeners();
  }

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

  /// Runs [work] as [job], its count told to it as it goes, until it ends
  /// or [ImportJob.abort] stops it.
  Future<ImportSummary> run(ImportJob job, ImportWork work) async {
    final stop = _started(job);
    try {
      return await _write(job, work, stop);
    } finally {
      _ended(job, stop);
    }
  }

  /// Runs [look] as [job], then the work [then] makes of what it found,
  /// as [run] does (obf-1078): the job runs all the while, also while
  /// [then] asks the user. Null when [then] makes none, and then nothing
  /// is written.
  Future<ImportSummary?> runLooking<L>(
    ImportJob job,
    LongWork<L> look,
    Future<ImportWork?> Function(L found) then,
  ) async {
    final stop = _started(job);
    try {
      final found = await runner.run(store, look, job._progress, stop);
      final work = await then(found);
      if (work == null) return null;
      return await _write(job, work, stop);
    } finally {
      _ended(job, stop);
    }
  }

  JobStop _started(ImportJob job) {
    final stop = JobStop();
    job._stop = stop;
    running.value = job;
    return stop;
  }

  Future<ImportSummary> _write(
    ImportJob job,
    ImportWork work,
    JobStop stop,
  ) async {
    final summary = await runner.run(store, work, job._progress, stop);
    if (summary.stopped case final of?) {
      job._stopped = (done: summary.notes, of: of);
    }
    return summary;
  }

  void _ended(ImportJob job, JobStop stop) {
    stop.clear();
    if (!_disposed && identical(running.value, job)) running.value = null;
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
