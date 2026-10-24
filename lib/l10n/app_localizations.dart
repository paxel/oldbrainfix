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
  /// **'Nothing captured yet. Tap + to capture a thought.'**
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
  /// **'Areas and Topics'**
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
