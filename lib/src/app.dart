import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'agenda_reader.dart';
import 'agenda_screen.dart';
import 'alarm_platform.dart';
import 'alarm_sync.dart';
import 'announcements.dart';
import 'app_links.dart';
import 'app_lock.dart';
import 'appointments.dart';
import 'areas_screen.dart';
import 'auto_sync.dart';
import 'browser.dart';
import 'bundle_platform.dart';
import 'conflicts_screen.dart';
import 'calendar_mirror.dart';
import 'calendar_platform.dart';
import 'capture_fan.dart';
import 'capture_requests.dart';
import 'capture_screen.dart';
import 'context_platform.dart';
import 'drafts.dart';
import 'failure_dialog.dart';
import 'first_start_screen.dart';
import 'folder_platform.dart';
import 'frame_times.dart';
import 'help_notes.dart';
import 'home_pages.dart';
import 'host_screen.dart';
import 'l10n.dart';
import 'location.dart';
import 'lock_platform.dart';
import 'lock_screen.dart';
import 'look.dart';
import 'note_editor_screen.dart';
import 'nudge_screen.dart';
import 'occurrence_answers.dart';
import 'ocr_platform.dart';
import 'pairing_platform.dart';
import 'person_screen.dart';
import 'place_names.dart';
import 'photo_platform.dart';
import 'profile.dart';
import 'profile_platform.dart';
import 'question_mark_background.dart';
import 'restore_screen.dart';
import 'ring_screen.dart';
import 'session_view.dart';
import 'routine_editor_screen.dart';
import 'sent_item_screen.dart';
import 'setup_screen.dart';
import 'speech_model_screen.dart';
import 'speech_offer_screen.dart';
import 'speech_platform.dart';
import 'starter_set_screen.dart';
import 'sync_platform.dart';
import 'sync_runner.dart';
import 'sync_screen.dart';
import 'text_marks_view.dart';
import 'voice_platform.dart';

class OldBrainFixApp extends StatefulWidget {
  final Store store;

  /// What the app has to say; the app keeps its own when none is given.
  final Announcements? announcements;

  /// The root navigator, for what is announced before the app runs,
  /// such as the report of a run that died.
  final GlobalKey<NavigatorState>? navigatorKey;

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

  /// RustO for Photo Texts; none when not given.
  final OcrPlatform ocr;

  /// The calendar app to add to and the apps to share calendar files
  /// with; none when not given.
  final CalendarPlatform calendar;

  /// The Device's calendars to mirror Routines and Reminders into; none
  /// when not given.
  final MirrorPlatform mirror;

  /// How long the calendar mirror waits after a change for more to come.
  final Duration mirrorDelay;

  /// The Device's calendars whose appointments the Agenda shows; none
  /// when not given.
  final AppointmentSource appointments;

  /// Biometrics and the screen lock for the App Lock and the Area Locks;
  /// none when not given, and the PIN unlocks.
  final LockPlatform lock;

  /// Reads the pair code in a photo for Sync; typing it always works.
  final PairingPlatform pairing;

  /// Finds paired Devices on the network and syncs in the background;
  /// none when not given, and paired Devices are tried where they were.
  final SyncPlatform sync;

  /// How an automatic sync is run; tests hand in a fake.
  final PairedJoin autoJoin;

  /// Where the automatic sync's folder rounds and joins run: the app
  /// hands in an isolate of their own (#225); without one, on [store]
  /// with [folders] and [autoJoin].
  final SyncRunner? syncRunner;

  /// Where the Agenda's reads and the alarms' plan run: the app hands in
  /// an isolate of their own (#207); without one, on [store] at once.
  final AgendaReader agendaReader;

  /// Starts this Device's paired host for automatic sync; while none is
  /// given, this Device only joins.
  final StartPairedHost? pairedHost;

  /// How the first start joins a Device of one's own and hosts for one;
  /// tests hand in fakes.
  final Join join;
  final StartHosting hosting;

  /// The shared folder to sync through; none when not given.
  final FolderPlatform folders;

  /// Sharing and importing bundle files; none when not given.
  final BundlePlatform bundles;

  /// The phone's name for the own Profile and its picture picker; none
  /// when not given.
  final ProfilePlatform profile;

