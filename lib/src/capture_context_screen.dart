import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'context_platform.dart';
import 'l10n.dart';
import 'look.dart';

/// Switches for the parts of the Capture Context that need their own
/// permission: the calendar event running and the connected Bluetooth
/// device. Each is off until switched on here; switching one on explains
/// it and asks for its permission, which Capture itself never does.
/// Opens from Settings.
class CaptureContextScreen extends StatefulWidget {
  final Store store;

  const CaptureContextScreen({super.key, required this.store});

  @override
  State<CaptureContextScreen> createState() => _CaptureContextScreenState();
}

class _CaptureContextScreenState extends State<CaptureContextScreen> {
  /// Which kinds are on and allowed; empty until looked up.
  final _on = <ContextKind>{};

  @override
  void initState() {
    super.initState();
    _look();
  }

  /// A kind counts as on only while it is allowed too: a permission taken
  /// back in the system settings shows as off.
  Future<void> _look() async {
    final source = ContextScope.of(context);
    for (final kind in ContextKind.values) {
      final on = contextOn(widget.store, kind) && await source.granted(kind);
      if (!mounted) return;
      setState(() => on ? _on.add(kind) : _on.remove(kind));
    }
  }

  Future<void> _switch(ContextKind kind, bool on) async {
    final source = ContextScope.of(context);
    final announcements = AnnouncementsScope.of(context);
    final t = context.t;
    if (on &&
        !await source.granted(kind) &&
        (!mounted || !await explainAndAskContext(context, source, kind))) {
      if (!mounted) return;
      announcements?.announce(switch (kind) {
        ContextKind.calendar => t.calendarNotAllowed,
        ContextKind.bluetooth => t.bluetoothNotAllowed,
      });
      return;
    }
    if (!mounted) return;
    switch (kind) {
      case ContextKind.calendar:
        widget.store.calendarContextOn = on;
      case ContextKind.bluetooth:
        widget.store.bluetoothContextOn = on;
    }
    setState(() => on ? _on.add(kind) : _on.remove(kind));
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(title: Text(t.captureContextTitle)),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Text(t.captureContextIntro),
          const SizedBox(height: 16),
          for (final (kind, icon, title, detail) in [
            (
              ContextKind.calendar,
              Icons.event_outlined,
              t.calendarContextSwitch,
              t.calendarContextDetail,
            ),
            (
              ContextKind.bluetooth,
              Icons.bluetooth,
              t.bluetoothContextSwitch,
              t.bluetoothContextDetail,
            ),
          ])
            Padding(
              padding: const EdgeInsets.only(bottom: 12),
              child: Embossed(
                // The tile's ink shows on the card, not under it.
                child: Material(
                  type: MaterialType.transparency,
                  child: SwitchListTile(
                    secondary: Icon(icon),
                    title: Text(title),
                    subtitle: Text(detail),
                    value: _on.contains(kind),
                    onChanged: (on) => _switch(kind, on),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}
