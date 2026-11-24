import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'app_lock.dart';
import 'areas_screen.dart';
import 'browser.dart';
import 'failure_dialog.dart';
import 'folder_import.dart';
import 'folder_platform.dart';
import 'import_run.dart';
import 'import_screen.dart';
import 'keep_import.dart';
import 'l10n.dart';
import 'page_bar.dart';
import 'place_names.dart';
import 'run_over.dart';
import 'settings_search.dart';
import 'system_bars.dart';

/// The page Import… in the Areas list's ⋮ opens (obf-1096): the Import
/// of a folder and of a Google Keep export (ADR 0036), each a row on one
/// card, beside the app in [ImportJobsScope] (obf-1061). Offered only
/// where a folder can be picked ([offered]).
class ImportsScreen extends StatefulWidget {
  final Store store;

  /// The notes browser's way back in, which the Area an Import made
  /// opens as the last selected one.
  final BrowserTrail trail;

  const ImportsScreen({super.key, required this.store, required this.trail});

  /// Whether this Device can import at all: a folder can be picked and
  /// the Imports have their place beside the app.
  static bool offered(BuildContext context) =>
      FolderScope.of(context).available && ImportJobsScope.of(context) != null;

  @override
  State<ImportsScreen> createState() => _ImportsScreenState();
}

