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

  /// Capture button and "+" fan way that fills the capture screen from the clipboard.
  ///
  /// In en, this message translates to:
  /// **'Paste'**
  String get paste;

  /// Line above the capture field after Paste found nothing usable on the clipboard.
  ///
  /// In en, this message translates to:
  /// **'Nothing to paste: the clipboard holds no text and no picture.'**
  String get pasteEmpty;

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

  /// Screen reader hint of the long press on the Agenda's and the lock screen's + (#282): it opens text capture at once, without the fan. Lowercase, as Android reads it after "Double-tap and hold to".
  ///
  /// In en, this message translates to:
  /// **'write a Note at once'**
  String get captureLongPressHint;

  /// Screen reader action of the Agenda's and the lock screen's + on the iPhone (#282), the long press's: opens text capture at once, without the fan.
  ///
  /// In en, this message translates to:
  /// **'Write a Note at once'**
  String get captureLongPressAction;

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

  /// The filter field at the top of File into…
  ///
  /// In en, this message translates to:
  /// **'Filter Areas and Topics'**
  String get fileIntoFilterHint;

  /// Heading over the last three places this Device filed into, at the top of File into…
  ///
  /// In en, this message translates to:
  /// **'Filed into recently'**
  String get fileIntoRecent;

  /// The row at the end of File into… that makes an Area and files into it
  ///
  /// In en, this message translates to:
  /// **'New Area…'**
  String get fileIntoNewArea;

  /// The row under an Area in File into… that makes a Topic there and files into it
  ///
  /// In en, this message translates to:
  /// **'New Topic…'**
  String get fileIntoNewTopic;

  /// File into… when nothing matches the filter
  ///
  /// In en, this message translates to:
  /// **'No Area or Topic matches.'**
  String get fileIntoNoMatch;

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

  /// In the fan of the browser's '+': a Note, captured as on the Agenda.
  ///
  /// In en, this message translates to:
  /// **'Note'**
  String get addFanNote;

  /// No description provided for @newChecklist.
  ///
  /// In en, this message translates to:
  /// **'New Checklist'**
  String get newChecklist;

  /// No description provided for @newShoppingList.
  ///
  /// In en, this message translates to:
  /// **'New Shopping List'**
  String get newShoppingList;

  /// No description provided for @newTally.
  ///
  /// In en, this message translates to:
  /// **'New Tally'**
  String get newTally;

  /// No description provided for @newFrog.
  ///
  /// In en, this message translates to:
  /// **'New Frog'**
  String get newFrog;

  /// No description provided for @kindName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get kindName;

  /// No description provided for @kindNameMissing.
  ///
  /// In en, this message translates to:
  /// **'Give it a name.'**
  String get kindNameMissing;

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
  /// **'Old Brain Fix can note where you are when you capture a Note, so “nees mor sauce” still makes sense days later. Name your own places, like Home or Work, and a Note says “Home” instead of coordinates. The place goes only where its Note goes: to your own Devices, and to the people of a Shared Area you file the Note in. Never to anyone else.'**
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

  /// Entry in a compact Note card's ⋮ menu that opens the editor of a new Routine for the Note.
  ///
  /// In en, this message translates to:
  /// **'Routine…'**
  String get spawnRoutine;

  /// Entry in a compact Note card's ⋮ menu that opens the editor of a new Reminder for the Note.
  ///
  /// In en, this message translates to:
  /// **'Reminder…'**
  String get spawnReminder;

  /// Entry in a compact Note card's ⋮ menu, in Advanced, that opens the editor of a new Session run from the Note.
  ///
  /// In en, this message translates to:
  /// **'Session…'**
  String get spawnSession;

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

  /// No description provided for @scheduleSentenceLabel.
  ///
  /// In en, this message translates to:
  /// **'When'**
  String get scheduleSentenceLabel;

  /// No description provided for @scheduleSentenceHint.
  ///
  /// In en, this message translates to:
  /// **'Every day at 08:00'**
  String get scheduleSentenceHint;

  /// No description provided for @scheduleSentenceUnknown.
  ///
  /// In en, this message translates to:
  /// **'Marked words are not understood and change nothing.'**
  String get scheduleSentenceUnknown;

  /// No description provided for @scheduleSentenceAdvanced.
  ///
  /// In en, this message translates to:
  /// **'Switch to Simple to type.'**
  String get scheduleSentenceAdvanced;

  /// The chip that sets a new Reminder to an hour from now, with its time (#271).
  ///
  /// In en, this message translates to:
  /// **'In 1 h, {time}'**
  String quickInAnHour(String time);

  /// The chip that sets a new Reminder to 19:00 today, with its time.
  ///
  /// In en, this message translates to:
  /// **'Tonight {time}'**
  String quickTonight(String time);

  /// The chip that sets a new Reminder to tomorrow when this Device's Quiet Hours end, with its time.
  ///
  /// In en, this message translates to:
  /// **'Tomorrow {time}'**
  String quickTomorrow(String time);

  /// The chip that sets a new Reminder to a week from today when the Quiet Hours end, with its time.
  ///
  /// In en, this message translates to:
  /// **'In a week, {time}'**
  String quickInAWeek(String time);

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

  /// Beside Snooze on an Occurrence's notification, in the Agenda row's menu and in the Note editor's Due line (#269): opens Skip and Tomorrow. CONTEXT.md, Not today.
  ///
  /// In en, this message translates to:
  /// **'Not today'**
  String get notToday;

  /// In the menu of an Occurrence's row on the Agenda (#270): opens the Routine or Reminder editor, as a tap on the row opens its Note.
  ///
  /// In en, this message translates to:
  /// **'Edit schedule'**
  String get editSchedule;

  /// The text of an Occurrence's notification once Not today was tapped, above Skip and Tomorrow (#269).
  ///
  /// In en, this message translates to:
  /// **'Skip it, or tomorrow at the same time?'**
  String get notTodayQuestion;

  /// A Not today choice (#269): the Occurrence is neither Done nor Missed, and rings no more. CONTEXT.md, Skip.
  ///
  /// In en, this message translates to:
  /// **'Skip'**
  String get skip;

  /// A Not today choice (#269): skips the Occurrence and adds a one-time one of the same Routine the day after, at the same time.
  ///
  /// In en, this message translates to:
  /// **'Tomorrow'**
  String get notTodayTomorrow;

  /// Under a skipped Occurrence on the Agenda, struck through for the day (#269).
  ///
  /// In en, this message translates to:
  /// **'Skipped'**
  String get occurrenceSkipped;

  /// Under an Occurrence on the Agenda that Not today ▸ Tomorrow skipped (#269); its one-time Occurrence shows the day after.
  ///
  /// In en, this message translates to:
  /// **'The day after instead'**
  String get occurrenceTomorrowInstead;

  /// Under the one-time Occurrence that Not today ▸ Tomorrow added, on the Agenda and in the Routine's history log (#269).
  ///
  /// In en, this message translates to:
  /// **'From the day before'**
  String get occurrenceFromDayBefore;

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

  /// No description provided for @permAlarms.
  ///
  /// In en, this message translates to:
  /// **'Alarms'**
  String get permAlarms;

  /// No description provided for @permAlarmsWhy.
  ///
  /// In en, this message translates to:
  /// **'Lets a Routine you keep ignoring turn into an alarm that rings through silent mode and Focus, if you switch that on for it. Allow it in the dialog that opens, or for Old Brain Fix in Settings.'**
  String get permAlarmsWhy;

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

  /// escalationSwitch on the iPhone (#244), which has no full-screen alarm: the escalated Occurrence rings as an AlarmKit alarm.
  ///
  /// In en, this message translates to:
  /// **'Escalate to an iPhone alarm'**
  String get escalationSwitchIphone;

  /// escalationSummary on the iPhone (#244).
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{iPhone alarm after 1 ignored Re-nag} other{iPhone alarm after {count} ignored Re-nags}}'**
  String escalationSummaryIphone(int count);

  /// escalationExplain on the iPhone (#244): AlarmKit's own alarm screen with Stop and one Snooze of 10 min.
  ///
  /// In en, this message translates to:
  /// **'When this one is ignored again and again, it rings as an iPhone alarm, through silent mode and Focus, also on the lock screen. Stop counts as Done; Snooze rings it again in 10 min.'**
  String get escalationExplainIphone;

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
  /// **'While anything is Unsorted, a Nudge comes again after this long when you ignore it. In the Quiet Hours Nudges wait until they end; Routines and Reminders still ring on time; only an all-day one waits until they end.'**
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
  /// **'They stay in the trash with it for 30 days and come back when you restore it.'**
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
  /// **'Old Brain Fix can note the calendar event running when you capture a Note, so it says “during: Dentist” days later. It reads your calendar only at that moment. The event goes only where its Note goes: to your own Devices, and to the people of a Shared Area you file the Note in. Never to anyone else.'**
  String get calendarExplainBody;

  /// No description provided for @bluetoothExplainTitle.
  ///
  /// In en, this message translates to:
  /// **'Remember what was connected?'**
  String get bluetoothExplainTitle;

  /// No description provided for @bluetoothExplainBody.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix can note the Bluetooth device connected when you capture a Note, like your car or your headphones. Android calls this “Nearby devices”; Old Brain Fix only reads the names of connected devices and never searches for others. The name goes only where its Note goes: to your own Devices, and to the people of a Shared Area you file the Note in. Never to anyone else.'**
  String get bluetoothExplainBody;

  /// bluetoothExplainBody on the iPhone (#241), which reads the audio route: no Nearby devices permission.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix can note the Bluetooth device the iPhone plays sound through when you capture a Note, like your car or your headphones. It reads only the name of that device and never searches for others. The name goes only where its Note goes: to your own Devices, and to the people of a Shared Area you file the Note in. Never to anyone else.'**
  String get bluetoothExplainBodyIphone;

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

  /// In the ⋮ of a Note's card: puts the text of the Note and of its Sections on the clipboard.
  ///
  /// In en, this message translates to:
  /// **'Copy text'**
  String get copyText;

  /// Under a Section of a Note whose text was edited: when it was edited last.
  ///
  /// In en, this message translates to:
  /// **'Edited {when}'**
  String sectionEdited(String when);

  /// Under a Section of a Note whose text was edited: when it was edited last and the Profile name of who edited it.
  ///
  /// In en, this message translates to:
  /// **'Edited {when} by {name}'**
  String sectionEditedBy(String when, String name);

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
  /// **'Voice Notes record what you say, so a thought is kept before it is gone. Old Brain Fix uses the microphone only while you record. The recording goes only where its Note goes: to your own Devices, and to the people of a Shared Area you file the Note in. Never to anyone else.'**
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

  /// A day of the last seven days in a Routine's history whose Occurrences were all skipped (#269).
  ///
  /// In en, this message translates to:
  /// **'{day}: skipped'**
  String daySkipped(String day);

  /// Beside a period's Adherence in a Routine's history, when Occurrences were skipped in it (#269): "92 % · 3 skipped". Skipped ones are not part of the share.
  ///
  /// In en, this message translates to:
  /// **'{count} skipped'**
  String adherenceSkipped(int count);

  /// An Occurrence in a Routine's history log that was skipped (#269).
  ///
  /// In en, this message translates to:
  /// **'Skipped'**
  String get logSkipped;

  /// An Occurrence in a Routine's history log that Not today ▸ Tomorrow skipped (#269).
  ///
  /// In en, this message translates to:
  /// **'Skipped, the day after instead'**
  String get logSkippedTomorrow;

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
  /// **'Simple or Advanced and the Trash setting sync to your own Devices; the other settings belong to this phone.'**
  String get settingsIntro;

  /// Hint of the search field at the top of Settings (#282), which filters to the matching settings with their headings.
  ///
  /// In en, this message translates to:
  /// **'Search Settings'**
  String get settingsSearchHint;

  /// Shown in Settings when its search finds no setting (#282).
  ///
  /// In en, this message translates to:
  /// **'No setting matches “{query}”.'**
  String settingsSearchNothing(String query);

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

  /// The switch in Settings > Look that shows text, icons and tap targets 1.3 times their size on this Device (#224). The desktop calls it Bigger text and icons.
  ///
  /// In en, this message translates to:
  /// **'Fat fingers'**
  String get fatFingers;

  /// Under the switch fatFingers.
  ///
  /// In en, this message translates to:
  /// **'Text, icons and buttons show 1.3 times their size, on top of the system\'s font size, on the Agenda\'s widget too.'**
  String get fatFingersDetail;

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

  /// No description provided for @allDayFrom.
  ///
  /// In en, this message translates to:
  /// **'All-day items remind from'**
  String get allDayFrom;

  /// No description provided for @allDayFromDetail.
  ///
  /// In en, this message translates to:
  /// **'Before this time an all-day Routine or Reminder only shows on the Agenda. From it on it rings and Re-nags. Quiet Hours win.'**
  String get allDayFromDetail;

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

  /// Announced when seven taps on the version under About switched the frame-time overlay on.
  ///
  /// In en, this message translates to:
  /// **'Frame times on'**
  String get frameTimesOn;

  /// Announced when seven taps on the version under About switched the frame-time overlay off.
  ///
  /// In en, this message translates to:
  /// **'Frame times off'**
  String get frameTimesOff;

  /// First line of the frame-time overlay: the slowest build and raster of the frames drawn lately, in milliseconds.
  ///
  /// In en, this message translates to:
  /// **'Last frames: build {build} ms, raster {raster} ms'**
  String frameTimesLast(String build, String raster);

  /// Second line of the frame-time overlay: the slowest frame of the last second, start to end, in milliseconds.
  ///
  /// In en, this message translates to:
  /// **'Slowest in the last second: {total} ms'**
  String frameTimesWorst(String total);

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

  /// unlockBiometrics on the iPhone (#244); the iPhone's passcode is the fallback.
  ///
  /// In en, this message translates to:
  /// **'Unlock with Face ID or Touch ID'**
  String get unlockBiometricsIphone;

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
  /// **'Simple or Advanced and the Trash setting sync to your own Devices; other settings stay on each Device.'**
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

  /// In place of a value a newer version of the app saved in a form this version does not know (ADR 0034): a Routine's schedule in the list of all Routines and Reminders and under its Note, a person's role in a Shared Area, a Profile or the name and picture given to a person; the value is kept as it is and cannot be changed here.
  ///
  /// In en, this message translates to:
  /// **'Saved by a newer version of Old Brain Fix. Update the app to change it.'**
  String get newerVersionNeeded;

  /// The only screen shown, on the phone and the desktop, when this Device's database was saved by a newer version of the app (ADR 0034): the app does not open it, offers no Capture and changes nothing; the desktop's command line prints it too.
  ///
  /// In en, this message translates to:
  /// **'This Device\'s data was saved by a newer Old Brain Fix; install that version again.'**
  String get storeNeedsUpdate;

  /// Under storeNeedsUpdate on the screen shown in place of the app.
  ///
  /// In en, this message translates to:
  /// **'Nothing on this Device was changed.'**
  String get storeNeedsUpdateUnchanged;

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
  /// **'Add Notes and change their own, add to any Note, check off Routines, Reminders and Frogs, and add and check Lines of Checklists and Shopping Lists.'**
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
  /// **'A Manager may do everything here. A Member may add Notes and change their own, add to any Note, check off Routines, Reminders and Frogs, and add and check Lines of Checklists and Shopping Lists.'**
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

  /// No description provided for @sentItemMakesShoppingList.
  ///
  /// In en, this message translates to:
  /// **'It becomes a Shopping List.'**
  String get sentItemMakesShoppingList;

  /// Under a Line of a Shopping List in a Sent Item's preview; time is shoppingReappearNone, shoppingOnce, shoppingDays, shoppingWeeks, shoppingMonths or shoppingYears.
  ///
  /// In en, this message translates to:
  /// **'Reappear time: {time}'**
  String sentItemLineReappear(String time);

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
  /// **'Most belong to this Device; Simple or Advanced and the Trash setting sync to your own Devices.'**
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

  /// Announcement at the next start after the app died; tapping it opens the dialog of what happened, with Close and Report.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix stopped unexpectedly last time. Tap for details.'**
  String get crashLastRunBody;

  /// Title of the dialog the announcement after an unexpected stop opens.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix stopped unexpectedly'**
  String get crashLastRunTitle;

  /// Cause in the dialog after an unexpected stop, when a crash log or the system's exit record says what happened.
  ///
  /// In en, this message translates to:
  /// **'The last run ended with an error or was ended by the system; the text below says how.'**
  String get crashLastRunErrorCause;

  /// Cause in the dialog after an unexpected stop, when nothing recorded why the app ended.
  ///
  /// In en, this message translates to:
  /// **'The last run was ended while on screen without leaving an error behind, most likely because the phone ran short of memory.'**
  String get crashLastRunKilledCause;

  /// Fix sentence in the dialog after an unexpected stop, after the cause.
  ///
  /// In en, this message translates to:
  /// **'Your saved Notes are safe. “Report” sends this text to the developer by mail; “Close” sends nothing.'**
  String get crashLastRunFix;

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

  /// No description provided for @deleteNoteLinkedQuestion.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Delete this Note and its 1 Routine?} other{Delete this Note and its {count} Routines and Reminders?}}'**
  String deleteNoteLinkedQuestion(int count);

  /// No description provided for @deleteNoteReminderQuestion.
  ///
  /// In en, this message translates to:
  /// **'Delete this Note and its 1 Reminder?'**
  String get deleteNoteReminderQuestion;

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

  /// What a screen reader says for the Remove at the end of a captured detail's line in the Captured badge's dialog.
  ///
  /// In en, this message translates to:
  /// **'{detail, select, place{Remove the place} event{Remove the calendar events} device{Remove the Bluetooth devices} other{Remove the app}}'**
  String removeCapturedLabel(String detail);

  /// No description provided for @removeCapturedQuestion.
  ///
  /// In en, this message translates to:
  /// **'{detail, select, place{Remove the place from this Note?} event{Remove the calendar events from this Note?} device{Remove the Bluetooth devices from this Note?} other{Remove the app from this Note?}}'**
  String removeCapturedQuestion(String detail);

  /// No description provided for @removeCapturedSectionQuestion.
  ///
  /// In en, this message translates to:
  /// **'{detail, select, place{Remove the place from this Section?} event{Remove the calendar events from this Section?} device{Remove the Bluetooth devices from this Section?} other{Remove the app from this Section?}}'**
  String removeCapturedSectionQuestion(String detail);

  /// No description provided for @removeCapturedDetail.
  ///
  /// In en, this message translates to:
  /// **'It goes from all your Devices and from everyone you share the Note\'s Area with. The time it was captured stays.'**
  String get removeCapturedDetail;

  /// Announcement when Remove in the Captured badge's dialog was refused.
  ///
  /// In en, this message translates to:
  /// **'A captured detail could not be removed.'**
  String get removeCapturedFailed;

  /// No description provided for @removeCapturedFailedTitle.
  ///
  /// In en, this message translates to:
  /// **'Not removed'**
  String get removeCapturedFailedTitle;

  /// No description provided for @removeCapturedFailedCause.
  ///
  /// In en, this message translates to:
  /// **'This Device may not change this Note any more, for example because a role in its Area changed.'**
  String get removeCapturedFailedCause;

  /// No description provided for @removeCapturedFailedFix.
  ///
  /// In en, this message translates to:
  /// **'The detail stays on the Note. A Manager of the Area can remove it.'**
  String get removeCapturedFailedFix;

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
  /// **'As a small file, through any messenger.'**
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

  /// The fix in the dialog behind 'profileCardNotMade': this Device's key is not bound to the Profile until it synced with another of the user's Devices.
  ///
  /// In en, this message translates to:
  /// **'Sync with your other Device first, then try again.'**
  String get profileCardNotYet;

  /// The cause in the dialog behind 'profileCardNotSent': no app took the shared Profile card.
  ///
  /// In en, this message translates to:
  /// **'No app here can take the file.'**
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
  /// **'Ask the person for their Profile: as a file through a messenger, which opens in Old Brain Fix, or as a QR on their phone next to you. A QR scanned in person marks them verified at once.'**
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

  /// The Frog badge's value in the Note editor while the Frog is not eaten (#259).
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get frogOpen;

  /// No description provided for @frogEatenToday.
  ///
  /// In en, this message translates to:
  /// **'Eaten today'**
  String get frogEatenToday;

  /// No description provided for @frogEatenYesterday.
  ///
  /// In en, this message translates to:
  /// **'Eaten yesterday'**
  String get frogEatenYesterday;

  /// The Frog badge's menu: checks the Frog off.
  ///
  /// In en, this message translates to:
  /// **'Eat now'**
  String get frogEatNow;

  /// The Frog badge's menu: moves the Frog to the trash after asking; its Note stays.
  ///
  /// In en, this message translates to:
  /// **'Remove the Frog'**
  String get frogRemove;

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

  /// In a Tally card's menu on Android: puts a shortcut to the Tally's entry page on the home screen (#284).
  ///
  /// In en, this message translates to:
  /// **'Add to home screen'**
  String get addToHomeScreen;

  /// In a Tally card's menu on the iPhone: puts the Tally's entry page among the actions of the app icon's long press (#284).
  ///
  /// In en, this message translates to:
  /// **'Add to the app icon'**
  String get addToHomeScreenIphone;

  /// Announced when the phone's home screen cannot take a Tally's shortcut (#284).
  ///
  /// In en, this message translates to:
  /// **'This home screen takes no shortcuts.'**
  String get tallyShortcutNotTaken;

  /// Announced on the iPhone after Add to the app icon (#284).
  ///
  /// In en, this message translates to:
  /// **'{name} is in the app icon\'s menu: touch and hold the icon on the Home Screen.'**
  String tallyShortcutOnIcon(String name);

  /// Announced when a Tally's home-screen shortcut is tapped after the Tally was deleted or trashed (#284).
  ///
  /// In en, this message translates to:
  /// **'This Tally is no longer there.'**
  String get tallyShortcutGone;

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

  /// The compact view of a Checklist's card (#189): how many of its Lines are open.
  ///
  /// In en, this message translates to:
  /// **'{open} of {count} open'**
  String checklistOpenCount(int open, int count);

  /// The detailed view of a Shopping List's card (#189): the folded struck Lines, how many; a tap shows or hides them.
  ///
  /// In en, this message translates to:
  /// **'{count} struck through'**
  String shoppingStruckCount(int count);

  /// The compact view of a Tally's card (#189): a field's name, its total today as a number and its target with the unit, such as 'Steps 7000 / 10000 steps'.
  ///
  /// In en, this message translates to:
  /// **'{field} {total} / {target}'**
  String tallyCardTotal(String field, String total, String target);

  /// The compact view of a Tally's card (#189): a field without a target, its name and its total today with the unit, such as 'Books 1 books'.
  ///
  /// In en, this message translates to:
  /// **'{field} {total}'**
  String tallyCardTotalOnly(String field, String total);

  /// The compact view of the card of a Note that runs a Session (#189): its next run, a date and a time.
  ///
  /// In en, this message translates to:
  /// **'Next: {when}'**
  String sessionCardNext(String when);

  /// The compact view of the card of a Note that runs a Session (#189): how many of its runs were Done and how many skipped (#269) since the week began.
  ///
  /// In en, this message translates to:
  /// **'done {done} / skipped {skipped} this week'**
  String sessionCardWeek(int done, int skipped);

  /// The heading over a Session Note's history on its detailed card (#191): how often its Routines were Done and Missed in each of the last four weeks.
  ///
  /// In en, this message translates to:
  /// **'Last 4 weeks'**
  String get sessionHistory;

  /// One week of a Session Note's history on its detailed card (#191): the day the week starts, and how many runs of all its Routines were Done and how many skipped (#269) in it.
  ///
  /// In en, this message translates to:
  /// **'Week of {day}: done {done} / skipped {skipped}'**
  String sessionHistoryWeek(String day, int done, int skipped);

  /// A Session's past runs on its card and its page (#264): each Occurrence before today with a pick, newest first, its date and each picked Tally done or not done.
  ///
  /// In en, this message translates to:
  /// **'Past runs'**
  String get sessionRuns;

  /// No description provided for @sessionRunsNone.
  ///
  /// In en, this message translates to:
  /// **'No past runs yet.'**
  String get sessionRunsNone;

  /// No description provided for @sessionRunDone.
  ///
  /// In en, this message translates to:
  /// **'{name}: done'**
  String sessionRunDone(String name);

  /// No description provided for @sessionRunNotDone.
  ///
  /// In en, this message translates to:
  /// **'{name}: not done'**
  String sessionRunNotDone(String name);

  /// No description provided for @sessionOlderRuns.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 older run on the Session\'s page} other{{count} older runs on the Session\'s page}}'**
  String sessionOlderRuns(int count);

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

  /// The setting a Shopping List Line's badge shows and the title of its dialog (#195): how long the Line stays struck once checked (CONTEXT.md: Shopping List).
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

  /// Reappear time of a Line without one (#195): once checked it stays struck through until a time is set. What a new Line has.
  ///
  /// In en, this message translates to:
  /// **'None'**
  String get shoppingReappearNone;

  /// In the reappear time dialog of a Shopping List Line (#195), the choice whose number and unit follow below it: the Line comes back this long after it was checked.
  ///
  /// In en, this message translates to:
  /// **'After'**
  String get shoppingReappearAfter;

  /// Reappear time of a Line: it comes back this many days after it was checked (#195).
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 day} other{{count} days}}'**
  String shoppingDays(int count);

  /// Reappear time of a Line: it comes back on the same day of the month this many months after it was checked (#195).
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 month} other{{count} months}}'**
  String shoppingMonths(int count);

  /// Reappear time of a Line: it comes back on the same day this many years after it was checked (#195).
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 year} other{{count} years}}'**
  String shoppingYears(int count);

  /// Short text on a Shopping List Line's badge (#195): a reappear time of this many days.
  ///
  /// In en, this message translates to:
  /// **'{count}d'**
  String shoppingShortDays(int count);

  /// Short text on a Shopping List Line's badge (#195): a reappear time of this many weeks.
  ///
  /// In en, this message translates to:
  /// **'{count}w'**
  String shoppingShortWeeks(int count);

  /// Short text on a Shopping List Line's badge (#195): a reappear time of this many months.
  ///
  /// In en, this message translates to:
  /// **'{count}m'**
  String shoppingShortMonths(int count);

  /// Short text on a Shopping List Line's badge (#195): a reappear time of this many years.
  ///
  /// In en, this message translates to:
  /// **'{count}y'**
  String shoppingShortYears(int count);

  /// On the desktop, in a Shopping List Line's menu and on its confirm button: moves the Line to the trash, as a Checklist's Delete does (ADR 0030). The phone uses delete.
  ///
  /// In en, this message translates to:
  /// **'Delete'**
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

  /// No description provided for @tallyUnitFixed.
  ///
  /// In en, this message translates to:
  /// **'Entries hold values for this field, so its kind of unit stays.'**
  String get tallyUnitFixed;

  /// Hint in the one field a Tally field's unit is typed into; suggestions of known units show under it as chips.
  ///
  /// In en, this message translates to:
  /// **'e.g. kg, min, pages'**
  String get tallyUnitHint;

  /// No description provided for @tallyUnitHelper.
  ///
  /// In en, this message translates to:
  /// **'Tap a suggestion for a known unit; anything else stays as typed.'**
  String get tallyUnitHelper;

  /// Not shown: the names, separated by commas, that a typed unit is matched against to suggest this known unit (ADR 0028). Every language's names are matched in every language. The unitNames keys name kg, g, km, m, cm, km/h, °C, °, h, min, s and, in imperial, lb, oz, mi, ft, in, mph, °F.
  ///
  /// In en, this message translates to:
  /// **'kilogram, kilograms, kilogramme, kilo, kilos'**
  String get unitNamesKg;

  /// No description provided for @unitNamesG.
  ///
  /// In en, this message translates to:
  /// **'gram, grams, gramme'**
  String get unitNamesG;

  /// No description provided for @unitNamesKm.
  ///
  /// In en, this message translates to:
  /// **'kilometre, kilometres, kilometer, kilometers'**
  String get unitNamesKm;

  /// No description provided for @unitNamesM.
  ///
  /// In en, this message translates to:
  /// **'metre, metres, meter, meters'**
  String get unitNamesM;

  /// No description provided for @unitNamesCm.
  ///
  /// In en, this message translates to:
  /// **'centimetre, centimetres, centimeter, centimeters'**
  String get unitNamesCm;

  /// No description provided for @unitNamesKmh.
  ///
  /// In en, this message translates to:
  /// **'kilometres per hour, kilometers per hour, kmh, kph'**
  String get unitNamesKmh;

  /// No description provided for @unitNamesCelsius.
  ///
  /// In en, this message translates to:
  /// **'degrees Celsius, Celsius'**
  String get unitNamesCelsius;

  /// No description provided for @unitNamesDegree.
  ///
  /// In en, this message translates to:
  /// **'degree, degrees'**
  String get unitNamesDegree;

  /// No description provided for @unitNamesHour.
  ///
  /// In en, this message translates to:
  /// **'hour, hours, hr'**
  String get unitNamesHour;

  /// No description provided for @unitNamesMinute.
  ///
  /// In en, this message translates to:
  /// **'minute, minutes'**
  String get unitNamesMinute;

  /// No description provided for @unitNamesSecond.
  ///
  /// In en, this message translates to:
  /// **'second, seconds, sec'**
  String get unitNamesSecond;

  /// No description provided for @unitNamesPound.
  ///
  /// In en, this message translates to:
  /// **'pound, pounds, lbs'**
  String get unitNamesPound;

  /// No description provided for @unitNamesOunce.
  ///
  /// In en, this message translates to:
  /// **'ounce, ounces'**
  String get unitNamesOunce;

  /// No description provided for @unitNamesMile.
  ///
  /// In en, this message translates to:
  /// **'mile, miles'**
  String get unitNamesMile;

  /// No description provided for @unitNamesFoot.
  ///
  /// In en, this message translates to:
  /// **'foot, feet'**
  String get unitNamesFoot;

  /// No description provided for @unitNamesInch.
  ///
  /// In en, this message translates to:
  /// **'inch, inches'**
  String get unitNamesInch;

  /// No description provided for @unitNamesMph.
  ///
  /// In en, this message translates to:
  /// **'miles per hour'**
  String get unitNamesMph;

  /// No description provided for @unitNamesFahrenheit.
  ///
  /// In en, this message translates to:
  /// **'degrees Fahrenheit, Fahrenheit'**
  String get unitNamesFahrenheit;

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
  /// **'Per bar of the chart, prefilled in every new entry'**
  String get tallyTargetHint;

  /// No description provided for @tallyNotANumber.
  ///
  /// In en, this message translates to:
  /// **'Not a number. Type it like 12, 2.5 or 1:30.'**
  String get tallyNotANumber;

  /// No description provided for @tallyRule.
  ///
  /// In en, this message translates to:
  /// **'Several sets or entries in a bar count'**
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

  /// No description provided for @tallyFieldsMissing.
  ///
  /// In en, this message translates to:
  /// **'Give at least one field a name.'**
  String get tallyFieldsMissing;

  /// Value of a field's rule badge on the New Tally page, after the sign Σ.
  ///
  /// In en, this message translates to:
  /// **'Sum'**
  String get tallyRuleSumShort;

  /// No description provided for @tallyRuleMaxShort.
  ///
  /// In en, this message translates to:
  /// **'Largest'**
  String get tallyRuleMaxShort;

  /// No description provided for @tallyNoUnit.
  ///
  /// In en, this message translates to:
  /// **'No unit'**
  String get tallyNoUnit;

  /// No description provided for @tallyTargetBadge.
  ///
  /// In en, this message translates to:
  /// **'Target'**
  String get tallyTargetBadge;

  /// No description provided for @tallyNoTarget.
  ///
  /// In en, this message translates to:
  /// **'No target'**
  String get tallyNoTarget;

  /// A Tally's setting (#264), a badge in its setup: with it each entry holds several sets, each a row across the Tally's number fields (CONTEXT.md: Set).
  ///
  /// In en, this message translates to:
  /// **'Counts in sets'**
  String get tallyCountsInSets;

  /// No description provided for @tallyCountsInSetsDetail.
  ///
  /// In en, this message translates to:
  /// **'Each entry holds several sets, such as 60 kg × 10 and then 65 kg × 8. Each field totals the sets of an entry by its rule: reps by their sum, a weight by the largest.'**
  String get tallyCountsInSetsDetail;

  /// The button under the sets of a Tally entry (#264): adds a set that starts with the values of the set before.
  ///
  /// In en, this message translates to:
  /// **'Add set'**
  String get tallyAddSet;

  /// The heading of one set of a Tally entry (#264).
  ///
  /// In en, this message translates to:
  /// **'Set {number}'**
  String tallySet(int number);

  /// No description provided for @tallyRemoveSet.
  ///
  /// In en, this message translates to:
  /// **'Remove set {number}'**
  String tallyRemoveSet(int number);

  /// No description provided for @tallyFieldKind.
  ///
  /// In en, this message translates to:
  /// **'Kind of field'**
  String get tallyFieldKind;

  /// No description provided for @tallyFieldKindNumber.
  ///
  /// In en, this message translates to:
  /// **'Number'**
  String get tallyFieldKindNumber;

  /// A Tally field that is a box in each entry, a tick counting as 1 (#264, CONTEXT.md: Check field).
  ///
  /// In en, this message translates to:
  /// **'Check field'**
  String get tallyFieldKindCheck;

  /// No description provided for @tallyFieldKindCheckDetail.
  ///
  /// In en, this message translates to:
  /// **'A box in each entry. A tick counts as 1, so a bar shows how often it was done. Its kind stays once it is added.'**
  String get tallyFieldKindCheckDetail;

  /// Badge of a Tally field that multiplied another field before sets replaced multipliers (#264): its value in older entries is their number of sets.
  ///
  /// In en, this message translates to:
  /// **'Counts the sets of older entries'**
  String get tallySetCounter;

  /// No description provided for @tallyBarHint.
  ///
  /// In en, this message translates to:
  /// **'The target counts per bar.'**
  String get tallyBarHint;

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

  /// The detailed view of a Tally's card (#190): the older entries folded under the latest five, how many; a tap shows or hides them.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 older entry} other{{count} older entries}}'**
  String tallyOlderEntries(int count);

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

  /// A Note that picks Tallies from its pool for each Occurrence of its Routines (CONTEXT.md: Session); also the kind's name in the kind badge (#191).
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

  /// Under a Session's pool filtered by tags (#220): how many ticked Tallies the filter hides; they stay ticked.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 more ticked, hidden by the filter} other{{count} more ticked, hidden by the filter}}'**
  String sessionPoolHidden(int count);

  /// No description provided for @sessionSchedule.
  ///
  /// In en, this message translates to:
  /// **'Schedule'**
  String get sessionSchedule;

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
  /// **'A weekday without its own count picks as many as every day.'**
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

  /// In the ⋮ of a capture screen, and the title of the page it opens: makes each line of the Note being captured a Line of a Shopping List picked there, and keeps no Note (CONTEXT.md: Shopping List, Line).
  ///
  /// In en, this message translates to:
  /// **'Add to a Shopping List'**
  String get addToShoppingList;

  /// In the ⋮ of a capture screen while there is one Shopping List to add to, by its title: makes each line of the Note being captured a Line of it at once, with no page to pick from (CONTEXT.md: Shopping List, Line).
  ///
  /// In en, this message translates to:
  /// **'Add to {list}'**
  String addToNamedShoppingList(String list);

  /// On a capture screen: the Shopping List the lines being captured go into, by its title.
  ///
  /// In en, this message translates to:
  /// **'Shopping List: {list}'**
  String captureChoiceShopping(String list);

  /// Announcement after a voice capture sorted into a Shopping List whose recording got no Transcript: the Note stays (CONTEXT.md: Transcript).
  ///
  /// In en, this message translates to:
  /// **'Kept as a Note: its recording could not be transcribed for the Shopping List.'**
  String get captureShoppingUntranscribed;

  /// Announcement after a capture sorted into a Shopping List that was deleted or changed meanwhile: the Note stays.
  ///
  /// In en, this message translates to:
  /// **'Kept as a Note: that Shopping List can no longer be added to.'**
  String get captureShoppingGone;

  /// Title of the dialog behind captureShoppingUntranscribed and captureShoppingGone.
  ///
  /// In en, this message translates to:
  /// **'Not added to the Shopping List'**
  String get captureShoppingNotAddedTitle;

  /// Cause in the dialog behind captureShoppingUntranscribed (CONTEXT.md: Transcript).
  ///
  /// In en, this message translates to:
  /// **'Whisper could not write a Transcript of the recording.'**
  String get captureShoppingUntranscribedCause;

  /// Cause in the dialog behind captureShoppingGone.
  ///
  /// In en, this message translates to:
  /// **'The Shopping List was deleted or made a plain Note meanwhile, here or on another Device.'**
  String get captureShoppingGoneCause;

  /// Fix in the dialog behind captureShoppingUntranscribed and captureShoppingGone: nothing is lost.
  ///
  /// In en, this message translates to:
  /// **'The Note keeps all of it: add its lines to a Shopping List by hand.'**
  String get captureShoppingKeptFix;

  /// In the Routine editor opened from a capture screen, for a Note without text yet.
  ///
  /// In en, this message translates to:
  /// **'The Note being captured'**
  String get noteBeingCaptured;

  /// Bar button of a Note list that shows every card in its compact view (#189).
  ///
  /// In en, this message translates to:
  /// **'Show compact views'**
  String get showNotesCompact;

  /// Bar button of a Note list that shows every card in its detailed view (#189).
  ///
  /// In en, this message translates to:
  /// **'Show detailed views'**
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

  /// The cause in the dialog behind 'profileCardNotSent': the card file could not be written before sharing.
  ///
  /// In en, this message translates to:
  /// **'The file for your Profile card could not be written.'**
  String get profileCardNotWrittenCause;

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

  /// The fix in the dialog when an opened file or text is no Profile card.
  ///
  /// In en, this message translates to:
  /// **'Ask the person to send their Profile again.'**
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
  /// **'Try a picture taken a little closer, or ask the person to send their Profile.'**
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

  /// The entry in a Note's ⋮ menu and the editor's bar that hands the Note's text, photos and recordings to another app on the share sheet.
  ///
  /// In en, this message translates to:
  /// **'Share to an app'**
  String get shareToApp;

  /// Entry in a Note card's ⋮ menu that opens a small menu with Send to a person, Share to an app, Copy text and Export as Pack…
  ///
  /// In en, this message translates to:
  /// **'Share…'**
  String get shareNote;

  /// Announced when a Note could not be handed to another app; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The Note was not shared.'**
  String get noteNotShared;

  /// Title of the dialog behind 'noteNotShared'.
  ///
  /// In en, this message translates to:
  /// **'Note not shared'**
  String get noteNotSharedTitle;

  /// Why a Note was not shared: no app takes text and files.
  ///
  /// In en, this message translates to:
  /// **'No app here can take the Note.'**
  String get shareNoApp;

  /// The fix for 'shareNoApp'.
  ///
  /// In en, this message translates to:
  /// **'Install a messenger or a mail app, then share it again.'**
  String get shareNoAppFix;

  /// Why a Note was not shared: its files could not be written.
  ///
  /// In en, this message translates to:
  /// **'The photos and recordings of the Note could not be written for sharing.'**
  String get shareNotWrittenCause;

  /// Why a Note without words was not shared: none of its photos and recordings has arrived from another Device yet.
  ///
  /// In en, this message translates to:
  /// **'The Note\'s photos and recordings are not on this Device yet.'**
  String get shareNothingCause;

  /// The fix for 'shareNothingCause'.
  ///
  /// In en, this message translates to:
  /// **'Sync with your other Devices, then share it again.'**
  String get shareNothingFix;

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

  /// An Agenda row of a Checklist without a Routine: its title and how many of its Lines are not checked of all its Lines, such as 'Before the trip: 3 of 7 open'.
  ///
  /// In en, this message translates to:
  /// **'{list}: {open} of {count} open'**
  String checklistRowOpen(String list, int open, int count);

  /// A Checklist from a locked Area set to Masked, on the Agenda.
  ///
  /// In en, this message translates to:
  /// **'Private Checklist'**
  String get maskedChecklist;

  /// The heading of the three choices in the menu of a Checklist's or a Shopping List's card, and the name of its badge in the Note editor: how the list shows on today's Agenda while it has open Lines. The person's own choice, kept on their own Devices.
  ///
  /// In en, this message translates to:
  /// **'Show on the Agenda'**
  String get checklistShowOnAgenda;

  /// Radio choice under 'Show on the Agenda' of a Shopping List or a Checklist, a Shopping List's default: one Agenda row with the list's title and how many of its Lines are open, such as 'Groceries: 4 open' or 'Before the trip: 3 of 7 open'.
  ///
  /// In en, this message translates to:
  /// **'One row with the number of open Lines'**
  String get shoppingOnAgendaCount;

  /// Radio choice under 'Show on the Agenda' of a Shopping List or a Checklist, a Checklist's default: every open Line gets its own checkable row on the Agenda, under the list's name.
  ///
  /// In en, this message translates to:
  /// **'Each open Line on its own row'**
  String get shoppingOnAgendaLines;

  /// Menu entry at a status mark at the start of a line of a Note's text: changes it to ✅.
  ///
  /// In en, this message translates to:
  /// **'Check'**
  String get textMarkCheck;

  /// Menu entry at a status mark: changes it to ❌, failed or not to be done.
  ///
  /// In en, this message translates to:
  /// **'Fail'**
  String get textMarkFail;

  /// Menu entry at a status mark: changes it back to ☐, not done yet.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get textMarkReopen;

  /// Screen reader label of a ☐ at the start of a line of a text: not done yet.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get textMarkOpen;

  /// Screen reader label of a ✅ at the start of a line of a text.
  ///
  /// In en, this message translates to:
  /// **'Checked'**
  String get textMarkChecked;

  /// Screen reader label of a ❌ at the start of a line of a text.
  ///
  /// In en, this message translates to:
  /// **'Failed'**
  String get textMarkFailed;

  /// Label of the button in the bar above the keyboard that puts an open box ☐ in at the cursor, also in the middle of a word.
  ///
  /// In en, this message translates to:
  /// **'Put in ☐'**
  String get boxInsert;

  /// Screen reader hint of a box ☐ ✅ ❌ in a text that can be tapped: it opens Check, Fail, Open and History.
  ///
  /// In en, this message translates to:
  /// **'change the box'**
  String get boxChange;

  /// Switch in a Note's menu: shows after each box in its text who changed its state last and when. Kept for this Note on this Device only.
  ///
  /// In en, this message translates to:
  /// **'Show ticks'**
  String get boxShowTicks;

  /// Small grey text after a box with Show ticks on, and a line of its History: who changed its state and when, such as 'Ann, Tue 14:02'.
  ///
  /// In en, this message translates to:
  /// **'{name}, {when}'**
  String boxTickBy(String name, String when);

  /// Who changed the state of a box when it was the user, on one of their own Devices: 'You, Tue 14:02'.
  ///
  /// In en, this message translates to:
  /// **'You'**
  String get boxTickYou;

  /// Line under the title of a box's History dialog, which lists each change of its state with who made it and when.
  ///
  /// In en, this message translates to:
  /// **'Every change of this box, the oldest first.'**
  String get boxHistoryHint;

  /// Screen reader label of a word with # in front, such as #Garten, in a Note's text: a tap opens the search for it.
  ///
  /// In en, this message translates to:
  /// **'Search for {tag}'**
  String searchForTag(String tag);

  /// Button of the format bar above the keyboard: makes the selection or the word at the cursor bold.
  ///
  /// In en, this message translates to:
  /// **'Bold'**
  String get formatBold;

  /// Format bar button: italic.
  ///
  /// In en, this message translates to:
  /// **'Italic'**
  String get formatItalic;

  /// Format bar button: strike-through.
  ///
  /// In en, this message translates to:
  /// **'Strike through'**
  String get formatStrike;

  /// Format bar button: makes the cursor's line a large heading.
  ///
  /// In en, this message translates to:
  /// **'Large heading'**
  String get formatHeading1;

  /// Format bar button: makes the cursor's line a small heading.
  ///
  /// In en, this message translates to:
  /// **'Small heading'**
  String get formatHeading2;

  /// Format bar button: makes the lines a bulleted list.
  ///
  /// In en, this message translates to:
  /// **'Bulleted list'**
  String get formatBullets;

  /// Format bar button: makes the lines a numbered list.
  ///
  /// In en, this message translates to:
  /// **'Numbered list'**
  String get formatNumbers;

  /// Format bar button: makes the lines a quote.
  ///
  /// In en, this message translates to:
  /// **'Quote'**
  String get formatQuote;

  /// Format bar button and title of the dialog asking for a link's address.
  ///
  /// In en, this message translates to:
  /// **'Link'**
  String get formatLink;

  /// Format bar button: shows the selection or word as code, in a fixed-width font.
  ///
  /// In en, this message translates to:
  /// **'Code'**
  String get formatCode;

  /// Format bar button: puts the lines the selection touches into a fenced code block, shown in a fixed-width font with nothing in it read as a format; in a code block, takes its fences off.
  ///
  /// In en, this message translates to:
  /// **'Code block'**
  String get formatCodeBlock;

  /// Format bar button: moves a list item one level in.
  ///
  /// In en, this message translates to:
  /// **'Indent list item'**
  String get formatIndent;

  /// Format bar button: moves a list item one level out.
  ///
  /// In en, this message translates to:
  /// **'Move list item out'**
  String get formatOutdent;

  /// Screen reader label of the ⋮ button at the end of the format bar that opens the formats with no room on it.
  ///
  /// In en, this message translates to:
  /// **'More formats'**
  String get moreFormats;

  /// Label of the field for a link's address.
  ///
  /// In en, this message translates to:
  /// **'Web or e-mail address'**
  String get linkAddress;

  /// Hint under the address field when the cursor is in a link already.
  ///
  /// In en, this message translates to:
  /// **'Leave it empty to remove the link.'**
  String get linkRemoveHint;

  /// Button that puts the link on the selection or word.
  ///
  /// In en, this message translates to:
  /// **'Set link'**
  String get linkSet;

  /// Format bar button (#261): opens the picker of the app's pages, Areas, Topics and Notes; the one picked becomes an app link on the selection or word, or is put in with its name.
  ///
  /// In en, this message translates to:
  /// **'Link to…'**
  String get formatLinkTo;

  /// Hint of the search field of the Link to… picker (#261).
  ///
  /// In en, this message translates to:
  /// **'Search Notes, Areas and pages'**
  String get linkToSearch;

  /// Heading of the app's pages in the Link to… picker (#261): the Agenda, Unsorted, Settings and its sections, All Routines and Reminders, Backups, Sync, the Profile, the Frogs, New Session and the user guide.
  ///
  /// In en, this message translates to:
  /// **'Pages'**
  String get linkToPages;

  /// Heading of the Areas and Topics in the Link to… picker (#261).
  ///
  /// In en, this message translates to:
  /// **'Areas and Topics'**
  String get linkToPlaces;

  /// Heading of the Notes in the Link to… picker (#261).
  ///
  /// In en, this message translates to:
  /// **'Notes'**
  String get linkToNotes;

  /// Shown in the Link to… picker when the search finds nothing (#261).
  ///
  /// In en, this message translates to:
  /// **'Nothing found.'**
  String get linkToNothing;

  /// The user guide's start page on the website, as a page of the Link to… picker (#261).
  ///
  /// In en, this message translates to:
  /// **'User guide'**
  String get linkToGuide;

  /// Said when an app link's target cannot be seen on this Device (#261): a Note or Area deleted, in the trash, in a closed locked Area or never synced here, or a file not in the shared folder.
  ///
  /// In en, this message translates to:
  /// **'Not available here'**
  String get linkNotAvailable;

  /// Title of the one question before a file link opens (#261).
  ///
  /// In en, this message translates to:
  /// **'Open this file?'**
  String get fileLinkTitle;

  /// The one question before a file link opens (#261), naming the file and the app that opens it.
  ///
  /// In en, this message translates to:
  /// **'Open \"{file}\" from the shared folder with {app}?'**
  String fileLinkQuestion(String file, String app);

  /// Stands for {app} in fileLinkQuestion where the system does not tell which app opens the file (#261).
  ///
  /// In en, this message translates to:
  /// **'the app the system picks for it'**
  String get fileLinkSomeApp;

  /// Button of the question before a file link opens (#261).
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get fileLinkOpen;

  /// Format bar button: puts a new table at the cursor, or opens the table the cursor is in full screen.
  ///
  /// In en, this message translates to:
  /// **'Table'**
  String get formatTable;

  /// Title of the page that shows a table of a Note's text full screen.
  ///
  /// In en, this message translates to:
  /// **'Table'**
  String get tableTitle;

  /// Screen reader hint of a table in a text: a tap opens it full screen.
  ///
  /// In en, this message translates to:
  /// **'Open the table'**
  String get tableOpen;

  /// Screen reader label of the bar above a table shown full screen that moves the table left and right.
  ///
  /// In en, this message translates to:
  /// **'Move the table sideways'**
  String get tableSideways;

  /// Screen reader label of a row's number beside a table: a tap or long press opens the row's menu.
  ///
  /// In en, this message translates to:
  /// **'Row {number}'**
  String tableRow(int number);

  /// Screen reader label of a column's letter above a table: a tap or long press opens the column's menu.
  ///
  /// In en, this message translates to:
  /// **'Column {letter}'**
  String tableColumn(String letter);

  /// Screen reader label of the + under a table's rows: a tap edits a cell of a new row.
  ///
  /// In en, this message translates to:
  /// **'Add a row'**
  String get tableAddRow;

  /// Screen reader label of the + beside a table's columns: a tap edits a cell of a new column.
  ///
  /// In en, this message translates to:
  /// **'Add a column'**
  String get tableAddColumn;

  /// Menu item of a table's row: puts an empty row above it.
  ///
  /// In en, this message translates to:
  /// **'Insert a row above'**
  String get tableRowAbove;

  /// Menu item of a table's row: puts an empty row below it.
  ///
  /// In en, this message translates to:
  /// **'Insert a row below'**
  String get tableRowBelow;

  /// Menu item of a table's row: moves it one row up.
  ///
  /// In en, this message translates to:
  /// **'Move up'**
  String get tableRowUp;

  /// Menu item of a table's row: moves it one row down.
  ///
  /// In en, this message translates to:
  /// **'Move down'**
  String get tableRowDown;

  /// Menu item of a table's row: deletes it.
  ///
  /// In en, this message translates to:
  /// **'Delete row'**
  String get tableRowDelete;

  /// Menu item of a table's column: puts an empty column to its left.
  ///
  /// In en, this message translates to:
  /// **'Insert a column to the left'**
  String get tableColumnBefore;

  /// Menu item of a table's column: puts an empty column to its right.
  ///
  /// In en, this message translates to:
  /// **'Insert a column to the right'**
  String get tableColumnAfter;

  /// Menu item of a table's column: moves it one column to the left.
  ///
  /// In en, this message translates to:
  /// **'Move left'**
  String get tableColumnLeft;

  /// Menu item of a table's column: moves it one column to the right.
  ///
  /// In en, this message translates to:
  /// **'Move right'**
  String get tableColumnRight;

  /// Menu item of a table's column: deletes it.
  ///
  /// In en, this message translates to:
  /// **'Delete column'**
  String get tableColumnDelete;

  /// Switch in the menu of a table's first column: its cells are headings and stay on screen when the table moves sideways.
  ///
  /// In en, this message translates to:
  /// **'First column as headings'**
  String get tableHeadingColumn;

  /// Question before a row of a table that holds words is deleted.
  ///
  /// In en, this message translates to:
  /// **'Delete this row?'**
  String get tableDeleteRowQuestion;

  /// Question before a column of a table that holds words is deleted.
  ///
  /// In en, this message translates to:
  /// **'Delete this column?'**
  String get tableDeleteColumnQuestion;

  /// Question before the last row or column of a table is deleted, which deletes the whole table.
  ///
  /// In en, this message translates to:
  /// **'Delete the table?'**
  String get tableDeleteTableQuestion;

  /// Body of the question before a row, column or table is deleted.
  ///
  /// In en, this message translates to:
  /// **'The words in its cells are deleted from the Note.'**
  String get tableDeleteBody;

  /// In the menu of a column of a table edited as a grid in the editor (#257): its cells line up on the left (Markdown `:--`).
  ///
  /// In en, this message translates to:
  /// **'Align left'**
  String get tableAlignLeft;

  /// In the menu of a column of a table edited as a grid in the editor (#257): its cells are centred (Markdown `:-:`).
  ///
  /// In en, this message translates to:
  /// **'Centre'**
  String get tableAlignCenter;

  /// In the menu of a column of a table edited as a grid in the editor (#257): its cells line up on the right (Markdown `--:`).
  ///
  /// In en, this message translates to:
  /// **'Align right'**
  String get tableAlignRight;

  /// Button on a table edited as a grid in the editor (#257): shows the table's Markdown lines instead, to edit as text, until the cursor leaves them or Show as table is pressed.
  ///
  /// In en, this message translates to:
  /// **'Show as lines'**
  String get tableShowLines;

  /// Button beside a table shown as its Markdown lines in the editor (#257): shows it as a table again.
  ///
  /// In en, this message translates to:
  /// **'Show as table'**
  String get tableShowGrid;

  /// What a screen reader says for a cell of a table edited as a grid in the editor (#257), before the cell's words.
  ///
  /// In en, this message translates to:
  /// **'Row {number}, column {letter}'**
  String tableCell(int number, String letter);

  /// Under the pair code field, on the phone and the desktop, when the code typed or scanned is of a layout a later version of the app writes (ADR 0034): this version cannot join with it.
  ///
  /// In en, this message translates to:
  /// **'This code comes from a newer Old Brain Fix. Update Old Brain Fix on this Device, then try again.'**
  String get pairCodeNeedsUpdate;

  /// Why a Sent Item was not opened: it is in a format a later version of the app writes (ADR 0034).
  ///
  /// In en, this message translates to:
  /// **'This file comes from a newer Old Brain Fix.'**
  String get sentItemNeedsUpdate;

  /// Why a Backup was not restored: it is in a format a later version of the app writes (ADR 0034).
  ///
  /// In en, this message translates to:
  /// **'This Backup was made by a newer Old Brain Fix.'**
  String get restoreNeedsUpdate;

  /// The fix for 'sentItemNeedsUpdate' and 'restoreNeedsUpdate': the file is kept as it is.
  ///
  /// In en, this message translates to:
  /// **'Update Old Brain Fix on this Device, then open it again.'**
  String get updateOpenAgainFix;

  /// Announced at the top in place of 'updateNeeded' for the notice 'settingsNeedUpdate'; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'An update of Old Brain Fix is needed.'**
  String get settingsUpdateNeeded;

  /// The cause in the update notice when this Device would have changed a setting a later version of the app saved in a form this version does not know (ADR 0034): the setting is not changed.
  ///
  /// In en, this message translates to:
  /// **'A setting a newer Old Brain Fix saved on this Device is kept as it is.'**
  String get settingsNeedUpdate;

  /// The fix for 'settingsNeedUpdate'.
  ///
  /// In en, this message translates to:
  /// **'Install the latest Old Brain Fix on this Device.'**
  String get settingsUpdateFix;

  /// Button on the quick note (the text capture screen): saves the Note at once as Unsorted, nudged in 1 h, and opens it in the full Note editor.
  ///
  /// In en, this message translates to:
  /// **'To full editor'**
  String get toFullEditor;

  /// Section title in Settings for what the quick note (the text capture screen) shows.
  ///
  /// In en, this message translates to:
  /// **'Quick note'**
  String get settingsQuickNote;

  /// Explanation under the Settings section 'Quick note'.
  ///
  /// In en, this message translates to:
  /// **'The quick note shows the text, Photo, Voice, Gallery, Paste, File, \"To full editor\" and the alarm clock for a Routine or Reminder. Switch on here what it shows besides; all of it is in the full editor as well.'**
  String get settingsQuickNoteDetail;

  /// Switch in Settings ▸ Quick note: the format bar above the keyboard.
  ///
  /// In en, this message translates to:
  /// **'Format bar'**
  String get quickNoteFormatBar;

  /// Switch in Settings ▸ Quick note for the three buttons 'remind me to sort it in 10 min, 30 min, 1 h'.
  ///
  /// In en, this message translates to:
  /// **'Nudge buttons'**
  String get quickNoteNudgeButtons;

  /// Explanation under the switch 'Nudge buttons'.
  ///
  /// In en, this message translates to:
  /// **'Off: a single Save in their place, with a Nudge in 1 h, as on leaving.'**
  String get quickNoteNudgeButtonsDetail;

  /// Switch in Settings ▸ Quick note, only in Advanced: adds Frog and Session to the alarm clock's menu, which offers a Routine or Reminder without it (#223), and 'Make it a Tally' (Q481).
  ///
  /// In en, this message translates to:
  /// **'Frog, Session or Tally'**
  String get quickNoteSchedule;

  /// In the ⋮ of the phone's full-screen photo: puts the picture on the clipboard (#149).
  ///
  /// In en, this message translates to:
  /// **'Copy'**
  String get copyPhoto;

  /// In the ⋮ of the phone's full-screen photo: saves it into the phone's Pictures, in the album 'Old Brain Fix' (#149).
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get savePhoto;

  /// In the ⋮ of the phone's full-screen photo: hands it to the app picked on the share sheet (#149).
  ///
  /// In en, this message translates to:
  /// **'Share'**
  String get sharePhoto;

  /// Announced at the top of the phone when Save in the full-screen photo's ⋮ failed; the tap opens the cause and the fix.
  ///
  /// In en, this message translates to:
  /// **'The photo was not saved.'**
  String get photoSaveFailed;

  /// Title of the dialog behind 'photoSaveFailed'.
  ///
  /// In en, this message translates to:
  /// **'Photo not saved'**
  String get photoSaveFailedTitle;

  /// The cause in the dialog behind 'photoSaveFailed'.
  ///
  /// In en, this message translates to:
  /// **'The photo could not be written into the phone\'s Pictures.'**
  String get photoSaveFailedCause;

  /// The fix in the dialog behind 'photoSaveFailed'.
  ///
  /// In en, this message translates to:
  /// **'Nothing was lost; free some space on the phone and save it again.'**
  String get photoSaveFailedFix;

  /// The cause in the dialog behind 'photoSaveFailed' on Android 7 to 9.
  ///
  /// In en, this message translates to:
  /// **'Saving into the phone\'s Pictures needs Android 10 or later.'**
  String get photoSaveTooOldCause;

  /// The fix that goes with 'photoSaveTooOldCause'.
  ///
  /// In en, this message translates to:
  /// **'Use Share and pick an app that keeps files.'**
  String get photoSaveTooOldFix;

  /// Announced at the top of the phone when Copy in the full-screen photo's ⋮ failed; the tap opens the cause and the fix.
  ///
  /// In en, this message translates to:
  /// **'The photo was not copied.'**
  String get photoCopyFailed;

  /// Title of the dialog behind 'photoCopyFailed'.
  ///
  /// In en, this message translates to:
  /// **'Photo not copied'**
  String get photoCopyFailedTitle;

  /// The cause in the dialog behind 'photoCopyFailed'.
  ///
  /// In en, this message translates to:
  /// **'The phone\'s clipboard did not take the picture.'**
  String get photoCopyFailedCause;

  /// The fix in the dialog behind 'photoCopyFailed'.
  ///
  /// In en, this message translates to:
  /// **'Use Share to hand it to the other app instead.'**
  String get photoCopyFailedFix;

  /// Announced at the top of the phone when Share in the full-screen photo's ⋮ failed; the tap opens the cause and the fix.
  ///
  /// In en, this message translates to:
  /// **'The photo was not shared.'**
  String get photoShareFailed;

  /// Title of the dialog behind 'photoShareFailed'.
  ///
  /// In en, this message translates to:
  /// **'Photo not shared'**
  String get photoShareFailedTitle;

  /// The cause in the dialog behind 'photoShareFailed' when no app takes a picture.
  ///
  /// In en, this message translates to:
  /// **'No app here can take the photo.'**
  String get photoShareNoAppCause;

  /// The fix that goes with 'photoShareNoAppCause'.
  ///
  /// In en, this message translates to:
  /// **'Install a messenger or a gallery app, then share it again.'**
  String get photoShareNoAppFix;

  /// The cause in the dialogs behind 'photoSaveFailed', 'photoCopyFailed' and 'photoShareFailed' when the file handed on could not be written.
  ///
  /// In en, this message translates to:
  /// **'The photo\'s file could not be written on this phone.'**
  String get photoNotWrittenCause;

  /// Section title in Settings for the Import of a folder (CONTEXT.md: Import).
  ///
  /// In en, this message translates to:
  /// **'Import'**
  String get settingsImport;

  /// Settings row that opens the system's folder picker for an Import.
  ///
  /// In en, this message translates to:
  /// **'Import a folder'**
  String get importFolder;

  /// Subtitle of 'importFolder'.
  ///
  /// In en, this message translates to:
  /// **'Each Markdown or text file in it becomes a Note in a new Area, dated when the file was last changed.'**
  String get importFolderDetail;

  /// Subtitle of 'importFolder' while the picked folder's files are read and imported.
  ///
  /// In en, this message translates to:
  /// **'Reading the folder…'**
  String get importReading;

  /// Name of the new Area an Import makes, after the picked folder's name. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'From {folder}'**
  String importAreaName(String folder);

  /// Announced after an Import that made Notes; the tap opens the dialog with what went where. Carries no values (UI laws). Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'Import finished.'**
  String get importFinished;

  /// First line in the dialog behind 'importFinished'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 Note went into the new Area {area}.} other{{count} Notes went into the new Area {area}.}}'**
  String importFinishedNotes(int count, String area);

  /// In the dialog behind 'importFinished' when files were skipped, above 'importSkippedDetails'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 file was skipped:} other{{count} files were skipped:}}'**
  String importFinishedSkipped(int count);

  /// Button in the dialog behind 'importFinished' that opens the new Area. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'Open the Area'**
  String get importOpenArea;

  /// Announced when an Import found nothing to make Notes of; the tap opens the dialog with the cause and the fix. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'No Markdown or text file found.'**
  String get importNothing;

  /// Title of the dialog behind 'importNothing'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'Nothing imported'**
  String get importNothingTitle;

  /// The cause in the dialog behind 'importNothing'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'The folder and its folders hold no Markdown file (.md, .markdown) or text file (.txt) that could be taken; other files and files that are too large are skipped.'**
  String get importNothingCause;

  /// The fix in the dialog behind 'importNothing'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'Pick the folder the notes lie in. Nothing was changed.'**
  String get importNothingFix;

  /// The details in the dialog behind 'importNothing': the folder and the files skipped. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'Folder: {folder}\nNot Markdown or text: {notText}\nToo large: {tooLarge}'**
  String importSkippedDetails(String folder, int notText, int tooLarge);

  /// Announced when the picked folder or one of its files could not be read; the tap opens the dialog with the cause and the fix. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'The folder could not be imported.'**
  String get importFailed;

  /// Title of the dialog behind 'importFailed'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'Import failed'**
  String get importFailedTitle;

  /// The cause in the dialog behind 'importFailed'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'The folder or one of its files could not be read.'**
  String get importFailedCause;

  /// The fix in the dialog behind 'importFailed' on the phone.
  ///
  /// In en, this message translates to:
  /// **'Nothing was imported. Copy the folder onto the phone and import it from there.'**
  String get importFailedFix;

  /// Settings row under 'importFolder' that imports a Google Takeout export of Google Keep (ADR 0036); also the title of the dialog behind it. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'Import from Google Keep'**
  String get importKeep;

  /// Subtitle of 'importKeep'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'The notes, lists and photos of a Google Takeout export become Notes in a new Area, dated when they were made in Keep.'**
  String get importKeepDetail;

  /// Subtitle of 'importKeep' while the picked export is read and imported. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'Reading the export…'**
  String get importKeepReading;

  /// The question in the dialog behind 'importKeep', above 'importKeepZip' and 'importKeepFolder'.
  ///
  /// In en, this message translates to:
  /// **'Is the Takeout export a zip file, or a folder unpacked from it?'**
  String get importKeepAsk;

  /// Button in the dialog behind 'importKeep' that opens the system's file picker for the Takeout zip.
  ///
  /// In en, this message translates to:
  /// **'Zip file'**
  String get importKeepZip;

  /// Button in the dialog behind 'importKeep' that opens the system's folder picker for the unpacked Takeout export.
  ///
  /// In en, this message translates to:
  /// **'Folder'**
  String get importKeepFolder;

  /// Name of the new Area the import from Google Keep makes. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'From Google Keep'**
  String get importKeepAreaName;

  /// In the dialog behind 'importFinished' after an import from Google Keep that made Checklists, under 'importFinishedNotes'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 of them is a Checklist. Checklists stay off the Agenda until you switch them on.} other{{count} of them are Checklists. Checklists stay off the Agenda until you switch them on.}}'**
  String importKeepChecklists(int count);

  /// In the dialog behind 'importFinished' after an import from Google Keep that brought photos. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 photo went into the Area with them.} other{{count} photos went into the Area with them.}}'**
  String importKeepPhotos(int count);

  /// In the dialog behind 'importFinished' after an import from Google Keep that skipped some, above 'importKeepSkippedDetails'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 note or file was skipped:} other{{count} notes or files were skipped:}}'**
  String importKeepSkipped(int count);

  /// What an import from Google Keep skipped, by reason, in the dialogs behind 'importFinished' and 'importKeepNothing'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'In Keep\'s trash: {trashed}\nMissing attachment: {missing}\nNot a photo: {notPhoto}\nToo large: {tooLarge}\nNot text: {notText}'**
  String importKeepSkippedDetails(
    int trashed,
    int missing,
    int notPhoto,
    int tooLarge,
    int notText,
  );

  /// Announced when an import from Google Keep found no note to make a Note of; the tap opens the dialog with the cause and the fix, titled 'importNothingTitle'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'No Keep note found.'**
  String get importKeepNothing;

  /// The cause in the dialog behind 'importKeepNothing'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'The export holds no note of Google Keep that could be taken; notes in Keep\'s trash and files that are too large are skipped.'**
  String get importKeepNothingCause;

  /// The fix in the dialog behind 'importKeepNothing'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'Pick the Takeout zip file, or the folder unpacked from it, with the folder Keep inside. Nothing was changed.'**
  String get importKeepNothingFix;

  /// Announced when the picked Google Keep export or one of its files could not be read; the tap opens the dialog with the cause and the fix, titled 'importFailedTitle'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'The export could not be imported.'**
  String get importKeepFailed;

  /// The cause in the dialog behind 'importKeepFailed'. Shared with the desktop.
  ///
  /// In en, this message translates to:
  /// **'The zip file or the folder, or one of the files in it, could not be read.'**
  String get importKeepFailedCause;

  /// The fix in the dialog behind 'importKeepFailed' on the phone.
  ///
  /// In en, this message translates to:
  /// **'Nothing was imported. Download the export again, copy it onto the phone and import it from there.'**
  String get importKeepFailedFix;

  /// What a screen reader reads for a badge of a Note's setting in the Note editor (#160): the setting's name and its current value.
  ///
  /// In en, this message translates to:
  /// **'{setting}: {value}'**
  String settingBadgeLabel(String setting, String value);

  /// The name of the Note editor's badge that shows where the Note is filed (#160).
  ///
  /// In en, this message translates to:
  /// **'Area and Topic'**
  String get badgePlace;

  /// The name of the Note editor's badge that shows the Note's kind: a plain Note, a Checklist, a Shopping List or a Tally (#160, #188).
  ///
  /// In en, this message translates to:
  /// **'Kind'**
  String get badgeKind;

  /// The kind of a Note of text alone (#188): the value of the kind badge and a choice in its dialog.
  ///
  /// In en, this message translates to:
  /// **'Plain Note'**
  String get kindPlain;

  /// The label of the one-line field for the name of a Note of a kind, a Checklist, Shopping List or Tally (#188): the first line of its text, shown as its title.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get noteName;

  /// The label of the field for the description of a Note of a kind (#188): the rest of its text after the name, shown small under it.
  ///
  /// In en, this message translates to:
  /// **'Description'**
  String get noteDescription;

  /// What a screen reader reads, and the tooltip, for the description of a Note of a kind folded to one line under its name (#188): a tap unfolds it.
  ///
  /// In en, this message translates to:
  /// **'Show the whole description'**
  String get noteDescriptionUnfold;

  /// What a screen reader reads, and the tooltip, for an unfolded description of a Note of a kind (#188): a tap folds it back to one line.
  ///
  /// In en, this message translates to:
  /// **'Fold the description'**
  String get noteDescriptionFold;

  /// The value of a Note editor's badge for a setting that is switched on, such as Show ticks (#160).
  ///
  /// In en, this message translates to:
  /// **'On'**
  String get settingOn;

  /// The value of a Note editor's badge for a setting that is switched off (#160).
  ///
  /// In en, this message translates to:
  /// **'Off'**
  String get settingOff;

  /// The heading of the Agenda's section in Settings (#217).
  ///
  /// In en, this message translates to:
  /// **'Agenda'**
  String get settingsAgenda;

  /// The switch in Settings > Agenda that hides what is checked on the Agenda instead of striking it through (#217).
  ///
  /// In en, this message translates to:
  /// **'Hide what\'s checked on the Agenda'**
  String get hideCheckedOnAgenda;

  /// Under the switch hideCheckedOnAgenda on the phone.
  ///
  /// In en, this message translates to:
  /// **'A Done Occurrence, a checked Line and a Session\'s pick with its entry leave the Agenda and the widget at once instead of staying struck through; a group with nothing left goes with them.'**
  String get hideCheckedOnAgendaDetail;

  /// The switch in Settings > Agenda that moves a plain Note to the trash once its only Reminder is checked off (#211); a setting of the person, the same on all of their own Devices.
  ///
  /// In en, this message translates to:
  /// **'Move a Note to the Trash when its one-time Reminder is done'**
  String get trashDoneReminder;

  /// Under the switch trashDoneReminder, on the phone and the desktop.
  ///
  /// In en, this message translates to:
  /// **'Done on a one-time Reminder that is a plain Note\'s only Routine or Reminder moves the Note to the trash with it, wherever you check it off. Lists, Tallies and Sessions stay. The same on all your own Devices; unchecking doesn\'t bring the Note back, the trash does for 30 days.'**
  String get trashDoneReminderDetail;

  /// At the top of the Note editor (#215): an Occurrence of the Note's Routine or Reminder that is due now, with its time ("Due 15:00"), before the Done button.
  ///
  /// In en, this message translates to:
  /// **'Due {when}'**
  String occurrenceDue(String when);

  /// At the top of the Note editor (#215): the latest Missed Occurrence of the Note's Routine or Reminder, with its day and time, before the Done button.
  ///
  /// In en, this message translates to:
  /// **'Missed {when}'**
  String occurrenceMissedAt(String when);

  /// At the top of the Note editor (#215), beside Done for a one-time Reminder: checks it off and moves the Note with it to the trash.
  ///
  /// In en, this message translates to:
  /// **'Done and move to trash'**
  String get doneAndTrash;

  /// The name of the Note editor's badge that shows the format of a Plain Note's text (#213): plain, formatted or code.
  ///
  /// In en, this message translates to:
  /// **'Text format'**
  String get badgeTextFormat;

  /// A value of the text format badge (#213): the text as typed, without formatting.
  ///
  /// In en, this message translates to:
  /// **'Plain text'**
  String get textFormatPlain;

  /// A value of the text format badge (#213): Markdown formatting, as the format bar applies it.
  ///
  /// In en, this message translates to:
  /// **'Formatted text'**
  String get textFormatFormatted;

  /// A value of the text format badge (#213): the whole text as code, in a fixed-width font exactly as written, unwrapped, nothing in it read as a format, a box or a #tag.
  ///
  /// In en, this message translates to:
  /// **'Code'**
  String get textFormatCode;

  /// Under the Note's text in the Note editor after a paste that looks like a script or code (#213), before the button keepAsCode.
  ///
  /// In en, this message translates to:
  /// **'This looks like code.'**
  String get codePasted;

  /// The button under the Note's text in the Note editor after a paste that looks like code (#213): makes the whole text a Code text.
  ///
  /// In en, this message translates to:
  /// **'Keep as code'**
  String get keepAsCode;

  /// Line under the title of a Note's History page (#193).
  ///
  /// In en, this message translates to:
  /// **'Every change of this Note, its Amends and its Lines, the newest first.'**
  String get noteHistoryHint;

  /// A Note's History: the Note was captured with the text shown under it.
  ///
  /// In en, this message translates to:
  /// **'Captured'**
  String get noteHistoryCaptured;

  /// A Note's History: the Note's text was changed; the old and the new text show under it.
  ///
  /// In en, this message translates to:
  /// **'Text changed'**
  String get noteHistoryText;

  /// A Note's History: an Amend added a Section, its text under it.
  ///
  /// In en, this message translates to:
  /// **'Amended'**
  String get noteHistoryAmended;

  /// A Note's History: the text of an Amend (a Section) was changed; old and new text under it.
  ///
  /// In en, this message translates to:
  /// **'Amend changed'**
  String get noteHistoryAmendEdited;

  /// A Note's History: another Note was merged into this one, its text under it.
  ///
  /// In en, this message translates to:
  /// **'Note merged in'**
  String get noteHistoryMergedIn;

  /// A Note's History: a Line of a Checklist or Shopping List was added, its text under it.
  ///
  /// In en, this message translates to:
  /// **'Line added'**
  String get noteHistoryLineAdded;

  /// A Note's History: a Line was renamed; old and new text under it.
  ///
  /// In en, this message translates to:
  /// **'Line renamed'**
  String get noteHistoryLineRenamed;

  /// A Note's History: a Line was checked, its text under it.
  ///
  /// In en, this message translates to:
  /// **'Line checked'**
  String get noteHistoryLineChecked;

  /// A Note's History: a Line was unchecked, its text under it.
  ///
  /// In en, this message translates to:
  /// **'Line unchecked'**
  String get noteHistoryLineUnchecked;

  /// A Note's History: a Line was deleted, its text under it.
  ///
  /// In en, this message translates to:
  /// **'Line deleted'**
  String get noteHistoryLineRemoved;

  /// A Note's History: a deleted Line came back, its text under it.
  ///
  /// In en, this message translates to:
  /// **'Line restored'**
  String get noteHistoryLineRestored;

  /// A Note's History: the Note went to the trash.
  ///
  /// In en, this message translates to:
  /// **'Moved to the trash'**
  String get noteHistoryTrashed;

  /// A Note's History: the Note came back from the trash.
  ///
  /// In en, this message translates to:
  /// **'Restored from the trash'**
  String get noteHistoryRestored;

  /// A Note's History: a photo was added.
  ///
  /// In en, this message translates to:
  /// **'Photo added'**
  String get noteHistoryPhotoAdded;

  /// A Note's History: a photo was removed.
  ///
  /// In en, this message translates to:
  /// **'Photo removed'**
  String get noteHistoryPhotoRemoved;

  /// A Note's History: a voice recording was added.
  ///
  /// In en, this message translates to:
  /// **'Recording added'**
  String get noteHistoryRecordingAdded;

  /// A Note's History: a voice recording was removed.
  ///
  /// In en, this message translates to:
  /// **'Recording removed'**
  String get noteHistoryRecordingRemoved;

  /// A Note's History: the Area or Topic the Note was filed into, deleted since.
  ///
  /// In en, this message translates to:
  /// **'An Area or Topic that is gone'**
  String get noteHistoryPlaceGone;

  /// A Note's History: the Area or Topic the Note was filed into lies in a locked Area that is not open now, so its name is not shown.
  ///
  /// In en, this message translates to:
  /// **'A locked Area'**
  String get noteHistoryLocked;

  /// A Note's History: the label of the text before a change.
  ///
  /// In en, this message translates to:
  /// **'Before'**
  String get noteHistoryBefore;

  /// A Note's History: the label of the text after a change.
  ///
  /// In en, this message translates to:
  /// **'After'**
  String get noteHistoryAfter;

  /// A Note's History: writes this old text as the Note's or the Amend's text again, as a new change; nothing is deleted (#193).
  ///
  /// In en, this message translates to:
  /// **'Use this version'**
  String get useThisVersion;

  /// Short message after Use this version (#193).
  ///
  /// In en, this message translates to:
  /// **'The old text is back. The History keeps every version.'**
  String get useThisVersionDone;

  /// The heading of the Simple / Advanced choice on the first start's Starter Set page and in Settings (#197); a setting of the person, the same on all of their own Devices.
  ///
  /// In en, this message translates to:
  /// **'What Old Brain Fix offers'**
  String get features;

  /// Under the heading features (#197).
  ///
  /// In en, this message translates to:
  /// **'What you already made stays, either way. The same on all your own Devices; Settings changes it any time.'**
  String get featuresDetail;

  /// The Simple choice of features (#197).
  ///
  /// In en, this message translates to:
  /// **'Simple'**
  String get featuresSimple;

  /// Under featuresSimple (#197): what Simple offers to make.
  ///
  /// In en, this message translates to:
  /// **'Notes, Checklists, Shopping Lists, Reminders and Routines.'**
  String get featuresSimpleDetail;

  /// The Advanced choice of features (#197).
  ///
  /// In en, this message translates to:
  /// **'Advanced'**
  String get featuresAdvanced;

  /// Under featuresAdvanced (#197): what Advanced adds.
  ///
  /// In en, this message translates to:
  /// **'Adds Tallies, Sessions, Frogs, Topics, the format bar and roles in Shared Areas.'**
  String get featuresAdvancedDetail;

  /// The name of the Area the app's own help Notes are filed into (#180); the app's name, the same in every language.
  ///
  /// In en, this message translates to:
  /// **'Old Brain Fix'**
  String get helpAreaName;

  /// The name of the help Checklist of steps to try (#180).
  ///
  /// In en, this message translates to:
  /// **'Quick start'**
  String get helpQuickStartTitle;

  /// A Line of the Quick start Checklist (#180, #260): where and how on the phone, the exact place and gesture, up to about 20 words, nothing of Advanced (no Tallies, Sessions, Frogs or Topics).
  ///
  /// In en, this message translates to:
  /// **'Tap the round + at the bottom right of the Agenda, then Text, type a thought and tap 1 h.'**
  String get helpQuickStart1;

  /// No description provided for @helpQuickStart2.
  ///
  /// In en, this message translates to:
  /// **'On the Agenda, tap \"1 Unsorted\", then the folder with an arrow under the Note, and pick an Area.'**
  String get helpQuickStart2;

  /// No description provided for @helpQuickStart3.
  ///
  /// In en, this message translates to:
  /// **'Tap + and Text, type a task, tap the alarm clock, then Reminder; set its time and tap Save twice.'**
  String get helpQuickStart3;

  /// No description provided for @helpQuickStart4.
  ///
  /// In en, this message translates to:
  /// **'Tap + and Text, type a habit, tap the alarm clock, then Routine; choose when it repeats and tap Save twice.'**
  String get helpQuickStart4;

  /// No description provided for @helpQuickStart5.
  ///
  /// In en, this message translates to:
  /// **'Tap the box in front of this Line to tick it: that is how a Checklist works.'**
  String get helpQuickStart5;

  /// No description provided for @helpQuickStart6.
  ///
  /// In en, this message translates to:
  /// **'Tap the folder at the top of the Agenda, or swipe left, to search all Notes and open your Areas.'**
  String get helpQuickStart6;

  /// No description provided for @helpQuickStart7.
  ///
  /// In en, this message translates to:
  /// **'Tap your Profile picture at the top right of the Agenda, then \"Back up this Device\" and \"Set up Backups\".'**
  String get helpQuickStart7;

  /// No description provided for @helpQuickStart8.
  ///
  /// In en, this message translates to:
  /// **'Tap your Profile picture, then \"Sync with another Device\", and scan the code your other Device shows.'**
  String get helpQuickStart8;

  /// The first line of the Quick start's Description (#260), folded under its name: the desktop's way for each step follows, one line each.
  ///
  /// In en, this message translates to:
  /// **'On the desktop (Cmd for Ctrl on macOS):'**
  String get helpQuickStartDesktopTitle;

  /// The desktop's way for the Quick start's Line of the same number (#260), a line of its Description: the step's short name, a colon, then the menus, keys and clicks; nothing of Advanced.
  ///
  /// In en, this message translates to:
  /// **'Capture: File ▸ New Note or Ctrl+N, type, then Ctrl+Enter.'**
  String get helpQuickStartDesktop1;

  /// No description provided for @helpQuickStartDesktop2.
  ///
  /// In en, this message translates to:
  /// **'File: in Unsorted in the sidebar, drag the card onto an Area, or right-click it, File into…, and pick the Area.'**
  String get helpQuickStartDesktop2;

  /// No description provided for @helpQuickStartDesktop3.
  ///
  /// In en, this message translates to:
  /// **'Reminder: in the quick note, click the alarm clock, then Reminder; set its time, click Save, then Ctrl+Enter.'**
  String get helpQuickStartDesktop3;

  /// No description provided for @helpQuickStartDesktop4.
  ///
  /// In en, this message translates to:
  /// **'Routine: the same, with Routine.'**
  String get helpQuickStartDesktop4;

  /// No description provided for @helpQuickStartDesktop5.
  ///
  /// In en, this message translates to:
  /// **'Tick: click the box in front of a Line.'**
  String get helpQuickStartDesktop5;

  /// No description provided for @helpQuickStartDesktop6.
  ///
  /// In en, this message translates to:
  /// **'Search and browse: click an Area in the sidebar, or Edit ▸ Find (Ctrl+F).'**
  String get helpQuickStartDesktop6;

  /// No description provided for @helpQuickStartDesktop7.
  ///
  /// In en, this message translates to:
  /// **'Backup: Sync ▸ Backups…, then \"Set up Backups\".'**
  String get helpQuickStartDesktop7;

  /// No description provided for @helpQuickStartDesktop8.
  ///
  /// In en, this message translates to:
  /// **'Sync: Sync ▸ Pair a Device.'**
  String get helpQuickStartDesktop8;

  /// The first line of the Quick start's Description (#261): a numbered link to the place of each step follows, one line each, before the desktop's way.
  ///
  /// In en, this message translates to:
  /// **'Go where each step is done:'**
  String get helpQuickStartPlacesTitle;

  /// The place of the Quick start's step 5, ticking a Line, in its list of places (#261): the Checklist itself, so no link.
  ///
  /// In en, this message translates to:
  /// **'Here, in this Checklist.'**
  String get helpQuickStartPlaceHere;

  /// The last line of the help Note on icons (#261), a link to the user guide's start page.
  ///
  /// In en, this message translates to:
  /// **'Everything else is in the user guide.'**
  String get helpIconsGuide;

  /// The name of the help Note listing the icons with their names and meanings (#180).
  ///
  /// In en, this message translates to:
  /// **'What the icons mean'**
  String get helpIconsTitle;

  /// A group heading of the help Note on icons, and of its picture (#180).
  ///
  /// In en, this message translates to:
  /// **'In a Note\'s badges'**
  String get helpIconsBadges;

  /// No description provided for @helpIconsAgenda.
  ///
  /// In en, this message translates to:
  /// **'On the Agenda'**
  String get helpIconsAgenda;

  /// No description provided for @helpIconsCard.
  ///
  /// In en, this message translates to:
  /// **'On a Note card'**
  String get helpIconsCard;

  /// An icon's name in the help Note on icons (#180); its meaning follows in helpIconKindMeans.
  ///
  /// In en, this message translates to:
  /// **'Kind'**
  String get helpIconKind;

  /// An icon's meaning in the help Note on icons (#180), after its name and a colon.
  ///
  /// In en, this message translates to:
  /// **'a plain Note, Checklist or Shopping List; in Advanced also a Tally or Session.'**
  String get helpIconKindMeans;

  /// No description provided for @helpIconImportance.
  ///
  /// In en, this message translates to:
  /// **'Importance'**
  String get helpIconImportance;

  /// No description provided for @helpIconImportanceMeans.
  ///
  /// In en, this message translates to:
  /// **'how important the Note is.'**
  String get helpIconImportanceMeans;

  /// No description provided for @helpIconPlace.
  ///
  /// In en, this message translates to:
  /// **'Area and Topic'**
  String get helpIconPlace;

  /// No description provided for @helpIconPlaceMeans.
  ///
  /// In en, this message translates to:
  /// **'where the Note is filed; new Topics are made in Advanced.'**
  String get helpIconPlaceMeans;

  /// No description provided for @helpIconCaptured.
  ///
  /// In en, this message translates to:
  /// **'Captured'**
  String get helpIconCaptured;

  /// No description provided for @helpIconCapturedMeans.
  ///
  /// In en, this message translates to:
  /// **'when you captured the Note.'**
  String get helpIconCapturedMeans;

  /// No description provided for @helpIconReminder.
  ///
  /// In en, this message translates to:
  /// **'Reminder'**
  String get helpIconReminder;

  /// No description provided for @helpIconReminderMeans.
  ///
  /// In en, this message translates to:
  /// **'rings once at its time.'**
  String get helpIconReminderMeans;

  /// No description provided for @helpIconRoutine.
  ///
  /// In en, this message translates to:
  /// **'Routine'**
  String get helpIconRoutine;

  /// No description provided for @helpIconRoutineMeans.
  ///
  /// In en, this message translates to:
  /// **'rings again on its days.'**
  String get helpIconRoutineMeans;

  /// No description provided for @helpIconFrog.
  ///
  /// In en, this message translates to:
  /// **'Frog'**
  String get helpIconFrog;

  /// No description provided for @helpIconFrogMeans.
  ///
  /// In en, this message translates to:
  /// **'the Note\'s Frog, open or just eaten; made in Advanced.'**
  String get helpIconFrogMeans;

  /// No description provided for @helpIconAdd.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get helpIconAdd;

  /// No description provided for @helpIconAddMeans.
  ///
  /// In en, this message translates to:
  /// **'gives the Note a Routine or Reminder, in Advanced also a Frog or Session.'**
  String get helpIconAddMeans;

  /// No description provided for @helpIconBrowse.
  ///
  /// In en, this message translates to:
  /// **'Search and browse'**
  String get helpIconBrowse;

  /// No description provided for @helpIconBrowseMeans.
  ///
  /// In en, this message translates to:
  /// **'opens your Areas and searches every Note.'**
  String get helpIconBrowseMeans;

  /// No description provided for @helpIconAllRoutines.
  ///
  /// In en, this message translates to:
  /// **'All Routines and Reminders'**
  String get helpIconAllRoutines;

  /// No description provided for @helpIconAllRoutinesMeans.
  ///
  /// In en, this message translates to:
  /// **'lists every Routine and Reminder.'**
  String get helpIconAllRoutinesMeans;

  /// No description provided for @helpIconNewRoutine.
  ///
  /// In en, this message translates to:
  /// **'New Routine or Reminder'**
  String get helpIconNewRoutine;

  /// No description provided for @helpIconNewRoutineMeans.
  ///
  /// In en, this message translates to:
  /// **'makes one with a new Note.'**
  String get helpIconNewRoutineMeans;

  /// The Agenda bar's New Session in the help Note on icons (#180, #246); its meaning follows in helpIconNewSessionMeans.
  ///
  /// In en, this message translates to:
  /// **'New Session'**
  String get helpIconNewSession;

  /// No description provided for @helpIconNewSessionMeans.
  ///
  /// In en, this message translates to:
  /// **'makes a Session with a new Note, in Advanced.'**
  String get helpIconNewSessionMeans;

  /// No description provided for @helpIconUnsorted.
  ///
  /// In en, this message translates to:
  /// **'Unsorted'**
  String get helpIconUnsorted;

  /// No description provided for @helpIconUnsortedMeans.
  ///
  /// In en, this message translates to:
  /// **'Notes still to file into an Area.'**
  String get helpIconUnsortedMeans;

  /// No description provided for @helpIconFileInto.
  ///
  /// In en, this message translates to:
  /// **'File into…'**
  String get helpIconFileInto;

  /// No description provided for @helpIconFileIntoMeans.
  ///
  /// In en, this message translates to:
  /// **'moves the Note into an Area or Topic.'**
  String get helpIconFileIntoMeans;

  /// No description provided for @helpIconAlarm.
  ///
  /// In en, this message translates to:
  /// **'Routine or Reminder'**
  String get helpIconAlarm;

  /// No description provided for @helpIconAlarmMeans.
  ///
  /// In en, this message translates to:
  /// **'gives the card\'s Note one, in Advanced also a Frog or Session.'**
  String get helpIconAlarmMeans;

  /// No description provided for @helpIconMerge.
  ///
  /// In en, this message translates to:
  /// **'Merge into…'**
  String get helpIconMerge;

  /// No description provided for @helpIconMergeMeans.
  ///
  /// In en, this message translates to:
  /// **'joins the Note with another one.'**
  String get helpIconMergeMeans;

  /// No description provided for @helpIconMore.
  ///
  /// In en, this message translates to:
  /// **'More'**
  String get helpIconMore;

  /// No description provided for @helpIconMoreMeans.
  ///
  /// In en, this message translates to:
  /// **'the card\'s other actions, in Advanced also Make it a Tally.'**
  String get helpIconMoreMeans;

  /// The name of the help Note holding a release's Play notes (#180).
  ///
  /// In en, this message translates to:
  /// **'What\'s new in {version}'**
  String helpWhatsNewTitle(String version);

  /// The entry in the menu of an Area, a Topic and a Note that opens the Pack's tree (#164).
  ///
  /// In en, this message translates to:
  /// **'Export as Pack…'**
  String get exportAsPack;

  /// Title of the page where the user ticks what goes into a Pack (#164).
  ///
  /// In en, this message translates to:
  /// **'Export as Pack'**
  String get packExportTitle;

  /// Ticks every item of a Pack's tree.
  ///
  /// In en, this message translates to:
  /// **'Tick all'**
  String get packTickAll;

  /// Unticks every item of a Pack's tree.
  ///
  /// In en, this message translates to:
  /// **'Untick all'**
  String get packUntickAll;

  /// The Pack's size with what is ticked now, shown live.
  ///
  /// In en, this message translates to:
  /// **'Size: {size}'**
  String packSize(String size);

  /// Tick on the export page: the Pack carries the sender's Profile card and is signed with it (Q462).
  ///
  /// In en, this message translates to:
  /// **'Add my Profile'**
  String get packAddProfile;

  /// Under 'Add my Profile'.
  ///
  /// In en, this message translates to:
  /// **'The receiver sees your name and picture, and can tell nobody changed the Pack.'**
  String get packAddProfileHint;

  /// The button on the export page that writes the Pack and opens the share sheet.
  ///
  /// In en, this message translates to:
  /// **'Share…'**
  String get packShare;

  /// Under the export and import pages' button while no Note is ticked.
  ///
  /// In en, this message translates to:
  /// **'Tick at least one Note.'**
  String get packNothingTicked;

  /// A photo in a Pack's tree, with its size.
  ///
  /// In en, this message translates to:
  /// **'Photo · {size}'**
  String packPhoto(String size);

  /// A voice recording in a Pack's tree, with its size.
  ///
  /// In en, this message translates to:
  /// **'Recording · {size}'**
  String packRecording(String size);

  /// A Photo Text in a Pack's tree, with the start of its text.
  ///
  /// In en, this message translates to:
  /// **'Photo Text: {text}'**
  String packPhotoText(String text);

  /// A Transcript in a Pack's tree, with the start of its text.
  ///
  /// In en, this message translates to:
  /// **'Transcript: {text}'**
  String packTranscript(String text);

  /// A Routine in a Pack's tree, with its schedule.
  ///
  /// In en, this message translates to:
  /// **'Routine: {when}'**
  String packRoutine(String when);

  /// A Reminder in a Pack's tree, with its day and time.
  ///
  /// In en, this message translates to:
  /// **'Reminder: {when}'**
  String packReminder(String when);

  /// A Note's open Frog in a Pack's tree.
  ///
  /// In en, this message translates to:
  /// **'Frog'**
  String get packFrog;

  /// Title of the page showing a Pack that was opened (#164).
  ///
  /// In en, this message translates to:
  /// **'Pack'**
  String get packTitle;

  /// Who a Pack is from, and the badge of a Note added from it without 'Claim as mine' (Q463).
  ///
  /// In en, this message translates to:
  /// **'From {name}'**
  String packFrom(String name);

  /// On an opened Pack with a Profile: keeps the person as a Profile card would.
  ///
  /// In en, this message translates to:
  /// **'Add {name} to People'**
  String packAddPerson(String name);

  /// Tick beside 'Add' on a Pack with a Profile (Q463), on by default.
  ///
  /// In en, this message translates to:
  /// **'Claim as mine'**
  String get packClaim;

  /// Under 'Claim as mine'.
  ///
  /// In en, this message translates to:
  /// **'Unticked, each Note shows who it came from until you remove it.'**
  String get packClaimHint;

  /// The button that adds the ticked Notes of a Pack.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get packAdd;

  /// Announced after 'Add' on a Pack.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 Note added} other{{count} Notes added}}'**
  String packAdded(int count);

  /// Put after the name of a new Area from a Pack when an Area has that name: 'Health (Pack)'.
  ///
  /// In en, this message translates to:
  /// **'Pack'**
  String get packSuffix;

  /// The name of the new Area for the Notes of a Pack that were in no Area.
  ///
  /// In en, this message translates to:
  /// **'From a Pack'**
  String get packNoArea;

  /// Announced when a Pack could not be written or no app took it.
  ///
  /// In en, this message translates to:
  /// **'The Pack was not shared.'**
  String get packNotSent;

  /// Title of the dialog behind 'packNotSent'.
  ///
  /// In en, this message translates to:
  /// **'Pack not shared'**
  String get packNotSentTitle;

  /// Why a Pack was not shared: its file could not be written.
  ///
  /// In en, this message translates to:
  /// **'The file for the Pack could not be written.'**
  String get packNotWrittenCause;

  /// Announced when a Pack is refused; the tap shows why.
  ///
  /// In en, this message translates to:
  /// **'The Pack was not opened.'**
  String get packNotOpened;

  /// Title of the dialog behind 'packNotOpened'.
  ///
  /// In en, this message translates to:
  /// **'Pack not opened'**
  String get packNotOpenedTitle;

  /// Why a Pack was not opened: it is damaged, or its signature does not hold.
  ///
  /// In en, this message translates to:
  /// **'This file is no Pack, or it was changed after it was made.'**
  String get packUnreadable;

  /// Why a Pack was not opened (Q471).
  ///
  /// In en, this message translates to:
  /// **'This Pack is from a person you blocked.'**
  String get packFromBlocked;

  /// The fix when a Pack could not be read.
  ///
  /// In en, this message translates to:
  /// **'Ask the person to send it again.'**
  String get packAskAgainFix;

  /// In a Note's menu: removes the badge saying who the Note came from in a Pack.
  ///
  /// In en, this message translates to:
  /// **'Remove “From {name}”'**
  String removePackFrom(String name);

  /// The page in Settings that sends feedback or a bug report to the developer (#214), and its entry there.
  ///
  /// In en, this message translates to:
  /// **'Send feedback'**
  String get feedbackTitle;

  /// Under 'Send feedback' in Settings.
  ///
  /// In en, this message translates to:
  /// **'Tell the developer what happened or what could be better. You see everything before it is sent.'**
  String get feedbackSettingsDetail;

  /// The text field on the Send feedback page.
  ///
  /// In en, this message translates to:
  /// **'What happened / what could be better'**
  String get feedbackTextLabel;

  /// Heading over the ticks of what the feedback carries besides the text.
  ///
  /// In en, this message translates to:
  /// **'What goes along'**
  String get feedbackGoesAlong;

  /// Tick: the app's version goes along, its value under it.
  ///
  /// In en, this message translates to:
  /// **'Version'**
  String get feedbackVersion;

  /// Tick: the operating system and the device go along, their value under it.
  ///
  /// In en, this message translates to:
  /// **'OS and device'**
  String get feedbackSystem;

  /// Tick: where the app was installed from goes along (Play, F-Droid, APK, Flatpak, Homebrew, Scoop…), its value under it.
  ///
  /// In en, this message translates to:
  /// **'Install method'**
  String get feedbackInstall;

  /// Tick, off at first: the report of the app's last unexpected stop goes along.
  ///
  /// In en, this message translates to:
  /// **'The last crash report'**
  String get feedbackCrash;

  /// Under 'The last crash report' when there is none; the tick is greyed.
  ///
  /// In en, this message translates to:
  /// **'This phone keeps none.'**
  String get feedbackCrashNone;

  /// Under 'The last crash report': when it was written.
  ///
  /// In en, this message translates to:
  /// **'From {date}'**
  String feedbackCrashFrom(String date);

  /// Tick, off at first: copies of Notes picked in the Pack's tree go along as a Pack file.
  ///
  /// In en, this message translates to:
  /// **'Notes as a Pack'**
  String get feedbackNotes;

  /// Under 'Notes as a Pack' while no Note is picked.
  ///
  /// In en, this message translates to:
  /// **'Pick them in the Pack\'s tree.'**
  String get feedbackNotesHint;

  /// Under 'Notes as a Pack': how many Notes are picked and the Pack's size.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 Note} other{{count} Notes}}, {size}'**
  String feedbackNotesPicked(int count, String size);

  /// Button under 'Notes as a Pack' that opens the Pack's tree again.
  ///
  /// In en, this message translates to:
  /// **'Pick the Notes…'**
  String get feedbackPickNotes;

  /// Title of the Pack's tree opened from Send feedback.
  ///
  /// In en, this message translates to:
  /// **'Notes for the feedback'**
  String get feedbackNotesTitle;

  /// Heading over the text the feedback sends, as it is sent.
  ///
  /// In en, this message translates to:
  /// **'Preview: exactly this is sent'**
  String get feedbackPreview;

  /// Under the preview while Notes go along.
  ///
  /// In en, this message translates to:
  /// **'With it, as a file: a Pack of {notes}'**
  String feedbackPackAlong(String notes);

  /// Button on the Send feedback page: opens the mail app with the text to the developer.
  ///
  /// In en, this message translates to:
  /// **'Mail'**
  String get feedbackMail;

  /// Button on the Send feedback page: hands the text and the Pack to the app picked in the share sheet.
  ///
  /// In en, this message translates to:
  /// **'Share…'**
  String get feedbackShare;

  /// Under the buttons of Send feedback while the text field is empty.
  ///
  /// In en, this message translates to:
  /// **'Write what happened first.'**
  String get feedbackTextFirst;

  /// Under the buttons of Send feedback on the phone when the text is longer than a mail link carries.
  ///
  /// In en, this message translates to:
  /// **'Too long for a mail link. Share sends all of it.'**
  String get feedbackTooLongForMail;

  /// Under the buttons of Send feedback on the phone while Notes go along.
  ///
  /// In en, this message translates to:
  /// **'A mail link cannot carry the Pack. Share sends it along: pick your mail app there.'**
  String get feedbackPackNotInMail;

  /// Announced when the feedback's Pack could not be written or no app took it.
  ///
  /// In en, this message translates to:
  /// **'The feedback was not shared.'**
  String get feedbackNotSent;

  /// Title of the dialog behind 'feedbackNotSent'.
  ///
  /// In en, this message translates to:
  /// **'Feedback not shared'**
  String get feedbackNotSentTitle;

  /// At the end of an announcement whose tap undoes what was just done (#268), and the screen reader's hint for that tap.
  ///
  /// In en, this message translates to:
  /// **'Undo'**
  String get undo;

  /// Announced after a Note was moved to the Trash without asking; its tap brings it back with its Routines and Reminders.
  ///
  /// In en, this message translates to:
  /// **'Moved to the Trash'**
  String get undoNoteTrashed;

  /// Announced after a Line of a Checklist or Shopping List was deleted without asking; its tap restores it.
  ///
  /// In en, this message translates to:
  /// **'Line deleted'**
  String get undoLineDeleted;

  /// Announced after a Tally entry was deleted without asking; its tap restores it.
  ///
  /// In en, this message translates to:
  /// **'Entry deleted'**
  String get undoEntryDeleted;

  /// Announced after a Frog was moved to the Trash without asking; its tap restores it.
  ///
  /// In en, this message translates to:
  /// **'Frog moved to the Trash'**
  String get undoFrogTrashed;

  /// Announced after a Routine was moved to the Trash without asking; its tap restores it.
  ///
  /// In en, this message translates to:
  /// **'Routine moved to the Trash'**
  String get undoRoutineTrashed;

  /// Announced after a Reminder was moved to the Trash without asking; its tap restores it.
  ///
  /// In en, this message translates to:
  /// **'Reminder moved to the Trash'**
  String get undoReminderTrashed;

  /// Announced after a Note's “From …” label was removed without asking; its tap brings it back. No name: values never go into an announcement.
  ///
  /// In en, this message translates to:
  /// **'“From” label removed'**
  String get undoPackFromRemoved;

  /// Announced after a Transcript was removed without asking; its tap brings it back.
  ///
  /// In en, this message translates to:
  /// **'Transcript removed'**
  String get undoTranscriptRemoved;

  /// Announced after a Photo Text was removed without asking; its tap brings it back.
  ///
  /// In en, this message translates to:
  /// **'Photo Text removed'**
  String get undoPhotoTextRemoved;

  /// Announced after Not today ▸ Skip in the app (#269); its tap takes the skip back.
  ///
  /// In en, this message translates to:
  /// **'Skipped'**
  String get undoSkipped;

  /// Announced after Not today ▸ Tomorrow in the app (#269); its tap takes it back, and the Occurrence added tomorrow goes.
  ///
  /// In en, this message translates to:
  /// **'The day after instead'**
  String get undoTomorrow;

  /// Folded heading under a Checklist's or Shopping List's Lines in the Note editor: the deleted Lines, each with Restore.
  ///
  /// In en, this message translates to:
  /// **'Removed Lines'**
  String get linesRemoved;

  /// Bar action of a Note list on the phone (#273): shows a box on every card this Device may change, to act on several Notes at once.
  ///
  /// In en, this message translates to:
  /// **'Select'**
  String get selectNotes;

  /// Screen reader label of the box on a Note card while its list selects (#273).
  ///
  /// In en, this message translates to:
  /// **'Select this Note'**
  String get selectNote;

  /// First bar button while a Note list selects (#273): drops the selection and hides the boxes.
  ///
  /// In en, this message translates to:
  /// **'Stop selecting'**
  String get stopSelecting;

  /// Title of a Note list while it selects (#273): how many Notes are selected.
  ///
  /// In en, this message translates to:
  /// **'{count} selected'**
  String notesSelected(int count);

  /// Title of the dialog asked when an edit page that cannot be saved as it is is left by Back.
  ///
  /// In en, this message translates to:
  /// **'Discard changes?'**
  String get leaveDiscardTitle;

  /// No description provided for @leaveDiscardBody.
  ///
  /// In en, this message translates to:
  /// **'It can\'t be saved as it is, so leaving drops it.'**
  String get leaveDiscardBody;

  /// Button of the leave dialog that closes the page without saving.
  ///
  /// In en, this message translates to:
  /// **'Discard'**
  String get leaveDiscard;

  /// Button of the leave dialog that stays on the page.
  ///
  /// In en, this message translates to:
  /// **'Keep editing'**
  String get leaveKeepEditing;

  /// Title of the card on the Backups page that opens a Backup to take Notes back from it (#285).
  ///
  /// In en, this message translates to:
  /// **'Take Notes back'**
  String get backupOpenTitle;

  /// Text of the card 'backupOpenTitle' on the Backups page.
  ///
  /// In en, this message translates to:
  /// **'Open a Backup to bring Notes back from it, such as a Note deleted for good or an earlier version of a list. They come back as copies in new Areas; nothing else changes.'**
  String get backupOpenIntro;

  /// Button on the Backups page that picks a Backup file to take Notes back from it.
  ///
  /// In en, this message translates to:
  /// **'Open a Backup…'**
  String get backupOpen;

  /// Title of the page that asks for the passphrase or recovery key of a Backup opened to take Notes back.
  ///
  /// In en, this message translates to:
  /// **'Open a Backup'**
  String get backupOpenPageTitle;

  /// Text over the passphrase and recovery key fields when a Backup is opened to take Notes back.
  ///
  /// In en, this message translates to:
  /// **'Open it with your passphrase or with the recovery key from the Recovery Sheet. Nothing is restored: you pick the Notes to take back.'**
  String get backupOpenUnlockIntro;

  /// Button that opens the Backup with the passphrase or recovery key typed.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get backupOpenStart;

  /// Shown beside a spinner while a Backup is opened.
  ///
  /// In en, this message translates to:
  /// **'Opening… this takes a moment.'**
  String get backupOpenWorking;

  /// Announced when a Backup could not be opened to take Notes back; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The Backup was not opened.'**
  String get backupNotOpened;

  /// Title of the dialog behind 'backupNotOpened'.
  ///
  /// In en, this message translates to:
  /// **'Backup not opened'**
  String get backupNotOpenedTitle;

  /// The fix when a Backup opened to take Notes back is damaged.
  ///
  /// In en, this message translates to:
  /// **'Nothing was changed; try an older Backup.'**
  String get backupOpenDamagedFix;

  /// Text over the tree of a Backup's Notes.
  ///
  /// In en, this message translates to:
  /// **'Tick the Notes to take back. Each comes back as a copy in a new Area, with its files; the Notes on this Device stay as they are.'**
  String get backupNotesHint;

  /// Over the tree of a Backup's Notes when the Notes of locked Areas that are closed now are not shown.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{The Notes of a locked Area are left out until you unlock it.} other{The Notes of {count} locked Areas are left out until you unlock them.}}'**
  String backupNotesLocked(int count);

  /// Button under 'backupNotesLocked': asks for the PIN, opens those locked Areas and shows their Notes in the tree.
  ///
  /// In en, this message translates to:
  /// **'Unlock and show them'**
  String get backupNotesUnlock;

  /// Title of the PIN or biometrics request behind 'backupNotesUnlock'.
  ///
  /// In en, this message translates to:
  /// **'Show the Notes of locked Areas'**
  String get backupNotesUnlockTitle;

  /// Under a Note of a Backup's tree that lay in the Trash when the Backup was made.
  ///
  /// In en, this message translates to:
  /// **'In the Trash'**
  String get backupNoteTrashed;

  /// Put after the name of a new Area from a Backup when an Area has that name: 'Health (Backup)'.
  ///
  /// In en, this message translates to:
  /// **'Backup'**
  String get backupSuffix;

  /// The name of the new Area for the Notes of a Backup that were in no Area.
  ///
  /// In en, this message translates to:
  /// **'From a Backup'**
  String get backupNoArea;

  /// Announced when Add on a Backup's Notes failed; the tap shows why and what to do.
  ///
  /// In en, this message translates to:
  /// **'The Notes were not added.'**
  String get backupNotesNotAdded;

  /// Title of the dialog behind 'backupNotesNotAdded'.
  ///
  /// In en, this message translates to:
  /// **'Notes not added'**
  String get backupNotesNotAddedTitle;

  /// Why the Notes of a Backup were not added.
  ///
  /// In en, this message translates to:
  /// **'The Backup file changed or could not be read again since it was opened.'**
  String get backupAddDamagedCause;

  /// The fix when the Notes of a Backup were not added.
  ///
  /// In en, this message translates to:
  /// **'Nothing was added; open the Backup again.'**
  String get backupAddDamagedFix;

  /// Row below the Areas in the Areas list, and the title of the page it opens: every tag (#word) of the Notes by its levels (#267).
  ///
  /// In en, this message translates to:
  /// **'Tags'**
  String get tagsTitle;

  /// The tag tree page while no Note has a tag.
  ///
  /// In en, this message translates to:
  /// **'No tags yet. A word with # in front in a Note, such as #garden or #life/health, shows here.'**
  String get tagsNone;

  /// How many Notes carry a tag of the tag tree, or a tag below it.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 Note} other{{count} Notes}}'**
  String tagNotes(int count);

  /// Button beside a tag in the tag tree: shows the tags nested below it, such as #life/health below #life.
  ///
  /// In en, this message translates to:
  /// **'Show the tags below'**
  String get tagUnfold;

  /// Button beside a tag in the tag tree whose tags below show: hides them.
  ///
  /// In en, this message translates to:
  /// **'Hide the tags below'**
  String get tagFold;

  /// The mark of a Note or an Area that hides it and everything on it from the AI on the user's computer (#267, ADR 0049): a switch in the card's and the Area's menu and a badge in the Note editor; also read by a screen reader for the shield beside a hidden Note or Area.
  ///
  /// In en, this message translates to:
  /// **'Hidden from AI'**
  String get hiddenFromAi;

  /// Value of the Note editor's badge Hidden from AI while the Note has no mark of its own but its Area, or the Note it is merged into, is hidden from AI.
  ///
  /// In en, this message translates to:
  /// **'Off, hidden by its Area'**
  String get hiddenFromAiByArea;

  /// Title of the question shown when something hidden from AI is about to be given to other people: filed into a Shared Area, shared, sent or put into a Pack.
  ///
  /// In en, this message translates to:
  /// **'Hidden from AI'**
  String get hiddenFromAiRefusedTitle;

  /// Text of that question; names are the Notes (their first line) and Areas whose mark is in the way.
  ///
  /// In en, this message translates to:
  /// **'{names}: hidden from AI. What is hidden from AI is not given to other people. Show it to the AI first.'**
  String hiddenFromAiRefusedBody(String names);

  /// Button of that question: takes the marks Hidden from AI off and goes on with filing, sharing, sending or the Pack.
  ///
  /// In en, this message translates to:
  /// **'Show to the AI'**
  String get hiddenFromAiShow;

  /// Capture button and entry of the Agenda's "+" fan: picks files of any kind with the system's file picker and attaches them to the Note (#267).
  ///
  /// In en, this message translates to:
  /// **'File'**
  String get file;

  /// Action in the Note editor's bar: picks files with the system's file picker and attaches them to the Note.
  ///
  /// In en, this message translates to:
  /// **'Attach a file'**
  String get attachFile;

  /// Label over the text read out of an attached file, shown dimmed so it does not read as typed text (like Photo Text).
  ///
  /// In en, this message translates to:
  /// **'Attachment Text'**
  String get attachmentTextLabel;

  /// Under an attached file whose text the phone leaves to the user's computer (a PDF of more than 10 pages, or a large file): the computer reads it once it has the file, and the text syncs back.
  ///
  /// In en, this message translates to:
  /// **'The text follows from the computer.'**
  String get attachmentTextFromComputer;

  /// How many pages an attached PDF has, beside its size.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 page} other{{count} pages}}'**
  String attachmentPages(int count);

  /// Screen reader label of an attached file's row: a tap opens it with the system's app.
  ///
  /// In en, this message translates to:
  /// **'Open {name}'**
  String openFile(String name);

  /// Title of the one question before an Attachment of a kind that could run a program opens (#267).
  ///
  /// In en, this message translates to:
  /// **'Open this file?'**
  String get fileRiskTitle;

  /// The question before an Attachment of a kind that could run a program, such as an .exe or a script, opens on the computer (#267).
  ///
  /// In en, this message translates to:
  /// **'\"{file}\" could run a program. Open it anyway?'**
  String fileRiskQuestion(String file);

  /// The question before an Attachment that is an Android app (.apk) opens on the phone (#267).
  ///
  /// In en, this message translates to:
  /// **'\"{file}\" could install an app. Open it anyway?'**
  String fileRiskAppQuestion(String file);

  /// Under that question: the person who added the file, when another person did (#267).
  ///
  /// In en, this message translates to:
  /// **'{person} added it.'**
  String fileRiskAddedBy(String person);

  /// Button of that question that opens the file (#267).
  ///
  /// In en, this message translates to:
  /// **'Open anyway'**
  String get fileRiskOpen;

  /// Announcement at the top when an attached file could not be opened; its tap gives the reason.
  ///
  /// In en, this message translates to:
  /// **'An Attachment was not opened.'**
  String get fileNotOpened;

  /// Title of the details of that announcement.
  ///
  /// In en, this message translates to:
  /// **'Attachment not opened'**
  String get fileNotOpenedTitle;

  /// Why an attached file was not opened: its bytes have not synced to this Device yet.
  ///
  /// In en, this message translates to:
  /// **'The Attachment is not on this Device yet.'**
  String get fileNotHereCause;

  /// The fix for that.
  ///
  /// In en, this message translates to:
  /// **'It comes with the next sync; nothing is lost.'**
  String get fileNotHereFix;

  /// Why an attached file was not opened: no app took it.
  ///
  /// In en, this message translates to:
  /// **'No app on this Device opens files of this kind.'**
  String get fileNoAppCause;

  /// The fix for that.
  ///
  /// In en, this message translates to:
  /// **'Install an app for it, or open the Attachment on the computer.'**
  String get fileNoAppFix;

  /// Announcement at the top when a picked or shared file could not be attached; its tap gives the reason.
  ///
  /// In en, this message translates to:
  /// **'A file was not attached.'**
  String get fileNotAttached;

  /// Title of the details of that announcement.
  ///
  /// In en, this message translates to:
  /// **'File not attached'**
  String get fileNotAttachedTitle;

  /// Why a file was not attached: too large.
  ///
  /// In en, this message translates to:
  /// **'It is larger than 256 MB, the most a file can be.'**
  String get fileTooLargeCause;

  /// Why a file was not attached: its copy could not be read.
  ///
  /// In en, this message translates to:
  /// **'It could not be read on this Device.'**
  String get fileUnreadableCause;

  /// The fix for a file not attached.
  ///
  /// In en, this message translates to:
  /// **'Nothing was written; the file is still where it was.'**
  String get fileNotAttachedFix;

  /// Button under an attached file with an Attachment Text, and entry of the card's menu: removes the file while its text stays (ADR 0020).
  ///
  /// In en, this message translates to:
  /// **'Remove Attachment, keep text'**
  String get removeFileKeepText;

  /// The question before removing an attached file while its text stays.
  ///
  /// In en, this message translates to:
  /// **'Remove this Attachment and keep its Attachment Text?'**
  String get removeFileQuestion;

  /// Offer at the top to undo removing an Attachment Text (#268).
  ///
  /// In en, this message translates to:
  /// **'Attachment Text removed'**
  String get undoAttachmentTextRemoved;

  /// Announcement when reading an attached file's text again failed.
  ///
  /// In en, this message translates to:
  /// **'The text of an Attachment could not be read.'**
  String get fileReadAgainFailed;

  /// Title of its details.
  ///
  /// In en, this message translates to:
  /// **'Attachment not read'**
  String get fileReadAgainFailedTitle;

  /// Why the text of a file was not read again.
  ///
  /// In en, this message translates to:
  /// **'This Attachment could not be read on this Device.'**
  String get fileReadAgainCause;

  /// What a Note made from a file alone is called where only its words fit, as in a row or an alarm, when the file has no name.
  ///
  /// In en, this message translates to:
  /// **'Attachment'**
  String get attachment;

  /// When a Note to be filed into a Shared Area, sent or put into a Pack lies in an Area hidden from AI: names the Area and says where its mark comes off; nothing is taken off from here.
  ///
  /// In en, this message translates to:
  /// **'{names}: hidden from AI. A Note in an Area hidden from AI is not given to other people. Take the mark off in the Area\'s menu in the Areas list first.'**
  String hiddenFromAiAreaFirst(String names);

  /// Announcement at the top when a Note captured in an Area hidden from AI was to be filed into a Shared Area: it stays where it was; its tap gives the reason.
  ///
  /// In en, this message translates to:
  /// **'A Note was not filed.'**
  String get captureNotFiledHidden;

  /// Title of the details of that announcement.
  ///
  /// In en, this message translates to:
  /// **'Note not filed'**
  String get captureNotFiledHiddenTitle;

  /// Why the Note was not filed.
  ///
  /// In en, this message translates to:
  /// **'It is hidden from AI, and what is hidden from AI does not go into a Shared Area.'**
  String get captureNotFiledHiddenCause;

  /// The fix for that.
  ///
  /// In en, this message translates to:
  /// **'It stays where it was; take the mark off, then file it.'**
  String get captureNotFiledHiddenFix;

  /// Title of the phone's Ask page and its button in the Agenda's bar (#267): asks the user's own computer about the Notes.
  ///
  /// In en, this message translates to:
  /// **'Ask'**
  String get phoneAskTitle;

  /// Shown on the Ask page while the phone looks for the user's own computer and the provider it asks.
  ///
  /// In en, this message translates to:
  /// **'Looking for your computer…'**
  String get phoneAskLooking;

  /// Button on the Ask page that looks for the computer again.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get phoneAskTryAgain;

  /// Hint in the Ask page's question field.
  ///
  /// In en, this message translates to:
  /// **'Ask about your Notes'**
  String get phoneAskHint;

  /// Button that sends the question to the computer.
  ///
  /// In en, this message translates to:
  /// **'Ask'**
  String get phoneAskButton;

  /// Bar action of the Ask page that drops the questions asked, for a new one.
  ///
  /// In en, this message translates to:
  /// **'Start over'**
  String get phoneAskStartOver;

  /// Shown under the question while the computer asks its provider.
  ///
  /// In en, this message translates to:
  /// **'Waiting for the answer from {provider}…'**
  String phoneAskAsking(String provider);

  /// Line at the top of the Ask page naming the computer, its provider and model.
  ///
  /// In en, this message translates to:
  /// **'Questions go through {computer} to {provider}, model {model}. Only what the model finds and reads in your Notes is sent there, never what is Hidden from AI.'**
  String phoneAskSends(String computer, String provider, String model);

  /// Line on the Ask page when the answer says an AI Task waits for review on the desktop.
  ///
  /// In en, this message translates to:
  /// **'Changes the AI made wait for your review on the computer. Sync with it waits until you decide there.'**
  String get phoneAskTaskWaits;

  /// Heading over the Notes an answer links, each opening its Note.
  ///
  /// In en, this message translates to:
  /// **'From these Notes'**
  String get phoneAskFromNotes;

  /// Button that saves the question and its answer as a Note in Unsorted.
  ///
  /// In en, this message translates to:
  /// **'Keep as Note'**
  String get phoneAskKeep;

  /// Shown once the answer was kept; a tap opens the Note.
  ///
  /// In en, this message translates to:
  /// **'Kept as a Note'**
  String get phoneAskKept;

  /// Line under an answer the provider cut off.
  ///
  /// In en, this message translates to:
  /// **'The answer was cut off at its length limit.'**
  String get phoneAskCut;

  /// Why the computer cannot ask its provider yet.
  ///
  /// In en, this message translates to:
  /// **'{provider} is not set up on the computer yet.'**
  String phoneAskUnreadyCause(String provider);

  /// Fix when the computer's provider is not set up, for a reason this version does not know.
  ///
  /// In en, this message translates to:
  /// **'Set up the provider under Ask panel in the computer\'s Settings.'**
  String get phoneAskUnreadyFix;

  /// Fix: the computer's provider has no model.
  ///
  /// In en, this message translates to:
  /// **'Name a model under Ask panel in the computer\'s Settings.'**
  String get phoneAskUnreadyModel;

  /// Fix: the computer's Anthropic provider has no key.
  ///
  /// In en, this message translates to:
  /// **'Enter the API key under Ask panel in the computer\'s Settings.'**
  String get phoneAskUnreadyKey;

  /// Fix: the computer's provider address may not be asked.
  ///
  /// In en, this message translates to:
  /// **'Give the provider an https:// address under Ask panel in the computer\'s Settings; http:// only on the computer or the home network.'**
  String get phoneAskUnreadyUrl;

  /// Heading of the permission asked on the Ask page before the first question to a provider (App Store guideline 5.1.2(i)).
  ///
  /// In en, this message translates to:
  /// **'Before your first question'**
  String get phoneAskConsentTitle;

  /// Names the computer and the third-party AI provider it sends the questions to.
  ///
  /// In en, this message translates to:
  /// **'Your questions go from this phone to your computer {computer}, which sends them to {provider}, model {model}, at {host}.'**
  String phoneAskConsentIntro(
    String computer,
    String provider,
    String model,
    String host,
  );

  /// Heading over the list of what is sent to the provider.
  ///
  /// In en, this message translates to:
  /// **'What {provider} receives:'**
  String phoneAskConsentWhat(String provider);

  /// What is sent: the question.
  ///
  /// In en, this message translates to:
  /// **'Your question, and the questions and answers before it when you ask on.'**
  String get phoneAskConsentQuestion;

  /// What is sent: names.
  ///
  /// In en, this message translates to:
  /// **'The names of your Areas, Topics and tags.'**
  String get phoneAskConsentNames;

  /// What is sent: Routines and Reminders.
  ///
  /// In en, this message translates to:
  /// **'Your Routines and Reminders with their check-offs, when the model lists them.'**
  String get phoneAskConsentRoutines;

  /// What is sent: search hits.
  ///
  /// In en, this message translates to:
  /// **'The titles and short excerpts of the Notes its searches find.'**
  String get phoneAskConsentFound;

  /// What is sent: Notes read in full.
  ///
  /// In en, this message translates to:
  /// **'The Notes it reads in full, with their Sections, Transcripts, Photo Texts, Attachment Texts and Capture Context (when and where a Note was captured, the calendar event and the Bluetooth device).'**
  String get phoneAskConsentRead;

  /// What is never sent, and who receives nothing.
  ///
  /// In en, this message translates to:
  /// **'Nothing Hidden from AI is sent, not even as a count. {provider} handles what it receives under its own terms; Old Brain Fix and its developer receive nothing.'**
  String phoneAskConsentNever(String provider);

  /// Says that nothing goes without the permission, and when it is asked again.
  ///
  /// In en, this message translates to:
  /// **'Nothing is sent until you allow it. When the computer asks another provider or model, this phone asks you again.'**
  String get phoneAskConsentAgain;

  /// Button that allows questions to the provider named.
  ///
  /// In en, this message translates to:
  /// **'Allow'**
  String get phoneAskAllow;

  /// Announcement at the top when a question got no answer; its tap gives cause and fix.
  ///
  /// In en, this message translates to:
  /// **'The question was not answered.'**
  String get phoneAskFailed;

  /// Title of its details.
  ///
  /// In en, this message translates to:
  /// **'Not answered'**
  String get phoneAskFailedTitle;

  /// Why a question was not answered.
  ///
  /// In en, this message translates to:
  /// **'{provider} refused the computer\'s API key.'**
  String phoneAskKeyRefused(String provider);

  /// Fix for a refused key.
  ///
  /// In en, this message translates to:
  /// **'Check the API key under Ask panel in the computer\'s Settings.'**
  String get phoneAskKeyFix;

  /// Why a question was not answered.
  ///
  /// In en, this message translates to:
  /// **'{provider} answered the computer with an error.'**
  String phoneAskProviderFailed(String provider);

  /// Fix for an error of the provider.
  ///
  /// In en, this message translates to:
  /// **'Check the model\'s name in the computer\'s Settings, or ask again later.'**
  String get phoneAskProviderFix;

  /// Why a question was not answered.
  ///
  /// In en, this message translates to:
  /// **'No answer came from {provider} to the computer.'**
  String phoneAskNoReply(String provider);

  /// Fix for no reply from the provider.
  ///
  /// In en, this message translates to:
  /// **'Check the computer\'s network and the provider\'s address in its Settings, then ask again.'**
  String get phoneAskNoReplyFix;

  /// Why a question was not answered.
  ///
  /// In en, this message translates to:
  /// **'The answer from {provider} could not be read on the computer.'**
  String phoneAskUnreadable(String provider);

  /// Fix for an unreadable reply.
  ///
  /// In en, this message translates to:
  /// **'Check that the base URL in the computer\'s Settings leads to an OpenAI-compatible API.'**
  String get phoneAskUnreadableFix;

  /// Why a question was not answered.
  ///
  /// In en, this message translates to:
  /// **'The model searched too long without answering.'**
  String get phoneAskTooManyRounds;

  /// Fix for a search too long.
  ///
  /// In en, this message translates to:
  /// **'Ask a narrower question.'**
  String get phoneAskNarrowerFix;

  /// Why a question was not answered.
  ///
  /// In en, this message translates to:
  /// **'The model declined to answer this question.'**
  String get phoneAskRefused;

  /// Fix for a declined question.
  ///
  /// In en, this message translates to:
  /// **'Ask it in other words.'**
  String get phoneAskOtherWordsFix;

  /// Why a question was not answered.
  ///
  /// In en, this message translates to:
  /// **'The computer stopped waiting for the answer.'**
  String get phoneAskStopped;

  /// Fix: ask again.
  ///
  /// In en, this message translates to:
  /// **'Ask again.'**
  String get phoneAskAgainFix;

  /// Why a question was not answered: one at a time on the desktop.
  ///
  /// In en, this message translates to:
  /// **'The computer is answering another question.'**
  String get phoneAskBusy;

  /// Fix for a busy computer.
  ///
  /// In en, this message translates to:
  /// **'Ask again in a moment.'**
  String get phoneAskBusyFix;

  /// Why a question was not answered: a failure of a later desktop.
  ///
  /// In en, this message translates to:
  /// **'The computer answered in a way this version cannot read.'**
  String get phoneAskUnknown;

  /// Fix for that.
  ///
  /// In en, this message translates to:
  /// **'Install the latest Old Brain Fix on this phone.'**
  String get phoneAskUpdatePhoneFix;

  /// Why the phone cannot ask: no own paired Device that hosts.
  ///
  /// In en, this message translates to:
  /// **'No computer of yours is paired with this phone.'**
  String get phoneAskNoComputer;

  /// Fix for that.
  ///
  /// In en, this message translates to:
  /// **'Pair your computer under Profile ▸ Sync with another Device, then ask.'**
  String get phoneAskNoComputerFix;

  /// Why the phone cannot ask: no address of the computer answered.
  ///
  /// In en, this message translates to:
  /// **'No computer of yours answered.'**
  String get phoneAskUnreachable;

  /// Fix for that.
  ///
  /// In en, this message translates to:
  /// **'Start Old Brain Fix on the computer and join its network or your VPN; through Tailscale, give it an address by hand under Profile ▸ Sync with another Device.'**
  String get phoneAskUnreachableFix;

  /// Why the phone cannot ask: the pinned certificate did not match.
  ///
  /// In en, this message translates to:
  /// **'Another device answered at your computer\'s address.'**
  String get phoneAskWrongHost;

  /// Fix for that.
  ///
  /// In en, this message translates to:
  /// **'Check the address by hand under Profile ▸ Sync with another Device, or pair the computer again.'**
  String get phoneAskWrongHostFix;

  /// Why the phone cannot ask: the computer declined it.
  ///
  /// In en, this message translates to:
  /// **'Your computer does not let this phone ask.'**
  String get phoneAskDeclined;

  /// Fix for that.
  ///
  /// In en, this message translates to:
  /// **'On the computer, switch on Sync automatically for this phone under Sync.'**
  String get phoneAskDeclinedFix;

  /// Why the phone cannot ask: only phones or older desktops answered.
  ///
  /// In en, this message translates to:
  /// **'None of your paired Devices answers questions.'**
  String get phoneAskNotOffered;

  /// Fix for that.
  ///
  /// In en, this message translates to:
  /// **'Install the latest Old Brain Fix on your computer; a phone answers no questions.'**
  String get phoneAskNotOfferedFix;

  /// Why the phone cannot ask: a timeout.
  ///
  /// In en, this message translates to:
  /// **'Your computer did not answer in time.'**
  String get phoneAskTimedOut;

  /// Fix for that.
  ///
  /// In en, this message translates to:
  /// **'Check that the computer is on and awake, then ask again.'**
  String get phoneAskTimedOutFix;

  /// Why the phone cannot ask: anything else on the way.
  ///
  /// In en, this message translates to:
  /// **'The connection to your computer broke off.'**
  String get phoneAskBrokeOff;

  /// Field under an own paired Device on the Sync page (#267): an address such as a Tailscale name, tried when the local network does not find it.
  ///
  /// In en, this message translates to:
  /// **'Address by hand'**
  String get handAddressLabel;

  /// Helper line under that field.
  ///
  /// In en, this message translates to:
  /// **'For a VPN such as Tailscale: its name or address there, such as desk.tail1234.ts.net, with :port if needed. Tried when this network does not find it.'**
  String get handAddressHelp;

  /// Error under the field for text that is no address.
  ///
  /// In en, this message translates to:
  /// **'Not an address. Type a name or an IP address, with :port if needed.'**
  String get handAddressInvalid;

  /// Error under the field for a port that is none.
  ///
  /// In en, this message translates to:
  /// **'The port must be a number from 1 to 65535.'**
  String get handAddressBadPort;

  /// At the top of an attached PDF shown full screen (#286): the page shown and how many there are.
  ///
  /// In en, this message translates to:
  /// **'Page {page} of {pages}'**
  String pdfPageOf(int page, int pages);

  /// The entry in the ⋮ of an attached PDF shown full screen (#286) that opens it with the system's app for PDFs.
  ///
  /// In en, this message translates to:
  /// **'Open in another app'**
  String get openInOtherApp;
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
