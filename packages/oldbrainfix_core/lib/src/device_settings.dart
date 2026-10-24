import 'store.dart';

/// Which theme this Device shows: the system's, or a fixed one.
enum ThemeChoice { system, light, dark }

/// Settings of this Device, kept as local settings and never synced
/// (ADR 0008).
extension DeviceSettings on Store {
  static const _theme = 'theme';

  /// The theme override; anything unknown follows the system.
  ThemeChoice get themeChoice =>
      ThemeChoice.values.asNameMap()[localSetting(_theme)] ??
      ThemeChoice.system;

  set themeChoice(ThemeChoice choice) => setLocalSetting(_theme, choice.name);
}
