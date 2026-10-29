import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'app_lock.dart';
import 'auto_sync.dart';
import 'conflicts_screen.dart';
import 'failure_dialog.dart';
import 'l10n.dart';
import 'pairing_platform.dart';
import 'profile.dart';
import 'share_area_screen.dart';
import 'system_bars.dart';

/// The pairing host as [HostScreen] shows it: its pair code, whether it
/// has a network to be reached on, and whether too many wrong PINs
/// locked it. The app wraps the core's [PairingHost], tests a fake.
abstract interface class Hosting {
  /// The pair code: the whole fingerprint for the QR, its first bytes
  /// for typing.
  String pairCode({required bool typed});

  /// Whether it hosts on the loopback only: this Device is on no Wi-Fi
  /// or Ethernet, and no other Device can reach it.
  bool get offline;

  bool get lockedOut;

  Future<void> stop();
}

/// How hosting is started; the app passes [startPairingHost], tests a
/// fake.
typedef StartHosting = Future<Hosting> Function(
  Store store, {
  required String name,
  required Future<bool> Function(JoinAsk ask, ProfileChoice? profile) ask,
  Invite? inviting,
  required void Function(SyncSession session) onSession,
});

/// The core's [PairingHost] on this Device's LAN address.
Future<Hosting> startPairingHost(
  Store store, {
  required String name,
  required Future<bool> Function(JoinAsk ask, ProfileChoice? profile) ask,
  Invite? inviting,
  required void Function(SyncSession session) onSession,
}) async => _PairingHosting(
  await PairingHost.start(
    store,
    name: name,
    ask: ask,
    inviting: inviting,
    onSession: onSession,
  ),
);

class _PairingHosting implements Hosting {
  final PairingHost _host;

  _PairingHosting(this._host);

  @override
  String pairCode({required bool typed}) => _host.pairCode(typed: typed);

  @override
  bool get offline => _host.host.address.isLoopback;

  @override
  bool get lockedOut => _host.lockedOut;

  @override
  Future<void> stop() => _host.stop();
}

/// Shows this Device's pair code for another Device to scan or type
/// (ADR 0003): a QR with the typed code under it, for as long as the
/// page is open. A Device not paired yet is let in only once the user
/// says yes; while the page invites to a Shared Area ([inviting], ADR
/// 0013), the user confirms every joiner by its person's Profile. What a
/// sync brought is announced at the top; the page stays open, so the
/// other Device can finish fetching photos and recordings. Without a
/// local network, or once too many wrong PINs locked the host, it says
/// so and offers a new code. Where [onContinue] is given, a button goes
/// on once a session completed.
class HostScreen extends StatefulWidget {
  final Store store;

  /// The page's title.
  final String title;

  /// The invite to a Shared Area the code hands out, with the Area's
  /// name; null for a Device of one's own.
  final Invite? inviting;
  final String area;

  final StartHosting start;

  /// Hears every session that completed.
  final ValueChanged<SyncSession>? onSession;

  /// Goes on once a session completed, as a button under the code; none
  /// where the page stays until it is left.
  final VoidCallback? onContinue;

  /// Hears the user turn away a Device asking to join.
  final VoidCallback? onDeclined;

  const HostScreen({
    super.key,
    required this.store,
    required this.title,
    this.inviting,
    this.area = '',
    this.start = startPairingHost,
    this.onSession,
    this.onContinue,
    this.onDeclined,
  });

  @override
  State<HostScreen> createState() => _HostScreenState();
}

class _HostScreenState extends State<HostScreen> {
  Hosting? _host;

  /// The QR's modules for the host's code, dark as true.
  List<List<bool>> _qr = const [];
  List<String> _typed = const [];
  var _failed = false;
  var _locked = false;

  /// Counts the starts: a start that comes back after a newer one, or
  /// after the page left, stops what it started.
  var _generation = 0;
  var _started = false;
  Timer? _watch;

  /// The Profile the user confirmed last, for the announcement.
  ProfileChoice? _confirmed;

