import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'about_screen.dart';
import 'alarm_sync.dart';
import 'announcements.dart';
import 'app_lock.dart';
import 'calendar_mirror.dart';
import 'capture_context_screen.dart';
import 'context_platform.dart';
import 'l10n.dart';
import 'lock_settings.dart';
import 'look.dart';
import 'named_places_screen.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';
import 'setup_screen.dart';
import 'speech_model_screen.dart';
import 'speech_platform.dart';
import 'system_bars.dart';

/// This Device's settings (ADR 0008), reached only through the Profile
/// page: the theme; the alarm setup and the default Re-nag interval; the
/// Nudges' repeat and Quiet Hours; the Named Places, the Capture Context
/// switches and the speech model; the calendar mirror; the App Lock and
/// the Area Locks; whether Streaks show; and About. Each is a local
/// setting, never synced, and applies at once; the alarms are armed
/// again after each change.
class SettingsScreen extends StatefulWidget {
  final Store store;
  final AlarmSync alarms;

  /// Called after the theme was changed, to show it at once.
  final VoidCallback? onThemeChanged;

  const SettingsScreen({
    super.key,
    required this.store,
    required this.alarms,
    this.onThemeChanged,
  });

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  late ThemeChoice _theme = widget.store.themeChoice;
  late Duration _renag = widget.store.renagInterval;
  late bool _streaksOn = widget.store.streaksOn;

  void _setTheme(ThemeChoice choice) {
    widget.store.themeChoice = choice;
    setState(() => _theme = choice);
    widget.onThemeChanged?.call();
  }

  void _setRenag(Duration interval) {
    widget.store.renagInterval = interval;
    setState(() => _renag = interval);
    unawaited(widget.alarms.refresh());
  }

  void _setStreaks(bool on) {
    widget.store.streaksOn = on;
    setState(() => _streaksOn = on);
  }

  Future<void> _push(Widget screen) =>
      Navigator.of(context)
          .push(MaterialPageRoute<void>(builder: (_) => screen));

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final speech = SpeechScope.of(context);
    final mirror = CalendarMirrorScope.of(context);
    final renag = _renag.inMinutes;
    // A value set elsewhere stays pickable.
    final renagChoices = {...RoutineEditorScreen.renagChoices, renag}.toList()
      ..sort();
    return Scaffold(
      appBar: AppBar(title: Text(t.settingsTitle)),
      // A short page, built whole.
      body: SingleChildScrollView(
        padding: clearOfSystemBars(context, const EdgeInsets.only(bottom: 24)),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 8, 24, 0),
              child: Text(t.settingsIntro),
            ),
            SettingsHeading(t.settingsLook),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: SegmentedButton<ThemeChoice>(
                key: const ValueKey('theme'),
                segments: [
                  ButtonSegment(
                    value: ThemeChoice.system,
                    label: Text(t.themeSystem),
                    icon: const Icon(Icons.brightness_auto_outlined),
                  ),
                  ButtonSegment(
                    value: ThemeChoice.light,
                    label: Text(t.themeLight),
                    icon: const Icon(Icons.light_mode_outlined),
                  ),
                  ButtonSegment(
                    value: ThemeChoice.dark,
                    label: Text(t.themeDark),
                    icon: const Icon(Icons.dark_mode_outlined),
                  ),
                ],
                selected: {_theme},
                showSelectedIcon: false,
                onSelectionChanged: (picked) => _setTheme(picked.single),
              ),
            ),
            SettingsHeading(t.settingsAlarms),
            SettingsSection(
              children: [
                if (widget.alarms.platform.available)
                  ListTile(
                    key: const ValueKey('alarmSetup'),
                    leading: const Icon(Icons.alarm_on),
                    title: Text(t.setupTitle),
                    subtitle: Text(t.settingsAlarmSetupDetail),
                    onTap: () => _push(
                      Builder(
                        builder: (context) => SetupScreen(
                          alarms: widget.alarms,
                          onDone: () => Navigator.of(context).pop(),
                        ),
                      ),
                    ),
                  ),
                ControlBelow(
                  leading: const Icon(Icons.repeat),
                  title: Text(t.renagDevice),
                  subtitle: Text(t.settingsRenagDetail),
                  control: DropdownButton<int>(
                    key: const ValueKey('renagDevice'),
                    value: renag,
                    onChanged: (minutes) {
                      if (minutes == null) return;
                      _setRenag(Duration(minutes: minutes));
                    },
                    items: [
                      for (final m in renagChoices)
                        DropdownMenuItem(
                          value: m,
                          child: Text(t.renagMinutes(m)),
                        ),
                    ],
                  ),
                ),
              ],
            ),
            SettingsHeading(t.settingsNudges),
            _NudgeSettings(store: widget.store, alarms: widget.alarms),
            SettingsHeading(t.settingsCapture),
            SettingsSection(
              children: [
                ListTile(
                  leading: const Icon(Icons.place_outlined),
                  title: Text(t.namedPlacesTitle),
                  subtitle: Text(t.settingsNamedPlacesDetail),
                  onTap: () => _push(
                    NamedPlacesScreen(namedPlaces: NamedPlaces(widget.store)),
                  ),
                ),
                if (ContextScope.of(context).available)
                  ListTile(
                    leading: const Icon(Icons.manage_history),
                    title: Text(t.captureContextTitle),
                    subtitle: Text(t.settingsCaptureContextDetail),
                    onTap: () =>
                        _push(CaptureContextScreen(store: widget.store)),
                  ),
                if (speech != null && speech.model.platform.available)
                  ListTile(
                    leading: const Icon(Icons.record_voice_over_outlined),
                    title: Text(t.speechModelTitle),
                    subtitle: Text(t.settingsSpeechModelDetail),
                    onTap: () => _push(SpeechModelScreen(model: speech.model)),
                  ),
              ],
            ),
            if (mirror != null && mirror.platform.available) ...[
              SettingsHeading(t.settingsCalendar),
              MirrorSwitch(mirror: mirror),
            ],
            if (LockScope.of(context) case final lock?) ...[
              SettingsHeading(t.settingsPrivacy),
              LockSettingsSection(
                lock: lock,
                alarms: widget.alarms,
                places: Places(widget.store),
              ),
            ],
            SettingsHeading(t.settingsHistory),
            SettingsSection(
              children: [
                SwitchBelow(
                  switchKey: const ValueKey('streaks'),
                  leading: const Icon(Icons.local_fire_department_outlined),
                  value: _streaksOn,
                  onChanged: _setStreaks,
                  title: Text(t.showStreaks),
                  subtitle: Text(t.showStreaksDetail),
                ),
              ],
            ),
            const SizedBox(height: 16),
            SettingsSection(
              children: [
                ListTile(
                  leading: const Icon(Icons.info_outline),
                  title: Text(t.aboutTitle),
                  onTap: () => _push(const AboutScreen()),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

/// A section title of Settings and the Profile page.
class SettingsHeading extends StatelessWidget {
  final String text;

  const SettingsHeading(this.text, {super.key});

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(24, 16, 24, 4),
    child: Text(text, style: Theme.of(context).textTheme.titleMedium),
  );
}

/// The tiles of one section on an embossed card.
class SettingsSection extends StatelessWidget {
  final List<Widget> children;

  const SettingsSection({super.key, required this.children});

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 4, 16, 4),
    child: Embossed(
      child: ClipRRect(
        borderRadius: BorderRadius.circular(16),
        // The tiles' ink shows on the card, not under it.
        child: Material(
          type: MaterialType.transparency,
          child: Column(children: children),
        ),
      ),
    ),
  );
}

