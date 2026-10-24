import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'notes_screen.dart';

class OldBrainFixApp extends StatelessWidget {
  final Store store;

  const OldBrainFixApp({super.key, required this.store});

  @override
  Widget build(BuildContext context) => MaterialApp(
    onGenerateTitle: (context) => AppLocalizations.of(context).appTitle,
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    theme: ThemeData(colorSchemeSeed: Colors.teal),
    home: NotesScreen(notes: Notes(store)),
  );
}
