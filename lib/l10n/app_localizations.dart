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

  /// No description provided for @photo.
  ///
  /// In en, this message translates to:
  /// **'Photo'**
  String get photo;

  /// No description provided for @voice.
  ///
  /// In en, this message translates to:
  /// **'Voice'**
  String get voice;

  /// No description provided for @gallery.
  ///
  /// In en, this message translates to:
  /// **'Gallery'**
  String get gallery;

  /// No description provided for @text.
  ///
  /// In en, this message translates to:
  /// **'Text'**
  String get text;

  /// No description provided for @captureFanClose.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get captureFanClose;

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

  /// No description provided for @firstStartTitle.
  ///
  /// In en, this message translates to:
  /// **'Welcome to Old Brain Fix'**
  String get firstStartTitle;

  /// No description provided for @firstStartIntro.
  ///
  /// In en, this message translates to:
  /// **'How does Old Brain Fix start on this Device?'**
  String get firstStartIntro;

  /// No description provided for @firstStartNew.
  ///
  /// In en, this message translates to:
  /// **'New to Old Brain Fix'**
  String get firstStartNew;

  /// No description provided for @firstStartNewDetail.
  ///
  /// In en, this message translates to:
  /// **'Start with a set of Areas to pick from.'**
  String get firstStartNewDetail;

  /// No description provided for @firstStartDevice.
  ///
  /// In en, this message translates to:
  /// **'Add a Device of mine'**
  String get firstStartDevice;

  /// No description provided for @firstStartDeviceDetail.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix runs on another Device of yours. Pair with it, and everything comes over.'**
  String get firstStartDeviceDetail;

  /// No description provided for @hostContinue.
  ///
  /// In en, this message translates to:
  /// **'Continue'**
  String get hostContinue;

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
  /// **'Old Brain Fix may not use your location, so it cannot tell where you are.'**
  String get locationNotAllowed;

  /// No description provided for @locationUnknown.
  ///
  /// In en, this message translates to:
  /// **'The phone could not find out where you are.'**
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

  /// Heading of a day on the Agenda after tomorrow: its weekday and date, as in "Thursday, 29 Oct".
  ///
  /// In en, this message translates to:
  /// **'{day}'**
  String agendaDay(DateTime day);

  /// Under the heading of a day after tomorrow on the Agenda when nothing is due that day.
  ///
  /// In en, this message translates to:
  /// **'Nothing planned'**
  String get nothingPlanned;

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
  /// **'Routine, Reminder, Frog or Session'**
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

  /// No description provided for @repeatMonthDay.
  ///
  /// In en, this message translates to:
  /// **'Monthly on a day'**
  String get repeatMonthDay;

  /// No description provided for @repeatMonthWeekday.
  ///
  /// In en, this message translates to:
  /// **'Monthly on a weekday'**
  String get repeatMonthWeekday;

  /// No description provided for @dayOfMonthLabel.
  ///
  /// In en, this message translates to:
  /// **'Day of the month'**
  String get dayOfMonthLabel;

  /// No description provided for @monthDaySkipped.
  ///
  /// In en, this message translates to:
  /// **'Months without this day are skipped.'**
  String get monthDaySkipped;

  /// No description provided for @whichInMonthLabel.
  ///
  /// In en, this message translates to:
  /// **'Which one in the month'**
  String get whichInMonthLabel;

  /// No description provided for @nthOption.
  ///
  /// In en, this message translates to:
  /// **'{which, select, 1{First} 2{Second} 3{Third} 4{Fourth} other{Last}}'**
  String nthOption(String which);

  /// No description provided for @weekdayLabel.
  ///
  /// In en, this message translates to:
  /// **'Weekday'**
  String get weekdayLabel;

  /// No description provided for @endsLabel.
  ///
  /// In en, this message translates to:
  /// **'Ends'**
  String get endsLabel;

  /// No description provided for @endNever.
  ///
  /// In en, this message translates to:
  /// **'Never'**
  String get endNever;

  /// No description provided for @endOnDay.
  ///
  /// In en, this message translates to:
  /// **'On a day'**
  String get endOnDay;

  /// No description provided for @endAfterCount.
  ///
  /// In en, this message translates to:
  /// **'After a number of times'**
  String get endAfterCount;

  /// No description provided for @endLastDay.
  ///
  /// In en, this message translates to:
  /// **'Last day'**
  String get endLastDay;

  /// No description provided for @endCountLabel.
  ///
  /// In en, this message translates to:
  /// **'How many times'**
  String get endCountLabel;

  /// No description provided for @endCountExplain.
  ///
  /// In en, this message translates to:
  /// **'Each time of day counts separately.'**
  String get endCountExplain;

  /// No description provided for @unitTimes.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{time} other{times}}'**
  String unitTimes(int count);

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

  /// No description provided for @everyMonthOnDay.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Every month on day {day}} other{Every {count} months on day {day}}}'**
  String everyMonthOnDay(int count, int day);

  /// No description provided for @everyMonthOnNth.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Every month on the {nth} {weekday}} other{Every {count} months on the {nth} {weekday}}}'**
  String everyMonthOnNth(int count, String nth, String weekday);

  /// No description provided for @nthWord.
  ///
  /// In en, this message translates to:
  /// **'{which, select, 1{first} 2{second} 3{third} 4{fourth} other{last}}'**
  String nthWord(String which);

  /// No description provided for @repeatUntil.
  ///
  /// In en, this message translates to:
  /// **'{repeat}, until {date}'**
  String repeatUntil(String repeat, String date);

  /// No description provided for @repeatCount.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{{repeat}, once} other{{repeat}, {count} times}}'**
  String repeatCount(int count, String repeat);

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
  /// **'Besides when and where, a Note can record more of what was going on. This is off until you switch it on, and stays in the Note on your own devices.'**
  String get captureContextIntro;

  /// No description provided for @calendarContextSwitch.
  ///
  /// In en, this message translates to:
  /// **'Note the running appointment'**
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

  /// No description provided for @bluetoothNotAllowed.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix may not see your Bluetooth devices.'**
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

  /// The big button that ends a voice recording on the capture screen; the screen stays.
  ///
  /// In en, this message translates to:
  /// **'Stop'**
  String get stopRecording;

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

  /// No description provided for @speechModelSourceModels.
  ///
  /// In en, this message translates to:
  /// **'Each download comes only from Old Brain Fix\'s own release on GitHub and is checked against a fixed checksum before it is used. Wi-Fi is best. Nothing else is ever sent or fetched.'**
  String get speechModelSourceModels;

  /// Credit for the downloadable Whisper models on the speech model page.
  ///
  /// In en, this message translates to:
  /// **'The speech models are the base and small models of Whisper by OpenAI, Copyright (c) 2022 OpenAI, under the MIT License.'**
  String get speechModelCreditModels;

  /// Heading above the choice of Whisper model on the speech model page.
  ///
  /// In en, this message translates to:
  /// **'Model'**
  String get speechModelChoice;

  /// Radio option for Whisper's base model, with its download size.
  ///
  /// In en, this message translates to:
  /// **'Base, {megabytes} MB'**
  String speechModelBase(int megabytes);

  /// No description provided for @speechModelBaseDetail.
  ///
  /// In en, this message translates to:
  /// **'Faster, and enough for English or German alone.'**
  String get speechModelBaseDetail;

  /// Radio option for Whisper's small model, with its download size.
  ///
  /// In en, this message translates to:
  /// **'Small, {megabytes} MB'**
  String speechModelSmall(int megabytes);

  /// No description provided for @speechModelSmallDetail.
  ///
  /// In en, this message translates to:
  /// **'Better with English words in German speech, but slower, and it needs more memory.'**
  String get speechModelSmallDetail;

  /// No description provided for @speechModelKeepsOne.
  ///
  /// In en, this message translates to:
  /// **'Choosing a model downloads it. Only the chosen model is kept: the one before is deleted once the new one is in place.'**
  String get speechModelKeepsOne;

  /// No description provided for @speechModelPickEither.
  ///
  /// In en, this message translates to:
  /// **'It must be whisper.cpp\'s multilingual ggml-base.bin or ggml-small.bin; any other file is rejected.'**
  String get speechModelPickEither;

  /// The name of the app's Agenda, as a word on its own. Whisper is told it so the word is recognised in speech.
  ///
  /// In en, this message translates to:
  /// **'Agenda'**
  String get agenda;

  /// No description provided for @speechOfferQuestion.
  ///
  /// In en, this message translates to:
  /// **'Transcribe voice Notes on the phone?'**
  String get speechOfferQuestion;

  /// No description provided for @speechOfferDetail.
  ///
  /// In en, this message translates to:
  /// **'Download the speech model ({megabytes} MB) once. It downloads in the background while you go on, and Settings offers it any time later.'**
  String speechOfferDetail(int megabytes);

  /// No description provided for @speechOfferMobileData.
  ///
  /// In en, this message translates to:
  /// **'You are on mobile data. The download uses {megabytes} MB of it.'**
  String speechOfferMobileData(int megabytes);

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

  /// No description provided for @share.
  ///
  /// In en, this message translates to:
  /// **'Share'**
  String get share;

  /// No description provided for @exportCalendarFile.
  ///
  /// In en, this message translates to:
  /// **'Export'**
  String get exportCalendarFile;

  /// No description provided for @allRoutinesTitle.
  ///
  /// In en, this message translates to:
  /// **'All Routines and Reminders'**
  String get allRoutinesTitle;

  /// No description provided for @allRoutinesEmpty.
  ///
  /// In en, this message translates to:
  /// **'No Routines or Reminders yet.'**
  String get allRoutinesEmpty;

  /// No description provided for @calendarPageTitle.
  ///
  /// In en, this message translates to:
  /// **'Calendar'**
  String get calendarPageTitle;

  /// No description provided for @calendarReadHeading.
  ///
  /// In en, this message translates to:
  /// **'Read from your calendar'**
  String get calendarReadHeading;

  /// No description provided for @calendarWriteHeading.
  ///
  /// In en, this message translates to:
  /// **'Write to your calendar'**
  String get calendarWriteHeading;

  /// No description provided for @settingsCalendarDetail.
  ///
  /// In en, this message translates to:
  /// **'What is read from your calendar and what is written to it.'**
  String get settingsCalendarDetail;

  /// No description provided for @calendarMirrorSwitch.
  ///
  /// In en, this message translates to:
  /// **'Write Routines and Reminders into a calendar'**
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
  /// **'Write them into your calendar?'**
  String get calendarMirrorExplainTitle;

  /// No description provided for @calendarMirrorExplainBody.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix can write your Routines and Reminders into a calendar you choose and keep them up to date. It only ever changes or removes the events it wrote itself, and turning this off removes them. Wherever that calendar syncs to, they show up too.'**
  String get calendarMirrorExplainBody;

  /// No description provided for @calendarMirrorNotAllowed.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix may not write to your calendar.'**
  String get calendarMirrorNotAllowed;

  /// No description provided for @calendarMirrorNoCalendar.
  ///
  /// In en, this message translates to:
  /// **'There is no calendar on this device to write into.'**
  String get calendarMirrorNoCalendar;

  /// No description provided for @calendarMirrorNotRemoved.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix could not remove the events it wrote from the calendar.'**
  String get calendarMirrorNotRemoved;

  /// No description provided for @agendaCalendarSwitch.
  ///
  /// In en, this message translates to:
  /// **'Show my calendar\'s appointments on the Agenda'**
  String get agendaCalendarSwitch;

  /// No description provided for @agendaCalendarDetail.
  ///
  /// In en, this message translates to:
  /// **'The appointments of the calendars you pick show beside your Routines and Reminders. Nothing is copied.'**
  String get agendaCalendarDetail;

  /// No description provided for @agendaCalendarNonePicked.
  ///
  /// In en, this message translates to:
  /// **'No calendar picked yet. Tap to pick calendars.'**
  String get agendaCalendarNonePicked;

  /// No description provided for @agendaCalendarShows.
  ///
  /// In en, this message translates to:
  /// **'From {calendars}. Tap to pick other calendars.'**
  String agendaCalendarShows(String calendars);

  /// No description provided for @agendaCalendarExplainTitle.
  ///
  /// In en, this message translates to:
  /// **'Show your appointments?'**
  String get agendaCalendarExplainTitle;

  /// No description provided for @agendaCalendarExplainBody.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix can show the appointments of the calendars you pick on the Agenda. It reads them whenever the Agenda shows, keeps no copy and sends nothing anywhere.'**
  String get agendaCalendarExplainBody;

  /// No description provided for @agendaCalendarNoCalendar.
  ///
  /// In en, this message translates to:
  /// **'There is no calendar on this device to read from.'**
  String get agendaCalendarNoCalendar;

  /// No description provided for @agendaCalendarsTitle.
  ///
  /// In en, this message translates to:
  /// **'Calendars on the Agenda'**
  String get agendaCalendarsTitle;

  /// No description provided for @agendaCalendarsHint.
  ///
  /// In en, this message translates to:
  /// **'The Agenda shows the appointments of the calendars picked here.'**
  String get agendaCalendarsHint;

  /// No description provided for @appointmentFromCalendar.
  ///
  /// In en, this message translates to:
  /// **'From the calendar “{calendar}”'**
  String appointmentFromCalendar(String calendar);

  /// No description provided for @appointmentCopy.
  ///
  /// In en, this message translates to:
  /// **'Copy into a Note'**
  String get appointmentCopy;

  /// No description provided for @appointmentNoTitle.
  ///
  /// In en, this message translates to:
  /// **'Appointment without a title'**
  String get appointmentNoTitle;

  /// No description provided for @appointmentSince.
  ///
  /// In en, this message translates to:
  /// **'from {time}'**
  String appointmentSince(String time);

  /// No description provided for @appointmentUntil.
  ///
  /// In en, this message translates to:
  /// **'until {time}'**
  String appointmentUntil(String time);

  /// No description provided for @appointmentPlace.
  ///
  /// In en, this message translates to:
  /// **'Place: {place}'**
  String appointmentPlace(String place);

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

  /// No description provided for @alarmSound.
  ///
  /// In en, this message translates to:
  /// **'Alarm sound'**
  String get alarmSound;

  /// No description provided for @alarmSoundOwn.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix\'s own chime. Tap to choose another sound of the phone.'**
  String get alarmSoundOwn;

  /// No description provided for @alarmSoundChosen.
  ///
  /// In en, this message translates to:
  /// **'A sound of the phone. Tap to choose another, or Old Brain Fix\'s own chime.'**
  String get alarmSoundChosen;

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
  /// **'Bluetooth device with each Note.'**
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

  /// No description provided for @dangerButton.
  ///
  /// In en, this message translates to:
  /// **'DON\'T PRESS.\nDANGER'**
  String get dangerButton;

  /// No description provided for @dangerThanks.
  ///
  /// In en, this message translates to:
  /// **'Thank you for using Old Brain Fix!'**
  String get dangerThanks;

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
  /// **'Open Sync on the other Device. Scan the code it shows, or type it in.'**
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

  /// No description provided for @syncShowIntro.
  ///
  /// In en, this message translates to:
  /// **'Or show a code here for the other Device to scan.'**
  String get syncShowIntro;

  /// No description provided for @syncShowCode.
  ///
  /// In en, this message translates to:
  /// **'Show this Device\'s code'**
  String get syncShowCode;

  /// No description provided for @shareShowIntro.
  ///
  /// In en, this message translates to:
  /// **'Or show a code here for their Device to scan.'**
  String get shareShowIntro;

  /// No description provided for @hostTitle.
  ///
  /// In en, this message translates to:
  /// **'This Device\'s code'**
  String get hostTitle;

  /// No description provided for @hostIntro.
  ///
  /// In en, this message translates to:
  /// **'On the other Device, open Sync and scan this code, or type in the code under it.'**
  String get hostIntro;

  /// No description provided for @hostInviteIntro.
  ///
  /// In en, this message translates to:
  /// **'On their Device, open Sync and scan this code, or type in the code under it. Their Profile then shows here to confirm.'**
  String get hostInviteIntro;

  /// No description provided for @hostTypedCode.
  ///
  /// In en, this message translates to:
  /// **'Code to type'**
  String get hostTypedCode;

  /// No description provided for @hostStays.
  ///
  /// In en, this message translates to:
  /// **'The code holds while this page is open. Go back once the other Device says it synced.'**
  String get hostStays;

  /// No description provided for @hostNoNetwork.
  ///
  /// In en, this message translates to:
  /// **'This Device is on no Wi-Fi. Connect it to the same Wi-Fi as the other Device, then try again.'**
  String get hostNoNetwork;

  /// No description provided for @hostLocked.
  ///
  /// In en, this message translates to:
  /// **'Too many wrong codes were tried. Make a new code to go on.'**
  String get hostLocked;

  /// No description provided for @hostFailed.
  ///
  /// In en, this message translates to:
  /// **'No code could be made. Try again.'**
  String get hostFailed;

  /// No description provided for @hostTryAgain.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get hostTryAgain;

  /// No description provided for @hostNewCode.
  ///
  /// In en, this message translates to:
  /// **'New code'**
  String get hostNewCode;

  /// No description provided for @hostAskTitle.
  ///
  /// In en, this message translates to:
  /// **'Let {name} join?'**
  String hostAskTitle(String name);

  /// No description provided for @hostAskUnnamed.
  ///
  /// In en, this message translates to:
  /// **'Let the other Device join?'**
  String get hostAskUnnamed;

  /// No description provided for @hostAskText.
  ///
  /// In en, this message translates to:
  /// **'It gets everything on this Device and syncs with it from then on. Say yes only to a Device of your own.'**
  String get hostAskText;

  /// No description provided for @hostAllow.
  ///
  /// In en, this message translates to:
  /// **'Let it join'**
  String get hostAllow;

  /// No description provided for @syncSyncing.
  ///
  /// In en, this message translates to:
  /// **'Syncing…'**
  String get syncSyncing;

  /// No description provided for @syncInvalidCode.
  ///
  /// In en, this message translates to:
  /// **'That is not a pair code. Check it against the other Device\'s screen.'**
  String get syncInvalidCode;

  /// No description provided for @syncNoCode.
  ///
  /// In en, this message translates to:
  /// **'No pair code was found in the photo.'**
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
  /// **'The other Device did not answer. Does it still show the code, and are both on the same Wi-Fi?'**
  String get syncFailUnreachable;

  /// No description provided for @syncFailWrongHost.
  ///
  /// In en, this message translates to:
  /// **'This is not the Device the code belongs to.'**
  String get syncFailWrongHost;

  /// No description provided for @syncFailWrongPin.
  ///
  /// In en, this message translates to:
  /// **'The PIN does not match. Type the code again.'**
  String get syncFailWrongPin;

  /// No description provided for @syncFailLocked.
  ///
  /// In en, this message translates to:
  /// **'Too many wrong tries. Show a new code on the other Device.'**
  String get syncFailLocked;

  /// No description provided for @syncFailDeclined.
  ///
  /// In en, this message translates to:
  /// **'The other Device said no to this one.'**
  String get syncFailDeclined;

  /// No description provided for @syncFailUpdateHere.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix on this Device needs an update to sync with the other Device.'**
  String get syncFailUpdateHere;

  /// No description provided for @syncFailUpdateThere.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix on the other Device needs an update to sync with this one.'**
  String get syncFailUpdateThere;

  /// No description provided for @syncUpdateHereWith.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix on this Device needs an update to sync with {name}.'**
  String syncUpdateHereWith(String name);

  /// No description provided for @syncUpdateThereWith.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix on {name} needs an update to sync with this Device.'**
  String syncUpdateThereWith(String name);

  /// No description provided for @folderNeedsUpdate.
  ///
  /// In en, this message translates to:
  /// **'Another Device writes a newer format into the shared folder. Old Brain Fix on this Device needs an update to read it.'**
  String get folderNeedsUpdate;

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
  /// **'Paired Devices on the same Wi-Fi sync on their own, while the app is open and every quarter of an hour in the background.'**
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

  /// No description provided for @conflictLineText.
  ///
  /// In en, this message translates to:
  /// **'Line of a Checklist'**
  String get conflictLineText;

  /// No description provided for @conflictShoppingLineText.
  ///
  /// In en, this message translates to:
  /// **'Line of a Shopping List'**
  String get conflictShoppingLineText;

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

  /// On a Conflict about a text when one of its versions is in a text format this version of the app does not know.
  ///
  /// In en, this message translates to:
  /// **'A version was written with a newer version of Old Brain Fix. Update the app to pick one.'**
  String get conflictNewerFormat;

  /// Above a Note's or Section's text in the editor when the text is in a format this version of the app does not know; the text then shows as it is stored and cannot be changed.
  ///
  /// In en, this message translates to:
  /// **'This text was written with a newer version of Old Brain Fix. Update the app to change it.'**
  String get textNewerFormat;

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

  /// No description provided for @bundleNeedsUpdate.
  ///
  /// In en, this message translates to:
  /// **'This bundle comes from a newer Old Brain Fix. Old Brain Fix on this Device needs an update to read it.'**
  String get bundleNeedsUpdate;

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
  /// **'Meet the other person. On their Device, open Sync and show the code; scan it or type it here. Their Profile shows to confirm; then they get this Area and nothing else.'**
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

  /// No description provided for @sentItemMakesChecklist.
  ///
  /// In en, this message translates to:
  /// **'It becomes a Checklist.'**
  String get sentItemMakesChecklist;

  /// No description provided for @sentItemMakesPlain.
  ///
  /// In en, this message translates to:
  /// **'It becomes a plain Note again.'**
  String get sentItemMakesPlain;

  /// No description provided for @sentItemLineAdded.
  ///
  /// In en, this message translates to:
  /// **'New Line: {text}'**
  String sentItemLineAdded(String text);

  /// No description provided for @sentItemLineRenamed.
  ///
  /// In en, this message translates to:
  /// **'Line: {before} → {after}'**
  String sentItemLineRenamed(String before, String after);

  /// No description provided for @sentItemLineRemoved.
  ///
  /// In en, this message translates to:
  /// **'Line removed: {text}'**
  String sentItemLineRemoved(String text);

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
  /// **'A Backup is an encrypted file of all your Areas, Notes, photos, voice recordings, Routines and their history. Only your passphrase, or the recovery key on the Recovery Sheet, opens it. This Device\'s settings stay out, except the playback speed. All your Devices share one passphrase and one recovery key, and each writes Backups of its own.'**
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
  /// **'No recovery key was found in the photo.'**
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

  /// No description provided for @personVerified.
  ///
  /// In en, this message translates to:
  /// **'Verified: code compared in person.'**
  String get personVerified;

  /// No description provided for @personNotVerified.
  ///
  /// In en, this message translates to:
  /// **'Not verified: compare codes in person to be sure.'**
  String get personNotVerified;

  /// No description provided for @personCodesDiffered.
  ///
  /// In en, this message translates to:
  /// **'The codes differ: this Profile may not be the person it names.'**
  String get personCodesDiffered;

  /// No description provided for @personForMe.
  ///
  /// In en, this message translates to:
  /// **'Name and picture for me'**
  String get personForMe;

  /// No description provided for @personForMeDetail.
  ///
  /// In en, this message translates to:
  /// **'Only you see them, on your own Devices. Their own name and picture stay as they are.'**
  String get personForMeDetail;

  /// No description provided for @personForMeName.
  ///
  /// In en, this message translates to:
  /// **'Name for me'**
  String get personForMeName;

  /// No description provided for @personForMePicture.
  ///
  /// In en, this message translates to:
  /// **'Change the picture for me'**
  String get personForMePicture;

  /// No description provided for @personTheirPicture.
  ///
  /// In en, this message translates to:
  /// **'Use their own picture'**
  String get personTheirPicture;

  /// No description provided for @profilePeopleEmpty.
  ///
  /// In en, this message translates to:
  /// **'Nobody yet. The people you add from their Profile card, share an Area with or accept Sent Items from show here.'**
  String get profilePeopleEmpty;

  /// The browser's button for the person filter: a menu of "Me" and every person, each one put in or out.
  ///
  /// In en, this message translates to:
  /// **'Show Notes by…'**
  String get personFilter;

  /// No description provided for @personFilterMe.
  ///
  /// In en, this message translates to:
  /// **'Me'**
  String get personFilterMe;

  /// No description provided for @personFilterEveryone.
  ///
  /// In en, this message translates to:
  /// **'Everyone'**
  String get personFilterEveryone;

  /// No description provided for @personFilterShowing.
  ///
  /// In en, this message translates to:
  /// **'Only Notes by'**
  String get personFilterShowing;

  /// On another person's Profile: opens All Notes with only the Notes this person wrote.
  ///
  /// In en, this message translates to:
  /// **'Their Notes'**
  String get personTheirNotes;

  /// Title of the crash screen that replaces the app after an unexpected error.
  ///
  /// In en, this message translates to:
  /// **'That should not have happened'**
  String get crashTitle;

  /// Text of the crash screen, under its title.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix ran into an unexpected error. Your saved Notes are safe. Restart the app, and if this happens again, send the report so it can be fixed.'**
  String get crashBody;

  /// Crash screen button: starts the app again.
  ///
  /// In en, this message translates to:
  /// **'Restart the app'**
  String get crashRestart;

  /// Crash screen button: opens a mail to the developer with the error report.
  ///
  /// In en, this message translates to:
  /// **'Send report to the developer'**
  String get crashSendReport;

  /// Announcement at the next start after the app died; tapping it opens a mail with the report.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix stopped unexpectedly last time. Tap here to send a short report to the developer.'**
  String get crashLastRunBody;

  /// No description provided for @sendToPerson.
  ///
  /// In en, this message translates to:
  /// **'Send to a person'**
  String get sendToPerson;

  /// No description provided for @addToCalendar.
  ///
  /// In en, this message translates to:
  /// **'Add to calendar'**
  String get addToCalendar;

  /// No description provided for @deleteNoteQuestion.
  ///
  /// In en, this message translates to:
  /// **'Delete this Note?'**
  String get deleteNoteQuestion;

  /// No description provided for @deleteNoteDetail.
  ///
  /// In en, this message translates to:
  /// **'It stays in the trash for 30 days.'**
  String get deleteNoteDetail;

  /// No description provided for @deleteNoteOnly.
  ///
  /// In en, this message translates to:
  /// **'Delete only the Note'**
  String get deleteNoteOnly;

  /// No description provided for @deleteNoteWithLinked.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Delete with its Routine or Reminder} other{Delete with its {count} Routines and Reminders}}'**
  String deleteNoteWithLinked(int count);

  /// No description provided for @removeRecordingKeepText.
  ///
  /// In en, this message translates to:
  /// **'Remove recording, keep text'**
  String get removeRecordingKeepText;

  /// No description provided for @removePhotoKeepText.
  ///
  /// In en, this message translates to:
  /// **'Remove photo, keep text'**
  String get removePhotoKeepText;

  /// No description provided for @removeRecordingQuestion.
  ///
  /// In en, this message translates to:
  /// **'Remove this recording and keep its Transcript?'**
  String get removeRecordingQuestion;

  /// No description provided for @removePhotoQuestion.
  ///
  /// In en, this message translates to:
  /// **'Remove this photo and keep its Photo Text?'**
  String get removePhotoQuestion;

  /// No description provided for @removeMediaDetail.
  ///
  /// In en, this message translates to:
  /// **'The file goes from all your Devices and from everyone you share the Note\'s Area with. The text stays on the Note.'**
  String get removeMediaDetail;

  /// No description provided for @remove.
  ///
  /// In en, this message translates to:
  /// **'Remove'**
  String get remove;

  /// Button under a Transcript or Photo Text that makes it the Note's text.
  ///
  /// In en, this message translates to:
  /// **'Replace Note text'**
  String get replaceNoteText;

  /// Button under a Transcript or Photo Text that adds it to the end of the Note's text.
  ///
  /// In en, this message translates to:
  /// **'Append to Note text'**
  String get appendToNoteText;

  /// Button under a Transcript or Photo Text that removes the text and keeps the recording or photo.
  ///
  /// In en, this message translates to:
  /// **'Remove generated text'**
  String get removeGeneratedText;

  /// Button under a voice recording that makes its Transcript anew.
  ///
  /// In en, this message translates to:
  /// **'Transcribe again'**
  String get transcribeAgain;

  /// Button under a photo that makes its Photo Text anew.
  ///
  /// In en, this message translates to:
  /// **'Read the text again'**
  String get readTextAgain;

  /// No description provided for @removeTranscriptQuestion.
  ///
  /// In en, this message translates to:
  /// **'Remove this Transcript?'**
  String get removeTranscriptQuestion;

  /// No description provided for @removePhotoTextQuestion.
  ///
  /// In en, this message translates to:
  /// **'Remove this Photo Text?'**
  String get removePhotoTextQuestion;

  /// No description provided for @removeGeneratedTextDetail.
  ///
  /// In en, this message translates to:
  /// **'Search no longer finds the text, on all your Devices. The file stays, and the text can be made from it again.'**
  String get removeGeneratedTextDetail;

  /// Announcement when Transcribe again brought no Transcript.
  ///
  /// In en, this message translates to:
  /// **'A recording could not be transcribed.'**
  String get transcribeAgainFailed;

  /// No description provided for @transcribeAgainFailedTitle.
  ///
  /// In en, this message translates to:
  /// **'Recording not transcribed'**
  String get transcribeAgainFailedTitle;

  /// No description provided for @transcribeAgainFailedCause.
  ///
  /// In en, this message translates to:
  /// **'Whisper made no text from this recording.'**
  String get transcribeAgainFailedCause;

  /// Announcement when Read the text again brought no Photo Text.
  ///
  /// In en, this message translates to:
  /// **'The text in a photo could not be read.'**
  String get readTextAgainFailed;

  /// No description provided for @readTextAgainFailedTitle.
  ///
  /// In en, this message translates to:
  /// **'Photo not read'**
  String get readTextAgainFailedTitle;

  /// No description provided for @readTextAgainFailedCause.
  ///
  /// In en, this message translates to:
  /// **'The text recognition could not read this photo.'**
  String get readTextAgainFailedCause;

  /// No description provided for @generatedTextAgainFix.
  ///
  /// In en, this message translates to:
  /// **'The file stays on the Note; try again later.'**
  String get generatedTextAgainFix;

  /// Heading of the menu that chooses how fast recordings play.
  ///
  /// In en, this message translates to:
  /// **'Playback speed'**
  String get playbackSpeed;

  /// The current playback speed beside a play button, as in "1.5×"; speed is the number as the locale writes it.
  ///
  /// In en, this message translates to:
  /// **'{speed}×'**
  String playbackSpeedLabel(String speed);

  /// One speed in the playback speed menu, as in "×1.5"; speed is the number as the locale writes it.
  ///
  /// In en, this message translates to:
  /// **'×{speed}'**
  String playbackSpeedOption(String speed);

  /// What a screen reader says for the speed label.
  ///
  /// In en, this message translates to:
  /// **'Playback speed {speed} times'**
  String playbackSpeedSemantics(String speed);

  /// Closes a dialog.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get close;

  /// Button in a failure dialog that mails its details to the developer.
  ///
  /// In en, this message translates to:
  /// **'Report'**
  String get report;

  /// Announcement after a sound file was shared in that this phone cannot transcribe; the Note is kept. The count only picks the grammar and is never shown.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{A shared recording cannot be transcribed.} other{Shared recordings cannot be transcribed.}}'**
  String sharedRecordingFailed(int count);

  /// Title of the dialog explaining why a shared recording cannot be transcribed.
  ///
  /// In en, this message translates to:
  /// **'Recording not transcribed'**
  String get sharedRecordingFailedTitle;

  /// Cause of a shared recording that cannot be transcribed.
  ///
  /// In en, this message translates to:
  /// **'No sound was found in the shared file: it is not a sound file, or it is damaged.'**
  String get sharedRecordingUnreadable;

  /// Cause of a shared recording that cannot be transcribed.
  ///
  /// In en, this message translates to:
  /// **'This phone cannot decode the format of the shared file.'**
  String get sharedRecordingUnsupported;

  /// What to do about a shared recording that cannot be transcribed.
  ///
  /// In en, this message translates to:
  /// **'The file stays in the Note as it is; type what it says into the Note, or share it again as m4a or mp3.'**
  String get sharedRecordingFix;

  /// On another person's Profile page: blocks them, so nothing they write is kept or shown on the user's Devices.
  ///
  /// In en, this message translates to:
  /// **'Block'**
  String get personBlock;

  /// On a blocked person's Profile page: ends the block.
  ///
  /// In en, this message translates to:
  /// **'Unblock'**
  String get personUnblock;

  /// Line on a blocked person's Profile page.
  ///
  /// In en, this message translates to:
  /// **'Blocked: nothing they write is kept or shown, only their check-offs.'**
  String get personBlocked;

  /// Title of the dialog that confirms blocking a person.
  ///
  /// In en, this message translates to:
  /// **'Block {name}?'**
  String personBlockQuestion(String name);

  /// Body of the dialog that confirms blocking a person who manages no Shared Area the user is in.
  ///
  /// In en, this message translates to:
  /// **'Everything {name} wrote is deleted from your Devices, except their check-offs, and nothing more of theirs is kept. A shared folder can still hold their encrypted files, which the app never opens; leaving the folder removes them.'**
  String personBlockDetail(String name);

  /// Body of the dialog that confirms blocking a person who manages Shared Areas the user is in; areas are their names, separated by commas.
  ///
  /// In en, this message translates to:
  /// **'Everything {name} wrote is deleted from your Devices, except their check-offs, and you leave {areas}, which they manage. A shared folder can still hold their encrypted files, which the app never opens; leaving the folder removes them.'**
  String personBlockLeaveDetail(String name, String areas);

  /// Confirms blocking a person together with leaving the Shared Areas they manage.
  ///
  /// In en, this message translates to:
  /// **'Block and leave'**
  String get personBlockLeave;

  /// Title of the dialog that confirms ending a block.
  ///
  /// In en, this message translates to:
  /// **'Unblock {name}?'**
  String personUnblockQuestion(String name);

  /// No description provided for @personUnblockDetail.
  ///
  /// In en, this message translates to:
  /// **'What they wrote comes back with the next sync with a Device that still holds it.'**
  String get personUnblockDetail;

  /// The folded section of People that lists the blocked people.
  ///
  /// In en, this message translates to:
  /// **'Blocked'**
  String get peopleBlocked;

  /// In a Sent Item's preview: keeps nothing of it and blocks its sender.
  ///
  /// In en, this message translates to:
  /// **'Decline and block'**
  String get sentItemBlock;

  /// No description provided for @sentItemBlocked.
  ///
  /// In en, this message translates to:
  /// **'Declined and blocked: nothing was kept.'**
  String get sentItemBlocked;

  /// The cause in the dialog behind 'sentItemNotTaken' when a Sent Item comes from a blocked person; shows nothing of it.
  ///
  /// In en, this message translates to:
  /// **'This file comes from a person you blocked.'**
  String get sentItemFromBlocked;

  /// In the hidden preview of a stranger's Sent Item, under who it is from.
  ///
  /// In en, this message translates to:
  /// **'An unknown person: not among your People.'**
  String get sentItemStranger;

  /// No description provided for @sentItemHidden.
  ///
  /// In en, this message translates to:
  /// **'What they sent stays hidden until you tap Show.'**
  String get sentItemHidden;

  /// Reveals what a stranger's Sent Item holds.
  ///
  /// In en, this message translates to:
  /// **'Show'**
  String get sentItemShow;

  /// No description provided for @sentItemGone.
  ///
  /// In en, this message translates to:
  /// **'This Sent Item is no longer waiting: it was accepted or declined.'**
  String get sentItemGone;

  /// Agenda line while strangers' Sent Items wait to be shown, accepted or declined.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 Sent Item waiting} other{{count} Sent Items waiting}}'**
  String sentItemsWaiting(int count);

  /// No description provided for @sentItemsWaitingTitle.
  ///
  /// In en, this message translates to:
  /// **'Sent Items waiting'**
  String get sentItemsWaitingTitle;

  /// No description provided for @sentItemsNoneWaiting.
  ///
  /// In en, this message translates to:
  /// **'No Sent Items are waiting.'**
  String get sentItemsNoneWaiting;

  /// On the Profile page: shares the own Profile card as a link through the app the user picks.
  ///
  /// In en, this message translates to:
  /// **'Send my Profile'**
  String get profileCardSend;

  /// Under 'Send my Profile'.
  ///
  /// In en, this message translates to:
  /// **'As a link, through any messenger.'**
  String get profileCardSendDetail;

  /// On the Profile page: shows the own Profile card as a QR code.
  ///
  /// In en, this message translates to:
  /// **'Show my QR'**
  String get profileCardShowQr;

  /// Under 'Show my QR'.
  ///
  /// In en, this message translates to:
  /// **'For someone next to you to scan.'**
  String get profileCardShowQrDetail;

  /// The text shared with the Profile card. The link must stay last, with nothing after it.
  ///
  /// In en, this message translates to:
  /// **'My Profile for Old Brain Fix:\n{link}'**
  String profileCardText(String link);

  /// The fix in the dialog behind 'profileCardNotMade': this Device's key is not bound to the Profile until it synced with another of the user's Devices.
  ///
  /// In en, this message translates to:
  /// **'Sync with your other Device first, then try again.'**
  String get profileCardNotYet;

  /// The cause in the dialog behind 'profileCardNotSent': no app took the shared Profile card.
  ///
  /// In en, this message translates to:
  /// **'No app here can take the link.'**
  String get profileCardNoApp;

  /// Title of the page showing the own Profile card as a QR code.
  ///
  /// In en, this message translates to:
  /// **'My Profile QR'**
  String get profileQrTitle;

  /// Under the Profile card's QR code.
  ///
  /// In en, this message translates to:
  /// **'The other person scans this in Old Brain Fix, under Profile, Add a person. Scanned in person, you are verified for them at once.'**
  String get profileQrHint;

  /// On the QR page when the card does not fit a QR code.
  ///
  /// In en, this message translates to:
  /// **'Your Profile has too many Devices for one QR. Send it as a link instead.'**
  String get profileQrTooLong;

  /// In People on the Profile page: opens the page to add a person from their Profile card.
  ///
  /// In en, this message translates to:
  /// **'Add a person'**
  String get personAdd;

  /// Under 'Add a person'.
  ///
  /// In en, this message translates to:
  /// **'From their Profile link or QR.'**
  String get personAddDetail;

  /// At the top of the 'Add a person' page.
  ///
  /// In en, this message translates to:
  /// **'Ask the person for their Profile: as a link through a messenger, or as a QR on their phone next to you. A QR scanned in person marks them verified at once.'**
  String get personAddIntro;

  /// Takes a photo of the other person's Profile QR with the camera.
  ///
  /// In en, this message translates to:
  /// **'Scan their QR'**
  String get personAddScan;

  /// Reads a Profile QR from a picture the user picks, such as a screenshot.
  ///
  /// In en, this message translates to:
  /// **'QR from a picture'**
  String get personAddPicture;

  /// Label of the field the user pastes a Profile card link into.
  ///
  /// In en, this message translates to:
  /// **'Their Profile link'**
  String get personAddLink;

  /// Opens the pasted Profile card link.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get personAddOpen;

  /// The cause in the dialog behind 'cardNotRead': a photo or picture shows no QR code.
  ///
  /// In en, this message translates to:
  /// **'No QR code was found in the picture.'**
  String get cardNoQr;

  /// When a Profile card cannot be read or its signature does not hold.
  ///
  /// In en, this message translates to:
  /// **'This is not a Profile card, or it was changed on the way. Ask for the link again.'**
  String get cardUnreadable;

  /// When the user opens their own Profile card.
  ///
  /// In en, this message translates to:
  /// **'This is your own Profile card.'**
  String get cardOwn;

  /// When the Profile card is a blocked person's.
  ///
  /// In en, this message translates to:
  /// **'This card is from a person you blocked.'**
  String get cardBlocked;

  /// When the Profile card names a person kept here under other keys.
  ///
  /// In en, this message translates to:
  /// **'This card names a person in People but was not made by them.'**
  String get cardRefused;

  /// On a person's Profile just added from their card: shares the own Profile card back.
  ///
  /// In en, this message translates to:
  /// **'Send yours back'**
  String get personSendBack;

  /// Above 'Send yours back'.
  ///
  /// In en, this message translates to:
  /// **'So {name} can add you too.'**
  String personSendBackDetail(String name);

  /// On the Share page: heading of sharing with a person in People through a file.
  ///
  /// In en, this message translates to:
  /// **'Someone not nearby'**
  String get shareAfarHeading;

  /// Under 'Someone not nearby' on the Share page.
  ///
  /// In en, this message translates to:
  /// **'Share it with a person in People. They get a file through any messenger and accept or decline it.'**
  String get shareAfarIntro;

  /// Button on the Share page: opens the list of People to share the Area with.
  ///
  /// In en, this message translates to:
  /// **'Share with…'**
  String get shareAfarPick;

  /// On the Share page when no person in People can be sent an Area.
  ///
  /// In en, this message translates to:
  /// **'Nobody in People can get it yet. Add the person from their Profile card first.'**
  String get shareAfarNobody;

  /// Title of the page that lists the People an Area can be shared with.
  ///
  /// In en, this message translates to:
  /// **'Share {area} with…'**
  String shareWithTitle(String area);

  /// Under a person's name in 'Share with…' when their code was never compared in person.
  ///
  /// In en, this message translates to:
  /// **'Not verified'**
  String get shareNotVerified;

  /// Dialog title, asked once per person when sharing an Area with them from afar.
  ///
  /// In en, this message translates to:
  /// **'How do changes travel with {name}?'**
  String transportTitle(String name);

  /// Choice in 'How do changes travel'.
  ///
  /// In en, this message translates to:
  /// **'A shared folder you both reach'**
  String get transportFolder;

  /// Under 'A shared folder you both reach'.
  ///
  /// In en, this message translates to:
  /// **'Nextcloud, Syncthing or a USB folder; changes travel on their own.'**
  String get transportFolderDetail;

  /// Choice in 'How do changes travel'.
  ///
  /// In en, this message translates to:
  /// **'Files through a messenger'**
  String get transportBundle;

  /// Under 'Files through a messenger'.
  ///
  /// In en, this message translates to:
  /// **'You send changes as a file from their Profile page.'**
  String get transportBundleDetail;

  /// Announced when sharing an Area from afar failed; the tap opens the dialog with the cause and the fix.
  ///
  /// In en, this message translates to:
  /// **'The Area could not be shared.'**
  String get shareAfarFailed;

  /// The cause in the dialog behind 'areaOfferNotTaken' when a received Area file was sealed for other Devices.
  ///
  /// In en, this message translates to:
  /// **'This shared Area is meant for another person or another of your Devices.'**
  String get areaOfferNotForUs;

  /// The cause in the dialog behind 'areaOfferNotTaken' when a received Area file comes from a blocked person.
  ///
  /// In en, this message translates to:
  /// **'This shared Area comes from a person you blocked.'**
  String get areaOfferFromBlocked;

  /// The cause in the dialog behind 'areaOfferNotTaken' when a received Area file offers an Area this Device holds as its own.
  ///
  /// In en, this message translates to:
  /// **'This Area is your own on this Device already.'**
  String get areaOfferRefused;

  /// The cause in the dialog behind 'areaOfferNotTaken' when a received Area file is damaged or not signed by its sender.
  ///
  /// In en, this message translates to:
  /// **'This file holds no shared Area this app can read.'**
  String get notAnAreaOffer;

  /// Announcement when an Area file came in while the app was locked.
  ///
  /// In en, this message translates to:
  /// **'A shared Area is waiting on the Agenda.'**
  String get areaOfferWaits;

  /// Title of the preview of an Area shared with the user from afar.
  ///
  /// In en, this message translates to:
  /// **'Shared Area'**
  String get areaOfferTitle;

  /// Preview of an Area shared from afar, e.g. 'Ann shares Household with you as Member'.
  ///
  /// In en, this message translates to:
  /// **'{name} shares {area} with you as {role}'**
  String areaOfferText(String name, String area, String role);

  /// Stands for the Area's name in 'shares … with you' when the Area has none.
  ///
  /// In en, this message translates to:
  /// **'an Area'**
  String get areaOfferNoName;

  /// In the preview of a shared Area.
  ///
  /// In en, this message translates to:
  /// **'Changes travel through a shared folder you both reach.'**
  String get areaOfferByFolder;

  /// In the preview of a shared Area.
  ///
  /// In en, this message translates to:
  /// **'Changes travel as files through a messenger.'**
  String get areaOfferByBundle;

  /// In the preview of a shared Area.
  ///
  /// In en, this message translates to:
  /// **'Nothing of the Area is kept until you accept.'**
  String get areaOfferKeepsNothing;

  /// Announcement after accepting a shared Area.
  ///
  /// In en, this message translates to:
  /// **'Shared Area accepted.'**
  String get areaOfferAccepted;

  /// Announcement when a waiting shared Area was decided meanwhile.
  ///
  /// In en, this message translates to:
  /// **'This shared Area is no longer waiting: it was accepted or declined.'**
  String get areaOfferGone;

  /// Dialog after accepting a shared Area whose changes travel by a folder, when none is chosen here.
  ///
  /// In en, this message translates to:
  /// **'Choose the shared folder?'**
  String get areaOfferFolderQuestion;

  /// Body of 'Choose the shared folder?'.
  ///
  /// In en, this message translates to:
  /// **'Changes of {area} travel through a folder you both reach. Choose it now, or later under Sync.'**
  String areaOfferFolderDetail(String area);

  /// Button in 'Choose the shared folder?'.
  ///
  /// In en, this message translates to:
  /// **'Later'**
  String get areaOfferFolderLater;

  /// Agenda line while Areas shared from afar wait to be accepted or declined.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 shared Area waiting} other{{count} shared Areas waiting}}'**
  String areaOffersWaiting(int count);

  /// Title of the list of Areas shared from afar that wait.
  ///
  /// In en, this message translates to:
  /// **'Shared Areas waiting'**
  String get areaOffersWaitingTitle;

  /// Empty list of waiting shared Areas.
  ///
  /// In en, this message translates to:
  /// **'No shared Areas are waiting.'**
  String get areaOffersNoneWaiting;

  /// On a person's Profile page when changes travel with them by files: shares a bundle of the Shared Areas.
  ///
  /// In en, this message translates to:
  /// **'Send changes to {name}'**
  String personSendChanges(String name);

  /// Under 'Send changes to …'.
  ///
  /// In en, this message translates to:
  /// **'As a file through any messenger.'**
  String get personSendChangesDetail;

  /// The Changes Nudge's notification: people synced by bundles lack changes. names is a list such as 'Bob and Carol'.
  ///
  /// In en, this message translates to:
  /// **'Changes for {names}'**
  String changesNudgeTitle(String names);

  /// Under the Changes Nudge's title.
  ///
  /// In en, this message translates to:
  /// **'Tap to send them.'**
  String get changesNudgeBody;

  /// The Changes Nudge on the lock screen, without the names.
  ///
  /// In en, this message translates to:
  /// **'Changes are waiting to be sent'**
  String get changesNudgePublic;

  /// Joins a list of names: first holds all but the last, separated by commas.
  ///
  /// In en, this message translates to:
  /// **'{first} and {last}'**
  String namesAnd(String first, String last);

  /// A Note's job with no time, done once (CONTEXT.md: Frog).
  ///
  /// In en, this message translates to:
  /// **'Frog'**
  String get frog;

  /// Heading of the Agenda section that offers two Frogs a day; tapping it opens the Frog view.
  ///
  /// In en, this message translates to:
  /// **'Eat a frog'**
  String get eatAFrog;

  /// Swaps a Frog offered on the Agenda for the next one.
  ///
  /// In en, this message translates to:
  /// **'Another one'**
  String get anotherFrog;

  /// No description provided for @frogsTitle.
  ///
  /// In en, this message translates to:
  /// **'Frogs'**
  String get frogsTitle;

  /// The folded section of the Frog view with the Frogs checked off.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get frogsDone;

  /// No description provided for @frogDoneOn.
  ///
  /// In en, this message translates to:
  /// **'Done on {date}'**
  String frogDoneOn(String date);

  /// No description provided for @frogsEmpty.
  ///
  /// In en, this message translates to:
  /// **'No open Frogs. \"Routine, Reminder, Frog or Session\" on a Note\'s card makes one.'**
  String get frogsEmpty;

  /// A Frog from a locked Area set to Masked, on the Agenda.
  ///
  /// In en, this message translates to:
  /// **'Private Frog'**
  String get maskedFrog;

  /// No description provided for @deleteFrogQuestion.
  ///
  /// In en, this message translates to:
  /// **'Delete this Frog? It stays in the trash. Its Note stays.'**
  String get deleteFrogQuestion;

  /// A Note card's menu entry that gives the Note its first Tally field (CONTEXT.md: Tally).
  ///
  /// In en, this message translates to:
  /// **'Make it a Tally'**
  String get makeTally;

  /// Records a new entry in a Tally (CONTEXT.md: Entry).
  ///
  /// In en, this message translates to:
  /// **'Add entry'**
  String get addTallyEntry;

  /// Makes a Note a Checklist: its first line stays its text, every other line becomes a Line (CONTEXT.md: Checklist, Line).
  ///
  /// In en, this message translates to:
  /// **'Make it a Checklist'**
  String get makeChecklist;

  /// Turns a Checklist back into a plain Note, every Line a line of its text.
  ///
  /// In en, this message translates to:
  /// **'Back to a plain Note'**
  String get checklistToPlain;

  /// Unchecks every Line of a Checklist, to use it again.
  ///
  /// In en, this message translates to:
  /// **'Uncheck all'**
  String get checklistUncheckAll;

  /// No description provided for @checklistHeading.
  ///
  /// In en, this message translates to:
  /// **'Checklist'**
  String get checklistHeading;

  /// A compact card of a Checklist: how many of its Lines are checked.
  ///
  /// In en, this message translates to:
  /// **'{checked} of {count} checked'**
  String checklistCount(int checked, int count);

  /// The hint of the field that adds Lines to a Checklist; each line typed becomes one.
  ///
  /// In en, this message translates to:
  /// **'New Line'**
  String get checklistNewLine;

  /// No description provided for @checklistAddLine.
  ///
  /// In en, this message translates to:
  /// **'Add Line'**
  String get checklistAddLine;

  /// No description provided for @checklistRenameTitle.
  ///
  /// In en, this message translates to:
  /// **'Rename Line'**
  String get checklistRenameTitle;

  /// No description provided for @checklistMoveUp.
  ///
  /// In en, this message translates to:
  /// **'Move up'**
  String get checklistMoveUp;

  /// No description provided for @checklistMoveDown.
  ///
  /// In en, this message translates to:
  /// **'Move down'**
  String get checklistMoveDown;

  /// No description provided for @checklistIndent.
  ///
  /// In en, this message translates to:
  /// **'Put under the Line above'**
  String get checklistIndent;

  /// Takes a Line out of the group it is in.
  ///
  /// In en, this message translates to:
  /// **'Back to the top level'**
  String get checklistOutdent;

  /// No description provided for @checklistDeleteQuestion.
  ///
  /// In en, this message translates to:
  /// **'Delete this Line?'**
  String get checklistDeleteQuestion;

  /// Makes a Note a Shopping List: its first line stays its text, every other line becomes a Line that comes back a while after it is checked (CONTEXT.md: Shopping List, Line).
  ///
  /// In en, this message translates to:
  /// **'Make it a Shopping List'**
  String get makeShoppingList;

  /// No description provided for @shoppingHeading.
  ///
  /// In en, this message translates to:
  /// **'Shopping List'**
  String get shoppingHeading;

  /// A compact card of a Shopping List: how many of its Lines are not struck through.
  ///
  /// In en, this message translates to:
  /// **'{count} open'**
  String shoppingOpenCount(int count);

  /// Under a struck Line of a Shopping List: the day it is open again, as a short weekday and date such as 'Tue, Nov 12'.
  ///
  /// In en, this message translates to:
  /// **'back {day}'**
  String shoppingBack(String day);

  /// Heading of the choices in a Shopping List Line's menu: how long the Line stays struck once checked (CONTEXT.md: Shopping List).
  ///
  /// In en, this message translates to:
  /// **'Reappear time'**
  String get shoppingReappear;

  /// Reappear time of a Line that is gone once checked and does not come back.
  ///
  /// In en, this message translates to:
  /// **'Once'**
  String get shoppingOnce;

  /// Reappear time of a Line: it comes back this many weeks after it was checked.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 week} other{{count} weeks}}'**
  String shoppingWeeks(int count);

  /// Removes a Line from a Shopping List, so it never comes back.
  ///
  /// In en, this message translates to:
  /// **'Delete for good'**
  String get shoppingDeleteForGood;

  /// No description provided for @tallyHeading.
  ///
  /// In en, this message translates to:
  /// **'Tally'**
  String get tallyHeading;

  /// No description provided for @tallyNewField.
  ///
  /// In en, this message translates to:
  /// **'New field'**
  String get tallyNewField;

  /// No description provided for @tallyField.
  ///
  /// In en, this message translates to:
  /// **'Field'**
  String get tallyField;

  /// No description provided for @tallyFieldName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get tallyFieldName;

  /// No description provided for @tallyFieldNameHint.
  ///
  /// In en, this message translates to:
  /// **'e.g. Push-ups, Pages read'**
  String get tallyFieldNameHint;

  /// No description provided for @tallyFieldNameMissing.
  ///
  /// In en, this message translates to:
  /// **'Give the field a name.'**
  String get tallyFieldNameMissing;

  /// No description provided for @tallyUnit.
  ///
  /// In en, this message translates to:
  /// **'Unit'**
  String get tallyUnit;

  /// The unit option that takes a unit as typed, such as pages or times.
  ///
  /// In en, this message translates to:
  /// **'Another unit'**
  String get tallyUnitTyped;

  /// No description provided for @tallyUnitTypedHint.
  ///
  /// In en, this message translates to:
  /// **'e.g. pages, times'**
  String get tallyUnitTypedHint;

  /// No description provided for @tallyUnitFixed.
  ///
  /// In en, this message translates to:
  /// **'Entries hold values for this field, so its kind of unit stays.'**
  String get tallyUnitFixed;

  /// No description provided for @unitKindMass.
  ///
  /// In en, this message translates to:
  /// **'Mass'**
  String get unitKindMass;

  /// No description provided for @unitKindDistance.
  ///
  /// In en, this message translates to:
  /// **'Distance'**
  String get unitKindDistance;

  /// No description provided for @unitKindSpeed.
  ///
  /// In en, this message translates to:
  /// **'Speed'**
  String get unitKindSpeed;

  /// No description provided for @unitKindTemperature.
  ///
  /// In en, this message translates to:
  /// **'Temperature'**
  String get unitKindTemperature;

  /// No description provided for @unitKindAngle.
  ///
  /// In en, this message translates to:
  /// **'Angle'**
  String get unitKindAngle;

  /// No description provided for @unitKindTime.
  ///
  /// In en, this message translates to:
  /// **'Time'**
  String get unitKindTime;

  /// No description provided for @tallyTarget.
  ///
  /// In en, this message translates to:
  /// **'Target (optional)'**
  String get tallyTarget;

  /// No description provided for @tallyTargetHint.
  ///
  /// In en, this message translates to:
  /// **'Prefilled in every new entry'**
  String get tallyTargetHint;

  /// No description provided for @tallyNotANumber.
  ///
  /// In en, this message translates to:
  /// **'Not a number. Type it like 12, 2.5 or 1:30.'**
  String get tallyNotANumber;

  /// No description provided for @tallyRule.
  ///
  /// In en, this message translates to:
  /// **'A day with several entries counts'**
  String get tallyRule;

  /// No description provided for @tallyRuleSum.
  ///
  /// In en, this message translates to:
  /// **'Their sum'**
  String get tallyRuleSum;

  /// No description provided for @tallyRuleMax.
  ///
  /// In en, this message translates to:
  /// **'The largest'**
  String get tallyRuleMax;

  /// No description provided for @tallyHideField.
  ///
  /// In en, this message translates to:
  /// **'Hide field'**
  String get tallyHideField;

  /// No description provided for @tallyHideFieldDetail.
  ///
  /// In en, this message translates to:
  /// **'Its values stay stored and return when it is shown again.'**
  String get tallyHideFieldDetail;

  /// No description provided for @tallyShowField.
  ///
  /// In en, this message translates to:
  /// **'Show field again'**
  String get tallyShowField;

  /// No description provided for @tallyFieldHidden.
  ///
  /// In en, this message translates to:
  /// **'Hidden'**
  String get tallyFieldHidden;

  /// No description provided for @tallyAddField.
  ///
  /// In en, this message translates to:
  /// **'Add field'**
  String get tallyAddField;

  /// No description provided for @tallyFields.
  ///
  /// In en, this message translates to:
  /// **'Fields'**
  String get tallyFields;

  /// No description provided for @tallyNewEntry.
  ///
  /// In en, this message translates to:
  /// **'New entry'**
  String get tallyNewEntry;

  /// No description provided for @tallyEntry.
  ///
  /// In en, this message translates to:
  /// **'Entry'**
  String get tallyEntry;

  /// No description provided for @tallyEntryDate.
  ///
  /// In en, this message translates to:
  /// **'Date'**
  String get tallyEntryDate;

  /// No description provided for @tallyEntryLine.
  ///
  /// In en, this message translates to:
  /// **'Line of text'**
  String get tallyEntryLine;

  /// No description provided for @tallyEntryEmpty.
  ///
  /// In en, this message translates to:
  /// **'Enter a value or a line of text.'**
  String get tallyEntryEmpty;

  /// No description provided for @tallyPeriod.
  ///
  /// In en, this message translates to:
  /// **'One bar per'**
  String get tallyPeriod;

  /// No description provided for @tallyPeriodDay.
  ///
  /// In en, this message translates to:
  /// **'Day'**
  String get tallyPeriodDay;

  /// No description provided for @tallyPeriodWeek.
  ///
  /// In en, this message translates to:
  /// **'Week'**
  String get tallyPeriodWeek;

  /// No description provided for @tallyPeriodMonth.
  ///
  /// In en, this message translates to:
  /// **'Month'**
  String get tallyPeriodMonth;

  /// No description provided for @tallyPeriodYear.
  ///
  /// In en, this message translates to:
  /// **'Year'**
  String get tallyPeriodYear;

  /// No description provided for @tallyShowEntries.
  ///
  /// In en, this message translates to:
  /// **'Show entries'**
  String get tallyShowEntries;

  /// No description provided for @tallyEntriesMine.
  ///
  /// In en, this message translates to:
  /// **'Mine'**
  String get tallyEntriesMine;

  /// No description provided for @tallyEntriesEveryone.
  ///
  /// In en, this message translates to:
  /// **'Everyone\'s'**
  String get tallyEntriesEveryone;

  /// No description provided for @tallyEntries.
  ///
  /// In en, this message translates to:
  /// **'Entries'**
  String get tallyEntries;

  /// No description provided for @tallyNoEntries.
  ///
  /// In en, this message translates to:
  /// **'No entries yet.'**
  String get tallyNoEntries;

  /// No description provided for @tallyChartEmpty.
  ///
  /// In en, this message translates to:
  /// **'No values yet.'**
  String get tallyChartEmpty;

  /// No description provided for @tallyChartRange.
  ///
  /// In en, this message translates to:
  /// **'{from} to {to}'**
  String tallyChartRange(String from, String to);

  /// No description provided for @tallyChartHighest.
  ///
  /// In en, this message translates to:
  /// **'Highest bar: {value}'**
  String tallyChartHighest(String value);

  /// No description provided for @tallyChartLabel.
  ///
  /// In en, this message translates to:
  /// **'Chart of {field}, {bars} bars'**
  String tallyChartLabel(String field, int bars);

  /// No description provided for @tallyTargetValue.
  ///
  /// In en, this message translates to:
  /// **'Target {value}'**
  String tallyTargetValue(String value);

  /// No description provided for @tallyMoreEntries.
  ///
  /// In en, this message translates to:
  /// **'Show more'**
  String get tallyMoreEntries;

  /// No description provided for @tallyRemovedEntries.
  ///
  /// In en, this message translates to:
  /// **'Removed entries'**
  String get tallyRemovedEntries;

  /// No description provided for @deleteEntryQuestion.
  ///
  /// In en, this message translates to:
  /// **'Delete this entry? It can be restored under \"Removed entries\".'**
  String get deleteEntryQuestion;

  /// No description provided for @edit.
  ///
  /// In en, this message translates to:
  /// **'Edit'**
  String get edit;

  /// No description provided for @settingsUnits.
  ///
  /// In en, this message translates to:
  /// **'Units'**
  String get settingsUnits;

  /// No description provided for @settingsUnitsDetail.
  ///
  /// In en, this message translates to:
  /// **'How Tallies show known units. Values are stored in metric either way.'**
  String get settingsUnitsDetail;

  /// No description provided for @unitSystemRegion.
  ///
  /// In en, this message translates to:
  /// **'As the phone\'s region: {system}'**
  String unitSystemRegion(String system);

  /// No description provided for @unitSystemMetric.
  ///
  /// In en, this message translates to:
  /// **'Metric (kg, km, °C)'**
  String get unitSystemMetric;

  /// No description provided for @unitSystemImperial.
  ///
  /// In en, this message translates to:
  /// **'Imperial (lb, mi, °F)'**
  String get unitSystemImperial;

  /// Fills unitSystemRegion.
  ///
  /// In en, this message translates to:
  /// **'metric'**
  String get unitSystemMetricName;

  /// No description provided for @unitSystemImperialName.
  ///
  /// In en, this message translates to:
  /// **'imperial'**
  String get unitSystemImperialName;

  /// A Routine that picks Tallies from a pool for each Occurrence (CONTEXT.md: Session).
  ///
  /// In en, this message translates to:
  /// **'Session'**
  String get session;

  /// No description provided for @newSession.
  ///
  /// In en, this message translates to:
  /// **'New Session'**
  String get newSession;

  /// The Tallies a Session picks from (CONTEXT.md: Session).
  ///
  /// In en, this message translates to:
  /// **'Pool'**
  String get sessionPool;

  /// No description provided for @sessionPoolExplain.
  ///
  /// In en, this message translates to:
  /// **'The Tallies it picks from.'**
  String get sessionPoolExplain;

  /// No description provided for @sessionPoolOrder.
  ///
  /// In en, this message translates to:
  /// **'Rolling goes through them from the top; drag a handle to change the order.'**
  String get sessionPoolOrder;

  /// No description provided for @sessionPoolNone.
  ///
  /// In en, this message translates to:
  /// **'No Tallies yet. \"Make it a Tally\" in a Note\'s ⋮ makes one.'**
  String get sessionPoolNone;

  /// No description provided for @sessionPoolMissing.
  ///
  /// In en, this message translates to:
  /// **'Choose at least one Tally.'**
  String get sessionPoolMissing;

  /// The drag handle of a Tally in a Session's pool.
  ///
  /// In en, this message translates to:
  /// **'Move {name}'**
  String sessionMove(String name);

  /// No description provided for @sessionCount.
  ///
  /// In en, this message translates to:
  /// **'How many it picks each time'**
  String get sessionCount;

  /// No description provided for @sessionCountInvalid.
  ///
  /// In en, this message translates to:
  /// **'Type a number from 1 to 99.'**
  String get sessionCountInvalid;

  /// No description provided for @sessionWeekdayCounts.
  ///
  /// In en, this message translates to:
  /// **'Count per weekday'**
  String get sessionWeekdayCounts;

  /// No description provided for @sessionWeekdayCountsSummary.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =0{Every day the same} =1{1 weekday with its own count} other{{count} weekdays with their own count}}'**
  String sessionWeekdayCountsSummary(int count);

  /// No description provided for @sessionWeekdayCountsExplain.
  ///
  /// In en, this message translates to:
  /// **'A weekday left empty picks as many as every day.'**
  String get sessionWeekdayCountsExplain;

  /// No description provided for @sessionWeekdayCountHint.
  ///
  /// In en, this message translates to:
  /// **'As every day'**
  String get sessionWeekdayCountHint;

  /// No description provided for @sessionRule.
  ///
  /// In en, this message translates to:
  /// **'How it picks'**
  String get sessionRule;

  /// No description provided for @sessionRuleRandom.
  ///
  /// In en, this message translates to:
  /// **'Random'**
  String get sessionRuleRandom;

  /// No description provided for @sessionRuleRandomDetail.
  ///
  /// In en, this message translates to:
  /// **'A new random pick each time.'**
  String get sessionRuleRandomDetail;

  /// No description provided for @sessionRuleRolling.
  ///
  /// In en, this message translates to:
  /// **'Rolling'**
  String get sessionRuleRolling;

  /// No description provided for @sessionRuleRollingDetail.
  ///
  /// In en, this message translates to:
  /// **'Through the pool in its order, continuing where it stopped.'**
  String get sessionRuleRollingDetail;

  /// No description provided for @sessionRuleLeast.
  ///
  /// In en, this message translates to:
  /// **'Least selected'**
  String get sessionRuleLeast;

  /// No description provided for @sessionRuleLeastDetail.
  ///
  /// In en, this message translates to:
  /// **'The Tallies with the fewest entries first.'**
  String get sessionRuleLeastDetail;

  /// Swaps a Tally a Session picked on the Agenda for the next by its rule.
  ///
  /// In en, this message translates to:
  /// **'Another one'**
  String get sessionAnother;

  /// A Session's row on a later day of the Agenda.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Picks 1 Tally} other{Picks {count} Tallies}}'**
  String sessionPicks(int count);

  /// No description provided for @sessionNothingToPick.
  ///
  /// In en, this message translates to:
  /// **'None of its Tallies is left to pick.'**
  String get sessionNothingToPick;

  /// A Tally a Session picked from a locked Area set to Masked, on the Agenda.
  ///
  /// In en, this message translates to:
  /// **'Private Tally'**
  String get maskedTally;

  /// On a capture screen: the Area or Topic picked for the Note, saved with it.
  ///
  /// In en, this message translates to:
  /// **'→ {place}'**
  String captureChoicePlace(String place);

  /// On a capture screen: a Routine set up for the Note, saved with it.
  ///
  /// In en, this message translates to:
  /// **'Routine: {schedule}'**
  String captureChoiceRoutine(String schedule);

  /// No description provided for @captureChoiceReminder.
  ///
  /// In en, this message translates to:
  /// **'Reminder: {schedule}'**
  String captureChoiceReminder(String schedule);

  /// No description provided for @captureChoiceSession.
  ///
  /// In en, this message translates to:
  /// **'Session: {schedule}'**
  String captureChoiceSession(String schedule);

  /// On a capture screen: the first field of the Tally the Note becomes.
  ///
  /// In en, this message translates to:
  /// **'Tally: {field}'**
  String captureChoiceTally(String field);

  /// In the Routine editor opened from a capture screen, for a Note without text yet.
  ///
  /// In en, this message translates to:
  /// **'The Note being captured'**
  String get noteBeingCaptured;

  /// Bar button of a Note list that shows each Note by its first line only.
  ///
  /// In en, this message translates to:
  /// **'Show Notes compact'**
  String get showNotesCompact;

  /// Bar button of a Note list that shows each Note whole, with its Sections.
  ///
  /// In en, this message translates to:
  /// **'Show Notes in full'**
  String get showNotesFull;

  /// Small label above a Transcript on a Note card.
  ///
  /// In en, this message translates to:
  /// **'Transcript'**
  String get transcriptLabel;

  /// Small label above a Photo Text on a Note card.
  ///
  /// In en, this message translates to:
  /// **'Photo Text'**
  String get photoTextLabel;

  /// Section title in Settings for how text is typed.
  ///
  /// In en, this message translates to:
  /// **'Typing'**
  String get settingsTyping;

  /// Switch in Settings for the two-row text editor of the Note editor and Amend.
  ///
  /// In en, this message translates to:
  /// **'Type in two rows'**
  String get twoRowEditor;

  /// No description provided for @twoRowEditorDetail.
  ///
  /// In en, this message translates to:
  /// **'In the Note editor and when adding to a Note, a line above the keyboard shows the text around the cursor, with buttons that move it one character.'**
  String get twoRowEditorDetail;

  /// Screen reader label of the left button of the line above the keyboard.
  ///
  /// In en, this message translates to:
  /// **'Cursor back one character'**
  String get cursorBack;

  /// No description provided for @cursorForward.
  ///
  /// In en, this message translates to:
  /// **'Cursor forward one character'**
  String get cursorForward;

  /// Screen reader label of the left button while selecting.
  ///
  /// In en, this message translates to:
  /// **'Selection end back one character'**
  String get selectionBack;

  /// No description provided for @selectionForward.
  ///
  /// In en, this message translates to:
  /// **'Selection end forward one character'**
  String get selectionForward;

  /// Screen reader label of the line above the keyboard.
  ///
  /// In en, this message translates to:
  /// **'Text around the cursor'**
  String get cursorLine;

  /// Screen reader hint of the long press on the line above the keyboard.
  ///
  /// In en, this message translates to:
  /// **'start selecting'**
  String get cursorLineSelect;

  /// No description provided for @cursorLineStopSelecting.
  ///
  /// In en, this message translates to:
  /// **'stop selecting'**
  String get cursorLineStopSelecting;

  /// Spoken when the cursor passes a space.
  ///
  /// In en, this message translates to:
  /// **'Space'**
  String get cursorSpace;

  /// Spoken when the cursor passes a line break.
  ///
  /// In en, this message translates to:
  /// **'New line'**
  String get cursorNewLine;

  /// Announced when no Profile card can be made yet; the tap opens the dialog with the cause and the fix.
  ///
  /// In en, this message translates to:
  /// **'Your Profile card could not be made.'**
  String get profileCardNotMade;

  /// Title of the dialog behind 'profileCardNotMade'.
  ///
  /// In en, this message translates to:
  /// **'No Profile card yet'**
  String get profileCardNotMadeTitle;

  /// The cause in the dialog behind 'profileCardNotMade'.
  ///
  /// In en, this message translates to:
  /// **'This Device joins your Profile only once it has synced with another of your Devices.'**
  String get profileCardNotYetCause;

  /// Announced when no app takes the shared Profile card; the tap opens the dialog with the cause and the fix.
  ///
  /// In en, this message translates to:
  /// **'Your Profile card was not sent.'**
  String get profileCardNotSent;

  /// Title of the dialog behind 'profileCardNotSent'.
  ///
  /// In en, this message translates to:
  /// **'Profile card not sent'**
  String get profileCardNotSentTitle;

  /// The fix in the dialog behind 'profileCardNotSent'.
  ///
  /// In en, this message translates to:
  /// **'Show your QR for them to scan instead, or install a messenger and send it again.'**
  String get profileCardNoAppFix;

  /// Announced when an opened Profile card added nobody to People; the tap opens the dialog with the cause and the fix.
  ///
  /// In en, this message translates to:
  /// **'Nobody was added from the card.'**
  String get cardNotAdded;

  /// Title of the dialog behind 'cardNotAdded'.
  ///
  /// In en, this message translates to:
  /// **'Nobody added'**
  String get cardNotAddedTitle;

  /// The fix in the dialog when the user opened their own Profile card.
  ///
  /// In en, this message translates to:
  /// **'To add someone, open their card, not yours.'**
  String get cardOwnFix;

  /// The fix in the dialog when a card names a person in People but was not made by them.
  ///
  /// In en, this message translates to:
  /// **'Ask the person for their card again, in person if you can.'**
  String get cardRefusedFix;

  /// The cause in the dialog when an opened text is no Profile card.
  ///
  /// In en, this message translates to:
  /// **'This is not a Profile card, or it was changed on the way.'**
  String get cardUnreadableCause;

  /// The fix in the dialog when an opened text is no Profile card.
  ///
  /// In en, this message translates to:
  /// **'Ask for the link again.'**
  String get cardAskAgainFix;

  /// Announced when no QR code is found in a photo or picture; the tap opens the dialog with the cause and the fix.
  ///
  /// In en, this message translates to:
  /// **'The picture could not be read.'**
  String get cardNotRead;

  /// Title of the dialog behind 'cardNotRead'.
  ///
  /// In en, this message translates to:
  /// **'No QR code found'**
  String get cardNotReadTitle;

  /// The fix in the dialog behind 'cardNotRead'.
  ///
  /// In en, this message translates to:
  /// **'Try a picture taken a little closer, or ask for the link.'**
  String get cardNoQrFix;

  /// The fix in a failure dialog when something comes from a person the user blocked.
  ///
  /// In en, this message translates to:
  /// **'To take what they send, unblock them on their Profile under People, then ask them to send it again.'**
  String get blockedFix;

  /// Announced when sharing an Area in person finished; the tap shows with whom.
  ///
  /// In en, this message translates to:
  /// **'Sharing finished.'**
  String get hostShareDone;

  /// Announced when a sync in person finished; the tap shows with which Device and how much came in.
  ///
  /// In en, this message translates to:
  /// **'Sync finished.'**
  String get hostSyncDone;

  /// Announced when a sync in person refused changes; the tap opens the dialog with the count, cause and fix. The count only picks the grammar and is never shown.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{A change was refused.} other{Changes were refused.}}'**
  String hostRefused(int count);

  /// Title of the dialog behind 'hostRefused'.
  ///
  /// In en, this message translates to:
  /// **'Changes refused'**
  String get hostRefusedTitle;

  /// The fix in the dialog behind 'hostRefused'.
  ///
  /// In en, this message translates to:
  /// **'Nothing of them was kept; the rest came in. If it happens again, report it.'**
  String get hostRefusedFix;

  /// Announced when a sync in person found Conflicts; the tap opens them. The count only picks the grammar and is never shown.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Something was changed on both Devices. Tap to pick which version stays.} other{Some things were changed on both Devices. Tap to pick which versions stay.}}'**
  String hostConflicts(int count);

  /// Announced when a shared Area is refused; the tap opens the dialog with the cause and the fix.
  ///
  /// In en, this message translates to:
  /// **'The shared Area was not taken.'**
  String get areaOfferNotTaken;

  /// Title of the dialog behind 'areaOfferNotTaken'.
  ///
  /// In en, this message translates to:
  /// **'Shared Area not taken'**
  String get areaOfferNotTakenTitle;

  /// The fix in the dialog when a shared Area is meant for someone else or cannot be read.
  ///
  /// In en, this message translates to:
  /// **'Ask the sender to share it again with you.'**
  String get areaOfferAskAgainFix;

  /// The fix in the dialog when a shared Area is the user's own already.
  ///
  /// In en, this message translates to:
  /// **'It is among your Areas already; nothing needs doing.'**
  String get areaOfferOwnFix;

  /// Announced when a Sent Item is refused; the tap opens the dialog with the cause and the fix.
  ///
  /// In en, this message translates to:
  /// **'The file was not taken.'**
  String get sentItemNotTaken;

  /// Title of the dialog behind 'sentItemNotTaken'.
  ///
  /// In en, this message translates to:
  /// **'File not taken'**
  String get sentItemNotTakenTitle;

  /// The fix in the dialog when a Sent Item cannot be read, is for someone else or is not signed by its sender.
  ///
  /// In en, this message translates to:
  /// **'Ask the sender to send it again.'**
  String get sentItemAskAgainFix;

  /// Title of the dialog behind 'shareAfarFailed'.
  ///
  /// In en, this message translates to:
  /// **'Area not shared'**
  String get shareAfarFailedTitle;

  /// The cause in the dialog behind 'shareAfarFailed' when writing the file failed.
  ///
  /// In en, this message translates to:
  /// **'The file for the shared Area could not be written.'**
  String get shareAfarFailedCause;

  /// The fix in the dialog behind 'shareAfarFailed' when writing the file failed.
  ///
  /// In en, this message translates to:
  /// **'Nothing was lost; try again.'**
  String get shareAfarFailedFix;

  /// The fix in the dialog behind 'shareAfarFailed' when no app takes the file.
  ///
  /// In en, this message translates to:
  /// **'Install a messenger, or choose a shared folder, then share again.'**
  String get shareAfarNoAppFix;

  /// Announced when no app takes the calendar file of Routines; the tap opens the dialog with the cause and the fix.
  ///
  /// In en, this message translates to:
  /// **'The calendar file was not shared.'**
  String get calendarFileNotShared;

  /// Title of the dialog behind 'calendarFileNotShared'.
  ///
  /// In en, this message translates to:
  /// **'Calendar file not shared'**
  String get calendarFileNotSharedTitle;

  /// The fix in the dialog behind 'calendarFileNotShared'.
  ///
  /// In en, this message translates to:
  /// **'Install a calendar app that opens .ics files, then try again.'**
  String get calendarFileFix;

  /// Announced when the calendar's appointments cannot be turned on for the Agenda; the tap opens the dialog with the cause and the fix.
  ///
  /// In en, this message translates to:
  /// **'The calendar cannot be shown on the Agenda.'**
  String get agendaCalendarFailed;

  /// Title of the dialog behind 'agendaCalendarFailed'.
  ///
  /// In en, this message translates to:
  /// **'Calendar not shown'**
  String get agendaCalendarFailedTitle;

  /// The cause in the dialog behind 'agendaCalendarFailed' when reading the calendar is not allowed.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix may not read your calendar.'**
  String get agendaCalendarNotAllowedCause;

  /// The fix in the dialog behind 'agendaCalendarFailed' when reading the calendar is not allowed.
  ///
  /// In en, this message translates to:
  /// **'You can allow it in the system settings, then turn this on again.'**
  String get agendaCalendarAllowFix;

  /// The fix in the dialog behind 'agendaCalendarFailed' when the phone has no calendar.
  ///
  /// In en, this message translates to:
  /// **'Add a calendar account in the phone\'s settings, then turn this on again.'**
  String get agendaCalendarAddFix;

  /// A fix in a failure dialog when nothing was lost and trying again may work.
  ///
  /// In en, this message translates to:
  /// **'Nothing was lost; try again.'**
  String get nothingLostFix;

  /// Announced when no code was found in a photo taken to read one; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The photo could not be read.'**
  String get codeNotRead;

  /// Title of the dialog behind 'codeNotRead'.
  ///
  /// In en, this message translates to:
  /// **'No code found'**
  String get codeNotReadTitle;

  /// The fix in the dialog behind 'codeNotRead' for a pair code.
  ///
  /// In en, this message translates to:
  /// **'Try again a little closer, or type the code.'**
  String get syncNoCodeFix;

  /// The fix in the dialog behind 'codeNotRead' for a recovery key.
  ///
  /// In en, this message translates to:
  /// **'Try again a little closer, or type the key.'**
  String get restoreNoCodeFix;

  /// Announced when a sync in person failed; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The sync did not go through.'**
  String get syncNotDone;

  /// Title of the dialog behind 'syncNotDone'.
  ///
  /// In en, this message translates to:
  /// **'Sync failed'**
  String get syncNotDoneTitle;

  /// Why a sync failed: the code names an address outside the local network.
  ///
  /// In en, this message translates to:
  /// **'This code points outside your home network.'**
  String get syncNotPrivateCause;

  /// The fix for 'syncNotPrivateCause'.
  ///
  /// In en, this message translates to:
  /// **'Only Devices on the same network can sync: connect both to the same Wi-Fi, then try again.'**
  String get syncNotPrivateFix;

  /// Why a sync failed: the other Device did not answer.
  ///
  /// In en, this message translates to:
  /// **'The other Device did not answer.'**
  String get syncUnreachableCause;

  /// The fix for 'syncUnreachableCause'.
  ///
  /// In en, this message translates to:
  /// **'Check that it still shows the code and that both are on the same Wi-Fi, then try again.'**
  String get syncUnreachableFix;

  /// The fix when the code belongs to another Device.
  ///
  /// In en, this message translates to:
  /// **'Scan or type the code the other Device shows now.'**
  String get syncWrongHostFix;

  /// Why a sync failed: the PIN in the code is wrong.
  ///
  /// In en, this message translates to:
  /// **'The PIN does not match.'**
  String get syncWrongPinCause;

  /// The fix for 'syncWrongPinCause'.
  ///
  /// In en, this message translates to:
  /// **'Type the code again.'**
  String get syncWrongPinFix;

  /// Why a sync failed: too many wrong PINs.
  ///
  /// In en, this message translates to:
  /// **'There were too many wrong tries.'**
  String get syncLockedCause;

  /// The fix for 'syncLockedCause'.
  ///
  /// In en, this message translates to:
  /// **'Show a new code on the other Device, then try again.'**
  String get syncLockedFix;

  /// The fix when the other Device said no.
  ///
  /// In en, this message translates to:
  /// **'Try again, and say yes on the other Device when it asks.'**
  String get syncDeclinedFix;

  /// Why a sync failed: anything else on the way.
  ///
  /// In en, this message translates to:
  /// **'The connection broke off before the sync finished.'**
  String get syncBrokeOffCause;

  /// The fix when Old Brain Fix on one Device needs an update to sync.
  ///
  /// In en, this message translates to:
  /// **'Install the latest Old Brain Fix on the Device named, then sync again.'**
  String get updateFix;

  /// Announced when an automatic sync needs an update of the app here or on another Device; the tap shows which.
  ///
  /// In en, this message translates to:
  /// **'An update is needed to sync.'**
  String get updateNeeded;

  /// Title of the dialog behind 'updateNeeded'.
  ///
  /// In en, this message translates to:
  /// **'Update needed'**
  String get updateNeededTitle;

  /// Announced when a bundle would hold nothing new for whom it is meant.
  ///
  /// In en, this message translates to:
  /// **'There is nothing new to send.'**
  String get bundleNothingToSend;

  /// Announced when a bundle could not be written or no app took it; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The bundle was not sent.'**
  String get bundleNotSent;

  /// Title of the dialog behind 'bundleNotSent'.
  ///
  /// In en, this message translates to:
  /// **'Bundle not sent'**
  String get bundleNotSentTitle;

  /// Why a bundle was not sent: its file could not be written.
  ///
  /// In en, this message translates to:
  /// **'The bundle file could not be written.'**
  String get bundleNotWrittenCause;

  /// The fix when no app takes the bundle.
  ///
  /// In en, this message translates to:
  /// **'Install a messenger or a mail app, or choose a shared folder, then send it again.'**
  String get bundleNoAppFix;

  /// Announced when a bundle brought nothing in; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The bundle was not imported.'**
  String get bundleNotImported;

  /// Title of the dialog behind 'bundleNotImported'.
  ///
  /// In en, this message translates to:
  /// **'Bundle not imported'**
  String get bundleNotImportedTitle;

  /// The fix when the file is no sync bundle.
  ///
  /// In en, this message translates to:
  /// **'Ask for the bundle again, or import another file.'**
  String get bundleAskAgainFix;

  /// Why a bundle was not imported: it comes from a newer app.
  ///
  /// In en, this message translates to:
  /// **'This bundle comes from a newer Old Brain Fix.'**
  String get bundleNeedsUpdateCause;

  /// The fix for 'bundleNeedsUpdateCause'.
  ///
  /// In en, this message translates to:
  /// **'Update Old Brain Fix on this Device, then import it again.'**
  String get bundleUpdateFix;

  /// Why a bundle was not imported: anything else broke on the way.
  ///
  /// In en, this message translates to:
  /// **'The bundle could not be read to the end.'**
  String get bundleNotReadCause;

  /// The fix for 'bundleNotReadCause'.
  ///
  /// In en, this message translates to:
  /// **'Nothing of it was kept; import it again.'**
  String get bundleImportAgainFix;

  /// Why a bundle was not imported: it is from an unpaired Device.
  ///
  /// In en, this message translates to:
  /// **'This bundle is from a Device this one is not paired with, so nothing in it can be read.'**
  String get bundlePairFirstCause;

  /// The fix for 'bundlePairFirstCause'.
  ///
  /// In en, this message translates to:
  /// **'Pair the two Devices first, then import it again.'**
  String get bundlePairFirstFix;

  /// Announced when a bundle was imported; the tap shows how much came in.
  ///
  /// In en, this message translates to:
  /// **'Bundle imported.'**
  String get bundleImported;

  /// Announced when a Sent Item could not be written or no app took it; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The Note was not sent.'**
  String get sentItemNotSent;

  /// Title of the dialog behind 'sentItemNotSent'.
  ///
  /// In en, this message translates to:
  /// **'Note not sent'**
  String get sentItemNotSentTitle;

  /// Why a Sent Item was not sent: its file could not be written.
  ///
  /// In en, this message translates to:
  /// **'The file for the Note could not be written.'**
  String get sentItemNotWrittenCause;

  /// The fix when no app takes a Sent Item.
  ///
  /// In en, this message translates to:
  /// **'Install a messenger or a mail app, then send it again.'**
  String get sendNoAppFix;

  /// Announced when the calendar mirror could not be turned on or moved; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The Routines and Reminders were not written into the calendar.'**
  String get calendarMirrorNotOn;

  /// Title of the dialog behind 'calendarMirrorNotOn'.
  ///
  /// In en, this message translates to:
  /// **'Not written into the calendar'**
  String get calendarMirrorNotOnTitle;

  /// The fix when the events could not be removed from the calendar mirrored into before.
  ///
  /// In en, this message translates to:
  /// **'Allow Old Brain Fix to use your calendar in the system settings, then choose the calendar again.'**
  String get calendarMirrorChooseAgainFix;

  /// Announced when the calendar mirror could not be turned off; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The events were not removed from the calendar.'**
  String get calendarMirrorNotOff;

  /// Title of the dialog behind 'calendarMirrorNotOff'.
  ///
  /// In en, this message translates to:
  /// **'Events not removed'**
  String get calendarMirrorNotOffTitle;

  /// The fix for 'calendarMirrorNotOff'.
  ///
  /// In en, this message translates to:
  /// **'Allow Old Brain Fix to use your calendar in the system settings, then turn this off again.'**
  String get calendarMirrorOffAgainFix;

  /// Announced when a part of the Capture Context could not be switched on; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'This part of the Capture Context was not switched on.'**
  String get contextNotOn;

  /// Title of the dialog behind 'contextNotOn'.
  ///
  /// In en, this message translates to:
  /// **'Not switched on'**
  String get contextNotOnTitle;

  /// Announced when no calendar app took an event; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The event was not added to the calendar.'**
  String get calendarEventNotAdded;

  /// Title of the dialog behind 'calendarEventNotAdded'.
  ///
  /// In en, this message translates to:
  /// **'Event not added'**
  String get calendarEventNotAddedTitle;

  /// The fix when there is no calendar app.
  ///
  /// In en, this message translates to:
  /// **'Install a calendar app, then try again.'**
  String get noCalendarAppFix;

  /// Announced when the Recovery Sheet could not be made or no app took it; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The Recovery Sheet was not printed or shared.'**
  String get backupSheetNotOut;

  /// Title of the dialog behind 'backupSheetNotOut'.
  ///
  /// In en, this message translates to:
  /// **'Recovery Sheet not handed over'**
  String get backupSheetNotOutTitle;

  /// Why the Recovery Sheet did not go out: its file could not be written.
  ///
  /// In en, this message translates to:
  /// **'The file for the Recovery Sheet could not be written.'**
  String get backupSheetFailedCause;

  /// The fix when no app takes the Recovery Sheet.
  ///
  /// In en, this message translates to:
  /// **'Install an app that prints or opens PDF files, then try again.'**
  String get backupSheetNoAppFix;

  /// Announced when writing a Backup failed; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The Backup was not written.'**
  String get backupNotWritten;

  /// Title of the dialog behind 'backupNotWritten'.
  ///
  /// In en, this message translates to:
  /// **'Backup not written'**
  String get backupNotWrittenTitle;

  /// Why a Backup was not written.
  ///
  /// In en, this message translates to:
  /// **'The Backup could not be written into its folder.'**
  String get backupFailedCause;

  /// The fix for 'backupFailedCause'.
  ///
  /// In en, this message translates to:
  /// **'Check that the folder is still there, or choose it again, then back up again.'**
  String get backupFailedFix;

  /// Announced when restoring a Backup failed; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The Backup was not restored.'**
  String get restoreNotDone;

  /// Title of the dialog behind 'restoreNotDone'.
  ///
  /// In en, this message translates to:
  /// **'Backup not restored'**
  String get restoreNotDoneTitle;

  /// The fix when the file picked is no Backup.
  ///
  /// In en, this message translates to:
  /// **'Pick the Backup file again.'**
  String get restorePickFix;

  /// The fix when the passphrase or recovery key does not open the Backup.
  ///
  /// In en, this message translates to:
  /// **'Type the passphrase again, or use the recovery key on the Recovery Sheet.'**
  String get restoreWrongFix;

  /// Why a Backup was not restored: it is damaged.
  ///
  /// In en, this message translates to:
  /// **'The Backup is damaged or incomplete.'**
  String get restoreDamagedCause;

  /// The fix for 'restoreDamagedCause'.
  ///
  /// In en, this message translates to:
  /// **'Nothing was restored; try an older Backup.'**
  String get restoreDamagedFix;

  /// The fix when the Device holds Notes already.
  ///
  /// In en, this message translates to:
  /// **'A Backup goes only onto a Device that holds no Notes yet.'**
  String get restoreNotEmptyFix;

  /// Why a Backup was not restored: anything else broke on the way.
  ///
  /// In en, this message translates to:
  /// **'The Backup could not be read to the end.'**
  String get restoreFailedCause;

  /// The fix for 'restoreFailedCause'.
  ///
  /// In en, this message translates to:
  /// **'Nothing was changed; try again.'**
  String get restoreFailedFix;

  /// Announced when a Backup was restored; the tap shows how much came back and what to set again.
  ///
  /// In en, this message translates to:
  /// **'Backup restored.'**
  String get restoreDoneShort;

  /// Announced when naming the current place failed; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The place was not named.'**
  String get namedPlaceNotAdded;

  /// Title of the dialog behind 'namedPlaceNotAdded'.
  ///
  /// In en, this message translates to:
  /// **'Place not named'**
  String get namedPlaceNotAddedTitle;

  /// The fix when the location may not be used.
  ///
  /// In en, this message translates to:
  /// **'You can allow it in the system settings, then try again.'**
  String get locationAllowFix;

  /// The fix when the position could not be found.
  ///
  /// In en, this message translates to:
  /// **'Turn location on, then try again.'**
  String get locationUnknownFix;

  /// Announced when a picture for the Profile cannot be read; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The picture was not set.'**
  String get profilePictureNotSet;

  /// Title of the dialog behind 'profilePictureNotSet'.
  ///
  /// In en, this message translates to:
  /// **'Picture not set'**
  String get profilePictureNotSetTitle;

  /// The fix when a Profile picture cannot be read.
  ///
  /// In en, this message translates to:
  /// **'Pick another picture, or take a new photo.'**
  String get profilePictureFix;

  /// Announced when downloading or picking the speech model failed; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The speech model was not kept.'**
  String get speechModelNotKept;

  /// Title of the dialog behind 'speechModelNotKept'.
  ///
  /// In en, this message translates to:
  /// **'Speech model not kept'**
  String get speechModelNotKeptTitle;

  /// Why the speech model was not kept: the download broke off.
  ///
  /// In en, this message translates to:
  /// **'The download did not finish.'**
  String get speechModelFailedCause;

  /// The fix for 'speechModelFailedCause'.
  ///
  /// In en, this message translates to:
  /// **'Nothing was kept; try again later.'**
  String get speechModelFailedFix;

  /// Why the speech model was not kept: it is not a pinned one.
  ///
  /// In en, this message translates to:
  /// **'That is not the expected speech model.'**
  String get speechModelMismatchCause;

  /// The fix for 'speechModelMismatchCause'.
  ///
  /// In en, this message translates to:
  /// **'Download one of the models listed, or pick the file of one of them.'**
  String get speechModelMismatchFix;

  /// Section title in Settings: how this Device's Agenda shows Shopping Lists, today only.
  ///
  /// In en, this message translates to:
  /// **'Shopping Lists on the Agenda'**
  String get settingsShoppingAgenda;

  /// Under the section title 'Shopping Lists on the Agenda' in Settings.
  ///
  /// In en, this message translates to:
  /// **'Today only, and on this Device only.'**
  String get settingsShoppingAgendaDetail;

  /// Radio choice: every Line of a Shopping List that came back today gets its own checkable row on the Agenda.
  ///
  /// In en, this message translates to:
  /// **'Each Line that came back today, on its own row'**
  String get shoppingAgendaLines;

  /// Radio choice, the default: one Agenda row per Shopping List with the number of Lines that came back today.
  ///
  /// In en, this message translates to:
  /// **'One row per Shopping List on a day when Lines came back'**
  String get shoppingAgendaBack;

  /// Radio choice: one Agenda row per Shopping List every day it has Lines not struck through.
  ///
  /// In en, this message translates to:
  /// **'One row per Shopping List on every day it has open Lines'**
  String get shoppingAgendaOpen;

  /// Radio choice: Shopping Lists do not show on the Agenda.
  ///
  /// In en, this message translates to:
  /// **'Not at all'**
  String get shoppingAgendaOff;

  /// An Agenda row of a Shopping List: its title and how many of its Lines came back today, such as 'Groceries: 3 back'.
  ///
  /// In en, this message translates to:
  /// **'{list}: {count} back'**
  String shoppingRowBack(String list, int count);

  /// An Agenda row of a Shopping List: its title and how many of its Lines are not struck through, such as 'Groceries: 5 open'.
  ///
  /// In en, this message translates to:
  /// **'{list}: {count} open'**
  String shoppingRowOpen(String list, int count);

  /// A Shopping List from a locked Area set to Masked, on the Agenda.
  ///
  /// In en, this message translates to:
  /// **'Private Shopping List'**
  String get maskedShoppingList;
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
