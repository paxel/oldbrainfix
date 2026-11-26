import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'app_lock.dart';
import 'areas_screen.dart';
import 'big_screen.dart';
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
      look: folderLook(folder),
      work: (listed, fillIn) => folderImport(
        folder,
        t.importAreaName(name),
        listed: listed,
        fillIn: fillIn,
      ),
      finished: (summary, went) => [
        went,
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
      look: keepLook(source, zip: zip),
      work: (listed, fillIn) => keepImport(
        source,
        t.importKeepAreaName,
        zip: zip,
        listed: listed,
        fillIn: fillIn,
      ),
      finished: (summary, went) => [
        went,
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
      // Cancelled at the question, the zip was not read: what the picker
      // copied of it goes all the same.
      cancelled: zip ? folders.dropUnzipped : null,
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

  /// Runs the Import [kind] beside the app in [jobs] (obf-1061), on a
  /// page of its own that counts its Notes and that the user may leave:
  /// first [look] lists what was picked; when an earlier Import came from
  /// it, the user is asked once whether this one fills it in, and Cancel
  /// ends it, writing nothing (obf-1078); then [work] of what was listed,
  /// with the Area to fill in or none. It announces at the top in [t],
  /// taken before any await, once done, even after this page closed,
  /// without values (UI laws): its tap shows what went where, [finished]
  /// says it after the line of where the Notes went, with a way into the
  /// Area; with nothing missing from the earlier Import, that nothing is
  /// new; or, with nothing to import, why ([nothing]); or that it
  /// [failed]. Stopped by Abort on the page
  /// Running (obf-1078), it announces "Import stopped.", its tap showing
  /// how many of how many Notes came in and, with any, what went where.
  /// The page says the same. Then gives up what was picked for it
  /// ([release]). [name] is the folder's, for the page Running.
  Future<void> _runImport<L>(
    ImportKind kind,
    AppLocalizations t,
    Announcements? announcements,
    NavigatorState navigator,
    ImportJobs jobs, {
    String? name,
    required LongWork<Looked<L>> look,
    required ImportWork Function(L listed, String? fillIn) work,
    required String Function(ImportSummary summary, String went) finished,
    required ({String announced, String cause, String fix, String details})
    Function(ImportSummary summary)
    nothing,
    required ({String announced, String cause, String fix}) failed,
    required Future<void> Function() release,
    Future<void> Function()? cancelled,
  }) async {
    final store = widget.store;
    final trail = widget.trail;
    final job = ImportJob(kind, name: name);
    _showImport(navigator, store, trail, job);
    // The Area of the earlier Import this one fills in, when the user
    // said so.
    String? fillIn;
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
      final summary = await jobs.runLooking(job, look, (looked) async {
        if (runOver(store)) return null;
        // Not asked once Abort was pressed: it writes nothing then.
        if (looked.earlier case final earlier? when !job.aborting) {
          // The question names an Area: not over the App Lock.
          await _unlocked(LockScope.of(navigator.context));
          if (runOver(store) || !navigator.mounted) return null;
          final fill = await _askFillIn(navigator, t, kind, store, earlier);
          if (fill == null || runOver(store)) return null;
          if (fill) fillIn = earlier.areaId;
        }
        return work(looked.listed, fillIn);
      });
      // A Restart ended this run of the app meanwhile: its store is
      // closed, and nobody is left to tell.
      if (runOver(store)) return;
      if (summary == null) {
        // Cancelled at the question: nothing was written, and its page
        // goes, also where its row opened it again.
        for (final page in _pages[job] ?? const <Route<void>>[]) {
          if (page.isActive) navigator.removeRoute(page);
        }
        if (cancelled != null) unawaited(cancelled());
        return;
      }
      final area = switch (summary.areaId) {
        final id? => Places(store).areaById(id),
        null => null,
      };
      // Filled in as asked; a fill-in the core refused made a new Area
      // with every item instead, told as any new Import.
      final filled = fillIn != null && summary.areaId == fillIn;
      String went(Area area) {
        final shown = areaName(t, area);
        if (!filled) return t.importFinishedNotes(summary.notes, shown);
        return [
          t.importFilledIn(summary.notes, shown),
          if (summary.already > 0) t.importAlready(summary.already),
        ].join('\n');
      }

      // Stopped by Abort, before its first Note or later (obf-1078).
      if (summary.stopped case final of?) {
        final said = t.jobsImportStopped(summary.notes, of);
        final details = area == null
            ? said
            : '$said\n\n${finished(summary, went(area))}';
        job.ended(ImportEnd(details, area: area));
        announceEnd(t.jobsImportEnded, details, area);
        return;
      }
      // Each item the earlier Import made already: nothing new.
      if (area == null && summary.already > 0) {
        final said = t.importNothingNew(summary.already);
        job.ended(ImportEnd(said));
        announceEnd(t.importFinished, said, null);
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
      final details = finished(summary, went(area));
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

  /// Waits while [lock] stands in front of browsing.
  static Future<void> _unlocked(AppLock? lock) async {
    if (lock == null || !lock.locked) return;
    final open = Completer<void>();
    void heard() {
      if (!lock.locked && !open.isCompleted) open.complete();
    }

    lock.addListener(heard);
    try {
      await open.future;
    } finally {
      lock.removeListener(heard);
    }
  }

  /// Asks once whether the Import [kind] fills in the [earlier] one of
  /// the same source (obf-1078), its date and Area named: true for Fill
  /// in, false for Import as new, null for Cancel or a tap beside it.
  static Future<bool?> _askFillIn(
    NavigatorState navigator,
    AppLocalizations t,
    ImportKind kind,
    Store store,
    EarlierImport earlier,
  ) {
    final context = navigator.context;
    final area = Places(store).areaById(earlier.areaId);
    final date = DateFormat.yMd(Localizations.localeOf(context).toString())
        .format(earlier.at.toLocal());
    final ask = t.importFillInAsk(
      date,
      area == null ? earlier.areaName : areaName(t, area),
    );
    return showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(switch (kind) {
          ImportKind.folder => context.t.importFolder,
          ImportKind.keep => context.t.importKeep,
        }),
        // The two answers stacked full width, Cancel below (UI laws).
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(ask),
              const SizedBox(height: 16),
              OutlinedButton(
                key: const ValueKey('importFillIn'),
                onPressed: () => Navigator.of(context).pop(true),
                child: Text(context.t.importFillIn),
              ),
              const SizedBox(height: 8),
              OutlinedButton(
                key: const ValueKey('importAsNew'),
                onPressed: () => Navigator.of(context).pop(false),
                child: Text(context.t.importAsNew),
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
          place: const ShownPlace(PlaceKind.areas),
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
          place: ShownPlace.area(area.id),
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
  /// it again. Each is kept in [_pages], for a Cancel to take away.
  static void _showImport(
    NavigatorState navigator,
    Store store,
    BrowserTrail trail,
    ImportJob job,
  ) {
    final route = MaterialPageRoute<void>(
      builder: (_) => ImportScreen(
        job: job,
        onOpen: (area) => _openImported(navigator, store, trail, area),
      ),
    );
    (_pages[job] ??= []).add(route);
    unawaited(navigator.push(route));
  }

  /// The pages [_showImport] opened of each Import.
  static final _pages = Expando<List<Route<void>>>();

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
