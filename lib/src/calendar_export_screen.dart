import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'browser.dart';
import 'calendar_mirror.dart';
import 'calendar_platform.dart';
import 'l10n.dart';
import 'look.dart';
import 'schedule_text.dart';

/// Opens the calendar export; returns once the user left it.
Future<void> openCalendarExport(BuildContext context, Notes notes) =>
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => CalendarExportScreen(notes: notes),
      ),
    );

/// Every Routine and Reminder to pick for one calendar file, all picked
/// at first; "All" picks or drops them together. Sharing hands the file
/// with their RRULEs and alarms to the app the user chooses. On top, the
/// switch that keeps them all in a calendar of the Device; it lives here
/// until Settings exist.
class CalendarExportScreen extends StatefulWidget {
  final Notes notes;

  const CalendarExportScreen({super.key, required this.notes});

  @override
  State<CalendarExportScreen> createState() => _CalendarExportScreenState();
}

class _CalendarExportScreenState extends State<CalendarExportScreen> {
  late final _routines = Routines(widget.notes.store);
  late final List<(Routine, Note?)> _rows;
  late final Set<String> _picked;

  @override
  void initState() {
    super.initState();
    _rows = [for (final r in _routines.all()) (r, _routines.note(r))]
      ..sort((a, b) {
        final byText = (a.$2?.text ?? '').compareTo(b.$2?.text ?? '');
        return byText != 0 ? byText : a.$1.id.compareTo(b.$1.id);
      });
    _picked = {for (final (routine, _) in _rows) routine.id};
  }

  void _pick(String id, bool picked) => setState(() {
    if (picked) {
      _picked.add(id);
    } else {
      _picked.remove(id);
    }
  });

  void _pickAll(bool picked) => setState(() {
    _picked.clear();
    if (picked) _picked.addAll([for (final (routine, _) in _rows) routine.id]);
  });

  Future<void> _share() => shareCalendarFile(context, widget.notes.store, [
    for (final (routine, _) in _rows)
      if (_picked.contains(routine.id))
        ...eventsOf(context, _routines, routine),
  ]);

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final mirror = CalendarMirrorScope.of(context);
    return Scaffold(
      appBar: AppBar(
        title: Text(t.calendarExportTitle),
        actions: [
          IconButton(
            tooltip: t.shareCalendarFile,
            onPressed: _picked.isEmpty ? null : _share,
            icon: const Icon(Icons.share),
          ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.only(bottom: 16),
        children: [
          if (mirror != null && mirror.platform.available)
            MirrorSwitch(mirror: mirror),
          if (_rows.isEmpty)
            BrowserEmpty(text: t.calendarExportEmpty)
          else ...[
            CheckboxListTile(
              value: _picked.length == _rows.length
                  ? true
                  : _picked.isEmpty
                  ? false
                  : null,
              tristate: true,
              title: Text(t.calendarExportAll),
              onChanged: (_) => _pickAll(_picked.length != _rows.length),
            ),
            const Divider(),
            for (final (routine, note) in _rows)
              CheckboxListTile(
                value: _picked.contains(routine.id),
                title: Text(
                  note == null ? t.noteGone : noteText(t, note),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
                subtitle: Text(scheduleText(t, locale, routine.schedule)),
                secondary: Icon(
                  routine.isReminder
                      ? Icons.notifications_outlined
                      : Icons.repeat,
                ),
                onChanged: (picked) => _pick(routine.id, picked ?? false),
              ),
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
          child: ListTile(
            leading: const Icon(Icons.event_repeat),
            title: Text(t.calendarMirrorSwitch),
            subtitle: Text(
              calendar == null
                  ? t.calendarMirrorDetail
                  : t.calendarMirrorIn(calendar.name),
            ),
            trailing: Switch(
              value: calendar != null,
              onChanged: _busy ? null : (on) => on ? _choose() : _turnOff(),
            ),
            onTap: _busy ? null : _choose,
          ),
        ),
      ),
    );
  }
}
