import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'about_screen.dart';
import 'alarm_sync.dart';
import 'announcements.dart';
import 'app_lock.dart';
import 'areas_screen.dart';
import 'browser.dart';
import 'calendar_screen.dart';
import 'capture_context_screen.dart';
import 'context_platform.dart';
import 'failure_dialog.dart';
import 'features_choice.dart';
import 'feedback_screen.dart';
import 'folder_import.dart';
import 'folder_platform.dart';
import 'keep_import.dart';
import 'l10n.dart';
import 'lock_settings.dart';
import 'look.dart';
import 'named_places_screen.dart';
import 'page_bar.dart';
import 'place_names.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';
import 'setup_screen.dart';
import 'speech_model_screen.dart';
import 'speech_platform.dart';
import 'system_bars.dart';

/// How this Device shows the known units of Tallies: as its region
/// does, or a system chosen.
enum _UnitChoice { region, metric, imperial }

/// The Imports in Settings: a folder of files, or Google Keep's export.
enum _Importing { folder, keep }

/// This Device's settings (ADR 0008), reached only through the Profile
/// page: the theme and "Fat fingers"; the units of Tallies; the alarm setup, the default Re-nag interval and the
/// alarm sound; the
/// Nudges' repeat and Quiet Hours; the Named Places, the Capture Context
/// switches and the speech model; what the quick note shows; the way to the Calendar page; the App Lock and
/// the Area Locks; whether Streaks show; the Import of a folder or of a
/// Google Keep export; Send feedback (#214); and About. Each is a local setting, never synced,
/// and applies at once; the alarms are armed again after each change.
class SettingsScreen extends StatefulWidget {
  final Store store;
  final AlarmSync alarms;

  /// Called after the theme or "Fat fingers" was changed, to show it at
  /// once.
  final VoidCallback? onThemeChanged;

  /// The notes browser's way back in, which the Area an Import made
  /// opens as the last selected one.
  final BrowserTrail? trail;

  const SettingsScreen({
    super.key,
    required this.store,
    required this.alarms,
    this.onThemeChanged,
    this.trail,
  });

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  late ThemeChoice _theme = widget.store.themeChoice;
  late bool _fatFingers = widget.store.fatFingers;
  late Duration _renag = widget.store.renagInterval;
  late bool _streaksOn = widget.store.streaksOn;
  late bool _hidesChecked = widget.store.agendaHidesChecked;
  late bool _trashesDone = widget.store.trashesDoneReminder;
  late bool _advanced = widget.store.advanced;
  late bool _twoRows = widget.store.twoRowEditorOn;
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

  void _setTwoRows(bool on) {
    widget.store.twoRowEditorOn = on;
    setState(() => _twoRows = on);
  }

  /// Which Import is reading its folder or zip, null while none is.
  _Importing? _importing;

