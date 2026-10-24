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
  String get noNotesYet => 'Noch nichts erfasst.';

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

  @override
  String get noteTitle => 'Notiz';

  @override
  String get importance => 'Wichtigkeit';

  @override
  String get importanceUnimportant => 'Unwichtig';

  @override
  String get importanceNormal => 'Normal';

  @override
  String get importanceImportant => 'Wichtig';

  @override
  String get capturedHeading => 'Erfasst';

  @override
  String get amend => 'Zur Notiz hinzufügen';

  @override
  String get amendHint => 'Was gibt es Neues?';

  @override
  String get add => 'Hinzufügen';

  @override
  String get moveToTrash => 'In den Papierkorb';

  @override
  String get trashTitle => 'Papierkorb';

  @override
  String get trashEmpty => 'Der Papierkorb ist leer.';

  @override
  String get restore => 'Wiederherstellen';

  @override
  String goneIn(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Wird in $count Tagen endgültig gelöscht',
      one: 'Wird in 1 Tag endgültig gelöscht',
      zero: 'Wird innerhalb eines Tages endgültig gelöscht',
    );
    return '$_temp0';
  }

  @override
  String get trashThem => 'In den Papierkorb';

  @override
  String get orderByImportance => 'Nach Wichtigkeit ordnen';

  @override
  String get orderByTime => 'Nach Zeit ordnen';

  @override
  String get hideUnimportant => 'Unwichtige Notizen ausblenden';

  @override
  String get showUnimportant => 'Unwichtige Notizen einblenden';

  @override
  String get addHere => 'Hier hinzufügen';

  @override
  String get noteHere => 'Notiz hier';

  @override
  String get topicChoice => 'Thema';

  @override
  String get mergeInto => 'Zusammenführen mit …';

  @override
  String get noOtherNotes => 'Es gibt keine andere Notiz zum Zusammenführen.';

  @override
  String get locationExplainTitle => 'Merken, wo du warst?';

  @override
  String get locationExplainBody =>
      'Old Brain Fix kann notieren, wo du bist, wenn du eine Notiz erfasst – damit „mehr soße kaufn“ auch Tage später noch Sinn ergibt. Gib deinen Orten eigene Namen wie Zuhause oder Arbeit, dann steht bei einer Notiz „Zuhause“ statt Koordinaten. Dein Standort bleibt auf diesem Handy und wird nirgendwohin gesendet.';

  @override
  String get notNow => 'Jetzt nicht';

  @override
  String get allowLocation => 'Erlauben';

  @override
  String get locationNotAllowed =>
      'Old Brain Fix darf deinen Standort nicht verwenden und weiß daher nicht, wo du bist. Du kannst ihn in den Systemeinstellungen erlauben.';

  @override
  String get locationUnknown =>
      'Konnte nicht herausfinden, wo du bist. Ist der Standort eingeschaltet?';

  @override
  String get namedPlacesTitle => 'Benannte Orte';

  @override
  String get noNamedPlacesYet =>
      'Noch keine benannten Orte. Geh an einen Ort wie Zuhause oder Arbeit und tippe auf +, um ihn zu benennen.';

  @override
  String get newNamedPlace => 'Diesen Ort benennen';

  @override
  String get editNamedPlace => 'Umbenennen oder Größe ändern';

  @override
  String get namedPlaceHint => 'Name, z. B. Zuhause';

  @override
  String get radiusHeading => 'Zählt im Umkreis von';

  @override
  String radiusMetres(int metres) {
    return '$metres m';
  }

  @override
  String deleteNamedPlaceQuestion(String name) {
    return 'Den benannten Ort „$name“ löschen?';
  }

  @override
  String get deleteNamedPlaceDetail =>
      'Notizen, die dort erfasst wurden, zeigen wieder ihre Koordinaten.';

  @override
  String get agendaToday => 'Heute';

  @override
  String get agendaTomorrow => 'Morgen';

  @override
  String get nothingToday =>
      'Heute steht nichts an. Tippe auf +, um einen Gedanken festzuhalten.';

  @override
  String get nothingTomorrow => 'Morgen steht nichts an.';

  @override
  String get justCaptured => 'Gerade erfasst';

  @override
  String get allNotesTitle => 'Alle Notizen';

  @override
  String get newRoutineTooltip => 'Neue Routine oder Erinnerung';

  @override
  String get routineOrReminder => 'Routine oder Erinnerung';

  @override
  String get routine => 'Routine';

  @override
  String get reminder => 'Erinnerung';

  @override
  String get newRoutine => 'Neue Routine';

  @override
  String get newReminder => 'Neue Erinnerung';

  @override
  String get routineWhatHint => 'Was ist zu tun?';

  @override
  String get fromNote => 'Aus der Notiz';

  @override
  String get noteGone => 'Die Notiz gibt es nicht mehr';

  @override
  String get scheduleSimple => 'Einfach';

  @override
  String get scheduleAdvanced => 'Erweitert';

  @override
  String get repeatOnce => 'Einmal';

  @override
  String get repeatEvery => 'Alle …';

  @override
  String get repeatWeekdays => 'Wochentage';

  @override
  String get everyLabel => 'Alle';

  @override
  String get fewer => 'Weniger';

  @override
  String get more => 'Mehr';

  @override
  String unitDays(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Tage',
      one: 'Tag',
    );
    return '$_temp0';
  }

  @override
  String unitWeeks(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Wochen',
      one: 'Woche',
    );
    return '$_temp0';
  }

  @override
  String unitMonths(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Monate',
      one: 'Monat',
    );
    return '$_temp0';
  }

  @override
  String unitYears(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Jahre',
      one: 'Jahr',
    );
    return '$_temp0';
  }

  @override
  String get startsOn => 'Ab';

  @override
  String get onDay => 'Am';

  @override
  String get timesHeading => 'Uhrzeiten';

  @override
  String get allDay => 'Ganztägig';

  @override
  String get addTime => 'Uhrzeit hinzufügen';

  @override
  String removeTime(String time) {
    return '$time entfernen';
  }

  @override
  String get rruleLabel => 'Wiederholregel (RRULE)';

  @override
  String get rruleHelper =>
      'Leer lassen für einmal. Beispiel: FREQ=MONTHLY;BYDAY=1MO';

  @override
  String rruleInvalid(String detail) {
    return 'Keine Regel, die die App versteht: $detail';
  }

  @override
  String get nextOccurrences => 'Als Nächstes';

  @override
  String get neverDue => 'Das kommt nie vor.';

  @override
  String get deleteRoutineQuestion =>
      'Diese Routine löschen? Ihre Notiz bleibt.';

  @override
  String get deleteReminderQuestion =>
      'Diese Erinnerung löschen? Ihre Notiz bleibt.';

  @override
  String get routinesHeading => 'Routinen und Erinnerungen';

  @override
  String everyNDays(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Alle $count Tage',
      one: 'Jeden Tag',
    );
    return '$_temp0';
  }

  @override
  String everyNWeeks(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Alle $count Wochen',
      one: 'Jede Woche',
    );
    return '$_temp0';
  }

  @override
  String everyNMonths(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Alle $count Monate',
      one: 'Jeden Monat',
    );
    return '$_temp0';
  }

  @override
  String everyNYears(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Alle $count Jahre',
      one: 'Jedes Jahr',
    );
    return '$_temp0';
  }

  @override
  String onWeekdays(String days) {
    return 'Immer $days';
  }

  @override
  String onceOn(String date) {
    return 'Einmal, $date';
  }

  @override
  String ruleSummary(String rule) {
    return 'Regel: $rule';
  }

  @override
  String get markDone => 'Erledigt';

  @override
  String get notFiled => 'Nicht abgelegt';
}
