import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'agenda_screen.dart';
import 'announcements.dart';
import 'browser.dart';
import 'drafts.dart';
import 'location.dart';
import 'look.dart';
import 'question_mark_background.dart';
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

  const OldBrainFixApp({
    super.key,
    required this.store,
    this.announcements,
    this.drafts,
    this.location = const NoLocation(),
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

  @override
  void initState() {
    super.initState();
    // What has been in the trash for 30 days goes for good.
    _notes.purgeTrash();
  }

  @override
  void dispose() {
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
    ),
  );
}

/// The Starter Set on this Device's first start, the Agenda after it.
class _Home extends StatefulWidget {
  final Store store;
  final Notes notes;
  final Places places;
  final BrowserTrail trail;

  const _Home({
    required this.store,
    required this.notes,
    required this.places,
    required this.trail,
  });

  @override
  State<_Home> createState() => _HomeState();
}

class _HomeState extends State<_Home> {
  late bool _starterSetOffered = widget.store.starterSetOffered;

  /// Plants what the user kept of the Starter Set; it is not offered
  /// again on this Device.
  void _plant(Set<String> picked) {
    widget.places.plantStarterSet(picked);
    widget.store.starterSetOffered = true;
    setState(() => _starterSetOffered = true);
  }

  @override
  Widget build(BuildContext context) => _starterSetOffered
      ? AgendaScreen(
          notes: widget.notes,
          places: widget.places,
          trail: widget.trail,
        )
      : StarterSetScreen(onDone: _plant);
}
