import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'app_lock.dart';
import 'auto_sync.dart';
import 'bundle_platform.dart';
import 'sent_item_screen.dart';
import 'conflicts_screen.dart';
import 'failure_dialog.dart';
import 'folder_platform.dart';
import 'host_screen.dart';
import 'l10n.dart';
import 'look.dart';
import 'pairing_platform.dart';
import 'photo_platform.dart';
import 'place_names.dart';
import 'profile.dart';
import 'system_bars.dart';

/// How a sync is run; the app passes the core's [lanSync], tests a fake.
typedef Join = Future<SyncResult> Function(
  Store store,
  PairInfo info, {
  required String name,
});

/// Sync, inside Settings (UI laws): joins the Device the pair code
/// names, scanned from a photo of its screen or typed in, or shows this
/// Device's code for the other to join ([HostScreen]), leads to the
/// Conflicts, and lists the paired Devices with when they last synced,
/// each of which can be forgotten, and those that host kept from syncing
/// on their own. The shared folder is picked and forgotten here too,
/// with how its last sync went, and a bundle file is sent, for a paired
/// Device or of everything, and one imported. What a sync did, or why it
/// did not, is announced at the top; Conflicts it found are announced
/// with a way to them.
///
/// On the first start, adding a Device of one's own ([firstStart]), it
/// shows only the ways to pair and pops with true once a sync went
/// through, either way round, and with false once one failed or the
/// user turned the Device asking away.
class SyncScreen extends StatefulWidget {
  final Store store;
  final Join join;
  final StartHosting host;
  final bool firstStart;

  const SyncScreen({
    super.key,
    required this.store,
    this.join = lanSync,
    this.host = startPairingHost,
    this.firstStart = false,
  });

  @override
  State<SyncScreen> createState() => _SyncScreenState();
}

class _SyncScreenState extends State<SyncScreen> {
  final _code = TextEditingController();

  /// Whether the code typed is no pair code, or one of a later version;
  /// said under the field until it changes.
  var _codeInvalid = false;
  var _busy = false;
  late var _paired = widget.store.pairedDevices();
  var _conflicts = 0;

  /// The shared folder's name for people, once known.
  String? _folderName;
  var _folderBusy = false;
  var _named = false;

