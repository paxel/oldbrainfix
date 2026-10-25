import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'backup_screen.dart';
import 'bundle_platform.dart';
import 'l10n.dart';
import 'pairing_platform.dart';
import 'photo_platform.dart';

/// The newest Backup in [dir] by when it was made, or null when none
/// lies there: what Android's Auto Backup brought back (ADR 0010).
String? newestBackupIn(String dir) {
  final List<File> files;
  try {
    files = Directory(dir).listSync().whereType<File>().toList();
  } on FileSystemException {
    return null;
  }
  String? newest;
  DateTime? made;
  for (final file in files) {
    if (!file.path.endsWith('.$backupExtension')) continue;
    final BackupHeader? header;
    try {
      header = readBackupHeader(file.path);
    } on FileSystemException {
      continue;
    }
    if (header == null) continue;
    if (made == null || header.created.isAfter(made)) {
      newest = file.path;
      made = header.created;
    }
  }
  return newest;
}

/// First start, before the Starter Set (ADR 0016): a Backup file picked
/// and opened with the passphrase, or with the recovery key typed or
/// scanned from the Recovery Sheet, goes into the empty log. Pops with
/// what was restored; why nothing was is announced at the top.
///
/// With [found], the Backup Android's Auto Backup brought back from the
/// user's Google account (ADR 0010) is open at once, asking for the
/// passphrase or the recovery key; it is never removed here, and
/// "Start without it" pops with nothing.
class RestoreScreen extends StatefulWidget {
  final Store store;

  /// The Backup that came back with the app, if one did.
  final String? found;

  /// How the passphrase's key is derived; in an isolate by default.
  final PassKeyDeriver derive;

  const RestoreScreen({
    super.key,
    required this.store,
    this.found,
    this.derive = isolatePassKey,
  });

  @override
  State<RestoreScreen> createState() => _RestoreScreenState();
}

class _RestoreScreenState extends State<RestoreScreen> {
  final _passphrase = TextEditingController();
  final _code = TextEditingController();
  String? _path;
  BackupHeader? _header;
  var _busy = false;

  @override
  void initState() {
    super.initState();
    if (widget.found case final found?) {
      _header = readBackupHeader(found);
      if (_header != null) _path = found;
    }
  }

  @override
  void dispose() {
    _passphrase.dispose();
    _code.dispose();
    _drop();
    super.dispose();
  }

  void _announce(String text) => AnnouncementsScope.of(context)?.announce(text);

  /// Removes the copy of the file picked; the Backup that came back with
  /// the app stays.
  void _drop() {
    final path = _path;
    if (path == null || path == widget.found) return;
    try {
      File(path).deleteSync();
    } on FileSystemException {
      // Gone already.
    }
  }

  Future<void> _pick() async {
    final t = context.t;
    final path = await BundleScope.of(context).pick();
    if (!mounted || path == null) return;
    _drop();
    final header = readBackupHeader(path);
    setState(() {
      _path = header == null ? null : path;
      _header = header;
    });
    if (header == null) {
      try {
        File(path).deleteSync();
      } on FileSystemException {
        // Gone already.
      }
      _announce(t.restoreNotABackup);
    }
  }

  /// Takes a photo of the Recovery Sheet and reads the key in its QR.
  Future<void> _scan() async {
    final t = context.t;
    final photos = PhotoScope.of(context);
    final pairing = PairingScope.of(context);
    final photo = await photos.takePhoto();
    if (!mounted || photo == null) return;
    final code = await pairing.readQr(photo);
    if (!mounted) return;
    if (code == null || decodeRecoveryCode(code) == null) {
      _announce(t.restoreNoCode);
      return;
    }
    setState(() => _code.text = code);
  }