class _ImportsScreenState extends State<ImportsScreen> {
  /// Picks a folder with the system's picker and makes Notes of its
  /// Markdown and text files in a new Area (ADR 0036), announced as
  /// [_runImport] does.
  Future<void> _import(ImportJobs jobs) async {
    if (jobs.running.value != null) return;
    final t = context.t;
    final folders = FolderScope.of(context);
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context, rootNavigator: true);
    final store = widget.store;
    final folder = await folders.pick();
    if (folder == null || jobs.running.value != null) return;
    final name = await folders.displayName(folder);
    if (jobs.running.value != null) return;
    await _runImport(
      ImportKind.folder,
      t,
      announcements,
      navigator,
      jobs,
      name: name,
      work: folderImport(folder, t.importAreaName(name)),
      finished: (summary, area) => [
        t.importFinishedNotes(summary.notes, area),
        if (summary.skipped > 0) ...[
          '',
          t.importFinishedSkipped(summary.skipped),
          t.importSkippedDetails(name, summary.notText, summary.tooLarge),
        ],
      ].join('\n'),
      nothing: (summary) => (
        announced: t.importNothing,
        cause: t.importNothingCause,
        fix: t.importNothingFix,
        details: t.importSkippedDetails(
          name,
          summary.notText,
          summary.tooLarge,
        ),
      ),
      failed: (
        announced: t.importFailed,
        cause: t.importFailedCause,
        fix: t.importFailedFix,
      ),
      // The folder was picked for this Import alone: its lasting access
      // goes, unless the app syncs or backs up through it.
      release: () =>
          folders.release(folder, [?store.syncFolder, ?store.backupFolder]),
    );
  }

  /// Asks whether the Google Takeout export is still a zip or an unpacked
  /// folder, picks it with the system's picker and makes Notes of its
  /// Google Keep notes in the new Area "From Google Keep" (ADR 0036),
  /// announced as [_runImport] does.
  Future<void> _importKeep(ImportJobs jobs) async {
    if (jobs.running.value != null) return;
    final t = context.t;
    final folders = FolderScope.of(context);
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context, rootNavigator: true);
    final store = widget.store;
    final zip = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(context.t.importKeep),
        // The two answers stacked full width, Cancel below (UI laws).
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(context.t.importKeepAsk),
              const SizedBox(height: 16),
              OutlinedButton(
                key: const ValueKey('importKeepZip'),
                onPressed: () => Navigator.of(context).pop(true),
                child: Text(context.t.importKeepZip),
              ),
              const SizedBox(height: 8),
              OutlinedButton(
                key: const ValueKey('importKeepFolder'),
                onPressed: () => Navigator.of(context).pop(false),
                child: Text(context.t.importKeepFolder),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(context.t.cancel),
          ),
        ],
      ),
    );
    if (zip == null || !mounted || jobs.running.value != null) return;
    final source = zip ? await folders.pickZip() : await folders.pick();
    if (source == null || jobs.running.value != null) return;
    await _runImport(
      ImportKind.keep,
      t,
      announcements,
      navigator,
      jobs,
      work: keepImport(source, t.importKeepAreaName, zip: zip),
      finished: (summary, area) => [
        t.importFinishedNotes(summary.notes, area),
        if (summary.checklists > 0) t.importKeepChecklists(summary.checklists),
        if (summary.photos > 0) t.importKeepPhotos(summary.photos),
        if (summary.skipped > 0) ...[
          '',
          t.importKeepSkipped(summary.skipped),
          _keepSkipped(t, summary),
        ],
      ].join('\n'),
      nothing: (summary) => (
        announced: t.importKeepNothing,
        cause: t.importKeepNothingCause,
        fix: t.importKeepNothingFix,
        details: _keepSkipped(t, summary),
      ),
      failed: (
        announced: t.importKeepFailed,
        cause: t.importKeepFailedCause,
        fix: t.importKeepFailedFix,
      ),
      // A zip leaves no lasting access behind, only the copies of its
      // files; a folder's goes as for any Import.
      release: () => zip
          ? Future<void>.value()
          : folders.release(source, [?store.syncFolder, ?store.backupFolder]),
    );
  }

  static String _keepSkipped(AppLocalizations t, ImportSummary summary) =>
      t.importKeepSkippedDetails(
        summary.trashed,
        summary.missing,
        summary.notPhoto,
        summary.tooLarge,
        summary.notText,
      );

  /// Runs the Import [kind] as [work] beside the app in [jobs]
  /// (obf-1061), on a page of its own that counts its Notes and that the
  /// user may leave, and announces at the top in [t], taken before any
  /// await, once done, even after this page closed, without values (UI
  /// laws): its tap shows what went where, [finished] says it of the new
  /// Area, with a way into the Area; or, with nothing to import, why
  /// ([nothing]); or that it [failed]. Stopped by Abort on the page
  /// Running (obf-1078), it announces "Import stopped.", its tap showing
  /// how many of how many Notes came in and, with any, what went where.
  /// The page says the same. Then gives up what was picked for it
  /// ([release]). [name] is the folder's, for the page Running.
  Future<void> _runImport(
    ImportKind kind,
    AppLocalizations t,
    Announcements? announcements,
    NavigatorState navigator,
    ImportJobs jobs, {
    String? name,
    required ImportWork work,
    required String Function(ImportSummary summary, String area) finished,
    required ({String announced, String cause, String fix, String details})
    Function(ImportSummary summary)
    nothing,
    required ({String announced, String cause, String fix}) failed,
    required Future<void> Function() release,
  }) async {
    final store = widget.store;
    final trail = widget.trail;
    final job = ImportJob(kind, name: name);
    _showImport(navigator, store, trail, job);
    // Says at the top that it ended as [title], its tap showing
    // [details], with a way into the new [area] when there is one.
    void announceEnd(String title, String details, Area? area) =>
        announcements?.announce(
          title,
          onTap: () => unawaited(
            showDialog<void>(
              context: navigator.context,
              builder: (context) => AlertDialog(
                title: Text(title),
                content: SingleChildScrollView(child: Text(details)),
                actions: [
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(),
                    child: Text(context.t.close),
                  ),
                  if (area != null)
                    TextButton(
                      key: const ValueKey('importOpenArea'),
                      onPressed: () {
                        Navigator.of(context).pop();
                        _openImported(navigator, store, trail, area);
                      },
                      child: Text(context.t.importOpenArea),
                    ),
                ],
              ),
            ),
          ),
        );
    try {
      final summary = await jobs.run(job, work);
      // A Restart ended this run of the app meanwhile: its store is
      // closed, and nobody is left to tell.
      if (runOver(store)) return;
      final area = switch (summary.areaId) {
        final id? => Places(store).areaById(id),
        null => null,
      };
      // Stopped by Abort, before its first Note or later (obf-1078).
      if (summary.stopped case final of?) {
        final said = t.jobsImportStopped(summary.notes, of);
        final details = area == null
            ? said
            : '$said\n\n${finished(summary, areaName(t, area))}';
        job.ended(ImportEnd(details, area: area));
        announceEnd(t.jobsImportEnded, details, area);
        return;
      }
      if (area == null) {
        final why = nothing(summary);
        job.ended(ImportEnd('${why.announced}\n\n${why.cause} ${why.fix}'));
        announceFailureWith(
          announcements,
          navigator,
          why.announced,
          title: t.importNothingTitle,
          cause: why.cause,
          fix: why.fix,
          details: why.details,
        );
        return;
      }
      final details = finished(summary, areaName(t, area));
      job.ended(ImportEnd(details, area: area));
      announceEnd(t.importFinished, details, area);
    } on Exception catch (e) {
      if (runOver(store)) return;
      job.ended(
        ImportEnd('${failed.announced}\n\n${failed.cause} ${failed.fix}'),
      );
      announceFailureWith(
        announcements,
        navigator,
        failed.announced,
        title: t.importFailedTitle,
        cause: failed.cause,
        fix: failed.fix,
        details: '$e',
      );
    } finally {
      // Not on a store a Restart closed meanwhile, which [release] reads.
      if (!runOver(store)) unawaited(release());
    }
  }

  /// The notes browser on top, the Areas list and in it the imported
  /// [area], chosen as the last selected one; the Areas list alone while
  /// [area] was locked and is closed meanwhile.
  static void _openImported(
    NavigatorState navigator,
    Store store,
    BrowserTrail trail,
    Area area,
  ) {
    final notes = Notes(store);
    final places = Places(store);
    trail.choseArea(area);
    unawaited(
      navigator.push(
        BrowserRoute<void>(
          builder: (_) =>
              AreasScreen(notes: notes, places: places, trail: trail),
        ),
      ),
    );
    final closed =
        LockScope.of(navigator.context)?.isAreaClosed(area.id) ?? false;
    if (closed) return;
    unawaited(
      navigator.push(
        BrowserRoute<void>(
          builder: (_) => AreaScreen(
            notes: notes,
            places: places,
            trail: trail,
            areaId: area.id,
          ),
        ),
      ),
    );
  }

  /// The row of the Import [kind] in [jobs]: [title] over [detail], or
  /// over [reading] with a spinner while it runs, its tap then opening
  /// its page again (obf-1061); the other row takes no tap meanwhile.
  Widget _importRow(
    ImportJobs jobs,
    ImportKind kind, {
    required Key key,
    required IconData icon,
    required String title,
    required String detail,
    required String reading,
    required VoidCallback onTap,
  }) => ValueListenableBuilder<ImportJob?>(
    valueListenable: jobs.running,
    builder: (context, job, _) {
      final running = job != null && job.kind == kind;
      return ListTile(
        key: key,
        leading: Icon(icon),
        title: Text(title),
        subtitle: Text(running ? reading : detail),
        trailing: running
            ? const SizedBox.square(
                dimension: 24,
                child: CircularProgressIndicator(strokeWidth: 3),
              )
            : null,
        enabled: job == null || running,
        onTap: running
            ? () => _showImport(
                Navigator.of(context, rootNavigator: true),
                widget.store,
                widget.trail,
                job,
              )
            : onTap,
      );
    },
  );

  /// The page of the Import [job] on top, its Open opening the new Area
  /// as [_openImported] does; once left, the running Import's row opens
  /// it again.
  static void _showImport(
    NavigatorState navigator,
    Store store,
    BrowserTrail trail,
    ImportJob job,
  ) {
    unawaited(
      navigator.push(
        MaterialPageRoute<void>(
          builder: (_) => ImportScreen(
            job: job,
            onOpen: (area) => _openImported(navigator, store, trail, area),
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final imports = ImportJobsScope.of(context);
    return Scaffold(
      appBar: pageBar(context, title: t.settingsImport),
      body: ListView(
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.symmetric(vertical: 8),
        ),
        children: [
          if (imports != null)
            SettingsSection(
              children: [
                _importRow(
                  imports,
                  ImportKind.folder,
                  key: const ValueKey('importFolder'),
                  icon: Icons.drive_folder_upload_outlined,
                  title: t.importFolder,
                  detail: t.importFolderDetail,
                  reading: t.importReading,
                  onTap: () => unawaited(_import(imports)),
                ),
                _importRow(
                  imports,
                  ImportKind.keep,
                  key: const ValueKey('importKeep'),
                  icon: Icons.sticky_note_2_outlined,
                  title: t.importKeep,
                  detail: t.importKeepDetail,
                  reading: t.importKeepReading,
                  onTap: () => unawaited(_importKeep(imports)),
                ),
              ],
            ),
        ],
      ),
    );
  }
}
