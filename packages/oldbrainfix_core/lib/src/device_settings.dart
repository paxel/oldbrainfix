import 'dart:convert';

import 'nudges.dart';
import 'schedule.dart';
import 'schedule_sentence.dart';
import 'shopping_lists.dart';
import 'store.dart';
import 'tally_units.dart';

/// Which theme this Device shows: the system's, or a fixed one.
enum ThemeChoice { system, light, dark }

/// What the quick note (the text capture screen) shows beyond its text
/// field, its Photo, Voice, Gallery and Paste, its Nudge buttons, "To
/// full editor" and the alarm clock for a Routine or Reminder (#223), each
/// once this Device switched it on (#142).
enum QuickNoteControl {
  /// "File into…".
  fileInto('quickNoteFileInto'),

  /// Frog and Session in the alarm clock's menu, and "Make it a Tally"
  /// (Q481); the clock offers a Routine or Reminder without it (#223).
  schedule('quickNoteSchedule'),

  /// "Add to a Shopping List".
  shopping('quickNoteShopping'),

  /// The Importance.
  importance('quickNoteImportance'),

  /// The format bar above the keyboard (ADR 0033).
  formatBar('quickNoteFormatBar');

  /// The local setting that holds "true" while it is switched on.
  final String key;

  const QuickNoteControl(this.key);
}

/// The playback speeds of voice recordings offered, 1 being normal speed.
const playbackSpeeds = [0.5, 0.75, 1.0, 1.5, 2.0];

/// Settings of this Device, kept as local settings and never synced
/// (ADR 0008).
extension DeviceSettings on Store {
  static const _theme = 'theme';
  static const _starterSetOffered = 'starterSetOffered';
  static const _locationExplained = 'locationExplained';
  static const _alarmSetupDone = 'alarmSetupDone';
  static const _renagMinutes = 'renagMinutes';
  static const _missedSeen = 'missedSeen';
  static const _quietHours = 'quietHours';
  static const _quietHoursOff = 'quietHoursOff';
  static const _allDayFrom = 'allDayFrom';
  static const _nudgeRepeatMinutes = 'nudgeRepeatMinutes';
  static const _calendarContext = 'calendarContext';
  static const _bluetoothContext = 'bluetoothContext';
  static const _agendaCalendar = 'agendaCalendar';
  static const _agendaCalendars = 'agendaCalendars';
  static const _streaksOff = 'streaksOff';
  static const _agendaHideChecked = 'agendaHideChecked';
  static const _fatFingers = 'fatFingers';
  static const _twoRowEditorOff = 'twoRowEditorOff';
  static const _playbackSpeed = 'playbackSpeed';
  static const _alarmSound = 'alarmSound';
  static const _unitSystem = 'unitSystem';
  static const _shoppingAgenda = 'shoppingAgenda';
  static const _quickNoteNudgeButtonsOff = 'quickNoteNudgeButtonsOff';
  static const _lastShoppingList = 'lastShoppingList';
  static const _recentlyFiledInto = 'recentlyFiledInto';

  /// How many places filed into File into… shows at its top (#272).
  static const recentPlacesKept = 3;

  /// The Re-nag interval when nothing else is set.
  static const defaultRenag = Duration(minutes: 10);

  /// How long an ignored Nudge waits before it comes again, when nothing
  /// else is set.
  static const defaultNudgeRepeat = Duration(hours: 1);

  /// The Quiet Hours when nothing else is set: 22:00 to 08:00.
  static final defaultQuietHours = QuietHours(
    ClockTime(22, 0),
    ClockTime(8, 0),
  );

  /// When all-day Occurrences start to ring when nothing else is set
  /// (#201).
  static final defaultAllDayFrom = ClockTime(15, 0);

  /// When the day starts without Quiet Hours (#271).
  static final defaultDayStart = ClockTime(9, 0);

  /// The theme override; anything unknown follows the system.
  ThemeChoice get themeChoice =>
      ThemeChoice.values.asNameMap()[localSetting(_theme)] ??
      ThemeChoice.system;