  Future<void> _restore() async {
    final t = context.t;
    final header = _header;
    final path = _path;
    if (header == null || path == null) return;
    final code = _code.text.trim();
    if (code.isNotEmpty && decodeRecoveryCode(code) == null) {
      _announce(t.restoreInvalidCode);
      return;
    }
    setState(() => _busy = true);
    String? failure;
    RestoreResult? result;
    try {
      final key = await unlockBackup(
        header,
        passphrase: code.isEmpty ? _passphrase.text : null,
        recoveryCode: code.isEmpty ? null : code,
        derive: widget.derive,
      );
      if (key == null) {
        failure = t.restoreWrong;
      } else {
        result = await restoreBackup(widget.store, path, key);
      }
    } on BackupException catch (e) {
      failure = switch (e.problem) {
        BackupProblem.notABackup => t.restoreNotABackup,
        BackupProblem.wrongSecret => t.restoreWrong,
        BackupProblem.damaged => t.restoreDamaged,
        BackupProblem.notEmpty => t.restoreNotEmpty,
      };
    } on Object {
      failure = t.restoreFailed;
    }
    if (!mounted) return;
    setState(() => _busy = false);
    if (failure != null || result == null) {
      _announce(failure ?? t.restoreFailed);
      return;
    }
    _announce(t.restoreDone(result.entries));
    Navigator.of(context).pop(result);
  }

  List<Widget> _unlock(AppLocalizations t, BackupHeader header) {
    final when = DateFormat.yMMMMd(Localizations.localeOf(context).toString())
        .add_Hm();
    return [
      Text(
        t.restoreFile(when.format(header.created.toLocal())),
        style: Theme.of(context).textTheme.titleSmall,
      ),
      const SizedBox(height: 16),
      TextField(
        key: const ValueKey('restore-passphrase'),
        controller: _passphrase,
        enabled: !_busy,
        obscureText: true,
        autocorrect: false,
        enableSuggestions: false,
        decoration: InputDecoration(
          labelText: t.backupPassphrase,
          border: const OutlineInputBorder(),
        ),
        onSubmitted: (_) => _restore(),
      ),
      Padding(
        padding: const EdgeInsets.symmetric(vertical: 12),
        child: Center(child: Text(t.restoreOr)),
      ),
      if (PhotoScope.of(context).available) ...[
        FilledButton.tonalIcon(
          key: const ValueKey('restore-scan'),
          onPressed: _busy ? null : _scan,
          icon: const Icon(Icons.qr_code_scanner),
          label: Text(t.restoreScan),
        ),
        const SizedBox(height: 12),
      ],
      TextField(
        key: const ValueKey('restore-code'),
        controller: _code,
        enabled: !_busy,
        autocorrect: false,
        enableSuggestions: false,
        textCapitalization: TextCapitalization.characters,
        decoration: InputDecoration(
          labelText: t.restoreCodeLabel,
          border: const OutlineInputBorder(),
        ),
        onSubmitted: (_) => _restore(),
      ),
      const SizedBox(height: 16),
      Align(
        alignment: Alignment.centerRight,
        child: _busy
            ? Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const SizedBox.square(
                    dimension: 20,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  ),
                  const SizedBox(width: 12),
                  Flexible(child: Text(t.restoreWorking)),
                ],
              )
            : FilledButton(
                key: const ValueKey('restore'),
                onPressed: _restore,
                child: Text(t.restoreStart),
              ),
      ),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final header = _header;
    final found = _path != null && _path == widget.found;
    final pick = FilledButton.icon(
      key: const ValueKey('restore-pick'),
      onPressed: _busy ? null : _pick,
      icon: const Icon(Icons.file_open_outlined),
      label: Text(t.restorePick),
    );
    return Scaffold(
      appBar: AppBar(title: Text(found ? t.restoreFoundTitle : t.restoreOffer)),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
        children: [
          Text(found ? t.restoreFound : t.restoreIntro),
          const SizedBox(height: 16),
          if (!found) ...[
            Align(alignment: Alignment.centerLeft, child: pick),
            const SizedBox(height: 16),
          ],
          if (header != null) ..._unlock(t, header),
          if (found) ...[
            const SizedBox(height: 24),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                OutlinedButton(
                  key: const ValueKey('restore-skip'),
                  onPressed: _busy ? null : () => Navigator.of(context).pop(),
                  child: Text(t.restoreFoundSkip),
                ),
                if (BundleScope.of(context).available) pick,
              ],
            ),
          ],
        ],
      ),
    );
  }
}
