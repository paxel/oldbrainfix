import 'dart:convert';

import 'places.dart';
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

  /// Simple or Advanced (#197): [simple] or [advanced] once chosen;
  /// absent, null or any other value reads as Advanced, so nothing is
  /// hidden from someone who never chose.
  static const features = 'features';

  static const simple = 'simple';
  static const advanced = 'advanced';

  /// The person's own order of the Areas (#280): a JSON list of Area ids,
  /// first to last. Ids that are no Area here, repeats and anything not a
  /// string are skipped; absent, null or anything not a list reads as no
  /// order, so the Areas show by name.
  static const areaOrder = 'areaOrder';
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

  /// Whether making Tallies, Sessions, Frogs and Topics, the format bar
  /// and Shared Area roles are offered (#197). Only [PersonSettingKeys.simple]
  /// hides them; never chosen, or a value no app writes, reads as
  /// Advanced. What exists stays visible and usable either way.
  bool get advanced =>
      currentByOwn(PersonSettingKeys.entity, PersonSettingKeys.features) !=
      PersonSettingKeys.simple;

  /// Chooses Simple or Advanced for all of the user's own Devices; the
  /// same value stored again writes nothing, while Advanced chosen where
  /// none was stored is written, so the choice is recorded.
  set advanced(bool on) {
    final value = on ? PersonSettingKeys.advanced : PersonSettingKeys.simple;
    if (currentByOwn(PersonSettingKeys.entity, PersonSettingKeys.features) ==
        value) {
      return;
    }
    append(PersonSettingKeys.entity, PersonSettingKeys.features, value);
  }

  /// Every Area in the person's own order (#280), as the Areas list and
  /// the sidebar show them: those in the stored order first, as it has
  /// them, then the Areas missing from it (a new one, a Shared Area joined
  /// later) by name, as [Places.areas] lists them. With no order stored,
  /// all of them by name.
  List<Area> areasInOrder() {
    final byName = Places(this).areas();
    final byId = {for (final area in byName) area.id: area};
    final ordered = <Area>[];
    for (final id in _heldAreaOrder()) {
      if (byId.remove(id) case final area?) ordered.add(area);
    }
    return [...ordered, ...byName.where((a) => byId.containsKey(a.id))];
  }

  /// Puts the Area [areaId] at [index] of [areasInOrder], counted after
  /// it was taken out, for all of the user's own Devices. An index past
  /// either end puts it first or last. The whole order is stored, so a
  /// new Area goes to the end. Throws when [areaId] is no Area.
  void moveArea(String areaId, int index) {
    final ids = [for (final area in areasInOrder()) area.id];
    if (!ids.remove(areaId)) {
      throw ArgumentError.value(areaId, 'areaId', 'not an Area');
    }
    ids.insert(index.clamp(0, ids.length), areaId);
    _storeAreaOrder(ids);
  }

  /// Puts the Areas in the order of [areaIds] for all of the user's own
  /// Devices; the Areas it leaves out follow in the order they had. Ids
  /// that are no Area are skipped.
  void setAreaOrder(List<String> areaIds) {
    final shown = [for (final area in areasInOrder()) area.id];
    final named = {...areaIds.where(shown.contains)};
    _storeAreaOrder([...named, ...shown.where((id) => !named.contains(id))]);
  }

  /// Stores [ids] as the order; the order already shown writes nothing.
  void _storeAreaOrder(List<String> ids) {
    final shown = [for (final area in areasInOrder()) area.id];
    if (jsonEncode(ids) == jsonEncode(shown)) return;
    append(
      PersonSettingKeys.entity,
      PersonSettingKeys.areaOrder,
      jsonEncode(ids),
    );
  }

  /// The ids of the stored order, as written by the user's own Devices,
  /// the newest such entry winning; empty when there is none or it is
  /// not a list.
  List<String> _heldAreaOrder() {
    final value = currentByOwn(
      PersonSettingKeys.entity,
      PersonSettingKeys.areaOrder,
    );
    if (value == null) return const [];
    final Object? list;
    try {
      list = jsonDecode(value);
    } on FormatException {
      return const [];
    }
    if (list is! List) return const [];
    return [...list.whereType<String>()];
  }
}
