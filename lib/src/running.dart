import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'app_lock.dart';
import 'area_deletion.dart';
import 'attachment_reader.dart';
import 'auto_sync.dart';
import 'import_run.dart';
import 'l10n.dart';
import 'look.dart';
import 'ocr_platform.dart';
import 'page_bar.dart';
import 'place_names.dart';
import 'schedule_text.dart';
import 'speech_platform.dart';
import 'system_bars.dart';

/// The most jobs stopped the page Running keeps listing, the newest
/// first.
const maxStoppedJobs = 10;

/// What a job of the page Running is (obf-1078).
enum RunningKind {
  /// The automatic syncs of this Device: the shared folder's rounds and
  /// the joins of its paired Devices.
  sync,
  import,
  transcripts,
  photoTexts,
  attachmentTexts,
  speechModel,
  areaDeletion,
}

/// One job running beside the app, as its row on the page Running shows
/// it: what it is, how far it is when that is known ([done] of [of]),
/// since when, and whether Abort was pressed and it is [stopping].
class RunningJob {
  final RunningKind kind;
  final DateTime? since;
  final int? done;
  final int? of;
  final bool stopping;

  /// The Import, for [RunningKind.import].
  final ImportJob? import;

  /// The Area, for [RunningKind.areaDeletion]; null once it is gone.
  final Area? area;

  /// The Area's id, for [RunningKind.areaDeletion].
  final String? areaId;

  /// Whether the speech model is downloaded, not copied from a file.
  final bool downloading;

  const RunningJob(
    this.kind, {
    this.since,
    this.done,
    this.of,
    this.stopping = false,
    this.import,
    this.area,
    this.areaId,
    this.downloading = false,
  });
}

/// One job Abort stopped or paused, as the page Running lists it under
/// "Stopped": how far it came ([done] of [of]) where that counts, and
/// whether it was only paused, so Go on starts it again ([goesOn]).
class StoppedJob {
  final RunningKind kind;
  final int? done;
  final int? of;
  final ImportJob? import;
  final Area? area;
  final bool goesOn;

  const StoppedJob(
    this.kind, {
    this.done,
    this.of,
    this.import,
    this.area,
    this.goesOn = false,
  });
}

/// Every job that runs beside the app, read from where each runs
/// (obf-1078): a sync with a Device, an Import, the Transcripts, Photo
/// Texts and Attachment Texts being read, the speech model being
/// fetched, an Area being deleted. Abort stops a job at its next safe
/// point: an Import or an Area deletion before its next Note, keeping
/// what it did; a sync round or a recording, photo or file under way
/// ends first, then the automatic syncs pause or the others wait, until
/// Go on or the next start; the speech model's fetch is cancelled. What
/// Abort stopped stays listed while the app runs, at most
/// [maxStoppedJobs]. It tells its listeners whenever a job starts, moves
/// on or ends.
class Running extends ChangeNotifier {
  final AutoSync? sync;
  final ImportJobs? imports;
  final AreaDeletions? deletions;
  final Transcriber? transcriber;
  final PhotoReader? photos;
  final AttachmentReader? files;
  final SpeechModel? model;

  /// Where the name of an Area being deleted is read.
  final Places places;

  /// The wall clock "Since 14:32" is read on.
  final DateTime Function() clock;

  var _jobs = const <RunningJob>[];
  final _stopped = <StoppedJob>[];

  DateTime? _syncSince;
  ImportJob? _import;
  DateTime? _importSince;
  final _deleting = <String, ({DateTime since, Area? area})>{};
  DateTime? _modelSince;
  var _modelAborted = false;
  var _disposed = false;

  Running({
    required this.places,
    this.sync,
    this.imports,
    this.deletions,
    this.transcriber,
    this.photos,
    this.files,
    this.model,
    this.clock = DateTime.now,
  }) {
    for (final source in _sources) {
      source.addListener(_update);
    }
    _update();
  }

  List<Listenable> get _sources => [
    ?sync,
    ?sync?.syncing,
    ?imports?.running,
    ?deletions,
    ?transcriber,
    ?photos,
    ?files,
    ?model,
  ];

  /// The jobs running now.
  List<RunningJob> get jobs => _jobs;

  /// The jobs Abort paused, read from where they pause, then those it
  /// stopped, the newest first.
  List<StoppedJob> get stopped => [
    if (sync case final sync? when sync.paused && !sync.syncing.value)
      const StoppedJob(RunningKind.sync, goesOn: true),
    for (final (kind, reader) in _readers)
      if (reader.stopped && reader.busy == null) StoppedJob(kind, goesOn: true),
    ..._stopped,
  ];

