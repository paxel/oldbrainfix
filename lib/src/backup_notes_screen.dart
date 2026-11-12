import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'backup_screen.dart' show isolatePassKey;
import 'bundle_platform.dart';
import 'failure_dialog.dart';
import 'l10n.dart';
import 'pack_screen.dart';
import 'page_bar.dart';
import 'place_names.dart';
import 'restore_screen.dart';
import 'routine_editor_screen.dart' show deviceWallClock;
import 'system_bars.dart';

/// Announces that the Backup was not opened; the tap opens [cause],
/// [fix] and [details].
void _notOpened(
  BuildContext context,
  String cause,
  String fix,
  String details,
) {
  final t = context.t;
  announceFailure(
    context,
    t.backupNotOpened,
    title: t.backupNotOpenedTitle,
    cause: cause,
    fix: fix,
    details: details,
  );
}

/// "Open a Backup…" on the Backups page (#285, ADR 0016): the file
/// picker in the Backups folder, then, for a Backup, its page; a file
/// that is none, or of a later format, is announced at the top, why
/// behind the tap, as restoring says it.
Future<void> openBackupFile(
  BuildContext context,
  Store store, {
  PassKeyDeriver derive = isolatePassKey,
}) async {
  final t = context.t;
  final navigator = Navigator.of(context);
  final path = await BundleScope.of(context).pick(from: store.backupFolder);
  if (!context.mounted || path == null) return;
  BackupHeader? header;
  var later = false;
  try {
    header = readBackupHeader(path);
  } on BackupException {
    later = true;
  } on FileSystemException {
    // Unreadable: no Backup.
  }
  if (header == null) {
    _drop(path);
    final (cause, fix) = backupRefusal(
      t,
      later ? BackupProblem.needsUpdate : BackupProblem.notABackup,
      damagedFix: t.backupOpenDamagedFix,
    );
    _notOpened(
      context,
      cause,
      fix,
      later
          ? 'backup: a Backup of a later format'
          : 'backup: no Backup header in the file picked',
    );
    return;
  }
  final found = header;
  await navigator.push(
    MaterialPageRoute<void>(
      builder: (_) => BackupNotesScreen(
        store: store,
        path: path,
        header: found,
        derive: derive,
      ),
    ),
  );
}

/// Removes the copy of the file picked.
void _drop(String path) {
  try {
    File(path).deleteSync();
  } on FileSystemException {
    // Gone already.
  }
}

/// A Backup opened to take Notes back from it (#285, Q682 (a)): first
/// the passphrase or the recovery key, as restoring asks for them, then
/// the Backup's Notes as a Pack's tree, under its date, the ones from its
/// Trash said to be there, none ticked but all under each. "Add" takes
/// the ticked ones in as copies in new Areas, announced at the top as a
/// Pack's are, and closes the page; the log is not touched otherwise.
/// The copy of the file at [path] is the page's: it stays until the page
/// closes, as "Add" reads the file again, and goes then.
class BackupNotesScreen extends StatefulWidget {
  final Store store;
  final String path;
  final BackupHeader header;

  /// How the passphrase's key is derived; in an isolate by default.
  final PassKeyDeriver derive;

  const BackupNotesScreen({
    super.key,
    required this.store,
    required this.path,
    required this.header,
    this.derive = isolatePassKey,
  });

  @override
  State<BackupNotesScreen> createState() => _BackupNotesScreenState();
}

class _BackupNotesScreenState extends State<BackupNotesScreen> {
  OpenedBackup? _opened;
  Set<String> _ticks = const {};
  var _picked = false;
  var _busy = false;

  @override
  void dispose() {
    _drop(widget.path);
    super.dispose();
  }

  /// [ticks] as they are now, with whether they pick a Note worked out
  /// once for them, not on each build.
  void _setTicks(Pack pack, Set<String> ticks) {
    countPackTicksWorkedOut();
    _ticks = ticks;
    _picked = pack.picked(ticks).notes.isNotEmpty;
  }