  set themeChoice(ThemeChoice choice) => setLocalSetting(_theme, choice.name);

  /// Whether this Device has already offered the Starter Set on first
  /// start.
  bool get starterSetOffered => localSetting(_starterSetOffered) == 'true';

  set starterSetOffered(bool offered) =>
      setLocalSetting(_starterSetOffered, '$offered');

  /// Whether this Device has already explained why Notes record where
  /// they were captured and asked for the location permission.
  bool get locationExplained => localSetting(_locationExplained) == 'true';

  set locationExplained(bool explained) =>
      setLocalSetting(_locationExplained, '$explained');

  /// Whether this Device has walked through the alarm setup once; after
  /// that only the red banner leads back to it.
  bool get alarmSetupDone => localSetting(_alarmSetupDone) == 'true';

  set alarmSetupDone(bool done) => setLocalSetting(_alarmSetupDone, '$done');

  /// The sound this Device rings every Occurrence with, as the system's
  /// sound picker named it; null for the app's own sound. Escalation
  /// keeps the system alarm sound either way.
  String? get alarmSound {
    final sound = localSetting(_alarmSound);
    return sound == null || sound.trim().isEmpty ? null : sound;
  }

  /// Null or blank goes back to the app's own sound.
  set alarmSound(String? sound) => sound == null || sound.trim().isEmpty
      ? removeLocalSetting(_alarmSound)
      : setLocalSetting(_alarmSound, sound);

  /// How often an ignored Occurrence fires again, unless its Routine has
  /// its own interval.
  Duration get renagInterval {
    final minutes = int.tryParse(localSetting(_renagMinutes) ?? '');
    return minutes == null || minutes < 1
        ? defaultRenag
        : Duration(minutes: minutes);
  }

  set renagInterval(Duration interval) {
    if (interval.inMinutes < 1) {
      throw ArgumentError.value(interval, 'interval', 'at least a minute');
    }
    setLocalSetting(_renagMinutes, '${interval.inMinutes}');
  }

  /// When the user last tapped "seen" on the Missed Summary on this
  /// Device; null before the first time.
  DateTime? get missedSeen =>
      DateTime.tryParse(localSetting(_missedSeen) ?? '')?.toUtc();

  set missedSeen(DateTime? seen) =>
      setLocalSetting(_missedSeen, seen?.toUtc().toIso8601String() ?? '');

  /// This Device's Quiet Hours, kept while they are switched off.
  QuietHours get quietHours {
    final parts = (localSetting(_quietHours) ?? '').split('-');
    final from = ClockTime.tryParse(parts.first);
    final until = parts.length == 2 ? ClockTime.tryParse(parts.last) : null;
    return from == null || until == null
        ? defaultQuietHours
        : QuietHours(from, until);
  }

  set quietHours(QuietHours quiet) =>
      setLocalSetting(_quietHours, '${quiet.start}-${quiet.end}');

  /// Whether Nudges wait out the Quiet Hours on this Device; on unless
  /// switched off.
  bool get quietHoursOn => localSetting(_quietHoursOff) != 'true';

  set quietHoursOn(bool on) => setLocalSetting(_quietHoursOff, '${!on}');

  /// When this Device's day starts, at which a Reminder for "tomorrow"
  /// rings (#271): when its Quiet Hours end, or 09:00 while they are
  /// switched off or empty.
  ClockTime get dayStart {
    final quiet = quietHours;
    return quietHoursOn && quiet.start != quiet.end
        ? quiet.end
        : defaultDayStart;
  }

  /// Now by the store's clock on [wallClock], with this Device's
  /// [dayStart]: what the schedule sentence's relative words and the
  /// quick Reminder chips are read at (#271).
  SentenceNow sentenceNow(WallClock wallClock) =>
      SentenceNow.at(wallClock(clock().toUtc()), dayStart);