  /// The paired Device the next bundle is for; everything when null.
  String? _bundleFor;
  var _bundleBusy = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _countConflicts();
    if (!_named) {
      _named = true;
      unawaited(_nameFolder());
    }
  }

  Future<void> _nameFolder() async {
    final folder = widget.store.syncFolder;
    if (folder == null) return;
    final name = await FolderScope.of(context).displayName(folder);
    if (!mounted) return;
    setState(() => _folderName = name);
  }

  /// Picks the shared folder and syncs through it at once.
  Future<void> _pickFolder() async {
    final folders = FolderScope.of(context);
    final picked = await folders.pick();
    if (!mounted || picked == null) return;
    widget.store.chooseSyncFolder(picked);
    final name = await folders.displayName(picked);
    if (!mounted) return;
    final auto = AutoSyncScope.of(context);
    setState(() {
      _folderName = name;
      _folderBusy = true;
    });
    auto?.devicesChanged();
    try {
      await (auto?.syncFolder() ??
          folderRound(widget.store, folders.open(picked)));
    } on Object {
      // Kept as the status; the next round tries again.
    }
    if (!mounted) return;
    setState(() {
      _folderBusy = false;
      _countConflicts();
    });
  }

  void _forgetFolder() {
    widget.store.forgetSyncFolder();
    setState(() => _folderName = null);
    AutoSyncScope.of(context)?.devicesChanged();
  }

  /// How the shared folder's last sync went, in words.
  String _folderStatus(AppLocalizations t, DateFormat when) {
    final status = widget.store.folderStatus;
    if (_folderBusy) return t.syncSyncing;
    if (status == null) return t.folderNotYet;
    final at = when.format(status.at.toLocal());
    if (status.failed) return t.folderUnreachable(at);
    final synced = t.folderSynced(at, status.entriesIn);
    return status.unreadable == 0
        ? synced
        : '$synced\n${t.folderUnreadable(status.unreadable)}';
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

  /// Shows this Device's code until the user comes back; on the first
  /// start, until a sync went through and the user goes on, or the user
  /// turned the Device asking away.
  Future<void> _showCode() async {
    final store = widget.store;
    final navigator = Navigator.of(context);
    final first = widget.firstStart;
    var synced = false;
    var declined = false;
    late final Route<void> route;
    route = MaterialPageRoute<void>(
      builder: (context) => HostScreen(
        store: store,
        title: context.t.hostTitle,
        start: widget.host,
        onSession: first ? (_) => synced = true : null,
        onContinue: first ? navigator.pop : null,
        onDeclined: first
            ? () {
                declined = true;
                if (route.isCurrent) navigator.pop();
              }
            : null,
      ),
    );
    await navigator.push(route);
    if (!mounted) return;
    if (first && (synced || declined)) {
      navigator.pop(synced);
      return;
    }
    setState(() {
      _paired = store.pairedDevices();
      _countConflicts();
    });
  }

  /// Takes a photo of the other Device's screen and reads the code in it.
  Future<void> _scan() async {
    final photos = PhotoScope.of(context);
    final pairing = PairingScope.of(context);
    final photo = await photos.takePhoto();
    if (!mounted || photo == null) return;
    final code = await pairing.readQr(photo);
    if (!mounted) return;
    if (code == null ||
        (decodePairCode(code) == null && !pairCodeNeedsUpdate(code))) {
      announceCodeNotRead(context);
      return;
    }
    _code.text = code;
    await _sync();
  }

  Future<void> _sync() async {
    final t = context.t;
    final info = decodePairCode(_code.text);
    if (info == null) {
      setState(() => _codeInvalid = true);
      return;
    }
    final pairing = PairingScope.of(context);
    setState(() {
      _codeInvalid = false;
      _busy = true;
    });
    var synced = false;
    try {
      final name = await pairing.deviceName();
      final result = await widget.join(widget.store, info, name: name);
      if (!mounted) return;
      // The root navigator outlives this screen: an announcement may be
      // tapped after leaving it.
      final navigator = Navigator.of(context, rootNavigator: true);
      final announcements = AnnouncementsScope.of(context);
      final brought = [
        t.syncCameIn(result.session.name, result.session.applied.length),
      ];
      final shared = result.session.sharedArea == null
          ? null
          : Places(widget.store).areaById(result.session.sharedArea ?? '');
      if (shared != null) {
        // Who shared it, as their Profile shows (ADR 0017).
        final person = widget.store
            .grantsOf(shared.id)
            ?.personOf(result.session.peer);
        final who = person == null
            ? result.session.name
            : profileDisplayName(t, widget.store.personShown(person).name);
        brought.add(t.sharedWithYou(who, areaName(t, shared)));
      }
      announceDoneWith(
        announcements,
        navigator,
        t.hostSyncDone,
        brought.join('\n'),
      );
      announceRefusedWith(
        t,
        announcements,
        navigator,
        result.session.report.refusedCount,
      );
      final conflicts = visibleConflicts(
        widget.store,
        LockScope.of(context),
        result.session.conflicts,
      ).length;
      if (conflicts > 0) {
        _announce(
          t.hostConflicts(conflicts),
          onTap: () => _openConflicts(navigator),
        );
      }
      _code.clear();
      synced = true;
    } on Object catch (e) {
      // Whatever else broke on the way: nothing half-taken stays.
      if (!mounted) return;
      announceSyncFailed(
        context,
        e,
        text: t.syncNotDone,
        title: t.syncNotDoneTitle,
      );
    } finally {
      if (mounted) {
        setState(() {
          _busy = false;
          _paired = widget.store.pairedDevices();
          _countConflicts();
        });
        AutoSyncScope.of(context)?.devicesChanged();
      }
    }
    // Two Profiles changed by the user met: asked once, right here.
    if (mounted && widget.store.profileQuestion != null) {
      await askWhichProfile(context, widget.store);
    }
    // The first start goes on, or back to its first screen.
    if (mounted && widget.firstStart) Navigator.of(context).pop(synced);
  }

  void _forget(PairedDevice device) {
    widget.store.forgetDevice(device.id);
    setState(() => _paired = widget.store.pairedDevices());
    AutoSyncScope.of(context)?.devicesChanged();
  }

  void _setAuto(PairedDevice device, bool on) {
    widget.store.setAutoSync(device.id, on);
    setState(() {});
    AutoSyncScope.of(context)?.devicesChanged();
  }

  /// The shared folder: picked or not, how its last sync went, and the
  /// way to forget it.
  List<Widget> _folder(AppLocalizations t, DateFormat when) {
    final folder = widget.store.syncFolder;
    return [
      const SizedBox(height: 16),
      Text(t.folderTitle, style: Theme.of(context).textTheme.titleSmall),
      Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Text(t.folderIntro),
      ),
      if (folder == null)
        Align(
          alignment: Alignment.centerLeft,
          child: FilledButton.tonalIcon(
            key: const ValueKey('pick-folder'),
            onPressed: _pickFolder,
            icon: const Icon(Icons.folder_open),
            label: Text(t.folderPick),
          ),
        )
      else
        ControlBelow(
          key: const ValueKey('folder'),
          padding: const EdgeInsets.symmetric(vertical: 8),
          leading: const Icon(Icons.folder_shared_outlined),
          title: Text(_folderName ?? ''),
          subtitle: Text(_folderStatus(t, when)),
          control: TextButton(
            key: const ValueKey('forget-folder'),
            onPressed: _folderBusy ? null : _forgetFolder,
            child: Text(t.folderForget),
          ),
        ),
    ];
  }

  /// Writes a bundle for the chosen Device, or of everything, and offers
  /// it to the app the user picks; nothing is written when there is
  /// nothing new for that Device.
  Future<void> _sendBundle() async {
    final t = context.t;
    final bundles = BundleScope.of(context);
    final store = widget.store;
    final device = _bundleFor;
    final since = device == null ? const <String, int>{} : store.heldBy(device);
    final nothing = device != null && store.isSharedPeer(device)
        ? store.entriesFor(device, since).isEmpty
        : store.entriesSince(since).isEmpty;
    if (nothing) {
      _announce(t.bundleNothingToSend);
      return;
    }
    setState(() => _bundleBusy = true);
    try {
      final path = await bundles.exportPath(
        bundleFileName(store.clock().toLocal()),
      );
      if (!mounted) return;
      if (path == null) {
        announceBundleNotSent(context, 'bundle: no path to write the file to');
        return;
      }
      await writeBundle(store, path, forDevice: device);
      final shared = await bundles.share(path);
      if (!mounted) return;
      if (shared == ShareOutcome.noApp) {
        announceBundleNotSent(
          context,
          'bundle: no app took the file',
          noApp: true,
        );
      }
    } on Object catch (e) {
      if (!mounted) return;
      announceBundleNotSent(context, '$e');
    } finally {
      if (mounted) setState(() => _bundleBusy = false);
    }
  }

  /// Imports a bundle the user picks.
  Future<void> _importBundle() async {
    final path = await BundleScope.of(context).pick();
    if (!mounted || path == null) return;
    setState(() => _bundleBusy = true);
    final conflicts = await openReceivedFile(context, widget.store, path);
    if (!mounted) return;
    if (widget.store.profileQuestion != null) {
      await askWhichProfile(context, widget.store);
      if (!mounted) return;
    }
    if (conflicts > 0) {
      // The root navigator outlives this screen: the announcement may be
      // tapped after leaving it.
      final navigator = Navigator.of(context, rootNavigator: true);
      _announce(
        context.t.hostConflicts(conflicts),
        onTap: () => _openConflicts(navigator),
      );
    }
    setState(() {
      _bundleBusy = false;
      _countConflicts();
    });
  }

  /// The bundle file: for which paired Device, or everything, and the
  /// ways to send one and to import one.
  List<Widget> _bundle(AppLocalizations t) {
    // A Device forgotten meanwhile is no choice any more.
    final chosen = _paired.any((d) => d.id == _bundleFor) ? _bundleFor : null;
    return [
      const SizedBox(height: 16),
      Text(t.bundleTitle, style: Theme.of(context).textTheme.titleSmall),
      Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Text(t.bundleIntro),
      ),
      ControlBelow(
        padding: EdgeInsets.zero,
        title: Text(t.bundleFor),
        control: DropdownButton<String?>(
          key: const ValueKey('bundle-for'),
          isExpanded: true,
          value: chosen,
          items: [
            for (final device in _paired)
              DropdownMenuItem(value: device.id, child: Text(device.name)),
            DropdownMenuItem(child: Text(t.bundleEverything)),
          ],
          onChanged: _bundleBusy
              ? null
              : (id) => setState(() => _bundleFor = id),
        ),
      ),
      const SizedBox(height: 8),
      Wrap(
        spacing: 8,
        runSpacing: 8,
        children: [
          FilledButton.tonalIcon(
            key: const ValueKey('send-bundle'),
            onPressed: _bundleBusy ? null : _sendBundle,
            icon: const Icon(Icons.send_outlined),
            label: Text(t.bundleSend),
          ),
          FilledButton.tonalIcon(
            key: const ValueKey('import-bundle'),
            onPressed: _bundleBusy ? null : _importBundle,
            icon: const Icon(Icons.file_open_outlined),
            label: Text(t.bundleImport),
          ),
        ],
      ),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final canScan = PhotoScope.of(context).available;
    final when = DateFormat.yMd(Localizations.localeOf(context).toString())
        .add_Hm();
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.firstStart ? t.firstStartDevice : t.syncTitle),
      ),
      body: ListView(
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.fromLTRB(16, 8, 16, 24),
        ),
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
              errorText: _codeInvalid ? invalidCodeText(t, _code.text) : null,
            ),
            onChanged: (_) {
              if (_codeInvalid) setState(() => _codeInvalid = false);
            },
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
          Text(t.syncShowIntro),
          const SizedBox(height: 8),
          Align(
            alignment: Alignment.centerLeft,
            child: FilledButton.tonalIcon(
              key: const ValueKey('show-code'),
              onPressed: _busy ? null : _showCode,
              icon: const Icon(Icons.qr_code_2),
              label: Text(t.syncShowCode),
            ),
          ),
          if (!widget.firstStart) ..._rest(t, when),
        ],
      ),
    );
  }

  /// Everything past the ways to pair, which the first start leaves
  /// out.
  List<Widget> _rest(AppLocalizations t, DateFormat when) => [
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
    Text(t.pairedDevicesTitle, style: Theme.of(context).textTheme.titleSmall),
    Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Text(_paired.isEmpty ? t.pairedNone : t.syncAutoIntro),
    ),
    for (final device in _paired) ...[
      ControlBelow(
        padding: const EdgeInsets.symmetric(vertical: 8),
        leading: Icon(
          device.shared ? Icons.people_outline : Icons.devices_outlined,
        ),
        title: Text(device.name),
        subtitle: Text(
          [
            if (device.shared) t.pairedPerson,
            t.pairedLastSync(when.format(device.lastSync.toLocal())),
          ].join('\n'),
        ),
        control: TextButton(
          onPressed: () => _forget(device),
          child: Text(t.forgetDevice),
        ),
      ),
      // Only a Device that hosts can be found and synced with.
      if (device.fingerprint.isNotEmpty)
        SwitchBelow(
          switchKey: ValueKey('auto-${device.id}'),
          // In line with the Device's name above.
          padding: const EdgeInsetsDirectional.fromSTEB(40, 0, 0, 8),
          title: Text(t.syncAutomatically),
          value: widget.store.autoSyncOn(device.id),
          onChanged: (on) => _setAuto(device, on),
        ),
    ],
    if (FolderScope.of(context).available) ..._folder(t, when),
    if (BundleScope.of(context).available) ..._bundle(t),
    const SizedBox(height: 16),
    Text(t.syncSettingsStay),
  ];
}

