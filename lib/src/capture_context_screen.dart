import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'context_platform.dart';
import 'failure_dialog.dart';
import 'l10n.dart';
import 'look.dart';
import 'system_bars.dart';

/// The switch for the part of the Capture Context that needs its own
/// permission: the connected Bluetooth device. It is off until switched
/// on here; switching it on explains it and asks for its permission,
/// which Capture itself never does. Opens from Settings; the calendar
/// event running has its switch on the Calendar page.
class CaptureContextScreen extends StatelessWidget {
  final Store store;

  const CaptureContextScreen({super.key, required this.store});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(title: Text(t.captureContextTitle)),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
        children: [
          Text(t.captureContextIntro),
          const SizedBox(height: 16),
          ContextSwitch(store: store, kind: ContextKind.bluetooth),
        ],
      ),
    );
  }
}

/// The switch of one part of the Capture Context, on an embossed card:
/// off until switched on, which explains it and asks for its permission
/// first. It counts as on only while it is allowed too: a permission
/// taken back in the system settings shows as off.
class ContextSwitch extends StatefulWidget {
  final Store store;
  final ContextKind kind;

  const ContextSwitch({super.key, required this.store, required this.kind});

  @override
  State<ContextSwitch> createState() => _ContextSwitchState();
}

class _ContextSwitchState extends State<ContextSwitch> {
  /// Whether it is on and allowed; off until looked up.
  var _on = false;

  @override
  void initState() {
    super.initState();
    _look();
  }

  Future<void> _look() async {
    final source = ContextScope.of(context);
    final kind = widget.kind;
    final on = contextOn(widget.store, kind) && await source.granted(kind);
    if (!mounted) return;
    setState(() => _on = on);
  }

  Future<void> _switch(bool on) async {
    final source = ContextScope.of(context);
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context, rootNavigator: true);
    final t = context.t;
    final kind = widget.kind;
    if (on &&
        !await source.granted(kind) &&
        (!mounted || !await explainAndAskContext(context, source, kind))) {
      if (!mounted) return;
      announceFailureWith(
        announcements,
        navigator,
        t.contextNotOn,
        title: t.contextNotOnTitle,
        cause: switch (kind) {
          ContextKind.calendar => t.agendaCalendarNotAllowedCause,
          ContextKind.bluetooth => t.bluetoothNotAllowed,
        },
        fix: t.agendaCalendarAllowFix,
        details: 'capture context: ${kind.name} not allowed',
      );
      return;
    }
    if (!mounted) return;
    switch (kind) {
      case ContextKind.calendar:
        widget.store.calendarContextOn = on;
      case ContextKind.bluetooth:
        widget.store.bluetoothContextOn = on;
    }
    setState(() => _on = on);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final kind = widget.kind;
    final (icon, title, detail) = switch (kind) {
      ContextKind.calendar => (
        Icons.event_outlined,
        t.calendarContextSwitch,
        t.calendarContextDetail,
      ),
      ContextKind.bluetooth => (
        Icons.bluetooth,
        t.bluetoothContextSwitch,
        t.bluetoothContextDetail,
      ),
    };
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Embossed(
        // The tile's ink shows on the card, not under it.
        child: Material(
          type: MaterialType.transparency,
          child: SwitchBelow(
            switchKey: ValueKey('context-${kind.name}'),
            leading: Icon(icon),
            title: Text(title),
            subtitle: Text(detail),
            value: _on,
            onChanged: _switch,
          ),
        ),
      ),
    );
  }
}
