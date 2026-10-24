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
  String get noNotesYet => 'Nothing captured yet.';

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

  @override
  String get noteTitle => 'Note';

  @override
  String get importance => 'Importance';

  @override
  String get importanceUnimportant => 'Unimportant';

  @override
  String get importanceNormal => 'Normal';

  @override
  String get importanceImportant => 'Important';

  @override
  String get capturedHeading => 'Captured';

  @override
  String get amend => 'Add to Note';

  @override
  String get amendHint => 'What\'s new?';

  @override
  String get add => 'Add';

  @override
  String get moveToTrash => 'Move to trash';

  @override
  String get trashTitle => 'Trash';

  @override
  String get trashEmpty => 'The trash is empty.';

  @override
  String get restore => 'Restore';

  @override
  String goneIn(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Deleted for good in $count days',
      one: 'Deleted for good in 1 day',
      zero: 'Deleted for good within a day',
    );
    return '$_temp0';
  }

  @override
  String get trashThem => 'Move to trash';

  @override
  String get orderByImportance => 'Order by Importance';

  @override
  String get orderByTime => 'Order by time';

  @override
  String get hideUnimportant => 'Hide unimportant Notes';

  @override
  String get showUnimportant => 'Show unimportant Notes';

  @override
  String get addHere => 'Add here';

  @override
  String get noteHere => 'Note here';

  @override
  String get topicChoice => 'Topic';

  @override
  String get mergeInto => 'Merge into…';

  @override
  String get noOtherNotes => 'There is no other Note to merge into.';

  @override
  String get locationExplainTitle => 'Remember where you were?';

  @override
  String get locationExplainBody =>
      'Old Brain Fix can note where you are when you capture a Note, so “nees mor sauce” still makes sense days later. Name your own places, like Home or Work, and a Note says “Home” instead of coordinates. Your location stays on this phone and is never sent anywhere.';

  @override
  String get notNow => 'Not now';

  @override
  String get allowLocation => 'Allow';

  @override
  String get locationNotAllowed =>
      'Old Brain Fix may not use your location, so it cannot tell where you are. You can allow it in the system settings.';

  @override
  String get locationUnknown =>
      'Could not find out where you are. Is location turned on?';

  @override
  String get namedPlacesTitle => 'Named Places';

  @override
  String get noNamedPlacesYet =>
      'No Named Places yet. Go to a spot like Home or Work and tap + to name it.';

  @override
  String get newNamedPlace => 'Name this place';

  @override
  String get editNamedPlace => 'Rename or resize';

  @override
  String get namedPlaceHint => 'Name, like Home';

  @override
  String get radiusHeading => 'Counts within';

  @override
  String radiusMetres(int metres) {
    return '$metres m';
  }

  @override
  String deleteNamedPlaceQuestion(String name) {
    return 'Delete the Named Place “$name”?';
  }

  @override
  String get deleteNamedPlaceDetail =>
      'Notes captured there show their coordinates again.';

  @override
  String get agendaToday => 'Today';

  @override
  String get agendaTomorrow => 'Tomorrow';

  @override
  String get nothingToday => 'Nothing due today. Tap + to capture a thought.';

  @override
  String get nothingTomorrow => 'Nothing due tomorrow.';

  @override
  String get justCaptured => 'Just captured';

  @override
  String get allNotesTitle => 'All Notes';

  @override
  String get newRoutineTooltip => 'New Routine or Reminder';

  @override
  String get routineOrReminder => 'Routine or Reminder';

  @override
  String get routine => 'Routine';

  @override
  String get reminder => 'Reminder';

  @override
  String get newRoutine => 'New Routine';

  @override
  String get newReminder => 'New Reminder';

  @override
  String get routineWhatHint => 'What is to be done?';

  @override
  String get fromNote => 'From the Note';

  @override
  String get noteGone => 'The Note is gone';

  @override
  String get scheduleSimple => 'Simple';

  @override
  String get scheduleAdvanced => 'Advanced';

  @override
  String get repeatOnce => 'Once';

  @override
  String get repeatEvery => 'Every …';

  @override
  String get repeatWeekdays => 'Weekdays';

  @override
  String get everyLabel => 'Every';

  @override
  String get fewer => 'Fewer';

  @override
  String get more => 'More';

  @override
  String unitDays(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'days',
      one: 'day',
    );
    return '$_temp0';
  }

  @override
  String unitWeeks(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'weeks',
      one: 'week',
    );
    return '$_temp0';
  }

  @override
  String unitMonths(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'months',
      one: 'month',
    );
    return '$_temp0';
  }

  @override
  String unitYears(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'years',
      one: 'year',
    );
    return '$_temp0';
  }

  @override
  String get startsOn => 'Starts';

  @override
  String get onDay => 'On';

  @override
  String get timesHeading => 'Times of day';

  @override
  String get allDay => 'All day';

  @override
  String get addTime => 'Add a time';

  @override
  String removeTime(String time) {
    return 'Remove $time';
  }

  @override
  String get rruleLabel => 'Repeat rule (RRULE)';

  @override
  String get rruleHelper =>
      'Leave empty for once. Example: FREQ=MONTHLY;BYDAY=1MO';

  @override
  String rruleInvalid(String detail) {
    return 'Not a rule this app understands: $detail';
  }

  @override
  String get nextOccurrences => 'Next';

  @override
  String get neverDue => 'This never comes up.';

  @override
  String get deleteRoutineQuestion => 'Delete this Routine? Its Note stays.';

  @override
  String get deleteReminderQuestion => 'Delete this Reminder? Its Note stays.';

  @override
  String get routinesHeading => 'Routines and Reminders';

  @override
  String everyNDays(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Every $count days',
      one: 'Every day',
    );
    return '$_temp0';
  }

  @override
  String everyNWeeks(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Every $count weeks',
      one: 'Every week',
    );
    return '$_temp0';
  }

  @override
  String everyNMonths(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Every $count months',
      one: 'Every month',
    );
    return '$_temp0';
  }

  @override
  String everyNYears(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Every $count years',
      one: 'Every year',
    );
    return '$_temp0';
  }

  @override
  String onWeekdays(String days) {
    return 'Every $days';
  }

  @override
  String onceOn(String date) {
    return 'Once, $date';
  }

  @override
  String ruleSummary(String rule) {
    return 'Rule: $rule';
  }

  @override
  String get markDone => 'Done';

  @override
  String get notFiled => 'Not filed';
}
