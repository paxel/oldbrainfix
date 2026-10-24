// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'Old Brain Fix';

  @override
  String get noNotesYet => 'Nothing captured yet. Tap + to capture a thought.';

  @override
  String get captureTooltip => 'Capture a Note';

  @override
  String get captureHint => 'What\'s on your mind?';

  @override
  String get save => 'Save';

  @override
  String get dismissAnnouncement => 'Dismiss';

  @override
  String get browseTooltip => 'Search and browse';

  @override
  String get areasTitle => 'Areas';

  @override
  String get noAreasYet => 'No Areas yet. Tap + to make one.';

  @override
  String get noTopicsYet =>
      'No Topics yet. Notes can sit in the Area itself, or tap + to make a Topic.';

  @override
  String get newArea => 'New Area';

  @override
  String get newTopic => 'New Topic';

  @override
  String get nameHint => 'Name';

  @override
  String get cancel => 'Cancel';

  @override
  String get create => 'Create';

  @override
  String get rename => 'Rename';

  @override
  String get delete => 'Delete';

  @override
  String get areaLook => 'Colour and icon';

  @override
  String get colourHeading => 'Colour';

  @override
  String get iconHeading => 'Icon';

  @override
  String get noColour => 'No colour';

  @override
  String get noIcon => 'No icon';

  @override
  String deleteAreaQuestion(String name) {
    return 'Delete the Area “$name”?';
  }

  @override
  String deleteTopicQuestion(String name) {
    return 'Delete the Topic “$name”?';
  }

  @override
  String get topicsGoToo => 'Its Topics are deleted with it.';

  @override
  String notesInside(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Notes inside.',
      one: '1 Note inside.',
    );
    return '$_temp0';
  }

  @override
  String get whereNotesGo => 'Where should they go?';

  @override
  String get moveToUnsorted => 'Move to Unsorted';

  @override
  String moveToArea(String area) {
    return 'Move to “$area”';
  }

  @override
  String get unsorted => 'Unsorted';

  @override
  String get fileInto => 'File into…';

  @override
  String get starterSetTitle => 'Pick your Areas';

  @override
  String get starterSetIntro =>
      'Here is a start. Untick what you don\'t need — you can change everything later.';

  @override
  String get starterSetDone => 'Start';

  @override
  String get starterHousehold => 'Household';

  @override
  String get starterKitchen => 'Kitchen';

  @override
  String get starterGarden => 'Garden';

  @override
  String get starterCleaning => 'Cleaning';

  @override
  String get starterWork => 'Work';

  @override
  String get starterMeetings => 'Meetings';

  @override
  String get starterToSortOut => 'To sort out';

  @override
  String get starterCar => 'Car';

  @override
  String get starterService => 'Service';

  @override
  String get starterRepairs => 'Repairs';

  @override
  String get starterHealth => 'Health';

  @override
  String get starterDoctor => 'Doctor';

  @override
  String get starterMedication => 'Medication';

  @override
  String get searchAllHint => 'Search all Notes';

  @override
  String get searchAreaHint => 'Search this Area';

  @override
  String get searchTopicHint => 'Search this Topic';

  @override
  String get clearSearch => 'Clear search';

  @override
  String get nothingFound => 'No Note matches.';

  @override
  String get noNotesHere => 'No Notes here yet. Tap + to add one.';
}
