import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'agenda_screen.dart';
import 'alarm_platform.dart';
import 'alarm_sync.dart';
import 'announcements.dart';
import 'browser.dart';
import 'drafts.dart';
import 'l10n.dart';
import 'location.dart';
import 'look.dart';
import 'question_mark_background.dart';
import 'routine_editor_screen.dart';
import 'setup_screen.dart';
import 'starter_set_screen.dart';

class OldBrainFixApp extends StatefulWidget {
  final Store store;

  /// What the app has to say; the app keeps its own when none is given.
  final Announcements? announcements;

  /// Where the Note editor keeps unsaved text; in memory when none is
  /// given.
  final Drafts? drafts;

  /// Where the Device is, for the Capture Context; none when not given.
  final LocationSource location;

  /// The Device's alarm layer; none when not given.
  final AlarmPlatform alarms;

  const OldBrainFixApp({
    super.key,
    required this.store,
    this.announcements,
    this.drafts,
    this.location = const NoLocation(),
    this.alarms = const NoAlarms(),
  });

  @override
  State<OldBrainFixApp> createState() => _OldBrainFixAppState();
}

class _OldBrainFixAppState extends State<OldBrainFixApp> {
  final _scroll = ParallaxScroll();
  final _light = lookTheme(Brightness.light);
  final _dark = lookTheme(Brightness.dark);
  late final _announcements = widget.announcements ?? Announcements();
  late final _drafts = widget.drafts ?? MemoryDrafts();
  late final _themeMode = themeModeOf(widget.store.themeChoice);
  late final _notes = Notes(widget.store);
  late final _places = Places(widget.store);
  final _trail = BrowserTrail();
  late final AlarmSync _alarms;

  @override
  void initState() {
    super.initState();
    // What has been in the trash for 30 days goes for good.
    _notes.purgeTrash();
    _alarms = AlarmSync(
      store: widget.store,
      platform: widget.alarms,
      wallClock: deviceWallClock,
    );
  }

  @override
  void dispose() {
    _alarms.dispose();
    _scroll.dispose();
    if (widget.announcements == null) _announcements.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => MaterialApp(
    onGenerateTitle: (context) => AppLocalizations.of(context).appTitle,
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    theme: _light,
    darkTheme: _dark,
    themeMode: _themeMode,
    navigatorObservers: [_scroll],
    builder: (context, child) => QuestionMarkBackground(
      scroll: _scroll,
      child: AnnouncementStrip(
        announcements: _announcements,
        child: AnnouncementsScope(
          announcements: _announcements,
          child: LocationScope(
            location: widget.location,
            child: DraftsScope(
              drafts: _drafts,
              child: child ?? const SizedBox.shrink(),
            ),
          ),
        ),
      ),
    ),
    home: _Home(
      store: widget.store,
      notes: _notes,
      places: _places,
      trail: _trail,
      alarms: _alarms,
    ),
  );
}

/// The Starter Set on this Device's first start, then the alarm setup
/// where there are alarms, the Agenda after them. Keeps the alarms in
/// step while the app runs.
class _Home extends StatefulWidget {
  final Store store;
  final Notes notes;
  final Places places;
  final BrowserTrail trail;
  final AlarmSync alarms;

  const _Home({
    required this.store,
    required this.notes,
    required this.places,
    required this.trail,
    required this.alarms,
  });

  @override
  State<_Home> createState() => _HomeState();
}

class _HomeState extends State<_Home> {
  late bool _starterSetOffered = widget.store.starterSetOffered;
  late bool _alarmSetupDone =
      widget.store.alarmSetupDone || !widget.alarms.platform.available;
  late final AppLifecycleListener _lifecycle;

  @override
  void initState() {
    super.initState();
    // Back from the background, or from a settings page: apply what was
    // tapped meanwhile, see what is allowed now, arm again. Arms on the
    // way out as well, so a changed Device setting (the Re-nag interval)
    // is armed even when the log did not change.
    _lifecycle = AppLifecycleListener(
      onResume: widget.alarms.refresh,
      onHide: widget.alarms.refresh,
    );
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    widget.alarms.setTexts(
      context.t,
      Localizations.localeOf(context).toString(),
    );
  }

  @override
  void dispose() {
    _lifecycle.dispose();
    super.dispose();
  }

  /// Plants what the user kept of the Starter Set; it is not offered
  /// again on this Device.
  void _plant(Set<String> picked) {
    widget.places.plantStarterSet(picked);
    widget.store.starterSetOffered = true;
    setState(() => _starterSetOffered = true);
  }

  /// The alarm setup was walked through; it is not offered again here.
  void _setUp() {
    widget.store.alarmSetupDone = true;
    setState(() => _alarmSetupDone = true);
  }

  @override
  Widget build(BuildContext context) {
    if (!_starterSetOffered) return StarterSetScreen(onDone: _plant);
    if (!_alarmSetupDone) {
      return SetupScreen(alarms: widget.alarms, onDone: _setUp);
    }
    return AgendaScreen(
      notes: widget.notes,
      places: widget.places,
      trail: widget.trail,
      alarms: widget.alarms,
    );
  }
}
