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

  @override
  String get browseTooltip => 'Suchen und stöbern';

  @override
  String get areasTitle => 'Bereiche';

  @override
  String get noAreasYet =>
      'Noch keine Bereiche. Tippe auf +, um einen anzulegen.';

  @override
  String get noTopicsYet =>
      'Noch keine Themen. Notizen können direkt im Bereich liegen, oder tippe auf +, um ein Thema anzulegen.';

  @override
  String get newArea => 'Neuer Bereich';

  @override
  String get newTopic => 'Neues Thema';

  @override
  String get nameHint => 'Name';

  @override
  String get cancel => 'Abbrechen';

  @override
  String get create => 'Anlegen';

  @override
  String get rename => 'Umbenennen';

  @override
  String get delete => 'Löschen';

  @override
  String get areaLook => 'Farbe und Symbol';

  @override
  String get colourHeading => 'Farbe';

  @override
  String get iconHeading => 'Symbol';

  @override
  String get noColour => 'Keine Farbe';

  @override
  String get noIcon => 'Kein Symbol';

  @override
  String deleteAreaQuestion(String name) {
    return 'Den Bereich „$name“ löschen?';
  }

  @override
  String deleteTopicQuestion(String name) {
    return 'Das Thema „$name“ löschen?';
  }

  @override
  String get topicsGoToo => 'Seine Themen werden mitgelöscht.';

  @override
  String notesInside(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Notizen darin.',
      one: '1 Notiz darin.',
    );
    return '$_temp0';
  }

  @override
  String get whereNotesGo => 'Wohin sollen sie verschoben werden?';

  @override
  String get moveToUnsorted => 'Nach Unsortiert verschieben';

  @override
  String moveToArea(String area) {
    return 'Nach „$area“ verschieben';
  }

  @override
  String get unsorted => 'Unsortiert';

  @override
  String get fileInto => 'Ablegen in …';

  @override
  String get starterSetTitle => 'Wähle deine Bereiche';

  @override
  String get starterSetIntro =>
      'Hier ist ein Anfang. Nimm die Haken weg bei allem, was du nicht brauchst – du kannst später alles ändern.';

  @override
  String get starterSetDone => 'Los geht\'s';

  @override
  String get starterHousehold => 'Haushalt';

  @override
  String get starterKitchen => 'Küche';

  @override
  String get starterGarden => 'Garten';

  @override
  String get starterCleaning => 'Putzen';

  @override
  String get starterWork => 'Arbeit';

  @override
  String get starterMeetings => 'Besprechungen';

  @override
  String get starterToSortOut => 'Zu klären';

  @override
  String get starterCar => 'Auto';

  @override
  String get starterService => 'Wartung';

  @override
  String get starterRepairs => 'Reparaturen';

  @override
  String get starterHealth => 'Gesundheit';

  @override
  String get starterDoctor => 'Arzt';

  @override
  String get starterMedication => 'Medikamente';

  @override
  String get searchAllHint => 'Alle Notizen durchsuchen';

  @override
  String get searchAreaHint => 'Diesen Bereich durchsuchen';

  @override
  String get searchTopicHint => 'Dieses Thema durchsuchen';

  @override
  String get clearSearch => 'Suche leeren';

  @override
  String get nothingFound => 'Keine Notiz passt.';

  @override
  String get noNotesHere =>
      'Hier gibt es noch keine Notizen. Tippe auf +, um eine hinzuzufügen.';
}
