import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'backup_screen.dart';
import 'bundle_platform.dart';
import 'failure_dialog.dart';
import 'l10n.dart';
import 'pairing_platform.dart';
import 'photo_platform.dart';
import 'system_bars.dart';

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
/// what was restored, announced at the top with what to set again behind
/// the tap; that nothing was is announced too, why behind the tap.
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

  /// Whether the key typed is no recovery key; said under the field
  /// until it changes.
  var _codeInvalid = false;
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

  /// Announces that the Backup was not restored; the tap opens [cause],
  /// [fix] and [details].
  void _notRestored(String cause, String fix, String details) {
    final t = context.t;
    announceFailure(
      context,
      t.restoreNotDone,
      title: t.restoreNotDoneTitle,
      cause: cause,
      fix: fix,
      details: details,
    );
  }

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
      _notRestored(
        t.restoreNotABackup,
        t.restorePickFix,
        'backup: no Backup header in the file picked',
      );
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
      announceFailure(
        context,
        t.codeNotRead,
        title: t.codeNotReadTitle,
        cause: t.restoreNoCode,
        fix: t.restoreNoCodeFix,
        details: 'recovery key: none in the photo',
      );
      return;
    }
    setState(() {
      _codeInvalid = false;
      _code.text = code;
    });
  }

  Future<void> _restore() async {
    final t = context.t;
    final header = _header;
    final path = _path;
    if (header == null || path == null) return;
    final code = _code.text.trim();
    if (code.isNotEmpty && decodeRecoveryCode(code) == null) {
      setState(() => _codeInvalid = true);
      return;
    }
    setState(() {
      _codeInvalid = false;
      _busy = true;
    });
    // Why nothing was restored, what to do, and the full text.
    (String, String, String)? failure;
    RestoreResult? result;
    try {
      final key = await unlockBackup(
        header,
        passphrase: code.isEmpty ? _passphrase.text : null,
        recoveryCode: code.isEmpty ? null : code,
        derive: widget.derive,
      );
      if (key == null) {
        failure = (
          t.restoreWrong,
          t.restoreWrongFix,
          'backup: the passphrase or recovery key does not open it',
        );
      } else {
        result = await restoreBackup(widget.store, path, key);
      }
    } on BackupException catch (e) {
      final (cause, fix) = switch (e.problem) {
        BackupProblem.notABackup => (t.restoreNotABackup, t.restorePickFix),
        BackupProblem.wrongSecret => (t.restoreWrong, t.restoreWrongFix),
        BackupProblem.damaged => (t.restoreDamagedCause, t.restoreDamagedFix),
        BackupProblem.notEmpty => (t.restoreNotEmpty, t.restoreNotEmptyFix),
      };
      failure = (cause, fix, '$e');
    } on Object catch (e) {
      failure = (t.restoreFailedCause, t.restoreFailedFix, '$e');
    }
    if (!mounted) return;
    setState(() => _busy = false);
    if (failure != null || result == null) {
      final (cause, fix, details) =
          failure ??
          (t.restoreFailedCause, t.restoreFailedFix, 'backup: no result');
      _notRestored(cause, fix, details);
      return;
    }
    announceDone(context, t.restoreDoneShort, t.restoreDone(result.entries));
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
          errorText: _codeInvalid ? t.restoreInvalidCode : null,
        ),
        onChanged: (_) {
          if (_codeInvalid) setState(() => _codeInvalid = false);
        },
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
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.fromLTRB(16, 8, 16, 24),
        ),
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
