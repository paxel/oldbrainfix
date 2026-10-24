// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for German (`de`).
class AppLocalizationsDe extends AppLocalizations {
  AppLocalizationsDe([String locale = 'de']) : super(locale);

  @override
  String get appTitle => 'Old Brain Fix';

  @override
  String get noNotesYet =>
      'Noch nichts erfasst. Tippe auf +, um einen Gedanken festzuhalten.';

  @override
  String get captureTooltip => 'Notiz erfassen';

  @override
  String get captureHint => 'Was geht dir durch den Kopf?';

  @override
  String get save => 'Speichern';

  @override
  String get dismissAnnouncement => 'Ausblenden';
}