/// What a sync that needs an update ([UpdateNotices]) says at the top:
/// which Device needs it, by the name it is paired under where known.
String updateNoticeText(
  AppLocalizations t,
  Store store,
  String source,
  UpdateNeeded where,
) {
  if (source == UpdateNotices.folderSource) return t.folderNeedsUpdate;
  if (source == UpdateNotices.settingsSource) return t.settingsNeedUpdate;
  final name = store.pairedDevice(source)?.name ?? '';
  return switch (where) {
    UpdateNeeded.here when name.isNotEmpty => t.syncUpdateHereWith(name),
    UpdateNeeded.there when name.isNotEmpty => t.syncUpdateThereWith(name),
    UpdateNeeded.here => t.syncFailUpdateHere,
    UpdateNeeded.there => t.syncFailUpdateThere,
  };
}

/// Why a sync did not happen and what to do, one sentence each; null
/// for [SyncFailure.cancelled], which the user chose here.
(String, String)? syncFailureWhy(AppLocalizations t, SyncFailure failure) =>
    switch (failure) {
      SyncFailure.notPrivate => (t.syncNotPrivateCause, t.syncNotPrivateFix),
      SyncFailure.unreachable => (t.syncUnreachableCause, t.syncUnreachableFix),
      SyncFailure.wrongHost => (t.syncFailWrongHost, t.syncWrongHostFix),
      SyncFailure.wrongPin => (t.syncWrongPinCause, t.syncWrongPinFix),
      SyncFailure.locked => (t.syncLockedCause, t.syncLockedFix),
      SyncFailure.declined => (t.syncFailDeclined, t.syncDeclinedFix),
      SyncFailure.cancelled => null,
      SyncFailure.updateHere => (t.syncFailUpdateHere, t.updateFix),
      SyncFailure.updateThere => (t.syncFailUpdateThere, t.updateFix),
      SyncFailure.failed => (t.syncBrokeOffCause, t.nothingLostFix),
    };

