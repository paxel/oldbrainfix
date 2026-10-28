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
  String get capturePhotoHint => 'Ein paar Worte dazu (freiwillig)';

  @override
  String get photo => 'Foto';

  @override
  String get voice => 'Sprache';

  @override
  String get gallery => 'Galerie';

  @override
  String get text => 'Text';

  @override
  String get captureFanClose => 'Schließen';

  @override
  String get photoMissing => 'Foto nicht auf diesem Gerät';

  @override
  String morePhotos(int count) {
    return '+$count';
  }

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
  String agendaDay(DateTime day) {
    final intl.DateFormat dayDateFormat = intl.DateFormat(
      'EEEE, d. MMM',
      localeName,
    );
    final String dayString = dayDateFormat.format(day);

    return '$dayString';
  }

  @override
  String get nothingPlanned => 'Nichts geplant';

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
  String get repeatMonthDay => 'Monatlich an einem Tag';

  @override
  String get repeatMonthWeekday => 'Monatlich an einem Wochentag';

  @override
  String get dayOfMonthLabel => 'Tag im Monat';

  @override
  String get monthDaySkipped => 'Monate ohne diesen Tag werden übersprungen.';

  @override
  String get whichInMonthLabel => 'Welcher im Monat';

  @override
  String nthOption(String which) {
    String _temp0 = intl.Intl.selectLogic(which, {
      '1': 'Erster',
      '2': 'Zweiter',
      '3': 'Dritter',
      '4': 'Vierter',
      'other': 'Letzter',
    });
    return '$_temp0';
  }

  @override
  String get weekdayLabel => 'Wochentag';

  @override
  String get endsLabel => 'Endet';

  @override
  String get endNever => 'Nie';

  @override
  String get endOnDay => 'An einem Tag';

  @override
  String get endAfterCount => 'Nach einer bestimmten Anzahl';

  @override
  String get endLastDay => 'Letzter Tag';

  @override
  String get endCountLabel => 'Wie oft';

  @override
  String get endCountExplain => 'Jede Uhrzeit zählt einzeln.';

  @override
  String unitTimes(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Mal',
      one: 'Mal',
    );
    return '$_temp0';
  }

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
      'Diese Routine löschen? Sie bleibt 30 Tage im Papierkorb. Ihre Notiz bleibt.';

  @override
  String get deleteReminderQuestion =>
      'Diese Erinnerung löschen? Sie bleibt 30 Tage im Papierkorb. Ihre Notiz bleibt.';

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
  String everyMonthOnDay(int count, int day) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Alle $count Monate am $day.',
      one: 'Jeden Monat am $day.',
    );
    return '$_temp0';
  }

  @override
  String everyMonthOnNth(int count, String nth, String weekday) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Alle $count Monate am $nth $weekday',
      one: 'Jeden Monat am $nth $weekday',
    );
    return '$_temp0';
  }

  @override
  String nthWord(String which) {
    String _temp0 = intl.Intl.selectLogic(which, {
      '1': 'ersten',
      '2': 'zweiten',
      '3': 'dritten',
      '4': 'vierten',
      'other': 'letzten',
    });
    return '$_temp0';
  }

  @override
  String repeatUntil(String repeat, String date) {
    return '$repeat, bis $date';
  }

  @override
  String repeatCount(int count, String repeat) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$repeat, $count-mal',
      one: '$repeat, einmal',
    );
    return '$_temp0';
  }

  @override
  String get markDone => 'Erledigt';

  @override
  String get notFiled => 'Nicht abgelegt';

  @override
  String get alarmChannel => 'Routinen und Erinnerungen';

  @override
  String get snooze => 'Später';

  @override
  String get snoozeQuestion => 'Erinnere mich wieder in …';

  @override
  String get snooze10 => '10 Min.';

  @override
  String get snooze30 => '30 Min.';

  @override
  String get snooze60 => '1 Std.';

  @override
  String get alarmsBanner =>
      'Alarme kommen vielleicht zu spät oder gar nicht: Eine Erlaubnis fehlt. Tippe hier, um das zu beheben.';

  @override
  String get setupTitle => 'Alarme einrichten';

  @override
  String setupStep(int step, int count) {
    return 'Schritt $step von $count';
  }

  @override
  String get setupAllow => 'Erlauben';

  @override
  String get setupGranted => 'Erlaubt';

  @override
  String get setupNext => 'Weiter';

  @override
  String get setupDone => 'Fertig';

  @override
  String get permNotifications => 'Benachrichtigungen';

  @override
  String get permNotificationsWhy =>
      'Routinen und Erinnerungen erscheinen als Benachrichtigung. Dort hakst du sie als erledigt ab oder verschiebst sie auf später – auch auf dem Sperrbildschirm.';

  @override
  String get permExactAlarms => 'Alarme auf die Minute';

  @override
  String get permExactAlarmsWhy =>
      'Ohne diese Erlaubnis kann Android einen Alarm bis zu einer Viertelstunde zu spät bringen. Schalte sie auf der Seite, die sich öffnet, für Old Brain Fix ein.';

  @override
  String get permFullScreen => 'Vollbild-Alarme';

  @override
  String get permFullScreenWhy =>
      'Damit kann eine Routine, die du immer wieder übergehst, zu einem Vollbild-Alarm werden, den du nicht übersiehst – wenn du das für sie einschaltest. Schalte es auf der Seite, die sich öffnet, für Old Brain Fix ein.';

  @override
  String get catchUpTitle => 'Verpasst, während das Handy aus war';

  @override
  String get catchUpOne =>
      'Ein Alarm wurde verpasst. Öffne die App, um ihn zu sehen.';

  @override
  String catchUpMany(String count) {
    return '$count Alarme wurden verpasst. Öffne die App, um sie zu sehen.';
  }

  @override
  String get missedTitle => 'Verpasst';

  @override
  String missedTimes(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count-mal',
      one: 'einmal',
    );
    return '$_temp0';
  }

  @override
  String missedMore(int count) {
    return 'und $count weitere';
  }

  @override
  String get missedSeen => 'Gesehen';

  @override
  String get renagHeading => 'Erneut erinnern';

  @override
  String renagSummary(int minutes) {
    return 'Klingelt alle $minutes Min. wieder, bis es erledigt ist';
  }

  @override
  String get renagExplain =>
      'Ein unbeachteter Alarm klingelt in diesem Abstand wieder, bis er erledigt oder verschoben ist oder der nächste fällig wird; dann gilt er als verpasst. Eine Erinnerung hört um Mitternacht auf.';

  @override
  String get renagThis => 'Für diese';

  @override
  String get renagDevice => 'Standard auf diesem Handy';

  @override
  String renagDeviceDefault(int minutes) {
    return 'Standard ($minutes Min.)';
  }

  @override
  String renagMinutes(int minutes) {
    return '$minutes Min.';
  }

  @override
  String get ringChannel => 'Vollbild-Alarme';

  @override
  String get escalatedChannel => 'Eskalierte Alarme';

  @override
  String get escalationHeading => 'Eskalation';

  @override
  String get escalationOff => 'Aus: klingelt nur wieder';

  @override
  String escalationSummary(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Vollbild-Alarm, nachdem es $count-mal unbeachtet wieder geklingelt hat',
      one: 'Vollbild-Alarm, nachdem es einmal unbeachtet wieder geklingelt hat',
    );
    return '$_temp0';
  }

  @override
  String get escalationExplain =>
      'Wird diese immer wieder übergangen, wird sie zu einem Vollbild-Alarm mit Ton, auch über dem Sperrbildschirm. Er klingelt bis zu 5 Minuten lang und klingelt danach in gewohntem Abstand wieder, bis sie erledigt oder verschoben ist.';

  @override
  String get escalationSwitch => 'Zum Vollbild-Alarm eskalieren';

  @override
  String get escalationAfter => 'Nach so oft unbeachtetem Wiederklingeln';

  @override
  String get nudgeHeading => 'Erinnere mich ans Einsortieren in';

  @override
  String unsortedBanner(int count) {
    return '$count unsortiert';
  }

  @override
  String get noUnsorted => 'Nichts ist unsortiert. Gut gemacht!';

  @override
  String get nudgeScreenTitle => 'Warten aufs Einsortieren';

  @override
  String nudgeMore(int count) {
    return '+$count';
  }

  @override
  String get nudgeMoreTooltip => 'Alle unsortierten Notizen';

  @override
  String get nudgeChannel => 'Stupser';

  @override
  String nudgeTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Notizen sind unsortiert',
      one: '1 Notiz ist unsortiert',
    );
    return '$_temp0';
  }

  @override
  String get nudgeBody => 'Tippe, um sie einzusortieren.';

  @override
  String get nudgeSettingsExplain =>
      'Solange etwas unsortiert ist, kommt ein unbeachteter Stupser nach dieser Zeit wieder. In der Ruhezeit warten Stupser, bis sie endet; Routinen und Erinnerungen klingeln trotzdem pünktlich.';

  @override
  String get nudgeRepeat => 'Erneut stupsen nach';

  @override
  String get quietHours => 'Ruhezeit';

  @override
  String get quietFrom => 'Von';

  @override
  String get quietUntil => 'Bis';

  @override
  String get yes => 'Ja';

  @override
  String get no => 'Nein';

  @override
  String get deleteNoteLinkedDetail =>
      'Mitgelöschte kommen mit der Notiz zurück, wenn du sie wiederherstellst. Behaltene erinnern dich weiter.';

  @override
  String get noteInTrash => 'Die Notiz liegt im Papierkorb';

  @override
  String get noteDeleted => 'Die Notiz wurde gelöscht';

  @override
  String get allowAccess => 'Erlauben';

  @override
  String get captureContextTitle => 'Erfassungskontext';

  @override
  String get captureContextIntro =>
      'Außer wann und wo kann eine Notiz noch mehr darüber festhalten, was gerade los war. Alles hier ist aus, bis du es einschaltest, und bleibt in der Notiz auf deinen eigenen Geräten.';

  @override
  String get calendarContextSwitch => 'Kalendertermin';

  @override
  String get calendarContextDetail =>
      'Der Termin, der läuft, wenn du eine Notiz erfasst, etwa „während: Zahnarzt“.';

  @override
  String get bluetoothContextSwitch => 'Bluetooth-Gerät';

  @override
  String get bluetoothContextDetail =>
      'Das Gerät, das verbunden ist, wenn du eine Notiz erfasst, etwa dein Auto oder deine Kopfhörer.';

  @override
  String get calendarExplainTitle => 'Deinen Termin merken?';

  @override
  String get calendarExplainBody =>
      'Old Brain Fix kann notieren, welcher Kalendertermin gerade läuft, wenn du eine Notiz erfasst – dann steht dort auch Tage später noch „während: Zahnarzt“. Dein Kalender wird nur in diesem Moment gelesen. Der Termin bleibt in der Notiz auf deinen eigenen Geräten und wird nirgendwohin sonst gesendet.';

  @override
  String get bluetoothExplainTitle => 'Merken, was verbunden war?';

  @override
  String get bluetoothExplainBody =>
      'Old Brain Fix kann notieren, welches Bluetooth-Gerät verbunden ist, wenn du eine Notiz erfasst, etwa dein Auto oder deine Kopfhörer. Android nennt das „Geräte in der Nähe“; Old Brain Fix liest nur die Namen verbundener Geräte und sucht nie nach anderen. Der Name bleibt in der Notiz auf deinen eigenen Geräten und wird nirgendwohin sonst gesendet.';

  @override
  String get calendarNotAllowed =>
      'Old Brain Fix darf deinen Kalender nicht lesen. Du kannst es in den Systemeinstellungen erlauben.';

  @override
  String get bluetoothNotAllowed =>
      'Old Brain Fix darf deine Bluetooth-Geräte nicht sehen. Du kannst es in den Systemeinstellungen erlauben.';

  @override
  String contextSourceApp(String app) {
    return 'Geteilt aus $app';
  }

  @override
  String contextDuring(String events) {
    return 'Während: $events';
  }

  @override
  String contextBluetooth(String devices) {
    return 'Verbunden: $devices';
  }

  @override
  String get moreCaptureContext => 'Mehr zur Erfassung';

  @override
  String get recordVoice => 'Sprache aufnehmen';

  @override
  String get recording => 'Aufnahme läuft …';

  @override
  String get stopAndSave => 'Beenden und speichern';

  @override
  String get voiceRecording => 'Sprachaufnahme';

  @override
  String get playRecording => 'Sprachaufnahme abspielen';

  @override
  String get stopPlaying => 'Anhalten';

  @override
  String get microphoneExplainTitle => 'Mikrofon verwenden?';

  @override
  String get microphoneExplainBody =>
      'Sprachnotizen nehmen auf, was du sagst – so bleibt ein Gedanke erhalten, bevor er weg ist. Old Brain Fix nutzt das Mikrofon nur, während du aufnimmst. Die Aufnahme bleibt in der Notiz auf deinen eigenen Geräten und wird nirgendwohin sonst gesendet.';

  @override
  String get microphoneMissing => 'Sprachnotizen brauchen das Mikrofon.';

  @override
  String get typeInstead => 'Lieber tippen';

  @override
  String get widgetNothingToday => 'Heute steht nichts an.';

  @override
  String widgetMore(String count) {
    return 'und $count weitere';
  }

  @override
  String get speechModelTitle => 'Sprachmodell';

  @override
  String get speechModelIntro =>
      'Sprachnotizen bekommen ein Transkript, das du lesen und durchsuchen kannst. Whisper schreibt es auf diesem Telefon, auf Deutsch oder Englisch, mit einem Sprachmodell, das es einmal braucht. Die Aufnahme bleibt immer erhalten.';

  @override
  String speechModelSource(int megabytes) {
    return 'Der Download ist etwa $megabytes MB groß, kommt nur aus dem eigenen Release von Old Brain Fix auf GitHub und wird vor der Verwendung mit einer festen Prüfsumme geprüft. Am besten im WLAN. Sonst wird nie etwas gesendet oder geladen.';
  }

  @override
  String get speechModelCredit =>
      'Das Sprachmodell ist das Basismodell von Whisper von OpenAI, Copyright (c) 2022 OpenAI, unter der MIT-Lizenz.';

  @override
  String get speechModelChecking => 'Sprachmodell wird geprüft …';

  @override
  String get speechModelUnsupported =>
      'Auf diesem Gerät gibt es keine Transkripte. Sprachaufnahmen bleiben, wie sie sind.';

  @override
  String get speechModelReady =>
      'Das Sprachmodell ist bereit. Sprachnotizen werden auf diesem Telefon transkribiert.';

  @override
  String get speechModelMissing => 'Noch kein Sprachmodell';

  @override
  String speechModelDownload(int megabytes) {
    return 'Herunterladen ($megabytes MB)';
  }

  @override
  String get speechModelPick => 'Eigene Datei verwenden';

  @override
  String get speechModelPickDetail =>
      'Es muss die mehrsprachige ggml-base.bin von whisper.cpp sein; jede andere Datei wird abgelehnt.';

  @override
  String speechModelDownloading(String received, String total) {
    return 'Wird geladen: $received von $total MB';
  }

  @override
  String get speechModelFailed =>
      'Der Download wurde nicht fertig, und nichts wurde behalten. Versuch es später noch einmal.';

  @override
  String get speechModelMismatch =>
      'Das ist nicht das erwartete Sprachmodell, deshalb wurde es nicht behalten.';

  @override
  String get speechModelSourceModels =>
      'Jeder Download kommt nur aus dem eigenen Release von Old Brain Fix auf GitHub und wird vor der Verwendung mit einer festen Prüfsumme geprüft. Am besten im WLAN. Sonst wird nie etwas gesendet oder geladen.';

  @override
  String get speechModelCreditModels =>
      'Die Sprachmodelle sind das Basismodell und das kleine Modell von Whisper von OpenAI, Copyright (c) 2022 OpenAI, unter der MIT-Lizenz.';

  @override
  String get speechModelChoice => 'Modell';

  @override
  String speechModelBase(int megabytes) {
    return 'Basis, $megabytes MB';
  }

  @override
  String get speechModelBaseDetail =>
      'Schneller und genug für Deutsch oder Englisch allein.';

  @override
  String speechModelSmall(int megabytes) {
    return 'Klein, $megabytes MB';
  }

  @override
  String get speechModelSmallDetail =>
      'Besser mit englischen Wörtern in deutscher Sprache, aber langsamer, und es braucht mehr Arbeitsspeicher.';

  @override
  String get speechModelKeepsOne =>
      'Wenn du ein Modell wählst, wird es heruntergeladen. Nur das gewählte Modell wird behalten: Das vorige wird gelöscht, sobald das neue bereit ist.';

  @override
  String get speechModelPickEither =>
      'Es muss die mehrsprachige ggml-base.bin oder ggml-small.bin von whisper.cpp sein; jede andere Datei wird abgelehnt.';

  @override
  String get agenda => 'Agenda';

  @override
  String get speechOfferQuestion =>
      'Sprachnotizen auf dem Telefon transkribieren?';

  @override
  String speechOfferDetail(int megabytes) {
    return 'Lade dafür einmal das Sprachmodell herunter ($megabytes MB). Es lädt im Hintergrund, während du weitermachst, und die Einstellungen bieten es jederzeit später an.';
  }

  @override
  String speechOfferMobileData(int megabytes) {
    return 'Du bist mit mobilen Daten online. Der Download verbraucht davon $megabytes MB.';
  }

  @override
  String get transcribing => 'Wird transkribiert …';

  @override
  String get noWordsHeard => 'Keine Worte erkannt.';

  @override
  String get historyTitle => 'Verlauf';

  @override
  String get streakHeading => 'Serie';

  @override
  String get streakNow => 'Aktuell';

  @override
  String get streakLongest => 'Längste';

  @override
  String get lastSevenDays => 'Letzte 7 Tage';

  @override
  String dayAllDone(String day) {
    return '$day: alles erledigt';
  }

  @override
  String dayPartlyDone(String day) {
    return '$day: teils erledigt, teils verpasst';
  }

  @override
  String dayMissed(String day) {
    return '$day: verpasst';
  }

  @override
  String dayOpen(String day) {
    return '$day: steht noch an';
  }

  @override
  String dayNothing(String day) {
    return '$day: nichts fällig';
  }

  @override
  String get adherenceHeading => 'Einhaltung';

  @override
  String get adherenceThisWeek => 'Diese Woche';

  @override
  String get adherenceThisMonth => 'Diesen Monat';

  @override
  String get adherenceLast30Days => 'Letzte 30 Tage';

  @override
  String adherenceOf(int done, int total) {
    return '$done von $total erledigt';
  }

  @override
  String get adherenceNone => 'Noch nichts';

  @override
  String get showStreaks => 'Serien auf diesem Telefon zeigen';

  @override
  String get showStreaksDetail => 'Die Einhaltung wird immer gezeigt.';

  @override
  String get historyLog => 'Protokoll';

  @override
  String get historyEmpty => 'Bisher war nichts fällig.';

  @override
  String get snoozesHere =>
      'Verschobenes ist nur auf diesem Telefon festgehalten, und nur für etwa den letzten Monat.';

  @override
  String logDone(String time) {
    return 'Erledigt: $time';
  }

  @override
  String logDoneEarly(String time) {
    return 'Früh erledigt: $time';
  }

  @override
  String logDoneLate(String time) {
    return 'Spät erledigt: $time';
  }

  @override
  String get logMissed => 'Verpasst';

  @override
  String get logOpen => 'Fällig, noch nicht erledigt';

  @override
  String get logComing => 'Kommt noch';

  @override
  String logSnoozed(String time) {
    return 'Verschoben bis $time';
  }

  @override
  String get calendarTooltip => 'Kalender';

  @override
  String get addNoteToCalendar => 'In den Kalender, Zeit selbst wählen';

  @override
  String addToCalendarAt(String when) {
    return 'In den Kalender: $when';
  }

  @override
  String get shareCalendarFile => 'Als Kalenderdatei teilen';

  @override
  String get noCalendarApp =>
      'Es gibt keine Kalender-App, die es aufnehmen kann.';

  @override
  String get nothingTakesCalendarFile =>
      'Keine App kann die Kalenderdatei annehmen.';

  @override
  String get calendarExportTooltip => 'In den Kalender exportieren';

  @override
  String get calendarExportTitle => 'Kalenderexport';

  @override
  String get calendarExportAll => 'Alle Routinen und Erinnerungen';

  @override
  String get calendarExportEmpty => 'Noch keine Routinen oder Erinnerungen.';

  @override
  String get calendarMirrorSwitch => 'Im Kalender führen';

  @override
  String get calendarMirrorDetail =>
      'Deine Routinen und Erinnerungen erscheinen in einem Kalender deiner Wahl auf diesem Gerät und folgen jeder Änderung.';

  @override
  String calendarMirrorIn(String calendar) {
    return 'In „$calendar“. Tippen, um einen anderen Kalender zu wählen.';
  }

  @override
  String get calendarMirrorExplainTitle => 'Im Kalender führen?';

  @override
  String get calendarMirrorExplainBody =>
      'Old Brain Fix kann deine Routinen und Erinnerungen in einen Kalender deiner Wahl schreiben und aktuell halten. Es ändert oder entfernt nur Termine, die es selbst geschrieben hat, und wenn du das ausschaltest, entfernt es sie wieder. Wohin dieser Kalender synchronisiert, dort erscheinen sie auch.';

  @override
  String get calendarMirrorNotAllowed =>
      'Old Brain Fix darf nicht in deinen Kalender schreiben. Du kannst es in den Systemeinstellungen erlauben.';

  @override
  String get calendarMirrorNoCalendar =>
      'Auf diesem Gerät gibt es keinen Kalender, in den geschrieben werden kann.';

  @override
  String get calendarMirrorNotRemoved =>
      'Die Termine konnten nicht aus dem Kalender entfernt werden. Erlaube Old Brain Fix den Kalender und schalte es noch einmal aus.';

  @override
  String get settingsTitle => 'Einstellungen';

  @override
  String get settingsIntro =>
      'Diese Einstellungen gelten nur für dieses Handy und werden nie synchronisiert.';

  @override
  String get settingsLook => 'Aussehen';

  @override
  String get themeSystem => 'Wie das System';

  @override
  String get themeLight => 'Hell';

  @override
  String get themeDark => 'Dunkel';

  @override
  String get settingsAlarms => 'Alarme';

  @override
  String get settingsAlarmSetupDetail =>
      'Die Berechtigungen, die Alarme brauchen, um pünktlich zu klingeln.';

  @override
  String get settingsRenagDetail =>
      'Wie oft ein unbeachteter Alarm erneut klingelt, wenn seine Routine keinen eigenen Abstand hat.';

  @override
  String get settingsNudges => 'Stupser';

  @override
  String get settingsCapture => 'Erfassen';

  @override
  String get settingsNamedPlacesDetail =>
      '„Zuhause“ oder „Arbeit“ statt Koordinaten. Sie gehören zu deinen Notizen, nicht nur zu diesem Handy.';

  @override
  String get settingsCaptureContextDetail =>
      'Kalendertermin und Bluetooth-Gerät zu jeder Notiz.';

  @override
  String get settingsSpeechModelDetail => 'Für Transkripte von Sprachnotizen.';

  @override
  String get settingsCalendar => 'Kalender';

  @override
  String get settingsHistory => 'Verlauf';

  @override
  String get aboutTitle => 'Über Old Brain Fix';

  @override
  String get aboutTagline =>
      'Einen Gedanken mit einem Tipp festhalten. Erinnert werden, bis er einsortiert oder erledigt ist.';

  @override
  String versionLabel(String version, String build) {
    return 'Version $version ($build)';
  }

  @override
  String get documentation => 'Dokumentation';

  @override
  String get privacyPolicy => 'Datenschutzerklärung';

  @override
  String get sourceCode => 'Quellcode';

  @override
  String get reportProblem => 'Ein Problem oder eine Idee melden';

  @override
  String get githubIssues => 'GitHub-Issues';

  @override
  String get writeTheDeveloper => 'Dem Entwickler schreiben';

  @override
  String get buyCoffee => 'Spendier mir einen Kaffee';

  @override
  String get coffeeSubtitle => 'Auf Ko-fi, wenn Old Brain Fix dir hilft';

  @override
  String get openSourceLicenses => 'Open-Source-Lizenzen';

  @override
  String get dangerButton => 'NICHT DRÜCKEN.\nGEFAHR';

  @override
  String get dangerThanks => 'Danke, dass du Old Brain Fix benutzt!';

  @override
  String get settingsPrivacy => 'Privatsphäre';

  @override
  String get appLock => 'App-Sperre';

  @override
  String get appLockDetail =>
      'Fragt nach der PIN, bevor deine Notizen zu sehen sind. Erfassen und Alarme brauchen sie nie.';

  @override
  String get appLockOff => 'App-Sperre ausschalten';

  @override
  String get unlockApp => 'Old Brain Fix entsperren';

  @override
  String get lockedTitle => 'Old Brain Fix ist gesperrt';

  @override
  String get lockedExplain =>
      'Entsperre, um deine Notizen zu sehen. Mit + kannst du sofort erfassen.';

  @override
  String get unlock => 'Entsperren';

  @override
  String get unlockBiometrics =>
      'Mit Fingerabdruck oder Displaysperre entsperren';

  @override
  String get biometricsDetail => 'Die PIN funktioniert immer auch.';

  @override
  String get pinLabel => 'PIN';

  @override
  String get pinAgainLabel => 'PIN wiederholen';

  @override
  String get pinWrong => 'Falsche PIN';

  @override
  String pinTooShort(int count) {
    return 'Mindestens $count Ziffern';
  }

  @override
  String get pinMismatch => 'Die beiden PINs sind verschieden';

  @override
  String get pinNewTitle => 'Neue PIN';

  @override
  String get pinNewExplain =>
      'Sie entsperrt die App-Sperre und die Bereichssperren auf diesem Handy.';

  @override
  String get pinChange => 'PIN ändern';

  @override
  String get relockAfter => 'Wieder sperren nach';

  @override
  String get relockDetail =>
      'Wie lange die App im Hintergrund sein darf, bevor sie wieder sperrt.';

  @override
  String get relockAtOnce => 'Sofort';

  @override
  String get areaLocksTitle => 'Bereichssperren';

  @override
  String get areaLocksDetail =>
      'Bereiche, die gesperrt bleiben, auch wenn die App offen ist.';

  @override
  String get areaLocksExplain =>
      'Ein gesperrter Bereich und seine Notizen bleiben verborgen, bis du ihn entsperrst. Seine Alarme erscheinen maskiert, „Private Routine, 08:00“, außer du lässt sie vollständig zeigen; abhaken kannst du sie immer.';

  @override
  String get areaLocked => 'Gesperrt';

  @override
  String unlockArea(String area) {
    return '$area entsperren';
  }

  @override
  String areaLockOff(String area) {
    return '$area dauerhaft entsperren';
  }

  @override
  String areaShowFull(String area) {
    return 'Alarme von $area vollständig zeigen';
  }

  @override
  String get areaAlarmsShow => 'Alarme und Agenda zeigen';

  @override
  String get areaAlarmsMasked => 'Maskiert';

  @override
  String get areaAlarmsFull => 'Vollständig';

  @override
  String get maskedRoutine => 'Private Routine';

  @override
  String maskedRoutineAt(String time) {
    return 'Private Routine, $time';
  }

  @override
  String get maskedReminder => 'Private Erinnerung';

  @override
  String get syncTitle => 'Synchronisieren';

  @override
  String get syncSettingsEntry => 'Mit einem anderen Gerät synchronisieren';

  @override
  String get syncSettingsDetail =>
      'Einmal per Code koppeln; danach haben beide dieselben Notizen, Bereiche und Routinen.';

  @override
  String get syncJoinIntro =>
      'Öffne auf dem anderen Gerät „Synchronisieren“. Scanne den angezeigten Code oder tippe ihn ein.';

  @override
  String get syncScan => 'Code scannen';

  @override
  String get syncCodeLabel => 'Kopplungscode';

  @override
  String get syncJoin => 'Jetzt synchronisieren';

  @override
  String get syncShowIntro =>
      'Oder zeig hier einen Code, den das andere Gerät scannt.';

  @override
  String get syncShowCode => 'Code dieses Geräts zeigen';

  @override
  String get shareShowIntro =>
      'Oder zeig hier einen Code, den ihr Gerät scannt.';

  @override
  String get hostTitle => 'Code dieses Geräts';

  @override
  String get hostIntro =>
      'Öffne auf dem anderen Gerät „Synchronisieren“ und scanne diesen Code oder tippe den Code darunter ein.';

  @override
  String get hostInviteIntro =>
      'Auf ihrem Gerät „Synchronisieren“ öffnen und diesen Code scannen oder den Code darunter eintippen. Ihr Profil wird dann hier zur Bestätigung gezeigt.';

  @override
  String get hostTypedCode => 'Code zum Eintippen';

  @override
  String get hostStays =>
      'Der Code gilt, solange diese Seite offen ist. Geh zurück, sobald das andere Gerät meldet, dass es synchronisiert hat.';

  @override
  String get hostNoNetwork =>
      'Dieses Gerät ist in keinem WLAN. Verbinde es mit demselben WLAN wie das andere Gerät und versuch es dann noch einmal.';

  @override
  String get hostLocked =>
      'Zu viele falsche Codes wurden versucht. Erzeuge einen neuen Code, um weiterzumachen.';

  @override
  String get hostFailed =>
      'Es konnte kein Code erzeugt werden. Versuch es noch einmal.';

  @override
  String get hostTryAgain => 'Noch einmal versuchen';

  @override
  String get hostNewCode => 'Neuer Code';

  @override
  String hostAskTitle(String name) {
    return '$name beitreten lassen?';
  }

  @override
  String get hostAskUnnamed => 'Das andere Gerät beitreten lassen?';

  @override
  String get hostAskText =>
      'Es bekommt alles auf diesem Gerät und synchronisiert sich von da an mit ihm. Sag nur bei einem eigenen Gerät Ja.';

  @override
  String get hostAllow => 'Beitreten lassen';

  @override
  String get syncSyncing => 'Wird synchronisiert …';

  @override
  String get syncInvalidCode =>
      'Das ist kein Kopplungscode. Vergleiche ihn mit dem Bildschirm des anderen Geräts.';

  @override
  String get syncNoCode =>
      'Auf dem Foto ist kein Code zu finden. Versuch es etwas näher oder tippe den Code ein.';

  @override
  String syncCameIn(String name, int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Änderungen angekommen',
      one: '1 Änderung angekommen',
      zero: 'nichts Neues angekommen',
    );
    return 'Mit $name synchronisiert: $_temp0.';
  }

  @override
  String syncRefused(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count Änderungen wurden abgelehnt: Ihre Signaturen passten nicht.',
      one: '1 Änderung wurde abgelehnt: Ihre Signatur passte nicht.',
    );
    return '$_temp0';
  }

  @override
  String get syncFailNotPrivate =>
      'Dieser Code zeigt aus deinem Heimnetz hinaus. Nur Geräte im selben Netz können sich synchronisieren.';

  @override
  String get syncFailUnreachable =>
      'Das andere Gerät hat nicht geantwortet. Zeigt es den Code noch, und sind beide im selben WLAN?';

  @override
  String get syncFailWrongHost =>
      'Das ist nicht das Gerät, zu dem der Code gehört.';

  @override
  String get syncFailWrongPin =>
      'Die PIN passt nicht. Tippe den Code noch einmal ein.';

  @override
  String get syncFailLocked =>
      'Zu viele falsche Versuche. Lass auf dem anderen Gerät einen neuen Code zeigen.';

  @override
  String get syncFailDeclined => 'Das andere Gerät hat dieses abgelehnt.';

  @override
  String get syncFailUpdateHere =>
      'Old Brain Fix auf diesem Gerät braucht ein Update, um mit dem anderen Gerät zu synchronisieren.';

  @override
  String get syncFailUpdateThere =>
      'Old Brain Fix auf dem anderen Gerät braucht ein Update, um mit diesem zu synchronisieren.';

  @override
  String syncUpdateHereWith(String name) {
    return 'Old Brain Fix auf diesem Gerät braucht ein Update, um mit $name zu synchronisieren.';
  }

  @override
  String syncUpdateThereWith(String name) {
    return 'Old Brain Fix auf $name braucht ein Update, um mit diesem Gerät zu synchronisieren.';
  }

  @override
  String get folderNeedsUpdate =>
      'Ein anderes Gerät schreibt ein neueres Format in den geteilten Ordner. Old Brain Fix auf diesem Gerät braucht ein Update, um es zu lesen.';

  @override
  String get syncFailFailed =>
      'Die Synchronisierung brach ab. Nichts ging verloren; versuch es noch einmal.';

  @override
  String get pairedDevicesTitle => 'Gekoppelte Geräte';

  @override
  String get pairedNone => 'Noch keine gekoppelten Geräte.';

  @override
  String pairedLastSync(String when) {
    return 'Zuletzt synchronisiert $when';
  }

  @override
  String get forgetDevice => 'Vergessen';

  @override
  String get syncSettingsStay =>
      'Einstellungen bleiben auf jedem Gerät und werden nie synchronisiert.';

  @override
  String get syncAutomatically => 'Automatisch synchronisieren';

  @override
  String get syncAutoIntro =>
      'Gekoppelte Geräte im selben WLAN synchronisieren sich von selbst: während die App offen ist und im Hintergrund jede Viertelstunde.';

  @override
  String get conflictsTitle => 'Konflikte';

  @override
  String conflictsEntry(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Konflikte zu klären',
      one: '1 Konflikt zu klären',
      zero: 'Keine Konflikte',
    );
    return '$_temp0';
  }

  @override
  String get conflictsIntro =>
      'Zwei Geräte haben dasselbe geändert, bevor sie sich synchronisiert haben. Tippe auf die Fassung, die bleibt; bis dahin wird die neuere gezeigt.';

  @override
  String get conflictsNone => 'Keine Konflikte. Alle Geräte sind sich einig.';

  @override
  String syncConflicts(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count Sachen wurden auf beiden Geräten geändert. Tippe, um zu wählen, welche Fassungen bleiben.',
      one: '1 Sache wurde auf beiden Geräten geändert. Tippe, um zu wählen, welche Fassung bleibt.',
    );
    return '$_temp0';
  }

  @override
  String get conflictNoteText => 'Text einer Notiz';

  @override
  String get conflictSectionText => 'Zur Notiz hinzugefügter Text';

  @override
  String get conflictAreaName => 'Name eines Bereichs';

  @override
  String get conflictTopicName => 'Name eines Themas';

  @override
  String get conflictPlaceName => 'Name eines benannten Orts';

  @override
  String get conflictPlaceRadius => 'Umkreis eines benannten Orts';

  @override
  String get conflictSchedule => 'Plan einer Routine';

  @override
  String get conflictOther => 'Änderung';

  @override
  String get conflictEmpty => '(leer)';

  @override
  String get conflictNewerFormat =>
      'Eine Fassung wurde mit einer neueren Version von Old Brain Fix geschrieben. Aktualisiere die App, um eine zu wählen.';

  @override
  String get textNewerFormat =>
      'Dieser Text wurde mit einer neueren Version von Old Brain Fix geschrieben. Aktualisiere die App, um ihn zu ändern.';

  @override
  String conflictOnThisDevice(String when) {
    return 'Auf diesem Gerät, $when';
  }

  @override
  String conflictOnDevice(String device, String when) {
    return 'Auf $device, $when';
  }

  @override
  String conflictOnUnnamed(String when) {
    return 'Auf einem Gerät ohne Namen, $when';
  }

  @override
  String get conflictKeep => 'Diese Fassung behalten';

  @override
  String get folderTitle => 'Geteilter Ordner';

  @override
  String get folderIntro =>
      'Synchronisiert über einen Ordner, den Syncthing, Nextcloud oder ein USB-Stick zwischen deinen Geräten trägt, wo immer sie sind. Alles darin ist verschlüsselt: Nur Geräte, die mit diesem gekoppelt sind, können es lesen.';

  @override
  String get folderPick => 'Ordner wählen';

  @override
  String get folderForget => 'Vergessen';

  @override
  String get folderNotYet => 'Noch nicht synchronisiert';

  @override
  String folderSynced(String when, int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Änderungen angekommen',
      one: '1 Änderung angekommen',
      zero: 'nichts Neues',
    );
    return 'Zuletzt synchronisiert $when: $_temp0';
  }

  @override
  String folderUnreachable(String when) {
    return 'Der Ordner war nicht erreichbar ($when). Es wird später wieder versucht.';
  }

  @override
  String folderUnreadable(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count Dateien stammen von Geräten, die nicht mit diesem gekoppelt sind. Kopple sie zuerst.',
      one: '1 Datei stammt von einem Gerät, das nicht mit diesem gekoppelt ist. Kopple sie zuerst.',
    );
    return '$_temp0';
  }

  @override
  String get bundleTitle => 'Sync-Paket';

  @override
  String get bundleIntro =>
      'Schickt das Neue als eine Datei über einen beliebigen Messenger oder per Mail, und das andere Gerät importiert sie. Alles darin ist verschlüsselt: Nur Geräte, die mit diesem gekoppelt sind, können es lesen.';

  @override
  String get bundleFor => 'Für';

  @override
  String get bundleEverything => 'Alles';

  @override
  String get bundleSend => 'Paket senden';

  @override
  String get bundleImport => 'Paket importieren';

  @override
  String bundleNothingNew(String name) {
    return 'Nichts Neues für $name.';
  }

  @override
  String bundleCameIn(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Mit dem Paket kamen $count Änderungen an.',
      one: 'Mit dem Paket kam 1 Änderung an.',
      zero: 'Mit dem Paket kam nichts Neues an.',
    );
    return '$_temp0';
  }

  @override
  String get bundlePairFirst =>
      'Dieses Paket stammt von einem Gerät, das nicht mit diesem gekoppelt ist, darum lässt sich nichts darin lesen. Kopple sie zuerst und importiere es dann noch einmal.';

  @override
  String get bundleNotABundle =>
      'Diese Datei ist kein Sync-Paket, das diese App lesen kann.';

  @override
  String get bundleNeedsUpdate =>
      'Dieses Sync-Paket stammt von einem neueren Old Brain Fix. Old Brain Fix auf diesem Gerät braucht ein Update, um es zu lesen.';

  @override
  String get bundleFailed =>
      'Das Paket hat nicht geklappt. Nichts ging verloren; versuch es noch einmal.';

  @override
  String get bundleNoApp => 'Keine App hier kann das Paket annehmen.';

  @override
  String get shareArea => 'Teilen …';

  @override
  String get areaShared => 'Geteilt';

  @override
  String get peopleTitle => 'Personen';

  @override
  String shareTitle(String area) {
    return '$area teilen';
  }

  @override
  String get shareIntro =>
      'Triff die andere Person. Auf ihrem Gerät „Synchronisieren“ öffnen und den Code zeigen lassen; scanne ihn oder tippe ihn hier ein. Ihr Profil wird zur Bestätigung gezeigt; dann bekommt sie diesen Bereich und sonst nichts.';

  @override
  String get roleHeading => 'Sie darf';

  @override
  String get roleManager => 'Verwalter';

  @override
  String get roleMember => 'Mitglied';

  @override
  String get roleManagerDetail => 'Alles in diesem Bereich.';

  @override
  String get roleMemberDetail =>
      'Notizen anlegen und die eigenen ändern, jede Notiz ergänzen und Routinen und Erinnerungen abhaken.';

  @override
  String get shareInvite => 'Einladen';

  @override
  String shareDone(String area, String name) {
    return '$area ist jetzt mit $name geteilt.';
  }

  @override
  String sharedWithYou(String name, String area) {
    return '$name hat $area mit dir geteilt.';
  }

  @override
  String peopleYou(String name) {
    return '$name (du)';
  }

  @override
  String get makeManager => 'Zum Verwalter machen';

  @override
  String get makeMember => 'Zum Mitglied machen';

  @override
  String get removePerson => 'Entfernen';

  @override
  String removePersonQuestion(String name, String area) {
    return '$name aus $area entfernen?';
  }

  @override
  String get removePersonDetail =>
      'Von diesem Bereich geht nichts mehr an sie. Was sie schon hat, bleibt bei ihr.';

  @override
  String get peopleIntro =>
      'Ein Verwalter darf hier alles. Ein Mitglied darf Notizen anlegen und die eigenen ändern, jede Notiz ergänzen und Routinen und Erinnerungen abhaken.';

  @override
  String get onlyManagers =>
      'Das kann nur ein Verwalter dieses Bereichs ändern.';

  @override
  String get pairedPerson => 'Gerät einer anderen Person';

  @override
  String get sendNote => 'Diese Notiz senden';

  @override
  String get sendTitle => 'Diese Notiz senden';

  @override
  String get sendWho => 'Wer bekommt sie?';

  @override
  String get sendAnyone => 'Irgendwer, unverschlüsselt';

  @override
  String sendEncrypted(String name) {
    return '$name, verschlüsselt';
  }

  @override
  String get sendPlainHint =>
      'Nicht verschlüsselt: Wer die Datei bekommt, kann sie lesen. Zum Verschlüsseln teile zuerst einen Bereich mit der Person oder kopple dich mit ihrem Gerät.';

  @override
  String get sendWhat =>
      'Sie enthält den Text, die Abschnitte und die Routinen und Erinnerungen; keine Fotos, Aufnahmen und nicht, wo sie erfasst wurde. Sendest du sie später noch einmal, kommt sie als Aktualisierung an.';

  @override
  String get sendAction => 'Senden';

  @override
  String get sendNoApp => 'Keine App hier kann die Datei annehmen.';

  @override
  String get sentItemTitle => 'An dich gesendet';

  @override
  String sentItemFrom(String name) {
    return 'Von $name';
  }

  @override
  String sentItemVerified(String code) {
    return 'Geprüft: Code persönlich verglichen. Code $code';
  }

  @override
  String sentItemKnown(String code) {
    return 'Nicht geprüft: hier bekannt, Code noch nicht verglichen. Code $code';
  }

  @override
  String sentItemUnknown(String code) {
    return 'Nicht geprüft: Dieses Profil ist hier neu. Code $code';
  }

  @override
  String get sentItemCompare => 'Codes vergleichen';

  @override
  String sentItemCompareHint(String name, String code) {
    return 'Bitte $name persönlich, den Code auf der eigenen Profilseite vorzulesen. Er muss $code lauten.';
  }

  @override
  String get sentItemCodesMatch => 'Codes stimmen überein';

  @override
  String get sentItemCodesDiffer => 'Codes weichen ab';

  @override
  String get sentItemVerifiedOnAccept =>
      'Wird beim Übernehmen als geprüft vermerkt.';

  @override
  String get sentItemCodesDiffered =>
      'Die Codes weichen ab: Diese Datei kommt vielleicht nicht von der Person, die sie nennt.';

  @override
  String get sentItemEncrypted => 'Sie kam verschlüsselt an dieses Gerät.';

  @override
  String get sentItemPlain =>
      'Sie kam unverschlüsselt: Andere könnten sie unterwegs gelesen haben.';

  @override
  String get sentItemAccept => 'Annehmen';

  @override
  String get sentItemDecline => 'Ablehnen';

  @override
  String get sentItemAccepted => 'In Unsortiert abgelegt, mit ihren Alarmen.';

  @override
  String get sentItemDeclined => 'Abgelehnt: Nichts wurde behalten.';

  @override
  String get sentItemAlready =>
      'Schon aktuell: Diese oder eine neuere Fassung hast du schon angenommen.';

  @override
  String get sentItemUnreadable =>
      'Diese Datei lässt sich nicht lesen: Sie ist beschädigt oder nicht mit dem Schlüssel signiert, den sie nennt.';

  @override
  String get sentItemNotForUs =>
      'Diese Datei ist für jemand anderen verschlüsselt.';

  @override
  String get sendUpdate => 'Eine Aktualisierung senden';

  @override
  String get sendCancel => 'Stattdessen eine Absage senden';

  @override
  String get sendCancelHint =>
      'Dann kann die andere Person sie mit ausgeschalteten Alarmen in den Papierkorb legen.';

  @override
  String get sentItemUpdate =>
      'Eine Aktualisierung einer Notiz, die du schon angenommen hast.';

  @override
  String get sentItemChanges => 'Was sich ändert';

  @override
  String sentItemTextWas(String text) {
    return 'Text bisher: $text';
  }

  @override
  String sentItemTextNow(String text) {
    return 'Neuer Text: $text';
  }

  @override
  String get sentItemTextYours =>
      'Du hast diesen Text selbst geändert; die Aktualisierung ersetzt deine Fassung.';

  @override
  String sentItemSectionAdded(String text) {
    return 'Hinzugefügt: $text';
  }

  @override
  String sentItemRescheduled(String before, String after) {
    return '$before → $after';
  }

  @override
  String sentItemRoutineAdded(String schedule) {
    return 'Neu: $schedule';
  }

  @override
  String sentItemRoutineRemoved(String schedule) {
    return 'Entfernt: $schedule';
  }

  @override
  String get sentItemNoChanges => 'Nichts hat sich geändert.';

  @override
  String get sentItemRestores =>
      'Sie liegt in deinem Papierkorb; die Aktualisierung holt sie zurück.';

  @override
  String get sentItemKeeps =>
      'Deine eigenen Abschnitte, Routinen und Erinnerungen, Erledigt-Haken, die Wichtigkeit und wo du sie abgelegt hast, bleiben, wie sie sind.';

  @override
  String get sentItemApply => 'Aktualisieren';

  @override
  String get sentItemApplied =>
      'Aktualisiert. Deine eigenen Ergänzungen sind geblieben.';

  @override
  String sentItemCancelled(String text) {
    return 'Abgesagt: $text';
  }

  @override
  String get sentItemCancelDetail =>
      'Annehmen legt sie mit ihren Routinen und Erinnerungen in den Papierkorb, die Alarme aus. Von dort kannst du sie zurückholen.';

  @override
  String get sentItemAcceptCancel => 'In den Papierkorb';

  @override
  String get sentItemTrashed =>
      'In den Papierkorb gelegt, die Alarme aus. Von dort kannst du sie zurückholen.';

  @override
  String get sentItemNothingToCancel =>
      'Nichts abzusagen: Diese Notiz ist nicht unter deinen Notizen oder schon im Papierkorb.';

  @override
  String get sentItemNotFromSender =>
      'Abgewiesen: Diese Datei würde eine Notiz ändern, die dir jemand gesendet hat, ist aber nicht von dieser Person signiert.';

  @override
  String get sentItemClose => 'Schließen';

  @override
  String get backupsTitle => 'Sicherungen';

  @override
  String get backupsSettingsEntry => 'Dieses Gerät sichern';

  @override
  String get backupsSettingsDetail =>
      'Verschlüsselte Dateien mit allem auf diesem Gerät, in einem Ordner deiner Wahl.';

  @override
  String get backupsIntro =>
      'Eine Sicherung ist eine verschlüsselte Datei mit all deinen Bereichen, Notizen, Fotos, Sprachaufnahmen, Routinen und ihrem Verlauf. Nur deine Passphrase oder der Wiederherstellungsschlüssel auf dem Wiederherstellungsblatt öffnet sie. Die Einstellungen dieses Geräts bleiben draußen. All deine Geräte teilen sich eine Passphrase und einen Wiederherstellungsschlüssel, und jedes schreibt eigene Sicherungen.';

  @override
  String get backupPassphrase => 'Passphrase';

  @override
  String get backupPassphraseAgain => 'Passphrase wiederholen';

  @override
  String get backupPassphraseHint =>
      'Mindestens 8 Zeichen. Ein kurzer Satz ist leicht zu merken.';

  @override
  String get backupPassphraseShort => 'Bitte mindestens 8 Zeichen.';

  @override
  String get backupPassphraseMismatch => 'Die beiden stimmen nicht überein.';

  @override
  String get backupSetUp => 'Sicherungen einrichten';

  @override
  String get backupSettingUp =>
      'Wird eingerichtet … das dauert ein paar Sekunden.';

  @override
  String get backupSetUpElsewhere =>
      'Schon auf einem deiner anderen Geräte eingerichtet? Synchronisiere mit ihm, dann übernimmt dieses Gerät dieselbe Passphrase und denselben Wiederherstellungsschlüssel.';

  @override
  String get backupSheetTitle => 'Wiederherstellungsblatt';

  @override
  String get backupSheetIntro =>
      'Drucke das Wiederherstellungsblatt aus oder bewahre es sicher auf. Sein Wiederherstellungsschlüssel öffnet die Sicherungen all deiner Geräte, wenn die Passphrase vergessen ist.';

  @override
  String get backupSheetPrint => 'Drucken oder speichern';

  @override
  String get backupSheetShare => 'Teilen';

  @override
  String get backupSheetFailed =>
      'Das Wiederherstellungsblatt konnte nicht erstellt werden. Versuch es noch einmal.';

  @override
  String get backupSheetNoApp =>
      'Keine App hier kann das Wiederherstellungsblatt annehmen.';

  @override
  String get backupFolderTitle => 'Ordner';

  @override
  String get backupFolderNone =>
      'Noch kein Ordner gewählt. Sicherungen kommen in einen Ordner deiner Wahl: auf diesem Gerät, auf einem USB-Stick oder in einer Cloud-App.';

  @override
  String backupFolderChosen(String name) {
    return 'Sicherungen kommen nach $name.';
  }

  @override
  String get backupChooseFolder => 'Ordner wählen';

  @override
  String get backupWeekly => 'Jede Woche sichern';

  @override
  String get backupWeeklyDetail =>
      'Die letzten 4 Sicherungen dieses Geräts bleiben, ältere gehen.';

  @override
  String get backupNow => 'Jetzt sichern';

  @override
  String get backupWorking => 'Wird gesichert …';

  @override
  String backupLast(String when) {
    return 'Letzte Sicherung: $when';
  }

  @override
  String get backupNotYet => 'Noch keine Sicherung.';

  @override
  String backupFailedAt(String when) {
    return 'Die Sicherung vom $when konnte nicht geschrieben werden. Gibt es den Ordner noch?';
  }

  @override
  String get backupDone => 'Sicherung geschrieben.';

  @override
  String get backupFailed =>
      'Die Sicherung konnte nicht geschrieben werden. Gibt es den Ordner noch?';

  @override
  String get restoreOffer => 'Aus einer Sicherung wiederherstellen';

  @override
  String get restoreOfferDetail =>
      'Du kommst von einem anderen Gerät? Hol alles aus einer Sicherungsdatei zurück.';

  @override
  String get restoreIntro =>
      'Wähle die Sicherungsdatei und öffne sie mit deiner Passphrase oder mit dem Wiederherstellungsschlüssel vom Wiederherstellungsblatt. Einstellungen sind nicht in einer Sicherung: Stell sie danach neu ein und kopple deine anderen Geräte noch einmal.';

  @override
  String get restorePick => 'Sicherungsdatei wählen';

  @override
  String get restoreNotABackup =>
      'Diese Datei ist keine Sicherung von Old Brain Fix.';

  @override
  String restoreFile(String when) {
    return 'Sicherung vom $when';
  }

  @override
  String get restoreOr => 'oder';

  @override
  String get restoreCodeLabel => 'Wiederherstellungsschlüssel';

  @override
  String get restoreScan => 'Wiederherstellungsblatt scannen';

  @override
  String get restoreNoCode =>
      'Kein Wiederherstellungsschlüssel im Foto gefunden. Versuch es etwas näher noch einmal oder tippe ihn ein.';

  @override
  String get restoreInvalidCode =>
      'Das ist kein Wiederherstellungsschlüssel. Vergleich ihn mit dem Wiederherstellungsblatt.';

  @override
  String get restoreStart => 'Wiederherstellen';

  @override
  String get restoreWorking =>
      'Wird wiederhergestellt … das dauert einen Moment.';

  @override
  String get restoreWrong =>
      'Diese Passphrase oder dieser Wiederherstellungsschlüssel öffnet die Sicherung nicht.';

  @override
  String get restoreDamaged =>
      'Die Sicherung ist beschädigt oder unvollständig. Nichts wurde wiederhergestellt; versuch eine ältere.';

  @override
  String get restoreNotEmpty =>
      'Auf diesem Gerät gibt es schon Notizen, deshalb wurde nichts wiederhergestellt.';

  @override
  String get restoreFailed =>
      'Die Sicherung konnte nicht wiederhergestellt werden. Nichts wurde geändert.';

  @override
  String restoreDone(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Änderungen',
      one: '1 Änderung',
    );
    return 'Wiederhergestellt: $_temp0. Stell Alarme und Sperren neu ein, wähl den Ordner für Sicherungen noch einmal und kopple deine anderen Geräte noch einmal.';
  }

  @override
  String get recoverySheetPdfTitle => 'Old Brain Fix: Wiederherstellungsblatt';

  @override
  String recoverySheetMade(String date) {
    return 'Erstellt am $date';
  }

  @override
  String recoverySheetProfile(String name) {
    return 'Profil: $name';
  }

  @override
  String get recoverySheetPdfIntro =>
      'Dieser Wiederherstellungsschlüssel öffnet die Sicherungen all deiner Geräte, auch wenn die Passphrase vergessen ist. Wer ihn hat, kann sie lesen: Bewahre dieses Blatt sicher auf, in einer Schublade oder einem Passwort-Manager, nie neben den Sicherungen.';

  @override
  String get recoverySheetKey => 'Wiederherstellungsschlüssel';

  @override
  String get recoverySheetStepRestore =>
      'Zum Wiederherstellen installiere Old Brain Fix, wähle auf dem ersten Bildschirm „Aus einer Sicherung wiederherstellen“, nimm die neueste Sicherungsdatei und tippe diesen Schlüssel ein oder scanne den QR-Code.';

  @override
  String get recoverySheetStepSettings =>
      'Einstellungen wie Alarme, Ruhezeiten und Sperren sind nicht in einer Sicherung. Stell sie auf dem neuen Gerät neu ein und kopple es noch einmal mit deinen anderen Geräten.';

  @override
  String get backupAutoTitle => 'Google-Sicherung';

  @override
  String get backupAutoSwitch =>
      'Die neueste Sicherung auch in meinem Google-Konto aufbewahren';

  @override
  String get backupAutoDetail =>
      'Die Sicherung von Android bewahrt dann die neueste Sicherungsdatei, verschlüsselt wie sie ist, in deinem Google-Konto auf, sonst nichts von der App. Auf einem neuen Handy kommt sie mit deinen Apps zurück, und die App fragt nach der Passphrase oder dem Wiederherstellungsschlüssel.';

  @override
  String get backupAutoTooBig =>
      'Die neueste Sicherung ist größer als die 25 MB, die Android an Google schickt, deshalb bewahrt Google keine auf. Ein Ordner nimmt sie weiter.';

  @override
  String get restoreFoundTitle => 'Eine Sicherung ist zurück';

  @override
  String get restoreFound =>
      'Dieses Handy hat eine Sicherung aus deinem Google-Konto zurückgebracht. Öffne sie mit deiner Passphrase oder mit dem Wiederherstellungsschlüssel vom Wiederherstellungsblatt.';

  @override
  String get restoreFoundSkip => 'Ohne sie anfangen';

  @override
  String get profileTitle => 'Profil';

  @override
  String get profileNoName => 'Ohne Namen';

  @override
  String get profileName => 'Name';

  @override
  String get profileCode => 'Dein Code';

  @override
  String get profilePictureChange => 'Bild ändern';

  @override
  String get profilePictureChoose => 'Bild auswählen';

  @override
  String get profilePictureTake => 'Foto aufnehmen';

  @override
  String get profilePictureInitials => 'Stattdessen Initialen zeigen';

  @override
  String get profilePictureUnreadable => 'Dieses Bild lässt sich nicht lesen.';

  @override
  String get profileCropTitle => 'Ausschnitt wählen';

  @override
  String get profileCropHint =>
      'Ziehe das Quadrat, um es zu verschieben. Der Regler bestimmt seine Größe.';

  @override
  String get profileCropSize => 'Größe des Quadrats';

  @override
  String get profileCropUse => 'Diesen Ausschnitt nehmen';

  @override
  String get profileBackups => 'Sicherungen und Wiederherstellung';

  @override
  String get profileDevices => 'Deine Geräte';

  @override
  String get profileDeviceSettings => 'Einstellungen dieses Geräts';

  @override
  String get profileDeviceSettingsDetail =>
      'Sie gelten nur für dieses Gerät und werden nie synchronisiert.';

  @override
  String get profileWhichTitle => 'Welches Profil bist du?';

  @override
  String get profileWhichText =>
      'Beide Geräte hatten ein eigenes Profil. Das gewählte gilt auf allen deinen Geräten.';

  @override
  String get profileWhichThisDevice => 'Auf diesem Gerät';

  @override
  String get profileWhichOtherDevice => 'Auf dem anderen Gerät';

  @override
  String get profileWhichPick => 'Das bin ich';

  @override
  String get profileWhichLater => 'Später';

  @override
  String get profileWhichWaiting =>
      'Deine Geräte haben zwei Profile. Wähle deines.';

  @override
  String inviteConfirmTitle(String area) {
    return '$area mit dieser Person teilen?';
  }

  @override
  String get inviteConfirmText =>
      'Vergleiche den Code mit dem auf ihrer Profilseite. Erst danach wird etwas geteilt.';

  @override
  String get inviteConfirmShare => 'Teilen';

  @override
  String get profileTheirCode => 'Ihr Code';

  @override
  String authorPicture(String name) {
    return 'Geschrieben von $name';
  }

  @override
  String get shareCancelled => 'Es wurde nichts geteilt.';

  @override
  String get personVerified => 'Bestätigt: Code persönlich verglichen.';

  @override
  String get personNotVerified =>
      'Nicht bestätigt: Vergleiche die Codes persönlich, um sicherzugehen.';

  @override
  String get personCodesDiffered =>
      'Die Codes unterscheiden sich: Dieses Profil gehört vielleicht nicht der Person, die es nennt.';

  @override
  String get personForMe => 'Name und Bild für mich';

  @override
  String get personForMeDetail =>
      'Nur du siehst sie, auf deinen eigenen Geräten. Name und Bild der Person bleiben, wie sie sind.';

  @override
  String get personForMeName => 'Name für mich';

  @override
  String get personForMePicture => 'Bild für mich ändern';

  @override
  String get personTheirPicture => 'Ihr eigenes Bild verwenden';

  @override
  String get profilePeopleEmpty =>
      'Noch niemand. Hier erscheinen die Personen, mit denen du einen Bereich teilst oder deren gesendete Notizen du annimmst.';

  @override
  String get personFilter => 'Notizen zeigen von…';

  @override
  String get personFilterMe => 'Mir';

  @override
  String get personFilterEveryone => 'Allen';

  @override
  String get personFilterShowing => 'Nur Notizen von';

  @override
  String get personTheirNotes => 'Notizen dieser Person';

  @override
  String get crashTitle => 'Das hätte nicht passieren dürfen';

  @override
  String get crashBody =>
      'Old Brain Fix ist auf einen unerwarteten Fehler gestoßen. Deine gespeicherten Notizen sind sicher. Starte die App neu, und wenn es wieder passiert, schick den Bericht, damit es behoben werden kann.';

  @override
  String get crashRestart => 'App neu starten';

  @override
  String get crashSendReport => 'Bericht an den Entwickler senden';

  @override
  String get crashLastRunBody =>
      'Old Brain Fix wurde beim letzten Mal unerwartet beendet. Tippe hier, um dem Entwickler einen kurzen Bericht zu schicken.';

  @override
  String get sendToPerson => 'An eine Person senden';

  @override
  String get addToCalendar => 'In den Kalender';

  @override
  String get deleteNoteQuestion => 'Diese Notiz löschen?';

  @override
  String get deleteNoteDetail => 'Sie bleibt 30 Tage im Papierkorb.';

  @override
  String get deleteNoteOnly => 'Nur die Notiz löschen';

  @override
  String deleteNoteWithLinked(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Mit ihren $count Routinen und Erinnerungen löschen',
      one: 'Mit ihrer Routine oder Erinnerung löschen',
    );
    return '$_temp0';
  }

  @override
  String get removeRecordingKeepText => 'Aufnahme entfernen, Text behalten';

  @override
  String get removePhotoKeepText => 'Foto entfernen, Text behalten';

  @override
  String get removeRecordingQuestion =>
      'Diese Aufnahme entfernen und ihr Transkript behalten?';

  @override
  String get removePhotoQuestion =>
      'Dieses Foto entfernen und seinen Text behalten?';

  @override
  String get removeMediaDetail =>
      'Die Datei verschwindet von all deinen Geräten und bei allen, mit denen du den Bereich der Notiz teilst. Der Text bleibt in der Notiz.';

  @override
  String get remove => 'Entfernen';

  @override
  String get playbackSpeed => 'Wiedergabegeschwindigkeit';

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
    return 'Wiedergabegeschwindigkeit $speed-fach';
  }

  @override
  String get close => 'Schließen';

  @override
  String get report => 'Melden';

  @override
  String sharedRecordingFailed(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count geteilte Aufnahmen können nicht transkribiert werden.',
      one: 'Eine geteilte Aufnahme kann nicht transkribiert werden.',
    );
    return '$_temp0';
  }

  @override
  String get sharedRecordingFailedTitle => 'Aufnahme nicht transkribiert';

  @override
  String get sharedRecordingUnreadable =>
      'In der geteilten Datei ist kein Ton zu finden: Sie ist keine Tondatei oder beschädigt.';

  @override
  String get sharedRecordingUnsupported =>
      'Dieses Telefon kann das Format der geteilten Datei nicht lesen.';

  @override
  String get sharedRecordingFix =>
      'Die Datei bleibt unverändert in der Notiz; schreib selbst in die Notiz, was sie sagt, oder teile sie noch einmal als m4a oder mp3.';

  @override
  String get personBlock => 'Blockieren';

  @override
  String get personUnblock => 'Blockierung aufheben';

  @override
  String get personBlocked =>
      'Blockiert: Nichts, was diese Person schreibt, wird behalten oder gezeigt, nur ihre Erledigt-Haken.';

  @override
  String personBlockQuestion(String name) {
    return '$name blockieren?';
  }

  @override
  String personBlockDetail(String name) {
    return 'Alles, was $name geschrieben hat, wird außer den Erledigt-Haken von deinen Geräten gelöscht, und nichts Neues dieser Person wird behalten. Ein geteilter Ordner kann noch verschlüsselte Dateien dieser Person enthalten, die die App nie öffnet; verlässt du den Ordner, sind sie fort.';
  }

  @override
  String personBlockLeaveDetail(String name, String areas) {
    return 'Alles, was $name geschrieben hat, wird außer den Erledigt-Haken von deinen Geräten gelöscht, und du verlässt, was diese Person verwaltet: $areas. Ein geteilter Ordner kann noch verschlüsselte Dateien dieser Person enthalten, die die App nie öffnet; verlässt du den Ordner, sind sie fort.';
  }

  @override
  String get personBlockLeave => 'Blockieren und verlassen';

  @override
  String personUnblockQuestion(String name) {
    return 'Blockierung von $name aufheben?';
  }

  @override
  String get personUnblockDetail =>
      'Was diese Person geschrieben hat, kommt mit der nächsten Synchronisierung mit einem Gerät zurück, das es noch hat.';

  @override
  String get peopleBlocked => 'Blockiert';

  @override
  String get sentItemBlock => 'Ablehnen und blockieren';

  @override
  String get sentItemBlocked =>
      'Abgelehnt und blockiert: Nichts wurde behalten.';

  @override
  String get sentItemFromBlocked =>
      'Abgewiesen: Diese Datei kommt von einer Person, die du blockiert hast.';

  @override
  String get sentItemStranger =>
      'Eine unbekannte Person: nicht unter deinen Personen.';

  @override
  String get sentItemHidden =>
      'Was sie gesendet hat, bleibt verborgen, bis du auf „Zeigen“ tippst.';

  @override
  String get sentItemShow => 'Zeigen';

  @override
  String get sentItemGone =>
      'Diese gesendete Notiz wartet nicht mehr: Sie wurde angenommen oder abgelehnt.';

  @override
  String sentItemsWaiting(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count gesendete Notizen warten',
      one: '1 gesendete Notiz wartet',
    );
    return '$_temp0';
  }

  @override
  String get sentItemsWaitingTitle => 'Wartende gesendete Notizen';

  @override
  String get sentItemsNoneWaiting => 'Keine gesendeten Notizen warten.';
}