  /// Whether a session completed while the page shows.
  var _synced = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_started) return;
    _started = true;
    unawaited(_start());
    // The host does not say when it locks: looked at every second.
    _watch = Timer.periodic(const Duration(seconds: 1), (_) => _look());
  }

  @override
  void dispose() {
    _generation++;
    _watch?.cancel();
    unawaited(_host?.stop());
    super.dispose();
  }

  void _look() {
    final locked = _host?.lockedOut ?? false;
    if (locked != _locked) setState(() => _locked = locked);
  }

  Future<void> _start() async {
    final generation = ++_generation;
    final pairing = PairingScope.of(context);
    Hosting? host;
    try {
      final name = await pairing.deviceName();
      if (!mounted || generation != _generation) return;
      host = await widget.start(
        widget.store,
        name: name,
        ask: _ask,
        inviting: widget.inviting,
        onSession: _session,
      );
    } on Object {
      host = null;
    }
    if (!mounted || generation != _generation) {
      await host?.stop();
      return;
    }
    final started = host;
    setState(() {
      _host = started;
      _failed = started == null;
      _locked = started?.lockedOut ?? false;
      _qr = started == null
          ? const []
          : recoveryQr(started.pairCode(typed: false));
      _typed = started == null
          ? const []
          : pairCodeLines(started.pairCode(typed: true));
    });
  }

  /// A new code: the old host stops, a new one starts with a fresh PIN.
  Future<void> _restart() async {
    final old = _host;
    setState(() {
      _host = null;
      _failed = false;
      _locked = false;
    });
    _generation++;
    await old?.stop();
    if (!mounted) return;
    await _start();
  }

  /// Whether to let the Device asking in.
  Future<bool> _ask(JoinAsk ask, ProfileChoice? profile) async {
    if (!mounted) return false;
    if (widget.inviting != null) {
      // Invited without a Profile: the core refuses it whatever is said.
      if (profile == null) return false;
      final sure = await showDialog<bool>(
        context: context,
        builder: (_) => InviteConfirmDialog(area: widget.area, other: profile),
      );
      if (sure ?? false) _confirmed = profile;
      return sure ?? false;
    }
    final sure = await showDialog<bool>(
      context: context,
      builder: (_) => JoinAskDialog(name: ask.name),
    );
    if (!(sure ?? false)) widget.onDeclined?.call();
    return sure ?? false;
  }

  /// A session completed: that it did is said at the top, what it
  /// brought behind the tap.
  void _session(SyncSession session) {
    widget.onSession?.call(session);
    if (!mounted) return;
    setState(() => _synced = true);
    final t = context.t;
    final store = widget.store;
    final announcements = AnnouncementsScope.of(context);
    // The root navigator outlives this page: an announcement may be
    // tapped after leaving it.
    final navigator = Navigator.of(context, rootNavigator: true);
    final (done, brought) = widget.inviting != null
        ? (
            t.hostShareDone,
            t.shareDone(
              widget.area,
              profileDisplayName(t, _confirmed?.profile.name ?? ''),
            ),
          )
        : (t.hostSyncDone, t.syncCameIn(session.name, session.applied.length));
    announceDoneWith(announcements, navigator, done, brought);
    announceRefusedWith(
      t,
      announcements,
      navigator,
      session.report.refusedCount,
    );
    final conflicts = visibleConflicts(
      store,
      LockScope.of(context),
      session.conflicts,
    ).length;
    if (conflicts > 0) {
      announcements?.announce(
        t.hostConflicts(conflicts),
        onTap: () => navigator.push(
          MaterialPageRoute<void>(
            builder: (_) => ConflictsScreen(store: store),
          ),
        ),
      );
    }
    AutoSyncScope.of(context)?.devicesChanged();
    // Two Profiles changed by the user met: asked once, right here.
    if (store.profileQuestion != null) {
      unawaited(askWhichProfile(context, store));
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final host = _host;
    final text = Theme.of(context).textTheme;
    final List<Widget> children;
    if (_failed) {
      children = [
        Text(t.hostFailed, key: const ValueKey('host-failed')),
        const SizedBox(height: 16),
        _again(t.hostTryAgain),
      ];
    } else if (host == null) {
      children = [
        const Center(
          child: Padding(
            padding: EdgeInsets.all(24),
            child: CircularProgressIndicator(),
          ),
        ),
      ];
    } else if (host.offline) {
      children = [
        Text(t.hostNoNetwork, key: const ValueKey('host-offline')),
        const SizedBox(height: 16),
        _again(t.hostTryAgain),
      ];
    } else if (_locked) {
      children = [
        Text(t.hostLocked, key: const ValueKey('host-locked')),
        const SizedBox(height: 16),
        _again(t.hostNewCode),
      ];
    } else {
      children = [
        Text(widget.inviting == null ? t.hostIntro : t.hostInviteIntro),
        const SizedBox(height: 16),
        Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 320),
            child: AspectRatio(
              aspectRatio: 1,
              child: CustomPaint(
                key: const ValueKey('host-qr'),
                painter: QrPainter(_qr),
              ),
            ),
          ),
        ),
        const SizedBox(height: 16),
        Text(t.hostTypedCode, style: text.labelMedium),
        SelectableText(
          _typed.join('\n'),
          key: const ValueKey('host-code'),
          style: text.titleMedium?.copyWith(fontFamily: 'monospace'),
        ),
        const SizedBox(height: 16),
        Text(t.hostStays),
      ];
    }
    final onContinue = widget.onContinue;
    return Scaffold(
      appBar: AppBar(title: Text(widget.title)),
      body: ListView(
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.fromLTRB(16, 8, 16, 24),
        ),
        children: [
          ...children,
          if (_synced && onContinue != null) ...[
            const SizedBox(height: 16),
            Align(
              alignment: Alignment.centerLeft,
              child: FilledButton(
                key: const ValueKey('host-continue'),
                onPressed: onContinue,
                child: Text(t.hostContinue),
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _again(String label) => Align(
    alignment: Alignment.centerLeft,
    child: FilledButton.tonal(
      key: const ValueKey('host-again'),
      onPressed: _restart,
      child: Text(label),
    ),
  );
}

/// "Let this Device join?": the name the Device asking gives. Pops with
/// true to let it in.
class JoinAskDialog extends StatelessWidget {
  final String name;

  const JoinAskDialog({super.key, required this.name});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return AlertDialog(
      title: Text(name.isEmpty ? t.hostAskUnnamed : t.hostAskTitle(name)),
      content: Text(t.hostAskText),
      actions: [
        TextButton(
          key: const ValueKey('host-decline'),
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(t.cancel),
        ),
        FilledButton(
          key: const ValueKey('host-allow'),
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(t.hostAllow),
        ),
      ],
    );
  }
}

/// Draws a QR's [modules], dark on white whatever the theme, with the
/// quiet zone of four modules around it that readers need.
class QrPainter extends CustomPainter {
  final List<List<bool>> modules;

  const QrPainter(this.modules);

  static const _quiet = 4;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(Offset.zero & size, Paint()..color = Colors.white);
    if (modules.isEmpty) return;
    final count = modules.length + 2 * _quiet;
    final module = size.shortestSide / count;
    final dark = Paint()..color = Colors.black;
    for (var y = 0; y < modules.length; y++) {
      final row = modules[y];
      for (var x = 0; x < row.length; x++) {
        if (!row[x]) continue;
        canvas.drawRect(
          Rect.fromLTWH(
            (x + _quiet) * module,
            (y + _quiet) * module,
            // A hair wider, so no seams show between modules.
            module + 0.5,
            module + 0.5,
          ),
          dark,
        );
      }
    }
  }

  @override
  bool shouldRepaint(QrPainter oldDelegate) => oldDelegate.modules != modules;
}

/// Announces that [refused] changes were refused, when there were any;
/// how many and why behind the tap.
void announceRefusedWith(
  AppLocalizations t,
  Announcements? announcements,
  NavigatorState navigator,
  int refused,
) {
  if (refused <= 0) return;
  announceFailureWith(
    announcements,
    navigator,
    t.hostRefused(refused),
    title: t.hostRefusedTitle,
    cause: t.syncRefused(refused),
    fix: t.hostRefusedFix,
    details: 'refused: $refused',
  );
}
