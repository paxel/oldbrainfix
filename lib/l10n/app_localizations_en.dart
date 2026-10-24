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
  String get takePhoto => 'Take a photo';

  @override
  String get photo => 'Photo';

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
  String get nudgeSettingsHeading => 'Nudges on this phone';

  @override
  String nudgeSettingsSummary(int minutes, String quiet) {
    return 'Again after $minutes min, $quiet';
  }

  @override
  String get nudgeSettingsExplain =>
      'While anything is Unsorted, a Nudge comes again after this long when you ignore it. In the Quiet Hours Nudges wait until they end; Routines and Reminders still ring on time.';

  @override
  String get nudgeRepeat => 'Nudge again after';

  @override
  String get quietHours => 'Quiet Hours';

  @override
  String quietHoursRange(String from, String until) {
    return 'Quiet Hours $from to $until';
  }

  @override
  String get quietHoursOff => 'no Quiet Hours';

  @override
  String get quietFrom => 'From';

  @override
  String get quietUntil => 'Until';

  @override
  String get yes => 'Yes';

  @override
  String get no => 'No';

  @override
  String deleteNoteRoutines(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Also delete the $count linked Routines?',
      one: 'Also delete the linked Routine?',
    );
    return '$_temp0';
  }

  @override
  String deleteNoteReminders(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Also delete the $count linked Reminders?',
      one: 'Also delete the linked Reminder?',
    );
    return '$_temp0';
  }

  @override
  String deleteNoteRoutinesAndReminders(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Also delete the $count linked Routines and Reminders?',
    );
    return '$_temp0';
  }

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
      'Besides when and where, a Note can record more of what was going on. Each of these is off until you switch it on, and stays in the Note on your own devices.';

  @override
  String get calendarContextSwitch => 'Calendar event';

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
      'Old Brain Fix can note the calendar event running when you capture a Note, so it says “during: Dentist” days later. It reads your calendar only at that moment. The event stays in the Note on your own devices and is never sent anywhere else.';

  @override
  String get bluetoothExplainTitle => 'Remember what was connected?';

  @override
  String get bluetoothExplainBody =>
      'Old Brain Fix can note the Bluetooth device connected when you capture a Note, like your car or your headphones. Android calls this “Nearby devices”; Old Brain Fix only reads the names of connected devices and never searches for others. The name stays in the Note on your own devices and is never sent anywhere else.';

  @override
  String get calendarNotAllowed =>
      'Old Brain Fix may not read your calendar. You can allow it in the system settings.';

  @override
  String get bluetoothNotAllowed =>
      'Old Brain Fix may not see your Bluetooth devices. You can allow it in the system settings.';

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
  String get stopAndSave => 'Stop and save';

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
      'Voice Notes record what you say, so a thought is kept before it is gone. Old Brain Fix uses the microphone only while you record. The recording stays in the Note on your own devices and is never sent anywhere else.';

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
  String get calendarExportTooltip => 'Export to calendar';

  @override
  String get calendarExportTitle => 'Calendar export';

  @override
  String get calendarExportAll => 'All Routines and Reminders';

  @override
  String get calendarExportEmpty => 'No Routines or Reminders yet.';

  @override
  String get calendarMirrorSwitch => 'Keep in a calendar';

  @override
  String get calendarMirrorDetail =>
      'Your Routines and Reminders appear in a calendar of your choice on this device and follow every change.';

  @override
  String calendarMirrorIn(String calendar) {
    return 'In “$calendar”. Tap to choose another calendar.';
  }

  @override
  String get calendarMirrorExplainTitle => 'Keep them in your calendar?';

  @override
  String get calendarMirrorExplainBody =>
      'Old Brain Fix can write your Routines and Reminders into a calendar you choose and keep them up to date. It only ever changes or removes the events it wrote itself, and turning this off removes them. Wherever that calendar syncs to, they show up too.';

  @override
  String get calendarMirrorNotAllowed =>
      'Old Brain Fix may not write to your calendar. You can allow it in the system settings.';

  @override
  String get calendarMirrorNoCalendar =>
      'There is no calendar on this device to write into.';

  @override
  String get calendarMirrorNotRemoved =>
      'The events could not be removed from the calendar. Allow Old Brain Fix to use your calendar and turn this off again.';
}
