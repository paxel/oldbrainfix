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
  String get capturePhotoHint => 'A few words about it (optional)';

  @override
  String get photo => 'Photo';

  @override
  String get voice => 'Voice';

  @override
  String get gallery => 'Gallery';

  @override
  String get paste => 'Paste';

  @override
  String get pasteEmpty =>
      'Nothing to paste: the clipboard holds no text and no picture.';

  @override
  String get text => 'Text';

  @override
  String get captureFanClose => 'Close';

  @override
  String get photoMissing => 'Photo not on this device';

  @override
  String morePhotos(int count) {
    return '+$count';
  }

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
  String get firstStartTitle => 'Welcome to Old Brain Fix';

  @override
  String get firstStartIntro => 'How does Old Brain Fix start on this Device?';

  @override
  String get firstStartNew => 'New to Old Brain Fix';

  @override
  String get firstStartNewDetail => 'Start with a set of Areas to pick from.';

  @override
  String get firstStartDevice => 'Add a Device of mine';

  @override
  String get firstStartDeviceDetail =>
      'Old Brain Fix runs on another Device of yours. Pair with it, and everything comes over.';

  @override
  String get hostContinue => 'Continue';

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
      'Old Brain Fix can note where you are when you capture a Note, so “nees mor sauce” still makes sense days later. Name your own places, like Home or Work, and a Note says “Home” instead of coordinates. The place goes only where its Note goes: to your own Devices, and to the people of a Shared Area you file the Note in. Never to anyone else.';

  @override
  String get notNow => 'Not now';

  @override
  String get allowLocation => 'Allow';

  @override
  String get locationNotAllowed =>
      'Old Brain Fix may not use your location, so it cannot tell where you are.';

  @override
  String get locationUnknown => 'The phone could not find out where you are.';

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
  String agendaDay(DateTime day) {
    final intl.DateFormat dayDateFormat = intl.DateFormat(
      'EEEE, d MMM',
      localeName,
    );
    final String dayString = dayDateFormat.format(day);

    return '$dayString';
  }

  @override
  String get nothingPlanned => 'Nothing planned';

  @override
  String get justCaptured => 'Just captured';

  @override
  String get allNotesTitle => 'All Notes';

  @override
  String get newRoutineTooltip => 'New Routine or Reminder';

  @override
  String get routineOrReminder => 'Routine, Reminder, Frog or Session';

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
  String get repeatMonthDay => 'Monthly on a day';

  @override
  String get repeatMonthWeekday => 'Monthly on a weekday';

  @override
  String get dayOfMonthLabel => 'Day of the month';

  @override
  String get monthDaySkipped => 'Months without this day are skipped.';

  @override
  String get whichInMonthLabel => 'Which one in the month';

  @override
  String nthOption(String which) {
    String _temp0 = intl.Intl.selectLogic(which, {
      '1': 'First',
      '2': 'Second',
      '3': 'Third',
      '4': 'Fourth',
      'other': 'Last',
    });
    return '$_temp0';
  }

  @override
  String get weekdayLabel => 'Weekday';

  @override
  String get endsLabel => 'Ends';

  @override
  String get endNever => 'Never';

  @override
  String get endOnDay => 'On a day';

  @override
  String get endAfterCount => 'After a number of times';

  @override
  String get endLastDay => 'Last day';

  @override
  String get endCountLabel => 'How many times';

  @override
  String get endCountExplain => 'Each time of day counts separately.';

  @override
  String unitTimes(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'times',
      one: 'time',
    );
    return '$_temp0';
  }

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
  String get deleteRoutineQuestion =>
      'Delete this Routine? It stays in the trash for 30 days. Its Note stays.';

  @override
  String get deleteReminderQuestion =>
      'Delete this Reminder? It stays in the trash for 30 days. Its Note stays.';

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
  String everyMonthOnDay(int count, int day) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Every $count months on day $day',
      one: 'Every month on day $day',
    );
    return '$_temp0';
  }

  @override
  String everyMonthOnNth(int count, String nth, String weekday) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Every $count months on the $nth $weekday',
      one: 'Every month on the $nth $weekday',
    );
    return '$_temp0';
  }

  @override
  String nthWord(String which) {
    String _temp0 = intl.Intl.selectLogic(which, {
      '1': 'first',
      '2': 'second',
      '3': 'third',
      '4': 'fourth',
      'other': 'last',
    });
    return '$_temp0';
  }

  @override
  String repeatUntil(String repeat, String date) {
    return '$repeat, until $date';
  }

  @override
  String repeatCount(int count, String repeat) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$repeat, $count times',
      one: '$repeat, once',
    );
    return '$_temp0';
  }

  @override
  String get markDone => 'Done';

  @override
  String get notFiled => 'Not filed';

  @override
  String get alarmChannel => 'Routines and Reminders';

  @override
  String get snooze => 'Snooze';

  @override
  String get snoozeQuestion => 'Remind me again in …';

  @override
  String get snooze10 => '10 min';

  @override
  String get snooze30 => '30 min';

  @override
  String get snooze60 => '1 h';

  @override
  String get alarmsBanner =>
      'Alarms may come late or not at all: a permission is missing. Tap to fix it.';

  @override
  String get setupTitle => 'Set up alarms';

  @override
  String setupStep(int step, int count) {
    return 'Step $step of $count';
  }

  @override
  String get setupAllow => 'Allow';

  @override
  String get setupGranted => 'Allowed';

  @override
  String get setupNext => 'Next';

  @override
  String get setupDone => 'Done';

  @override
  String get permNotifications => 'Notifications';

  @override
  String get permNotificationsWhy =>
      'Routines and Reminders show up as notifications. Tick them off as Done or snooze them right there, even on the lock screen.';

  @override
  String get permExactAlarms => 'Alarms on the minute';

  @override
  String get permExactAlarmsWhy =>
      'Without this, Android may bring an alarm up to a quarter of an hour late. Turn it on for Old Brain Fix on the page that opens.';

  @override
  String get permFullScreen => 'Full-screen alarms';

  @override
  String get permFullScreenWhy =>
      'Lets a Routine you keep ignoring turn into a full-screen alarm you can\'t miss, if you switch that on for it. Turn it on for Old Brain Fix on the page that opens.';

  @override
  String get catchUpTitle => 'Missed while the phone was off';

  @override
  String get catchUpOne => 'One alarm was Missed. Open the app to see it.';

  @override
  String catchUpMany(String count) {
    return '$count alarms were Missed. Open the app to see them.';
  }

  @override
  String get missedTitle => 'Missed';

  @override
  String missedTimes(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count times',
      one: 'once',
    );
    return '$_temp0';
  }

  @override
  String missedMore(int count) {
    return 'and $count more';
  }

  @override
  String get missedSeen => 'Seen';

  @override
  String get renagHeading => 'Re-nag';

  @override
  String renagSummary(int minutes) {
    return 'Rings again every $minutes min until Done';
  }

  @override
  String get renagExplain =>
      'An ignored alarm rings again at this interval until it is Done or snoozed, or until the next one is due; then it counts as Missed. A Reminder stops at midnight.';

  @override
  String get renagThis => 'This one';

  @override
  String get renagDevice => 'Default on this phone';

  @override
  String renagDeviceDefault(int minutes) {
    return 'Default ($minutes min)';
  }

  @override
  String renagMinutes(int minutes) {
    return '$minutes min';
  }

  @override
  String get ringChannel => 'Full-screen alarms';

  @override
  String get escalatedChannel => 'Escalated alarms';

  @override
  String get escalationHeading => 'Escalation';

  @override
  String get escalationOff => 'Off: it only Re-nags';

  @override
  String escalationSummary(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Full-screen alarm after $count ignored Re-nags',
      one: 'Full-screen alarm after 1 ignored Re-nag',
    );
    return '$_temp0';
  }

  @override
  String get escalationExplain =>
      'When this one is ignored again and again, it turns into a full-screen alarm with sound, even over the lock screen. It rings for up to 5 minutes, then Re-nags again, until it is Done or snoozed.';

  @override
  String get escalationSwitch => 'Escalate to a full-screen alarm';

  @override
  String get escalationAfter => 'After ignored Re-nags';

  @override
  String get nudgeHeading => 'Remind me to sort it in';

  @override
  String unsortedBanner(int count) {
    return '$count Unsorted';
  }

  @override
  String get noUnsorted => 'Nothing is Unsorted. Well done!';

  @override
  String get nudgeScreenTitle => 'Waiting to be sorted';

  @override
  String nudgeMore(int count) {
    return '+$count';
  }

  @override
  String get nudgeMoreTooltip => 'All Unsorted Notes';

  @override
  String get nudgeChannel => 'Nudges';

  @override
  String nudgeTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Notes are Unsorted',
      one: '1 Note is Unsorted',
    );
    return '$_temp0';
  }

  @override
  String get nudgeBody => 'Tap to sort them.';

  @override
  String get nudgeSettingsExplain =>
      'While anything is Unsorted, a Nudge comes again after this long when you ignore it. In the Quiet Hours Nudges wait until they end; Routines and Reminders still ring on time.';

  @override
  String get nudgeRepeat => 'Nudge again after';

  @override
  String get quietHours => 'Quiet Hours';

  @override
  String get quietFrom => 'From';

  @override
  String get quietUntil => 'Until';

  @override
  String get yes => 'Yes';

  @override
  String get no => 'No';

  @override
  String get deleteNoteLinkedDetail =>
      'Deleted ones come back with the Note when you restore it. Kept ones go on reminding you.';

  @override
  String get noteInTrash => 'The Note is in the trash';

  @override
  String get noteDeleted => 'The Note was deleted';

  @override
  String get allowAccess => 'Allow';

  @override
  String get captureContextTitle => 'Capture Context';

  @override
  String get captureContextIntro =>
      'Besides when and where, a Note can record more of what was going on. This is off until you switch it on, and stays in the Note on your own devices.';

  @override
  String get calendarContextSwitch => 'Note the running appointment';

  @override
  String get calendarContextDetail =>
      'The event running when a Note is captured, like “during: Dentist”.';

  @override
  String get bluetoothContextSwitch => 'Bluetooth device';

  @override
  String get bluetoothContextDetail =>
      'The device connected when a Note is captured, like your car or headphones.';

  @override
  String get calendarExplainTitle => 'Remember your appointment?';

  @override
  String get calendarExplainBody =>
      'Old Brain Fix can note the calendar event running when you capture a Note, so it says “during: Dentist” days later. It reads your calendar only at that moment. The event goes only where its Note goes: to your own Devices, and to the people of a Shared Area you file the Note in. Never to anyone else.';

  @override
  String get bluetoothExplainTitle => 'Remember what was connected?';

  @override
  String get bluetoothExplainBody =>
      'Old Brain Fix can note the Bluetooth device connected when you capture a Note, like your car or your headphones. Android calls this “Nearby devices”; Old Brain Fix only reads the names of connected devices and never searches for others. The name goes only where its Note goes: to your own Devices, and to the people of a Shared Area you file the Note in. Never to anyone else.';

  @override
  String get bluetoothNotAllowed =>
      'Old Brain Fix may not see your Bluetooth devices.';

  @override
  String contextSourceApp(String app) {
    return 'Shared from $app';
  }

  @override
  String contextDuring(String events) {
    return 'During: $events';
  }

  @override
  String contextBluetooth(String devices) {
    return 'Connected: $devices';
  }

  @override
  String get moreCaptureContext => 'More about the capture';

  @override
  String get recordVoice => 'Record voice';

  @override
  String get recording => 'Recording…';

  @override
  String get stopRecording => 'Stop';

  @override
  String get voiceRecording => 'Voice recording';

  @override
  String get playRecording => 'Play the voice recording';

  @override
  String get stopPlaying => 'Stop';

  @override
  String get microphoneExplainTitle => 'Use the microphone?';

  @override
  String get microphoneExplainBody =>
      'Voice Notes record what you say, so a thought is kept before it is gone. Old Brain Fix uses the microphone only while you record. The recording goes only where its Note goes: to your own Devices, and to the people of a Shared Area you file the Note in. Never to anyone else.';

  @override
  String get microphoneMissing => 'Voice Notes need the microphone.';

  @override
  String get typeInstead => 'Type instead';

  @override
  String get widgetNothingToday => 'Nothing due today.';

  @override
  String widgetMore(String count) {
    return 'and $count more';
  }

  @override
  String get speechModelTitle => 'Speech model';

  @override
  String get speechModelIntro =>
      'Voice Notes get a Transcript you can read and search. Whisper writes it on this phone, in English or German, with a speech model it needs once. The recording is always kept.';

  @override
  String speechModelSource(int megabytes) {
    return 'The download is about $megabytes MB, comes only from Old Brain Fix\'s own release on GitHub, and is checked against a fixed checksum before it is used. Wi-Fi is best. Nothing else is ever sent or fetched.';
  }

  @override
  String get speechModelCredit =>
      'The speech model is the base model of Whisper by OpenAI, Copyright (c) 2022 OpenAI, under the MIT License.';

  @override
  String get speechModelChecking => 'Checking the speech model…';

  @override
  String get speechModelUnsupported =>
      'Transcripts are not available on this device. Voice recordings are kept as they are.';

  @override
  String get speechModelReady =>
      'The speech model is ready. Voice Notes are transcribed on this phone.';

  @override
  String get speechModelMissing => 'No speech model yet';

  @override
  String speechModelDownload(int megabytes) {
    return 'Download ($megabytes MB)';
  }

  @override
  String get speechModelPick => 'Use a file of my own';

  @override
  String get speechModelPickDetail =>
      'It must be whisper.cpp\'s multilingual ggml-base.bin; any other file is rejected.';

  @override
  String speechModelDownloading(String received, String total) {
    return 'Downloading: $received of $total MB';
  }

  @override
  String get speechModelFailed =>
      'The download did not finish, and nothing was kept. Try again later.';

  @override
  String get speechModelMismatch =>
      'That is not the expected speech model, so it was not kept.';

  @override
  String get speechModelSourceModels =>
      'Each download comes only from Old Brain Fix\'s own release on GitHub and is checked against a fixed checksum before it is used. Wi-Fi is best. Nothing else is ever sent or fetched.';

  @override
  String get speechModelCreditModels =>
      'The speech models are the base and small models of Whisper by OpenAI, Copyright (c) 2022 OpenAI, under the MIT License.';

  @override
  String get speechModelChoice => 'Model';

  @override
  String speechModelBase(int megabytes) {
    return 'Base, $megabytes MB';
  }

  @override
  String get speechModelBaseDetail =>
      'Faster, and enough for English or German alone.';

  @override
  String speechModelSmall(int megabytes) {
    return 'Small, $megabytes MB';
  }

  @override
  String get speechModelSmallDetail =>
      'Better with English words in German speech, but slower, and it needs more memory.';

  @override
  String get speechModelKeepsOne =>
      'Choosing a model downloads it. Only the chosen model is kept: the one before is deleted once the new one is in place.';

  @override
  String get speechModelPickEither =>
      'It must be whisper.cpp\'s multilingual ggml-base.bin or ggml-small.bin; any other file is rejected.';

  @override
  String get agenda => 'Agenda';

  @override
  String get speechOfferQuestion => 'Transcribe voice Notes on the phone?';

  @override
  String speechOfferDetail(int megabytes) {
    return 'Download the speech model ($megabytes MB) once. It downloads in the background while you go on, and Settings offers it any time later.';
  }

  @override
  String speechOfferMobileData(int megabytes) {
    return 'You are on mobile data. The download uses $megabytes MB of it.';
  }

  @override
  String get transcribing => 'Transcribing…';

  @override
  String get noWordsHeard => 'No words heard.';

  @override
  String get historyTitle => 'History';

  @override
  String get streakHeading => 'Streak';

  @override
  String get streakNow => 'Now';

  @override
  String get streakLongest => 'Longest';

  @override
  String get lastSevenDays => 'Last 7 days';

  @override
  String dayAllDone(String day) {
    return '$day: all Done';
  }

  @override
  String dayPartlyDone(String day) {
    return '$day: partly Done, partly Missed';
  }

  @override
  String dayMissed(String day) {
    return '$day: Missed';
  }

  @override
  String dayOpen(String day) {
    return '$day: still due';
  }

  @override
  String dayNothing(String day) {
    return '$day: nothing due';
  }

  @override
  String get adherenceHeading => 'Adherence';

  @override
  String get adherenceThisWeek => 'This week';

  @override
  String get adherenceThisMonth => 'This month';

  @override
  String get adherenceLast30Days => 'Last 30 days';

  @override
  String adherenceOf(int done, int total) {
    return '$done of $total Done';
  }

  @override
  String get adherenceNone => 'Nothing yet';

  @override
  String get showStreaks => 'Show Streaks on this phone';

  @override
  String get showStreaksDetail => 'Adherence shows either way.';

  @override
  String get historyLog => 'Log';

  @override
  String get historyEmpty => 'Nothing was due yet.';

  @override
  String get snoozesHere =>
      'Snoozes show only as this phone recorded them, for about the last month.';

  @override
  String logDone(String time) {
    return 'Done: $time';
  }

  @override
  String logDoneEarly(String time) {
    return 'Done early: $time';
  }

  @override
  String logDoneLate(String time) {
    return 'Done late: $time';
  }

  @override
  String get logMissed => 'Missed';

  @override
  String get logOpen => 'Due, not Done yet';

  @override
  String get logComing => 'Still to come';

  @override
  String logSnoozed(String time) {
    return 'Snoozed until $time';
  }

  @override
  String get calendarTooltip => 'Calendar';

  @override
  String get addNoteToCalendar => 'Add to calendar at a time you pick';

  @override
  String addToCalendarAt(String when) {
    return 'Add to calendar: $when';
  }

  @override
  String get shareCalendarFile => 'Share as calendar file';

  @override
  String get noCalendarApp => 'There is no calendar app to add it to.';

  @override
  String get nothingTakesCalendarFile => 'No app can take the calendar file.';

  @override
  String get share => 'Share';

  @override
  String get exportCalendarFile => 'Export';

  @override
  String get allRoutinesTitle => 'All Routines and Reminders';

  @override
  String get allRoutinesEmpty => 'No Routines or Reminders yet.';

  @override
  String get calendarPageTitle => 'Calendar';

  @override
  String get calendarReadHeading => 'Read from your calendar';

  @override
  String get calendarWriteHeading => 'Write to your calendar';

  @override
  String get settingsCalendarDetail =>
      'What is read from your calendar and what is written to it.';

  @override
  String get calendarMirrorSwitch =>
      'Write Routines and Reminders into a calendar';

  @override
  String get calendarMirrorDetail =>
      'Your Routines and Reminders appear in a calendar of your choice on this device and follow every change.';

  @override
  String calendarMirrorIn(String calendar) {
    return 'In “$calendar”. Tap to choose another calendar.';
  }

  @override
  String get calendarMirrorExplainTitle => 'Write them into your calendar?';

  @override
  String get calendarMirrorExplainBody =>
      'Old Brain Fix can write your Routines and Reminders into a calendar you choose and keep them up to date. It only ever changes or removes the events it wrote itself, and turning this off removes them. Wherever that calendar syncs to, they show up too.';

  @override
  String get calendarMirrorNotAllowed =>
      'Old Brain Fix may not write to your calendar.';

  @override
  String get calendarMirrorNoCalendar =>
      'There is no calendar on this device to write into.';

  @override
  String get calendarMirrorNotRemoved =>
      'Old Brain Fix could not remove the events it wrote from the calendar.';

  @override
  String get agendaCalendarSwitch =>
      'Show my calendar\'s appointments on the Agenda';

  @override
  String get agendaCalendarDetail =>
      'The appointments of the calendars you pick show beside your Routines and Reminders. Nothing is copied.';

  @override
  String get agendaCalendarNonePicked =>
      'No calendar picked yet. Tap to pick calendars.';

  @override
  String agendaCalendarShows(String calendars) {
    return 'From $calendars. Tap to pick other calendars.';
  }

  @override
  String get agendaCalendarExplainTitle => 'Show your appointments?';

  @override
  String get agendaCalendarExplainBody =>
      'Old Brain Fix can show the appointments of the calendars you pick on the Agenda. It reads them whenever the Agenda shows, keeps no copy and sends nothing anywhere.';

  @override
  String get agendaCalendarNoCalendar =>
      'There is no calendar on this device to read from.';

  @override
  String get agendaCalendarsTitle => 'Calendars on the Agenda';

  @override
  String get agendaCalendarsHint =>
      'The Agenda shows the appointments of the calendars picked here.';

  @override
  String appointmentFromCalendar(String calendar) {
    return 'From the calendar “$calendar”';
  }

  @override
  String get appointmentCopy => 'Copy into a Note';

  @override
  String get appointmentNoTitle => 'Appointment without a title';

  @override
  String appointmentSince(String time) {
    return 'from $time';
  }

  @override
  String appointmentUntil(String time) {
    return 'until $time';
  }

  @override
  String appointmentPlace(String place) {
    return 'Place: $place';
  }

  @override
  String get settingsTitle => 'Settings';

  @override
  String get settingsIntro =>
      'These settings belong to this phone and are never synced.';

  @override
  String get settingsLook => 'Look';

  @override
  String get themeSystem => 'System';

  @override
  String get themeLight => 'Light';

  @override
  String get themeDark => 'Dark';

  @override
  String get settingsAlarms => 'Alarms';

  @override
  String get settingsAlarmSetupDetail =>
      'The permissions alarms need to ring on time.';

  @override
  String get settingsRenagDetail =>
      'How often an ignored alarm rings again, unless its Routine has its own interval.';

  @override
  String get alarmSound => 'Alarm sound';

  @override
  String get alarmSoundOwn =>
      'Old Brain Fix\'s own chime. Tap to choose another sound of the phone.';

  @override
  String get alarmSoundChosen =>
      'A sound of the phone. Tap to choose another, or Old Brain Fix\'s own chime.';

  @override
  String get settingsNudges => 'Nudges';

  @override
  String get settingsCapture => 'Capture';

  @override
  String get settingsNamedPlacesDetail =>
      '“Home” or “Work” instead of coordinates. Kept with your Notes, not just on this phone.';

  @override
  String get settingsCaptureContextDetail => 'Bluetooth device with each Note.';

  @override
  String get settingsSpeechModelDetail => 'For Transcripts of voice Notes.';

  @override
  String get settingsCalendar => 'Calendar';

  @override
  String get settingsHistory => 'History';

  @override
  String get aboutTitle => 'About Old Brain Fix';

  @override
  String get aboutTagline =>
      'Capture a thought in one tap. Get nagged until it is sorted or done.';

  @override
  String versionLabel(String version, String build) {
    return 'Version $version ($build)';
  }

  @override
  String get documentation => 'Documentation';

  @override
  String get privacyPolicy => 'Privacy policy';

  @override
  String get sourceCode => 'Source code';

  @override
  String get reportProblem => 'Report a problem or an idea';

  @override
  String get githubIssues => 'GitHub issues';

  @override
  String get writeTheDeveloper => 'Write to the developer';

  @override
  String get buyCoffee => 'Buy me a coffee';

  @override
  String get coffeeSubtitle => 'On Ko-fi, if Old Brain Fix helps you';

  @override
  String get openSourceLicenses => 'Open-source licenses';

  @override
  String get dangerButton => 'DON\'T PRESS.\nDANGER';

  @override
  String get dangerThanks => 'Thank you for using Old Brain Fix!';

  @override
  String get settingsPrivacy => 'Privacy';

  @override
  String get appLock => 'App Lock';

  @override
  String get appLockDetail =>
      'Asks for the PIN before showing your Notes. Capture and alarms never need it.';

  @override
  String get appLockOff => 'Turn off the App Lock';

  @override
  String get unlockApp => 'Unlock Old Brain Fix';

  @override
  String get lockedTitle => 'Old Brain Fix is locked';

  @override
  String get lockedExplain =>
      'Unlock to see your Notes. You can capture with + right away.';

  @override
  String get unlock => 'Unlock';

  @override
  String get unlockBiometrics => 'Unlock with fingerprint or screen lock';

  @override
  String get biometricsDetail => 'The PIN always works as well.';

  @override
  String get pinLabel => 'PIN';

  @override
  String get pinAgainLabel => 'PIN again';

  @override
  String get pinWrong => 'Wrong PIN';

  @override
  String pinTooShort(int count) {
    return 'At least $count digits';
  }

  @override
  String get pinMismatch => 'The two PINs differ';

  @override
  String get pinNewTitle => 'New PIN';

  @override
  String get pinNewExplain =>
      'It unlocks the App Lock and the Area Locks on this phone.';

  @override
  String get pinChange => 'Change PIN';

  @override
  String get relockAfter => 'Lock again after';

  @override
  String get relockDetail =>
      'How long the app may be in the background before it locks again.';

  @override
  String get relockAtOnce => 'At once';

  @override
  String get areaLocksTitle => 'Area Locks';

  @override
  String get areaLocksDetail =>
      'Areas that stay locked even while the app is open.';

  @override
  String get areaLocksExplain =>
      'A locked Area and its Notes stay hidden until you unlock it. Its alarms show Masked, “Private Routine, 08:00”, unless you choose to show them in full; they can be checked off either way.';

  @override
  String get areaLocked => 'Locked';

  @override
  String unlockArea(String area) {
    return 'Unlock $area';
  }

  @override
  String areaLockOff(String area) {
    return 'Unlock $area for good';
  }

  @override
  String areaShowFull(String area) {
    return 'Show the alarms of $area in full';
  }

  @override
  String get areaAlarmsShow => 'Alarms and Agenda show';

  @override
  String get areaAlarmsMasked => 'Masked';

  @override
  String get areaAlarmsFull => 'In full';

  @override
  String get maskedRoutine => 'Private Routine';

  @override
  String maskedRoutineAt(String time) {
    return 'Private Routine, $time';
  }

  @override
  String get maskedReminder => 'Private Reminder';

  @override
  String get syncTitle => 'Sync';

  @override
  String get syncSettingsEntry => 'Sync with another Device';

  @override
  String get syncSettingsDetail =>
      'Pair once by code; both then hold the same Notes, Areas and Routines.';

  @override
  String get syncJoinIntro =>
      'Open Sync on the other Device. Scan the code it shows, or type it in.';

  @override
  String get syncScan => 'Scan the code';

  @override
  String get syncCodeLabel => 'Pair code';

  @override
  String get syncJoin => 'Sync now';

  @override
  String get syncShowIntro =>
      'Or show a code here for the other Device to scan.';

  @override
  String get syncShowCode => 'Show this Device\'s code';

  @override
  String get shareShowIntro => 'Or show a code here for their Device to scan.';

  @override
  String get hostTitle => 'This Device\'s code';

  @override
  String get hostIntro =>
      'On the other Device, open Sync and scan this code, or type in the code under it.';

  @override
  String get hostInviteIntro =>
      'On their Device, open Sync and scan this code, or type in the code under it. Their Profile then shows here to confirm.';

  @override
  String get hostTypedCode => 'Code to type';

  @override
  String get hostStays =>
      'The code holds while this page is open. Go back once the other Device says it synced.';

  @override
  String get hostNoNetwork =>
      'This Device is on no Wi-Fi. Connect it to the same Wi-Fi as the other Device, then try again.';

  @override
  String get hostLocked =>
      'Too many wrong codes were tried. Make a new code to go on.';

  @override
  String get hostFailed => 'No code could be made. Try again.';

  @override
  String get hostTryAgain => 'Try again';

  @override
  String get hostNewCode => 'New code';

  @override
  String hostAskTitle(String name) {
    return 'Let $name join?';
  }

  @override
  String get hostAskUnnamed => 'Let the other Device join?';

  @override
  String get hostAskText =>
      'It gets everything on this Device and syncs with it from then on. Say yes only to a Device of your own.';

  @override
  String get hostAllow => 'Let it join';

  @override
  String get syncSyncing => 'Syncing…';

  @override
  String get syncInvalidCode =>
      'That is not a pair code. Check it against the other Device\'s screen.';

  @override
  String get syncNoCode => 'No pair code was found in the photo.';

  @override
  String syncCameIn(String name, int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count changes came in',
      one: '1 change came in',
      zero: 'nothing new came in',
    );
    return 'Synced with $name: $_temp0.';
  }

  @override
  String syncRefused(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count changes were refused: their signatures did not match.',
      one: '1 change was refused: its signature did not match.',
    );
    return '$_temp0';
  }

  @override
  String get syncFailNotPrivate =>
      'This code points outside your home network. Only Devices on the same network can sync.';

  @override
  String get syncFailUnreachable =>
      'The other Device did not answer. Does it still show the code, and are both on the same Wi-Fi?';

  @override
  String get syncFailWrongHost => 'This is not the Device the code belongs to.';

  @override
  String get syncFailWrongPin => 'The PIN does not match. Type the code again.';

  @override
  String get syncFailLocked =>
      'Too many wrong tries. Show a new code on the other Device.';

  @override
  String get syncFailDeclined => 'The other Device said no to this one.';

  @override
  String get syncFailUpdateHere =>
      'Old Brain Fix on this Device needs an update to sync with the other Device.';

  @override
  String get syncFailUpdateThere =>
      'Old Brain Fix on the other Device needs an update to sync with this one.';

  @override
  String syncUpdateHereWith(String name) {
    return 'Old Brain Fix on this Device needs an update to sync with $name.';
  }

  @override
  String syncUpdateThereWith(String name) {
    return 'Old Brain Fix on $name needs an update to sync with this Device.';
  }

  @override
  String get folderNeedsUpdate =>
      'Another Device writes a newer format into the shared folder. Old Brain Fix on this Device needs an update to read it.';

  @override
  String get syncFailFailed => 'Sync broke off. Nothing was lost; try again.';

  @override
  String get pairedDevicesTitle => 'Paired Devices';

  @override
  String get pairedNone => 'No paired Devices yet.';

  @override
  String pairedLastSync(String when) {
    return 'Last synced $when';
  }

  @override
  String get forgetDevice => 'Forget';

  @override
  String get syncSettingsStay =>
      'Settings stay on each Device and are never synced.';

  @override
  String get syncAutomatically => 'Sync automatically';

  @override
  String get syncAutoIntro =>
      'Paired Devices on the same Wi-Fi sync on their own, while the app is open and every quarter of an hour in the background.';

  @override
  String get conflictsTitle => 'Conflicts';

  @override
  String conflictsEntry(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count conflicts to settle',
      one: '1 conflict to settle',
      zero: 'No conflicts',
    );
    return '$_temp0';
  }

  @override
  String get conflictsIntro =>
      'Two Devices changed the same thing before they synced. Tap the version that stays; until then the newer one shows.';

  @override
  String get conflictsNone => 'No conflicts. Every Device agrees.';

  @override
  String get conflictNoteText => 'Note text';

  @override
  String get conflictSectionText => 'Text added to a Note';

  @override
  String get conflictLineText => 'Line of a Checklist';

  @override
  String get conflictShoppingLineText => 'Line of a Shopping List';

  @override
  String get conflictAreaName => 'Area name';

  @override
  String get conflictTopicName => 'Topic name';

  @override
  String get conflictPlaceName => 'Named Place name';

  @override
  String get conflictPlaceRadius => 'Named Place radius';

  @override
  String get conflictSchedule => 'Routine schedule';

  @override
  String get conflictOther => 'Change';

  @override
  String get conflictEmpty => '(empty)';

  @override
  String get conflictNewerFormat =>
      'A version was written with a newer version of Old Brain Fix. Update the app to pick one.';

  @override
  String get textNewerFormat =>
      'This text was written with a newer version of Old Brain Fix. Update the app to change it.';

  @override
  String get newerVersionNeeded =>
      'Saved by a newer version of Old Brain Fix. Update the app to change it.';

  @override
  String get storeNeedsUpdate =>
      'This Device\'s data was saved by a newer Old Brain Fix; install that version again.';

  @override
  String get storeNeedsUpdateUnchanged => 'Nothing on this Device was changed.';

  @override
  String conflictOnThisDevice(String when) {
    return 'On this Device, $when';
  }

  @override
  String conflictOnDevice(String device, String when) {
    return 'On $device, $when';
  }

  @override
  String conflictOnUnnamed(String when) {
    return 'On a Device without a name, $when';
  }

  @override
  String get conflictKeep => 'Keep this version';

  @override
  String get folderTitle => 'Shared folder';

  @override
  String get folderIntro =>
      'Syncs through a folder that Syncthing, Nextcloud or a USB stick carries between your Devices, wherever they are. Everything in it is encrypted: only Devices paired with this one can read it.';

  @override
  String get folderPick => 'Choose folder';

  @override
  String get folderForget => 'Forget';

  @override
  String get folderNotYet => 'Not synced yet';

  @override
  String folderSynced(String when, int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count changes came in',
      one: '1 change came in',
      zero: 'nothing new',
    );
    return 'Last synced $when: $_temp0';
  }

  @override
  String folderUnreachable(String when) {
    return 'The folder could not be reached at $when. It is tried again later.';
  }

  @override
  String folderUnreadable(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count files are from Devices this one is not paired with. Pair them first.',
      one: '1 file is from a Device this one is not paired with. Pair them first.',
    );
    return '$_temp0';
  }

  @override
  String get bundleTitle => 'Sync bundle';

  @override
  String get bundleIntro =>
      'Sends what is new as one file through any messenger or mail, and the other Device imports it. Everything in it is encrypted: only Devices paired with this one can read it.';

  @override
  String get bundleFor => 'For';

  @override
  String get bundleEverything => 'Everything';

  @override
  String get bundleSend => 'Send bundle';

  @override
  String get bundleImport => 'Import bundle';

  @override
  String bundleNothingNew(String name) {
    return 'Nothing new to send to $name.';
  }

  @override
  String bundleCameIn(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count changes came in with the bundle.',
      one: '1 change came in with the bundle.',
      zero: 'Nothing new came in with the bundle.',
    );
    return '$_temp0';
  }

  @override
  String get bundlePairFirst =>
      'This bundle is from a Device this one is not paired with, so nothing in it can be read. Pair them first, then import it again.';

  @override
  String get bundleNotABundle =>
      'This file is no sync bundle this app can read.';

  @override
  String get bundleNeedsUpdate =>
      'This bundle comes from a newer Old Brain Fix. Old Brain Fix on this Device needs an update to read it.';

  @override
  String get bundleFailed =>
      'The bundle did not work out. Nothing was lost; try again.';

  @override
  String get bundleNoApp => 'No app here can take the bundle.';

  @override
  String get shareArea => 'Share…';

  @override
  String get areaShared => 'Shared';

  @override
  String get peopleTitle => 'People';

  @override
  String shareTitle(String area) {
    return 'Share $area';
  }

  @override
  String get shareIntro =>
      'Meet the other person. On their Device, open Sync and show the code; scan it or type it here. Their Profile shows to confirm; then they get this Area and nothing else.';

  @override
  String get roleHeading => 'They may';

  @override
  String get roleManager => 'Manager';

  @override
  String get roleMember => 'Member';

  @override
  String get roleManagerDetail => 'Everything in this Area.';

  @override
  String get roleMemberDetail =>
      'Add Notes and change their own, add to any Note, check off Routines, Reminders and Frogs, and add and check Lines of Checklists and Shopping Lists.';

  @override
  String get shareInvite => 'Invite';

  @override
  String shareDone(String area, String name) {
    return '$area is now shared with $name.';
  }

  @override
  String sharedWithYou(String name, String area) {
    return '$name shared $area with you.';
  }

  @override
  String peopleYou(String name) {
    return '$name (you)';
  }

  @override
  String get makeManager => 'Make Manager';

  @override
  String get makeMember => 'Make Member';

  @override
  String get removePerson => 'Remove';

  @override
  String removePersonQuestion(String name, String area) {
    return 'Remove $name from $area?';
  }

  @override
  String get removePersonDetail =>
      'Nothing more of this Area goes to them. What they already have stays with them.';

  @override
  String get peopleIntro =>
      'A Manager may do everything here. A Member may add Notes and change their own, add to any Note, check off Routines, Reminders and Frogs, and add and check Lines of Checklists and Shopping Lists.';

  @override
  String get onlyManagers => 'Only a Manager of this Area can change this.';

  @override
  String get pairedPerson => 'Another person\'s Device';

  @override
  String get sendTitle => 'Send this Note';

  @override
  String get sendWho => 'Who gets it?';

  @override
  String get sendAnyone => 'Anyone, not encrypted';

  @override
  String sendEncrypted(String name) {
    return '$name, encrypted';
  }

  @override
  String get sendPlainHint =>
      'Not encrypted: whoever gets the file can read it. To encrypt it, share an Area with the person or pair with their Device first.';

  @override
  String get sendWhat =>
      'It carries the text, the Sections and the Routines and Reminders; no photos, recordings or where it was captured. Sending it again later sends it as an update.';

  @override
  String get sendAction => 'Send';

  @override
  String get sendNoApp => 'No app here can take the file.';

  @override
  String get sentItemTitle => 'Sent to you';

  @override
  String sentItemFrom(String name) {
    return 'From $name';
  }

  @override
  String sentItemVerified(String code) {
    return 'Verified: code compared in person. Code $code';
  }

  @override
  String sentItemKnown(String code) {
    return 'Not verified: known here, code not compared yet. Code $code';
  }

  @override
  String sentItemUnknown(String code) {
    return 'Not verified: this Profile is new here. Code $code';
  }

  @override
  String get sentItemCompare => 'Compare codes';

  @override
  String sentItemCompareHint(String name, String code) {
    return 'Ask $name, in person, to read out the code on their Profile page. It must be $code.';
  }

  @override
  String get sentItemCodesMatch => 'Codes match';

  @override
  String get sentItemCodesDiffer => 'Codes differ';

  @override
  String get sentItemVerifiedOnAccept => 'Marked as verified when you accept.';

  @override
  String get sentItemCodesDiffered =>
      'The codes differ: this file may not come from the person it names.';

  @override
  String get sentItemEncrypted => 'It came encrypted to this Device.';

  @override
  String get sentItemPlain =>
      'It came unencrypted: others may have read it on the way.';

  @override
  String get sentItemAccept => 'Accept';

  @override
  String get sentItemDecline => 'Decline';

  @override
  String get sentItemAccepted => 'Added to Unsorted, with its alarms on.';

  @override
  String get sentItemDeclined => 'Declined: nothing was kept.';

  @override
  String get sentItemAlready =>
      'Already up to date: you accepted this version or a newer one before.';

  @override
  String get sentItemUnreadable =>
      'This file cannot be read: it is damaged, or not signed by the key it names.';

  @override
  String get sentItemNotForUs => 'This file is encrypted for someone else.';

  @override
  String get sendUpdate => 'Send an update';

  @override
  String get sendCancel => 'Send a cancellation instead';

  @override
  String get sendCancelHint =>
      'They can then move it to their trash, with its alarms off.';

  @override
  String get sentItemUpdate => 'An update to a Note you accepted before.';

  @override
  String get sentItemChanges => 'What changes';

  @override
  String sentItemTextWas(String text) {
    return 'Text so far: $text';
  }

  @override
  String sentItemTextNow(String text) {
    return 'New text: $text';
  }

  @override
  String get sentItemTextYours =>
      'You changed this text yourself; the update replaces your version.';

  @override
  String sentItemSectionAdded(String text) {
    return 'Added: $text';
  }

  @override
  String sentItemRescheduled(String before, String after) {
    return '$before → $after';
  }

  @override
  String sentItemRoutineAdded(String schedule) {
    return 'New: $schedule';
  }

  @override
  String sentItemRoutineRemoved(String schedule) {
    return 'Removed: $schedule';
  }

  @override
  String get sentItemMakesChecklist => 'It becomes a Checklist.';

  @override
  String get sentItemMakesShoppingList => 'It becomes a Shopping List.';

  @override
  String sentItemLineReappear(String time) {
    return 'Reappear time: $time';
  }

  @override
  String get sentItemMakesPlain => 'It becomes a plain Note again.';

  @override
  String sentItemLineAdded(String text) {
    return 'New Line: $text';
  }

  @override
  String sentItemLineRenamed(String before, String after) {
    return 'Line: $before → $after';
  }

  @override
  String sentItemLineRemoved(String text) {
    return 'Line removed: $text';
  }

  @override
  String get sentItemNoChanges => 'Nothing changed.';

  @override
  String get sentItemRestores =>
      'It is in your trash; the update brings it back.';

  @override
  String get sentItemKeeps =>
      'Your own Sections, Routines and Reminders, Done marks, Importance and where you filed it stay as they are.';

  @override
  String get sentItemApply => 'Apply update';

  @override
  String get sentItemApplied => 'Updated. Your own additions stayed.';

  @override
  String sentItemCancelled(String text) {
    return 'Cancelled: $text';
  }

  @override
  String get sentItemCancelDetail =>
      'Accepting moves it and its Routines and Reminders to the trash, alarms off. You can restore it from there.';

  @override
  String get sentItemAcceptCancel => 'Move to the trash';

  @override
  String get sentItemTrashed =>
      'Moved to the trash with its alarms off. You can restore it from there.';

  @override
  String get sentItemNothingToCancel =>
      'Nothing to cancel: this Note is not among yours, or is in the trash already.';

  @override
  String get sentItemNotFromSender =>
      'Refused: this file would change a Note someone sent you, but it is not signed by them.';

  @override
  String get sentItemClose => 'Close';

  @override
  String get backupsTitle => 'Backups';

  @override
  String get backupsSettingsEntry => 'Back up this Device';

  @override
  String get backupsSettingsDetail =>
      'Encrypted files of everything on this Device, in a folder you choose.';

  @override
  String get backupsIntro =>
      'A Backup is an encrypted file of all your Areas, Notes, photos, voice recordings, Routines and their history. Only your passphrase, or the recovery key on the Recovery Sheet, opens it. This Device\'s settings stay out, except the playback speed. All your Devices share one passphrase and one recovery key, and each writes Backups of its own.';

  @override
  String get backupPassphrase => 'Passphrase';

  @override
  String get backupPassphraseAgain => 'Passphrase again';

  @override
  String get backupPassphraseHint =>
      'At least 8 characters. A short sentence is easy to remember.';

  @override
  String get backupPassphraseShort => 'At least 8 characters, please.';

  @override
  String get backupPassphraseMismatch => 'The two do not match.';

  @override
  String get backupSetUp => 'Set up Backups';

  @override
  String get backupSettingUp => 'Setting up… this takes a few seconds.';

  @override
  String get backupSetUpElsewhere =>
      'Set up already on another of your Devices? Sync with it, and this Device takes the same passphrase and recovery key.';

  @override
  String get backupSheetTitle => 'Recovery Sheet';

  @override
  String get backupSheetIntro =>
      'Print the Recovery Sheet or save it somewhere safe. Its recovery key opens the Backups of all your Devices when the passphrase is forgotten.';

  @override
  String get backupSheetPrint => 'Print or save';

  @override
  String get backupSheetShare => 'Share';

  @override
  String get backupSheetFailed =>
      'The Recovery Sheet could not be made. Try again.';

  @override
  String get backupSheetNoApp => 'No app here can take the Recovery Sheet.';

  @override
  String get backupFolderTitle => 'Folder';

  @override
  String get backupFolderNone =>
      'No folder chosen yet. Backups go into a folder you choose: on this Device, on a USB stick or in a cloud app.';

  @override
  String backupFolderChosen(String name) {
    return 'Backups go into $name.';
  }

  @override
  String get backupChooseFolder => 'Choose folder';

  @override
  String get backupWeekly => 'Back up every week';

  @override
  String get backupWeeklyDetail =>
      'The last 4 Backups of this Device are kept; older ones go.';

  @override
  String get backupNow => 'Back up now';

  @override
  String get backupWorking => 'Backing up…';

  @override
  String backupLast(String when) {
    return 'Last Backup: $when';
  }

  @override
  String get backupNotYet => 'No Backup yet.';

  @override
  String backupFailedAt(String when) {
    return 'The Backup of $when could not be written. Is the folder still there?';
  }

  @override
  String get backupDone => 'Backup written.';

  @override
  String get backupFailed =>
      'The Backup could not be written. Is the folder still there?';

  @override
  String get restoreOffer => 'Restore from a Backup';

  @override
  String get restoreOfferDetail =>
      'Coming from another Device? Bring everything back from a Backup file.';

  @override
  String get restoreIntro =>
      'Choose the Backup file, then open it with your passphrase or with the recovery key from the Recovery Sheet. Settings are not in a Backup: set them again afterwards, and pair your other Devices again.';

  @override
  String get restorePick => 'Choose the Backup file';

  @override
  String get restoreNotABackup => 'This file is not a Backup of Old Brain Fix.';

  @override
  String restoreFile(String when) {
    return 'Backup of $when';
  }

  @override
  String get restoreOr => 'or';

  @override
  String get restoreCodeLabel => 'Recovery key';

  @override
  String get restoreScan => 'Scan the Recovery Sheet';

  @override
  String get restoreNoCode => 'No recovery key was found in the photo.';

  @override
  String get restoreInvalidCode =>
      'That is not a recovery key. Check it against the Recovery Sheet.';

  @override
  String get restoreStart => 'Restore';

  @override
  String get restoreWorking => 'Restoring… this takes a moment.';

  @override
  String get restoreWrong =>
      'This passphrase or recovery key does not open the Backup.';

  @override
  String get restoreDamaged =>
      'The Backup is damaged or incomplete. Nothing was restored; try an older one.';

  @override
  String get restoreNotEmpty =>
      'This Device holds Notes already, so nothing was restored.';

  @override
  String get restoreFailed =>
      'The Backup could not be restored. Nothing was changed.';

  @override
  String restoreDone(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count changes',
      one: '1 change',
    );
    return 'Restored: $_temp0. Set alarms and locks again, choose the Backup folder again, and pair your other Devices again.';
  }

  @override
  String get recoverySheetPdfTitle => 'Old Brain Fix: Recovery Sheet';

  @override
  String recoverySheetMade(String date) {
    return 'Made on $date';
  }

  @override
  String recoverySheetProfile(String name) {
    return 'Profile: $name';
  }

  @override
  String get recoverySheetPdfIntro =>
      'This recovery key opens the Backups of all your Devices, even when the passphrase is forgotten. Whoever holds it can read them: keep this sheet somewhere safe, in a drawer or a password manager, never next to the Backups.';

  @override
  String get recoverySheetKey => 'Recovery key';

  @override
  String get recoverySheetStepRestore =>
      'To restore, install Old Brain Fix, choose “Restore from a Backup” on the first screen, pick the newest Backup file and type this key, or scan the QR code.';

  @override
  String get recoverySheetStepSettings =>
      'Settings such as alarms, Quiet Hours and locks are not in a Backup. Set them again on the new Device, and pair it again with your other Devices.';

  @override
  String get backupAutoTitle => 'Google backup';

  @override
  String get backupAutoSwitch =>
      'Also keep the newest Backup in my Google account';

  @override
  String get backupAutoDetail =>
      'Android\'s own backup then keeps the newest Backup file, encrypted as it is, in your Google account, and nothing else of the app. On a new phone it comes back with your apps, and the app asks for the passphrase or the recovery key.';

  @override
  String get backupAutoTooBig =>
      'The newest Backup is larger than the 25 MB Android sends to Google, so Google keeps none. A folder still takes it.';

  @override
  String get restoreFoundTitle => 'A Backup came back';

  @override
  String get restoreFound =>
      'This phone brought a Backup back from your Google account. Open it with your passphrase or with the recovery key from the Recovery Sheet.';

  @override
  String get restoreFoundSkip => 'Start without it';

  @override
  String get profileTitle => 'Profile';

  @override
  String get profileNoName => 'No name';

  @override
  String get profileName => 'Name';

  @override
  String get profileCode => 'Your code';

  @override
  String get profilePictureChange => 'Change the picture';

  @override
  String get profilePictureChoose => 'Choose a picture';

  @override
  String get profilePictureTake => 'Take a photo';

  @override
  String get profilePictureInitials => 'Show initials instead';

  @override
  String get profilePictureUnreadable => 'This picture cannot be read.';

  @override
  String get profileCropTitle => 'Choose the square';

  @override
  String get profileCropHint =>
      'Drag the square to move it. The slider sets its size.';

  @override
  String get profileCropSize => 'Size of the square';

  @override
  String get profileCropUse => 'Use this part';

  @override
  String get profileBackups => 'Backups and recovery';

  @override
  String get profileDevices => 'Your Devices';

  @override
  String get profileDeviceSettings => 'Settings of this Device';

  @override
  String get profileDeviceSettingsDetail =>
      'They belong to this Device and are never synced.';

  @override
  String get profileWhichTitle => 'Which one is you?';

  @override
  String get profileWhichText =>
      'Both Devices had a Profile of their own. The one you pick is kept on all of your Devices.';

  @override
  String get profileWhichThisDevice => 'On this Device';

  @override
  String get profileWhichOtherDevice => 'On the other Device';

  @override
  String get profileWhichPick => 'This is me';

  @override
  String get profileWhichLater => 'Later';

  @override
  String get profileWhichWaiting =>
      'Your Devices have two Profiles. Pick yours.';

  @override
  String inviteConfirmTitle(String area) {
    return 'Share $area with this person?';
  }

  @override
  String get inviteConfirmText =>
      'Compare the code with the one on their Profile page. Only then is anything shared.';

  @override
  String get inviteConfirmShare => 'Share';

  @override
  String get profileTheirCode => 'Their code';

  @override
  String authorPicture(String name) {
    return 'Written by $name';
  }

  @override
  String get shareCancelled => 'Nothing was shared.';

  @override
  String get personVerified => 'Verified: code compared in person.';

  @override
  String get personNotVerified =>
      'Not verified: compare codes in person to be sure.';

  @override
  String get personCodesDiffered =>
      'The codes differ: this Profile may not be the person it names.';

  @override
  String get personForMe => 'Name and picture for me';

  @override
  String get personForMeDetail =>
      'Only you see them, on your own Devices. Their own name and picture stay as they are.';

  @override
  String get personForMeName => 'Name for me';

  @override
  String get personForMePicture => 'Change the picture for me';

  @override
  String get personTheirPicture => 'Use their own picture';

  @override
  String get profilePeopleEmpty =>
      'Nobody yet. The people you add from their Profile card, share an Area with or accept Sent Items from show here.';

  @override
  String get personFilter => 'Show Notes by…';

  @override
  String get personFilterMe => 'Me';

  @override
  String get personFilterEveryone => 'Everyone';

  @override
  String get personFilterShowing => 'Only Notes by';

  @override
  String get personTheirNotes => 'Their Notes';

  @override
  String get crashTitle => 'That should not have happened';

  @override
  String get crashBody =>
      'Old Brain Fix ran into an unexpected error. Your saved Notes are safe. Restart the app, and if this happens again, send the report so it can be fixed.';

  @override
  String get crashRestart => 'Restart the app';

  @override
  String get crashSendReport => 'Send report to the developer';

  @override
  String get crashLastRunBody =>
      'Old Brain Fix stopped unexpectedly last time. Tap here to send a short report to the developer.';

  @override
  String get sendToPerson => 'Send to a person';

  @override
  String get addToCalendar => 'Add to calendar';

  @override
  String get deleteNoteQuestion => 'Delete this Note?';

  @override
  String get deleteNoteDetail => 'It stays in the trash for 30 days.';

  @override
  String get deleteNoteOnly => 'Delete only the Note';

  @override
  String deleteNoteWithLinked(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Delete with its $count Routines and Reminders',
      one: 'Delete with its Routine or Reminder',
    );
    return '$_temp0';
  }

  @override
  String get removeRecordingKeepText => 'Remove recording, keep text';

  @override
  String get removePhotoKeepText => 'Remove photo, keep text';

  @override
  String get removeRecordingQuestion =>
      'Remove this recording and keep its Transcript?';

  @override
  String get removePhotoQuestion =>
      'Remove this photo and keep its Photo Text?';

  @override
  String get removeMediaDetail =>
      'The file goes from all your Devices and from everyone you share the Note\'s Area with. The text stays on the Note.';

  @override
  String get remove => 'Remove';

  @override
  String get replaceNoteText => 'Replace Note text';

  @override
  String get appendToNoteText => 'Append to Note text';

  @override
  String get removeGeneratedText => 'Remove generated text';

  @override
  String get transcribeAgain => 'Transcribe again';

  @override
  String get readTextAgain => 'Read the text again';

  @override
  String get removeTranscriptQuestion => 'Remove this Transcript?';

  @override
  String get removePhotoTextQuestion => 'Remove this Photo Text?';

  @override
  String get removeGeneratedTextDetail =>
      'Search no longer finds the text, on all your Devices. The file stays, and the text can be made from it again.';

  @override
  String get transcribeAgainFailed => 'A recording could not be transcribed.';

  @override
  String get transcribeAgainFailedTitle => 'Recording not transcribed';

  @override
  String get transcribeAgainFailedCause =>
      'Whisper made no text from this recording.';

  @override
  String get readTextAgainFailed => 'The text in a photo could not be read.';

  @override
  String get readTextAgainFailedTitle => 'Photo not read';

  @override
  String get readTextAgainFailedCause =>
      'The text recognition could not read this photo.';

  @override
  String get generatedTextAgainFix =>
      'The file stays on the Note; try again later.';

  @override
  String get playbackSpeed => 'Playback speed';

  @override
  String playbackSpeedLabel(String speed) {
    return '$speed×';
  }

  @override
  String playbackSpeedOption(String speed) {
    return '×$speed';
  }

  @override
  String playbackSpeedSemantics(String speed) {
    return 'Playback speed $speed times';
  }

  @override
  String get close => 'Close';

  @override
  String get report => 'Report';

  @override
  String sharedRecordingFailed(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Shared recordings cannot be transcribed.',
      one: 'A shared recording cannot be transcribed.',
    );
    return '$_temp0';
  }

  @override
  String get sharedRecordingFailedTitle => 'Recording not transcribed';

  @override
  String get sharedRecordingUnreadable =>
      'No sound was found in the shared file: it is not a sound file, or it is damaged.';

  @override
  String get sharedRecordingUnsupported =>
      'This phone cannot decode the format of the shared file.';

  @override
  String get sharedRecordingFix =>
      'The file stays in the Note as it is; type what it says into the Note, or share it again as m4a or mp3.';

  @override
  String get personBlock => 'Block';

  @override
  String get personUnblock => 'Unblock';

  @override
  String get personBlocked =>
      'Blocked: nothing they write is kept or shown, only their check-offs.';

  @override
  String personBlockQuestion(String name) {
    return 'Block $name?';
  }

  @override
  String personBlockDetail(String name) {
    return 'Everything $name wrote is deleted from your Devices, except their check-offs, and nothing more of theirs is kept. A shared folder can still hold their encrypted files, which the app never opens; leaving the folder removes them.';
  }

  @override
  String personBlockLeaveDetail(String name, String areas) {
    return 'Everything $name wrote is deleted from your Devices, except their check-offs, and you leave $areas, which they manage. A shared folder can still hold their encrypted files, which the app never opens; leaving the folder removes them.';
  }

  @override
  String get personBlockLeave => 'Block and leave';

  @override
  String personUnblockQuestion(String name) {
    return 'Unblock $name?';
  }

  @override
  String get personUnblockDetail =>
      'What they wrote comes back with the next sync with a Device that still holds it.';

  @override
  String get peopleBlocked => 'Blocked';

  @override
  String get sentItemBlock => 'Decline and block';

  @override
  String get sentItemBlocked => 'Declined and blocked: nothing was kept.';

  @override
  String get sentItemFromBlocked =>
      'This file comes from a person you blocked.';

  @override
  String get sentItemStranger => 'An unknown person: not among your People.';

  @override
  String get sentItemHidden =>
      'What they sent stays hidden until you tap Show.';

  @override
  String get sentItemShow => 'Show';

  @override
  String get sentItemGone =>
      'This Sent Item is no longer waiting: it was accepted or declined.';

  @override
  String sentItemsWaiting(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Sent Items waiting',
      one: '1 Sent Item waiting',
    );
    return '$_temp0';
  }

  @override
  String get sentItemsWaitingTitle => 'Sent Items waiting';

  @override
  String get sentItemsNoneWaiting => 'No Sent Items are waiting.';

  @override
  String get profileCardSend => 'Send my Profile';

  @override
  String get profileCardSendDetail => 'As a link, through any messenger.';

  @override
  String get profileCardShowQr => 'Show my QR';

  @override
  String get profileCardShowQrDetail => 'For someone next to you to scan.';

  @override
  String profileCardText(String link) {
    return 'My Profile for Old Brain Fix:\n$link';
  }

  @override
  String get profileCardNotYet =>
      'Sync with your other Device first, then try again.';

  @override
  String get profileCardNoApp => 'No app here can take the link.';

  @override
  String get profileQrTitle => 'My Profile QR';

  @override
  String get profileQrHint =>
      'The other person scans this in Old Brain Fix, under Profile, Add a person. Scanned in person, you are verified for them at once.';

  @override
  String get profileQrTooLong =>
      'Your Profile has too many Devices for one QR. Send it as a link instead.';

  @override
  String get personAdd => 'Add a person';

  @override
  String get personAddDetail => 'From their Profile link or QR.';

  @override
  String get personAddIntro =>
      'Ask the person for their Profile: as a link through a messenger, or as a QR on their phone next to you. A QR scanned in person marks them verified at once.';

  @override
  String get personAddScan => 'Scan their QR';

  @override
  String get personAddPicture => 'QR from a picture';

  @override
  String get personAddLink => 'Their Profile link';

  @override
  String get personAddOpen => 'Add';

  @override
  String get cardNoQr => 'No QR code was found in the picture.';

  @override
  String get cardUnreadable =>
      'This is not a Profile card, or it was changed on the way. Ask for the link again.';

  @override
  String get cardOwn => 'This is your own Profile card.';

  @override
  String get cardBlocked => 'This card is from a person you blocked.';

  @override
  String get cardRefused =>
      'This card names a person in People but was not made by them.';

  @override
  String get personSendBack => 'Send yours back';

  @override
  String personSendBackDetail(String name) {
    return 'So $name can add you too.';
  }

  @override
  String get shareAfarHeading => 'Someone not nearby';

  @override
  String get shareAfarIntro =>
      'Share it with a person in People. They get a file through any messenger and accept or decline it.';

  @override
  String get shareAfarPick => 'Share with…';

  @override
  String get shareAfarNobody =>
      'Nobody in People can get it yet. Add the person from their Profile card first.';

  @override
  String shareWithTitle(String area) {
    return 'Share $area with…';
  }

  @override
  String get shareNotVerified => 'Not verified';

  @override
  String transportTitle(String name) {
    return 'How do changes travel with $name?';
  }

  @override
  String get transportFolder => 'A shared folder you both reach';

  @override
  String get transportFolderDetail =>
      'Nextcloud, Syncthing or a USB folder; changes travel on their own.';

  @override
  String get transportBundle => 'Files through a messenger';

  @override
  String get transportBundleDetail =>
      'You send changes as a file from their Profile page.';

  @override
  String get shareAfarFailed => 'The Area could not be shared.';

  @override
  String get areaOfferNotForUs =>
      'This shared Area is meant for another person or another of your Devices.';

  @override
  String get areaOfferFromBlocked =>
      'This shared Area comes from a person you blocked.';

  @override
  String get areaOfferRefused =>
      'This Area is your own on this Device already.';

  @override
  String get notAnAreaOffer =>
      'This file holds no shared Area this app can read.';

  @override
  String get areaOfferWaits => 'A shared Area is waiting on the Agenda.';

  @override
  String get areaOfferTitle => 'Shared Area';

  @override
  String areaOfferText(String name, String area, String role) {
    return '$name shares $area with you as $role';
  }

  @override
  String get areaOfferNoName => 'an Area';

  @override
  String get areaOfferByFolder =>
      'Changes travel through a shared folder you both reach.';

  @override
  String get areaOfferByBundle =>
      'Changes travel as files through a messenger.';

  @override
  String get areaOfferKeepsNothing =>
      'Nothing of the Area is kept until you accept.';

  @override
  String get areaOfferAccepted => 'Shared Area accepted.';

  @override
  String get areaOfferGone =>
      'This shared Area is no longer waiting: it was accepted or declined.';

  @override
  String get areaOfferFolderQuestion => 'Choose the shared folder?';

  @override
  String areaOfferFolderDetail(String area) {
    return 'Changes of $area travel through a folder you both reach. Choose it now, or later under Sync.';
  }

  @override
  String get areaOfferFolderLater => 'Later';

  @override
  String areaOffersWaiting(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count shared Areas waiting',
      one: '1 shared Area waiting',
    );
    return '$_temp0';
  }

  @override
  String get areaOffersWaitingTitle => 'Shared Areas waiting';

  @override
  String get areaOffersNoneWaiting => 'No shared Areas are waiting.';

  @override
  String personSendChanges(String name) {
    return 'Send changes to $name';
  }

  @override
  String get personSendChangesDetail => 'As a file through any messenger.';

  @override
  String changesNudgeTitle(String names) {
    return 'Changes for $names';
  }

  @override
  String get changesNudgeBody => 'Tap to send them.';

  @override
  String get changesNudgePublic => 'Changes are waiting to be sent';

  @override
  String namesAnd(String first, String last) {
    return '$first and $last';
  }

  @override
  String get frog => 'Frog';

  @override
  String get eatAFrog => 'Eat a frog';

  @override
  String get anotherFrog => 'Another one';

  @override
  String get frogsTitle => 'Frogs';

  @override
  String get frogsDone => 'Done';

  @override
  String frogDoneOn(String date) {
    return 'Done on $date';
  }

  @override
  String get frogsEmpty =>
      'No open Frogs. \"Routine, Reminder, Frog or Session\" on a Note\'s card makes one.';

  @override
  String get maskedFrog => 'Private Frog';

  @override
  String get deleteFrogQuestion =>
      'Delete this Frog? It stays in the trash. Its Note stays.';

  @override
  String get makeTally => 'Make it a Tally';

  @override
  String get addTallyEntry => 'Add entry';

  @override
  String get makeChecklist => 'Make it a Checklist';

  @override
  String get checklistToPlain => 'Back to a plain Note';

  @override
  String get checklistUncheckAll => 'Uncheck all';

  @override
  String get checklistHeading => 'Checklist';

  @override
  String checklistCount(int checked, int count) {
    return '$checked of $count checked';
  }

  @override
  String get checklistNewLine => 'New Line';

  @override
  String get checklistAddLine => 'Add Line';

  @override
  String get checklistRenameTitle => 'Rename Line';

  @override
  String get checklistMoveUp => 'Move up';

  @override
  String get checklistMoveDown => 'Move down';

  @override
  String get checklistIndent => 'Put under the Line above';

  @override
  String get checklistOutdent => 'Back to the top level';

  @override
  String get checklistDeleteQuestion => 'Delete this Line?';

  @override
  String get makeShoppingList => 'Make it a Shopping List';

  @override
  String get shoppingHeading => 'Shopping List';

  @override
  String shoppingOpenCount(int count) {
    return '$count open';
  }

  @override
  String shoppingBack(String day) {
    return 'back $day';
  }

  @override
  String get shoppingReappear => 'Reappear time';

  @override
  String get shoppingOnce => 'Once';

  @override
  String shoppingWeeks(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count weeks',
      one: '1 week',
    );
    return '$_temp0';
  }

  @override
  String get shoppingDeleteForGood => 'Delete';

  @override
  String get tallyHeading => 'Tally';

  @override
  String get tallyNewField => 'New field';

  @override
  String get tallyField => 'Field';

  @override
  String get tallyFieldName => 'Name';

  @override
  String get tallyFieldNameHint => 'e.g. Push-ups, Pages read';

  @override
  String get tallyFieldNameMissing => 'Give the field a name.';

  @override
  String get tallyUnit => 'Unit';

  @override
  String get tallyUnitTyped => 'Another unit';

  @override
  String get tallyUnitTypedHint => 'e.g. pages, times';

  @override
  String get tallyUnitFixed =>
      'Entries hold values for this field, so its kind of unit stays.';

  @override
  String get unitKindMass => 'Mass';

  @override
  String get unitKindDistance => 'Distance';

  @override
  String get unitKindSpeed => 'Speed';

  @override
  String get unitKindTemperature => 'Temperature';

  @override
  String get unitKindAngle => 'Angle';

  @override
  String get unitKindTime => 'Time';

  @override
  String get tallyTarget => 'Target (optional)';

  @override
  String get tallyTargetHint => 'Prefilled in every new entry';

  @override
  String get tallyNotANumber => 'Not a number. Type it like 12, 2.5 or 1:30.';

  @override
  String get tallyRule => 'A day with several entries counts';

  @override
  String get tallyRuleSum => 'Their sum';

  @override
  String get tallyRuleMax => 'The largest';

  @override
  String get tallyHideField => 'Hide field';

  @override
  String get tallyHideFieldDetail =>
      'Its values stay stored and return when it is shown again.';

  @override
  String get tallyShowField => 'Show field again';

  @override
  String get tallyFieldHidden => 'Hidden';

  @override
  String get tallyAddField => 'Add field';

  @override
  String get tallyFields => 'Fields';

  @override
  String get tallyNewEntry => 'New entry';

  @override
  String get tallyEntry => 'Entry';

  @override
  String get tallyEntryDate => 'Date';

  @override
  String get tallyEntryLine => 'Line of text';

  @override
  String get tallyEntryEmpty => 'Enter a value or a line of text.';

  @override
  String get tallyPeriod => 'One bar per';

  @override
  String get tallyPeriodDay => 'Day';

  @override
  String get tallyPeriodWeek => 'Week';

  @override
  String get tallyPeriodMonth => 'Month';

  @override
  String get tallyPeriodYear => 'Year';

  @override
  String get tallyShowEntries => 'Show entries';

  @override
  String get tallyEntriesMine => 'Mine';

  @override
  String get tallyEntriesEveryone => 'Everyone\'s';

  @override
  String get tallyEntries => 'Entries';

  @override
  String get tallyNoEntries => 'No entries yet.';

  @override
  String get tallyChartEmpty => 'No values yet.';

  @override
  String tallyChartRange(String from, String to) {
    return '$from to $to';
  }

  @override
  String tallyChartHighest(String value) {
    return 'Highest bar: $value';
  }

  @override
  String tallyChartLabel(String field, int bars) {
    return 'Chart of $field, $bars bars';
  }

  @override
  String tallyTargetValue(String value) {
    return 'Target $value';
  }

  @override
  String get tallyMoreEntries => 'Show more';

  @override
  String get tallyRemovedEntries => 'Removed entries';

  @override
  String get deleteEntryQuestion =>
      'Delete this entry? It can be restored under \"Removed entries\".';

  @override
  String get edit => 'Edit';

  @override
  String get settingsUnits => 'Units';

  @override
  String get settingsUnitsDetail =>
      'How Tallies show known units. Values are stored in metric either way.';

  @override
  String unitSystemRegion(String system) {
    return 'As the phone\'s region: $system';
  }

  @override
  String get unitSystemMetric => 'Metric (kg, km, °C)';

  @override
  String get unitSystemImperial => 'Imperial (lb, mi, °F)';

  @override
  String get unitSystemMetricName => 'metric';

  @override
  String get unitSystemImperialName => 'imperial';

  @override
  String get session => 'Session';

  @override
  String get newSession => 'New Session';

  @override
  String get sessionPool => 'Pool';

  @override
  String get sessionPoolExplain => 'The Tallies it picks from.';

  @override
  String get sessionPoolOrder =>
      'Rolling goes through them from the top; drag a handle to change the order.';

  @override
  String get sessionPoolNone =>
      'No Tallies yet. \"Make it a Tally\" in a Note\'s ⋮ makes one.';

  @override
  String get sessionPoolMissing => 'Choose at least one Tally.';

  @override
  String sessionMove(String name) {
    return 'Move $name';
  }

  @override
  String get sessionCount => 'How many it picks each time';

  @override
  String get sessionCountInvalid => 'Type a number from 1 to 99.';

  @override
  String get sessionWeekdayCounts => 'Count per weekday';

  @override
  String sessionWeekdayCountsSummary(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count weekdays with their own count',
      one: '1 weekday with its own count',
      zero: 'Every day the same',
    );
    return '$_temp0';
  }

  @override
  String get sessionWeekdayCountsExplain =>
      'A weekday left empty picks as many as every day.';

  @override
  String get sessionWeekdayCountHint => 'As every day';

  @override
  String get sessionRule => 'How it picks';

  @override
  String get sessionRuleRandom => 'Random';

  @override
  String get sessionRuleRandomDetail => 'A new random pick each time.';

  @override
  String get sessionRuleRolling => 'Rolling';

  @override
  String get sessionRuleRollingDetail =>
      'Through the pool in its order, continuing where it stopped.';

  @override
  String get sessionRuleLeast => 'Least selected';

  @override
  String get sessionRuleLeastDetail =>
      'The Tallies with the fewest entries first.';

  @override
  String get sessionAnother => 'Another one';

  @override
  String sessionPicks(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Picks $count Tallies',
      one: 'Picks 1 Tally',
    );
    return '$_temp0';
  }

  @override
  String get sessionNothingToPick => 'None of its Tallies is left to pick.';

  @override
  String get maskedTally => 'Private Tally';

  @override
  String captureChoicePlace(String place) {
    return '→ $place';
  }

  @override
  String captureChoiceRoutine(String schedule) {
    return 'Routine: $schedule';
  }

  @override
  String captureChoiceReminder(String schedule) {
    return 'Reminder: $schedule';
  }

  @override
  String captureChoiceSession(String schedule) {
    return 'Session: $schedule';
  }

  @override
  String captureChoiceTally(String field) {
    return 'Tally: $field';
  }

  @override
  String get addToShoppingList => 'Add to a Shopping List';

  @override
  String captureChoiceShopping(String list) {
    return 'Shopping List: $list';
  }

  @override
  String get captureShoppingUntranscribed =>
      'Kept as a Note: its recording could not be transcribed for the Shopping List.';

  @override
  String get captureShoppingGone =>
      'Kept as a Note: that Shopping List can no longer be added to.';

  @override
  String get captureShoppingNotAddedTitle => 'Not added to the Shopping List';

  @override
  String get captureShoppingUntranscribedCause =>
      'Whisper could not write a Transcript of the recording.';

  @override
  String get captureShoppingGoneCause =>
      'The Shopping List was deleted or made a plain Note meanwhile, here or on another Device.';

  @override
  String get captureShoppingKeptFix =>
      'The Note keeps all of it: add its lines to a Shopping List by hand.';

  @override
  String get noteBeingCaptured => 'The Note being captured';

  @override
  String get showNotesCompact => 'Show Notes compact';

  @override
  String get showNotesFull => 'Show Notes in full';

  @override
  String get transcriptLabel => 'Transcript';

  @override
  String get photoTextLabel => 'Photo Text';

  @override
  String get settingsTyping => 'Typing';

  @override
  String get twoRowEditor => 'Type in two rows';

  @override
  String get twoRowEditorDetail =>
      'In the Note editor and when adding to a Note, a line above the keyboard shows the text around the cursor, with buttons that move it one character.';

  @override
  String get cursorBack => 'Cursor back one character';

  @override
  String get cursorForward => 'Cursor forward one character';

  @override
  String get selectionBack => 'Selection end back one character';

  @override
  String get selectionForward => 'Selection end forward one character';

  @override
  String get cursorLine => 'Text around the cursor';

  @override
  String get cursorLineSelect => 'start selecting';

  @override
  String get cursorLineStopSelecting => 'stop selecting';

  @override
  String get cursorSpace => 'Space';

  @override
  String get cursorNewLine => 'New line';

  @override
  String get profileCardNotMade => 'Your Profile card could not be made.';

  @override
  String get profileCardNotMadeTitle => 'No Profile card yet';

  @override
  String get profileCardNotYetCause =>
      'This Device joins your Profile only once it has synced with another of your Devices.';

  @override
  String get profileCardNotSent => 'Your Profile card was not sent.';

  @override
  String get profileCardNotSentTitle => 'Profile card not sent';

  @override
  String get profileCardNoAppFix =>
      'Show your QR for them to scan instead, or install a messenger and send it again.';

  @override
  String get cardNotAdded => 'Nobody was added from the card.';

  @override
  String get cardNotAddedTitle => 'Nobody added';

  @override
  String get cardOwnFix => 'To add someone, open their card, not yours.';

  @override
  String get cardRefusedFix =>
      'Ask the person for their card again, in person if you can.';

  @override
  String get cardUnreadableCause =>
      'This is not a Profile card, or it was changed on the way.';

  @override
  String get cardAskAgainFix => 'Ask for the link again.';

  @override
  String get cardNotRead => 'The picture could not be read.';

  @override
  String get cardNotReadTitle => 'No QR code found';

  @override
  String get cardNoQrFix =>
      'Try a picture taken a little closer, or ask for the link.';

  @override
  String get blockedFix =>
      'To take what they send, unblock them on their Profile under People, then ask them to send it again.';

  @override
  String get hostShareDone => 'Sharing finished.';

  @override
  String get hostSyncDone => 'Sync finished.';

  @override
  String hostRefused(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Changes were refused.',
      one: 'A change was refused.',
    );
    return '$_temp0';
  }

  @override
  String get hostRefusedTitle => 'Changes refused';

  @override
  String get hostRefusedFix =>
      'Nothing of them was kept; the rest came in. If it happens again, report it.';

  @override
  String hostConflicts(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Some things were changed on both Devices. Tap to pick which versions stay.',
      one: 'Something was changed on both Devices. Tap to pick which version stays.',
    );
    return '$_temp0';
  }

  @override
  String get areaOfferNotTaken => 'The shared Area was not taken.';

  @override
  String get areaOfferNotTakenTitle => 'Shared Area not taken';

  @override
  String get areaOfferAskAgainFix =>
      'Ask the sender to share it again with you.';

  @override
  String get areaOfferOwnFix =>
      'It is among your Areas already; nothing needs doing.';

  @override
  String get sentItemNotTaken => 'The file was not taken.';

  @override
  String get sentItemNotTakenTitle => 'File not taken';

  @override
  String get sentItemAskAgainFix => 'Ask the sender to send it again.';

  @override
  String get shareAfarFailedTitle => 'Area not shared';

  @override
  String get shareAfarFailedCause =>
      'The file for the shared Area could not be written.';

  @override
  String get shareAfarFailedFix => 'Nothing was lost; try again.';

  @override
  String get shareAfarNoAppFix =>
      'Install a messenger, or choose a shared folder, then share again.';

  @override
  String get calendarFileNotShared => 'The calendar file was not shared.';

  @override
  String get calendarFileNotSharedTitle => 'Calendar file not shared';

  @override
  String get calendarFileFix =>
      'Install a calendar app that opens .ics files, then try again.';

  @override
  String get agendaCalendarFailed =>
      'The calendar cannot be shown on the Agenda.';

  @override
  String get agendaCalendarFailedTitle => 'Calendar not shown';

  @override
  String get agendaCalendarNotAllowedCause =>
      'Old Brain Fix may not read your calendar.';

  @override
  String get agendaCalendarAllowFix =>
      'You can allow it in the system settings, then turn this on again.';

  @override
  String get agendaCalendarAddFix =>
      'Add a calendar account in the phone\'s settings, then turn this on again.';

  @override
  String get nothingLostFix => 'Nothing was lost; try again.';

  @override
  String get codeNotRead => 'The photo could not be read.';

  @override
  String get codeNotReadTitle => 'No code found';

  @override
  String get syncNoCodeFix => 'Try again a little closer, or type the code.';

  @override
  String get restoreNoCodeFix => 'Try again a little closer, or type the key.';

  @override
  String get syncNotDone => 'The sync did not go through.';

  @override
  String get syncNotDoneTitle => 'Sync failed';

  @override
  String get syncNotPrivateCause =>
      'This code points outside your home network.';

  @override
  String get syncNotPrivateFix =>
      'Only Devices on the same network can sync: connect both to the same Wi-Fi, then try again.';

  @override
  String get syncUnreachableCause => 'The other Device did not answer.';

  @override
  String get syncUnreachableFix =>
      'Check that it still shows the code and that both are on the same Wi-Fi, then try again.';

  @override
  String get syncWrongHostFix =>
      'Scan or type the code the other Device shows now.';

  @override
  String get syncWrongPinCause => 'The PIN does not match.';

  @override
  String get syncWrongPinFix => 'Type the code again.';

  @override
  String get syncLockedCause => 'There were too many wrong tries.';

  @override
  String get syncLockedFix =>
      'Show a new code on the other Device, then try again.';

  @override
  String get syncDeclinedFix =>
      'Try again, and say yes on the other Device when it asks.';

  @override
  String get syncBrokeOffCause =>
      'The connection broke off before the sync finished.';

  @override
  String get updateFix =>
      'Install the latest Old Brain Fix on the Device named, then sync again.';

  @override
  String get updateNeeded => 'An update is needed to sync.';

  @override
  String get updateNeededTitle => 'Update needed';

  @override
  String get bundleNothingToSend => 'There is nothing new to send.';

  @override
  String get bundleNotSent => 'The bundle was not sent.';

  @override
  String get bundleNotSentTitle => 'Bundle not sent';

  @override
  String get bundleNotWrittenCause => 'The bundle file could not be written.';

  @override
  String get bundleNoAppFix =>
      'Install a messenger or a mail app, or choose a shared folder, then send it again.';

  @override
  String get bundleNotImported => 'The bundle was not imported.';

  @override
  String get bundleNotImportedTitle => 'Bundle not imported';

  @override
  String get bundleAskAgainFix =>
      'Ask for the bundle again, or import another file.';

  @override
  String get bundleNeedsUpdateCause =>
      'This bundle comes from a newer Old Brain Fix.';

  @override
  String get bundleUpdateFix =>
      'Update Old Brain Fix on this Device, then import it again.';

  @override
  String get bundleNotReadCause => 'The bundle could not be read to the end.';

  @override
  String get bundleImportAgainFix => 'Nothing of it was kept; import it again.';

  @override
  String get bundlePairFirstCause =>
      'This bundle is from a Device this one is not paired with, so nothing in it can be read.';

  @override
  String get bundlePairFirstFix =>
      'Pair the two Devices first, then import it again.';

  @override
  String get bundleImported => 'Bundle imported.';

  @override
  String get sentItemNotSent => 'The Note was not sent.';

  @override
  String get sentItemNotSentTitle => 'Note not sent';

  @override
  String get sentItemNotWrittenCause =>
      'The file for the Note could not be written.';

  @override
  String get sendNoAppFix =>
      'Install a messenger or a mail app, then send it again.';

  @override
  String get shareToApp => 'Share to an app';

  @override
  String get noteNotShared => 'The Note was not shared.';

  @override
  String get noteNotSharedTitle => 'Note not shared';

  @override
  String get shareNoApp => 'No app here can take the Note.';

  @override
  String get shareNoAppFix =>
      'Install a messenger or a mail app, then share it again.';

  @override
  String get shareNotWrittenCause =>
      'The photos and recordings of the Note could not be written for sharing.';

  @override
  String get shareNothingCause =>
      'The Note\'s photos and recordings are not on this Device yet.';

  @override
  String get shareNothingFix =>
      'Sync with your other Devices, then share it again.';

  @override
  String get calendarMirrorNotOn =>
      'The Routines and Reminders were not written into the calendar.';

  @override
  String get calendarMirrorNotOnTitle => 'Not written into the calendar';

  @override
  String get calendarMirrorChooseAgainFix =>
      'Allow Old Brain Fix to use your calendar in the system settings, then choose the calendar again.';

  @override
  String get calendarMirrorNotOff =>
      'The events were not removed from the calendar.';

  @override
  String get calendarMirrorNotOffTitle => 'Events not removed';

  @override
  String get calendarMirrorOffAgainFix =>
      'Allow Old Brain Fix to use your calendar in the system settings, then turn this off again.';

  @override
  String get contextNotOn =>
      'This part of the Capture Context was not switched on.';

  @override
  String get contextNotOnTitle => 'Not switched on';

  @override
  String get calendarEventNotAdded =>
      'The event was not added to the calendar.';

  @override
  String get calendarEventNotAddedTitle => 'Event not added';

  @override
  String get noCalendarAppFix => 'Install a calendar app, then try again.';

  @override
  String get backupSheetNotOut =>
      'The Recovery Sheet was not printed or shared.';

  @override
  String get backupSheetNotOutTitle => 'Recovery Sheet not handed over';

  @override
  String get backupSheetFailedCause =>
      'The file for the Recovery Sheet could not be written.';

  @override
  String get backupSheetNoAppFix =>
      'Install an app that prints or opens PDF files, then try again.';

  @override
  String get backupNotWritten => 'The Backup was not written.';

  @override
  String get backupNotWrittenTitle => 'Backup not written';

  @override
  String get backupFailedCause =>
      'The Backup could not be written into its folder.';

  @override
  String get backupFailedFix =>
      'Check that the folder is still there, or choose it again, then back up again.';

  @override
  String get restoreNotDone => 'The Backup was not restored.';

  @override
  String get restoreNotDoneTitle => 'Backup not restored';

  @override
  String get restorePickFix => 'Pick the Backup file again.';

  @override
  String get restoreWrongFix =>
      'Type the passphrase again, or use the recovery key on the Recovery Sheet.';

  @override
  String get restoreDamagedCause => 'The Backup is damaged or incomplete.';

  @override
  String get restoreDamagedFix => 'Nothing was restored; try an older Backup.';

  @override
  String get restoreNotEmptyFix =>
      'A Backup goes only onto a Device that holds no Notes yet.';

  @override
  String get restoreFailedCause => 'The Backup could not be read to the end.';

  @override
  String get restoreFailedFix => 'Nothing was changed; try again.';

  @override
  String get restoreDoneShort => 'Backup restored.';

  @override
  String get namedPlaceNotAdded => 'The place was not named.';

  @override
  String get namedPlaceNotAddedTitle => 'Place not named';

  @override
  String get locationAllowFix =>
      'You can allow it in the system settings, then try again.';

  @override
  String get locationUnknownFix => 'Turn location on, then try again.';

  @override
  String get profilePictureNotSet => 'The picture was not set.';

  @override
  String get profilePictureNotSetTitle => 'Picture not set';

  @override
  String get profilePictureFix => 'Pick another picture, or take a new photo.';

  @override
  String get speechModelNotKept => 'The speech model was not kept.';

  @override
  String get speechModelNotKeptTitle => 'Speech model not kept';

  @override
  String get speechModelFailedCause => 'The download did not finish.';

  @override
  String get speechModelFailedFix => 'Nothing was kept; try again later.';

  @override
  String get speechModelMismatchCause =>
      'That is not the expected speech model.';

  @override
  String get speechModelMismatchFix =>
      'Download one of the models listed, or pick the file of one of them.';

  @override
  String get settingsShoppingAgenda => 'Shopping Lists on the Agenda';

  @override
  String get settingsShoppingAgendaDetail =>
      'Today only, and on this Device only.';

  @override
  String get shoppingAgendaLines =>
      'Each Line that came back today, on its own row';

  @override
  String get shoppingAgendaBack =>
      'One row per Shopping List on a day when Lines came back';

  @override
  String get shoppingAgendaOpen =>
      'One row per Shopping List on every day it has open Lines';

  @override
  String get shoppingAgendaOff => 'Not at all';

  @override
  String shoppingRowBack(String list, int count) {
    return '$list: $count back';
  }

  @override
  String shoppingRowOpen(String list, int count) {
    return '$list: $count open';
  }

  @override
  String get maskedShoppingList => 'Private Shopping List';

  @override
  String get textMarkCheck => 'Check';

  @override
  String get textMarkFail => 'Fail';

  @override
  String get textMarkReopen => 'Open';

  @override
  String get textMarkOpen => 'Open';

  @override
  String get textMarkChecked => 'Checked';

  @override
  String get textMarkFailed => 'Failed';

  @override
  String get textMarkChange => 'change the mark';

  @override
  String get textMarkInsert => 'Put ☐ at the start of the line';

  @override
  String searchForTag(String tag) {
    return 'Search for $tag';
  }

  @override
  String get formatBold => 'Bold';

  @override
  String get formatItalic => 'Italic';

  @override
  String get formatStrike => 'Strike through';

  @override
  String get formatHeading1 => 'Large heading';

  @override
  String get formatHeading2 => 'Small heading';

  @override
  String get formatBullets => 'Bulleted list';

  @override
  String get formatNumbers => 'Numbered list';

  @override
  String get formatQuote => 'Quote';

  @override
  String get formatLink => 'Link';

  @override
  String get formatCode => 'Code';

  @override
  String get formatIndent => 'Indent list item';

  @override
  String get formatOutdent => 'Move list item out';

  @override
  String get moreFormats => 'More formats';

  @override
  String get linkAddress => 'Web or e-mail address';

  @override
  String get linkRemoveHint => 'Leave it empty to remove the link.';

  @override
  String get linkSet => 'Set link';

  @override
  String get formatTable => 'Table';

  @override
  String get tableTitle => 'Table';

  @override
  String get tableOpen => 'Open the table';

  @override
  String get tableSideways => 'Move the table sideways';

  @override
  String tableRow(int number) {
    return 'Row $number';
  }

  @override
  String tableColumn(String letter) {
    return 'Column $letter';
  }

  @override
  String get tableAddRow => 'Add a row';

  @override
  String get tableAddColumn => 'Add a column';

  @override
  String get tableRowAbove => 'Insert a row above';

  @override
  String get tableRowBelow => 'Insert a row below';

  @override
  String get tableRowUp => 'Move up';

  @override
  String get tableRowDown => 'Move down';

  @override
  String get tableRowDelete => 'Delete row';

  @override
  String get tableColumnBefore => 'Insert a column to the left';

  @override
  String get tableColumnAfter => 'Insert a column to the right';

  @override
  String get tableColumnLeft => 'Move left';

  @override
  String get tableColumnRight => 'Move right';

  @override
  String get tableColumnDelete => 'Delete column';

  @override
  String get tableHeadingColumn => 'First column as headings';

  @override
  String get tableDeleteRowQuestion => 'Delete this row?';

  @override
  String get tableDeleteColumnQuestion => 'Delete this column?';

  @override
  String get tableDeleteTableQuestion => 'Delete the table?';

  @override
  String get tableDeleteBody =>
      'The words in its cells are deleted from the Note.';

  @override
  String get pairCodeNeedsUpdate =>
      'This code comes from a newer Old Brain Fix. Update Old Brain Fix on this Device, then try again.';

  @override
  String get sentItemNeedsUpdate =>
      'This file comes from a newer Old Brain Fix.';

  @override
  String get restoreNeedsUpdate =>
      'This Backup was made by a newer Old Brain Fix.';

  @override
  String get updateOpenAgainFix =>
      'Update Old Brain Fix on this Device, then open it again.';

  @override
  String get settingsUpdateNeeded => 'An update of Old Brain Fix is needed.';

  @override
  String get settingsNeedUpdate =>
      'A setting a newer Old Brain Fix saved on this Device is kept as it is.';

  @override
  String get settingsUpdateFix =>
      'Install the latest Old Brain Fix on this Device.';
}