/// Announces that a sync, or sharing an Area in person, did not go
/// through, in [text]; why and what to do behind the tap, under [title],
/// as [error] says. An invite the user did not confirm only says that
/// nothing was shared.
void announceSyncFailed(
  BuildContext context,
  Object error, {
  required String text,
  required String title,
}) {
  final t = context.t;
  final failure = error is SyncException ? error.failure : SyncFailure.failed;
  final why = syncFailureWhy(t, failure);
  if (why == null) {
    AnnouncementsScope.of(context)?.announce(t.shareCancelled);
    return;
  }
  final (cause, fix) = why;
  announceFailure(
    context,
    text,
    title: title,
    cause: cause,
    fix: fix,
    details: error is SyncException ? 'sync: ${failure.name}' : '$error',
  );
}

/// What the code field says under a [code] that does not join: that it
/// is none, or that it comes from a later version (ADR 0034).
String invalidCodeText(AppLocalizations t, String code) =>
    pairCodeNeedsUpdate(code) ? t.pairCodeNeedsUpdate : t.syncInvalidCode;

/// Announces that no pair code was found in the photo taken; what to do
/// behind the tap.
void announceCodeNotRead(BuildContext context) {
  final t = context.t;
  announceFailure(
    context,
    t.codeNotRead,
    title: t.codeNotReadTitle,
    cause: t.syncNoCode,
    fix: t.syncNoCodeFix,
    details: 'pair code: none in the photo',
  );
}
