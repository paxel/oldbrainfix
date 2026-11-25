import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'alarm_sync.dart';
import 'announcements.dart';
import 'app_lock.dart';
import 'areas_screen.dart';
import 'big_screen.dart';
import 'browser.dart';
import 'calendar_screen.dart';
import 'capture_context_screen.dart';
import 'context_platform.dart';
import 'features_choice.dart';
import 'feedback_screen.dart';
import 'file_platform.dart';
import 'imports_screen.dart';
import 'l10n.dart';
import 'lock_settings.dart';
import 'look.dart';
import 'named_places_screen.dart';
import 'page_bar.dart';
import 'print_platform.dart';
import 'recording_pause.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';
import 'settings_search.dart';
import 'setup_screen.dart';
import 'speech_model_screen.dart';
import 'speech_platform.dart';
import 'spotlight.dart';
import 'system_bars.dart';
import 'tap_mark.dart';
import 'voice_platform.dart';

export 'settings_search.dart' show SettingsHeading, SettingsSection;

/// How this Device shows the known units of Tallies: as its region
/// does, or a system chosen.
enum _UnitChoice { region, metric, imperial }

/// This Device's settings (ADR 0008), reached only through the Profile
/// page: the theme, the text size (#326, obf-1019) and "Fat fingers"; the units of Tallies; the alarm setup, the default Re-nag interval and the
/// alarm sound; the
/// Nudges' repeat and Quiet Hours; the Named Places, the Capture Context
/// switches, the speech model, End a recording after a pause (obf-1053) and Print to Old Brain Fix (#293); what the quick note shows; the way to the Calendar page; the App Lock and
/// the Area Locks; whether Streaks show; while searching for the Imports,
/// a line that leads to the Areas list, whose ⋮ holds them (obf-1096); in
/// Advanced, Ask (#333); Show tips (obf-1060)
/// and Show tips again (obf-1043); the person's own Code languages (obf-1037); and Send
/// feedback (#214). About is on the Profile page (#325). Each is a local setting, never synced,
/// and applies at once; the alarms are armed again after each change.
class SettingsScreen extends StatefulWidget {
  final Store store;
  final AlarmSync alarms;

  /// Called after the theme, the text size or "Fat fingers" was changed,
  /// to show it at once.
  final VoidCallback? onThemeChanged;

  /// The notes browser's way back in, for the Areas list the line about
  /// the Imports opens (obf-1096).
  final BrowserTrail? trail;

  /// The section an app link opens the page at (#261), by its link's
  /// name ([AppLink.settingsSections]); the top when null or when this
  /// Device has no such section.
  final String? section;

  const SettingsScreen({
    super.key,
    required this.store,
    required this.alarms,
    this.onThemeChanged,
    this.trail,
    this.section,
  });

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  /// The headings by their section's link name, for an app link (#261).
  final _headings = <String, GlobalKey>{};

  GlobalKey _heading(String section) =>
      _headings.putIfAbsent(section, () => GlobalKey(debugLabel: section));

  /// The search on top (#282), its query as [SettingsSearch] takes it,
  /// and the query that showed no row, which the page then says.
  final _search = TextEditingController();
  var _query = '';
  String? _nothingFor;

