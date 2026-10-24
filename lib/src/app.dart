import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'look.dart';
import 'notes_screen.dart';
import 'question_mark_background.dart';

class OldBrainFixApp extends StatefulWidget {
  final Store store;

  /// What the app has to say; the app keeps its own when none is given.
  final Announcements? announcements;

  const OldBrainFixApp({super.key, required this.store, this.announcements});

  @override
  State<OldBrainFixApp> createState() => _OldBrainFixAppState();
}

class _OldBrainFixAppState extends State<OldBrainFixApp> {
  final _scroll = ParallaxScroll();
  final _light = lookTheme(Brightness.light);
  final _dark = lookTheme(Brightness.dark);
  late final _announcements = widget.announcements ?? Announcements();
  late final _themeMode = themeModeOf(widget.store.themeChoice);
  late final _notes = Notes(widget.store);

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
        child: child ?? const SizedBox.shrink(),
      ),
    ),
    home: NotesScreen(notes: _notes),
  );
}