  /// From when on this Device an all-day Occurrence rings, on the wall
  /// clock; before that it only shows on the Agenda (#201).
  ClockTime get allDayFrom =>
      ClockTime.tryParse(localSetting(_allDayFrom) ?? '') ?? defaultAllDayFrom;

  set allDayFrom(ClockTime time) => setLocalSetting(_allDayFrom, '$time');

  /// How long an ignored Nudge waits before it comes again.
  Duration get nudgeRepeat {
    final minutes = int.tryParse(localSetting(_nudgeRepeatMinutes) ?? '');
    return minutes == null || minutes < 1
        ? defaultNudgeRepeat
        : Duration(minutes: minutes);
  }

  set nudgeRepeat(Duration interval) {
    if (interval.inMinutes < 1) {
      throw ArgumentError.value(interval, 'interval', 'at least a minute');
    }
    setLocalSetting(_nudgeRepeatMinutes, '${interval.inMinutes}');
  }

  /// Whether Notes captured on this Device record the calendar events
  /// running; off until the user switches it on.
  bool get calendarContextOn => localSetting(_calendarContext) == 'true';

  set calendarContextOn(bool on) => setLocalSetting(_calendarContext, '$on');

  /// Whether this Device's Agenda shows the appointments of the
  /// calendars picked in [agendaCalendars]; off until switched on.
  bool get agendaCalendarOn => localSetting(_agendaCalendar) == 'true';

  set agendaCalendarOn(bool on) => setLocalSetting(_agendaCalendar, '$on');

  /// The ids of this Device's calendars whose appointments the Agenda
  /// shows; none until the user picks some. Kept while the switch is
  /// off, so turning it on again brings them back.
  Set<String> get agendaCalendars {
    try {
      if (jsonDecode(localSetting(_agendaCalendars) ?? '[]')
          case final List<Object?> ids) {
        return {...ids.whereType<String>()};
      }
    } on FormatException {
      // A broken setting picks none.
    }
    return {};
  }

  set agendaCalendars(Set<String> ids) =>
      setLocalSetting(_agendaCalendars, jsonEncode([...ids]..sort()));

  /// Whether Notes captured on this Device record the connected
  /// Bluetooth devices; off until the user switches it on.
  bool get bluetoothContextOn => localSetting(_bluetoothContext) == 'true';

  set bluetoothContextOn(bool on) => setLocalSetting(_bluetoothContext, '$on');

  /// Whether this Device shows Streaks; on unless switched off. Adherence
  /// shows either way.
  bool get streaksOn => localSetting(_streaksOff) != 'true';

  set streaksOn(bool on) => setLocalSetting(_streaksOff, '${!on}');

  /// Whether this Device's Agenda, and its widget, hide what is checked
  /// instead of striking it through (#217): a Done Occurrence, a checked
  /// Line, a pick with its entry, and a group with nothing left; off
  /// until switched on.
  bool get agendaHidesChecked => localSetting(_agendaHideChecked) == 'true';

  set agendaHidesChecked(bool on) => setLocalSetting(_agendaHideChecked, '$on');

  /// Whether this Device shows text, icons and tap targets 1.3 times
  /// their size, "Fat fingers" on the phone and "Bigger text and icons"
  /// on the desktop (#224); off until switched on.
  bool get fatFingers => localSetting(_fatFingers) == 'true';

  set fatFingers(bool on) => setLocalSetting(_fatFingers, '$on');

  /// Whether the Note editor and Amend on this Device type in two rows,
  /// the bottom one above the keyboard; on unless switched off, which
  /// gives the system's text field back.
  bool get twoRowEditorOn => localSetting(_twoRowEditorOff) != 'true';

  set twoRowEditorOn(bool on) => setLocalSetting(_twoRowEditorOff, '${!on}');

  /// Whether the quick note on this Device shows [control]; off until
  /// switched on.
  bool quickNoteShows(QuickNoteControl control) =>
      localSetting(control.key) == 'true';