  /// Picks a folder with the system's picker and makes Notes of its
  /// Markdown and text files in a new Area (ADR 0036), announced as
  /// [_runImport] does.
  Future<void> _import() async {
    if (_importing != null) return;
    final t = context.t;
    final folders = FolderScope.of(context);
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context, rootNavigator: true);
    final store = widget.store;
    final folder = await folders.pick();
    if (folder == null) return;
    var name = '';
    await _runImport(
      _Importing.folder,
      t,
      announcements,
      navigator,
      run: () async {
        name = await folders.displayName(folder);
        return importFolder(store, folders, folder, t.importAreaName(name));
      },
      finished: (summary, area) => [
        t.importFinishedNotes(summary.notes, area),
        if (summary.skipped > 0) ...[
          '',
          t.importFinishedSkipped(summary.skipped),
          t.importSkippedDetails(name, summary.notText, summary.tooLarge),
        ],
      ].join('\n'),
      nothing: (summary) => (
        announced: t.importNothing,
        cause: t.importNothingCause,
        fix: t.importNothingFix,
        details: t.importSkippedDetails(
          name,
          summary.notText,
          summary.tooLarge,
        ),
      ),
      failed: (
        announced: t.importFailed,
        cause: t.importFailedCause,
        fix: t.importFailedFix,
      ),
      // The folder was picked for this Import alone: its lasting access
      // goes, unless the app syncs or backs up through it.
      release: () =>
          folders.release(folder, [?store.syncFolder, ?store.backupFolder]),
    );
  }

  /// Asks whether the Google Takeout export is still a zip or an unpacked
  /// folder, picks it with the system's picker and makes Notes of its
  /// Google Keep notes in the new Area "From Google Keep" (ADR 0036),
  /// announced as [_runImport] does.
  Future<void> _importKeep() async {
    if (_importing != null) return;
    final t = context.t;
    final folders = FolderScope.of(context);
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context, rootNavigator: true);
    final store = widget.store;
    final zip = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(context.t.importKeep),
        // The two answers stacked full width, Cancel below (UI laws).
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(context.t.importKeepAsk),
              const SizedBox(height: 16),
              FilledButton.tonal(
                key: const ValueKey('importKeepZip'),
                onPressed: () => Navigator.of(context).pop(true),
                child: Text(context.t.importKeepZip),
              ),
              const SizedBox(height: 8),
              FilledButton.tonal(
                key: const ValueKey('importKeepFolder'),
                onPressed: () => Navigator.of(context).pop(false),
                child: Text(context.t.importKeepFolder),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(context.t.cancel),
          ),
        ],
      ),
    );
    if (zip == null || !mounted || _importing != null) return;
    final source = zip ? await folders.pickZip() : await folders.pick();
    if (source == null) return;
    await _runImport(
      _Importing.keep,
      t,
      announcements,
      navigator,
      run: () => zip
          ? importKeepZip(store, folders, source, t.importKeepAreaName)
          : importKeepFolder(store, folders, source, t.importKeepAreaName),
      finished: (summary, area) => [
        t.importFinishedNotes(summary.notes, area),
        if (summary.checklists > 0) t.importKeepChecklists(summary.checklists),
        if (summary.photos > 0) t.importKeepPhotos(summary.photos),
        if (summary.skipped > 0) ...[
          '',
          t.importKeepSkipped(summary.skipped),
          _keepSkipped(t, summary),
        ],
      ].join('\n'),
      nothing: (summary) => (
        announced: t.importKeepNothing,
        cause: t.importKeepNothingCause,
        fix: t.importKeepNothingFix,
        details: _keepSkipped(t, summary),
      ),
      failed: (
        announced: t.importKeepFailed,
        cause: t.importKeepFailedCause,
        fix: t.importKeepFailedFix,
      ),
      // A zip leaves no lasting access behind, only the copies of its
      // files; a folder's goes as for any Import.
      release: () => zip
          ? Future<void>.value()
          : folders.release(source, [?store.syncFolder, ?store.backupFolder]),
    );
  }

  static String _keepSkipped(AppLocalizations t, ImportSummary summary) =>
      t.importKeepSkippedDetails(
        summary.trashed,
        summary.missing,
        summary.notPhoto,
        summary.tooLarge,
        summary.notText,
      );

  /// Runs the Import [kind] by [run], its row showing that it reads
  /// meanwhile, and announces at the top in [t], taken before any await,
  /// once done, even after this page closed, without values (UI laws):
  /// its tap shows what went where, [finished] says it of the new Area,
  /// with a way into the Area; or, with nothing to import, why
  /// ([nothing]); or that it [failed]. Then gives up what was picked for
  /// it ([release]).
  Future<void> _runImport(
    _Importing kind,
    AppLocalizations t,
    Announcements? announcements,
    NavigatorState navigator, {
    required Future<ImportSummary> Function() run,
    required String Function(ImportSummary summary, String area) finished,
    required ({String announced, String cause, String fix, String details})
    Function(ImportSummary summary)
    nothing,
    required ({String announced, String cause, String fix}) failed,
    required Future<void> Function() release,
  }) async {
    final store = widget.store;
    final trail = widget.trail ?? BrowserTrail();
    if (mounted) setState(() => _importing = kind);
    try {
      final summary = await run();
      final area = switch (summary.areaId) {
        final id? => Places(store).areaById(id),
        null => null,
      };
      if (area == null) {
        final why = nothing(summary);
        announceFailureWith(
          announcements,
          navigator,
          why.announced,
          title: t.importNothingTitle,
          cause: why.cause,
          fix: why.fix,
          details: why.details,
        );
        return;
      }
      final details = finished(summary, areaName(t, area));
      announcements?.announce(
        t.importFinished,
        onTap: () => unawaited(
          showDialog<void>(
            context: navigator.context,
            builder: (context) => AlertDialog(
              title: Text(context.t.importFinished),
              content: SingleChildScrollView(child: Text(details)),
              actions: [
                TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: Text(context.t.close),
                ),
                TextButton(
                  key: const ValueKey('importOpenArea'),
                  onPressed: () {
                    Navigator.of(context).pop();
                    _openImported(navigator, store, trail, area);
                  },
                  child: Text(context.t.importOpenArea),
                ),
              ],
            ),
          ),
        ),
      );
    } on Exception catch (e) {
      announceFailureWith(
        announcements,
        navigator,
        failed.announced,
        title: t.importFailedTitle,
        cause: failed.cause,
        fix: failed.fix,
        details: '$e',
      );
    } finally {
      unawaited(release());
      if (mounted) setState(() => _importing = null);
    }
  }

  /// The notes browser on top, the Areas list and in it the imported
  /// [area], chosen as the last selected one; the Areas list alone while
  /// [area] was locked and is closed meanwhile.
  static void _openImported(
    NavigatorState navigator,
    Store store,
    BrowserTrail trail,
    Area area,
  ) {
    final notes = Notes(store);
    final places = Places(store);
    trail.choseArea(area);
    unawaited(
      navigator.push(
        BrowserRoute<void>(
          builder: (_) =>
              AreasScreen(notes: notes, places: places, trail: trail),
        ),
      ),
    );
    final closed =
        LockScope.of(navigator.context)?.isAreaClosed(area.id) ?? false;
    if (closed) return;
    unawaited(
      navigator.push(
        BrowserRoute<void>(
          builder: (_) => AreaScreen(
            notes: notes,
            places: places,
            trail: trail,
            areaId: area.id,
          ),
        ),
      ),
    );
  }

  /// The row of the Import [kind]: [title] over [detail], or over
  /// [reading] with a spinner while it runs; no row takes a tap while
  /// any Import runs.
  Widget _importRow(
    _Importing kind, {
    required Key key,
    required IconData icon,
    required String title,
    required String detail,
    required String reading,
    required VoidCallback onTap,
  }) {
    final running = _importing == kind;
    return ListTile(
      key: key,
      leading: Icon(icon),
      title: Text(title),
      subtitle: Text(running ? reading : detail),
      trailing: running
          ? const SizedBox.square(
              dimension: 24,
              child: CircularProgressIndicator(strokeWidth: 3),
            )
          : null,
      enabled: _importing == null,
      onTap: onTap,
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
    return Scaffold(
      appBar: pageBar(context, title: t.settingsTitle),
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
              child: ChoiceChips<ThemeChoice>(
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
            SettingsSection(
              children: [
                SwitchBelow(
                  switchKey: const ValueKey('fatFingers'),
                  leading: const Icon(Icons.touch_app_outlined),
                  value: _fatFingers,
                  onChanged: _setFatFingers,
                  title: Text(t.fatFingers),
                  subtitle: Text(t.fatFingersDetail),
                ),
              ],
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
            SettingsHeading(t.settingsAgenda),
            SettingsSection(
              children: [
                SwitchBelow(
                  switchKey: const ValueKey('agendaHideChecked'),
                  leading: const Icon(Icons.playlist_remove),
                  value: _hidesChecked,
                  onChanged: _setHidesChecked,
                  title: Text(t.hideCheckedOnAgenda),
                  subtitle: Text(t.hideCheckedOnAgendaDetail),
                ),
                SwitchBelow(
                  switchKey: const ValueKey('trashDoneReminder'),
                  leading: const Icon(Icons.auto_delete_outlined),
                  value: _trashesDone,
                  onChanged: _setTrashesDone,
                  title: Text(t.trashDoneReminder),
                  subtitle: Text(t.trashDoneReminderDetail),
                ),
              ],
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
                  control: ChoiceChips<int>(
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
                ListTile(
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
            SettingsHeading(t.settingsQuickNote),
            _QuickNoteSettings(store: widget.store, advanced: _advanced),
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
            SettingsHeading(t.settingsTyping),
            SettingsSection(
              children: [
                SwitchBelow(
                  switchKey: const ValueKey('twoRowEditor'),
                  leading: const Icon(Icons.keyboard_outlined),
                  value: _twoRows,
                  onChanged: _setTwoRows,
                  title: Text(t.twoRowEditor),
                  subtitle: Text(t.twoRowEditorDetail),
                ),
              ],
            ),
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
            if (FolderScope.of(context).available) ...[
              SettingsHeading(t.settingsImport),
              SettingsSection(
                children: [
                  _importRow(
                    _Importing.folder,
                    key: const ValueKey('importFolder'),
                    icon: Icons.drive_folder_upload_outlined,
                    title: t.importFolder,
                    detail: t.importFolderDetail,
                    reading: t.importReading,
                    onTap: _import,
                  ),
                  _importRow(
                    _Importing.keep,
                    key: const ValueKey('importKeep'),
                    icon: Icons.sticky_note_2_outlined,
                    title: t.importKeep,
                    detail: t.importKeepDetail,
                    reading: t.importKeepReading,
                    onTap: _importKeep,
                  ),
                ],
              ),
            ],
            // The person's own, on all of their own Devices (#197): after
            // the settings of this phone, before About.
            SettingsHeading(t.features),
            FeaturesChoice(advanced: _advanced, onChanged: _setAdvanced),
            const SizedBox(height: 16),
            SettingsSection(
              children: [
                ListTile(
                  key: const ValueKey('sendFeedback'),
                  leading: const Icon(Icons.feedback_outlined),
                  title: Text(t.feedbackTitle),
                  subtitle: Text(t.feedbackSettingsDetail),
                  onTap: () => _push(FeedbackScreen(store: widget.store)),
                ),
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

/// What this Device's quick note shows besides its text field, its
/// Photo, Voice, Gallery and Paste and "To full editor" (#142): each of
/// the card's actions and the format bar once switched on, and the Nudge
/// buttons until switched off.
class _QuickNoteSettings extends StatefulWidget {
  final Store store;

  /// Whether the format bar's switch shows: only in Advanced (#197).
  final bool advanced;

  const _QuickNoteSettings({required this.store, required this.advanced});

  @override
  State<_QuickNoteSettings> createState() => _QuickNoteSettingsState();
}

class _QuickNoteSettingsState extends State<_QuickNoteSettings> {
  late final _shows = {
    for (final control in QuickNoteControl.values)
      control: widget.store.quickNoteShows(control),
  };
  late bool _nudgeButtons = widget.store.quickNoteNudgeButtonsOn;

  void _set(QuickNoteControl control, bool on) {
    widget.store.setQuickNoteShows(control, on);
    setState(() => _shows[control] = on);
  }

  void _setNudgeButtons(bool on) {
    widget.store.quickNoteNudgeButtonsOn = on;
    setState(() => _nudgeButtons = on);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    Widget shows(QuickNoteControl control, IconData icon, String title) =>
        SwitchBelow(
          switchKey: ValueKey('quickNote-${control.name}'),
          leading: Icon(icon),
          title: Text(title),
          value: _shows[control] ?? false,
          onChanged: (on) => _set(control, on),
        );
    return SettingsSection(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
          child: Text(t.settingsQuickNoteDetail),
        ),
        shows(
          QuickNoteControl.fileInto,
          Icons.drive_file_move_outline,
          t.fileInto,
        ),
        // The alarm clock offers a Routine or Reminder without it (#223);
        // Frogs, Sessions and Tallies are made only in Advanced (#197).
        if (widget.advanced)
          shows(
            QuickNoteControl.schedule,
            Icons.alarm_add_outlined,
            t.quickNoteSchedule,
          ),
        shows(
          QuickNoteControl.shopping,
          Icons.add_shopping_cart,
          t.addToShoppingList,
        ),
        shows(QuickNoteControl.importance, Icons.priority_high, t.importance),
        if (widget.advanced)
          shows(
            QuickNoteControl.formatBar,
            Icons.format_bold,
            t.quickNoteFormatBar,
          ),
        SwitchBelow(
          switchKey: const ValueKey('quickNote-nudgeButtons'),
          leading: const Icon(Icons.snooze),
          title: Text(t.quickNoteNudgeButtons),
          subtitle: Text(t.quickNoteNudgeButtonsDetail),
          value: _nudgeButtons,
          onChanged: _setNudgeButtons,
        ),
      ],
    );
  }
}
