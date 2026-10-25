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

  /// No description provided for @widgetNothingToday.
  ///
  /// In en, this message translates to:
  /// **'Nothing due today.'**
  String get widgetNothingToday;

  /// No description provided for @widgetMore.
  ///
  /// In en, this message translates to:
  /// **'and {count} more'**
  String widgetMore(String count);

  /// No description provided for @speechModelTitle.
  ///
  /// In en, this message translates to:
  /// **'Speech model'**
  String get speechModelTitle;

  /// No description provided for @speechModelIntro.
  ///
  /// In en, this message translates to:
  /// **'Voice Notes get a Transcript you can read and search. Whisper writes it on this phone, in English or German, with a speech model it needs once. The recording is always kept.'**
  String get speechModelIntro;

  /// No description provided for @speechModelSource.
  ///
  /// In en, this message translates to:
  /// **'The download is about {megabytes} MB, comes only from Old Brain Fix\'s own release on GitHub, and is checked against a fixed checksum before it is used. Wi-Fi is best. Nothing else is ever sent or fetched.'**
  String speechModelSource(int megabytes);

  /// Credit for the downloaded Whisper model on the speech model page.
  ///
  /// In en, this message translates to:
  /// **'The speech model is the base model of Whisper by OpenAI, Copyright (c) 2022 OpenAI, under the MIT License.'**
  String get speechModelCredit;

  /// No description provided for @speechModelChecking.
  ///
  /// In en, this message translates to:
  /// **'Checking the speech model…'**
  String get speechModelChecking;

  /// No description provided for @speechModelUnsupported.
  ///
  /// In en, this message translates to:
  /// **'Transcripts are not available on this device. Voice recordings are kept as they are.'**
  String get speechModelUnsupported;

  /// No description provided for @speechModelReady.
  ///
  /// In en, this message translates to:
  /// **'The speech model is ready. Voice Notes are transcribed on this phone.'**
  String get speechModelReady;

  /// No description provided for @speechModelMissing.
  ///
  /// In en, this message translates to:
  /// **'No speech model yet'**
  String get speechModelMissing;

  /// No description provided for @speechModelDownload.
  ///
  /// In en, this message translates to:
  /// **'Download ({megabytes} MB)'**
  String speechModelDownload(int megabytes);

  /// No description provided for @speechModelPick.
  ///
  /// In en, this message translates to:
  /// **'Use a file of my own'**
  String get speechModelPick;

  /// No description provided for @speechModelPickDetail.
  ///
  /// In en, this message translates to:
  /// **'It must be whisper.cpp\'s multilingual ggml-base.bin; any other file is rejected.'**
  String get speechModelPickDetail;

  /// No description provided for @speechModelDownloading.
  ///
  /// In en, this message translates to:
  /// **'Downloading: {received} of {total} MB'**
  String speechModelDownloading(String received, String total);

  /// No description provided for @speechModelFailed.
  ///
  /// In en, this message translates to:
  /// **'The download did not finish, and nothing was kept. Try again later.'**
  String get speechModelFailed;

  /// No description provided for @speechModelMismatch.
  ///
  /// In en, this message translates to:
  /// **'That is not the expected speech model, so it was not kept.'**
  String get speechModelMismatch;

  /// No description provided for @transcribing.
  ///
  /// In en, this message translates to:
  /// **'Transcribing…'**
  String get transcribing;

  /// No description provided for @noWordsHeard.
  ///
  /// In en, this message translates to:
  /// **'No words heard.'**
  String get noWordsHeard;

  /// No description provided for @historyTitle.
  ///
  /// In en, this message translates to:
  /// **'History'**
  String get historyTitle;

  /// No description provided for @streakHeading.
  ///
  /// In en, this message translates to:
  /// **'Streak'**
  String get streakHeading;

  /// No description provided for @streakNow.
  ///
  /// In en, this message translates to:
  /// **'Now'**
  String get streakNow;

  /// No description provided for @streakLongest.
  ///
  /// In en, this message translates to:
  /// **'Longest'**
  String get streakLongest;

  /// No description provided for @lastSevenDays.
  ///
  /// In en, this message translates to:
  /// **'Last 7 days'**
  String get lastSevenDays;

  /// No description provided for @dayAllDone.
  ///
  /// In en, this message translates to:
  /// **'{day}: all Done'**
  String dayAllDone(String day);

  /// No description provided for @dayPartlyDone.
  ///
  /// In en, this message translates to:
  /// **'{day}: partly Done, partly Missed'**
  String dayPartlyDone(String day);

  /// No description provided for @dayMissed.
  ///
  /// In en, this message translates to:
  /// **'{day}: Missed'**
  String dayMissed(String day);

  /// No description provided for @dayOpen.
  ///
  /// In en, this message translates to:
  /// **'{day}: still due'**
  String dayOpen(String day);

  /// No description provided for @dayNothing.
  ///
  /// In en, this message translates to:
  /// **'{day}: nothing due'**
  String dayNothing(String day);

  /// No description provided for @adherenceHeading.
  ///
  /// In en, this message translates to:
  /// **'Adherence'**
  String get adherenceHeading;

  /// No description provided for @adherenceThisWeek.
  ///
  /// In en, this message translates to:
  /// **'This week'**
  String get adherenceThisWeek;

  /// No description provided for @adherenceThisMonth.
  ///
  /// In en, this message translates to:
  /// **'This month'**
  String get adherenceThisMonth;

  /// No description provided for @adherenceLast30Days.
  ///
  /// In en, this message translates to:
  /// **'Last 30 days'**
  String get adherenceLast30Days;

  /// No description provided for @adherenceOf.
  ///
  /// In en, this message translates to:
  /// **'{done} of {total} Done'**
  String adherenceOf(int done, int total);

  /// No description provided for @adherenceNone.
  ///
  /// In en, this message translates to:
  /// **'Nothing yet'**
  String get adherenceNone;

  /// No description provided for @showStreaks.
  ///
  /// In en, this message translates to:
  /// **'Show Streaks on this phone'**
  String get showStreaks;

  /// No description provided for @showStreaksDetail.
  ///
  /// In en, this message translates to:
  /// **'Adherence shows either way.'**
  String get showStreaksDetail;

  /// No description provided for @historyLog.
  ///
  /// In en, this message translates to:
  /// **'Log'**
  String get historyLog;

  /// No description provided for @historyEmpty.
  ///
  /// In en, this message translates to:
  /// **'Nothing was due yet.'**
  String get historyEmpty;

  /// No description provided for @snoozesHere.
  ///
  /// In en, this message translates to:
  /// **'Snoozes show only as this phone recorded them, for about the last month.'**
  String get snoozesHere;

  /// No description provided for @logDone.
  ///
  /// In en, this message translates to:
  /// **'Done: {time}'**
  String logDone(String time);

  /// No description provided for @logDoneEarly.
  ///
  /// In en, this message translates to:
  /// **'Done early: {time}'**
  String logDoneEarly(String time);

  /// No description provided for @logDoneLate.
  ///
  /// In en, this message translates to:
  /// **'Done late: {time}'**
  String logDoneLate(String time);

  /// No description provided for @logMissed.
  ///
  /// In en, this message translates to:
  /// **'Missed'**
  String get logMissed;

  /// No description provided for @logOpen.
  ///
  /// In en, this message translates to:
  /// **'Due, not Done yet'**
  String get logOpen;

  /// No description provided for @logComing.
  ///
  /// In en, this message translates to:
  /// **'Still to come'**
  String get logComing;

  /// No description provided for @logSnoozed.
  ///
  /// In en, this message translates to:
  /// **'Snoozed until {time}'**
  String logSnoozed(String time);

  /// No description provided for @calendarTooltip.
  ///
  /// In en, this message translates to:
  /// **'Calendar'**
  String get calendarTooltip;

  /// No description provided for @addNoteToCalendar.
  ///
  /// In en, this message translates to:
  /// **'Add to calendar at a time you pick'**
  String get addNoteToCalendar;

  /// No description provided for @addToCalendarAt.
  ///
  /// In en, this message translates to:
  /// **'Add to calendar: {when}'**
  String addToCalendarAt(String when);

  /// No description provided for @shareCalendarFile.
  ///
  /// In en, this message translates to:
  /// **'Share as calendar file'**
  String get shareCalendarFile;

  /// No description provided for @noCalendarApp.
  ///
  /// In en, this message translates to:
  /// **'There is no calendar app to add it to.'**
  String get noCalendarApp;

  /// No description provided for @nothingTakesCalendarFile.
  ///
  /// In en, this message translates to:
  /// **'No app can take the calendar file.'**
  String get nothingTakesCalendarFile;

  /// No description provided for @calendarExportTooltip.
  ///
  /// In en, this message translates to:
  /// **'Export to calendar'**
  String get calendarExportTooltip;

  /// No description provided for @calendarExportTitle.
  ///
  /// In en, this message translates to:
  /// **'Calendar export'**
  String get calendarExportTitle;

  /// No description provided for @calendarExportAll.
  ///
  /// In en, this message translates to:
  /// **'All Routines and Reminders'**
  String get calendarExportAll;

  /// No description provided for @calendarExportEmpty.
  ///
  /// In en, this message translates to:
  /// **'No Routines or Reminders yet.'**
  String get calendarExportEmpty;

  /// No description provided for @calendarMirrorSwitch.
  ///
  /// In en, this message translates to:
  /// **'Keep in a calendar'**
  String get calendarMirrorSwitch;

  /// No description provided for @calendarMirrorDetail.
  ///
  /// In en, this message translates to:
  /// **'Your Routines and Reminders appear in a calendar of your choice on this device and follow every change.'**
  String get calendarMirrorDetail;

  /// No description provided for @calendarMirrorIn.
  ///
  /// In en, this message translates to:
  /// **'In “{calendar}”. Tap to choose another calendar.'**
  String calendarMirrorIn(String calendar);

  /// No description provided for @calendarMirrorExplainTitle.
  ///
  /// In en, this message translates to:
  /// **'Keep them in your calendar?'**
  String get calendarMirrorExplainTitle;

  /// No description provided for @calendarMirrorExplainBody.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix can write your Routines and Reminders into a calendar you choose and keep them up to date. It only ever changes or removes the events it wrote itself, and turning this off removes them. Wherever that calendar syncs to, they show up too.'**
  String get calendarMirrorExplainBody;

  /// No description provided for @calendarMirrorNotAllowed.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix may not write to your calendar. You can allow it in the system settings.'**
  String get calendarMirrorNotAllowed;

  /// No description provided for @calendarMirrorNoCalendar.
  ///
  /// In en, this message translates to:
  /// **'There is no calendar on this device to write into.'**
  String get calendarMirrorNoCalendar;

  /// No description provided for @calendarMirrorNotRemoved.
  ///
  /// In en, this message translates to:
  /// **'The events could not be removed from the calendar. Allow Old Brain Fix to use your calendar and turn this off again.'**
  String get calendarMirrorNotRemoved;

  /// No description provided for @settingsTitle.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get settingsTitle;

  /// No description provided for @settingsIntro.
  ///
  /// In en, this message translates to:
  /// **'These settings belong to this phone and are never synced.'**
  String get settingsIntro;

  /// No description provided for @settingsLook.
  ///
  /// In en, this message translates to:
  /// **'Look'**
  String get settingsLook;

  /// No description provided for @themeSystem.
  ///
  /// In en, this message translates to:
  /// **'System'**
  String get themeSystem;

  /// No description provided for @themeLight.
  ///
  /// In en, this message translates to:
  /// **'Light'**
  String get themeLight;

  /// No description provided for @themeDark.
  ///
  /// In en, this message translates to:
  /// **'Dark'**
  String get themeDark;

  /// No description provided for @settingsAlarms.
  ///
  /// In en, this message translates to:
  /// **'Alarms'**
  String get settingsAlarms;

  /// No description provided for @settingsAlarmSetupDetail.
  ///
  /// In en, this message translates to:
  /// **'The permissions alarms need to ring on time.'**
  String get settingsAlarmSetupDetail;

  /// No description provided for @settingsRenagDetail.
  ///
  /// In en, this message translates to:
  /// **'How often an ignored alarm rings again, unless its Routine has its own interval.'**
  String get settingsRenagDetail;

  /// No description provided for @settingsNudges.
  ///
  /// In en, this message translates to:
  /// **'Nudges'**
  String get settingsNudges;

  /// No description provided for @settingsCapture.
  ///
  /// In en, this message translates to:
  /// **'Capture'**
  String get settingsCapture;

  /// No description provided for @settingsNamedPlacesDetail.
  ///
  /// In en, this message translates to:
  /// **'“Home” or “Work” instead of coordinates. Kept with your Notes, not just on this phone.'**
  String get settingsNamedPlacesDetail;

  /// No description provided for @settingsCaptureContextDetail.
  ///
  /// In en, this message translates to:
  /// **'Calendar event and Bluetooth device with each Note.'**
  String get settingsCaptureContextDetail;

  /// No description provided for @settingsSpeechModelDetail.
  ///
  /// In en, this message translates to:
  /// **'For Transcripts of voice Notes.'**
  String get settingsSpeechModelDetail;

  /// No description provided for @settingsCalendar.
  ///
  /// In en, this message translates to:
  /// **'Calendar'**
  String get settingsCalendar;

  /// No description provided for @settingsHistory.
  ///
  /// In en, this message translates to:
  /// **'History'**
  String get settingsHistory;

  /// No description provided for @aboutTitle.
  ///
  /// In en, this message translates to:
  /// **'About Old Brain Fix'**
  String get aboutTitle;

  /// No description provided for @aboutTagline.
  ///
  /// In en, this message translates to:
  /// **'Capture a thought in one tap. Get nagged until it is sorted or done.'**
  String get aboutTagline;

  /// No description provided for @versionLabel.
  ///
  /// In en, this message translates to:
  /// **'Version {version} ({build})'**
  String versionLabel(String version, String build);

  /// No description provided for @documentation.
  ///
  /// In en, this message translates to:
  /// **'Documentation'**
  String get documentation;

  /// About: opens the privacy policy on the documentation site.
  ///
  /// In en, this message translates to:
  /// **'Privacy policy'**
  String get privacyPolicy;

  /// No description provided for @sourceCode.
  ///
  /// In en, this message translates to:
  /// **'Source code'**
  String get sourceCode;

  /// No description provided for @reportProblem.
  ///
  /// In en, this message translates to:
  /// **'Report a problem or an idea'**
  String get reportProblem;

  /// No description provided for @githubIssues.
  ///
  /// In en, this message translates to:
  /// **'GitHub issues'**
  String get githubIssues;

  /// No description provided for @writeTheDeveloper.
  ///
  /// In en, this message translates to:
  /// **'Write to the developer'**
  String get writeTheDeveloper;

  /// No description provided for @buyCoffee.
  ///
  /// In en, this message translates to:
  /// **'Buy me a coffee'**
  String get buyCoffee;

  /// No description provided for @coffeeSubtitle.
  ///
  /// In en, this message translates to:
  /// **'On Ko-fi, if Old Brain Fix helps you'**
  String get coffeeSubtitle;

  /// No description provided for @openSourceLicenses.
  ///
  /// In en, this message translates to:
  /// **'Open-source licenses'**
  String get openSourceLicenses;

  /// No description provided for @settingsPrivacy.
  ///
  /// In en, this message translates to:
  /// **'Privacy'**
  String get settingsPrivacy;

  /// No description provided for @appLock.
  ///
  /// In en, this message translates to:
  /// **'App Lock'**
  String get appLock;

  /// No description provided for @appLockDetail.
  ///
  /// In en, this message translates to:
  /// **'Asks for the PIN before showing your Notes. Capture and alarms never need it.'**
  String get appLockDetail;

  /// No description provided for @appLockOff.
  ///
  /// In en, this message translates to:
  /// **'Turn off the App Lock'**
  String get appLockOff;

  /// No description provided for @unlockApp.
  ///
  /// In en, this message translates to:
  /// **'Unlock Old Brain Fix'**
  String get unlockApp;

  /// No description provided for @lockedTitle.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix is locked'**
  String get lockedTitle;

  /// No description provided for @lockedExplain.
  ///
  /// In en, this message translates to:
  /// **'Unlock to see your Notes. You can capture with + right away.'**
  String get lockedExplain;

  /// No description provided for @unlock.
  ///
  /// In en, this message translates to:
  /// **'Unlock'**
  String get unlock;

  /// No description provided for @unlockBiometrics.
  ///
  /// In en, this message translates to:
  /// **'Unlock with fingerprint or screen lock'**
  String get unlockBiometrics;

  /// No description provided for @biometricsDetail.
  ///
  /// In en, this message translates to:
  /// **'The PIN always works as well.'**
  String get biometricsDetail;

  /// No description provided for @pinLabel.
  ///
  /// In en, this message translates to:
  /// **'PIN'**
  String get pinLabel;

  /// No description provided for @pinAgainLabel.
  ///
  /// In en, this message translates to:
  /// **'PIN again'**
  String get pinAgainLabel;

  /// No description provided for @pinWrong.
  ///
  /// In en, this message translates to:
  /// **'Wrong PIN'**
  String get pinWrong;

  /// No description provided for @pinTooShort.
  ///
  /// In en, this message translates to:
  /// **'At least {count} digits'**
  String pinTooShort(int count);

  /// No description provided for @pinMismatch.
  ///
  /// In en, this message translates to:
  /// **'The two PINs differ'**
  String get pinMismatch;

  /// No description provided for @pinNewTitle.
  ///
  /// In en, this message translates to:
  /// **'New PIN'**
  String get pinNewTitle;

  /// No description provided for @pinNewExplain.
  ///
  /// In en, this message translates to:
  /// **'It unlocks the App Lock and the Area Locks on this phone.'**
  String get pinNewExplain;

  /// No description provided for @pinChange.
  ///
  /// In en, this message translates to:
  /// **'Change PIN'**
  String get pinChange;

  /// No description provided for @relockAfter.
  ///
  /// In en, this message translates to:
  /// **'Lock again after'**
  String get relockAfter;

  /// No description provided for @relockDetail.
  ///
  /// In en, this message translates to:
  /// **'How long the app may be in the background before it locks again.'**
  String get relockDetail;

  /// No description provided for @relockAtOnce.
  ///
  /// In en, this message translates to:
  /// **'At once'**
  String get relockAtOnce;

  /// No description provided for @areaLocksTitle.
  ///
  /// In en, this message translates to:
  /// **'Area Locks'**
  String get areaLocksTitle;

  /// No description provided for @areaLocksDetail.
  ///
  /// In en, this message translates to:
  /// **'Areas that stay locked even while the app is open.'**
  String get areaLocksDetail;

  /// No description provided for @areaLocksExplain.
  ///
  /// In en, this message translates to:
  /// **'A locked Area and its Notes stay hidden until you unlock it. Its alarms show Masked, “Private Routine, 08:00”, unless you choose to show them in full; they can be checked off either way.'**
  String get areaLocksExplain;

  /// No description provided for @areaLocked.
  ///
  /// In en, this message translates to:
  /// **'Locked'**
  String get areaLocked;

  /// No description provided for @unlockArea.
  ///
  /// In en, this message translates to:
  /// **'Unlock {area}'**
  String unlockArea(String area);

  /// No description provided for @areaLockOff.
  ///
  /// In en, this message translates to:
  /// **'Unlock {area} for good'**
  String areaLockOff(String area);

  /// No description provided for @areaShowFull.
  ///
  /// In en, this message translates to:
  /// **'Show the alarms of {area} in full'**
  String areaShowFull(String area);

  /// No description provided for @areaAlarmsShow.
  ///
  /// In en, this message translates to:
  /// **'Alarms and Agenda show'**
  String get areaAlarmsShow;

  /// No description provided for @areaAlarmsMasked.
  ///
  /// In en, this message translates to:
  /// **'Masked'**
  String get areaAlarmsMasked;

  /// No description provided for @areaAlarmsFull.
  ///
  /// In en, this message translates to:
  /// **'In full'**
  String get areaAlarmsFull;

  /// No description provided for @maskedRoutine.
  ///
  /// In en, this message translates to:
  /// **'Private Routine'**
  String get maskedRoutine;

  /// No description provided for @maskedRoutineAt.
  ///
  /// In en, this message translates to:
  /// **'Private Routine, {time}'**
  String maskedRoutineAt(String time);

  /// No description provided for @maskedReminder.
  ///
  /// In en, this message translates to:
  /// **'Private Reminder'**
  String get maskedReminder;

  /// No description provided for @syncTitle.
  ///
  /// In en, this message translates to:
  /// **'Sync'**
  String get syncTitle;

  /// No description provided for @syncSettingsEntry.
  ///
  /// In en, this message translates to:
  /// **'Sync with another Device'**
  String get syncSettingsEntry;

  /// No description provided for @syncSettingsDetail.
  ///
  /// In en, this message translates to:
  /// **'Pair once by code; both then hold the same Notes, Areas and Routines.'**
  String get syncSettingsDetail;

  /// No description provided for @syncJoinIntro.
  ///
  /// In en, this message translates to:
  /// **'Open Sync on the computer. Scan the code it shows, or type it in.'**
  String get syncJoinIntro;

  /// No description provided for @syncScan.
  ///
  /// In en, this message translates to:
  /// **'Scan the code'**
  String get syncScan;

  /// No description provided for @syncCodeLabel.
  ///
  /// In en, this message translates to:
  /// **'Pair code'**
  String get syncCodeLabel;

  /// No description provided for @syncJoin.
  ///
  /// In en, this message translates to:
  /// **'Sync now'**
  String get syncJoin;

  /// No description provided for @syncSyncing.
  ///
  /// In en, this message translates to:
  /// **'Syncing…'**
  String get syncSyncing;

  /// No description provided for @syncInvalidCode.
  ///
  /// In en, this message translates to:
  /// **'That is not a pair code. Check it against the computer\'s screen.'**
  String get syncInvalidCode;

  /// No description provided for @syncNoCode.
  ///
  /// In en, this message translates to:
  /// **'No code found in the photo. Try again a little closer, or type the code.'**
  String get syncNoCode;

  /// No description provided for @syncCameIn.
  ///
  /// In en, this message translates to:
  /// **'Synced with {name}: {count, plural, =0{nothing new came in} =1{1 change came in} other{{count} changes came in}}.'**
  String syncCameIn(String name, int count);

  /// No description provided for @syncRefused.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 change was refused: its signature did not match.} other{{count} changes were refused: their signatures did not match.}}'**
  String syncRefused(int count);

  /// No description provided for @syncFailNotPrivate.
  ///
  /// In en, this message translates to:
  /// **'This code points outside your home network. Only Devices on the same network can sync.'**
  String get syncFailNotPrivate;

  /// No description provided for @syncFailUnreachable.
  ///
  /// In en, this message translates to:
  /// **'The computer did not answer. Is Sync open on it, and are both on the same Wi-Fi?'**
  String get syncFailUnreachable;

  /// No description provided for @syncFailWrongHost.
  ///
  /// In en, this message translates to:
  /// **'This is not the computer the code belongs to.'**
  String get syncFailWrongHost;

  /// No description provided for @syncFailWrongPin.
  ///
  /// In en, this message translates to:
  /// **'The PIN does not match. Type the code again.'**
  String get syncFailWrongPin;

  /// No description provided for @syncFailLocked.
  ///
  /// In en, this message translates to:
  /// **'Too many wrong tries. Open Sync on the computer again for a new code.'**
  String get syncFailLocked;

  /// No description provided for @syncFailDeclined.
  ///
  /// In en, this message translates to:
  /// **'The computer said no to this Device.'**
  String get syncFailDeclined;

  /// No description provided for @syncFailFormat.
  ///
  /// In en, this message translates to:
  /// **'The other Device runs another version of Old Brain Fix. Update both.'**
  String get syncFailFormat;

  /// No description provided for @syncFailFailed.
  ///
  /// In en, this message translates to:
  /// **'Sync broke off. Nothing was lost; try again.'**
  String get syncFailFailed;

  /// No description provided for @pairedDevicesTitle.
  ///
  /// In en, this message translates to:
  /// **'Paired Devices'**
  String get pairedDevicesTitle;

  /// No description provided for @pairedNone.
  ///
  /// In en, this message translates to:
  /// **'No paired Devices yet.'**
  String get pairedNone;

  /// No description provided for @pairedLastSync.
  ///
  /// In en, this message translates to:
  /// **'Last synced {when}'**
  String pairedLastSync(String when);

  /// No description provided for @forgetDevice.
  ///
  /// In en, this message translates to:
  /// **'Forget'**
  String get forgetDevice;

  /// No description provided for @syncSettingsStay.
  ///
  /// In en, this message translates to:
  /// **'Settings stay on each Device and are never synced.'**
  String get syncSettingsStay;

  /// The switch per paired Device: it syncs on its own on the same network.
  ///
  /// In en, this message translates to:
  /// **'Sync automatically'**
  String get syncAutomatically;

  /// No description provided for @syncAutoIntro.
  ///
  /// In en, this message translates to:
  /// **'Paired computers on the same Wi-Fi sync on their own, while the app is open and every quarter of an hour in the background.'**
  String get syncAutoIntro;

  /// No description provided for @conflictsTitle.
  ///
  /// In en, this message translates to:
  /// **'Conflicts'**
  String get conflictsTitle;

  /// No description provided for @conflictsEntry.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =0{No conflicts} =1{1 conflict to settle} other{{count} conflicts to settle}}'**
  String conflictsEntry(int count);

  /// No description provided for @conflictsIntro.
  ///
  /// In en, this message translates to:
  /// **'Two Devices changed the same thing before they synced. Tap the version that stays; until then the newer one shows.'**
  String get conflictsIntro;

  /// No description provided for @conflictsNone.
  ///
  /// In en, this message translates to:
  /// **'No conflicts. Every Device agrees.'**
  String get conflictsNone;

  /// No description provided for @syncConflicts.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 thing was changed on both Devices. Tap to pick which version stays.} other{{count} things were changed on both Devices. Tap to pick which versions stay.}}'**
  String syncConflicts(int count);

  /// No description provided for @conflictNoteText.
  ///
  /// In en, this message translates to:
  /// **'Note text'**
  String get conflictNoteText;

  /// No description provided for @conflictSectionText.
  ///
  /// In en, this message translates to:
  /// **'Text added to a Note'**
  String get conflictSectionText;

  /// No description provided for @conflictAreaName.
  ///
  /// In en, this message translates to:
  /// **'Area name'**
  String get conflictAreaName;

  /// No description provided for @conflictTopicName.
  ///
  /// In en, this message translates to:
  /// **'Topic name'**
  String get conflictTopicName;

  /// No description provided for @conflictPlaceName.
  ///
  /// In en, this message translates to:
  /// **'Named Place name'**
  String get conflictPlaceName;

  /// No description provided for @conflictPlaceRadius.
  ///
  /// In en, this message translates to:
  /// **'Named Place radius'**
  String get conflictPlaceRadius;

  /// No description provided for @conflictSchedule.
  ///
  /// In en, this message translates to:
  /// **'Routine schedule'**
  String get conflictSchedule;

  /// No description provided for @conflictOther.
  ///
  /// In en, this message translates to:
  /// **'Change'**
  String get conflictOther;

  /// No description provided for @conflictEmpty.
  ///
  /// In en, this message translates to:
  /// **'(empty)'**
  String get conflictEmpty;

  /// No description provided for @conflictOnThisDevice.
  ///
  /// In en, this message translates to:
  /// **'On this Device, {when}'**
  String conflictOnThisDevice(String when);

  /// No description provided for @conflictOnDevice.
  ///
  /// In en, this message translates to:
  /// **'On {device}, {when}'**
  String conflictOnDevice(String device, String when);

  /// No description provided for @conflictOnUnnamed.
  ///
  /// In en, this message translates to:
  /// **'On a Device without a name, {when}'**
  String conflictOnUnnamed(String when);

  /// No description provided for @conflictKeep.
  ///
  /// In en, this message translates to:
  /// **'Keep this version'**
  String get conflictKeep;

  /// No description provided for @folderTitle.
  ///
  /// In en, this message translates to:
  /// **'Shared folder'**
  String get folderTitle;

  /// No description provided for @folderIntro.
  ///
  /// In en, this message translates to:
  /// **'Syncs through a folder that Syncthing, Nextcloud or a USB stick carries between your Devices, wherever they are. Everything in it is encrypted: only Devices paired with this one can read it.'**
  String get folderIntro;

  /// No description provided for @folderPick.
  ///
  /// In en, this message translates to:
  /// **'Choose folder'**
  String get folderPick;

  /// Stops syncing through the shared folder; the folder itself stays as it is.
  ///
  /// In en, this message translates to:
  /// **'Forget'**
  String get folderForget;

  /// No description provided for @folderNotYet.
  ///
  /// In en, this message translates to:
  /// **'Not synced yet'**
  String get folderNotYet;

  /// No description provided for @folderSynced.
  ///
  /// In en, this message translates to:
  /// **'Last synced {when}: {count, plural, =0{nothing new} =1{1 change came in} other{{count} changes came in}}'**
  String folderSynced(String when, int count);

  /// No description provided for @folderUnreachable.
  ///
  /// In en, this message translates to:
  /// **'The folder could not be reached at {when}. It is tried again later.'**
  String folderUnreachable(String when);

  /// No description provided for @folderUnreadable.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 file is from a Device this one is not paired with. Pair them first.} other{{count} files are from Devices this one is not paired with. Pair them first.}}'**
  String folderUnreadable(int count);

  /// No description provided for @bundleTitle.
  ///
  /// In en, this message translates to:
  /// **'Sync bundle'**
  String get bundleTitle;

  /// No description provided for @bundleIntro.
  ///
  /// In en, this message translates to:
  /// **'Sends what is new as one file through any messenger or mail, and the other Device imports it. Everything in it is encrypted: only Devices paired with this one can read it.'**
  String get bundleIntro;

  /// Label of the choice which paired Device a bundle is written for.
  ///
  /// In en, this message translates to:
  /// **'For'**
  String get bundleFor;

  /// The bundle carries the whole log, for a Device that holds nothing yet.
  ///
  /// In en, this message translates to:
  /// **'Everything'**
  String get bundleEverything;

  /// No description provided for @bundleSend.
  ///
  /// In en, this message translates to:
  /// **'Send bundle'**
  String get bundleSend;

  /// No description provided for @bundleImport.
  ///
  /// In en, this message translates to:
  /// **'Import bundle'**
  String get bundleImport;

  /// No description provided for @bundleNothingNew.
  ///
  /// In en, this message translates to:
  /// **'Nothing new to send to {name}.'**
  String bundleNothingNew(String name);

  /// No description provided for @bundleCameIn.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =0{Nothing new came in with the bundle.} =1{1 change came in with the bundle.} other{{count} changes came in with the bundle.}}'**
  String bundleCameIn(int count);

  /// No description provided for @bundlePairFirst.
  ///
  /// In en, this message translates to:
  /// **'This bundle is from a Device this one is not paired with, so nothing in it can be read. Pair them first, then import it again.'**
  String get bundlePairFirst;

  /// No description provided for @bundleNotABundle.
  ///
  /// In en, this message translates to:
  /// **'This file is no sync bundle this app can read.'**
  String get bundleNotABundle;

  /// No description provided for @bundleFailed.
  ///
  /// In en, this message translates to:
  /// **'The bundle did not work out. Nothing was lost; try again.'**
  String get bundleFailed;

  /// No description provided for @bundleNoApp.
  ///
  /// In en, this message translates to:
  /// **'No app here can take the bundle.'**
  String get bundleNoApp;

  /// No description provided for @shareArea.
  ///
  /// In en, this message translates to:
  /// **'Share…'**
  String get shareArea;

  /// No description provided for @areaShared.
  ///
  /// In en, this message translates to:
  /// **'Shared'**
  String get areaShared;

  /// No description provided for @peopleTitle.
  ///
  /// In en, this message translates to:
  /// **'People'**
  String get peopleTitle;

  /// No description provided for @shareTitle.
  ///
  /// In en, this message translates to:
  /// **'Share {area}'**
  String shareTitle(String area);

  /// No description provided for @shareIntro.
  ///
  /// In en, this message translates to:
  /// **'Meet the other person. On their computer, open Sync and show the code; scan it or type it here. Their Profile shows to confirm; then they get this Area and nothing else.'**
  String get shareIntro;

  /// No description provided for @roleHeading.
  ///
  /// In en, this message translates to:
  /// **'They may'**
  String get roleHeading;

  /// No description provided for @roleManager.
  ///
  /// In en, this message translates to:
  /// **'Manager'**
  String get roleManager;

  /// No description provided for @roleMember.
  ///
  /// In en, this message translates to:
  /// **'Member'**
  String get roleMember;

  /// No description provided for @roleManagerDetail.
  ///
  /// In en, this message translates to:
  /// **'Everything in this Area.'**
  String get roleManagerDetail;

  /// No description provided for @roleMemberDetail.
  ///
  /// In en, this message translates to:
  /// **'Add Notes and change their own, add to any Note and check off Routines and Reminders.'**
  String get roleMemberDetail;

  /// No description provided for @shareInvite.
  ///
  /// In en, this message translates to:
  /// **'Invite'**
  String get shareInvite;

  /// No description provided for @shareDone.
  ///
  /// In en, this message translates to:
  /// **'{area} is now shared with {name}.'**
  String shareDone(String area, String name);

  /// No description provided for @sharedWithYou.
  ///
  /// In en, this message translates to:
  /// **'{name} shared {area} with you.'**
  String sharedWithYou(String name, String area);

  /// No description provided for @peopleYou.
  ///
  /// In en, this message translates to:
  /// **'{name} (you)'**
  String peopleYou(String name);

  /// No description provided for @makeManager.
  ///
  /// In en, this message translates to:
  /// **'Make Manager'**
  String get makeManager;

  /// No description provided for @makeMember.
  ///
  /// In en, this message translates to:
  /// **'Make Member'**
  String get makeMember;

  /// No description provided for @removePerson.
  ///
  /// In en, this message translates to:
  /// **'Remove'**
  String get removePerson;

  /// No description provided for @removePersonQuestion.
  ///
  /// In en, this message translates to:
  /// **'Remove {name} from {area}?'**
  String removePersonQuestion(String name, String area);

  /// No description provided for @removePersonDetail.
  ///
  /// In en, this message translates to:
  /// **'Nothing more of this Area goes to them. What they already have stays with them.'**
  String get removePersonDetail;

  /// No description provided for @peopleIntro.
  ///
  /// In en, this message translates to:
  /// **'A Manager may do everything here. A Member may add Notes and change their own, add to any Note and check off Routines and Reminders.'**
  String get peopleIntro;

  /// No description provided for @onlyManagers.
  ///
  /// In en, this message translates to:
  /// **'Only a Manager of this Area can change this.'**
  String get onlyManagers;

  /// No description provided for @pairedPerson.
  ///
  /// In en, this message translates to:
  /// **'Another person\'s Device'**
  String get pairedPerson;

  /// No description provided for @sendNote.
  ///
  /// In en, this message translates to:
  /// **'Send this Note'**
  String get sendNote;

  /// No description provided for @sendTitle.
  ///
  /// In en, this message translates to:
  /// **'Send this Note'**
  String get sendTitle;

  /// No description provided for @sendWho.
  ///
  /// In en, this message translates to:
  /// **'Who gets it?'**
  String get sendWho;

  /// No description provided for @sendAnyone.
  ///
  /// In en, this message translates to:
  /// **'Anyone, not encrypted'**
  String get sendAnyone;

  /// No description provided for @sendEncrypted.
  ///
  /// In en, this message translates to:
  /// **'{name}, encrypted'**
  String sendEncrypted(String name);

  /// No description provided for @sendPlainHint.
  ///
  /// In en, this message translates to:
  /// **'Not encrypted: whoever gets the file can read it. To encrypt it, share an Area with the person or pair with their Device first.'**
  String get sendPlainHint;

  /// No description provided for @sendWhat.
  ///
  /// In en, this message translates to:
  /// **'It carries the text, the Sections and the Routines and Reminders; no photos, recordings or where it was captured. Sending it again later sends it as an update.'**
  String get sendWhat;

  /// No description provided for @sendAction.
  ///
  /// In en, this message translates to:
  /// **'Send'**
  String get sendAction;

  /// No description provided for @sendNoApp.
  ///
  /// In en, this message translates to:
  /// **'No app here can take the file.'**
  String get sendNoApp;

  /// No description provided for @sentItemTitle.
  ///
  /// In en, this message translates to:
  /// **'Sent to you'**
  String get sentItemTitle;

  /// No description provided for @sentItemFrom.
  ///
  /// In en, this message translates to:
  /// **'From {name}'**
  String sentItemFrom(String name);

  /// No description provided for @sentItemVerified.
  ///
  /// In en, this message translates to:
  /// **'Verified: code compared in person. Code {code}'**
  String sentItemVerified(String code);

  /// No description provided for @sentItemKnown.
  ///
  /// In en, this message translates to:
  /// **'Not verified: known here, code not compared yet. Code {code}'**
  String sentItemKnown(String code);

  /// No description provided for @sentItemUnknown.
  ///
  /// In en, this message translates to:
  /// **'Not verified: this Profile is new here. Code {code}'**
  String sentItemUnknown(String code);

  /// No description provided for @sentItemCompare.
  ///
  /// In en, this message translates to:
  /// **'Compare codes'**
  String get sentItemCompare;

  /// No description provided for @sentItemCompareHint.
  ///
  /// In en, this message translates to:
  /// **'Ask {name}, in person, to read out the code on their Profile page. It must be {code}.'**
  String sentItemCompareHint(String name, String code);

  /// No description provided for @sentItemCodesMatch.
  ///
  /// In en, this message translates to:
  /// **'Codes match'**
  String get sentItemCodesMatch;

  /// No description provided for @sentItemCodesDiffer.
  ///
  /// In en, this message translates to:
  /// **'Codes differ'**
  String get sentItemCodesDiffer;

  /// No description provided for @sentItemVerifiedOnAccept.
  ///
  /// In en, this message translates to:
  /// **'Marked as verified when you accept.'**
  String get sentItemVerifiedOnAccept;

  /// No description provided for @sentItemCodesDiffered.
  ///
  /// In en, this message translates to:
  /// **'The codes differ: this file may not come from the person it names.'**
  String get sentItemCodesDiffered;

  /// No description provided for @sentItemEncrypted.
  ///
  /// In en, this message translates to:
  /// **'It came encrypted to this Device.'**
  String get sentItemEncrypted;

  /// No description provided for @sentItemPlain.
  ///
  /// In en, this message translates to:
  /// **'It came unencrypted: others may have read it on the way.'**
  String get sentItemPlain;

  /// No description provided for @sentItemAccept.
  ///
  /// In en, this message translates to:
  /// **'Accept'**
  String get sentItemAccept;

  /// No description provided for @sentItemDecline.
  ///
  /// In en, this message translates to:
  /// **'Decline'**
  String get sentItemDecline;

  /// No description provided for @sentItemAccepted.
  ///
  /// In en, this message translates to:
  /// **'Added to Unsorted, with its alarms on.'**
  String get sentItemAccepted;

  /// No description provided for @sentItemDeclined.
  ///
  /// In en, this message translates to:
  /// **'Declined: nothing was kept.'**
  String get sentItemDeclined;

  /// No description provided for @sentItemAlready.
  ///
  /// In en, this message translates to:
  /// **'Already up to date: you accepted this version or a newer one before.'**
  String get sentItemAlready;

  /// No description provided for @sentItemUnreadable.
  ///
  /// In en, this message translates to:
  /// **'This file cannot be read: it is damaged, or not signed by the key it names.'**
  String get sentItemUnreadable;

  /// No description provided for @sentItemNotForUs.
  ///
  /// In en, this message translates to:
  /// **'This file is encrypted for someone else.'**
  String get sentItemNotForUs;

  /// No description provided for @sendUpdate.
  ///
  /// In en, this message translates to:
  /// **'Send an update'**
  String get sendUpdate;

  /// No description provided for @sendCancel.
  ///
  /// In en, this message translates to:
  /// **'Send a cancellation instead'**
  String get sendCancel;

  /// No description provided for @sendCancelHint.
  ///
  /// In en, this message translates to:
  /// **'They can then move it to their trash, with its alarms off.'**
  String get sendCancelHint;

  /// No description provided for @sentItemUpdate.
  ///
  /// In en, this message translates to:
  /// **'An update to a Note you accepted before.'**
  String get sentItemUpdate;

  /// No description provided for @sentItemChanges.
  ///
  /// In en, this message translates to:
  /// **'What changes'**
  String get sentItemChanges;

  /// No description provided for @sentItemTextWas.
  ///
  /// In en, this message translates to:
  /// **'Text so far: {text}'**
  String sentItemTextWas(String text);

  /// No description provided for @sentItemTextNow.
  ///
  /// In en, this message translates to:
  /// **'New text: {text}'**
  String sentItemTextNow(String text);

  /// No description provided for @sentItemTextYours.
  ///
  /// In en, this message translates to:
  /// **'You changed this text yourself; the update replaces your version.'**
  String get sentItemTextYours;

  /// No description provided for @sentItemSectionAdded.
  ///
  /// In en, this message translates to:
  /// **'Added: {text}'**
  String sentItemSectionAdded(String text);

  /// No description provided for @sentItemRescheduled.
  ///
  /// In en, this message translates to:
  /// **'{before} → {after}'**
  String sentItemRescheduled(String before, String after);

  /// No description provided for @sentItemRoutineAdded.
  ///
  /// In en, this message translates to:
  /// **'New: {schedule}'**
  String sentItemRoutineAdded(String schedule);

  /// No description provided for @sentItemRoutineRemoved.
  ///
  /// In en, this message translates to:
  /// **'Removed: {schedule}'**
  String sentItemRoutineRemoved(String schedule);

  /// No description provided for @sentItemNoChanges.
  ///
  /// In en, this message translates to:
  /// **'Nothing changed.'**
  String get sentItemNoChanges;

  /// No description provided for @sentItemRestores.
  ///
  /// In en, this message translates to:
  /// **'It is in your trash; the update brings it back.'**
  String get sentItemRestores;

  /// No description provided for @sentItemKeeps.
  ///
  /// In en, this message translates to:
  /// **'Your own Sections, Routines and Reminders, Done marks, Importance and where you filed it stay as they are.'**
  String get sentItemKeeps;

  /// No description provided for @sentItemApply.
  ///
  /// In en, this message translates to:
  /// **'Apply update'**
  String get sentItemApply;

  /// No description provided for @sentItemApplied.
  ///
  /// In en, this message translates to:
  /// **'Updated. Your own additions stayed.'**
  String get sentItemApplied;

  /// No description provided for @sentItemCancelled.
  ///
  /// In en, this message translates to:
  /// **'Cancelled: {text}'**
  String sentItemCancelled(String text);

  /// No description provided for @sentItemCancelDetail.
  ///
  /// In en, this message translates to:
  /// **'Accepting moves it and its Routines and Reminders to the trash, alarms off. You can restore it from there.'**
  String get sentItemCancelDetail;

  /// No description provided for @sentItemAcceptCancel.
  ///
  /// In en, this message translates to:
  /// **'Move to the trash'**
  String get sentItemAcceptCancel;

  /// No description provided for @sentItemTrashed.
  ///
  /// In en, this message translates to:
  /// **'Moved to the trash with its alarms off. You can restore it from there.'**
  String get sentItemTrashed;

  /// No description provided for @sentItemNothingToCancel.
  ///
  /// In en, this message translates to:
  /// **'Nothing to cancel: this Note is not among yours, or is in the trash already.'**
  String get sentItemNothingToCancel;

  /// No description provided for @sentItemNotFromSender.
  ///
  /// In en, this message translates to:
  /// **'Refused: this file would change a Note someone sent you, but it is not signed by them.'**
  String get sentItemNotFromSender;

  /// No description provided for @sentItemClose.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get sentItemClose;

  /// No description provided for @backupsTitle.
  ///
  /// In en, this message translates to:
  /// **'Backups'**
  String get backupsTitle;

  /// No description provided for @backupsSettingsEntry.
  ///
  /// In en, this message translates to:
  /// **'Back up this Device'**
  String get backupsSettingsEntry;

  /// No description provided for @backupsSettingsDetail.
  ///
  /// In en, this message translates to:
  /// **'Encrypted files of everything on this Device, in a folder you choose.'**
  String get backupsSettingsDetail;

  /// No description provided for @backupsIntro.
  ///
  /// In en, this message translates to:
  /// **'A Backup is an encrypted file of all your Areas, Notes, photos, voice recordings, Routines and their history. Only your passphrase, or the recovery key on the Recovery Sheet, opens it. This Device\'s settings stay out. All your Devices share one passphrase and one recovery key, and each writes Backups of its own.'**
  String get backupsIntro;

  /// No description provided for @backupPassphrase.
  ///
  /// In en, this message translates to:
  /// **'Passphrase'**
  String get backupPassphrase;

  /// No description provided for @backupPassphraseAgain.
  ///
  /// In en, this message translates to:
  /// **'Passphrase again'**
  String get backupPassphraseAgain;

  /// No description provided for @backupPassphraseHint.
  ///
  /// In en, this message translates to:
  /// **'At least 8 characters. A short sentence is easy to remember.'**
  String get backupPassphraseHint;

  /// No description provided for @backupPassphraseShort.
  ///
  /// In en, this message translates to:
  /// **'At least 8 characters, please.'**
  String get backupPassphraseShort;

  /// No description provided for @backupPassphraseMismatch.
  ///
  /// In en, this message translates to:
  /// **'The two do not match.'**
  String get backupPassphraseMismatch;

  /// No description provided for @backupSetUp.
  ///
  /// In en, this message translates to:
  /// **'Set up Backups'**
  String get backupSetUp;

  /// No description provided for @backupSettingUp.
  ///
  /// In en, this message translates to:
  /// **'Setting up… this takes a few seconds.'**
  String get backupSettingUp;

  /// No description provided for @backupSetUpElsewhere.
  ///
  /// In en, this message translates to:
  /// **'Set up already on another of your Devices? Sync with it, and this Device takes the same passphrase and recovery key.'**
  String get backupSetUpElsewhere;

  /// No description provided for @backupSheetTitle.
  ///
  /// In en, this message translates to:
  /// **'Recovery Sheet'**
  String get backupSheetTitle;

  /// No description provided for @backupSheetIntro.
  ///
  /// In en, this message translates to:
  /// **'Print the Recovery Sheet or save it somewhere safe. Its recovery key opens the Backups of all your Devices when the passphrase is forgotten.'**
  String get backupSheetIntro;

  /// No description provided for @backupSheetPrint.
  ///
  /// In en, this message translates to:
  /// **'Print or save'**
  String get backupSheetPrint;

  /// No description provided for @backupSheetShare.
  ///
  /// In en, this message translates to:
  /// **'Share'**
  String get backupSheetShare;

  /// No description provided for @backupSheetFailed.
  ///
  /// In en, this message translates to:
  /// **'The Recovery Sheet could not be made. Try again.'**
  String get backupSheetFailed;

  /// No description provided for @backupSheetNoApp.
  ///
  /// In en, this message translates to:
  /// **'No app here can take the Recovery Sheet.'**
  String get backupSheetNoApp;

  /// No description provided for @backupFolderTitle.
  ///
  /// In en, this message translates to:
  /// **'Folder'**
  String get backupFolderTitle;

  /// No description provided for @backupFolderNone.
  ///
  /// In en, this message translates to:
  /// **'No folder chosen yet. Backups go into a folder you choose: on this Device, on a USB stick or in a cloud app.'**
  String get backupFolderNone;

  /// No description provided for @backupFolderChosen.
  ///
  /// In en, this message translates to:
  /// **'Backups go into {name}.'**
  String backupFolderChosen(String name);

  /// No description provided for @backupChooseFolder.
  ///
  /// In en, this message translates to:
  /// **'Choose folder'**
  String get backupChooseFolder;

  /// No description provided for @backupWeekly.
  ///
  /// In en, this message translates to:
  /// **'Back up every week'**
  String get backupWeekly;

  /// No description provided for @backupWeeklyDetail.
  ///
  /// In en, this message translates to:
  /// **'The last 4 Backups of this Device are kept; older ones go.'**
  String get backupWeeklyDetail;

  /// No description provided for @backupNow.
  ///
  /// In en, this message translates to:
  /// **'Back up now'**
  String get backupNow;

  /// No description provided for @backupWorking.
  ///
  /// In en, this message translates to:
  /// **'Backing up…'**
  String get backupWorking;

  /// No description provided for @backupLast.
  ///
  /// In en, this message translates to:
  /// **'Last Backup: {when}'**
  String backupLast(String when);

  /// No description provided for @backupNotYet.
  ///
  /// In en, this message translates to:
  /// **'No Backup yet.'**
  String get backupNotYet;

  /// No description provided for @backupFailedAt.
  ///
  /// In en, this message translates to:
  /// **'The Backup of {when} could not be written. Is the folder still there?'**
  String backupFailedAt(String when);

  /// No description provided for @backupDone.
  ///
  /// In en, this message translates to:
  /// **'Backup written.'**
  String get backupDone;

  /// No description provided for @backupFailed.
  ///
  /// In en, this message translates to:
  /// **'The Backup could not be written. Is the folder still there?'**
  String get backupFailed;

  /// No description provided for @restoreOffer.
  ///
  /// In en, this message translates to:
  /// **'Restore from a Backup'**
  String get restoreOffer;

  /// No description provided for @restoreOfferDetail.
  ///
  /// In en, this message translates to:
  /// **'Coming from another Device? Bring everything back from a Backup file.'**
  String get restoreOfferDetail;

  /// No description provided for @restoreIntro.
  ///
  /// In en, this message translates to:
  /// **'Choose the Backup file, then open it with your passphrase or with the recovery key from the Recovery Sheet. Settings are not in a Backup: set them again afterwards, and pair your other Devices again.'**
  String get restoreIntro;

  /// No description provided for @restorePick.
  ///
  /// In en, this message translates to:
  /// **'Choose the Backup file'**
  String get restorePick;

  /// No description provided for @restoreNotABackup.
  ///
  /// In en, this message translates to:
  /// **'This file is not a Backup of Old Brain Fix.'**
  String get restoreNotABackup;

  /// No description provided for @restoreFile.
  ///
  /// In en, this message translates to:
  /// **'Backup of {when}'**
  String restoreFile(String when);

  /// No description provided for @restoreOr.
  ///
  /// In en, this message translates to:
  /// **'or'**
  String get restoreOr;

  /// No description provided for @restoreCodeLabel.
  ///
  /// In en, this message translates to:
  /// **'Recovery key'**
  String get restoreCodeLabel;

  /// No description provided for @restoreScan.
  ///
  /// In en, this message translates to:
  /// **'Scan the Recovery Sheet'**
  String get restoreScan;

  /// No description provided for @restoreNoCode.
  ///
  /// In en, this message translates to:
  /// **'No recovery key found in the photo. Try again a little closer, or type it.'**
  String get restoreNoCode;

  /// No description provided for @restoreInvalidCode.
  ///
  /// In en, this message translates to:
  /// **'That is not a recovery key. Check it against the Recovery Sheet.'**
  String get restoreInvalidCode;

  /// No description provided for @restoreStart.
  ///
  /// In en, this message translates to:
  /// **'Restore'**
  String get restoreStart;

  /// No description provided for @restoreWorking.
  ///
  /// In en, this message translates to:
  /// **'Restoring… this takes a moment.'**
  String get restoreWorking;

  /// No description provided for @restoreWrong.
  ///
  /// In en, this message translates to:
  /// **'This passphrase or recovery key does not open the Backup.'**
  String get restoreWrong;

  /// No description provided for @restoreDamaged.
  ///
  /// In en, this message translates to:
  /// **'The Backup is damaged or incomplete. Nothing was restored; try an older one.'**
  String get restoreDamaged;

  /// No description provided for @restoreNotEmpty.
  ///
  /// In en, this message translates to:
  /// **'This Device holds Notes already, so nothing was restored.'**
  String get restoreNotEmpty;

  /// No description provided for @restoreFailed.
  ///
  /// In en, this message translates to:
  /// **'The Backup could not be restored. Nothing was changed.'**
  String get restoreFailed;

  /// No description provided for @restoreDone.
  ///
  /// In en, this message translates to:
  /// **'Restored: {count, plural, =1{1 change} other{{count} changes}}. Set alarms and locks again, choose the Backup folder again, and pair your other Devices again.'**
  String restoreDone(int count);

  /// No description provided for @recoverySheetPdfTitle.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix: Recovery Sheet'**
  String get recoverySheetPdfTitle;

  /// No description provided for @recoverySheetMade.
  ///
  /// In en, this message translates to:
  /// **'Made on {date}'**
  String recoverySheetMade(String date);

  /// No description provided for @recoverySheetProfile.
  ///
  /// In en, this message translates to:
  /// **'Profile: {name}'**
  String recoverySheetProfile(String name);

  /// No description provided for @recoverySheetPdfIntro.
  ///
  /// In en, this message translates to:
  /// **'This recovery key opens the Backups of all your Devices, even when the passphrase is forgotten. Whoever holds it can read them: keep this sheet somewhere safe, in a drawer or a password manager, never next to the Backups.'**
  String get recoverySheetPdfIntro;

  /// No description provided for @recoverySheetKey.
  ///
  /// In en, this message translates to:
  /// **'Recovery key'**
  String get recoverySheetKey;

  /// No description provided for @recoverySheetStepRestore.
  ///
  /// In en, this message translates to:
  /// **'To restore, install Old Brain Fix, choose “Restore from a Backup” on the first screen, pick the newest Backup file and type this key, or scan the QR code.'**
  String get recoverySheetStepRestore;

  /// No description provided for @recoverySheetStepSettings.
  ///
  /// In en, this message translates to:
  /// **'Settings such as alarms, Quiet Hours and locks are not in a Backup. Set them again on the new Device, and pair it again with your other Devices.'**
  String get recoverySheetStepSettings;

  /// No description provided for @backupAutoTitle.
  ///
  /// In en, this message translates to:
  /// **'Google backup'**
  String get backupAutoTitle;

  /// No description provided for @backupAutoSwitch.
  ///
  /// In en, this message translates to:
  /// **'Also keep the newest Backup in my Google account'**
  String get backupAutoSwitch;

  /// No description provided for @backupAutoDetail.
  ///
  /// In en, this message translates to:
  /// **'Android\'s own backup then keeps the newest Backup file, encrypted as it is, in your Google account, and nothing else of the app. On a new phone it comes back with your apps, and the app asks for the passphrase or the recovery key.'**
  String get backupAutoDetail;

  /// No description provided for @backupAutoTooBig.
  ///
  /// In en, this message translates to:
  /// **'The newest Backup is larger than the 25 MB Android sends to Google, so Google keeps none. A folder still takes it.'**
  String get backupAutoTooBig;

  /// No description provided for @restoreFoundTitle.
  ///
  /// In en, this message translates to:
  /// **'A Backup came back'**
  String get restoreFoundTitle;

  /// No description provided for @restoreFound.
  ///
  /// In en, this message translates to:
  /// **'This phone brought a Backup back from your Google account. Open it with your passphrase or with the recovery key from the Recovery Sheet.'**
  String get restoreFound;

  /// No description provided for @restoreFoundSkip.
  ///
  /// In en, this message translates to:
  /// **'Start without it'**
  String get restoreFoundSkip;

  /// No description provided for @profileTitle.
  ///
  /// In en, this message translates to:
  /// **'Profile'**
  String get profileTitle;

  /// No description provided for @profileNoName.
  ///
  /// In en, this message translates to:
  /// **'No name'**
  String get profileNoName;

  /// No description provided for @profileName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get profileName;

  /// No description provided for @profileCode.
  ///
  /// In en, this message translates to:
  /// **'Your code'**
  String get profileCode;

  /// No description provided for @profilePictureChange.
  ///
  /// In en, this message translates to:
  /// **'Change the picture'**
  String get profilePictureChange;

  /// No description provided for @profilePictureChoose.
  ///
  /// In en, this message translates to:
  /// **'Choose a picture'**
  String get profilePictureChoose;

  /// No description provided for @profilePictureTake.
  ///
  /// In en, this message translates to:
  /// **'Take a photo'**
  String get profilePictureTake;

  /// No description provided for @profilePictureInitials.
  ///
  /// In en, this message translates to:
  /// **'Show initials instead'**
  String get profilePictureInitials;

  /// No description provided for @profilePictureUnreadable.
  ///
  /// In en, this message translates to:
  /// **'This picture cannot be read.'**
  String get profilePictureUnreadable;

  /// No description provided for @profileCropTitle.
  ///
  /// In en, this message translates to:
  /// **'Choose the square'**
  String get profileCropTitle;

  /// No description provided for @profileCropHint.
  ///
  /// In en, this message translates to:
  /// **'Drag the square to move it. The slider sets its size.'**
  String get profileCropHint;

  /// No description provided for @profileCropSize.
  ///
  /// In en, this message translates to:
  /// **'Size of the square'**
  String get profileCropSize;

  /// No description provided for @profileCropUse.
  ///
  /// In en, this message translates to:
  /// **'Use this part'**
  String get profileCropUse;

  /// No description provided for @profileBackups.
  ///
  /// In en, this message translates to:
  /// **'Backups and recovery'**
  String get profileBackups;

  /// No description provided for @profileDevices.
  ///
  /// In en, this message translates to:
  /// **'Your Devices'**
  String get profileDevices;

  /// No description provided for @profileDeviceSettings.
  ///
  /// In en, this message translates to:
  /// **'Settings of this Device'**
  String get profileDeviceSettings;

  /// No description provided for @profileDeviceSettingsDetail.
  ///
  /// In en, this message translates to:
  /// **'They belong to this Device and are never synced.'**
  String get profileDeviceSettingsDetail;

  /// No description provided for @profileWhichTitle.
  ///
  /// In en, this message translates to:
  /// **'Which one is you?'**
  String get profileWhichTitle;

  /// No description provided for @profileWhichText.
  ///
  /// In en, this message translates to:
  /// **'Both Devices had a Profile of their own. The one you pick is kept on all of your Devices.'**
  String get profileWhichText;

  /// No description provided for @profileWhichThisDevice.
  ///
  /// In en, this message translates to:
  /// **'On this Device'**
  String get profileWhichThisDevice;

  /// No description provided for @profileWhichOtherDevice.
  ///
  /// In en, this message translates to:
  /// **'On the other Device'**
  String get profileWhichOtherDevice;

  /// No description provided for @profileWhichPick.
  ///
  /// In en, this message translates to:
  /// **'This is me'**
  String get profileWhichPick;

  /// No description provided for @profileWhichLater.
  ///
  /// In en, this message translates to:
  /// **'Later'**
  String get profileWhichLater;

  /// No description provided for @profileWhichWaiting.
  ///
  /// In en, this message translates to:
  /// **'Your Devices have two Profiles. Pick yours.'**
  String get profileWhichWaiting;

  /// Over the other person's Profile while inviting them to a Shared Area.
  ///
  /// In en, this message translates to:
  /// **'Share {area} with this person?'**
  String inviteConfirmTitle(String area);

  /// No description provided for @inviteConfirmText.
  ///
  /// In en, this message translates to:
  /// **'Compare the code with the one on their Profile page. Only then is anything shared.'**
  String get inviteConfirmText;

  /// No description provided for @inviteConfirmShare.
  ///
  /// In en, this message translates to:
  /// **'Share'**
  String get inviteConfirmShare;

  /// No description provided for @profileTheirCode.
  ///
  /// In en, this message translates to:
  /// **'Their code'**
  String get profileTheirCode;

  /// Read out for another person's picture on a Note's card.
  ///
  /// In en, this message translates to:
  /// **'Written by {name}'**
  String authorPicture(String name);

  /// No description provided for @shareCancelled.
  ///
  /// In en, this message translates to:
  /// **'Nothing was shared.'**
  String get shareCancelled;
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
