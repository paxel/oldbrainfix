import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'calendar_mirror.dart';
import 'capture_context_screen.dart';
import 'context_platform.dart';
import 'l10n.dart';
import 'look.dart';
import 'settings_screen.dart';
import 'system_bars.dart';

/// Opens the Calendar page; returns once the user left it.
Future<void> openCalendarPage(BuildContext context, Store store) =>
    Navigator.of(context).push(
      MaterialPageRoute<void>(builder: (_) => CalendarScreen(store: store)),
    );

/// Whether this Device has anything for the Calendar page: a calendar
/// to read the running appointment from, or one to write into. The
/// Agenda's button and the row in Settings only show where it has.
bool hasCalendarPage(BuildContext context) =>
    ContextScope.of(context).available ||
    (CalendarMirrorScope.of(context)?.platform.available ?? false);

/// Everything between the app and this Device's calendar, in two
/// sections: what is read from it, the appointment running when a Note
/// is captured, and what is written into it, the mirror of the Routines
/// and Reminders. Opens from the Agenda's bar and from Settings.
class CalendarScreen extends StatelessWidget {
  final Store store;

  const CalendarScreen({super.key, required this.store});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final mirror = CalendarMirrorScope.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(t.calendarPageTitle)),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.only(bottom: 24)),
        children: [
          if (ContextScope.of(context).available) ...[
            SettingsHeading(t.calendarReadHeading),
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
              child: ContextSwitch(store: store, kind: ContextKind.calendar),
            ),
          ],
          if (mirror != null && mirror.platform.available) ...[
            SettingsHeading(t.calendarWriteHeading),
            MirrorSwitch(mirror: mirror),
          ],
        ],
      ),
    );
  }
}

/// The calendar mirror's switch: off until the user turns it on, which
/// explains it, asks for calendar access and lets the user pick the
/// calendar where the finger is. While on it names that calendar, and a
/// tap picks another. Turning it off removes the events it wrote.
class MirrorSwitch extends StatefulWidget {
  final CalendarMirror mirror;

  const MirrorSwitch({super.key, required this.mirror});

  @override
  State<MirrorSwitch> createState() => _MirrorSwitchState();
}

class _MirrorSwitchState extends State<MirrorSwitch> {
  late MirrorCalendar? _calendar = widget.mirror.calendar;
  var _busy = false;

  @override
  void initState() {
    super.initState();
    widget.mirror.addListener(_mirrorChanged);
  }

  @override
  void dispose() {
    widget.mirror.removeListener(_mirrorChanged);
    super.dispose();
  }

  void _mirrorChanged() {
    if (!mounted) return;
    setState(() => _calendar = widget.mirror.calendar);
  }

  /// Runs [work] with the switch held still meanwhile.
  Future<void> _hold(Future<void> Function() work) async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      await work();
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  /// Turns the mirror on, or moves it to another calendar.
  Future<void> _choose() => _hold(() async {
    final platform = widget.mirror.platform;
    final announcements = AnnouncementsScope.of(context);
    final t = context.t;
    if (!await platform.granted() &&
        (!mounted || !await explainAndAskMirror(context, platform))) {
      if (mounted) announcements?.announce(t.calendarMirrorNotAllowed);
      return;
    }
    final calendars = await platform.calendars();
    if (!mounted) return;
    if (calendars.isEmpty) {
      announcements?.announce(t.calendarMirrorNoCalendar);
      return;
    }
    final picked = calendars.length == 1
        ? calendars.single
        : await _pick(calendars);
    if (picked == null || !mounted) return;
    if (!await widget.mirror.turnOn((id: picked.id, name: picked.name))) {
      announcements?.announce(t.calendarMirrorNotRemoved);
    }
  });

  /// Opens the [calendars] to pick from where the finger is.
  Future<DeviceCalendar?> _pick(List<DeviceCalendar> calendars) async {
    final box = context.findRenderObject() as RenderBox?;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (box == null || overlay == null) return null;
    final at = box.localToGlobal(box.size.bottomLeft(Offset.zero));
    final index = await showMenu<int>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        for (final (i, calendar) in calendars.indexed)
          PopupMenuItem(
            value: i,
            child: ListTile(
              title: Text(calendar.name),
              subtitle:
                  calendar.account.isEmpty || calendar.account == calendar.name
                  ? null
                  : Text(calendar.account),
            ),
          ),
      ],
    );
    return index == null ? null : calendars[index];
  }

  /// Turns the mirror off; its events leave the calendar. Without access
  /// it asks once more, as they cannot be removed otherwise; it was
  /// explained when the mirror was turned on.
  Future<void> _turnOff() => _hold(() async {
    final mirror = widget.mirror;
    final platform = mirror.platform;
    final announcements = AnnouncementsScope.of(context);
    final t = context.t;
    if (await mirror.turnOff()) return;
    if (!await platform.granted() &&
        await platform.request() &&
        await mirror.turnOff()) {
      return;
    }
    if (!mounted) return;
    announcements?.announce(t.calendarMirrorNotRemoved);
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final calendar = _calendar;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
      child: Embossed(
        // The tile's ink shows on the card, not under it.
        child: Material(
          type: MaterialType.transparency,
          child: SwitchBelow(
            leading: const Icon(Icons.event_repeat),
            title: Text(t.calendarMirrorSwitch),
            subtitle: Text(
              calendar == null
                  ? t.calendarMirrorDetail
                  : t.calendarMirrorIn(calendar.name),
            ),
            value: calendar != null,
            onChanged: _busy ? null : (on) => on ? _choose() : _turnOff(),
            // While on, a tap on the text picks another calendar; only
            // the switch turns the mirror on or off.
            onTap: _busy || calendar == null ? null : _choose,
          ),
        ),
      ),
    );
  }
}