  Future<void> _open(String? passphrase, String? code) async {
    final t = context.t;
    setState(() => _busy = true);
    // Why it did not open, what to do, and the full text.
    (String, String, String)? failure;
    OpenedBackup? opened;
    try {
      final key = await unlockBackup(
        widget.header,
        passphrase: passphrase,
        recoveryCode: code,
        derive: widget.derive,
      );
      if (key == null) {
        failure = (
          t.restoreWrong,
          t.restoreWrongFix,
          'backup: the passphrase or recovery key does not open it',
        );
      } else {
        opened = await openBackupNotes(
          widget.store,
          widget.path,
          key,
          shown: (a) => areaName(t, a),
        );
      }
    } on BackupException catch (e) {
      final (cause, fix) = backupRefusal(
        t,
        e.problem,
        damagedFix: t.backupOpenDamagedFix,
      );
      failure = (cause, fix, '$e');
    } on Object catch (e) {
      failure = (t.restoreFailedCause, t.restoreFailedFix, '$e');
    }
    if (!mounted) return;
    if (opened == null) {
      setState(() => _busy = false);
      final (cause, fix, details) =
          failure ??
          (t.restoreFailedCause, t.restoreFailedFix, 'backup: no result');
      _notOpened(context, cause, fix, details);
      return;
    }
    final pack = opened.pack;
    setState(() {
      _busy = false;
      _opened = opened;
      _setTicks(pack, pack.backupTicks());
    });
  }

  Future<void> _add(OpenedBackup opened) async {
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    final root = Navigator.of(context, rootNavigator: true);
    setState(() => _busy = true);
    final PackAdded added;
    try {
      added = await addBackupNotes(
        widget.store,
        opened,
        _ticks,
        wallClock: deviceWallClock,
        suffix: t.backupSuffix,
        noArea: t.backupNoArea,
      );
    } on Object catch (e) {
      if (mounted) setState(() => _busy = false);
      announceFailureWith(
        announcements,
        root,
        t.backupNotesNotAdded,
        title: t.backupNotesNotAddedTitle,
        cause: e is BackupException
            ? t.backupAddDamagedCause
            : t.restoreFailedCause,
        fix: t.backupAddDamagedFix,
        details: '$e',
      );
      return;
    }
    announcements?.announce(t.packAdded(added.notes));
    if (mounted) Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final opened = _opened;
    // Leaving while the file is read would take it away half-way.
    return PopScope(
      canPop: !_busy,
      child: Scaffold(
        appBar: opened == null
            ? pageBar(context, title: t.backupOpenPageTitle)
            : pageBar(
                context,
                title: t.backupOpenPageTitle,
                actions: packTickActions(
                  t,
                  onAll: () => setState(
                    () => _setTicks(opened.pack, opened.pack.allKeys()),
                  ),
                  // Nothing of a Note ticked, everything under each: a
                  // Note ticked again comes back whole.
                  onNone: () => setState(
                    () => _setTicks(opened.pack, opened.pack.backupTicks()),
                  ),
                ),
              ),
        body: ListView(
          padding: clearOfSystemBars(
            context,
            const EdgeInsets.fromLTRB(16, 8, 16, 24),
          ),
          children: opened == null ? _unlock(t) : _tree(t, opened),
        ),
      ),
    );
  }

  List<Widget> _unlock(AppLocalizations t) => [
    Text(t.backupOpenUnlockIntro),
    const SizedBox(height: 16),
    BackupUnlockForm(
      header: widget.header,
      busy: _busy,
      action: t.backupOpenStart,
      working: t.backupOpenWorking,
      onUnlock: (passphrase, code) => unawaited(_open(passphrase, code)),
    ),
  ];

  List<Widget> _tree(AppLocalizations t, OpenedBackup opened) {
    final when = DateFormat.yMMMMd(Localizations.localeOf(context).toString())
        .add_Hm();
    final pack = opened.pack;
    return [
      Text(
        t.restoreFile(when.format(opened.created.toLocal())),
        key: const ValueKey('backup-notes-when'),
        style: Theme.of(context).textTheme.titleMedium,
      ),
      const SizedBox(height: 8),
      Text(t.backupNotesHint),
      const SizedBox(height: 8),
      PackTreeView(
        pack: pack,
        ticks: _ticks,
        trashed: opened.trashed,
        onTick: (key, on) =>
            setState(() => _setTicks(pack, pack.tick(_ticks, key, on))),
      ),
      const Divider(height: 32),
      FilledButton(
        key: const ValueKey('backup-notes-add'),
        onPressed: _picked && !_busy ? () => unawaited(_add(opened)) : null,
        child: _busy
            ? const SizedBox.square(
                dimension: 20,
                child: CircularProgressIndicator(strokeWidth: 2),
              )
            : Text(t.packAdd),
      ),
      if (!_picked) ...[const SizedBox(height: 8), Text(t.packNothingTicked)],
    ];
  }
}
