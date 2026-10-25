import 'dart:async';
import 'dart:io';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'auto_sync.dart';
import 'bundle_platform.dart';
import 'folder_platform.dart';
import 'l10n.dart';
import 'look.dart';
import 'pairing_platform.dart';

/// [backupPassKey] in an isolate of its own, so the screen keeps
/// moving while Argon2id works.
Future<Uint8List> isolatePassKey(
  String passphrase,
  BackupKdf kdf,
  List<int> salt,
) => Isolate.run(() => backupPassKey(passphrase, kdf, salt));

/// The name the Recovery Sheet is shared and printed under.
const recoverySheetName = 'oldbrainfix-recovery-sheet.pdf';

/// The Recovery Sheet for [code] in the app's language, made [at] on
/// the Device [device].
Uint8List recoverySheetBytes(
  AppLocalizations t,
  String locale,
  String code, {
  required String device,
  required DateTime at,
}) => recoverySheetPdf(
  RecoverySheet(
    title: t.recoverySheetPdfTitle,
    about: [
      t.recoverySheetMade(DateFormat.yMMMMd(locale).add_Hm().format(at)),
      t.recoverySheetDevice(device),
    ],
    intro: [t.recoverySheetPdfIntro],
    codeLabel: t.recoverySheetKey,
    steps: [t.recoverySheetStepRestore, t.recoverySheetStepSettings],
  ),
  code,
);

/// Backups, inside Settings (ADR 0010, 0016): set up once with a
/// passphrase, which makes the Recovery Sheet to print or share at once
/// and again later; then the folder they go into, the weekly switch,
/// "Back up now" and how the last one went. What happened, or why not,
/// is announced at the top.
class BackupScreen extends StatefulWidget {
  final Store store;

  /// The passphrase's cost; tests hand in a cheap one.
  final BackupKdf kdf;

  /// How the passphrase's key is derived; in an isolate by default.
  final PassKeyDeriver derive;

  const BackupScreen({
    super.key,
    required this.store,
    this.kdf = BackupKdf.standard,
    this.derive = isolatePassKey,
  });

  @override
  State<BackupScreen> createState() => _BackupScreenState();
}

class _BackupScreenState extends State<BackupScreen> {
  final _passphrase = TextEditingController();
  final _again = TextEditingController();
  String? _passError;
  String? _againError;
  var _settingUp = false;
  var _backingUp = false;
  var _sheetBusy = false;
  String? _folderName;
  var _named = false;

