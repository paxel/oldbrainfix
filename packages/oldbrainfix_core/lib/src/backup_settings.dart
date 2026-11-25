import 'device_settings.dart';
import 'store.dart';

/// The Device settings a Backup carries (ADR 0016, obf-1098): one record
/// after the Profile, `{"settings":{…}}`, keys sorted, each value as the
/// setting keeps it. Only the settings of [_carried] go into it and come
/// out of it, each only with a value the Device offers, so a crafted file
/// cannot plant a signing seed, a PIN, the backup key or a folder. The
/// desktop core keeps the same list, `backup_settings.rs`.

/// The app a Device runs: a Backup's settings are taken only by the app
/// that has them, so a phone's Backup restored on the desktop brings
/// what both have.
enum SettingsApp { phone, desktop }

/// The local setting a Backup's speech model comes back as, on the
/// Device it is restored on: an offer, as the model's file stays behind.
const _speechModelOffer = 'speechModelOffer';

/// The local setting of the speech model in use (obf-1098).
const _speechModel = 'speechModel';

/// One setting a Backup carries: its key, the values a Device offers
/// for it, and which apps have it.
typedef _Carried = ({
  String key,
  bool Function(String value) offered,
  bool phone,
  bool desktop,
});

bool _switch(String value) => value == 'true' || value == 'false';

bool Function(String) _oneOf(List<String> values) => values.contains;

/// A whole number of minutes from 1 to a day, as the apps write it: an
/// interval set elsewhere stays offered beside the choices.
bool _minutes(String value) {
  if (value.isEmpty || value.length > 4 || value.startsWith('0')) {
    return false;
  }
  if (value.codeUnits.any((c) => c < 0x30 || c > 0x39)) return false;
  return int.parse(value) <= 1440;
}

/// A time of day as the cores write it: `08:00`.
bool _clock(String value) {
  if (value.length != 5 || value[2] != ':') return false;
  final digits = '${value.substring(0, 2)}${value.substring(3)}';
  if (digits.codeUnits.any((c) => c < 0x30 || c > 0x39)) return false;
  return int.parse(value.substring(0, 2)) < 24 &&
      int.parse(value.substring(3)) < 60;
}

/// Quiet Hours as the cores write them: `22:00-08:00`.
bool _quietHours(String value) =>
    value.length == 11 &&
    value[5] == '-' &&
    _clock(value.substring(0, 5)) &&
    _clock(value.substring(6));

/// A speech model and how it came: `base:download`, `small:file`.
bool _speechModelOffered(String value) => const [
  'base:download',
  'base:file',
  'small:download',
  'small:file',
].contains(value);

/// Every setting a Backup carries, by key. Left behind, and asked for
/// again: the App Lock with its PIN, biometrics and relock time, the
/// Area Locks, the paired Devices, the shared folder and the Backup
/// folder (with Google's copy), the alarm sound and the calendars picked
/// (named as only that Device names them), the switches that need a
/// permission granted there, the AI with its provider, keys and the
/// questions allowed, the autostart and the capture key chord, the
/// window and the folds, and what the Device only notes for itself.
final List<_Carried> _carried = [
  (key: 'agendaHideChecked', offered: _switch, phone: true, desktop: true),
  (key: 'alertsOff', offered: _switch, phone: false, desktop: true),
  (key: 'allDayFrom', offered: _clock, phone: true, desktop: true),
  (key: 'fatFingers', offered: _switch, phone: true, desktop: true),
  (key: 'nudgeRepeatMinutes', offered: _minutes, phone: true, desktop: true),
  (
    key: 'quickNoteNudgeButtonsOff',
    offered: _switch,
    phone: true,
    desktop: true,
  ),
  (key: 'quietHours', offered: _quietHours, phone: true, desktop: true),
  (key: 'quietHoursOff', offered: _switch, phone: true, desktop: true),
  (
    key: 'recordingPauseSeconds',
    offered: _oneOf(const ['2', '3', '5', '10']),
    phone: true,
    desktop: true,
  ),
  (key: 'renagMinutes', offered: _minutes, phone: true, desktop: true),
  (
    key: 'shoppingAgenda',
    offered: _oneOf(const ['lines', 'back', 'open', 'off']),
    phone: true,
    desktop: true,
  ),
  (key: _speechModel, offered: _speechModelOffered, phone: true, desktop: true),
  (key: 'streaksOff', offered: _switch, phone: true, desktop: true),
  (
    key: 'textSize',
    offered: _oneOf(const ['normal', 'larger', 'largest']),
    phone: true,
    desktop: true,
  ),
  (
    key: 'theme',
    offered: _oneOf(const ['system', 'light', 'dark']),
    phone: true,
    desktop: true,
  ),
  (key: 'tipsOff', offered: _switch, phone: true, desktop: true),
  (key: 'twoRowEditorOff', offered: _switch, phone: true, desktop: false),
  (
    key: 'unitSystem',
    offered: _oneOf(const ['metric', 'imperial']),
    phone: true,
    desktop: true,
  ),
];

/// The settings record of a Backup of [store], keys sorted: each setting
/// of [_carried] it holds with a value offered, for either app; the speech
/// model in use, else the one still offered after a restore, so an offer
/// not yet answered is not lost. Empty when it holds none.
Map<String, String> carriedSettings(Store store) {
  final held = [
    for (final setting in _carried)
      if (_held(store, setting) case final value?) (setting.key, value),
  ]..sort((a, b) => a.$1.compareTo(b.$1));
  return {for (final (key, value) in held) key: value};
}

String? _held(Store store, _Carried setting) {
  final keys = setting.key == _speechModel
      ? const [_speechModel, _speechModelOffer]
      : [setting.key];
  for (final key in keys) {
    final value = store.localSetting(key);
    if (value != null && setting.offered(value)) return value;
  }
  return null;
}

/// What [app] takes of a Backup's settings [record], as the local
/// settings to keep, sorted by key: each setting of [_carried] that
/// [app] has, written as text with a value offered. Anything else is
/// passed over: a key not carried, a value not offered or not text, and
/// a setting the other app has alone. The speech model comes as the
/// offer of it ([DeviceSettings.speechModelOffer]).
List<(String, String)> takenSettings(Object? record, SettingsApp app) {
  if (record is! Map) return const [];
  final taken = <(String, String)>[];
  for (final setting in _carried) {
    final has = switch (app) {
      SettingsApp.phone => setting.phone,
      SettingsApp.desktop => setting.desktop,
    };
    if (!has) continue;
    if (record[setting.key] case final String value
        when setting.offered(value)) {
      final key = setting.key == _speechModel ? _speechModelOffer : setting.key;
      taken.add((key, value));
    }
  }
  return taken..sort((a, b) => a.$1.compareTo(b.$1));
}
