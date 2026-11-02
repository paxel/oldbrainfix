import 'profiles.dart';
import 'store.dart';

/// Where the person's own settings lie in the log (#211).
abstract final class PersonSettingKeys {
  /// The entity holding them, outside every Area like an Override: sealed
  /// under the personal key, it goes to the user's own Devices only, and
  /// only what they wrote counts.
  static const entity = 'settings:person';

  /// "Move a Note to the Trash when its one-time Reminder is done":
  /// [on] while it is on; absent, null or any other value for off.
  static const trashDoneReminder = 'trashDoneReminder';

  static const on = 'on';
}

/// The settings of the person rather than of this Device (#211): kept in
/// the log like an Override, so they are the same on all of the user's
/// own Devices and never reach another person. What another person's
/// Device wrote counts for nothing. The per-Device ones are
/// [DeviceSettings].
extension PersonSettings on Store {
  /// Whether Done on a one-time Reminder that is the only Routine or
  /// Reminder of a plain Note moves the Note to the trash with it
  /// ([Routines.checkOff]). Off until switched on; a value no app writes
  /// reads as off, so nothing is trashed by it.
  bool get trashesDoneReminder =>
      currentByOwn(
        PersonSettingKeys.entity,
        PersonSettingKeys.trashDoneReminder,
      ) ==
      PersonSettingKeys.on;

  /// Switches it on or off for all of the user's own Devices; the same
  /// again writes nothing.
  set trashesDoneReminder(bool on) {
    if (trashesDoneReminder == on) return;
    append(
      PersonSettingKeys.entity,
      PersonSettingKeys.trashDoneReminder,
      on ? PersonSettingKeys.on : null,
    );
  }
}