  Store get _store => widget.store;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!_named) {
      _named = true;
      unawaited(_nameFolder());
    }
  }

  @override
  void dispose() {
    _passphrase.dispose();
    _again.dispose();
    super.dispose();
  }

  void _announce(String text) => AnnouncementsScope.of(context)?.announce(text);

  Future<void> _nameFolder() async {
    final folder = _store.backupFolder;
    if (folder == null) return;
    final name = await FolderScope.of(context).displayName(folder);
    if (!mounted) return;
    setState(() => _folderName = name);
  }

  Future<void> _setUp() async {
    final t = context.t;
    final passphrase = _passphrase.text;
    final short = passphrase.trim().length < minPassphraseLength;
    final mismatch = !short && passphrase != _again.text;
    setState(() {
      _passError = short ? t.backupPassphraseShort : null;
      _againError = mismatch ? t.backupPassphraseMismatch : null;
    });
    if (short || mismatch) return;
    setState(() => _settingUp = true);
    try {
      await setUpBackups(
        _store,
        passphrase,
        kdf: widget.kdf,
        derive: widget.derive,
      );
    } finally {
      if (mounted) setState(() => _settingUp = false);
    }
    if (!mounted) return;
    _passphrase.clear();
    _again.clear();
    AutoSyncScope.of(context)?.devicesChanged();
  }

  /// Writes the Recovery Sheet where the platform shares from and hands
  /// it to the print dialog, or to the app the user picks.
  Future<void> _sheet({required bool print}) async {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final bundles = BundleScope.of(context);
    final pairing = PairingScope.of(context);
    final code = _store.recoveryCode;
    if (code == null) return;
    setState(() => _sheetBusy = true);
    try {
      final path = await bundles.exportPath(recoverySheetName);
      final device = await pairing.deviceName();
      if (!mounted) return;
      if (path == null) {
        _announce(t.backupSheetFailed);
        return;
      }
      File(path).writeAsBytesSync(
        recoverySheetBytes(
          t,
          locale,
          code,
          device: device,
          at: _store.clock().toLocal(),
        ),
        flush: true,
      );
      final handed = print
          ? await bundles.print(path, recoverySheetName)
          : await bundles.share(path, type: 'application/pdf');
      if (!mounted) return;
      if (!handed) _announce(t.backupSheetNoApp);
    } on FileSystemException {
      if (mounted) _announce(t.backupSheetFailed);
    } finally {
      if (mounted) setState(() => _sheetBusy = false);
    }
  }

  /// Picks the folder and writes the first Backup into it at once.
  Future<void> _pickFolder() async {
    final folders = FolderScope.of(context);
    final picked = await folders.pick();
    if (!mounted || picked == null) return;
    _store.backupFolder = picked;
    final name = await folders.displayName(picked);
    if (!mounted) return;
    setState(() => _folderName = name);
    AutoSyncScope.of(context)?.devicesChanged();
    await _backUp();
  }

  void _setWeekly(bool on) {
    setState(() => _store.backupsWeekly = on);
    AutoSyncScope.of(context)?.devicesChanged();
  }

  Future<void> _backUp() async {
    final t = context.t;
    final folder = _store.backupFolder;
    if (folder == null || _backingUp) return;
    final folders = FolderScope.of(context);
    final work =
        AutoSyncScope.of(context)?.workDir ?? Directory.systemTemp.path;
    setState(() => _backingUp = true);
    try {
      await backUpNow(_store, folders.openBackups(folder), work);
      if (mounted) _announce(t.backupDone);
    } on Object {
      // Kept as the status below as well.
      if (mounted) _announce(t.backupFailed);
    } finally {
      if (mounted) setState(() => _backingUp = false);
    }
  }

  String _status(AppLocalizations t, DateFormat when) {
    if (_backingUp) return t.backupWorking;
    final failed = _store.backupFailed;
    if (failed != null) return t.backupFailedAt(when.format(failed.toLocal()));
    final last = _store.lastBackup;
    return last == null
        ? t.backupNotYet
        : t.backupLast(when.format(last.toLocal()));
  }

  List<Widget> _setUpForm(AppLocalizations t) => [
    TextField(
      key: const ValueKey('passphrase'),
      controller: _passphrase,
      enabled: !_settingUp,
      obscureText: true,
      autocorrect: false,
      enableSuggestions: false,
      decoration: InputDecoration(
        labelText: t.backupPassphrase,
        helperText: t.backupPassphraseHint,
        helperMaxLines: 2,
        errorText: _passError,
        border: const OutlineInputBorder(),
      ),
    ),
    const SizedBox(height: 12),
    TextField(
      key: const ValueKey('passphrase-again'),
      controller: _again,
      enabled: !_settingUp,
      obscureText: true,
      autocorrect: false,
      enableSuggestions: false,
      decoration: InputDecoration(
        labelText: t.backupPassphraseAgain,
        errorText: _againError,
        border: const OutlineInputBorder(),
      ),
      onSubmitted: (_) => _setUp(),
    ),
    const SizedBox(height: 12),
    Align(
      alignment: Alignment.centerRight,
      child: _settingUp
          ? _Busy(t.backupSettingUp)
          : FilledButton(
              key: const ValueKey('set-up'),
              onPressed: _setUp,
              child: Text(t.backupSetUp),
            ),
    ),
  ];

  List<Widget> _setUpDone(AppLocalizations t, DateFormat when) {
    final folders = FolderScope.of(context);
    final folder = _store.backupFolder;
    return [
      _Card(
        title: t.backupSheetTitle,
        children: [
          Text(t.backupSheetIntro),
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              FilledButton.icon(
                key: const ValueKey('sheet-print'),
                onPressed: _sheetBusy ? null : () => _sheet(print: true),
                icon: const Icon(Icons.print_outlined),
                label: Text(t.backupSheetPrint),
              ),
              FilledButton.tonalIcon(
                key: const ValueKey('sheet-share'),
                onPressed: _sheetBusy ? null : () => _sheet(print: false),
                icon: const Icon(Icons.share_outlined),
                label: Text(t.backupSheetShare),
              ),
            ],
          ),
        ],
      ),
      if (folders.available)
        _Card(
          title: t.backupFolderTitle,
          children: [
            Text(
              folder == null
                  ? t.backupFolderNone
                  : t.backupFolderChosen(_folderName ?? '…'),
            ),
            const SizedBox(height: 8),
            Align(
              alignment: Alignment.centerLeft,
              child: FilledButton.tonalIcon(
                key: const ValueKey('backup-folder'),
                onPressed: _backingUp ? null : _pickFolder,
                icon: const Icon(Icons.folder_open_outlined),
                label: Text(t.backupChooseFolder),
              ),
            ),
            if (folder != null) ...[
              SwitchListTile(
                key: const ValueKey('backup-weekly'),
                contentPadding: EdgeInsets.zero,
                title: Text(t.backupWeekly),
                subtitle: Text(t.backupWeeklyDetail),
                value: _store.backupsWeekly,
                onChanged: _setWeekly,
              ),
              Text(_status(t, when)),
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerRight,
                child: _backingUp
                    ? _Busy(t.backupWorking)
                    : FilledButton(
                        key: const ValueKey('backup-now'),
                        onPressed: _backUp,
                        child: Text(t.backupNow),
                      ),
              ),
            ],
          ],
        ),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final when = DateFormat.yMd(Localizations.localeOf(context).toString())
        .add_Hm();
    return Scaffold(
      appBar: AppBar(title: Text(t.backupsTitle)),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
        children: [
          Text(t.backupsIntro),
          const SizedBox(height: 16),
          if (_store.backupsSetUp) ..._setUpDone(t, when) else ..._setUpForm(t),
        ],
      ),
    );
  }
}

/// One part of the Backups page on an embossed card.
class _Card extends StatelessWidget {
  final String title;
  final List<Widget> children;

  const _Card({required this.title, required this.children});

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 16),
    child: Embossed(
      child: Material(
        type: MaterialType.transparency,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(title, style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              ...children,
            ],
          ),
        ),
      ),
    ),
  );
}

/// A small spinner with what it waits for.
class _Busy extends StatelessWidget {
  final String text;

  const _Busy(this.text);

  @override
  Widget build(BuildContext context) => Row(
    mainAxisSize: MainAxisSize.min,
    children: [
      const SizedBox.square(
        dimension: 20,
        child: CircularProgressIndicator(strokeWidth: 2),
      ),
      const SizedBox(width: 12),
      Flexible(child: Text(text)),
    ],
  );
}