  List<(RunningKind, _Reader)> get _readers => [
    if (transcriber case final t?)
      (
        RunningKind.transcripts,
        _Reader(() => t.busy, () => t.stopped, t.stop, t.goOn),
      ),
    if (photos case final p?)
      (
        RunningKind.photoTexts,
        _Reader(() => p.busy, () => p.stopped, p.stop, p.goOn),
      ),
    if (files case final f?)
      (
        RunningKind.attachmentTexts,
        _Reader(() => f.busy, () => f.stopped, f.stop, f.goOn),
      ),
  ];

  /// Stops [job] at its next safe point.
  void abort(RunningJob job) {
    switch (job.kind) {
      case RunningKind.sync:
        sync?.pause();
      case RunningKind.import:
        job.import?.abort();
      case RunningKind.transcripts ||
          RunningKind.photoTexts ||
          RunningKind.attachmentTexts:
        for (final (kind, reader) in _readers) {
          if (kind == job.kind) reader.stop();
        }
      case RunningKind.speechModel:
        final model = this.model;
        if (model == null || _modelAborted) return;
        _modelAborted = true;
        _update();
        unawaited(model.cancel());
      case RunningKind.areaDeletion:
        if (job.areaId case final id?) deletions?.abort(id);
    }
  }

  /// Starts the paused [job] again.
  void goOn(StoppedJob job) {
    switch (job.kind) {
      case RunningKind.sync:
        sync?.goOn();
      case RunningKind.transcripts ||
          RunningKind.photoTexts ||
          RunningKind.attachmentTexts:
        for (final (kind, reader) in _readers) {
          if (kind == job.kind) reader.goOn();
        }
      case RunningKind.import ||
          RunningKind.speechModel ||
          RunningKind.areaDeletion:
        break;
    }
  }

  void _push(StoppedJob job) {
    _stopped.insert(0, job);
    if (_stopped.length > maxStoppedJobs) _stopped.removeLast();
  }

  void _update() {
    if (_disposed) return;
    final now = clock();
    final jobs = <RunningJob>[];
    if (sync case final sync? when sync.syncing.value) {
      jobs.add(
        RunningJob(
          RunningKind.sync,
          since: _syncSince ??= now,
          stopping: sync.paused,
        ),
      );
    } else {
      _syncSince = null;
    }
    final import = imports?.running.value;
    if (!identical(import, _import)) {
      final before = _import;
      before?.removeListener(_update);
      if (before?.stopped case final at?) {
        _push(
          StoppedJob(
            RunningKind.import,
            done: at.done,
            of: at.of,
            import: before,
          ),
        );
      }
      _import = import;
      _importSince = import == null ? null : now;
      import?.addListener(_update);
    }
    if (import != null) {
      jobs.add(
        RunningJob(
          RunningKind.import,
          since: _importSince,
          done: import.of == null ? null : import.written,
          of: import.of,
          stopping: import.aborting,
          import: import,
        ),
      );
    }
    for (final (kind, reader) in _readers) {
      final busy = reader.busy;
      if (busy == null) continue;
      jobs.add(
        RunningJob(
          kind,
          since: busy.since,
          done: busy.done,
          of: busy.of,
          stopping: reader.stopped,
        ),
      );
    }
    final model = this.model;
    if (model != null && model.status == ModelStatus.fetching) {
      final progress = model.progress;
      jobs.add(
        RunningJob(
          RunningKind.speechModel,
          since: _modelSince ??= now,
          done: model.downloading ? progress.received : null,
          of: model.downloading ? progress.total : null,
          stopping: _modelAborted,
          downloading: model.downloading,
        ),
      );
    } else if (_modelSince != null) {
      if (_modelAborted) _push(const StoppedJob(RunningKind.speechModel));
      _modelSince = null;
      _modelAborted = false;
    }
    if (deletions case final deletions?) {
      for (final stopped in deletions.takeStopped()) {
        _push(
          StoppedJob(
            RunningKind.areaDeletion,
            done: stopped.moved,
            of: stopped.of,
            area:
                _deleting[stopped.areaId]?.area ??
                places.areaById(stopped.areaId),
          ),
        );
      }
      final ids = deletions.areaIds;
      _deleting.removeWhere((id, _) => !ids.contains(id));
      for (final id in ids) {
        final at = _deleting[id] ??= (since: now, area: places.areaById(id));
        final far = deletions.of(id);
        jobs.add(
          RunningJob(
            RunningKind.areaDeletion,
            since: at.since,
            done: far?.of == null ? null : far?.moved,
            of: far?.of,
            stopping: deletions.aborting(id),
            area: at.area,
            areaId: id,
          ),
        );
      }
    }
    _jobs = List.unmodifiable(jobs);
    notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    for (final source in _sources) {
      source.removeListener(_update);
    }
    _import?.removeListener(_update);
    super.dispose();
  }
}