  void setQuickNoteShows(QuickNoteControl control, bool on) =>
      setLocalSetting(control.key, '$on');

  /// Whether the quick note on this Device shows the three Nudge buttons;
  /// on unless switched off, which gives a single Save in their place.
  bool get quickNoteNudgeButtonsOn =>
      localSetting(_quickNoteNudgeButtonsOff) != 'true';

  set quickNoteNudgeButtonsOn(bool on) =>
      setLocalSetting(_quickNoteNudgeButtonsOff, '${!on}');

  /// The Note id of the Shopping List a capture on this Device went into
  /// last, at the top of the lists to pick from (#276); null for none. It
  /// may name a list gone since.
  String? get lastShoppingList => switch (localSetting(_lastShoppingList)) {
    final id? when id.isNotEmpty => id,
    _ => null,
  };

  set lastShoppingList(String? noteId) => noteId == null
      ? removeLocalSetting(_lastShoppingList)
      : setLocalSetting(_lastShoppingList, noteId);

  /// The ids of the Areas and Topics this Device last filed into, the
  /// latest first, at most [recentPlacesKept] (#272). They may name a
  /// place deleted since; File into… shows only those it lists.
  List<String> get recentlyFiledInto {
    try {
      if (jsonDecode(localSetting(_recentlyFiledInto) ?? '[]')
          case final List<Object?> ids) {
        return [
          ...{...ids.whereType<String>()},
        ].take(recentPlacesKept).toList();
      }
    } on FormatException {
      // A broken setting names none.
    }
    return [];
  }

  /// Puts [placeId] first among [recentlyFiledInto], once, and forgets
  /// the oldest beyond [recentPlacesKept].
  void filedInto(String placeId) => setLocalSetting(
    _recentlyFiledInto,
    jsonEncode(
      [
        placeId,
        ...recentlyFiledInto.where((id) => id != placeId),
      ].take(recentPlacesKept).toList(),
    ),
  );

  /// The system this Device shows the known units of Tallies in, when
  /// one was chosen; null follows the region.
  UnitSystem? get chosenUnitSystem =>
      UnitSystem.values.asNameMap()[localSetting(_unitSystem)];

  /// Null follows the region again.
  set chosenUnitSystem(UnitSystem? system) => system == null
      ? removeLocalSetting(_unitSystem)
      : setLocalSetting(_unitSystem, system.name);

  /// The system this Device shows the known units of Tallies in: the one
  /// chosen, else the one of [region], the ISO country code the system
  /// reports.
  UnitSystem unitSystem(String? region) =>
      chosenUnitSystem ?? TallyUnits.systemForRegion(region);

  /// How this Device's Agenda showed Shopping Lists before each list had
  /// its own choice (#186); one row per list on a day when Lines came
  /// back unless another way was chosen. Read only for the default of a
  /// list nobody chose for ([ShoppingOnAgenda.fromDevice]); no app offers
  /// it any more.
  ShoppingAgenda get shoppingAgenda =>
      ShoppingAgenda.read(localSetting(_shoppingAgenda));

  /// How fast this Device plays voice recordings: one of
  /// [playbackSpeeds], normal speed when none was chosen. The one Device
  /// setting a Backup carries (ADR 0016).
  double get playbackSpeed => chosenPlaybackSpeed ?? 1;

  /// The playback speed chosen on this Device; null when none of
  /// [playbackSpeeds] was.
  double? get chosenPlaybackSpeed {
    final speed = double.tryParse(localSetting(_playbackSpeed) ?? '');
    return speed != null && playbackSpeeds.contains(speed) ? speed : null;
  }

  /// Keeps [speed] as written since 0.1.1: "1.0", "1.5".
  set playbackSpeed(double speed) {
    if (!playbackSpeeds.contains(speed)) {
      throw ArgumentError.value(speed, 'speed', 'not a speed offered');
    }
    setLocalSetting(_playbackSpeed, '$speed');
  }
}