/// This Device's Nudge settings: how long an ignored Nudge waits before
/// it comes again, and the Quiet Hours.
class _NudgeSettings extends StatefulWidget {
  /// The repeat intervals offered, in minutes.
  static const repeatChoices = [10, 30, 60];

  final Store store;
  final AlarmSync alarms;

  const _NudgeSettings({required this.store, required this.alarms});

  @override
  State<_NudgeSettings> createState() => _NudgeSettingsState();
}

class _NudgeSettingsState extends State<_NudgeSettings> {
  late Duration _repeat = widget.store.nudgeRepeat;
  late QuietHours _quiet = widget.store.quietHours;
  late bool _quietOn = widget.store.quietHoursOn;

  void _changed() {
    setState(() {
      _repeat = widget.store.nudgeRepeat;
      _quiet = widget.store.quietHours;
      _quietOn = widget.store.quietHoursOn;
    });
    unawaited(widget.alarms.refresh());
  }

  Future<void> _pick({required bool start}) async {
    final shown = start ? _quiet.start : _quiet.end;
    final picked = await showTimePicker(
      context: context,
      initialTime: TimeOfDay(hour: shown.hour, minute: shown.minute),
    );
    if (picked == null || !mounted) return;
    final time = ClockTime(picked.hour, picked.minute);
    widget.store.quietHours = start
        ? QuietHours(time, _quiet.end)
        : QuietHours(_quiet.start, time);
    _changed();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    String time(ClockTime c) => clockText(locale, c);
    final repeat = _repeat.inMinutes;
    final choices = {..._NudgeSettings.repeatChoices, repeat}.toList()..sort();
    return SettingsSection(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
          child: Text(t.nudgeSettingsExplain),
        ),
        ControlBelow(
          leading: const Icon(Icons.notifications_paused_outlined),
          title: Text(t.nudgeRepeat),
          control: DropdownButton<int>(
            key: const ValueKey('nudgeRepeat'),
            value: repeat,
            onChanged: (minutes) {
              if (minutes == null) return;
              widget.store.nudgeRepeat = Duration(minutes: minutes);
              _changed();
            },
            items: [
              for (final m in choices)
                DropdownMenuItem(value: m, child: Text(t.renagMinutes(m))),
            ],
          ),
        ),
        SwitchBelow(
          switchKey: const ValueKey('quietHours'),
          leading: const Icon(Icons.bedtime_outlined),
          title: Text(t.quietHours),
          value: _quietOn,
          onChanged: (on) {
            widget.store.quietHoursOn = on;
            _changed();
          },
        ),
        if (_quietOn) ...[
          ListTile(
            key: const ValueKey('quietFrom'),
            leading: const SizedBox.shrink(),
            title: Text(t.quietFrom),
            trailing: Text(time(_quiet.start)),
            onTap: () => _pick(start: true),
          ),
          ListTile(
            key: const ValueKey('quietUntil'),
            leading: const SizedBox.shrink(),
            title: Text(t.quietUntil),
            trailing: Text(time(_quiet.end)),
            onTap: () => _pick(start: false),
          ),
        ],
      ],
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