/// The parts of a reader beside the app the page Running reads: the
/// [Transcriber], the [PhotoReader] and the [AttachmentReader] alike.
class _Reader {
  final ReaderBusy? Function() _busy;
  final bool Function() _stopped;
  final void Function() stop;
  final void Function() goOn;

  _Reader(this._busy, this._stopped, this.stop, this.goOn);

  ReaderBusy? get busy => _busy();
  bool get stopped => _stopped();
}

/// Hands the app's [Running] down to the page bars' line and the menus
/// that open the page Running.
class RunningScope extends InheritedWidget {
  final Running running;

  const RunningScope({super.key, required this.running, required super.child});

  /// The app's [Running], or null where there is none; read once, so it
  /// does not listen.
  static Running? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<RunningScope>()?.running;

  @override
  bool updateShouldNotify(RunningScope oldWidget) =>
      running != oldWidget.running;
}

/// The name of the page Running's route, so its own line does not open
/// it again.
const runningRouteName = 'running';

/// Whether the page Running can be opened from [context]: inside the app,
/// while it is not locked, as it names Areas and folders.
bool mayOpenRunning(BuildContext context) =>
    RunningScope.of(context) != null &&
    !(LockScope.of(context)?.locked ?? false);

/// Opens the page Running (obf-1078) from the line under a bar or a
/// menu; nothing on the page itself, without the app or while it is
/// locked.
void openRunning(BuildContext context) {
  final running = RunningScope.of(context);
  if (running == null || !mayOpenRunning(context)) return;
  if (ModalRoute.of(context)?.settings.name == runningRouteName) return;
  Navigator.of(context).push(
    MaterialPageRoute<void>(
      settings: const RouteSettings(name: runningRouteName),
      builder: (_) => RunningScreen(running: running),
    ),
  );
}

/// The page Running (obf-1078): a card for each job beside the app with
/// what it does, how far it is when known, since when and Abort, which
/// reads "Stopping…" until the job stops; "Nothing runs in the
/// background." without any; under "Stopped" what Abort stopped, the
/// paused ones with Go on. It closes when the app locks.
class RunningScreen extends StatefulWidget {
  final Running running;

  const RunningScreen({super.key, required this.running});

  @override
  State<RunningScreen> createState() => _RunningScreenState();
}