  void _searched(String typed) {
    setState(() => _query = SettingsSearch.queryOf(typed));
    // The sections decide which of their rows show while they build.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final nothing = _query.isNotEmpty && !SettingsSearch.showsAnyRow(context)
          ? _query
          : null;
      if (nothing != _nothingFor) setState(() => _nothingFor = nothing);
    });
  }

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  /// Show tips (obf-1060), which "No more tips" on a tip over this page
  /// switches off too.
  late final _tips = TipsSwitch.of(widget.store);

  /// Show button taps (obf-1143).
  late final _tapMarks = TapMarkSwitch.of(widget.store);

  /// Show tips again (obf-1043): every screen of this Device shows its
  /// Spotlights again on its next visit, "Show tips" on, and the top
  /// says so.
  void _showTips() {
    resetSpotlights(widget.store);
    AnnouncementsScope.of(context)?.announce(context.t.settingsShowTipsDone);
  }

  @override
  void initState() {
    super.initState();
    final section = widget.section;
    if (section == null) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final heading = _headings[section]?.currentContext;
      if (heading != null && heading.mounted) {
        unawaited(Scrollable.ensureVisible(heading));
      }
    });
  }

  late ThemeChoice _theme = widget.store.themeChoice;
  late bool _fatFingers = widget.store.fatFingers;
  late TextSize _textSize = widget.store.textSize;
  late Duration _renag = widget.store.renagInterval;
  late bool _streaksOn = widget.store.streaksOn;
  late bool _hidesChecked = widget.store.agendaHidesChecked;
  late bool _trashesDone = widget.store.trashesDoneReminder;
  late bool _advanced = widget.store.advanced;
  late bool _twoRows = widget.store.twoRowEditorOn;
  late bool _askOn = widget.store.askOn;
  late int _pauseSeconds = recordingPause(widget.store)?.inSeconds ?? 0;
  late String? _sound = widget.store.alarmSound;
  late ClockTime _allDayFrom = widget.store.allDayFrom;
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

  void _setFatFingers(bool on) {
    widget.store.fatFingers = on;
    setState(() => _fatFingers = on);
    widget.onThemeChanged?.call();
    // The widget's text grows with it.
    unawaited(widget.alarms.refresh());
  }

  /// The text's own step (#326, obf-1019), shown at once; the Agenda's
  /// widget keeps "Fat fingers" alone.
  void _setTextSize(TextSize size) {
    widget.store.textSize = size;
    setState(() => _textSize = size);
    widget.onThemeChanged?.call();
  }

  void _setRenag(Duration interval) {
    widget.store.renagInterval = interval;
    setState(() => _renag = interval);
    unawaited(widget.alarms.refresh());
  }

  /// Picks from when all-day items ring (#201); backing out changes
  /// nothing.
  Future<void> _pickAllDayFrom() async {
    final picked = await showTimePicker(
      context: context,
      initialTime: TimeOfDay(
        hour: _allDayFrom.hour,
        minute: _allDayFrom.minute,
      ),
    );
    if (picked == null || !mounted) return;
    widget.store.allDayFrom = ClockTime(picked.hour, picked.minute);
    setState(() => _allDayFrom = widget.store.allDayFrom);
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

  void _setHidesChecked(bool on) {
    widget.store.agendaHidesChecked = on;
    setState(() => _hidesChecked = on);
  }

  /// The person's own setting, on all of their own Devices (#211).
  void _setTrashesDone(bool on) {
    widget.store.trashesDoneReminder = on;
    setState(() => _trashesDone = on);
  }

  /// Simple or Advanced (#197), the person's own, on all of their own
  /// Devices.
  void _setAdvanced(bool on) {
    widget.store.advanced = on;
    setState(() => _advanced = on);
  }

  void _setStreaks(bool on) {
    widget.store.streaksOn = on;
    setState(() => _streaksOn = on);
  }

  /// Ask, experimental (#333): the Agenda reads it again on the way
  /// back.
  void _setAsk(bool on) {
    widget.store.askOn = on;
    setState(() => _askOn = on);
  }

  /// End a recording after a pause (obf-1053): its seconds, 0 for Off;
  /// the next recording follows it.
  void _setPause(int seconds) {
    setRecordingPause(widget.store, seconds == 0 ? null : seconds);
    setState(() => _pauseSeconds = seconds);
  }

  void _setTwoRows(bool on) {
    widget.store.twoRowEditorOn = on;
    setState(() => _twoRows = on);
  }

  /// The Areas list on top, whose ⋮ holds Import… (obf-1096).
  void _openAreas() {
    final store = widget.store;
    unawaited(
      Navigator.of(context).push(
        BrowserRoute<void>(
          place: const ShownPlace(PlaceKind.areas),
          builder: (_) => AreasScreen(
            notes: Notes(store),
            places: Places(store),
            trail: widget.trail ?? BrowserTrail(),
          ),
        ),
      ),
    );
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
    final regionUnits = t.unitSystemRegion(switch (_regionUnits) {
      UnitSystem.metric => t.unitSystemMetricName,
      UnitSystem.imperial => t.unitSystemImperialName,
    });
    final soundDetail = _sound == null ? t.alarmSoundOwn : t.alarmSoundChosen;
    final typed = _search.text.trim();
    return Scaffold(
      appBar: pageBar(
        context,
        title: t.settingsTitle,
        background: searchHeaderColour(context),
      ),
      body: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // At the top, above the page it filters (#282), on its band, so
          // the page scrolls under a line, not an edge (obf-1088).
          SearchBand(
            child: Spotlight(
              id: 'settings-search',
              child: BrowserSearchField(
                key: const ValueKey('settingsSearch'),
                controller: _search,
                hint: t.settingsSearchHint,
                onChanged: _searched,
              ),
            ),
          ),
          Expanded(
            // A short page, built whole.
            child: SingleChildScrollView(
              padding: clearOfSystemBars(
                context,
                const EdgeInsets.only(bottom: 24),
              ),
              child: SettingsSearch(
                query: _query,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (_query.isEmpty)
                      Padding(
                        padding: const EdgeInsets.fromLTRB(24, 8, 24, 0),
                        child: Text(t.settingsIntro),
                      ),
                    if (_query.isNotEmpty && _nothingFor == _query)
                      Padding(
                        padding: const EdgeInsets.all(24),
                        child: Text(
                          t.settingsSearchNothing(typed),
                          key: const ValueKey('settingsSearchNothing'),
                        ),
                      ),
                    SettingsGroup(
                      heading: t.settingsLook,
                      headingKey: _heading('look'),
                      children: [
                        SettingsSection(
                          children: [
                            SettingsRow(
                              words: [t.themeSystem, t.themeLight, t.themeDark],
                              child: Padding(
                                padding: const EdgeInsets.fromLTRB(
                                  16,
                                  8,
                                  16,
                                  0,
                                ),
                                // At the card's start, not its middle.
                                child: Align(
                                  alignment: AlignmentDirectional.centerStart,
                                  child: ChoiceChips<ThemeChoice>(
                                    key: const ValueKey('theme'),
                                    value: _theme,
                                    options: [
                                      RadioOption(
                                        value: ThemeChoice.system,
                                        title: Text(t.themeSystem),
                                        secondary: const Icon(
                                          Icons.brightness_auto_outlined,
                                        ),
                                      ),
                                      RadioOption(
                                        value: ThemeChoice.light,
                                        title: Text(t.themeLight),
                                        secondary: const Icon(
                                          Icons.light_mode_outlined,
                                        ),
                                      ),
                                      RadioOption(
                                        value: ThemeChoice.dark,
                                        title: Text(t.themeDark),
                                        secondary: const Icon(
                                          Icons.dark_mode_outlined,
                                        ),
                                      ),
                                    ],
                                    onChanged: _setTheme,
                                  ),
                                ),
                              ),
                            ),
                            SettingsRow(
                              words: [
                                t.textSize,
                                t.textSizeDetail,
                                t.textSizeNormal,
                                t.textSizeLarger,
                                t.textSizeLargest,
                              ],
                              child: ControlBelow(
                                leading: const Icon(Icons.format_size),
                                title: Text(t.textSize),
                                subtitle: Text(t.textSizeDetail),
                                control: ChoiceChips<TextSize>(
                                  key: const ValueKey('textSize'),
                                  value: _textSize,
                                  options: [
                                    RadioOption(
                                      key: const ValueKey('textSize-normal'),
                                      value: TextSize.normal,
                                      title: Text(t.textSizeNormal),
                                    ),
                                    RadioOption(
                                      key: const ValueKey('textSize-larger'),
                                      value: TextSize.larger,
                                      title: Text(t.textSizeLarger),
                                    ),
                                    RadioOption(
                                      key: const ValueKey('textSize-largest'),
                                      value: TextSize.largest,
                                      title: Text(t.textSizeLargest),
                                    ),
                                  ],
                                  onChanged: _setTextSize,
                                ),
                              ),
                            ),
                            SettingsRow(
                              words: [t.fatFingers, t.fatFingersDetail],
                              child: SwitchBelow(
                                switchKey: const ValueKey('fatFingers'),
                                leading: const Icon(Icons.touch_app_outlined),
                                value: _fatFingers,
                                onChanged: _setFatFingers,
                                title: Text(t.fatFingers),
                                subtitle: Text(t.fatFingersDetail),
                              ),
                            ),
                            SettingsRow(
                              words: [t.tapMarks, t.tapMarksDetail],
                              child: ValueListenableBuilder<bool>(
                                valueListenable: _tapMarks,
                                builder: (context, on, _) => SwitchBelow(
                                  switchKey: const ValueKey('tapMarks'),
                                  leading: const Icon(Icons.priority_high),
                                  value: on,
                                  onChanged: _tapMarks.set,
                                  title: Text(t.tapMarks),
                                  subtitle: Text(t.tapMarksDetail),
                                ),
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                    SettingsGroup(
                      heading: t.settingsUnits,
                      headingKey: _heading('units'),
                      children: [
                        SettingsSection(
                          children: [
                            SettingsRow(
                              words: [
                                t.settingsUnitsDetail,
                                regionUnits,
                                t.unitSystemMetric,
                                t.unitSystemImperial,
                              ],
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.stretch,
                                children: [
                                  Padding(
                                    padding: const EdgeInsets.fromLTRB(
                                      16,
                                      12,
                                      16,
                                      0,
                                    ),
                                    child: Text(t.settingsUnitsDetail),
                                  ),
                                  Padding(
                                    padding: const EdgeInsets.symmetric(
                                      horizontal: 16,
                                    ),
                                    child: RadioColumn<_UnitChoice>(
                                      key: const ValueKey('unitSystem'),
                                      value: _units,
                                      options: [
                                        RadioOption(
                                          key: const ValueKey(
                                            'unitSystem-region',
                                          ),
                                          value: _UnitChoice.region,
                                          title: Text(regionUnits),
                                        ),
                                        RadioOption(
                                          key: const ValueKey(
                                            'unitSystem-metric',
                                          ),
                                          value: _UnitChoice.metric,
                                          title: Text(t.unitSystemMetric),
                                        ),
                                        RadioOption(
                                          key: const ValueKey(
                                            'unitSystem-imperial',
                                          ),
                                          value: _UnitChoice.imperial,
                                          title: Text(t.unitSystemImperial),
                                        ),
                                      ],
                                      onChanged: _setUnits,
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                    SettingsGroup(
                      heading: t.settingsAgenda,
                      headingKey: _heading('agenda'),
                      children: [
                        SettingsSection(
                          children: [
                            SettingsRow(
                              words: [
                                t.hideCheckedOnAgenda,
                                t.hideCheckedOnAgendaDetail,
                              ],
                              child: SwitchBelow(
                                switchKey: const ValueKey('agendaHideChecked'),
                                leading: const Icon(Icons.playlist_remove),
                                value: _hidesChecked,
                                onChanged: _setHidesChecked,
                                title: Text(t.hideCheckedOnAgenda),
                                subtitle: Text(t.hideCheckedOnAgendaDetail),
                              ),
                            ),
                            SettingsRow(
                              words: [
                                t.trashDoneReminder,
                                t.trashDoneReminderDetail,
                              ],
                              child: SwitchBelow(
                                switchKey: const ValueKey('trashDoneReminder'),
                                leading: const Icon(Icons.auto_delete_outlined),
                                value: _trashesDone,
                                onChanged: _setTrashesDone,
                                title: Text(t.trashDoneReminder),
                                subtitle: Text(t.trashDoneReminderDetail),
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                    SettingsGroup(
                      heading: t.settingsAlarms,
                      headingKey: _heading('alarms'),
                      children: [
                        SettingsSection(
                          children: [
                            if (widget.alarms.platform.available)
                              SettingsRow(
                                words: [
                                  t.setupTitle,
                                  t.settingsAlarmSetupDetail,
                                ],
                                child: ListTile(
                                  key: const ValueKey('alarmSetup'),
                                  leading: const Icon(Icons.alarm_on),
                                  title: Text(t.setupTitle),
                                  subtitle: Text(t.settingsAlarmSetupDetail),
                                  onTap: () => _push(
                                    Builder(
                                      builder: (context) => SetupScreen(
                                        alarms: widget.alarms,
                                        onDone: () =>
                                            Navigator.of(context).pop(),
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                            SettingsRow(
                              words: [t.renagDevice, t.settingsRenagDetail],
                              child: ControlBelow(
                                leading: const Icon(Icons.repeat),
                                title: Text(t.renagDevice),
                                subtitle: Text(t.settingsRenagDetail),
                                control: ChoiceChips<int>(
                                  key: const ValueKey('renagDevice'),
                                  value: renag,
                                  onChanged: (minutes) =>
                                      _setRenag(Duration(minutes: minutes)),
                                  options: [
                                    for (final m in renagChoices)
                                      RadioOption(
                                        value: m,
                                        title: Text(t.renagMinutes(m)),
                                      ),
                                  ],
                                ),
                              ),
                            ),
                            SettingsRow(
                              words: [t.allDayFrom, t.allDayFromDetail],
                              child: ListTile(
                                key: const ValueKey('allDayFrom'),
                                leading: const Icon(Icons.wb_sunny_outlined),
                                title: Text(t.allDayFrom),
                                subtitle: Text(t.allDayFromDetail),
                                trailing: Text(
                                  clockText(
                                    Localizations.localeOf(context).toString(),
                                    _allDayFrom,
                                  ),
                                ),
                                onTap: _pickAllDayFrom,
                              ),
                            ),
                            if (widget.alarms.platform.available &&
                                widget.alarms.platform.choosesSound)
                              SettingsRow(
                                words: [t.alarmSound, soundDetail],
                                child: ListTile(
                                  key: const ValueKey('alarmSound'),
                                  leading: const Icon(
                                    Icons.music_note_outlined,
                                  ),
                                  title: Text(t.alarmSound),
                                  subtitle: Text(soundDetail),
                                  onTap: _pickSound,
                                ),
                              ),
                          ],
                        ),
                      ],
                    ),
                    _NudgeSettings(
                      store: widget.store,
                      alarms: widget.alarms,
                      headingKey: _heading('nudges'),
                    ),
                    SettingsGroup(
                      heading: t.settingsCapture,
                      headingKey: _heading('capture'),
                      children: [
                        SettingsSection(
                          children: [
                            SettingsRow(
                              words: [
                                t.namedPlacesTitle,
                                t.settingsNamedPlacesDetail,
                              ],
                              child: ListTile(
                                leading: const Icon(Icons.place_outlined),
                                title: Text(t.namedPlacesTitle),
                                subtitle: Text(t.settingsNamedPlacesDetail),
                                onTap: () => _push(
                                  NamedPlacesScreen(
                                    namedPlaces: NamedPlaces(widget.store),
                                  ),
                                ),
                              ),
                            ),
                            if (ContextScope.of(context).available)
                              SettingsRow(
                                words: [
                                  t.captureContextTitle,
                                  t.settingsCaptureContextDetail,
                                ],
                                child: ListTile(
                                  leading: const Icon(Icons.manage_history),
                                  title: Text(t.captureContextTitle),
                                  subtitle: Text(
                                    t.settingsCaptureContextDetail,
                                  ),
                                  onTap: () => _push(
                                    CaptureContextScreen(store: widget.store),
                                  ),
                                ),
                              ),
                            if (speech != null &&
                                speech.model.platform.available)
                              SettingsRow(
                                words: [
                                  t.speechModelTitle,
                                  t.settingsSpeechModelDetail,
                                ],
                                child: ListTile(
                                  leading: const Icon(
                                    Icons.record_voice_over_outlined,
                                  ),
                                  title: Text(t.speechModelTitle),
                                  subtitle: Text(t.settingsSpeechModelDetail),
                                  onTap: () => _push(
                                    SpeechModelScreen(model: speech.model),
                                  ),
                                ),
                              ),
                            if (VoiceScope.of(context).available)
                              SettingsRow(
                                words: [
                                  t.recordingPause,
                                  t.recordingPauseDetail,
                                ],
                                child: ControlBelow(
                                  leading: const Icon(Icons.mic_off_outlined),
                                  title: Text(t.recordingPause),
                                  subtitle: Text(t.recordingPauseDetail),
                                  control: ChoiceChips<int>(
                                    key: const ValueKey('recordingPause'),
                                    value: _pauseSeconds,
                                    onChanged: _setPause,
                                    options: [
                                      RadioOption(
                                        value: 0,
                                        title: Text(t.recordingPauseOff),
                                      ),
                                      for (final s in recordingPauseChoices)
                                        RadioOption(
                                          value: s,
                                          title: Text(
                                            t.recordingPauseSeconds(s),
                                          ),
                                        ),
                                    ],
                                  ),
                                ),
                              ),
                            if (PrintScope.of(context) case final printing
                                when printing.available)
                              SettingsRow(
                                words: [
                                  t.printService,
                                  t.printServiceOn,
                                  t.printServiceOff,
                                  t.printServiceUnknown,
                                  t.printServiceOpen,
                                ],
                                child: _PrintServiceRow(printing: printing),
                              ),
                          ],
                        ),
                      ],
                    ),
                    _QuickNoteSettings(
                      store: widget.store,
                      headingKey: _heading('quick-note'),
                    ),
                    if (hasCalendarPage(context))
                      SettingsGroup(
                        heading: t.settingsCalendar,
                        headingKey: _heading('calendar'),
                        children: [
                          SettingsSection(
                            children: [
                              SettingsRow(
                                words: [
                                  t.calendarPageTitle,
                                  t.settingsCalendarDetail,
                                ],
                                child: ListTile(
                                  key: const ValueKey('calendarPage'),
                                  leading: const Icon(Icons.event_outlined),
                                  title: Text(t.calendarPageTitle),
                                  subtitle: Text(t.settingsCalendarDetail),
                                  onTap: () => _push(
                                    CalendarScreen(store: widget.store),
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    if (LockScope.of(context) case final lock?)
                      LockSettingsSection(
                        lock: lock,
                        alarms: widget.alarms,
                        places: Places(widget.store),
                        headingKey: _heading('privacy'),
                      ),
                    SettingsGroup(
                      heading: t.settingsTyping,
                      headingKey: _heading('typing'),
                      children: [
                        SettingsSection(
                          children: [
                            SettingsRow(
                              words: [t.twoRowEditor, t.twoRowEditorDetail],
                              child: SwitchBelow(
                                switchKey: const ValueKey('twoRowEditor'),
                                leading: const Icon(Icons.keyboard_outlined),
                                value: _twoRows,
                                onChanged: _setTwoRows,
                                title: Text(t.twoRowEditor),
                                subtitle: Text(t.twoRowEditorDetail),
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                    SettingsGroup(
                      heading: t.settingsHistory,
                      headingKey: _heading('history'),
                      children: [
                        SettingsSection(
                          children: [
                            SettingsRow(
                              words: [t.showStreaks, t.showStreaksDetail],
                              child: SwitchBelow(
                                switchKey: const ValueKey('streaks'),
                                leading: const Icon(
                                  Icons.local_fire_department_outlined,
                                ),
                                value: _streaksOn,
                                onChanged: _setStreaks,
                                title: Text(t.showStreaks),
                                subtitle: Text(t.showStreaksDetail),
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                    // The Imports are in the Areas list's ⋮ (obf-1096);
                    // a search for them says so, and its tap leads there.
                    if (_query.isNotEmpty && ImportsScreen.offered(context))
                      SettingsGroup(
                        children: [
                          SettingsSection(
                            children: [
                              SettingsRow(
                                // Found by the Imports' words, not
                                // by the Areas list's.
                                words: [
                                  t.settingsImport,
                                  t.importFolder,
                                  t.importKeep,
                                ],
                                child: ListTile(
                                  key: const ValueKey('settingsImportMoved'),
                                  leading: const Icon(
                                    Icons.drive_folder_upload_outlined,
                                  ),
                                  title: Text(t.settingsImportMoved),
                                  onTap: _openAreas,
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    // Asking the computer is experimental and offered
                    // in Advanced only (#333); switched on, it stays on
                    // in Simple, where what exists still works.
                    if (_advanced)
                      SettingsGroup(
                        heading: t.phoneAskTitle,
                        children: [
                          SettingsSection(
                            children: [
                              SettingsRow(
                                words: [
                                  t.phoneAskSwitch,
                                  t.phoneAskSwitchDetail,
                                ],
                                child: SwitchBelow(
                                  switchKey: const ValueKey('askOn'),
                                  leading: const Icon(
                                    Icons.question_answer_outlined,
                                  ),
                                  value: _askOn,
                                  onChanged: _setAsk,
                                  title: Text(t.phoneAskSwitch),
                                  subtitle: Text(t.phoneAskSwitchDetail),
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    // The last of this phone's own: whether tips show
                    // (obf-1060), and every screen explaining itself
                    // again (obf-1043).
                    SettingsGroup(
                      heading: t.settingsTips,
                      children: [
                        SettingsSection(
                          children: [
                            SettingsRow(
                              words: [
                                t.settingsTipsSwitch,
                                t.settingsTipsSwitchDetail,
                              ],
                              child: Spotlight(
                                id: 'settings-tips-switch',
                                child: ValueListenableBuilder<bool>(
                                  valueListenable: _tips,
                                  builder: (context, on, _) => SwitchBelow(
                                    switchKey: const ValueKey('tipsOn'),
                                    leading: const Icon(
                                      Icons.lightbulb_outline,
                                    ),
                                    value: on,
                                    onChanged: _tips.set,
                                    title: Text(t.settingsTipsSwitch),
                                    subtitle: Text(t.settingsTipsSwitchDetail),
                                  ),
                                ),
                              ),
                            ),
                            SettingsRow(
                              words: [t.settingsShowTips],
                              child: Spotlight(
                                id: 'settings-show-tips',
                                child: ListTile(
                                  key: const ValueKey('showTips'),
                                  leading: const Icon(
                                    Icons.tips_and_updates_outlined,
                                  ),
                                  title: Text(t.settingsShowTips),
                                  onTap: _showTips,
                                ),
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                    // The person's own, on all of their own Devices
                    // (#197): after the settings of this phone, before
                    // Send feedback.
                    SettingsGroup(
                      heading: t.features,
                      headingKey: _heading('features'),
                      children: [
                        SettingsSection(
                          children: [
                            SettingsRow(
                              words: [
                                t.featuresDetail,
                                t.featuresSimple,
                                t.featuresSimpleDetail,
                                t.featuresAdvanced,
                                t.featuresAdvancedDetail,
                              ],
                              child: Spotlight(
                                id: 'settings-features',
                                child: FeaturesChoice(
                                  advanced: _advanced,
                                  onChanged: _setAdvanced,
                                  detailPadding: const EdgeInsets.fromLTRB(
                                    16,
                                    12,
                                    16,
                                    0,
                                  ),
                                ),
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                    // The person's own too (obf-1037), Simple and
                    // Advanced alike.
                    _GrammarSettings(store: widget.store),
                    SettingsGroup(
                      children: [
                        const SizedBox(height: 16),
                        SettingsSection(
                          children: [
                            SettingsRow(
                              words: [
                                t.feedbackTitle,
                                t.feedbackSettingsDetail,
                              ],
                              child: ListTile(
                                key: const ValueKey('sendFeedback'),
                                leading: const Icon(Icons.feedback_outlined),
                                title: Text(t.feedbackTitle),
                                subtitle: Text(t.feedbackSettingsDetail),
                                onTap: () =>
                                    _push(FeedbackScreen(store: widget.store)),
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// Print to Old Brain Fix (#293): whether Android has the app's print
/// service on, read again each time the app comes back, as from Android's
/// print settings, and the button that opens them. Android lets no app
/// turn it on itself.
class _PrintServiceRow extends StatefulWidget {
  final PrintPlatform printing;

  const _PrintServiceRow({required this.printing});

  @override
  State<_PrintServiceRow> createState() => _PrintServiceRowState();
}

class _PrintServiceRowState extends State<_PrintServiceRow> {
  /// What Android said last; null until it answered.
  PrintServiceState? _state;
  late final AppLifecycleListener _lifecycle;

  @override
  void initState() {
    super.initState();
    _lifecycle = AppLifecycleListener(onResume: () => unawaited(_read()));
    unawaited(_read());
  }

  @override
  void dispose() {
    _lifecycle.dispose();
    super.dispose();
  }

  Future<void> _read() async {
    final state = await widget.printing.serviceState();
    if (!mounted || state == _state) return;
    setState(() => _state = state);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return ControlBelow(
      key: const ValueKey('printService'),
      leading: const Icon(Icons.print_outlined),
      title: Text(t.printService),
      subtitle: Text(switch (_state) {
        PrintServiceState.on => t.printServiceOn,
        PrintServiceState.off => t.printServiceOff,
        PrintServiceState.unknown || null => t.printServiceUnknown,
      }, key: const ValueKey('printService-state')),
      control: TextButton.icon(
        key: const ValueKey('printService-open'),
        onPressed: () => unawaited(widget.printing.openSettings()),
        icon: const Icon(Icons.open_in_new),
        label: Text(t.printServiceOpen),
      ),
    );
  }
}

/// This Device's Nudge settings: how long an ignored Nudge waits before
/// it comes again, and the Quiet Hours.
class _NudgeSettings extends StatefulWidget {
  /// The repeat intervals offered, in minutes.
  static const repeatChoices = [10, 30, 60];

  final Store store;
  final AlarmSync alarms;

  /// The heading's key, for an app link (#261).
  final Key? headingKey;

  const _NudgeSettings({
    required this.store,
    required this.alarms,
    this.headingKey,
  });

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
    return SettingsGroup(
      heading: t.settingsNudges,
      headingKey: widget.headingKey,
      children: [
        SettingsSection(
          children: [
            SettingsRow(
              words: [t.nudgeSettingsExplain],
              child: Padding(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
                child: Text(t.nudgeSettingsExplain),
              ),
            ),
            SettingsRow(
              words: [t.nudgeRepeat],
              child: ControlBelow(
                leading: const Icon(Icons.notifications_paused_outlined),
                title: Text(t.nudgeRepeat),
                control: ChoiceChips<int>(
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
            ),
            SettingsRow(
              words: [t.quietHours],
              child: SwitchBelow(
                switchKey: const ValueKey('quietHours'),
                leading: const Icon(Icons.bedtime_outlined),
                title: Text(t.quietHours),
                value: _quietOn,
                onChanged: (on) {
                  widget.store.quietHoursOn = on;
                  _changed();
                },
              ),
            ),
            // Found with the Quiet Hours they belong to.
            if (_quietOn) ...[
              SettingsRow(
                words: [t.quietHours, t.quietFrom],
                child: ListTile(
                  key: const ValueKey('quietFrom'),
                  leading: const SizedBox.shrink(),
                  title: Text(t.quietFrom),
                  trailing: Text(time(_quiet.start)),
                  onTap: () => _pick(start: true),
                ),
              ),
              SettingsRow(
                words: [t.quietHours, t.quietUntil],
                child: ListTile(
                  key: const ValueKey('quietUntil'),
                  leading: const SizedBox.shrink(),
                  title: Text(t.quietUntil),
                  trailing: Text(time(_quiet.end)),
                  onTap: () => _pick(start: false),
                ),
              ),
            ],
          ],
        ),
      ],
    );
  }
}

/// The quick note's settings (#352): the text, the three Nudge buttons
/// and More, nothing else, the rest in the full editor; only the Nudge
/// buttons can be switched off for a single Save. The quick note's
/// switches of before (#142) stay stored and no longer show.
class _QuickNoteSettings extends StatefulWidget {
  final Store store;

  /// The heading's key, for an app link (#261).
  final Key? headingKey;

  const _QuickNoteSettings({required this.store, this.headingKey});

  @override
  State<_QuickNoteSettings> createState() => _QuickNoteSettingsState();
}

class _QuickNoteSettingsState extends State<_QuickNoteSettings> {
  late bool _nudgeButtons = widget.store.quickNoteNudgeButtonsOn;

  void _setNudgeButtons(bool on) {
    widget.store.quickNoteNudgeButtonsOn = on;
    setState(() => _nudgeButtons = on);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return SettingsGroup(
      heading: t.settingsQuickNote,
      headingKey: widget.headingKey,
      children: [
        SettingsSection(
          children: [
            SettingsRow(
              words: [t.settingsQuickNoteDetail],
              child: Padding(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
                child: Text(t.settingsQuickNoteDetail),
              ),
            ),
            SettingsRow(
              words: [t.quickNoteNudgeButtons, t.quickNoteNudgeButtonsDetail],
              child: SwitchBelow(
                switchKey: const ValueKey('quickNote-nudgeButtons'),
                leading: const Icon(Icons.snooze),
                title: Text(t.quickNoteNudgeButtons),
                subtitle: Text(t.quickNoteNudgeButtonsDetail),
                value: _nudgeButtons,
                onChanged: _setNudgeButtons,
              ),
            ),
          ],
        ),
      ],
    );
  }
}

/// Code languages (obf-1037): the person's own grammars, a setting of the
/// person synced to all of their own Devices, each with Remove, and "Add
/// grammar…", which picks a file of any kind with the system's picker,
/// as a Sublime syntax definition has no media type. A file the cores
/// refuse is said in red under the button, with what is wrong, and
/// nothing is added.
class _GrammarSettings extends StatefulWidget {
  final Store store;

  const _GrammarSettings({required this.store});

  @override
  State<_GrammarSettings> createState() => _GrammarSettingsState();
}

class _GrammarSettingsState extends State<_GrammarSettings> {
  late List<Grammar> _added = widget.store.grammars.added;

  /// The file the last pick refused and why; null once one was added.
  ({String file, String reason})? _refused;

  var _picking = false;

  Future<void> _add() async {
    if (_picking) return;
    final files = FileScope.of(context);
    setState(() => _picking = true);
    var picked = const <DeviceFile>[];
    try {
      picked = await files.pickFiles();
    } finally {
      if (mounted) setState(() => _picking = false);
    }
    ({String file, String reason})? refused;
    var added = false;
    try {
      for (final file in picked) {
        try {
          widget.store.addGrammar(_grammarBytes(file.path));
          added = true;
        } on GrammarRefused catch (e) {
          refused ??= (file: _fileName(file), reason: e.reason);
        } on FileSystemException catch (e) {
          refused ??= (file: _fileName(file), reason: e.message);
        }
      }
    } finally {
      dropDeviceFiles(picked);
    }
    if (!mounted) return;
    setState(() {
      _added = widget.store.grammars.added;
      if (refused != null) {
        _refused = refused;
      } else if (added) {
        _refused = null;
      }
    });
  }

  void _remove(String scope) {
    widget.store.removeGrammar(scope);
    setState(() => _added = widget.store.grammars.added);
  }

  /// An added grammar: its name, the built-in one it replaces or else its
  /// scope, and Remove, which removes it at once.
  SettingsRow _grammarRow(AppLocalizations t, Grammar grammar) {
    final detail = switch (Grammars.builtIn.byScope(grammar.scope)) {
      final builtIn? => t.grammarInPlaceOf(builtIn.name),
      null => grammar.scope,
    };
    return SettingsRow(
      words: [t.grammarsTitle, grammar.name, detail],
      child: ListTile(
        key: ValueKey('grammar-${grammar.scope}'),
        leading: const Icon(Icons.code),
        title: Text(grammar.name),
        subtitle: Text(detail),
        trailing: TextButton(
          key: ValueKey('grammar-remove-${grammar.scope}'),
          onPressed: () => _remove(grammar.scope),
          child: Text(t.grammarRemove),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final refused = _refused;
    return SettingsGroup(
      heading: t.grammarsTitle,
      children: [
        SettingsSection(
          children: [
            SettingsRow(
              words: [t.grammarsTitle, t.grammarsHint],
              child: Padding(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
                child: Text(t.grammarsHint),
              ),
            ),
            for (final grammar in _added) _grammarRow(t, grammar),
            if (FileScope.of(context).pickable)
              SettingsRow(
                words: [t.grammarsTitle, t.grammarAdd],
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Spotlight(
                      id: 'settings-grammar',
                      child: ListTile(
                        key: const ValueKey('grammarAdd'),
                        leading: const Icon(Icons.add),
                        title: Text(t.grammarAdd),
                        enabled: !_picking,
                        onTap: _add,
                      ),
                    ),
                    if (refused != null)
                      Padding(
                        padding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
                        child: Text(
                          t.grammarRefused(refused.file, refused.reason),
                          key: const ValueKey('grammarRefused'),
                          style: TextStyle(
                            color: Theme.of(context).colorScheme.error,
                          ),
                        ),
                      ),
                  ],
                ),
              ),
          ],
        ),
      ],
    );
  }
}

/// A picked file's name as the picker gave it, else its copy's.
String _fileName(DeviceFile file) => file.name.isNotEmpty
    ? file.name
    : file.path.substring(file.path.lastIndexOf(Platform.pathSeparator) + 1);

/// The first [maxGrammarBytes] and one more of the file at [path]: enough
/// for the core to refuse a larger one, without reading a large one whole.
List<int> _grammarBytes(String path) {
  final file = File(path).openSync();
  try {
    return file.readSync(maxGrammarBytes + 1);
  } finally {
    file.closeSync();
  }
}
