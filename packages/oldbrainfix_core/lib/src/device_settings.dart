import 'nudges.dart';
import 'schedule.dart';
import 'store.dart';

/// Which theme this Device shows: the system's, or a fixed one.
enum ThemeChoice { system, light, dark }

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
  static const _nudgeRepeatMinutes = 'nudgeRepeatMinutes';
  static const _calendarContext = 'calendarContext';
  static const _bluetoothContext = 'bluetoothContext';
  static const _streaksOff = 'streaksOff';

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

  /// Whether Notes captured on this Device record the connected
  /// Bluetooth devices; off until the user switches it on.
  bool get bluetoothContextOn => localSetting(_bluetoothContext) == 'true';

  set bluetoothContextOn(bool on) => setLocalSetting(_bluetoothContext, '$on');

  /// Whether this Device shows Streaks; on unless switched off. Adherence
  /// shows either way.
  bool get streaksOn => localSetting(_streaksOff) != 'true';

  set streaksOn(bool on) => setLocalSetting(_streaksOff, '${!on}');
}