class _RunningScreenState extends State<RunningScreen> {
  AppLock? _lock;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final lock = LockScope.of(context);
    if (identical(lock, _lock)) return;
    _lock?.removeListener(_locked);
    _lock = lock?..addListener(_locked);
  }

  /// Leaves the page once the app locks: it names Areas and folders.
  void _locked() {
    if (!mounted || !(_lock?.locked ?? false)) return;
    // Gone already where the lock cleared the browser.
    if (ModalRoute.of(context) case final route? when route.isActive) {
      Navigator.of(context).removeRoute(route);
    }
  }

  @override
  void dispose() {
    _lock?.removeListener(_locked);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    final locale = Localizations.localeOf(context).toString();
    final weak = theme.colorScheme.onSurfaceVariant;
    return Scaffold(
      appBar: pageBar(context, title: t.jobsTitle),
      body: ListenableBuilder(
        listenable: widget.running,
        builder: (context, _) {
          final jobs = widget.running.jobs;
          final stopped = widget.running.stopped;
          return ListView(
            padding: clearOfSystemBars(
              context,
              const EdgeInsets.fromLTRB(16, 16, 16, 24),
            ),
            children: [
              if (jobs.isEmpty)
                Text(
                  t.jobsNone,
                  key: const ValueKey('running-none'),
                  style: theme.textTheme.bodyLarge?.copyWith(color: weak),
                ),
              for (final job in jobs)
                _card(
                  key: ValueKey('running-${job.kind.name}-${job.areaId}'),
                  children: [
                    Text(_what(t, job), style: theme.textTheme.titleMedium),
                    if (_far(t, job) case final far?) Text(far),
                    if (job.since case final since?)
                      Text(
                        t.jobsSince(
                          clockText(
                            locale,
                            ClockTime(since.hour, since.minute),
                          ),
                        ),
                        style: theme.textTheme.bodySmall?.copyWith(color: weak),
                      ),
                    if (job.stopping)
                      Text(
                        t.jobsStopping,
                        key: const ValueKey('running-stopping'),
                        style: theme.textTheme.bodyMedium?.copyWith(
                          color: weak,
                        ),
                      )
                    else ...[
                      if (_abortDoes(t, job) case final does?)
                        Text(
                          does,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: weak,
                          ),
                        ),
                      Align(
                        alignment: AlignmentDirectional.centerStart,
                        child: OutlinedButton(
                          key: const ValueKey('running-abort'),
                          onPressed: () => widget.running.abort(job),
                          child: Text(t.jobsAbort),
                        ),
                      ),
                    ],
                  ],
                ),
              if (stopped.isNotEmpty) ...[
                const SizedBox(height: 24),
                Semantics(
                  header: true,
                  child: Text(
                    t.jobsStopped,
                    style: theme.textTheme.titleMedium,
                  ),
                ),
                const SizedBox(height: 12),
                for (final job in stopped)
                  _card(
                    children: [
                      Text(
                        _stoppedWhat(t, job),
                        style: theme.textTheme.titleMedium,
                      ),
                      Text(_said(t, job)),
                      if (job.goesOn)
                        Align(
                          alignment: AlignmentDirectional.centerStart,
                          child: OutlinedButton(
                            key: ValueKey('running-go-on-${job.kind.name}'),
                            onPressed: () => widget.running.goOn(job),
                            child: Text(t.jobsGoOn),
                          ),
                        ),
                    ],
                  ),
              ],
            ],
          );
        },
      ),
    );
  }

  /// One row of the page, as a card.
  Widget _card({Key? key, required List<Widget> children}) => Padding(
    key: key,
    padding: const EdgeInsets.only(bottom: 12),
    child: Embossed(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (final (i, child) in children.indexed) ...[
              if (i > 0) const SizedBox(height: 8),
              child,
            ],
          ],
        ),
      ),
    ),
  );

  static String _importWhat(AppLocalizations t, ImportJob? job) =>
      switch ((job?.kind, job?.name)) {
        (ImportKind.keep, _) => t.importKeep,
        (_, final name?) => t.jobsImport(name),
        _ => t.importFolder,
      };

  static String _areaWhat(AppLocalizations t, Area? area) =>
      t.jobsAreaDeletion(area == null ? '' : areaName(t, area));

  static String _what(AppLocalizations t, RunningJob job) => switch (job.kind) {
    RunningKind.sync => t.jobsSync,
    RunningKind.import => _importWhat(t, job.import),
    RunningKind.transcripts => t.jobsTranscripts,
    RunningKind.photoTexts => t.jobsPhotoTexts,
    RunningKind.attachmentTexts => t.jobsAttachmentTexts,
    RunningKind.speechModel =>
      job.downloading ? t.jobsModelDownload : t.jobsModelCopy,
    RunningKind.areaDeletion => _areaWhat(t, job.area),
  };

  /// How far [job] is, in words, when that is known.
  static String? _far(AppLocalizations t, RunningJob job) {
    final (done, of) = (job.done, job.of);
    if (done == null) return null;
    return switch (job.kind) {
      RunningKind.sync => null,
      RunningKind.import when of != null => t.importWritten(done, of),
      RunningKind.areaDeletion when of != null => t.areaDeleting(done, of),
      RunningKind.transcripts ||
      RunningKind.photoTexts ||
      RunningKind.attachmentTexts when of != null => t.jobsRead(done, of),
      RunningKind.speechModel => t.speechModelDownloading(
        _megabytes(done),
        of == null ? '?' : _megabytes(of),
      ),
      _ => null,
    };
  }

  static String _megabytes(int bytes) => '${(bytes / 1e6).round()}';

  /// What Abort does for [job], where it does more or less than stop at
  /// once.
  static String? _abortDoes(AppLocalizations t, RunningJob job) =>
      switch (job.kind) {
        RunningKind.sync => t.jobsSyncAbort,
        RunningKind.import => t.jobsImportAbort,
        RunningKind.transcripts ||
        RunningKind.photoTexts ||
        RunningKind.attachmentTexts => t.jobsReadAbort,
        RunningKind.speechModel => null,
        RunningKind.areaDeletion => t.jobsAreaAbort,
      };

  static String _stoppedWhat(AppLocalizations t, StoppedJob job) =>
      switch (job.kind) {
        RunningKind.sync => t.jobsSync,
        RunningKind.import => _importWhat(t, job.import),
        RunningKind.transcripts => t.jobsTranscripts,
        RunningKind.photoTexts => t.jobsPhotoTexts,
        RunningKind.attachmentTexts => t.jobsAttachmentTexts,
        RunningKind.speechModel => t.jobsModelDownload,
        RunningKind.areaDeletion => _areaWhat(t, job.area),
      };

  /// How [job] ended: "Stopped: 120 of 340 imported".
  static String _said(AppLocalizations t, StoppedJob job) => switch (job.kind) {
    RunningKind.sync => t.jobsSyncPaused,
    RunningKind.import => t.jobsImportStopped(job.done ?? 0, job.of ?? 0),
    RunningKind.transcripts ||
    RunningKind.photoTexts ||
    RunningKind.attachmentTexts => t.jobsReadStopped,
    RunningKind.speechModel => t.jobsModelStopped,
    RunningKind.areaDeletion => t.jobsAreaStopped(job.done ?? 0, job.of ?? 0),
  };
}