  /// Where a Backup is written before it goes into its folder; while
  /// none is given, no Backup is written on its own while the app shows.
  final String? workDir;

  /// The help Notes this version makes (#180) and what they are made
  /// from; none in tests, so no Area "Old Brain Fix" appears there unless
  /// a test hands one in.
  final HelpSetup? help;

  const OldBrainFixApp({
    super.key,
    required this.store,
    this.announcements,
    this.navigatorKey,
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
    this.appointments = const NoAppointments(),
    this.lock = const NoBiometrics(),
    this.pairing = const NoPairing(),
    this.sync = const NoSync(),
    this.autoJoin = quickPairedSync,
    this.syncRunner,
    this.agendaReader = const InlineAgendaReader(),
    this.pairedHost,
    this.join = lanSync,
    this.hosting = startPairingHost,
    this.folders = const NoFolders(),
    this.bundles = const NoBundles(),
    this.profile = const NoProfilePlatform(),
    this.workDir,
    this.help,
  });

  @override
  State<OldBrainFixApp> createState() => _OldBrainFixAppState();
}

class _OldBrainFixAppState extends State<OldBrainFixApp> {
  final _scroll = ParallaxScroll();
  late var _fatFingers = widget.store.fatFingers;
  late var _light = lookTheme(Brightness.light, fatFingers: _fatFingers);
  late var _dark = lookTheme(Brightness.dark, fatFingers: _fatFingers);
  late final _announcements = widget.announcements ?? Announcements();
  late final _drafts = widget.drafts ?? MemoryDrafts();
  late var _themeMode = themeModeOf(widget.store.themeChoice);
  late final _notes = Notes(widget.store);
  late final _places = Places(widget.store);
  final _trail = BrowserTrail();
  late final _frameTimes = FrameTimesSwitch(widget.store);
  late final AlarmSync _alarms;
  late final _mirror = CalendarMirror(
    store: widget.store,
    platform: widget.mirror,
    delay: widget.mirrorDelay,
  );
  late final _playback = Playback(
    voice: widget.voice,
    bytes: _notes.recordingBytes,
    speed: widget.store.playbackSpeed,
    onSpeed: (speed) => widget.store.playbackSpeed = speed,
  );
  late final _speechModel = SpeechModel(widget.speech);
  late final _lock = AppLock(store: widget.store, platform: widget.lock);
  final _routes = RouteStack();
  late final _autoSync = AutoSync(
    store: widget.store,
    platform: widget.sync,
    deviceName: widget.pairing.deviceName,
    join: widget.autoJoin,
    folders: widget.folders,
    workDir: widget.workDir,
    host: widget.pairedHost,
    runner: widget.syncRunner,
  );
  late final Transcriber _transcriber;
  late final PhotoReader _photoReader;
  late final _profile = OwnProfileModel(
    store: widget.store,
    platform: widget.profile,
  );

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
      reader: widget.agendaReader,
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
    // The own Profile from the phone's name, asking nothing; on the first
    // start only once it is past its first screen (ADR 0017), so a Device
    // being added takes the Profile of the user's other Devices.
    if (!_places.starterSetDue) unawaited(_profile.applyDefaults());
  }

  @override
  void dispose() {
    _alarms.dispose();
    _autoSync.dispose();
    _mirror.dispose();
    _playback.dispose();
    _transcriber.dispose();
    _speechModel.dispose();
    _photoReader.dispose();
    _lock.dispose();
    _profile.dispose();
    _pager.dispose();
    _scroll.dispose();
    _frameTimes.dispose();
    if (widget.announcements == null) _announcements.dispose();
    super.dispose();
  }

  /// The Agenda and the Areas list beside it (#258), kept here so an
  /// app link to the Agenda shows it from any page (#261).
  final _pager = HomePager();

  /// What the format bar's "Link to…" picks from (#261).
  Future<LinkTarget?> _pickLink(BuildContext context) =>
      pickLinkTarget(context, _notes, _places);

  /// Where a tapped app link goes (#261, ADR 0046).
  late final _links = AppLinks(
    store: widget.store,
    notes: _notes,
    places: _places,
    trail: _trail,
    alarms: _alarms,
    profile: _profile,
    onThemeChanged: _themeChanged,
    showAgenda: (context) {
      Navigator.of(context).popUntil((route) => route.isFirst);
      _pager.jumpToAgenda();
    },
  );

  /// A tag tapped on a page of its own, such as the editor, opens the
  /// search over every Note for it on top (ADR 0032).
  void _searchTag(BuildContext context, String query) => unawaited(
    Navigator.of(context).push(
      BrowserRoute<void>(
        builder: (_) => AreasScreen(
          notes: _notes,
          places: _places,
          trail: _trail,
          query: query,
        ),
      ),
    ),
  );

  @override
  Widget build(BuildContext context) => MaterialApp(
    navigatorKey: widget.navigatorKey,
    onGenerateTitle: (context) => AppLocalizations.of(context).appTitle,
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    theme: _light,
    darkTheme: _dark,
    themeMode: _themeMode,
    navigatorObservers: [_scroll, _routes],
    builder: (context, child) => FrameTimesScope(
      frameTimes: _frameTimes,
      child: FrameTimesOverlay(
        frameTimes: _frameTimes,
        child: FatFingersText(
          on: _fatFingers,
          child: QuestionMarkBackground(
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
                                child: AppointmentScope(
                                  source: widget.appointments,
                                  child: DraftsScope(
                                    drafts: _drafts,
                                    child: LockScope(
                                      lock: _lock,
                                      child: PairingScope(
                                        pairing: widget.pairing,
                                        child: AutoSyncScope(
                                          sync: _autoSync,
                                          child: FolderScope(
                                            folders: widget.folders,
                                            child: BundleScope(
                                              bundles: widget.bundles,
                                              child: PersonScope(
                                                store: widget.store,
                                                platform: widget.profile,
                                                onChangesSent: _alarms.replan,
                                                child: TagSearchScope(
                                                  onSearch: _searchTag,
                                                  child: AppLinkScope(
                                                    onOpen: _links.open,
                                                    onPick: _pickLink,
                                                    child: AlarmSyncScope(
                                                      alarms: _alarms,
                                                      child:
                                                          child ??
                                                          const SizedBox.shrink(),
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
    // Its own route, so a browser level coming does not build the
    // Agenda anew (BrowserHomeRoute). There are no named pages.
    onGenerateRoute: (_) => null,
    onGenerateInitialRoutes: (_) => [
      BrowserHomeRoute<void>(
        settings: const RouteSettings(name: Navigator.defaultRouteName),
        builder: (_) => _home(),
      ),
    ],
  );

  _Home _home() => _Home(
    store: widget.store,
    notes: _notes,
    places: _places,
    trail: _trail,
    alarms: _alarms,
    autoSync: _autoSync,
    mirror: _mirror,
    lock: _lock,
    routes: _routes,
    requests: widget.requests,
    bundles: widget.bundles,
    profile: _profile,
    speech: _speechModel,
    join: widget.join,
    hosting: widget.hosting,
    help: widget.help,
    onThemeChanged: _themeChanged,
    pager: _pager,
  );

  /// Shows the theme this Device chose in Settings at once, and its
  /// "Fat fingers" (#224).
  void _themeChanged() => setState(() {
    _themeMode = themeModeOf(widget.store.themeChoice);
    _fatFingers = widget.store.fatFingers;
    _light = lookTheme(Brightness.light, fatFingers: _fatFingers);
    _dark = lookTheme(Brightness.dark, fatFingers: _fatFingers);
  });
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

/// This Device's first start: its first screen, then the Starter Set
/// for someone new, or a restore, or a Device of one's own added and
/// synced; then the speech model offer where there is a speech layer,
/// then the alarm setup where there are alarms, the Agenda after them. Keeps the alarms in
/// step while the app runs, shows the full-screen alarm on top while
/// one rings, the Nudge screen when the Nudge's notification was
/// tapped, the Unsorted list when the Agenda widget's Unsorted
/// count was and a pick's entry form when its pick was, and imports the bundles other apps opened with this one
/// and opens the Profile card links tapped in them.
/// While the App Lock is locked, the lock screen stands in
/// front of the Agenda, which still opens Capture for shares and quick
/// entry points; what was open of the browser goes when the app locks
/// again, while Capture and the full-screen alarm stay, and the Nudge
/// screen, the Unsorted list and a pick's entry form wait for the
/// unlock.
class _Home extends StatefulWidget {
  final Store store;
  final Notes notes;
  final Places places;
  final BrowserTrail trail;
  final AlarmSync alarms;
  final AutoSync autoSync;
  final CalendarMirror mirror;
  final AppLock lock;
  final RouteStack routes;
  final CaptureRequests requests;
  final BundlePlatform bundles;
  final OwnProfileModel profile;
  final SpeechModel speech;
  final Join join;
  final StartHosting hosting;
  final HelpSetup? help;
  final VoidCallback onThemeChanged;
  final HomePager pager;

  const _Home({
    required this.store,
    required this.notes,
    required this.places,
    required this.trail,
    required this.alarms,
    required this.autoSync,
    required this.mirror,
    required this.lock,
    required this.routes,
    required this.requests,
    required this.bundles,
    required this.profile,
    required this.speech,
    required this.join,
    required this.hosting,
    required this.help,
    required this.onThemeChanged,
    required this.pager,
  });

  @override
  State<_Home> createState() => _HomeState();
}

class _HomeState extends State<_Home> {
  /// Not offered once Areas arrived from another Device, which planted
  /// its own (ADR 0008: the flag is this Device's).
  late bool _starterSetOffered = !widget.places.starterSetDue;

  /// Whether "New to Old Brain Fix" was picked on the first screen: the
  /// Starter Set shows, and Back goes to the first screen again.
  var _new = false;

  /// Whether a Backup may be restored: only into an empty log, and only
  /// where a file can be picked; worked out once.
  late final bool _restorable =
      widget.bundles.available && widget.store.currentSeq() == 0;

  /// The Backup Android's Auto Backup brought back with the app, found
  /// on the first start (ADR 0010): opened at once, asking for the
  /// passphrase or the recovery key.
  String? _found;

  /// Whether the speech model is offered now: right after the Starter
  /// Set, a restore or a Device added on the first start, and never again, as Settings
  /// offers it as well.
  var _speechOfferDue = false;
  late bool _alarmSetupDone =
      widget.store.alarmSetupDone || !widget.alarms.platform.available;
  late final AppLifecycleListener _lifecycle;

  /// The full-screen alarm on screen, and what it shows.
  Route<void>? _ringRoute;
  Ringing? _ringShown;

  /// Whether the app is locked, as shown.
  late bool _locked = widget.lock.locked;

  /// The Agenda and the Areas list beside it (#258), the app's.
  HomePager get _pager => widget.pager;

  /// The Nudge screen and the Unsorted list, asked for while locked, and
  /// Conflicts a sync brought meanwhile.
  var _nudgeWaiting = false;
  var _changesWaiting = false;
  var _unsortedWaiting = false;
  WidgetPick? _pickWaiting;
  OpenedOccurrence? _occurrenceWaiting;

  /// Whether the bundles of the Changes Nudge are going out now.
  var _sendingChanges = false;
  var _conflictsWaiting = false;

  late final StreamSubscription<void> _bundlesOpened;

  /// Sent Items and Profile card files other apps opened with this one
  /// while it was locked, and card files opened before the first start
  /// made the own Profile.
  final _openedWaiting = <String>[];

  @override
  void initState() {
    super.initState();
    _bundlesOpened = widget.bundles.opened.listen(
      (_) => unawaited(_importOpened()),
    );
    widget.profile.addListener(_profileMade);
    widget.alarms.addListener(_ringChanged);
    widget.alarms.addListener(_nudgeOpened);
    widget.alarms.addListener(_changesOpened);
    widget.alarms.addListener(_unsortedOpened);
    widget.alarms.addListener(_pickOpened);
    widget.alarms.addListener(_occurrenceOpened);
    widget.lock.addListener(_lockChanged);
    widget.speech.addListener(_speechChanged);
    widget.autoSync.addListener(_conflictsSynced);
    widget.autoSync.addListener(_updatesNeeded);
    widget.autoSync.syncing.addListener(_syncing);
    // Paired Devices sync while the app shows, from the first frame on:
    // what it brings is announced.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      widget.autoSync.shown();
      // A bundle, Sent Item or Profile card file that opened the app.
      unawaited(_importOpened());
      unawaited(_autoBackupStart());
      // Past the first start already: what this version's help Notes
      // still owe (#180), an update's What's new among them.
      if (_starterSetOffered) _helpDue(fresh: false);
    });
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
    SpeechScope.of(context)?.transcriber.setTexts(context.t);
  }

  /// Whether "Syncing" was said since the app came into view.
  var _syncingSaid = false;

  /// A sync of this Device started: a screen reader hears so once each
  /// time the app comes into view, not with every sync while it shows;
  /// the line under the bar shows every one (#225).
  void _syncing() {
    if (!mounted || _syncingSaid || !widget.autoSync.syncing.value) return;
    if (!MediaQuery.accessibleNavigationOf(context)) return;
    _syncingSaid = true;
    unawaited(
      SemanticsService.sendAnnouncement(
        View.of(context),
        context.t.syncSyncing,
        Directionality.of(context),
      ),
    );
  }

  void _hidden() {
    // A sync out of sight is not announced; one after coming back is.
    _syncingSaid = true;
    widget.lock.hidden();
    unawaited(widget.alarms.refresh());
    widget.autoSync.hidden();
  }

  /// Back from the background: the alarms as above, the calendar mirror
  /// in step with what other Devices synced meanwhile, and a sync with
  /// the paired Devices.
  void _resumed() {
    _syncingSaid = false;
    unawaited(widget.alarms.refresh());
    unawaited(widget.mirror.sync());
    widget.autoSync.shown();
  }

  /// An automatic sync, now or in the background, or the shared folder
  /// needs an update of the app here or on another Device, or a setting
  /// a newer version saved was kept (ADR 0034): announced at the top,
  /// once (ADR 0019), which Device behind the tap.
  void _updatesNeeded() {
    if (!mounted) return;
    final t = context.t;
    for (final (source, where) in widget.autoSync.takeUpdates()) {
      final settings = source == UpdateNotices.settingsSource;
      announceFailure(
        context,
        settings ? t.settingsUpdateNeeded : t.updateNeeded,
        title: t.updateNeededTitle,
        cause: updateNoticeText(t, widget.store, source, where),
        fix: settings ? t.settingsUpdateFix : t.updateFix,
        details: 'update needed: ${where.name}',
      );
    }
  }

  /// An automatic sync, now or in the background, brought Conflicts:
  /// announced at the top, a tap away from the list.
  void _conflictsSynced() {
    if (!mounted || widget.autoSync.takeConflicts() == 0) return;
    // Not past the App Lock: announced once it is unlocked.
    if (widget.lock.locked) {
      _conflictsWaiting = true;
      return;
    }
    _announceConflicts();
  }

  void _announceConflicts() {
    _conflictsWaiting = false;
    final store = widget.store;
    final count = visibleConflicts(
      store,
      widget.lock,
      store.conflicts(),
    ).length;
    if (count == 0) return;
    final navigator = Navigator.of(context);
    AnnouncementsScope.of(context)?.announce(
      context.t.hostConflicts(count),
      onTap: () => unawaited(
        navigator.push(
          MaterialPageRoute<void>(
            builder: (_) => ConflictsScreen(store: store),
          ),
        ),
      ),
    );
  }

  @override
  void dispose() {
    widget.alarms.removeListener(_ringChanged);
    widget.alarms.removeListener(_nudgeOpened);
    widget.alarms.removeListener(_changesOpened);
    widget.alarms.removeListener(_unsortedOpened);
    widget.alarms.removeListener(_pickOpened);
    widget.alarms.removeListener(_occurrenceOpened);
    widget.lock.removeListener(_lockChanged);
    widget.speech.removeListener(_speechChanged);
    widget.autoSync.removeListener(_conflictsSynced);
    widget.autoSync.removeListener(_updatesNeeded);
    widget.autoSync.syncing.removeListener(_syncing);
    unawaited(_bundlesOpened.cancel());
    widget.profile.removeListener(_profileMade);
    _lifecycle.dispose();
    super.dispose();
  }

  /// Imports the bundles other apps opened with this one, announcing
  /// what each brought; Conflicts as an automatic sync's, not past the
  /// App Lock. A Sent Item among them opens its preview, behind the App
  /// Lock once it is unlocked. A Profile card file (#144) opens the
  /// person's Profile, and a Pack (#164) its tree, which is browsing, so
  /// each waits for the unlock as
  /// well; nor before the first start made the own Profile, so nothing is
  /// kept before the user chose to restore or add a Device of theirs.
  Future<void> _importOpened() async {
    _openedWaiting.addAll(await widget.bundles.takeOpened());
    final paths = [..._openedWaiting];
    _openedWaiting.clear();
    var conflicts = 0;
    for (final path in paths) {
      if (!mounted) return;
      // A Pack names its sender as a card does (#164).
      final card = isProfileCardFile(path) || await isPackFile(path);
      if (!mounted) return;
      // A Sent Item, a card or a Pack shows what is in it: not past the
      // App Lock.
      if ((widget.lock.locked && (card || isSentItemFile(path))) ||
          (card && widget.profile.profile == null)) {
        _openedWaiting.add(path);
        continue;
      }
      conflicts += await openReceivedFile(context, widget.store, path);
    }
    if (!mounted || conflicts == 0) return;
    if (widget.lock.locked) {
      _conflictsWaiting = true;
      return;
    }
    _announceConflicts();
  }

  /// The first start made the own Profile: the card files that waited
  /// for it open.
  void _profileMade() {
    if (_openedWaiting.isEmpty || widget.profile.profile == null) return;
    unawaited(_importOpened());
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
    if (_changesWaiting) unawaited(_sendWaitingChanges());
    if (_conflictsWaiting) _announceConflicts();
    if (_openedWaiting.isNotEmpty) unawaited(_importOpened());
    if (_unsortedWaiting) {
      _unsortedWaiting = false;
      unawaited(openUnsorted(context, widget.notes, widget.places));
    }
    if (_pickWaiting case final pick?) {
      _pickWaiting = null;
      unawaited(_openPick(pick));
    }
    if (_occurrenceWaiting case final opened?) {
      _occurrenceWaiting = null;
      unawaited(_openOccurrence(opened));
    }
  }

  /// Removes every route over the Agenda up to the first that needs no
  /// unlocking: Capture, and what it opened, stays and the full-screen
  /// alarm too, the browser goes, the Areas list beside the Agenda as
  /// well. Unsaved text in the editor is kept in its draft.
  void _clearBrowsing() {
    _pager.jumpToAgenda();
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
  void _capture(CaptureWay way) => unawaited(
    captureNote(
      context,
      widget.notes,
      voice: way == CaptureWay.voice,
      photo: way == CaptureWay.photo,
      gallery: way == CaptureWay.gallery,
      paste: way == CaptureWay.paste,
      onChanged: () {},
    ),
  );

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

  /// Sends the waiting changes when the Changes Nudge's notification was
  /// tapped (#109); behind the App Lock they wait for the unlock.
  void _changesOpened() {
    if (!mounted || !widget.alarms.takeChangesOpened()) return;
    if (widget.lock.locked) {
      _changesWaiting = true;
      return;
    }
    unawaited(_sendWaitingChanges());
  }

  /// Offers each person the Changes Nudge covers their bundle, one share
  /// sheet after another; a sheet left without an app passes to the next
  /// person and keeps the Changes Nudge for that one.
  Future<void> _sendWaitingChanges() async {
    _changesWaiting = false;
    if (_sendingChanges) return;
    _sendingChanges = true;
    try {
      for (final person in widget.alarms.changesWaiting) {
        if (!mounted) return;
        await sendChanges(
          store: widget.store,
          bundles: widget.bundles,
          t: context.t,
          announcements: AnnouncementsScope.of(context),
          navigator: Navigator.of(context, rootNavigator: true),
          person: person,
          onSent: widget.alarms.replan,
        );
      }
    } finally {
      _sendingChanges = false;
    }
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

  /// Opens a pick's entry form when it was tapped on the Agenda widget
  /// (#209); behind the App Lock it waits for the unlock.
  void _pickOpened() {
    if (!mounted) return;
    final pick = widget.alarms.takePickOpened();
    if (pick == null) return;
    if (widget.lock.locked) {
      _pickWaiting = pick;
      return;
    }
    unawaited(_openPick(pick));
  }

  /// Opens the entry form of today's [pick], as its box on the Agenda
  /// does: its locked Area first. A pick no longer there, or one this
  /// Device may not record for, opens nothing, the Agenda being open.
  Future<void> _openPick(WidgetPick pick) async {
    final store = widget.store;
    final today = Sessions(store)
        .today(deviceWallClock)
        .where(
          (o) =>
              o.session.id == pick.sessionId &&
              o.occurrence.key == pick.occurrenceKey,
        )
        .firstOrNull;
    final picked = today?.picks
        .where((p) => p.noteId == pick.noteId)
        .firstOrNull;
    if (today == null || picked == null) return;
    if (!opensPick(store, pick.sessionId, picked)) return;
    final lock = LockScope.of(context);
    final areaId = switch (picked.note) {
      final note? => AreaLocks(store).lockedAreaOf(note),
      null => null,
    };
    if (areaId != null) {
      final area = widget.places.areaById(areaId);
      final open = await openLockedArea(
        context,
        lock,
        areaId,
        area == null ? '' : areaName(context.t, area),
      );
      if (!open || !mounted) return;
    }
    await openPick(context, widget.notes, today, picked);
    lock?.closeAreas();
  }

  /// Opens the Note of an Occurrence whose notification was tapped
  /// (#270); behind the App Lock it waits for the unlock.
  void _occurrenceOpened() {
    if (!mounted) return;
    final opened = widget.alarms.takeOccurrenceOpened();
    if (opened == null) return;
    if (widget.lock.locked) {
      _occurrenceWaiting = opened;
      return;
    }
    unawaited(_openOccurrence(opened));
  }

  /// Opens the editor of the Note of [opened]'s Routine, as a tap on its
  /// row on the Agenda does: its locked Area first. A Routine or Note no
  /// longer there, or in the trash, opens nothing, the Agenda being open.
  Future<void> _openOccurrence(OpenedOccurrence opened) async {
    final store = widget.store;
    final routine = Routines(store).byId(opened.routineId);
    if (routine == null || routine.isTrashed) return;
    final note = widget.notes.byId(routine.noteId);
    if (note == null || note.isTrashed) return;
    final lock = LockScope.of(context);
    final areaId = AreaLocks(store).lockedAreaOf(note);
    if (areaId != null) {
      final area = widget.places.areaById(areaId);
      final open = await openLockedArea(
        context,
        lock,
        areaId,
        area == null ? '' : areaName(context.t, area),
      );
      if (!open || !mounted) return;
    }
    await openNote(context, widget.notes, widget.places, note);
    lock?.closeAreas();
  }

  /// Plants what the user kept of the Starter Set and keeps Simple or
  /// Advanced (#197) for all of their own Devices; it is not offered
  /// again on this Device.
  void _plant(Set<String> picked, bool advanced) {
    widget.store.advanced = advanced;
    widget.places.plantStarterSet(picked);
    widget.store.starterSetOffered = true;
    unawaited(widget.profile.applyDefaults());
    _helpDue(fresh: true);
    setState(() {
      _starterSetOffered = true;
      _speechOfferDue = _speechWanted;
    });
  }

  /// Whether the speech model is worth offering: there is a speech layer,
  /// Whisper may run here, and no model is in place.
  bool get _speechWanted =>
      widget.speech.platform.available &&
      widget.speech.status != ModelStatus.ready &&
      widget.speech.status != ModelStatus.unsupported;

  /// The offer goes once the check finds a model in place or that Whisper
  /// does not run here.
  void _speechChanged() {
    if (!mounted || !_speechOfferDue || _speechWanted) return;
    setState(() => _speechOfferDue = false);
  }

  /// Starts the speech model's download in the background, announced
  /// at the top should it fail, and goes on with the setup at once.
  void _downloadSpeech() {
    unawaited(fetchSpeechModel(context, widget.speech.download));
    setState(() => _speechOfferDue = false);
  }

  /// The platform's backup is told the opt-in this Device keeps, and on
  /// the first start a Backup that came back with the app is offered at
  /// once.
  Future<void> _autoBackupStart() async {
    final folders = widget.autoSync.folders;
    await folders.setAutoBackup(widget.store.autoBackupOn);
    if (_starterSetOffered || widget.store.currentSeq() != 0) return;
    final dir = await folders.autoBackupDir();
    if (!mounted || dir == null) return;
    final found = newestBackupIn(dir);
    if (found == null) return;
    setState(() => _found = found);
    await _restore();
  }

  /// Restores a Backup into the empty log; the Starter Set is not
  /// offered after, the Areas came with it. The one that came back with
  /// the app is removed once restored.
  Future<void> _restore() async {
    final found = _found;
    final restored = await Navigator.of(context).push<RestoreResult>(
      MaterialPageRoute(
        builder: (_) => RestoreScreen(
          store: widget.store,
          found: found != null && File(found).existsSync() ? found : null,
        ),
      ),
    );
    if (!mounted || restored == null) return;
    if (found != null) {
      try {
        File(found).deleteSync();
      } on FileSystemException {
        // Gone already.
      }
      _found = null;
    }
    widget.store.starterSetOffered = true;
    // The playback speed came with it.
    VoiceScope.playbackOf(context)?.speed.value = widget.store.playbackSpeed;
    _setUpAfterwards();
  }

  /// Adds this Device to the user's others: paired either way round and
  /// synced, it holds their Profile, Areas and the rest, and goes on with
  /// this Device's own setup, without the Starter Set. Back, a failure or
  /// a Device turned away leave the first screen as it was.
  Future<void> _addDevice() async {
    final synced = await Navigator.of(context).push<bool>(
      MaterialPageRoute(
        builder: (_) => SyncScreen(
          store: widget.store,
          join: widget.join,
          host: widget.hosting,
          firstStart: true,
        ),
      ),
    );
    if (!mounted || synced != true) return;
    widget.store.starterSetOffered = true;
    _setUpAfterwards();
  }

  /// After a restore or a Device added: what came with it is in place;
  /// the Profile is there already, the defaults only fill a gap.
  void _setUpAfterwards() {
    setState(() {
      _starterSetOffered = true;
      _speechOfferDue = _speechWanted;
    });
    unawaited(widget.profile.applyDefaults());
    _helpDue(fresh: true);
    unawaited(widget.alarms.refresh());
    widget.autoSync.devicesChanged();
  }

  /// Makes the help Notes due (#180) once the own Profile exists, which
  /// their ids are made from; [fresh] right after the first start, which
  /// makes no What's new. Nothing without [_Home.help].
  void _helpDue({required bool fresh}) {
    final help = widget.help;
    if (help == null) return;
    final t = context.t;
    unawaited(() async {
      await widget.profile.applyDefaults();
      if (!mounted) return;
      await makeHelpNotes(
        widget.store,
        help.source,
        t,
        version: help.version,
        fresh: fresh,
      );
    }());
  }

  /// The alarm setup was walked through; it is not offered again here.
  void _setUp() {
    widget.store.alarmSetupDone = true;
    setState(() => _alarmSetupDone = true);
  }

  @override
  Widget build(BuildContext context) {
    if (!_starterSetOffered && _new) {
      return PopScope(
        canPop: false,
        onPopInvokedWithResult: (didPop, _) {
          if (!didPop) setState(() => _new = false);
        },
        child: StarterSetScreen(onDone: _plant),
      );
    }
    if (!_starterSetOffered) {
      return FirstStartScreen(
        onNew: () => setState(() => _new = true),
        onAddDevice: () => unawaited(_addDevice()),
        onRestore: _restorable || _found != null
            ? () => unawaited(_restore())
            : null,
      );
    }
    if (_speechOfferDue) {
      return SpeechOfferScreen(
        model: widget.speech,
        onDownload: _downloadSpeech,
        onSkip: () => setState(() => _speechOfferDue = false),
      );
    }
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
            child: HomePages(
              pager: _pager,
              agenda: AgendaScreen(
                notes: widget.notes,
                places: widget.places,
                trail: widget.trail,
                alarms: widget.alarms,
                requests: widget.requests,
                profile: widget.profile,
                onThemeChanged: widget.onThemeChanged,
                pager: _pager,
              ),
              areas: AreasScreen(
                notes: widget.notes,
                places: widget.places,
                trail: widget.trail,
                onBack: _pager.showAgenda,
              ),
            ),
          ),
        ),
        if (_locked) LockScreen(lock: widget.lock, onCapture: _capture),
      ],
    );
  }
}
