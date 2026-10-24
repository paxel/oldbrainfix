import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'agenda_screen.dart';
import 'alarm_platform.dart';
import 'alarm_sync.dart';
import 'announcements.dart';
import 'app_lock.dart';
import 'browser.dart';
import 'calendar_mirror.dart';
import 'calendar_platform.dart';
import 'capture_requests.dart';
import 'capture_screen.dart';
import 'context_platform.dart';
import 'drafts.dart';
import 'l10n.dart';
import 'location.dart';
import 'lock_platform.dart';
import 'lock_screen.dart';
import 'look.dart';
import 'nudge_screen.dart';
import 'ocr_platform.dart';
import 'photo_platform.dart';
import 'question_mark_background.dart';
import 'ring_screen.dart';
import 'routine_editor_screen.dart';
import 'setup_screen.dart';
import 'speech_platform.dart';
import 'starter_set_screen.dart';
import 'voice_platform.dart';

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

  /// The Device's camera and what other apps share in; none when not
  /// given.
  final PhotoPlatform photos;

  /// The Device's microphone and playback of voice recordings; none when
  /// not given.
  final VoicePlatform voice;

  /// The calendar and Bluetooth devices for the Capture Context; none
  /// when not given.
  final ContextSource context;

  /// The quick entry points asking for a capture screen; none when not
  /// given.
  final CaptureRequests requests;

  /// Whisper and its model for Transcripts; none when not given.
  final SpeechPlatform speech;

  /// ocrs for Photo Texts; none when not given.
  final OcrPlatform ocr;

  /// The calendar app to add to and the apps to share calendar files
  /// with; none when not given.
  final CalendarPlatform calendar;

  /// The Device's calendars to mirror Routines and Reminders into; none
  /// when not given.
  final MirrorPlatform mirror;

  /// How long the calendar mirror waits after a change for more to come.
  final Duration mirrorDelay;

  /// Biometrics and the screen lock for the App Lock and the Area Locks;
  /// none when not given, and the PIN unlocks.
  final LockPlatform lock;

  const OldBrainFixApp({
    super.key,
    required this.store,
    this.announcements,
    this.drafts,
    this.location = const NoLocation(),
    this.alarms = const NoAlarms(),
    this.photos = const NoPhotos(),
    this.voice = const NoVoice(),
    this.context = const NoContext(),
    this.requests = const NoCaptureRequests(),
    this.speech = const NoSpeech(),
    this.ocr = const NoOcr(),
    this.calendar = const NoCalendar(),
    this.mirror = const NoMirror(),
    this.mirrorDelay = CalendarMirror.defaultDelay,
    this.lock = const NoBiometrics(),
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
  late var _themeMode = themeModeOf(widget.store.themeChoice);
  late final _notes = Notes(widget.store);
  late final _places = Places(widget.store);
  final _trail = BrowserTrail();
  late final AlarmSync _alarms;
  late final _mirror = CalendarMirror(
    store: widget.store,
    platform: widget.mirror,
    delay: widget.mirrorDelay,
  );
  late final _playback = Playback(
    voice: widget.voice,
    bytes: _notes.recordingBytes,
  );
  late final _speechModel = SpeechModel(widget.speech);
  late final _lock = AppLock(store: widget.store, platform: widget.lock);
  final _routes = RouteStack();
  late final Transcriber _transcriber;
  late final PhotoReader _photoReader;

  @override
  void initState() {
    super.initState();
    // What has been in the trash for 30 days goes for good.
    _notes.purgeTrash();
    Routines(widget.store).purgeTrash();
    _alarms = AlarmSync(
      store: widget.store,
      platform: widget.alarms,
      wallClock: deviceWallClock,
    );
    // Transcripts come in the background once the model is checked.
    _transcriber = Transcriber(
      notes: _notes,
      model: _speechModel,
      platform: widget.speech,
    );
    unawaited(_speechModel.check());
    // Photo Texts come in the background too, never holding Capture up.
    _photoReader = PhotoReader(notes: _notes, platform: widget.ocr);
  }

  @override
  void dispose() {
    _alarms.dispose();
    _mirror.dispose();
    _playback.dispose();
    _transcriber.dispose();
    _speechModel.dispose();
    _photoReader.dispose();
    _lock.dispose();
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
    navigatorObservers: [_scroll, _routes],
    builder: (context, child) => QuestionMarkBackground(
      scroll: _scroll,
      child: AnnouncementStrip(
        announcements: _announcements,
        child: AnnouncementsScope(
          announcements: _announcements,
          child: LocationScope(
            location: widget.location,
            child: ContextScope(
              source: widget.context,
              child: PhotoScope(
                photos: widget.photos,
                bytes: _notes.photoBytes,
                child: VoiceScope(
                  voice: widget.voice,
                  playback: _playback,
                  child: SpeechScope(
                    model: _speechModel,
                    transcriber: _transcriber,
                    child: PhotoReaderScope(
                      reader: _photoReader,
                      child: CalendarScope(
                        calendar: widget.calendar,
                        child: CalendarMirrorScope(
                          mirror: _mirror,
                          child: DraftsScope(
                            drafts: _drafts,
                            child: LockScope(
                              lock: _lock,
                              child: child ?? const SizedBox.shrink(),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
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
      mirror: _mirror,
      lock: _lock,
      routes: _routes,
      requests: widget.requests,
      onThemeChanged: _themeChanged,
    ),
  );

  /// Shows the theme this Device chose in Settings at once.
  void _themeChanged() =>
      setState(() => _themeMode = themeModeOf(widget.store.themeChoice));
}

/// Keeps track of the routes on the app's navigator, bottom first, so
/// the browser can go when the app locks while Capture stays.
class RouteStack extends NavigatorObserver {
  final _routes = <Route<dynamic>>[];

  /// The routes now, bottom first.
  List<Route<dynamic>> get routes => List.unmodifiable(_routes);

  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) =>
      _routes.add(route);

  @override
  void didPop(Route<dynamic> route, Route<dynamic>? previousRoute) =>
      _routes.remove(route);

  @override
  void didRemove(Route<dynamic> route, Route<dynamic>? previousRoute) =>
      _routes.remove(route);

  @override
  void didReplace({Route<dynamic>? newRoute, Route<dynamic>? oldRoute}) {
    final at = oldRoute == null ? -1 : _routes.indexOf(oldRoute);
    if (newRoute == null) {
      if (at >= 0) _routes.removeAt(at);
    } else if (at < 0) {
      _routes.add(newRoute);
    } else {
      _routes[at] = newRoute;
    }
  }
}

/// The Starter Set on this Device's first start, then the alarm setup
/// where there are alarms, the Agenda after them. Keeps the alarms in
/// step while the app runs, shows the full-screen alarm on top while
/// one rings, the Nudge screen when the Nudge's notification was
/// tapped, and the Unsorted list when the Agenda widget's Unsorted
/// count was. While the App Lock is locked, the lock screen stands in
/// front of the Agenda, which still opens Capture for shares and quick
/// entry points; what was open of the browser goes when the app locks
/// again, while Capture and the full-screen alarm stay, and the Nudge
/// screen and the Unsorted list wait for the unlock.
class _Home extends StatefulWidget {
  final Store store;
  final Notes notes;
  final Places places;
  final BrowserTrail trail;
  final AlarmSync alarms;
  final CalendarMirror mirror;
  final AppLock lock;
  final RouteStack routes;
  final CaptureRequests requests;
  final VoidCallback onThemeChanged;

  const _Home({
    required this.store,
    required this.notes,
    required this.places,
    required this.trail,
    required this.alarms,
    required this.mirror,
    required this.lock,
    required this.routes,
    required this.requests,
    required this.onThemeChanged,
  });

  @override
  State<_Home> createState() => _HomeState();
}

class _HomeState extends State<_Home> {
  late bool _starterSetOffered = widget.store.starterSetOffered;
  late bool _alarmSetupDone =
      widget.store.alarmSetupDone || !widget.alarms.platform.available;
  late final AppLifecycleListener _lifecycle;

  /// The full-screen alarm on screen, and what it shows.
  Route<void>? _ringRoute;
  Ringing? _ringShown;

  /// Whether the app is locked, as shown.
  late bool _locked = widget.lock.locked;

  /// The Nudge screen and the Unsorted list, asked for while locked.
  var _nudgeWaiting = false;
  var _unsortedWaiting = false;

  @override
  void initState() {
    super.initState();
    widget.alarms.addListener(_ringChanged);
    widget.alarms.addListener(_nudgeOpened);
    widget.alarms.addListener(_unsortedOpened);
    widget.lock.addListener(_lockChanged);
    // Back from the background, or from a settings page: apply what was
    // tapped meanwhile, see what is allowed now, arm again. Arms on the
    // way out as well, so a changed Device setting is armed even when the
    // log did not change.
    _lifecycle = AppLifecycleListener(
      onResume: _resumed,
      onHide: _hidden,
      onShow: widget.lock.shown,
    );
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    widget.alarms.setTexts(
      context.t,
      Localizations.localeOf(context).toString(),
    );
    widget.mirror.setTexts(context.t);
  }

  void _hidden() {
    widget.lock.hidden();
    unawaited(widget.alarms.refresh());
  }

  /// Back from the background: the alarms as above, and the calendar
  /// mirror in step with what other Devices synced meanwhile.
  void _resumed() {
    unawaited(widget.alarms.refresh());
    unawaited(widget.mirror.sync());
  }

  @override
  void dispose() {
    widget.alarms.removeListener(_ringChanged);
    widget.alarms.removeListener(_nudgeOpened);
    widget.alarms.removeListener(_unsortedOpened);
    widget.lock.removeListener(_lockChanged);
    _lifecycle.dispose();
    super.dispose();
  }

  /// Opens the full-screen alarm when one starts ringing, and closes it
  /// once it is answered or stops.
  void _ringChanged() {
    if (!mounted) return;
    final ringing = widget.alarms.ringing;
    if (ringing == _ringShown) return;
    final navigator = Navigator.of(context);
    final open = _ringRoute;
    if (open != null && open.isActive) navigator.removeRoute(open);
    _ringRoute = null;
    _ringShown = ringing;
    if (ringing == null) return;
    final route = MaterialPageRoute<void>(
      settings: const RouteSettings(name: LockFreeRoutes.ring),
      fullscreenDialog: true,
      builder: (_) => RingScreen(alarms: widget.alarms, ring: ringing),
    );
    _ringRoute = route;
    unawaited(navigator.push(route));
  }

  /// Shows the lock screen when the app locks, with the browser gone,
  /// and what waited for the unlock after it.
  void _lockChanged() {
    if (!mounted) return;
    final locked = widget.lock.locked;
    if (locked == _locked) return;
    setState(() => _locked = locked);
    if (locked) {
      _clearBrowsing();
      return;
    }
    if (_nudgeWaiting) _openNudge();
    if (_unsortedWaiting) {
      _unsortedWaiting = false;
      unawaited(openUnsorted(context, widget.notes, widget.places));
    }
  }

  /// Removes every route over the Agenda up to the first that needs no
  /// unlocking: Capture, and what it opened, stays and the full-screen
  /// alarm too, the browser goes. Unsaved text in the editor is kept in
  /// its draft.
  void _clearBrowsing() {
    final navigator = Navigator.of(context);
    final routes = widget.routes.routes;
    final free = routes.indexWhere(LockFreeRoutes.contains);
    final end = free < 0 ? routes.length : free;
    if (end <= 1) return;
    for (final route in routes.sublist(1, end).reversed) {
      navigator.removeRoute(route);
    }
  }

  /// Capture from the lock screen, the same as "+" on the Agenda.
  void _capture() =>
      unawaited(captureNote(context, widget.notes, onChanged: () {}));

  /// Opens the Nudge screen when the Nudge's notification was tapped.
  void _nudgeOpened() {
    if (!mounted || !widget.alarms.takeNudgeOpened()) return;
    if (widget.lock.locked) {
      _nudgeWaiting = true;
      return;
    }
    _openNudge();
  }

  void _openNudge() {
    _nudgeWaiting = false;
    unawaited(
      Navigator.of(context).push(
        MaterialPageRoute<void>(
          builder: (_) =>
              NudgeScreen(notes: widget.notes, places: widget.places),
        ),
      ),
    );
  }

  /// Opens the Unsorted list when the Agenda widget's Unsorted count
  /// was tapped.
  void _unsortedOpened() {
    if (!mounted || !widget.alarms.takeUnsortedOpened()) return;
    if (widget.lock.locked) {
      _unsortedWaiting = true;
      return;
    }
    unawaited(openUnsorted(context, widget.notes, widget.places));
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
    // The Agenda stays behind the lock screen, hidden, so shares and
    // quick entry points still open Capture.
    return Stack(
      fit: StackFit.expand,
      children: [
        Offstage(
          offstage: _locked,
          child: TickerMode(
            enabled: !_locked,
            child: AgendaScreen(
              notes: widget.notes,
              places: widget.places,
              trail: widget.trail,
              alarms: widget.alarms,
              requests: widget.requests,
              onThemeChanged: widget.onThemeChanged,
            ),
          ),
        ),
        if (_locked) LockScreen(lock: widget.lock, onCapture: _capture),
      ],
    );
  }
}
