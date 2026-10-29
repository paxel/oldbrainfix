import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'about_screen.dart';
import 'alarm_sync.dart';
import 'app_lock.dart';
import 'calendar_screen.dart';
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

/// How this Device shows the known units of Tallies: as its region
/// does, or a system chosen.
enum _UnitChoice { region, metric, imperial }

/// This Device's settings (ADR 0008), reached only through the Profile
/// page: the theme; the units of Tallies; the alarm setup, the default Re-nag interval and the
/// alarm sound; the
/// Nudges' repeat and Quiet Hours; the Named Places, the Capture Context
/// switches and the speech model; the way to the Calendar page; the App Lock and
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
  late String? _sound = widget.store.alarmSound;
  late _UnitChoice _units = switch (widget.store.chosenUnitSystem) {
    null => _UnitChoice.region,
    UnitSystem.metric => _UnitChoice.metric,
    UnitSystem.imperial => _UnitChoice.imperial,
  };

  /// The system of the phone's region, read once.
  late final UnitSystem _regionUnits = TallyUnits.systemForRegion(
    WidgetsBinding.instance.platformDispatcher.locale.countryCode,
  );

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

  /// Opens the system's sound picker; leaving it changes nothing.
  Future<void> _pickSound() async {
    final picked = await widget.alarms.platform.pickSound(
      _sound,
      context.t.alarmSound,
    );
    if (picked == null || !mounted) return;
    widget.store.alarmSound = picked.sound;
    setState(() => _sound = widget.store.alarmSound);
    unawaited(widget.alarms.refresh());
  }

  void _setUnits(_UnitChoice choice) {
    widget.store.chosenUnitSystem = switch (choice) {
      _UnitChoice.region => null,
      _UnitChoice.metric => UnitSystem.metric,
      _UnitChoice.imperial => UnitSystem.imperial,
    };
    setState(() => _units = choice);
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
              child: RadioColumn<ThemeChoice>(
                key: const ValueKey('theme'),
                value: _theme,
                options: [
                  RadioOption(
                    value: ThemeChoice.system,
                    title: Text(t.themeSystem),
                    secondary: const Icon(Icons.brightness_auto_outlined),
                  ),
                  RadioOption(
                    value: ThemeChoice.light,
                    title: Text(t.themeLight),
                    secondary: const Icon(Icons.light_mode_outlined),
                  ),
                  RadioOption(
                    value: ThemeChoice.dark,
                    title: Text(t.themeDark),
                    secondary: const Icon(Icons.dark_mode_outlined),
                  ),
                ],
                onChanged: _setTheme,
              ),
            ),
            SettingsHeading(t.settingsUnits),
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 0, 24, 0),
              child: Text(t.settingsUnitsDetail),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: RadioColumn<_UnitChoice>(
                key: const ValueKey('unitSystem'),
                value: _units,
                options: [
                  RadioOption(
                    key: const ValueKey('unitSystem-region'),
                    value: _UnitChoice.region,
                    title: Text(
                      t.unitSystemRegion(switch (_regionUnits) {
                        UnitSystem.metric => t.unitSystemMetricName,
                        UnitSystem.imperial => t.unitSystemImperialName,
                      }),
                    ),
                  ),
                  RadioOption(
                    key: const ValueKey('unitSystem-metric'),
                    value: _UnitChoice.metric,
                    title: Text(t.unitSystemMetric),
                  ),
                  RadioOption(
                    key: const ValueKey('unitSystem-imperial'),
                    value: _UnitChoice.imperial,
                    title: Text(t.unitSystemImperial),
                  ),
                ],
                onChanged: _setUnits,
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
                  control: RadioColumn<int>(
                    key: const ValueKey('renagDevice'),
                    value: renag,
                    onChanged: (minutes) =>
                        _setRenag(Duration(minutes: minutes)),
                    options: [
                      for (final m in renagChoices)
                        RadioOption(value: m, title: Text(t.renagMinutes(m))),
                    ],
                  ),
                ),
                if (widget.alarms.platform.available)
                  ListTile(
                    key: const ValueKey('alarmSound'),
                    leading: const Icon(Icons.music_note_outlined),
                    title: Text(t.alarmSound),
                    subtitle: Text(
                      _sound == null ? t.alarmSoundOwn : t.alarmSoundChosen,
                    ),
                    onTap: _pickSound,
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
            if (hasCalendarPage(context)) ...[
              SettingsHeading(t.settingsCalendar),
              SettingsSection(
                children: [
                  ListTile(
                    key: const ValueKey('calendarPage'),
                    leading: const Icon(Icons.event_outlined),
                    title: Text(t.calendarPageTitle),
                    subtitle: Text(t.settingsCalendarDetail),
                    onTap: () => _push(CalendarScreen(store: widget.store)),
                  ),
                ],
              ),
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
          control: RadioColumn<int>(
            key: const ValueKey('nudgeRepeat'),
            value: repeat,
            onChanged: (minutes) {
              widget.store.nudgeRepeat = Duration(minutes: minutes);
              _changed();
            },
            options: [
              for (final m in choices)
                RadioOption(value: m, title: Text(t.renagMinutes(m))),
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
