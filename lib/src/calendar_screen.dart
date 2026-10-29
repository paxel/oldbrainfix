import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'appointments.dart';
import 'calendar_mirror.dart';
import 'capture_context_screen.dart';
import 'context_platform.dart';
import 'failure_dialog.dart';
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
/// to read the running appointment or the Agenda's appointments from,
/// or one to write into. The Agenda's button and the row in Settings
/// only show where it has.
bool hasCalendarPage(BuildContext context) =>
    ContextScope.of(context).available ||
    AppointmentScope.of(context).available ||
    (CalendarMirrorScope.of(context)?.platform.available ?? false);

/// Everything between the app and this Device's calendar, in two
/// sections: what is read from it, the appointment running when a Note
/// is captured and the appointments the Agenda shows, and what is
/// written into it, the mirror of the Routines and Reminders. Opens from
/// the Agenda's bar and from Settings.
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
          if (ContextScope.of(context).available ||
              AppointmentScope.of(context).available)
            SettingsHeading(t.calendarReadHeading),
          if (ContextScope.of(context).available)
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
              child: ContextSwitch(store: store, kind: ContextKind.calendar),
            ),
          if (AppointmentScope.of(context).available)
            AgendaCalendarSwitch(store: store),
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

/// The switch that shows the calendar's appointments on the Agenda: off
/// until the user turns it on, which explains it, asks for calendar
/// access and opens the list of calendars to pick from, none picked at
/// first. While on it names the calendars picked, and a tap opens the
/// list again. Turning it off keeps the calendars picked for next time.
class AgendaCalendarSwitch extends StatefulWidget {
  final Store store;

  const AgendaCalendarSwitch({super.key, required this.store});

  @override
  State<AgendaCalendarSwitch> createState() => _AgendaCalendarSwitchState();
}

class _AgendaCalendarSwitchState extends State<AgendaCalendarSwitch> {
  /// Whether it is on and allowed; off until looked up.
  var _on = false;
  var _busy = false;

  /// The Device's calendars, to name the ones picked.
  List<ReadCalendar> _calendars = const [];

  @override
  void initState() {
    super.initState();
    _look();
  }

  Future<void> _look() async {
    final source = AppointmentScope.of(context);
    final on = widget.store.agendaCalendarOn && await source.granted();
    final calendars = on ? await source.calendars() : const <ReadCalendar>[];
    if (!mounted) return;
    setState(() {
      _on = on;
      _calendars = calendars;
    });
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

  /// Announces that the calendar cannot be shown on the Agenda; the tap
  /// opens [cause], [fix] and [details].
  void _announceFailed(String cause, String fix, String details) {
    final t = context.t;
    announceFailure(
      context,
      t.agendaCalendarFailed,
      title: t.agendaCalendarFailedTitle,
      cause: cause,
      fix: fix,
      details: details,
    );
  }

  Future<void> _switch(bool on) => _hold(() async {
    final source = AppointmentScope.of(context);
    final t = context.t;
    if (!on) {
      widget.store.agendaCalendarOn = false;
      setState(() => _on = false);
      return;
    }
    if (!await source.granted() &&
        (!mounted || !await explainAndAskAppointments(context, source))) {
      if (mounted) {
        _announceFailed(
          t.agendaCalendarNotAllowedCause,
          t.agendaCalendarAllowFix,
          'agenda calendar: reading not allowed',
        );
      }
      return;
    }
    if (!mounted) return;
    widget.store.agendaCalendarOn = true;
    setState(() => _on = true);
    await _pick();
  });

  /// Opens the list of calendars to pick from.
  Future<void> _pick() async {
    final source = AppointmentScope.of(context);
    final t = context.t;
    final calendars = await source.calendars();
    if (!mounted) return;
    setState(() => _calendars = calendars);
    if (calendars.isEmpty) {
      _announceFailed(
        t.agendaCalendarNoCalendar,
        t.agendaCalendarAddFix,
        'agenda calendar: no calendar on this device',
      );
      return;
    }
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) =>
            AgendaCalendarsScreen(store: widget.store, calendars: calendars),
      ),
    );
    if (!mounted) return;
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final picked = widget.store.agendaCalendars;
    final names = [
      for (final calendar in _calendars)
        if (picked.contains(calendar.id)) calendar.name,
    ];
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
      child: Embossed(
        // The tile's ink shows on the card, not under it.
        child: Material(
          type: MaterialType.transparency,
          child: SwitchBelow(
            switchKey: const ValueKey('agendaCalendar'),
            leading: const Icon(Icons.event_note_outlined),
            title: Text(t.agendaCalendarSwitch),
            subtitle: Text(
              !_on
                  ? t.agendaCalendarDetail
                  : names.isEmpty
                  ? t.agendaCalendarNonePicked
                  : t.agendaCalendarShows(names.join(', ')),
            ),
            value: _on,
            onChanged: _busy ? null : _switch,
            // While on, a tap on the text picks the calendars again; only
            // the switch turns it on or off.
            onTap: _busy || !_on ? null : () => _hold(_pick),
          ),
        ),
      ),
    );
  }
}

/// The calendars whose appointments the Agenda shows: one checkbox per
/// calendar on this Device, each kept as soon as it is ticked.
class AgendaCalendarsScreen extends StatefulWidget {
  final Store store;
  final List<ReadCalendar> calendars;

  const AgendaCalendarsScreen({
    super.key,
    required this.store,
    required this.calendars,
  });

  @override
  State<AgendaCalendarsScreen> createState() => _AgendaCalendarsScreenState();
}

class _AgendaCalendarsScreenState extends State<AgendaCalendarsScreen> {
  late var _picked = widget.store.agendaCalendars;

  void _toggle(String id, bool on) {
    final picked = {..._picked};
    on ? picked.add(id) : picked.remove(id);
    widget.store.agendaCalendars = picked;
    setState(() => _picked = picked);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(title: Text(t.agendaCalendarsTitle)),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.only(bottom: 24)),
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(24, 16, 24, 8),
            child: Text(t.agendaCalendarsHint),
          ),
          for (final calendar in widget.calendars)
            CheckboxListTile(
              key: ValueKey('agendaCalendar-${calendar.id}'),
              controlAffinity: ListTileControlAffinity.leading,
              value: _picked.contains(calendar.id),
              title: Text(calendar.name),
              subtitle:
                  calendar.account.isEmpty || calendar.account == calendar.name
                  ? null
                  : Text(calendar.account),
              secondary: Icon(
                Icons.circle,
                color: calendarColour(calendar.colour),
              ),
              onChanged: (on) => _toggle(calendar.id, on ?? false),
            ),
        ],
      ),
    );
  }
}
