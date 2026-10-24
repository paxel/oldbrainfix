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
  String get takePhoto => 'Foto aufnehmen';

  @override
  String get photo => 'Foto';

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
  String get nudgeSettingsHeading => 'Stupser auf diesem Handy';

  @override
  String nudgeSettingsSummary(int minutes, String quiet) {
    return 'Erneut nach $minutes Min., $quiet';
  }

  @override
  String get nudgeSettingsExplain =>
      'Solange etwas unsortiert ist, kommt ein unbeachteter Stupser nach dieser Zeit wieder. In der Ruhezeit warten Stupser, bis sie endet; Routinen und Erinnerungen klingeln trotzdem pünktlich.';

  @override
  String get nudgeRepeat => 'Erneut stupsen nach';

  @override
  String get quietHours => 'Ruhezeit';

  @override
  String quietHoursRange(String from, String until) {
    return 'Ruhezeit $from bis $until';
  }

  @override
  String get quietHoursOff => 'keine Ruhezeit';

  @override
  String get quietFrom => 'Von';

  @override
  String get quietUntil => 'Bis';

  @override
  String get yes => 'Ja';

  @override
  String get no => 'Nein';

  @override
  String deleteNoteRoutines(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Auch die $count verknüpften Routinen löschen?',
      one: 'Auch die verknüpfte Routine löschen?',
    );
    return '$_temp0';
  }

  @override
  String deleteNoteReminders(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Auch die $count verknüpften Erinnerungen löschen?',
      one: 'Auch die verknüpfte Erinnerung löschen?',
    );
    return '$_temp0';
  }

  @override
  String deleteNoteRoutinesAndReminders(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Auch die $count verknüpften Routinen und Erinnerungen löschen?',
    );
    return '$_temp0';
  }

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
}
