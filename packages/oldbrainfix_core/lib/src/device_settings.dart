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
}
