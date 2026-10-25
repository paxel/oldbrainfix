import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'app_lock.dart';
import 'conflicts_screen.dart';
import 'l10n.dart';
import 'pairing_platform.dart';
import 'photo_platform.dart';

/// How a sync is run; the app passes the core's [lanSync], tests a fake.
typedef Join = Future<SyncResult> Function(
  Store store,
  PairInfo info, {
  required String name,
});

/// Sync, inside Settings (UI laws): joins the computer the pair code
/// names, scanned from a photo of its screen or typed in, leads to the
/// Conflicts, and lists the paired Devices, each of which can be
/// forgotten. What a sync did, or why it did not, is announced at the
/// top; Conflicts it found are announced with a way to them.
class SyncScreen extends StatefulWidget {
  final Store store;
  final Join join;

  const SyncScreen({super.key, required this.store, this.join = lanSync});

  @override
  State<SyncScreen> createState() => _SyncScreenState();
}

class _SyncScreenState extends State<SyncScreen> {
  final _code = TextEditingController();
  var _busy = false;
  late var _paired = widget.store.pairedDevices();
  var _conflicts = 0;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _countConflicts();
  }

  /// The Conflicts the list shows, as many as it shows.
  void _countConflicts() => _conflicts = visibleConflicts(
    widget.store,
    LockScope.of(context),
    widget.store.conflicts(),
  ).length;

  @override
  void dispose() {
    _code.dispose();
    super.dispose();
  }

  void _announce(String text, {VoidCallback? onTap}) =>
      AnnouncementsScope.of(context)?.announce(text, onTap: onTap);

  Future<void> _openConflicts(NavigatorState navigator) async {
    // Held here: the announcement may be tapped after this screen left.
    final store = widget.store;
    await navigator.push(
      MaterialPageRoute<void>(builder: (_) => ConflictsScreen(store: store)),
    );
    if (!mounted) return;
    setState(_countConflicts);
  }

  /// Takes a photo of the computer's screen and reads the code in it.
  Future<void> _scan() async {
    final photos = PhotoScope.of(context);
    final pairing = PairingScope.of(context);
    final photo = await photos.takePhoto();
    if (!mounted || photo == null) return;
    final code = await pairing.readQr(photo);
    if (!mounted) return;
    if (code == null || decodePairCode(code) == null) {
      _announce(context.t.syncNoCode);
      return;
    }
    _code.text = code;
    await _sync();
  }

  Future<void> _sync() async {
    final t = context.t;
    final info = decodePairCode(_code.text);
    if (info == null) {
      _announce(t.syncInvalidCode);
      return;
    }
    final pairing = PairingScope.of(context);
    setState(() => _busy = true);
    try {
      final name = await pairing.deviceName();
      final result = await widget.join(widget.store, info, name: name);
      if (!mounted) return;
      _announce(
        t.syncCameIn(result.session.name, result.session.applied.length),
      );
      final refused = result.session.report.refusedCount;
      if (refused > 0) _announce(t.syncRefused(refused));
      final conflicts = visibleConflicts(
        widget.store,
        LockScope.of(context),
        result.session.conflicts,
      ).length;
      if (conflicts > 0) {
        // The root navigator outlives this screen: the announcement may
        // be tapped after leaving it.
        final navigator = Navigator.of(context, rootNavigator: true);
        _announce(
          t.syncConflicts(conflicts),
          onTap: () => _openConflicts(navigator),
        );
      }
      _code.clear();
    } on SyncException catch (e) {
      if (!mounted) return;
      _announce(_failure(t, e.failure));
    } on Object {
      // Whatever else broke on the way: nothing half-taken stays.
      if (!mounted) return;
      _announce(t.syncFailFailed);
    } finally {
      if (mounted) {
        setState(() {
          _busy = false;
          _paired = widget.store.pairedDevices();
          _countConflicts();
        });
      }
    }
  }

  void _forget(PairedDevice device) {
    widget.store.forgetDevice(device.id);
    setState(() => _paired = widget.store.pairedDevices());
  }

  static String _failure(AppLocalizations t, SyncFailure failure) =>
      switch (failure) {
        SyncFailure.notPrivate => t.syncFailNotPrivate,
        SyncFailure.unreachable => t.syncFailUnreachable,
        SyncFailure.wrongHost => t.syncFailWrongHost,
        SyncFailure.wrongPin => t.syncFailWrongPin,
        SyncFailure.locked => t.syncFailLocked,
        SyncFailure.declined => t.syncFailDeclined,
        SyncFailure.format => t.syncFailFormat,
        SyncFailure.failed => t.syncFailFailed,
      };

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final canScan = PhotoScope.of(context).available;
    final when = DateFormat.yMd(Localizations.localeOf(context).toString())
        .add_Hm();
    return Scaffold(
      appBar: AppBar(title: Text(t.syncTitle)),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
        children: [
          Text(t.syncJoinIntro),
          const SizedBox(height: 16),
          if (canScan)
            FilledButton.icon(
              key: const ValueKey('scan'),
              onPressed: _busy ? null : _scan,
              icon: const Icon(Icons.qr_code_scanner),
              label: Text(t.syncScan),
            ),
          const SizedBox(height: 16),
          TextField(
            key: const ValueKey('code'),
            controller: _code,
            enabled: !_busy,
            autocorrect: false,
            enableSuggestions: false,
            textCapitalization: TextCapitalization.none,
            decoration: InputDecoration(
              labelText: t.syncCodeLabel,
              border: const OutlineInputBorder(),
            ),
            onSubmitted: (_) => _sync(),
          ),
          const SizedBox(height: 8),
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
                      Text(t.syncSyncing),
                    ],
                  )
                : FilledButton.tonal(
                    key: const ValueKey('join'),
                    onPressed: _sync,
                    child: Text(t.syncJoin),
                  ),
          ),
          const SizedBox(height: 16),
          ListTile(
            key: const ValueKey('conflicts'),
            contentPadding: EdgeInsets.zero,
            leading: const Icon(Icons.call_split),
            title: Text(t.conflictsTitle),
            subtitle: Text(t.conflictsEntry(_conflicts)),
            onTap: () => _openConflicts(Navigator.of(context)),
          ),
          const SizedBox(height: 8),
          Text(
            t.pairedDevicesTitle,
            style: Theme.of(context).textTheme.titleSmall,
          ),
          if (_paired.isEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 8),
              child: Text(t.pairedNone),
            ),
          for (final device in _paired)
            ListTile(
              contentPadding: EdgeInsets.zero,
              leading: const Icon(Icons.computer_outlined),
              title: Text(device.name),
              subtitle: Text(
                t.pairedLastSync(when.format(device.lastSync.toLocal())),
              ),
              trailing: TextButton(
                onPressed: () => _forget(device),
                child: Text(t.forgetDevice),
              ),
            ),
          const SizedBox(height: 16),
          Text(t.syncSettingsStay),
        ],
      ),
    );
  }
}
