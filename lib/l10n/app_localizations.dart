import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_de.dart';
import 'app_localizations_en.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('de'),
    Locale('en'),
  ];

  /// No description provided for @appTitle.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix'**
  String get appTitle;

  /// No description provided for @noNotesYet.
  ///
  /// In en, this message translates to:
  /// **'Nothing captured yet.'**
  String get noNotesYet;

  /// No description provided for @captureTooltip.
  ///
  /// In en, this message translates to:
  /// **'Capture a Note'**
  String get captureTooltip;

  /// No description provided for @captureHint.
  ///
  /// In en, this message translates to:
  /// **'What\'s on your mind?'**
  String get captureHint;

  /// No description provided for @capturePhotoHint.
  ///
  /// In en, this message translates to:
  /// **'A few words about it (optional)'**
  String get capturePhotoHint;

  /// No description provided for @takePhoto.
  ///
  /// In en, this message translates to:
  /// **'Take a photo'**
  String get takePhoto;

  /// No description provided for @photo.
  ///
  /// In en, this message translates to:
  /// **'Photo'**
  String get photo;

  /// No description provided for @photoMissing.
  ///
  /// In en, this message translates to:
  /// **'Photo not on this device'**
  String get photoMissing;

  /// No description provided for @morePhotos.
  ///
  /// In en, this message translates to:
  /// **'+{count}'**
  String morePhotos(int count);

  /// No description provided for @save.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get save;

  /// No description provided for @dismissAnnouncement.
  ///
  /// In en, this message translates to:
  /// **'Dismiss'**
  String get dismissAnnouncement;

  /// No description provided for @browseTooltip.
  ///
  /// In en, this message translates to:
  /// **'Search and browse'**
  String get browseTooltip;

  /// No description provided for @areasTitle.
  ///
  /// In en, this message translates to:
  /// **'Areas'**
  String get areasTitle;

  /// No description provided for @noAreasYet.
  ///
  /// In en, this message translates to:
  /// **'No Areas yet. Tap + to make one.'**
  String get noAreasYet;

  /// No description provided for @noTopicsYet.
  ///
  /// In en, this message translates to:
  /// **'No Topics yet. Notes can sit in the Area itself, or tap + to make a Topic.'**
  String get noTopicsYet;

  /// No description provided for @newArea.
  ///
  /// In en, this message translates to:
  /// **'New Area'**
  String get newArea;

  /// No description provided for @newTopic.
  ///
  /// In en, this message translates to:
  /// **'New Topic'**
  String get newTopic;

  /// No description provided for @nameHint.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get nameHint;

  /// No description provided for @cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancel;

  /// No description provided for @create.
  ///
  /// In en, this message translates to:
  /// **'Create'**
  String get create;

  /// No description provided for @rename.
  ///
  /// In en, this message translates to:
  /// **'Rename'**
  String get rename;

  /// No description provided for @delete.
  ///
  /// In en, this message translates to:
  /// **'Delete'**
  String get delete;

  /// No description provided for @areaLook.
  ///
  /// In en, this message translates to:
  /// **'Colour and icon'**
  String get areaLook;

  /// No description provided for @colourHeading.
  ///
  /// In en, this message translates to:
  /// **'Colour'**
  String get colourHeading;

  /// No description provided for @iconHeading.
  ///
  /// In en, this message translates to:
  /// **'Icon'**
  String get iconHeading;

  /// No description provided for @noColour.
  ///
  /// In en, this message translates to:
  /// **'No colour'**
  String get noColour;

  /// No description provided for @noIcon.
  ///
  /// In en, this message translates to:
  /// **'No icon'**
  String get noIcon;

  /// No description provided for @deleteAreaQuestion.
  ///
  /// In en, this message translates to:
  /// **'Delete the Area “{name}”?'**
  String deleteAreaQuestion(String name);

  /// No description provided for @deleteTopicQuestion.
  ///
  /// In en, this message translates to:
  /// **'Delete the Topic “{name}”?'**
  String deleteTopicQuestion(String name);

  /// No description provided for @topicsGoToo.
  ///
  /// In en, this message translates to:
  /// **'Its Topics are deleted with it.'**
  String get topicsGoToo;

  /// No description provided for @notesInside.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 Note inside.} other{{count} Notes inside.}}'**
  String notesInside(int count);

  /// No description provided for @whereNotesGo.
  ///
  /// In en, this message translates to:
  /// **'Where should they go?'**
  String get whereNotesGo;

  /// No description provided for @moveToUnsorted.
  ///
  /// In en, this message translates to:
  /// **'Move to Unsorted'**
  String get moveToUnsorted;

  /// No description provided for @moveToArea.
  ///
  /// In en, this message translates to:
  /// **'Move to “{area}”'**
  String moveToArea(String area);

  /// No description provided for @unsorted.
  ///
  /// In en, this message translates to:
  /// **'Unsorted'**
  String get unsorted;

  /// No description provided for @fileInto.
  ///
  /// In en, this message translates to:
  /// **'File into…'**
  String get fileInto;

  /// No description provided for @starterSetTitle.
  ///
  /// In en, this message translates to:
  /// **'Pick your Areas'**
  String get starterSetTitle;

  /// No description provided for @starterSetIntro.
  ///
  /// In en, this message translates to:
  /// **'Here is a start. Untick what you don\'t need — you can change everything later.'**
  String get starterSetIntro;

  /// No description provided for @starterSetDone.
  ///
  /// In en, this message translates to:
  /// **'Start'**
  String get starterSetDone;

  /// No description provided for @starterHousehold.
  ///
  /// In en, this message translates to:
  /// **'Household'**
  String get starterHousehold;

  /// No description provided for @starterKitchen.
  ///
  /// In en, this message translates to:
  /// **'Kitchen'**
  String get starterKitchen;

  /// No description provided for @starterGarden.
  ///
  /// In en, this message translates to:
  /// **'Garden'**
  String get starterGarden;

  /// No description provided for @starterCleaning.
  ///
  /// In en, this message translates to:
  /// **'Cleaning'**
  String get starterCleaning;

  /// No description provided for @starterWork.
  ///
  /// In en, this message translates to:
  /// **'Work'**
  String get starterWork;

  /// No description provided for @starterMeetings.
  ///
  /// In en, this message translates to:
  /// **'Meetings'**
  String get starterMeetings;

  /// No description provided for @starterToSortOut.
  ///
  /// In en, this message translates to:
  /// **'To sort out'**
  String get starterToSortOut;

  /// No description provided for @starterCar.
  ///
  /// In en, this message translates to:
  /// **'Car'**
  String get starterCar;

  /// No description provided for @starterService.
  ///
  /// In en, this message translates to:
  /// **'Service'**
  String get starterService;

  /// No description provided for @starterRepairs.
  ///
  /// In en, this message translates to:
  /// **'Repairs'**
  String get starterRepairs;

  /// No description provided for @starterHealth.
  ///
  /// In en, this message translates to:
  /// **'Health'**
  String get starterHealth;

  /// No description provided for @starterDoctor.
  ///
  /// In en, this message translates to:
  /// **'Doctor'**
  String get starterDoctor;

  /// No description provided for @starterMedication.
  ///
  /// In en, this message translates to:
  /// **'Medication'**
  String get starterMedication;

  /// No description provided for @searchAllHint.
  ///
  /// In en, this message translates to:
  /// **'Search all Notes'**
  String get searchAllHint;

  /// No description provided for @searchAreaHint.
  ///
  /// In en, this message translates to:
  /// **'Search this Area'**
  String get searchAreaHint;

  /// No description provided for @searchTopicHint.
  ///
  /// In en, this message translates to:
  /// **'Search this Topic'**
  String get searchTopicHint;

  /// No description provided for @clearSearch.
  ///
  /// In en, this message translates to:
  /// **'Clear search'**
  String get clearSearch;

  /// No description provided for @nothingFound.
  ///
  /// In en, this message translates to:
  /// **'No Note matches.'**
  String get nothingFound;

  /// No description provided for @noNotesHere.
  ///
  /// In en, this message translates to:
  /// **'No Notes here yet. Tap + to add one.'**
  String get noNotesHere;

  /// No description provided for @noteTitle.
  ///
  /// In en, this message translates to:
  /// **'Note'**
  String get noteTitle;

  /// No description provided for @importance.
  ///
  /// In en, this message translates to:
  /// **'Importance'**
  String get importance;

  /// No description provided for @importanceUnimportant.
  ///
  /// In en, this message translates to:
  /// **'Unimportant'**
  String get importanceUnimportant;

  /// No description provided for @importanceNormal.
  ///
  /// In en, this message translates to:
  /// **'Normal'**
  String get importanceNormal;

  /// No description provided for @importanceImportant.
  ///
  /// In en, this message translates to:
  /// **'Important'**
  String get importanceImportant;

  /// No description provided for @capturedHeading.
  ///
  /// In en, this message translates to:
  /// **'Captured'**
  String get capturedHeading;

  /// No description provided for @amend.
  ///
  /// In en, this message translates to:
  /// **'Add to Note'**
  String get amend;

  /// No description provided for @amendHint.
  ///
  /// In en, this message translates to:
  /// **'What\'s new?'**
  String get amendHint;

  /// No description provided for @add.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get add;

  /// No description provided for @moveToTrash.
  ///
  /// In en, this message translates to:
  /// **'Move to trash'**
  String get moveToTrash;

  /// No description provided for @trashTitle.
  ///
  /// In en, this message translates to:
  /// **'Trash'**
  String get trashTitle;

  /// No description provided for @trashEmpty.
  ///
  /// In en, this message translates to:
  /// **'The trash is empty.'**
  String get trashEmpty;

  /// No description provided for @restore.
  ///
  /// In en, this message translates to:
  /// **'Restore'**
  String get restore;

  /// No description provided for @goneIn.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =0{Deleted for good within a day} =1{Deleted for good in 1 day} other{Deleted for good in {count} days}}'**
  String goneIn(int count);

  /// No description provided for @trashThem.
  ///
  /// In en, this message translates to:
  /// **'Move to trash'**
  String get trashThem;

  /// No description provided for @orderByImportance.
  ///
  /// In en, this message translates to:
  /// **'Order by Importance'**
  String get orderByImportance;

  /// No description provided for @orderByTime.
  ///
  /// In en, this message translates to:
  /// **'Order by time'**
  String get orderByTime;

  /// No description provided for @hideUnimportant.
  ///
  /// In en, this message translates to:
  /// **'Hide unimportant Notes'**
  String get hideUnimportant;

  /// No description provided for @showUnimportant.
  ///
  /// In en, this message translates to:
  /// **'Show unimportant Notes'**
  String get showUnimportant;

  /// No description provided for @addHere.
  ///
  /// In en, this message translates to:
  /// **'Add here'**
  String get addHere;

  /// No description provided for @noteHere.
  ///
  /// In en, this message translates to:
  /// **'Note here'**
  String get noteHere;

  /// No description provided for @topicChoice.
  ///
  /// In en, this message translates to:
  /// **'Topic'**
  String get topicChoice;

  /// No description provided for @mergeInto.
  ///
  /// In en, this message translates to:
  /// **'Merge into…'**
  String get mergeInto;

  /// No description provided for @noOtherNotes.
  ///
  /// In en, this message translates to:
  /// **'There is no other Note to merge into.'**
  String get noOtherNotes;

  /// No description provided for @locationExplainTitle.
  ///
  /// In en, this message translates to:
  /// **'Remember where you were?'**
  String get locationExplainTitle;

  /// No description provided for @locationExplainBody.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix can note where you are when you capture a Note, so “nees mor sauce” still makes sense days later. Name your own places, like Home or Work, and a Note says “Home” instead of coordinates. Your location stays on this phone and is never sent anywhere.'**
  String get locationExplainBody;

  /// No description provided for @notNow.
  ///
  /// In en, this message translates to:
  /// **'Not now'**
  String get notNow;

  /// No description provided for @allowLocation.
  ///
  /// In en, this message translates to:
  /// **'Allow'**
  String get allowLocation;

  /// No description provided for @locationNotAllowed.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix may not use your location, so it cannot tell where you are. You can allow it in the system settings.'**
  String get locationNotAllowed;

  /// No description provided for @locationUnknown.
  ///
  /// In en, this message translates to:
  /// **'Could not find out where you are. Is location turned on?'**
  String get locationUnknown;

  /// No description provided for @namedPlacesTitle.
  ///
  /// In en, this message translates to:
  /// **'Named Places'**
  String get namedPlacesTitle;

  /// No description provided for @noNamedPlacesYet.
  ///
  /// In en, this message translates to:
  /// **'No Named Places yet. Go to a spot like Home or Work and tap + to name it.'**
  String get noNamedPlacesYet;

  /// No description provided for @newNamedPlace.
  ///
  /// In en, this message translates to:
  /// **'Name this place'**
  String get newNamedPlace;

  /// No description provided for @editNamedPlace.
  ///
  /// In en, this message translates to:
  /// **'Rename or resize'**
  String get editNamedPlace;

  /// No description provided for @namedPlaceHint.
  ///
  /// In en, this message translates to:
  /// **'Name, like Home'**
  String get namedPlaceHint;

  /// No description provided for @radiusHeading.
  ///
  /// In en, this message translates to:
  /// **'Counts within'**
  String get radiusHeading;

  /// No description provided for @radiusMetres.
  ///
  /// In en, this message translates to:
  /// **'{metres} m'**
  String radiusMetres(int metres);

  /// No description provided for @deleteNamedPlaceQuestion.
  ///
  /// In en, this message translates to:
  /// **'Delete the Named Place “{name}”?'**
  String deleteNamedPlaceQuestion(String name);

  /// No description provided for @deleteNamedPlaceDetail.
  ///
  /// In en, this message translates to:
  /// **'Notes captured there show their coordinates again.'**
  String get deleteNamedPlaceDetail;

  /// No description provided for @agendaToday.
  ///
  /// In en, this message translates to:
  /// **'Today'**
  String get agendaToday;

  /// No description provided for @agendaTomorrow.
  ///
  /// In en, this message translates to:
  /// **'Tomorrow'**
  String get agendaTomorrow;

  /// No description provided for @nothingToday.
  ///
  /// In en, this message translates to:
  /// **'Nothing due today. Tap + to capture a thought.'**
  String get nothingToday;

  /// No description provided for @nothingTomorrow.
  ///
  /// In en, this message translates to:
  /// **'Nothing due tomorrow.'**
  String get nothingTomorrow;

  /// No description provided for @justCaptured.
  ///
  /// In en, this message translates to:
  /// **'Just captured'**
  String get justCaptured;

  /// No description provided for @allNotesTitle.
  ///
  /// In en, this message translates to:
  /// **'All Notes'**
  String get allNotesTitle;

  /// No description provided for @newRoutineTooltip.
  ///
  /// In en, this message translates to:
  /// **'New Routine or Reminder'**
  String get newRoutineTooltip;

  /// No description provided for @routineOrReminder.
  ///
  /// In en, this message translates to:
  /// **'Routine or Reminder'**
  String get routineOrReminder;

  /// No description provided for @routine.
  ///
  /// In en, this message translates to:
  /// **'Routine'**
  String get routine;

  /// No description provided for @reminder.
  ///
  /// In en, this message translates to:
  /// **'Reminder'**
  String get reminder;

  /// No description provided for @newRoutine.
  ///
  /// In en, this message translates to:
  /// **'New Routine'**
  String get newRoutine;

  /// No description provided for @newReminder.
  ///
  /// In en, this message translates to:
  /// **'New Reminder'**
  String get newReminder;

  /// No description provided for @routineWhatHint.
  ///
  /// In en, this message translates to:
  /// **'What is to be done?'**
  String get routineWhatHint;

  /// No description provided for @fromNote.
  ///
  /// In en, this message translates to:
  /// **'From the Note'**
  String get fromNote;

  /// No description provided for @noteGone.
  ///
  /// In en, this message translates to:
  /// **'The Note is gone'**
  String get noteGone;

  /// No description provided for @scheduleSimple.
  ///
  /// In en, this message translates to:
  /// **'Simple'**
  String get scheduleSimple;

  /// No description provided for @scheduleAdvanced.
  ///
  /// In en, this message translates to:
  /// **'Advanced'**
  String get scheduleAdvanced;

  /// No description provided for @repeatOnce.
  ///
  /// In en, this message translates to:
  /// **'Once'**
  String get repeatOnce;

  /// No description provided for @repeatEvery.
  ///
  /// In en, this message translates to:
  /// **'Every …'**
  String get repeatEvery;

  /// No description provided for @repeatWeekdays.
  ///
  /// In en, this message translates to:
  /// **'Weekdays'**
  String get repeatWeekdays;

  /// No description provided for @everyLabel.
  ///
  /// In en, this message translates to:
  /// **'Every'**
  String get everyLabel;

  /// No description provided for @fewer.
  ///
  /// In en, this message translates to:
  /// **'Fewer'**
  String get fewer;

  /// No description provided for @more.
  ///
  /// In en, this message translates to:
  /// **'More'**
  String get more;

  /// No description provided for @unitDays.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{day} other{days}}'**
  String unitDays(int count);

  /// No description provided for @unitWeeks.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{week} other{weeks}}'**
  String unitWeeks(int count);

  /// No description provided for @unitMonths.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{month} other{months}}'**
  String unitMonths(int count);

  /// No description provided for @unitYears.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{year} other{years}}'**
  String unitYears(int count);

  /// No description provided for @startsOn.
  ///
  /// In en, this message translates to:
  /// **'Starts'**
  String get startsOn;

  /// No description provided for @onDay.
  ///
  /// In en, this message translates to:
  /// **'On'**
  String get onDay;

  /// No description provided for @timesHeading.
  ///
  /// In en, this message translates to:
  /// **'Times of day'**
  String get timesHeading;

  /// No description provided for @allDay.
  ///
  /// In en, this message translates to:
  /// **'All day'**
  String get allDay;

  /// No description provided for @addTime.
  ///
  /// In en, this message translates to:
  /// **'Add a time'**
  String get addTime;

  /// No description provided for @removeTime.
  ///
  /// In en, this message translates to:
  /// **'Remove {time}'**
  String removeTime(String time);

  /// No description provided for @rruleLabel.
  ///
  /// In en, this message translates to:
  /// **'Repeat rule (RRULE)'**
  String get rruleLabel;

  /// No description provided for @rruleHelper.
  ///
  /// In en, this message translates to:
  /// **'Leave empty for once. Example: FREQ=MONTHLY;BYDAY=1MO'**
  String get rruleHelper;

  /// No description provided for @rruleInvalid.
  ///
  /// In en, this message translates to:
  /// **'Not a rule this app understands: {detail}'**
  String rruleInvalid(String detail);

  /// No description provided for @nextOccurrences.
  ///
  /// In en, this message translates to:
  /// **'Next'**
  String get nextOccurrences;

  /// No description provided for @neverDue.
  ///
  /// In en, this message translates to:
  /// **'This never comes up.'**
  String get neverDue;

  /// No description provided for @deleteRoutineQuestion.
  ///
  /// In en, this message translates to:
  /// **'Delete this Routine? It stays in the trash for 30 days. Its Note stays.'**
  String get deleteRoutineQuestion;

  /// No description provided for @deleteReminderQuestion.
  ///
  /// In en, this message translates to:
  /// **'Delete this Reminder? It stays in the trash for 30 days. Its Note stays.'**
  String get deleteReminderQuestion;

  /// No description provided for @routinesHeading.
  ///
  /// In en, this message translates to:
  /// **'Routines and Reminders'**
  String get routinesHeading;

  /// No description provided for @everyNDays.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Every day} other{Every {count} days}}'**
  String everyNDays(int count);

  /// No description provided for @everyNWeeks.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Every week} other{Every {count} weeks}}'**
  String everyNWeeks(int count);

  /// No description provided for @everyNMonths.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Every month} other{Every {count} months}}'**
  String everyNMonths(int count);

  /// No description provided for @everyNYears.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Every year} other{Every {count} years}}'**
  String everyNYears(int count);

  /// No description provided for @onWeekdays.
  ///
  /// In en, this message translates to:
  /// **'Every {days}'**
  String onWeekdays(String days);

  /// No description provided for @onceOn.
  ///
  /// In en, this message translates to:
  /// **'Once, {date}'**
  String onceOn(String date);

  /// No description provided for @ruleSummary.
  ///
  /// In en, this message translates to:
  /// **'Rule: {rule}'**
  String ruleSummary(String rule);

  /// No description provided for @markDone.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get markDone;

  /// No description provided for @notFiled.
  ///
  /// In en, this message translates to:
  /// **'Not filed'**
  String get notFiled;

  /// No description provided for @alarmChannel.
  ///
  /// In en, this message translates to:
  /// **'Routines and Reminders'**
  String get alarmChannel;

  /// No description provided for @snooze.
  ///
  /// In en, this message translates to:
  /// **'Snooze'**
  String get snooze;

  /// No description provided for @snoozeQuestion.
  ///
  /// In en, this message translates to:
  /// **'Remind me again in …'**
  String get snoozeQuestion;

  /// No description provided for @snooze10.
  ///
  /// In en, this message translates to:
  /// **'10 min'**
  String get snooze10;

  /// No description provided for @snooze30.
  ///
  /// In en, this message translates to:
  /// **'30 min'**
  String get snooze30;

  /// No description provided for @snooze60.
  ///
  /// In en, this message translates to:
  /// **'1 h'**
  String get snooze60;

  /// No description provided for @alarmsBanner.
  ///
  /// In en, this message translates to:
  /// **'Alarms may come late or not at all: a permission is missing. Tap to fix it.'**
  String get alarmsBanner;

  /// No description provided for @setupTitle.
  ///
  /// In en, this message translates to:
  /// **'Set up alarms'**
  String get setupTitle;

  /// No description provided for @setupStep.
  ///
  /// In en, this message translates to:
  /// **'Step {step} of {count}'**
  String setupStep(int step, int count);

  /// No description provided for @setupAllow.
  ///
  /// In en, this message translates to:
  /// **'Allow'**
  String get setupAllow;

  /// No description provided for @setupGranted.
  ///
  /// In en, this message translates to:
  /// **'Allowed'**
  String get setupGranted;

  /// No description provided for @setupNext.
  ///
  /// In en, this message translates to:
  /// **'Next'**
  String get setupNext;

  /// No description provided for @setupDone.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get setupDone;

  /// No description provided for @permNotifications.
  ///
  /// In en, this message translates to:
  /// **'Notifications'**
  String get permNotifications;

  /// No description provided for @permNotificationsWhy.
  ///
  /// In en, this message translates to:
  /// **'Routines and Reminders show up as notifications. Tick them off as Done or snooze them right there, even on the lock screen.'**
  String get permNotificationsWhy;

  /// No description provided for @permExactAlarms.
  ///
  /// In en, this message translates to:
  /// **'Alarms on the minute'**
  String get permExactAlarms;

  /// No description provided for @permExactAlarmsWhy.
  ///
  /// In en, this message translates to:
  /// **'Without this, Android may bring an alarm up to a quarter of an hour late. Turn it on for Old Brain Fix on the page that opens.'**
  String get permExactAlarmsWhy;

  /// No description provided for @permFullScreen.
  ///
  /// In en, this message translates to:
  /// **'Full-screen alarms'**
  String get permFullScreen;

  /// No description provided for @permFullScreenWhy.
  ///
  /// In en, this message translates to:
  /// **'Lets a Routine you keep ignoring turn into a full-screen alarm you can\'t miss, if you switch that on for it. Turn it on for Old Brain Fix on the page that opens.'**
  String get permFullScreenWhy;

  /// No description provided for @catchUpTitle.
  ///
  /// In en, this message translates to:
  /// **'Missed while the phone was off'**
  String get catchUpTitle;

  /// No description provided for @catchUpOne.
  ///
  /// In en, this message translates to:
  /// **'One alarm was Missed. Open the app to see it.'**
  String get catchUpOne;

  /// No description provided for @catchUpMany.
  ///
  /// In en, this message translates to:
  /// **'{count} alarms were Missed. Open the app to see them.'**
  String catchUpMany(String count);

  /// No description provided for @missedTitle.
  ///
  /// In en, this message translates to:
  /// **'Missed'**
  String get missedTitle;

  /// No description provided for @missedTimes.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{once} other{{count} times}}'**
  String missedTimes(int count);

  /// No description provided for @missedMore.
  ///
  /// In en, this message translates to:
  /// **'and {count} more'**
  String missedMore(int count);

  /// No description provided for @missedSeen.
  ///
  /// In en, this message translates to:
  /// **'Seen'**
  String get missedSeen;

  /// No description provided for @renagHeading.
  ///
  /// In en, this message translates to:
  /// **'Re-nag'**
  String get renagHeading;

  /// No description provided for @renagSummary.
  ///
  /// In en, this message translates to:
  /// **'Rings again every {minutes} min until Done'**
  String renagSummary(int minutes);

  /// No description provided for @renagExplain.
  ///
  /// In en, this message translates to:
  /// **'An ignored alarm rings again at this interval until it is Done or snoozed, or until the next one is due; then it counts as Missed. A Reminder stops at midnight.'**
  String get renagExplain;

  /// No description provided for @renagThis.
  ///
  /// In en, this message translates to:
  /// **'This one'**
  String get renagThis;

  /// No description provided for @renagDevice.
  ///
  /// In en, this message translates to:
  /// **'Default on this phone'**
  String get renagDevice;

  /// No description provided for @renagDeviceDefault.
  ///
  /// In en, this message translates to:
  /// **'Default ({minutes} min)'**
  String renagDeviceDefault(int minutes);

  /// No description provided for @renagMinutes.
  ///
  /// In en, this message translates to:
  /// **'{minutes} min'**
  String renagMinutes(int minutes);

  /// No description provided for @ringChannel.
  ///
  /// In en, this message translates to:
  /// **'Full-screen alarms'**
  String get ringChannel;

  /// No description provided for @escalatedChannel.
  ///
  /// In en, this message translates to:
  /// **'Escalated alarms'**
  String get escalatedChannel;

  /// No description provided for @escalationHeading.
  ///
  /// In en, this message translates to:
  /// **'Escalation'**
  String get escalationHeading;

  /// No description provided for @escalationOff.
  ///
  /// In en, this message translates to:
  /// **'Off: it only Re-nags'**
  String get escalationOff;

  /// No description provided for @escalationSummary.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Full-screen alarm after 1 ignored Re-nag} other{Full-screen alarm after {count} ignored Re-nags}}'**
  String escalationSummary(int count);

  /// No description provided for @escalationExplain.
  ///
  /// In en, this message translates to:
  /// **'When this one is ignored again and again, it turns into a full-screen alarm with sound, even over the lock screen. It rings for up to 5 minutes, then Re-nags again, until it is Done or snoozed.'**
  String get escalationExplain;

  /// No description provided for @escalationSwitch.
  ///
  /// In en, this message translates to:
  /// **'Escalate to a full-screen alarm'**
  String get escalationSwitch;

  /// No description provided for @escalationAfter.
  ///
  /// In en, this message translates to:
  /// **'After ignored Re-nags'**
  String get escalationAfter;

  /// No description provided for @nudgeHeading.
  ///
  /// In en, this message translates to:
  /// **'Remind me to sort it in'**
  String get nudgeHeading;

  /// No description provided for @unsortedBanner.
  ///
  /// In en, this message translates to:
  /// **'{count} Unsorted'**
  String unsortedBanner(int count);

  /// No description provided for @noUnsorted.
  ///
  /// In en, this message translates to:
  /// **'Nothing is Unsorted. Well done!'**
  String get noUnsorted;

  /// No description provided for @nudgeScreenTitle.
  ///
  /// In en, this message translates to:
  /// **'Waiting to be sorted'**
  String get nudgeScreenTitle;

  /// No description provided for @nudgeMore.
  ///
  /// In en, this message translates to:
  /// **'+{count}'**
  String nudgeMore(int count);

  /// No description provided for @nudgeMoreTooltip.
  ///
  /// In en, this message translates to:
  /// **'All Unsorted Notes'**
  String get nudgeMoreTooltip;

  /// No description provided for @nudgeChannel.
  ///
  /// In en, this message translates to:
  /// **'Nudges'**
  String get nudgeChannel;

  /// No description provided for @nudgeTitle.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 Note is Unsorted} other{{count} Notes are Unsorted}}'**
  String nudgeTitle(int count);

  /// No description provided for @nudgeBody.
  ///
  /// In en, this message translates to:
  /// **'Tap to sort them.'**
  String get nudgeBody;

  /// No description provided for @nudgeSettingsHeading.
  ///
  /// In en, this message translates to:
  /// **'Nudges on this phone'**
  String get nudgeSettingsHeading;

  /// No description provided for @nudgeSettingsSummary.
  ///
  /// In en, this message translates to:
  /// **'Again after {minutes} min, {quiet}'**
  String nudgeSettingsSummary(int minutes, String quiet);

  /// No description provided for @nudgeSettingsExplain.
  ///
  /// In en, this message translates to:
  /// **'While anything is Unsorted, a Nudge comes again after this long when you ignore it. In the Quiet Hours Nudges wait until they end; Routines and Reminders still ring on time.'**
  String get nudgeSettingsExplain;

  /// No description provided for @nudgeRepeat.
  ///
  /// In en, this message translates to:
  /// **'Nudge again after'**
  String get nudgeRepeat;

  /// No description provided for @quietHours.
  ///
  /// In en, this message translates to:
  /// **'Quiet Hours'**
  String get quietHours;

  /// No description provided for @quietHoursRange.
  ///
  /// In en, this message translates to:
  /// **'Quiet Hours {from} to {until}'**
  String quietHoursRange(String from, String until);

  /// No description provided for @quietHoursOff.
  ///
  /// In en, this message translates to:
  /// **'no Quiet Hours'**
  String get quietHoursOff;

  /// No description provided for @quietFrom.
  ///
  /// In en, this message translates to:
  /// **'From'**
  String get quietFrom;

  /// No description provided for @quietUntil.
  ///
  /// In en, this message translates to:
  /// **'Until'**
  String get quietUntil;

  /// No description provided for @yes.
  ///
  /// In en, this message translates to:
  /// **'Yes'**
  String get yes;

  /// No description provided for @no.
  ///
  /// In en, this message translates to:
  /// **'No'**
  String get no;

  /// No description provided for @deleteNoteRoutines.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Also delete the linked Routine?} other{Also delete the {count} linked Routines?}}'**
  String deleteNoteRoutines(int count);

  /// No description provided for @deleteNoteReminders.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Also delete the linked Reminder?} other{Also delete the {count} linked Reminders?}}'**
  String deleteNoteReminders(int count);

  /// No description provided for @deleteNoteRoutinesAndReminders.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, other{Also delete the {count} linked Routines and Reminders?}}'**
  String deleteNoteRoutinesAndReminders(int count);

  /// No description provided for @deleteNoteLinkedDetail.
  ///
  /// In en, this message translates to:
  /// **'Deleted ones come back with the Note when you restore it. Kept ones go on reminding you.'**
  String get deleteNoteLinkedDetail;

  /// No description provided for @noteInTrash.
  ///
  /// In en, this message translates to:
  /// **'The Note is in the trash'**
  String get noteInTrash;

  /// No description provided for @noteDeleted.
  ///
  /// In en, this message translates to:
  /// **'The Note was deleted'**
  String get noteDeleted;

  /// No description provided for @allowAccess.
  ///
  /// In en, this message translates to:
  /// **'Allow'**
  String get allowAccess;

  /// No description provided for @captureContextTitle.
  ///
  /// In en, this message translates to:
  /// **'Capture Context'**
  String get captureContextTitle;

  /// No description provided for @captureContextIntro.
  ///
  /// In en, this message translates to:
  /// **'Besides when and where, a Note can record more of what was going on. Each of these is off until you switch it on, and stays in the Note on your own devices.'**
  String get captureContextIntro;

  /// No description provided for @calendarContextSwitch.
  ///
  /// In en, this message translates to:
  /// **'Calendar event'**
  String get calendarContextSwitch;

  /// No description provided for @calendarContextDetail.
  ///
  /// In en, this message translates to:
  /// **'The event running when a Note is captured, like “during: Dentist”.'**
  String get calendarContextDetail;

  /// No description provided for @bluetoothContextSwitch.
  ///
  /// In en, this message translates to:
  /// **'Bluetooth device'**
  String get bluetoothContextSwitch;

  /// No description provided for @bluetoothContextDetail.
  ///
  /// In en, this message translates to:
  /// **'The device connected when a Note is captured, like your car or headphones.'**
  String get bluetoothContextDetail;

  /// No description provided for @calendarExplainTitle.
  ///
  /// In en, this message translates to:
  /// **'Remember your appointment?'**
  String get calendarExplainTitle;

  /// No description provided for @calendarExplainBody.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix can note the calendar event running when you capture a Note, so it says “during: Dentist” days later. It reads your calendar only at that moment. The event stays in the Note on your own devices and is never sent anywhere else.'**
  String get calendarExplainBody;

  /// No description provided for @bluetoothExplainTitle.
  ///
  /// In en, this message translates to:
  /// **'Remember what was connected?'**
  String get bluetoothExplainTitle;

  /// No description provided for @bluetoothExplainBody.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix can note the Bluetooth device connected when you capture a Note, like your car or your headphones. Android calls this “Nearby devices”; Old Brain Fix only reads the names of connected devices and never searches for others. The name stays in the Note on your own devices and is never sent anywhere else.'**
  String get bluetoothExplainBody;

  /// No description provided for @calendarNotAllowed.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix may not read your calendar. You can allow it in the system settings.'**
  String get calendarNotAllowed;

  /// No description provided for @bluetoothNotAllowed.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix may not see your Bluetooth devices. You can allow it in the system settings.'**
  String get bluetoothNotAllowed;

  /// No description provided for @contextSourceApp.
  ///
  /// In en, this message translates to:
  /// **'Shared from {app}'**
  String contextSourceApp(String app);

  /// No description provided for @contextDuring.
  ///
  /// In en, this message translates to:
  /// **'During: {events}'**
  String contextDuring(String events);

  /// No description provided for @contextBluetooth.
  ///
  /// In en, this message translates to:
  /// **'Connected: {devices}'**
  String contextBluetooth(String devices);

  /// No description provided for @moreCaptureContext.
  ///
  /// In en, this message translates to:
  /// **'More about the capture'**
  String get moreCaptureContext;

  /// No description provided for @recordVoice.
  ///
  /// In en, this message translates to:
  /// **'Record voice'**
  String get recordVoice;

  /// No description provided for @recording.
  ///
  /// In en, this message translates to:
  /// **'Recording…'**
  String get recording;

  /// No description provided for @stopAndSave.
  ///
  /// In en, this message translates to:
  /// **'Stop and save'**
  String get stopAndSave;

  /// No description provided for @voiceRecording.
  ///
  /// In en, this message translates to:
  /// **'Voice recording'**
  String get voiceRecording;

  /// No description provided for @playRecording.
  ///
  /// In en, this message translates to:
  /// **'Play the voice recording'**
  String get playRecording;

  /// No description provided for @stopPlaying.
  ///
  /// In en, this message translates to:
  /// **'Stop'**
  String get stopPlaying;

  /// No description provided for @microphoneExplainTitle.
  ///
  /// In en, this message translates to:
  /// **'Use the microphone?'**
  String get microphoneExplainTitle;

  /// No description provided for @microphoneExplainBody.
  ///
  /// In en, this message translates to:
  /// **'Voice Notes record what you say, so a thought is kept before it is gone. Old Brain Fix uses the microphone only while you record. The recording stays in the Note on your own devices and is never sent anywhere else.'**
  String get microphoneExplainBody;

  /// No description provided for @microphoneMissing.
  ///
  /// In en, this message translates to:
  /// **'Voice Notes need the microphone.'**
  String get microphoneMissing;

  /// No description provided for @typeInstead.
  ///
  /// In en, this message translates to:
  /// **'Type instead'**
  String get typeInstead;
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['de', 'en'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'de':
      return AppLocalizationsDe();
    case 'en':
      return AppLocalizationsEn();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
